//! Which-key popup: Magit-style transient panel at the bottom of the screen.
//!
//! Bindings are shown in a column-major grid. The column count grows with the
//! terminal width; each column is at least 20 cells wide (1 lead + 7 key + 2 gap +
//! 10 description). The panel spans the full terminal width and is anchored to the
//! bottom edge.
//!
//! # Example
//!
//! ```rust,ignore
//! frame.render_widget(
//!     WhichKey::new(GLOBAL_BINDINGS, component_bindings),
//!     frame.area(),
//! );
//! ```

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Clear, Paragraph, Widget},
};

use crate::theme::THEME;

use super::KeyBinding;

/// Key column width.
const KEY_W: u16 = 7;
/// Gap between key and description within one entry.
const ENTRY_GAP: u16 = 2;
/// Leading space before each entry.
const LEAD: u16 = 1;
/// Gap between columns.
const COL_GAP: u16 = 4;
/// Minimum description width used to calculate how many columns fit.
const MIN_DESC_W: u16 = 10;

/// Which-key popup widget.
///
/// Merges `global` and `component` bindings into a dynamic column grid anchored
/// to the bottom of the area it is rendered into. Pass the full terminal area so
/// the popup is positioned at the screen bottom.
pub struct WhichKey {
    global: &'static [KeyBinding],
    component: &'static [KeyBinding],
}

impl WhichKey {
    /// Creates a new `WhichKey` popup combining global and component bindings.
    pub fn new(global: &'static [KeyBinding], component: &'static [KeyBinding]) -> Self {
        Self { global, component }
    }
}

impl Widget for WhichKey {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let all: Vec<&KeyBinding> = self.global.iter().chain(self.component.iter()).collect();
        let n = all.len();
        if n == 0 {
            return;
        }

        // How many columns fit? Each column needs at least min_col_w cells; columns
        // after the first each cost an additional COL_GAP.
        let min_col_w = LEAD + KEY_W + ENTRY_GAP + MIN_DESC_W;
        let cols = ((area.width + COL_GAP) / (min_col_w + COL_GAP)).max(1) as usize;
        let rows = n.div_ceil(cols);

        let popup_h = (rows as u16).min(area.height);
        let popup = bottom_rect(popup_h, area);

        Clear.render(popup, buf);
        buf.set_style(popup, Style::default().bg(THEME.mantle));

        let col_rects = Layout::horizontal(vec![Constraint::Fill(1); cols])
            .spacing(COL_GAP)
            .split(popup);

        for (col_idx, col_rect) in col_rects.iter().enumerate() {
            let row_rects = Layout::vertical(vec![Constraint::Length(1); rows]).split(*col_rect);
            for row in 0..rows {
                if let Some(b) = all.get(col_idx * rows + row) {
                    render_entry(b, row_rects[row], buf);
                }
            }
        }
    }
}

/// Renders a single binding into `area` using a [`LEAD` | `KEY_W` | `ENTRY_GAP` | desc] horizontal layout.
fn render_entry(b: &KeyBinding, area: Rect, buf: &mut Buffer) {
    let [_lead, key_area, _gap, desc_area] = Layout::horizontal([
        Constraint::Length(LEAD),
        Constraint::Length(KEY_W),
        Constraint::Length(ENTRY_GAP),
        Constraint::Fill(1),
    ])
    .areas(area);

    Paragraph::new(b.key)
        .style(Style::default().fg(THEME.peach).add_modifier(Modifier::BOLD))
        .render(key_area, buf);

    Paragraph::new(b.desc)
        .style(Style::default().fg(THEME.text))
        .render(desc_area, buf);
}

/// A `Rect` spanning the full width of `area`, `height` rows tall, anchored to the bottom edge.
fn bottom_rect(height: u16, area: Rect) -> Rect {
    Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(height),
        width: area.width,
        height: height.min(area.height),
    }
}
