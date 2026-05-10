//! Application state and tab routing.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::Paragraph,
};

use crate::{
    action::Action,
    components::{Component, mod_tab::ModTab, ppo2_tab::PpO2Tab},
};

pub struct App {
    tabs: Vec<Box<dyn Component>>,
    active: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            tabs: vec![Box::new(ModTab::new()), Box::new(PpO2Tab::new())],
            active: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
            KeyCode::Tab => {
                self.active = (self.active + 1) % self.tabs.len();
                Action::None
            }
            _ => self.tabs[self.active].handle_key(key),
        }
    }

    pub fn render(&mut self, f: &mut Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1), Constraint::Length(1)])
            .split(area);
        self.tabs[self.active].render(f, chunks[0]);
        f.render_widget(self.tabs[self.active].status_bar(), chunks[1]);
        f.render_widget(
            Paragraph::new(self.tabs[self.active].help_text())
                .style(Style::default().fg(Color::DarkGray)),
            chunks[2],
        );
    }
}
