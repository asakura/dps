//! Application state and tab routing.

use std::{fmt, path::Path};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Tabs, Widget},
};
use tokio::sync::mpsc;

use tracing::{debug, info};

use crate::{
    action::Action,
    components::{
        Component, ComponentNew, FpsCounter, Home, KeyBinding, hint_bar::HintBar, mod_tab::ModTab,
        ppo2_tab::PpO2Tab, which_key::WhichKey,
    },
    config::Config,
    keymap::{ChordEngine, ChordResult, KeyBindings, Mode, ModeMap, SequenceEngine},
    theme::Theme,
    tui::{Event, Tui},
};

static GLOBAL_BINDINGS: &[KeyBinding] = [
    KeyBinding {
        key: "Tab",
        desc: "next table",
    },
    KeyBinding {
        key: "q/Esc",
        desc: "quit",
    },
    KeyBinding {
        key: "?",
        desc: "toggle bindings",
    },
]
.as_slice();

/// Top-level coordinator: owns the tab list, tracks the active tab, and routes
/// key events and render calls to the appropriate component.
pub struct App {
    tabs: Vec<Box<dyn Component + Send>>,
    active: usize,
    show_which_key: bool,
    keybindings: KeyBindings,
    chord: SequenceEngine,
    mode: Mode,
    tick_rate: f64,
    frame_rate: f64,
    theme: Theme,
}

impl fmt::Debug for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("App")
            .field("active", &self.active)
            .field("show_which_key", &self.show_which_key)
            .field("mode", &self.mode)
            .field("tick_rate", &self.tick_rate)
            .field("frame_rate", &self.frame_rate)
            .finish_non_exhaustive()
    }
}

impl Default for App {
    /// Creates an `App` with hardcoded rates and [`Config::default`] — no disk I/O.
    ///
    /// Intended for tests and harness use. For production, use [`App::new`], which
    /// loads configuration from disk and propagates errors.
    fn default() -> Self {
        Self::from_config(4.0, 60.0, Config::default())
    }
}

