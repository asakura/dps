//! Application state and tab routing.

use std::{fmt, path::Path};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Paragraph, Tabs, Widget},
};

use crate::{
    action::Action,
    components::{Component, KeyBinding, mod_tab::ModTab, ppo2_tab::PpO2Tab, which_key::WhichKey},
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
    key_buffer: Vec<KeyEvent>,
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

enum MatchResult {
    Exact(Action),
    /// Buffer is a prefix of at least one binding; keep accumulating.
    Prefix,
    NoMatch,
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
            key_buffer: Vec::new(),
            mode: Mode::Home,
            tick_rate,
            frame_rate,
            theme,
        }
    }

    /// Runs the event loop until the user quits.
    ///
    /// Enters the terminal (raw mode + alternate screen), then drives
    /// tick/render intervals and key input until [`Action::Quit`] is received
    /// or the event stream closes. Terminal state is restored on return.
    ///
    /// # Errors
    ///
    /// Propagates I/O errors from terminal setup ([`Tui::new`], [`Tui::enter`]),
    /// frame rendering, and terminal teardown.
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

        loop {
            match tui.next_event().await {
                Some(Event::Render) => {
                    tui.draw(|f| self.render(f))?;
                }
                Some(Event::Key(key)) => {
                    if matches!(self.handle_key(key), Action::Quit) {
                        break;
                    }
                }
                Some(Event::Quit | Event::Closed) | None => break,
                Some(
                    Event::Error
                    | Event::Init
                    | Event::Tick
                    | Event::FocusGained
                    | Event::FocusLost
                    | Event::Paste(_)
                    | Event::Mouse(_)
                    | Event::Resize(_, _),
                ) => {}
            }
        }

        Ok(())
    }

    /// Checks `key_buffer` against the current mode's bindings.
    fn match_sequence(&self) -> MatchResult {
        let Some(bindings) = self.keybindings.0.get(&self.mode) else {
            return MatchResult::NoMatch;
        };

        let mut exact: Option<Action> = None;
        let mut has_prefix = false;

        for (seq, action) in bindings {
            if seq.as_slice() == self.key_buffer.as_slice() {
                exact = Some(*action);
            } else if seq.starts_with(&self.key_buffer) {
                has_prefix = true;
            }
        }

        match (exact, has_prefix) {
            (Some(action), _) => MatchResult::Exact(action),
            (None, true) => MatchResult::Prefix,
            (None, false) => MatchResult::NoMatch,
        }
    }

    /// Appends `key` to the pending buffer and attempts to match a configured
    /// binding.
    ///
    /// - **Exact match** — clears the buffer and returns the bound [`Action`].
    /// - **Prefix match** — returns [`Action::None`] and keeps accumulating.
    /// - **No match** — clears the buffer and falls back: if the failed
    ///   sequence was a chord, the latest key is retried alone (it may start a
    ///   new sequence); otherwise the hardcoded global bindings are consulted.
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        self.key_buffer.push(key);

        match self.match_sequence() {
            MatchResult::Exact(action) => {
                self.key_buffer.clear();
                self.dispatch(action)
            }
            MatchResult::Prefix => Action::None,
            MatchResult::NoMatch => {
                let was_chord = self.key_buffer.len() > 1;
                self.key_buffer.clear();

                if was_chord {
                    // Retry the key that broke the chord as the start of a new sequence.
                    self.key_buffer.push(key);

                    match self.match_sequence() {
                        MatchResult::Exact(action) => {
                            self.key_buffer.clear();
                            return self.dispatch(action);
                        }
                        MatchResult::Prefix => return Action::None,
                        MatchResult::NoMatch => self.key_buffer.clear(),
                    }
                }

                self.handle_key_fallback(key)
            }
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
            other @ (Action::Move(_) | Action::Select | Action::None) => {
                self.tabs[self.active].handle_action(other);
                Action::None
            }
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

/// One-line hint bar showing component bindings followed by global bindings.
struct HintBar<'a> {
    component: &'a [KeyBinding],
    global: &'a [KeyBinding],
    theme: Theme,
}

