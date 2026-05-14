//! Component trait and per-screen implementations.

pub mod mod_tab;
pub mod ppo2_tab;
pub mod which_key;

use ratatui::{buffer::Buffer, layout::Rect};

use crate::action::Action;

/// A single key→action entry for the which-key popup and help line.
pub struct KeyBinding {
    /// Key label shown in the popup (e.g. `"j/k"`, `"Enter"`).
    pub key: &'static str,
    /// Short description of the action.
    pub desc: &'static str,
}

/// Interface that every screen must implement to participate in the event loop
/// and render pipeline.
pub trait Component {
    /// Short display name shown in the tab bar.
    fn title(&self) -> &'static str;
    /// Draw the component's content into `area`.
    fn render(&mut self, area: Rect, buf: &mut Buffer);
    /// Render a one-line status bar below the main content area.
    fn render_status(&self, area: Rect, buf: &mut Buffer);
    /// Respond to a semantic action produced by the keybinding layer.
    ///
    /// Called when a configured key sequence resolves to an [`Action`] before
    /// the raw-key fallback path is reached.  The default implementation is a
    /// no-op; components override it for the actions they support.
    fn handle_action(&mut self, _action: Action) {}

    /// Structured key bindings for the which-key popup and hint line.
    fn key_bindings(&self) -> &'static [KeyBinding] {
        &[]
    }
}

#[cfg(test)]
pub mod test_utils {
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

    pub fn widget_text(widget: impl Widget, width: u16) -> String {
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);
        buf.content.iter().map(|c| c.symbol()).collect()
    }
}