impl App {
    /// Creates an `App`, loading configuration from disk.
    ///
    /// `tick_rate` controls how often the internal timer fires (Hz);
    /// `frame_rate` caps the render rate (Hz).  `config_dir` and `data_dir`
    /// override the env-var / platform defaults when `Some`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Config`] if a config file is present but cannot
    /// be parsed, if theme resolution fails, or if `defaultTheme` does not
    /// match any resolved theme.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use dps::app::App;
    /// let _app = App::new(4.0, 60.0, None, None)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        tick_rate: f64,
        frame_rate: f64,
        config_dir: Option<&Path>,
        data_dir: Option<&Path>,
    ) -> Result<Self, crate::Error> {
        let config = Config::from_dirs(config_dir, data_dir)?;
        tracing::debug!(
            data_dir = %config.config.data_dir.display(),
            config_dir = %config.config.config_dir.display(),
            "effective directories"
        );
        Ok(Self::from_config(tick_rate, frame_rate, config))
    }

    fn from_config(tick_rate: f64, frame_rate: f64, config: Config) -> Self {
        let theme = *config.active_theme();
        Self {
            tabs: vec![Box::new(ModTab::new()), Box::new(PpO2Tab::new())],
            active: 0,
            show_which_key: false,
            keybindings: config.keybindings,
            chord: SequenceEngine::default(),
            mode: Mode::Normal,
            tick_rate,
            frame_rate,
            theme,
        }
    }

    /// Runs the event loop until the user quits.
    ///
    /// Enters the terminal (raw mode + alternate screen), then drives
    /// tick/render intervals and key input until [`Action::Quit`] is received,
    /// the event stream closes, or a termination signal (`SIGTERM`/`SIGINT`) arrives.
    /// On Unix, `Ctrl+Z` (mapped to [`Action::Suspend`]) saves the terminal state,
    /// suspends the process, and restores it on resume (`SIGCONT`).
    /// Terminal state is restored on return.
    ///
    /// # Errors
    ///
    /// Propagates I/O errors from terminal setup ([`Tui::new`], [`Tui::enter`]),
    /// frame rendering, signal-handler registration, and terminal teardown.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use dps::app::App;
    /// let mut app = App::new(4.0, 60.0, None, None)?;
    /// app.run().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);
        tui.enter()?;

        #[cfg(unix)]
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        #[cfg(unix)]
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

        let mut needs_render = true;

        loop {
            #[cfg(unix)]
            let event = tokio::select! {
                event = tui.next_event() => event,
                _ = sigterm.recv() => Some(Event::Quit),
                _ = sigint.recv() => Some(Event::Quit),
            };
            #[cfg(not(unix))]
            let event = tui.next_event().await;

            match event {
                Some(Event::Render) => {
                    if needs_render {
                        tui.draw(|f| self.render(f))?;
                        needs_render = false;
                    }
                }
                Some(Event::Init | Event::Resize(_, _)) => {
                    needs_render = true;
                }
                Some(Event::Key(key)) => {
                    match self.handle_key(key) {
                        Action::Quit => break,
                        #[cfg(unix)]
                        Action::Suspend => {
                            tui.exit()?;
                            signal_hook::low_level::emulate_default_handler(
                                signal_hook::consts::SIGTSTP,
                            )?;
                            tui.resume()?;
                        }
                        _ => {}
                    }
                    needs_render = true;
                }
                Some(Event::Quit | Event::Closed) | None => break,
                Some(
                    Event::Error
                    | Event::Tick
                    | Event::FocusGained
                    | Event::FocusLost
                    | Event::Paste(_)
                    | Event::Mouse(_),
                ) => {}
            }
        }

        Ok(())
    }

    /// Advances the chord engine with `key` and routes the result.
    ///
    /// - **Exact match** — dispatches the bound [`Action`] and returns its result.
    /// - **Prefix match** — returns [`Action::None`] and keeps accumulating.
    /// - **No match** — delegates to the hardcoded global fallback.
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        static EMPTY: std::sync::LazyLock<ModeMap> = std::sync::LazyLock::new(ModeMap::default);
        let bindings = self.keybindings.get(&self.mode).unwrap_or(&EMPTY);

        match self.chord.advance(key, bindings) {
            ChordResult::Exact(action) => self.dispatch(action),
            ChordResult::Prefix => Action::None,
            ChordResult::NoMatch => self.handle_key_fallback(key),
        }
    }

    /// Routes a resolved [`Action`] to the right handler.
    ///
    /// `Quit` propagates to the caller; all other actions are dispatched to
    /// the active component and `None` is returned so the event loop only
    /// needs to check for `Quit`.
    fn dispatch(&mut self, action: Action) -> Action {
        match action {
            Action::Quit => Action::Quit,
            Action::Suspend => Action::Suspend,
            other @ (Action::Move(_) | Action::Select | Action::None) => {
                self.tabs[self.active].handle_action(other);
                Action::None
            }
            _ => Action::None,
        }
    }

    /// Hardcoded global bindings: `?` toggles which-key, `q`/Esc quits,
    /// Tab cycles tabs. Everything else is delegated to the active component.
    fn handle_key_fallback(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('?') => {
                self.show_which_key = !self.show_which_key;
                Action::None
            }
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            KeyCode::Tab => {
                self.active = (self.active + 1) % self.tabs.len();
                Action::None
            }
            _ => Action::None,
        }
    }

    /// Draws the tab bar, active component, status bar, and help line.
    pub fn render(&mut self, f: &mut Frame<'_>) {
        f.render_widget(self, f.area());
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

        let titles: Vec<&str> = self.tabs.iter().map(|t| t.title()).collect();
        Tabs::new(titles)
            .select(self.active)
            .style(self.theme.nav_bar())
            .highlight_style(self.theme.selection())
            .divider("│")
            .render(chunks[0], buf);

        self.tabs[self.active].render(chunks[1], buf, &self.theme);
        self.tabs[self.active].render_status(chunks[2], buf, &self.theme);

        HintBar::new(
            self.tabs[self.active].key_bindings(),
            GLOBAL_BINDINGS,
            self.theme,
        )
        .render(chunks[3], buf);

        if self.show_which_key {
            WhichKey::new(
                GLOBAL_BINDINGS,
                self.tabs[self.active].key_bindings(),
                self.theme,
            )
            .render(area, buf);
        }
    }
}

