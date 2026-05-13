//! Component trait and per-screen implementations.

pub mod mod_tab;
pub mod ppo2_tab;
pub mod which_key;

use crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Paragraph};

use crate::action::Action;

/// A single key→action entry for the which-key popup and help line.
pub struct KeyBinding {
    pub key: &'static str,
    pub desc: &'static str,
}

/// Interface that every screen must implement to participate in the event loop
/// and render pipeline.
pub trait Component {
    /// Short display name shown in the tab bar.
    fn title(&self) -> &'static str;
    /// Process a key press and return the resulting action.
    fn handle_key(&mut self, key: KeyEvent) -> Action;
    /// Draw the component's content into `area`.
    fn render(&mut self, area: Rect, buf: &mut Buffer);
    /// One-line status paragraph rendered below the main content area.
    fn status_bar(&self) -> Paragraph<'static>;
    /// Structured key bindings for the which-key popup and hint line.
    fn key_bindings(&self) -> &'static [KeyBinding] { &[] }
}