impl Widget for HintBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let hint = self
            .component
            .iter()
            .chain(self.global.iter())
            .map(|b| format!("{} {}", b.key, b.desc))
            .collect::<Vec<_>>()
            .join("   ");
        Paragraph::new(format!(" {hint}"))
            .style(self.theme.hint())
            .render(area, buf);
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

        HintBar {
            component: self.tabs[self.active].key_bindings(),
            global: GLOBAL_BINDINGS,
            theme: self.theme,
        }
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
    use crate::components::test_utils::widget_text;
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
                home_map.insert(parse_key_sequence(seq_str)?, *action);
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
        fn chord_first_key_is_prefix() -> Result<()> {
            let mut app = with_config(config_with_keybindings(
                [("gg", Action::Move(Movement::GotoTop))].as_slice(),
            )?);

            assert!(matches!(
                app.handle_key(press(KeyCode::Char('g'))),
                Action::None
            ));

            Ok(())
        }

        #[test]
        fn chord_completes_on_second_key() -> Result<()> {
            // Action is dispatched to the component; caller sees None.
            let mut app = with_config(config_with_keybindings(
                [("gg", Action::Move(Movement::GotoTop))].as_slice(),
            )?);

            app.handle_key(press(KeyCode::Char('g')));

            assert!(matches!(
                app.handle_key(press(KeyCode::Char('g'))),
                Action::None
            ));

            Ok(())
        }

        #[test]
        fn chord_broken_key_retried_as_new_binding() -> Result<()> {
            // "g" is a prefix of "gg"; when "j" breaks the chord it is retried
            // as a standalone key, matched as Down, dispatched, and None returned.
            let mut app = with_config(config_with_keybindings(
                [
                    ("gg", Action::Move(Movement::GotoTop)),
                    ("j", Action::Move(Movement::Down)),
                ]
                .as_slice(),
            )?);

            app.handle_key(press(KeyCode::Char('g')));

            assert!(matches!(
                app.handle_key(press(KeyCode::Char('j'))),
                Action::None
            ));

            Ok(())
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
        fn exact_match_clears_buffer_for_next_chord() -> Result<()> {
            // After a chord fires the buffer is cleared; the next key starts fresh.
            let mut app = with_config(config_with_keybindings(
                [("gg", Action::Move(Movement::GotoTop))].as_slice(),
            )?);

            app.handle_key(press(KeyCode::Char('g')));
            app.handle_key(press(KeyCode::Char('g'))); // exact → GotoTop, buffer cleared

            // 'g' is a prefix again — should return None, not misfire.
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('g'))),
                Action::None
            ));

            Ok(())
        }

        #[test]
        fn three_key_chord_accumulates_and_fires() -> Result<()> {
            let mut app = with_config(config_with_keybindings(
                [("abc", Action::Move(Movement::GotoTop))].as_slice(),
            )?);

            assert!(matches!(
                app.handle_key(press(KeyCode::Char('a'))),
                Action::None
            ));
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('b'))),
                Action::None
            ));
            // Action dispatched to component; caller sees None.
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('c'))),
                Action::None
            ));

            Ok(())
        }

        #[test]
        fn broken_chord_retry_starts_new_prefix() -> Result<()> {
            // "gg" and "jk" are bound. Pressing g (prefix) then j breaks gg;
            // j is retried alone and is a prefix of jk, so None is returned and
            // the buffer still holds j. Pressing k then completes jk.
            let mut app = with_config(config_with_keybindings(
                [
                    ("gg", Action::Move(Movement::GotoTop)),
                    ("jk", Action::Move(Movement::ScrollUp)),
                ]
                .as_slice(),
            )?);

            app.handle_key(press(KeyCode::Char('g'))); // prefix

            assert!(matches!(
                app.handle_key(press(KeyCode::Char('j'))), // breaks gg, j → prefix of jk
                Action::None
            ));
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('k'))), // completes jk → dispatched, returns None
                Action::None
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

    mod hint_bar {
        use super::*;
        use color_eyre::eyre::eyre;

        static COMP: &[KeyBinding] = [KeyBinding {
            key: "j/k",
            desc: "move",
        }]
        .as_slice();
        static GLOB: &[KeyBinding] = [KeyBinding {
            key: "q",
            desc: "quit",
        }]
        .as_slice();

        #[test]
        fn renders_component_bindings_first() -> Result<()> {
            let text = widget_text(
                HintBar {
                    component: COMP,
                    global: GLOB,
                    theme: Theme::default(),
                },
                60,
            );
            let j_pos = text
                .find("j/k")
                .ok_or_else(|| eyre!("'j/k' not found in hint bar text"))?;
            let q_pos = text
                .find("q quit")
                .ok_or_else(|| eyre!("'q quit' not found in hint bar text"))?;

            assert!(j_pos < q_pos);

            Ok(())
        }

        #[test]
        fn renders_all_bindings() {
            let text = widget_text(
                HintBar {
                    component: COMP,
                    global: GLOB,
                    theme: Theme::default(),
                },
                60,
            );
            assert!(text.contains("j/k move"));
            assert!(text.contains("q quit"));
        }

        #[test]
        fn empty_bindings_renders_without_panic() {
            widget_text(
                HintBar {
                    component: [].as_slice(),
                    global: [].as_slice(),
                    theme: Theme::default(),
                },
                40,
            );
        }
    }
}