/// Event-loop coordinator that owns a set of [`ComponentNew`] instances.
///
/// `AppNew` drives the TUI using the channel-based action pipeline introduced
/// by the [`ComponentNew`] trait family.  Every component receives a clone of
/// the [`mpsc::UnboundedSender<Action>`] during initialisation so it can push
/// actions at any time; `AppNew` owns the matching receiver and drains it on
/// every loop iteration.
///
/// # Architecture
///
/// The event loop is split into two phases per iteration:
///
/// 1. **Event phase** (`handle_events`) — waits for the next [`Event`] from
///    the [`Tui`] stream, converts it to an [`Action`], and fans it out to all
///    components.
/// 2. **Action phase** (`handle_actions`) — drains the action channel,
///    applies infrastructure actions (`Quit`, `Suspend`, `Resume`,
///    `ClearScreen`, `Resize`, `Render`) directly, then forwards every action
///    to each component's [`ComponentNew::update`] so components can react and
///    optionally enqueue follow-up actions.
///
/// # Keybindings
///
/// Key events are resolved via a [`SequenceEngine`] chord engine: prefix
/// sequences accumulate until either an exact binding fires or the sequence
/// breaks.
///
/// # Default components
///
/// A fresh instance always starts with two components:
/// - [`Home`] — main application screen.
/// - [`FpsCounter`] — tick/frame-rate overlay.
///
/// # Suspension
///
/// When [`Action::Suspend`] arrives the TUI is torn down, a `Resume` +
/// `ClearScreen` pair is enqueued, and the TUI is immediately re-entered so the
/// next render paints over any shell output that appeared while suspended.
pub struct AppNew {
    config: Config,
    tick_rate: f64,
    frame_rate: f64,
    components: Vec<Box<dyn ComponentNew>>,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    chord: SequenceEngine,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
}

impl fmt::Debug for AppNew {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppNew")
            .field("tick_rate", &self.tick_rate)
            .field("frame_rate", &self.frame_rate)
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}

impl AppNew {
    /// Creates an `AppNew` with the given tick and frame rates, loading config from disk.
    ///
    /// `tick_rate` controls how many [`Action::Tick`] events fire per second.
    /// `frame_rate` caps the render rate in Hz.  Both are forwarded to [`Tui`]
    /// when [`run`] is called.
    ///
    /// The action channel is created here so components can be handed a sender
    /// before [`run`] enters the event loop.
    ///
    /// # Errors
    ///
    /// Returns an error if [`Config::new`] fails to load or parse the
    /// configuration from disk.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> color_eyre::Result<()> {
    /// use dps::app::AppNew;
    /// let _app = AppNew::new(4.0, 60.0, None, None)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`run`]: AppNew::run
    pub fn new(
        tick_rate: f64,
        frame_rate: f64,
        config_dir: Option<&Path>,
        data_dir: Option<&Path>,
    ) -> Result<Self, crate::Error> {
        let config = Config::from_dirs(config_dir, data_dir)?;
        tracing::debug!(
            data_dir = %config.config.data_dir.display(),
            config_dir = %config.config.config_dir.display(),
            "effective directories"
        );

        let (action_tx, action_rx) = mpsc::unbounded_channel();

        Ok(Self {
            tick_rate,
            frame_rate,
            components: vec![Box::new(Home::new()), Box::new(FpsCounter::new())],
            should_quit: false,
            should_suspend: false,
            config,
            mode: Mode::Normal,
            chord: SequenceEngine::default(),
            action_tx,
            action_rx,
        })
    }

