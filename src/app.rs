//! Application state and tab routing.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Paragraph, Tabs},
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
        if key.code == KeyCode::Char('?') {
            self.show_which_key = !self.show_which_key;
            return Action::None;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            KeyCode::Tab => {
                self.active = (self.active + 1) % self.tabs.len();
                Action::None
            }
            _ => self.tabs[self.active].handle_key(key),
        }
    }

    /// Draws the tab bar, active component, status bar, and help line.
    pub fn render(&mut self, f: &mut Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        let titles: Vec<&str> = self.tabs.iter().map(|t| t.title()).collect();
        f.render_widget(
            Tabs::new(titles)
                .select(self.active)
                .style(Style::default().bg(THEME.surface0).fg(THEME.subtext0))
                .highlight_style(
                    Style::default()
                        .bg(THEME.mauve)
                        .fg(THEME.base)
                        .add_modifier(Modifier::BOLD),
                )
                .divider("│"),
            chunks[0],
        );
        self.tabs[self.active].render(f, chunks[1]);
        f.render_widget(self.tabs[self.active].status_bar(), chunks[2]);
        let hint = self.tabs[self.active]
            .key_bindings()
            .iter()
            .chain(GLOBAL_BINDINGS.iter())
            .map(|b| format!("{} {}", b.key, b.desc))
            .collect::<Vec<_>>()
            .join("   ");
        f.render_widget(
            Paragraph::new(format!(" {hint}")).style(Style::default().fg(THEME.subtext0)),
            chunks[3],
        );
        if self.show_which_key {
            f.render_widget(
                WhichKey::new(GLOBAL_BINDINGS, self.tabs[self.active].key_bindings()),
                area,
            );
        }
    }
}
