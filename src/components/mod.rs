//! Component trait and per-screen implementations.

pub mod mod_tab;
pub mod ppo2_tab;

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect, widgets::Paragraph};

use crate::action::Action;

/// Interface that every screen must implement to participate in the event loop
/// and render pipeline.
pub trait Component {
    /// Short display name shown in the tab bar.
    fn title(&self) -> &'static str;
    /// Process a key press and return the resulting action.
    fn handle_key(&mut self, key: KeyEvent) -> Action;
    /// Draw the component's content into `area`.
    fn render(&mut self, f: &mut Frame, area: Rect);
    /// One-line status paragraph rendered below the main content area.
    fn status_bar(&self) -> Paragraph<'static>;
    /// Keybinding hint line rendered at the very bottom of the screen.
    fn help_text(&self) -> &'static str;
}