    /// Runs the event loop until the application quits.
    ///
    /// **Initialisation** (before the loop):
    /// 1. Constructs and enters the [`Tui`] with mouse support enabled and the
    ///    configured tick/frame rates.
    /// 2. Calls [`ComponentNew::register_action_handler`] on every component so
    ///    each receives a sender it can use to push actions asynchronously.
    /// 3. Calls [`ComponentNew::register_config_handler`] to distribute a clone
    ///    of the loaded [`Config`] to every component.
    /// 4. Calls [`ComponentNew::init`] with the current terminal size so
    ///    components can pre-compute layout-dependent state.
    ///
    /// **Event loop** (each iteration):
    /// - `handle_events` awaits the next [`Tui`] event, converts it to an
    ///   [`Action`], and fans it out to all components.
    /// - `handle_actions` drains the action channel, applies infrastructure
    ///   actions, and forwards every action to each component's
    ///   [`ComponentNew::update`].
    ///
    /// **Suspension** (`Action::Suspend`):
    /// The TUI is torn down, then `Resume` and `ClearScreen` are enqueued, and
    /// the TUI is immediately re-entered.  This lets the terminal process the
    /// suspend signal while ensuring the next render clears any output that
    /// appeared while the TUI was down.
    ///
    /// **Quit** (`Action::Quit`):
    /// The TUI is stopped and the loop breaks.  [`Tui::exit`] is called once
    /// more after the loop to guarantee terminal state is restored regardless of
    /// how the loop ended.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`Tui::new`], [`Tui::enter`], [`Tui::exit`],
    /// [`Tui::stop`], terminal size queries, component initialisation, event
    /// handling, and rendering.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> color_eyre::Result<()> {
    /// use dps::app::AppNew;
    /// let mut app = AppNew::new(4.0, 60.0, None, None)?;
    /// app.run().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[expect(
        clippy::future_not_send,
        reason = "dyn ComponentNew is not Send by design"
    )]
    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let mut tui = Tui::new()?
            .mouse(true)
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);

        tui.enter()?;

        for component in &mut self.components {
            component.register_action_handler(self.action_tx.clone())?;
        }

        for component in &mut self.components {
            component.register_config_handler(self.config.clone())?;
        }

        for component in &mut self.components {
            component.init(tui.size()?)?;
        }

        let action_tx = self.action_tx.clone();

        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;

            if self.should_suspend {
                tui.exit()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }

        tui.exit()?;

        Ok(())
    }

    /// Awaits the next [`Event`] from `tui`, converts it to an [`Action`], and
    /// fans the raw event out to every component.
    ///
    /// Infrastructure events (`Quit`, `Tick`, `Render`, `Resize`) are converted
    /// to their matching [`Action`] variants and sent on the channel.  [`Key`]
    /// events are forwarded to [`handle_key_event`] for keymap lookup.  All
    /// other events are silently ignored at this layer but still passed to
    /// component [`handle_events`] handlers so components may react to them
    /// directly.
    ///
    /// Returns `Ok(())` immediately if the event stream is exhausted.
    ///
    /// [`Key`]: Event::Key
    /// [`handle_key_event`]: AppNew::handle_key_event
    /// [`handle_events`]: ComponentNew::handle_events
    #[expect(
        clippy::future_not_send,
        reason = "dyn ComponentNew is not Send by design"
    )]
    async fn handle_events(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };

        let action_tx = self.action_tx.clone();

        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }

        for component in &mut self.components {
            if let Some(action) = component.handle_events(Some(event.clone()))? {
                action_tx.send(action)?;
            }
        }

        Ok(())
    }

    /// Resolves a raw [`KeyEvent`] against the active mode's keymap via the
    /// [`SequenceEngine`] chord engine.
    ///
    /// - **Exact match** — the bound [`Action`] is sent on the channel.
    /// - **Prefix match** — the engine keeps accumulating; nothing is sent.
    /// - **No match** — the key is silently dropped; no hardcoded fallback.
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<()> {
        static EMPTY: std::sync::LazyLock<ModeMap> = std::sync::LazyLock::new(ModeMap::default);
        let bindings = self.config.keybindings.get(&self.mode).unwrap_or(&EMPTY);

        if let ChordResult::Exact(action) = self.chord.advance(key, bindings) {
            info!("Got action: {action:?}");
            self.action_tx.send(action)?;
        }

        Ok(())
    }

    /// Drains the action channel and processes every queued [`Action`].
    ///
    /// **Infrastructure actions** are handled directly:
    ///
    /// | Action | Effect |
    /// |--------|--------|
    /// | `Tick` | No infrastructure effect; forwarded to components. |
    /// | `Quit` | Sets `should_quit`; the caller checks this flag after returning. |
    /// | `Suspend` / `Resume` | Toggles `should_suspend`. |
    /// | `ClearScreen` | Calls [`Tui::clear`]. |
    /// | `Resize(w, h)` | Delegates to [`handle_resize`]. |
    /// | `Render` | Delegates to [`render`]. |
    ///
    /// After each infrastructure action, the same action is forwarded to every
    /// component's [`ComponentNew::update`].  Any [`Action`] returned by
    /// `update` is re-enqueued so components can chain effects.
    ///
    /// [`handle_resize`]: AppNew::handle_resize
    /// [`render`]: AppNew::render
    fn handle_actions(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                debug!("{action:?}");
            }

            match action {
                Action::Tick => {}
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                _ => {}
            }

            for component in &mut self.components {
                if let Some(action) = component.update(action.clone())? {
                    self.action_tx.send(action)?;
                }
            }
        }

        Ok(())
    }

    /// Updates the [`Tui`] viewport to `(w, h)` and triggers an immediate render.
    ///
    /// Called in response to [`Action::Resize`]; the immediate render prevents
    /// a blank frame between the terminal resize and the next scheduled render tick.
    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> color_eyre::Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;

        self.render(tui)?;

        Ok(())
    }

    /// Draws all components onto the terminal frame.
    ///
    /// Calls [`ComponentNew::draw`] on each component in registration order.
    /// If a component returns an error it is converted to [`Action::Error`] and
    /// sent on the channel rather than propagating — this keeps a single
    /// misbehaving component from aborting the entire render pass.
    fn render(&mut self, tui: &mut Tui) -> color_eyre::Result<()> {
        tui.draw(|frame| {
            for component in &mut self.components {
                if let Err(err) = component.draw(frame, frame.area()) {
                    let _ = self
                        .action_tx
                        .send(Action::Error(format!("Failed to draw: {err:?}")));
                }
            }
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use crossterm::event::KeyModifiers;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    mod new {
        use super::*;

        #[test]
        fn succeeds_with_valid_rates() {
            assert!(App::new(4.0, 60.0, None, None).is_ok());
        }

        #[test]
        fn stores_tick_and_frame_rate() -> Result<()> {
            let app = App::new(10.0, 30.0, None, None)?;

            assert_relative_eq!(app.tick_rate, 10.0);
            assert_relative_eq!(app.frame_rate, 30.0);

            Ok(())
        }

        #[test]
        fn starts_on_first_tab() -> Result<()> {
            let app = App::new(4.0, 60.0, None, None)?;

            assert_eq!(app.active, 0);

            Ok(())
        }

        #[test]
        fn default_uses_fallback_rates() {
            let app = App::default();

            assert_relative_eq!(app.tick_rate, 4.0);
            assert_relative_eq!(app.frame_rate, 60.0);
        }
    }

    mod handle_key {
        use super::*;
        use std::collections::HashMap;

        use crate::action::Movement;
        use crate::config::{AppConfig, Config, Styles};
        use crate::keymap::{KeyBindingsBuilder, KeySeq, parse_key_sequence};

        fn with_config(config: Config) -> App {
            App::from_config(4.0, 60.0, config)
        }

        fn config_with_keybindings(bindings: &[(&str, Action)]) -> Result<Config> {
            let mut builder = KeyBindingsBuilder::new();
            for (seq_str, action) in bindings {
                builder.bind(
                    Mode::Normal,
                    KeySeq::from(parse_key_sequence(seq_str)?),
                    action.clone(),
                );
            }

            Ok(Config {
                config: AppConfig::default(),
                keybindings: builder.build(),
                styles: Styles(),
                themes: HashMap::from([("catpuccineFrappe".to_string(), Theme::default())]),
                default_theme: "catpuccineFrappe".to_string(),
            })
        }

        #[test]
        fn q_quits() {
            assert!(matches!(
                with_config(Config::default()).handle_key(press(KeyCode::Char('q'))),
                Action::Quit
            ));
        }

        #[test]
        fn esc_quits() {
            assert!(matches!(
                with_config(Config::default()).handle_key(press(KeyCode::Esc)),
                Action::Quit
            ));
        }

        #[test]
        fn question_mark_toggles_which_key() {
            let mut app = with_config(Config::default());

            assert!(!app.show_which_key);

            app.handle_key(press(KeyCode::Char('?')));
            assert!(app.show_which_key);

            app.handle_key(press(KeyCode::Char('?')));
            assert!(!app.show_which_key);
        }

        #[test]
        fn tab_cycles_active() {
            let mut app = with_config(Config::default());

            assert_eq!(app.active, 0);

            app.handle_key(press(KeyCode::Tab));
            assert_eq!(app.active, 1);

            app.handle_key(press(KeyCode::Tab));
            assert_eq!(app.active, 0);
        }

        #[test]
        fn other_keys_return_none() {
            assert!(matches!(
                with_config(Config::default()).handle_key(press(KeyCode::Char('j'))),
                Action::None
            ));
        }

        #[test]
        fn chord_broken_unbound_key_falls_to_global_fallback() -> Result<()> {
            // "g" is a prefix of "gg"; "q" breaks the chord and has no
            // configured binding, so the hardcoded fallback fires: q → Quit.
            let mut app = with_config(config_with_keybindings(
                [("gg", Action::Move(Movement::GotoTop))].as_slice(),
            )?);

            app.handle_key(press(KeyCode::Char('g')));

            assert!(matches!(
                app.handle_key(press(KeyCode::Char('q'))),
                Action::Quit
            ));

            Ok(())
        }

        #[test]
        fn bound_movement_action_is_dispatched_and_returns_none() -> Result<()> {
            // A configured binding resolves to Action::Move(Down); App dispatches it
            // to the component and returns None — the caller never sees the movement.
            let mut app = with_config(config_with_keybindings(
                [("j", Action::Move(Movement::Down))].as_slice(),
            )?);

            assert!(matches!(
                app.handle_key(press(KeyCode::Char('j'))),
                Action::None
            ));

            Ok(())
        }

        #[test]
        fn quit_action_propagates_to_caller() -> Result<()> {
            // Quit must still reach the event loop even when routed through dispatch.
            let mut app = with_config(config_with_keybindings([("q", Action::Quit)].as_slice())?);

            assert!(matches!(
                app.handle_key(press(KeyCode::Char('q'))),
                Action::Quit
            ));

            Ok(())
        }

        #[test]
        fn chord_break_retries_against_config_before_fallback() -> Result<()> {
            // "gg" makes 'g' a prefix; "<Tab>" is explicitly bound to None, overriding
            // the built-in fallback that cycles tabs.
            // When a chord breaks (g then Tab), Tab must be retried against config bindings
            // before falling through to the built-in handler — otherwise it bypasses the
            // explicit binding and cycles the active tab from 0 to 1.
            let mut app = with_config(config_with_keybindings(
                [
                    ("gg", Action::Move(Movement::GotoTop)),
                    ("<Tab>", Action::None),
                ]
                .as_slice(),
            )?);

            app.handle_key(press(KeyCode::Char('g'))); // prefix
            app.handle_key(press(KeyCode::Tab)); // breaks chord; retried as "<Tab>" binding

            assert_eq!(app.active, 0);

            Ok(())
        }
    }
}
