//! Top-level application coordinator: owns components, drives the event loop.

use std::{fmt, path::Path};

use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::{
    action::Action,
    components::{ComponentNew, FpsCounter, Home},
    config::Config,
    keymap::{ChordEngine, ChordResult, Mode, ModeMap, SequenceEngine},
    tui::{Event, Tui},
};

/// Event-loop coordinator that owns a set of [`ComponentNew`] instances.
///
/// `App` drives the TUI using the channel-based action pipeline introduced
/// by the [`ComponentNew`] trait family.  Every component receives a clone of
/// the [`mpsc::UnboundedSender<Action>`] during initialisation so it can push
/// actions at any time; `App` owns the matching receiver and drains it on
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
pub struct App {
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

impl fmt::Debug for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("App")
            .field("tick_rate", &self.tick_rate)
            .field("frame_rate", &self.frame_rate)
            .field("mode", &self.mode)
            .finish_non_exhaustive()
    }
}

impl App {
    /// Creates an `App` with the given tick and frame rates, loading config from disk.
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
    /// use dps::app::App;
    /// let _app = App::new(4.0, 60.0, None, None)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`run`]: App::run
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
    /// - The loop acquires the next [`Tui`] event.  On Unix, it races
    ///   `SIGTERM` and `SIGINT` alongside `tui.next_event()`; either signal
    ///   produces [`Event::Quit`] immediately.
    /// - `handle_events` converts the event to an [`Action`] and fans it out
    ///   to all components.
    /// - `handle_actions` drains the action channel, applies infrastructure
    ///   actions, and forwards every action to each component's
    ///   [`ComponentNew::update`].
    ///
    /// **Suspension** (`Action::Suspend`):
    /// On Unix, the TUI is torn down, `SIGTSTP` is emitted so the OS actually
    /// suspends the process, and `Resume` + `ClearScreen` are enqueued so
    /// components react when the process wakes.  [`Tui::resume`] is called (not
    /// `enter`) to restore the terminal after `SIGCONT`.  On non-Unix platforms
    /// the suspend path exits and immediately re-enters the TUI without raising a
    /// signal.
    ///
    /// **Quit** (`Action::Quit`):
    /// The TUI is stopped and the loop breaks.  [`Tui::exit`] is called once
    /// more after the loop to guarantee terminal state is restored regardless of
    /// how the loop ended.
    ///
    /// # Errors
    ///
    /// Propagates errors from [`Tui::new`], [`Tui::enter`], [`Tui::exit`],
    /// [`Tui::stop`], terminal size queries, component initialisation,
    /// signal-handler registration (Unix), event handling, and rendering.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> color_eyre::Result<()> {
    /// use dps::app::App;
    /// let mut app = App::new(4.0, 60.0, None, None)?;
    /// app.run().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let mut tui = Tui::new()?
            .mouse(true)
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);

        tui.enter()?;

        let size = tui.size()?;
        for component in &mut self.components {
            component.register_action_handler(self.action_tx.clone())?;
            component.register_config_handler(self.config.clone())?;
            component.init(size)?;
        }

        #[cfg(unix)]
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        #[cfg(unix)]
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

        loop {
            #[cfg(unix)]
            let event = tokio::select! {
                event = tui.next_event() => event,
                _ = sigterm.recv() => Some(Event::Quit),
                _ = sigint.recv() => Some(Event::Quit),
            };
            #[cfg(not(unix))]
            let event = tui.next_event().await;

            self.handle_events(event)?;
            self.handle_actions(&mut tui)?;

            if self.should_suspend {
                self.should_suspend = false;
                tui.exit()?;
                #[cfg(unix)]
                signal_hook::low_level::emulate_default_handler(signal_hook::consts::SIGTSTP)?;
                self.action_tx.send(Action::Resume)?;
                self.action_tx.send(Action::ClearScreen)?;
                tui.resume()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }

        tui.exit()?;

        Ok(())
    }

    /// Converts a raw [`Event`] to an [`Action`] and fans it out to every component.
    ///
    /// Infrastructure events (`Quit`, `Tick`, `Render`, `Resize`) are converted
    /// to their matching [`Action`] variants and sent on the channel.  [`Key`]
    /// events are forwarded to [`handle_key_event`] for keymap lookup.  All
    /// other events are silently ignored at this layer but still passed to
    /// component [`handle_events`] handlers so components may react to them
    /// directly.
    ///
    /// Returns `Ok(())` immediately if `event` is `None`.
    ///
    /// [`Key`]: Event::Key
    /// [`handle_key_event`]: App::handle_key_event
    /// [`handle_events`]: ComponentNew::handle_events
    fn handle_events(&mut self, event: Option<Event>) -> color_eyre::Result<()> {
        let Some(event) = event else {
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

        if let ChordResult::Exact(action, count) = self.chord.advance(key, bindings) {
            let repeat = if action.accepts_count() { count } else { 1 };
            info!("Got action: {action:?} ×{repeat}");

            for _ in 0..repeat {
                self.action_tx.send(action.clone())?;
            }
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
    /// [`handle_resize`]: App::handle_resize
    /// [`render`]: App::render
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
    use std::collections::HashMap;

    use approx::assert_relative_eq;
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::Frame;
    use rstest::rstest;

    use super::*;
    use crate::{
        action::Movement,
        config::{AppConfig, Styles},
        keymap::{KeyBindingsBuilder, KeySeq, parse_key_sequence},
        theme::Theme,
        tui::Tui,
    };

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn make_app(config: Config) -> App {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        App {
            config,
            tick_rate: 4.0,
            frame_rate: 60.0,
            components: vec![],
            should_quit: false,
            should_suspend: false,
            mode: Mode::Normal,
            chord: SequenceEngine::default(),
            action_tx,
            action_rx,
        }
    }

    fn default_app() -> App {
        make_app(Config::default())
    }

    fn drain(app: &mut App) -> Vec<Action> {
        let mut out = vec![];
        while let Ok(a) = app.action_rx.try_recv() {
            out.push(a);
        }
        out
    }

    fn config_with_bindings(bindings: &[(&str, Action)]) -> color_eyre::Result<Config> {
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

    /// Returns a fixed action from `handle_events` for every event; no-op update.
    struct EventSpy(Option<Action>);

    impl ComponentNew for EventSpy {
        fn handle_events(&mut self, _: Option<Event>) -> crate::components::Result<Option<Action>> {
            Ok(self.0.clone())
        }

        fn draw(&mut self, _: &mut Frame<'_>, _: Rect) -> crate::components::Result<()> {
            Ok(())
        }
    }

    /// Returns `response` from `update` the first time it sees `trigger`; silent thereafter.
    struct UpdateSpy {
        trigger: Action,
        response: Action,
        fired: bool,
    }

    impl UpdateSpy {
        fn new(trigger: Action, response: Action) -> Self {
            Self {
                trigger,
                response,
                fired: false,
            }
        }
    }

    impl ComponentNew for UpdateSpy {
        fn update(&mut self, action: Action) -> crate::components::Result<Option<Action>> {
            if !self.fired && action == self.trigger {
                self.fired = true;
                return Ok(Some(self.response.clone()));
            }
            Ok(None)
        }

        fn draw(&mut self, _: &mut Frame<'_>, _: Rect) -> crate::components::Result<()> {
            Ok(())
        }
    }

    mod new {
        use super::*;

        #[test]
        fn succeeds_with_valid_rates() {
            assert!(App::new(4.0, 60.0, None, None).is_ok());
        }

        #[rstest]
        #[case(10.0, 30.0)]
        #[case(4.0, 60.0)]
        fn stores_tick_and_frame_rate(
            #[case] tick: f64,
            #[case] frame: f64,
        ) -> color_eyre::Result<()> {
            let app = App::new(tick, frame, None, None)?;

            assert_relative_eq!(app.tick_rate, tick);
            assert_relative_eq!(app.frame_rate, frame);

            Ok(())
        }

        #[test]
        fn starts_with_mode_normal() -> color_eyre::Result<()> {
            let app = App::new(4.0, 60.0, None, None)?;

            assert_eq!(app.mode, Mode::Normal);

            Ok(())
        }

        #[test]
        fn starts_with_flags_cleared() -> color_eyre::Result<()> {
            let app = App::new(4.0, 60.0, None, None)?;

            assert!(!app.should_quit);
            assert!(!app.should_suspend);

            Ok(())
        }
    }

    mod handle_events {
        use super::*;

        #[rstest]
        #[case(Event::Quit, Action::Quit)]
        #[case(Event::Tick, Action::Tick)]
        #[case(Event::Render, Action::Render)]
        fn infrastructure_event_enqueues_matching_action(
            #[case] event: Event,
            #[case] expected: Action,
        ) -> color_eyre::Result<()> {
            let mut app = default_app();

            app.handle_events(Some(event))?;

            assert_eq!(drain(&mut app), vec![expected]);

            Ok(())
        }

        #[test]
        fn resize_enqueues_resize_action() -> color_eyre::Result<()> {
            let mut app = default_app();

            app.handle_events(Some(Event::Resize(80, 24)))?;

            assert_eq!(drain(&mut app), vec![Action::Resize(80, 24)]);

            Ok(())
        }

        #[test]
        fn none_enqueues_nothing() -> color_eyre::Result<()> {
            let mut app = default_app();

            app.handle_events(None)?;

            assert!(drain(&mut app).is_empty());

            Ok(())
        }

        #[test]
        fn unbound_key_enqueues_nothing() -> color_eyre::Result<()> {
            let mut app = default_app();

            app.handle_events(Some(Event::Key(press(KeyCode::Char('z')))))?;

            assert!(drain(&mut app).is_empty());

            Ok(())
        }

        #[test]
        fn component_returned_action_is_enqueued() -> color_eyre::Result<()> {
            let mut app = default_app();
            // FocusGained has no built-in routing, so only the component contributes.
            app.components
                .push(Box::new(EventSpy(Some(Action::Select))));

            app.handle_events(Some(Event::FocusGained))?;

            assert_eq!(drain(&mut app), vec![Action::Select]);

            Ok(())
        }

        #[test]
        fn key_event_fans_out_to_component_after_keymap_lookup() -> color_eyre::Result<()> {
            let mut app = default_app();
            // Unbound key → handle_key_event enqueues nothing.
            // Component still receives the event and contributes Select.
            app.components
                .push(Box::new(EventSpy(Some(Action::Select))));

            app.handle_events(Some(Event::Key(press(KeyCode::Char('x')))))?;

            assert_eq!(drain(&mut app), vec![Action::Select]);

            Ok(())
        }
    }

    mod handle_key_event {
        use super::*;

        #[test]
        fn bound_key_enqueues_action() -> color_eyre::Result<()> {
            let mut app = make_app(config_with_bindings(&[(
                "j",
                Action::Move(Movement::Down),
            )])?);

            app.handle_key_event(press(KeyCode::Char('j')))?;

            assert_eq!(drain(&mut app), vec![Action::Move(Movement::Down)]);

            Ok(())
        }

        #[test]
        fn prefix_enqueues_nothing_until_chord_completes() -> color_eyre::Result<()> {
            let mut app = make_app(config_with_bindings(&[(
                "gg",
                Action::Move(Movement::GotoTop),
            )])?);

            app.handle_key_event(press(KeyCode::Char('g')))?;
            assert!(drain(&mut app).is_empty());

            app.handle_key_event(press(KeyCode::Char('g')))?;
            assert_eq!(drain(&mut app), vec![Action::Move(Movement::GotoTop)]);

            Ok(())
        }

        #[test]
        fn unbound_key_enqueues_nothing() -> color_eyre::Result<()> {
            let mut app = make_app(Config::default());

            app.handle_key_event(press(KeyCode::Char('q')))?;

            assert!(drain(&mut app).is_empty());

            Ok(())
        }

        #[test]
        fn chord_break_after_prefix_enqueues_nothing() -> color_eyre::Result<()> {
            let mut app = make_app(config_with_bindings(&[(
                "gg",
                Action::Move(Movement::GotoTop),
            )])?);

            app.handle_key_event(press(KeyCode::Char('g')))?; // prefix
            app.handle_key_event(press(KeyCode::Char('x')))?; // breaks chord — no binding

            assert!(drain(&mut app).is_empty());

            Ok(())
        }
    }

    mod handle_actions {
        use super::*;

        #[test]
        fn quit_sets_should_quit() -> color_eyre::Result<()> {
            let mut app = default_app();
            let mut tui = Tui::new()?;

            app.action_tx.send(Action::Quit)?;
            app.handle_actions(&mut tui)?;

            assert!(app.should_quit);

            Ok(())
        }

        #[test]
        fn suspend_sets_should_suspend() -> color_eyre::Result<()> {
            let mut app = default_app();
            let mut tui = Tui::new()?;

            app.action_tx.send(Action::Suspend)?;
            app.handle_actions(&mut tui)?;

            assert!(app.should_suspend);

            Ok(())
        }

        #[test]
        fn resume_clears_should_suspend() -> color_eyre::Result<()> {
            let mut app = default_app();
            let mut tui = Tui::new()?;
            app.should_suspend = true;

            app.action_tx.send(Action::Resume)?;
            app.handle_actions(&mut tui)?;

            assert!(!app.should_suspend);

            Ok(())
        }

        #[test]
        fn tick_leaves_flags_unchanged() -> color_eyre::Result<()> {
            let mut app = default_app();
            let mut tui = Tui::new()?;

            app.action_tx.send(Action::Tick)?;
            app.handle_actions(&mut tui)?;

            assert!(!app.should_quit);
            assert!(!app.should_suspend);

            Ok(())
        }

        #[test]
        fn processes_all_queued_actions_in_one_call() -> color_eyre::Result<()> {
            let mut app = default_app();
            let mut tui = Tui::new()?;

            app.action_tx.send(Action::Suspend)?;
            app.action_tx.send(Action::Resume)?;
            app.handle_actions(&mut tui)?;

            assert!(!app.should_suspend);

            Ok(())
        }

        #[test]
        fn component_returned_action_is_reenqueued_and_processed() -> color_eyre::Result<()> {
            let mut app = default_app();
            // On first Tick, spy returns Quit once; Quit is re-enqueued and processed.
            app.components
                .push(Box::new(UpdateSpy::new(Action::Tick, Action::Quit)));
            let mut tui = Tui::new()?;

            app.action_tx.send(Action::Tick)?;
            app.handle_actions(&mut tui)?;

            assert!(app.should_quit);

            Ok(())
        }
    }
}
