//! Application state and tab routing.

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
    theme::THEME,
};

static GLOBAL_BINDINGS: &[KeyBinding] = &[
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
];

/// Top-level coordinator: owns the tab list, tracks the active tab, and routes
/// key events and render calls to the appropriate component.
pub struct App {
    tabs: Vec<Box<dyn Component>>,
    active: usize,
    show_which_key: bool,
    keybindings: KeyBindings,
    key_buffer: Vec<KeyEvent>,
    mode: Mode,
}

enum MatchResult {
    Exact(Action),
    /// Buffer is a prefix of at least one binding; keep accumulating.
    Prefix,
    NoMatch,
}


impl Default for App {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl App {
    /// Creates an `App` pre-loaded with all tabs in their default state.
    pub fn new(config: Config) -> Self {
        Self {
            tabs: vec![Box::new(ModTab::new()), Box::new(PpO2Tab::new())],
            active: 0,
            show_which_key: false,
            keybindings: config.keybindings,
            key_buffer: Vec::new(),
            mode: Mode::Home,
        }
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
                exact = Some(action.clone());
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
    pub fn render(&mut self, f: &mut Frame) {
        f.render_widget(self, f.area());
    }
}

/// One-line hint bar showing component bindings followed by global bindings.
struct HintBar<'a> {
    component: &'a [KeyBinding],
    global: &'a [KeyBinding],
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
            .style(THEME.hint())
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
            .style(THEME.nav_bar())
            .highlight_style(THEME.selection())
            .divider("│")
            .render(chunks[0], buf);

        self.tabs[self.active].render(chunks[1], buf);
        self.tabs[self.active].render_status(chunks[2], buf);

        HintBar {
            component: self.tabs[self.active].key_bindings(),
            global: GLOBAL_BINDINGS,
        }
        .render(chunks[3], buf);

        if self.show_which_key {
            WhichKey::new(GLOBAL_BINDINGS, self.tabs[self.active].key_bindings()).render(area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    use crate::components::test_utils::widget_text;

    mod handle_key {
        use super::*;
        use crate::action::Movement;
        use crate::config::{AppConfig, Config, KeyBindings, Styles};
        use crate::config::keys::parse_key_sequence;
        use std::collections::HashMap;

        fn config_with_keybindings(bindings: &[(&str, Action)]) -> Config {
            let mut home_map = HashMap::new();
            for (seq_str, action) in bindings {
                home_map.insert(parse_key_sequence(seq_str).unwrap(), action.clone());
            }
            let mut mode_map = HashMap::new();
            mode_map.insert(Mode::Home, home_map);
            Config {
                config: AppConfig::default(),
                keybindings: KeyBindings(mode_map),
                styles: Styles(),
            }
        }

        #[test]
        fn q_quits() {
            assert!(matches!(
                App::new(Config::default()).handle_key(press(KeyCode::Char('q'))),
                Action::Quit
            ));
        }

        #[test]
        fn esc_quits() {
            assert!(matches!(
                App::new(Config::default()).handle_key(press(KeyCode::Esc)),
                Action::Quit
            ));
        }

        #[test]
        fn question_mark_toggles_which_key() {
            let mut app = App::new(Config::default());
            assert!(!app.show_which_key);
            app.handle_key(press(KeyCode::Char('?')));
            assert!(app.show_which_key);
            app.handle_key(press(KeyCode::Char('?')));
            assert!(!app.show_which_key);
        }

        #[test]
        fn tab_cycles_active() {
            let mut app = App::new(Config::default());
            assert_eq!(app.active, 0);
            app.handle_key(press(KeyCode::Tab));
            assert_eq!(app.active, 1);
            app.handle_key(press(KeyCode::Tab));
            assert_eq!(app.active, 0);
        }

        #[test]
        fn other_keys_return_none() {
            assert!(matches!(
                App::new(Config::default()).handle_key(press(KeyCode::Char('j'))),
                Action::None
            ));
        }

        #[test]
        fn chord_first_key_is_prefix() {
            let mut app = App::new(config_with_keybindings(&[("gg", Action::Move(Movement::GotoTop))]));
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('g'))),
                Action::None
            ));
        }

