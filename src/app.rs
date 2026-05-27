//! Application state and tab routing.

use std::{collections::HashMap, fmt, path::Path};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Tabs, Widget},
};

use crate::{
    action::Action,
    chord::{ChordEngine, ChordResult, SequenceEngine},
    components::{
        Component, KeyBinding, hint_bar::HintBar, mod_tab::ModTab, ppo2_tab::PpO2Tab,
        which_key::WhichKey,
    },
    config::{Config, KeyBindings},
    mode::Mode,
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
            mode: Mode::Home,
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
        static EMPTY: std::sync::LazyLock<HashMap<Vec<KeyEvent>, Action>> =
            std::sync::LazyLock::new(HashMap::new);
        let bindings = self.keybindings.0.get(&self.mode).unwrap_or(&EMPTY);

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
        use crate::action::Movement;
        use crate::config::keys::parse_key_sequence;
        use crate::config::{AppConfig, Config, KeyBindings, Styles};
        use std::collections::HashMap;

        fn with_config(config: Config) -> App {
            App::from_config(4.0, 60.0, config)
        }

        fn config_with_keybindings(bindings: &[(&str, Action)]) -> Result<Config> {
            let mut home_map = HashMap::new();

            for (seq_str, action) in bindings {
                home_map.insert(parse_key_sequence(seq_str)?, action.clone());
            }

            let mut mode_map = HashMap::new();

            mode_map.insert(Mode::Home, home_map);

            Ok(Config {
                config: AppConfig::default(),
                keybindings: KeyBindings(mode_map),
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
