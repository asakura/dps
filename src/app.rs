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
    theme::THEME,
};

static GLOBAL_BINDINGS: &[KeyBinding] = &[
    KeyBinding { key: "Tab",   desc: "next table"      },
    KeyBinding { key: "q/Esc", desc: "quit"            },
    KeyBinding { key: "?",     desc: "toggle bindings" },
];

/// Top-level coordinator: owns the tab list, tracks the active tab, and routes
/// key events and render calls to the appropriate component.
pub struct App {
    tabs: Vec<Box<dyn Component>>,
    active: usize,
    show_which_key: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates an `App` pre-loaded with all tabs in their default state.
    pub fn new() -> Self {
        Self {
            tabs: vec![Box::new(ModTab::new()), Box::new(PpO2Tab::new())],
            active: 0,
            show_which_key: false,
        }
    }

    /// Intercepts `?` (which-key toggle), `q`/Esc (quit), and Tab (cycle tabs)
    /// globally; delegates all other keys to the active component.
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('?') => { self.show_which_key = !self.show_which_key; Action::None }
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            KeyCode::Tab => { self.active = (self.active + 1) % self.tabs.len(); Action::None }
            _ => self.tabs[self.active].handle_key(key),
        }
    }

    /// Draws the tab bar, active component, status bar, and help line.
    pub fn render(&mut self, f: &mut Frame) {
        f.render_widget(self, f.area());
    }
}

struct HintBar<'a> {
    component: &'a [KeyBinding],
    global: &'a [KeyBinding],
}

impl Widget for HintBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let hint = self.component.iter()
            .chain(self.global.iter())
            .map(|b| format!("{} {}", b.key, b.desc))
            .collect::<Vec<_>>()
            .join("   ");
        Paragraph::new(format!(" {hint}")).style(THEME.hint()).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyModifiers};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn widget_text(widget: impl Widget, width: u16) -> String {
        let area = ratatui::layout::Rect::new(0, 0, width, 1);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        widget.render(area, &mut buf);
        buf.content.iter().map(|c| c.symbol()).collect()
    }

    mod handle_key {
        use super::*;

        #[test]
        fn q_quits() {
            assert!(matches!(App::new().handle_key(press(KeyCode::Char('q'))), Action::Quit));
        }

        #[test]
        fn esc_quits() {
            assert!(matches!(App::new().handle_key(press(KeyCode::Esc)), Action::Quit));
        }

        #[test]
        fn question_mark_toggles_which_key() {
            let mut app = App::new();
            assert!(!app.show_which_key);
            app.handle_key(press(KeyCode::Char('?')));
            assert!(app.show_which_key);
            app.handle_key(press(KeyCode::Char('?')));
            assert!(!app.show_which_key);
        }

        #[test]
        fn tab_cycles_active() {
            let mut app = App::new();
            assert_eq!(app.active, 0);
            app.handle_key(press(KeyCode::Tab));
            assert_eq!(app.active, 1);
            app.handle_key(press(KeyCode::Tab));
            assert_eq!(app.active, 0);
        }

        #[test]
        fn other_keys_return_none() {
            assert!(matches!(App::new().handle_key(press(KeyCode::Char('j'))), Action::None));
        }
    }

    mod hint_bar {
        use super::*;

        static COMP: &[KeyBinding] = &[KeyBinding { key: "j/k", desc: "move" }];
        static GLOB: &[KeyBinding] = &[KeyBinding { key: "q",   desc: "quit" }];

        #[test]
        fn renders_component_bindings_first() {
            let text = widget_text(HintBar { component: COMP, global: GLOB }, 60);
            let j_pos = text.find("j/k").unwrap();
            let q_pos = text.find("q quit").unwrap();
            assert!(j_pos < q_pos);
        }

        #[test]
        fn renders_all_bindings() {
            let text = widget_text(HintBar { component: COMP, global: GLOB }, 60);
            assert!(text.contains("j/k move"));
            assert!(text.contains("q quit"));
        }

        #[test]
        fn empty_bindings_renders_without_panic() {
            widget_text(HintBar { component: &[], global: &[] }, 40);
        }
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

        HintBar { component: self.tabs[self.active].key_bindings(), global: GLOBAL_BINDINGS }
            .render(chunks[3], buf);

        if self.show_which_key {
            WhichKey::new(GLOBAL_BINDINGS, self.tabs[self.active].key_bindings())
                .render(area, buf);
        }
    }
}