        #[test]
        fn chord_completes_on_second_key() {
            // Action is dispatched to the component; caller sees None.
            let mut app = App::new(config_with_keybindings(&[("gg", Action::Move(Movement::GotoTop))]));
            app.handle_key(press(KeyCode::Char('g')));
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('g'))),
                Action::None
            ));
        }

        #[test]
        fn chord_broken_key_retried_as_new_binding() {
            // "g" is a prefix of "gg"; when "j" breaks the chord it is retried
            // as a standalone key, matched as Down, dispatched, and None returned.
            let mut app = App::new(config_with_keybindings(&[
                ("gg", Action::Move(Movement::GotoTop)),
                ("j", Action::Move(Movement::Down)),
            ]));
            app.handle_key(press(KeyCode::Char('g')));
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('j'))),
                Action::None
            ));
        }

        #[test]
        fn chord_broken_unbound_key_falls_to_global_fallback() {
            // "g" is a prefix of "gg"; "q" breaks the chord and has no
            // configured binding, so the hardcoded fallback fires: q → Quit.
            let mut app = App::new(config_with_keybindings(&[("gg", Action::Move(Movement::GotoTop))]));
            app.handle_key(press(KeyCode::Char('g')));
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('q'))),
                Action::Quit
            ));
        }

        #[test]
        fn exact_match_clears_buffer_for_next_chord() {
            // After a chord fires the buffer is cleared; the next key starts fresh.
            let mut app = App::new(config_with_keybindings(&[("gg", Action::Move(Movement::GotoTop))]));
            app.handle_key(press(KeyCode::Char('g')));
            app.handle_key(press(KeyCode::Char('g'))); // exact → GotoTop, buffer cleared
            // 'g' is a prefix again — should return None, not misfire.
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('g'))),
                Action::None
            ));
        }

        #[test]
        fn three_key_chord_accumulates_and_fires() {
            let mut app = App::new(config_with_keybindings(&[("abc", Action::Move(Movement::GotoTop))]));
            assert!(matches!(app.handle_key(press(KeyCode::Char('a'))), Action::None));
            assert!(matches!(app.handle_key(press(KeyCode::Char('b'))), Action::None));
            // Action dispatched to component; caller sees None.
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('c'))),
                Action::None
            ));
        }

        #[test]
        fn broken_chord_retry_starts_new_prefix() {
            // "gg" and "jk" are bound. Pressing g (prefix) then j breaks gg;
            // j is retried alone and is a prefix of jk, so None is returned and
            // the buffer still holds j. Pressing k then completes jk.
            let mut app = App::new(config_with_keybindings(&[
                ("gg", Action::Move(Movement::GotoTop)),
                ("jk", Action::Move(Movement::ScrollUp)),
            ]));
            app.handle_key(press(KeyCode::Char('g'))); // prefix
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('j'))), // breaks gg, j → prefix of jk
                Action::None
            ));
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('k'))), // completes jk → dispatched, returns None
                Action::None
            ));
        }

        #[test]
        fn bound_movement_action_is_dispatched_and_returns_none() {
            // A configured binding resolves to Action::Move(Down); App dispatches it
            // to the component and returns None — the caller never sees the movement.
            let mut app = App::new(config_with_keybindings(&[("j", Action::Move(Movement::Down))]));
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('j'))),
                Action::None
            ));
        }

        #[test]
        fn quit_action_propagates_to_caller() {
            // Quit must still reach the event loop even when routed through dispatch.
            let mut app = App::new(config_with_keybindings(&[("q", Action::Quit)]));
            assert!(matches!(
                app.handle_key(press(KeyCode::Char('q'))),
                Action::Quit
            ));
        }
    }

    mod hint_bar {
        use super::*;

        static COMP: &[KeyBinding] = &[KeyBinding {
            key: "j/k",
            desc: "move",
        }];
        static GLOB: &[KeyBinding] = &[KeyBinding {
            key: "q",
            desc: "quit",
        }];

        #[test]
        fn renders_component_bindings_first() {
            let text = widget_text(
                HintBar {
                    component: COMP,
                    global: GLOB,
                },
                60,
            );
            let j_pos = text.find("j/k").unwrap();
            let q_pos = text.find("q quit").unwrap();
            assert!(j_pos < q_pos);
        }

        #[test]
        fn renders_all_bindings() {
            let text = widget_text(
                HintBar {
                    component: COMP,
                    global: GLOB,
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
                    component: &[],
                    global: &[],
                },
                40,
            );
        }
    }
}
