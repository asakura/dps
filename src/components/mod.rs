//! Component trait and per-screen implementations.

pub mod mod_tab;
pub mod ppo2_tab;

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect, widgets::Paragraph};

use crate::action::Action;

pub trait Component {
    fn handle_key(&mut self, key: KeyEvent) -> Action;
    fn render(&mut self, f: &mut Frame, area: Rect);
    fn status_bar(&self) -> Paragraph<'static>;
    fn help_text(&self) -> &'static str;
}
