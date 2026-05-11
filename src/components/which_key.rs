//! Which-key popup: Magit-style transient panel at the bottom of the screen.
//!
//! Bindings are shown in a 2-column grid (column-major order). The panel spans
//! the full terminal width and is anchored to the bottom edge.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};

use crate::theme::THEME;

use super::KeyBinding;

/// Key column width (all keys are ASCII so byte == display width).
const KEY_W: usize = 7;
/// Gap between key and description within one entry.
const ENTRY_GAP: usize = 2;
/// Leading space before each entry.
const LEAD: usize = 1;
/// Gap between the two columns.
const COL_GAP: usize = 4;

pub fn render(
    f: &mut Frame,
    global: &'static [KeyBinding],
    component: &'static [KeyBinding],
) {
    let area = f.area();

    let all: Vec<&KeyBinding> = global.iter().chain(component.iter()).collect();

    let n = all.len();
    let rows = (n + 1) / 2;

    let inner_w = area.width as usize;
    let col_w = inner_w.saturating_sub(COL_GAP) / 2;
    let desc_w = col_w.saturating_sub(LEAD + KEY_W + ENTRY_GAP);

    let lines: Vec<Line<'static>> = (0..rows)
        .map(|row| {
            let mut spans = entry_spans(all.get(row), KEY_W, ENTRY_GAP, LEAD, col_w, desc_w);
            spans.push(Span::raw(format!("{:COL_GAP$}", "")));
            spans.extend(entry_spans(
                all.get(row + rows),
                KEY_W, ENTRY_GAP, 0, col_w, desc_w,
            ));
            Line::from(spans)
        })
        .collect();

    let popup_h = (rows as u16).min(area.height);
    let popup = bottom_rect(popup_h, area);

    f.render_widget(Clear, popup);
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(THEME.mantle)),
        popup,
    );
}

/// Spans for a single binding entry, or blank padding if the slot is empty.
fn entry_spans(
    binding: Option<&&KeyBinding>,
    key_w: usize,
    gap: usize,
    lead: usize,
    col_w: usize,
    desc_w: usize,
) -> Vec<Span<'static>> {
    match binding {
        Some(b) => {
            // Pad by char count, not bytes, so multi-byte chars like ₂ don't
            // shift the right column.
            let desc_pad = desc_w.saturating_sub(b.desc.chars().count());
            vec![
                Span::raw(format!("{:lead$}", "")),
                Span::styled(
                    format!("{:<key_w$}", b.key),
                    Style::default().fg(THEME.peach).add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{:gap$}", "")),
                Span::styled(b.desc, Style::default().fg(THEME.text)),
                Span::raw(" ".repeat(desc_pad)),
            ]
        }
        None => vec![Span::raw(format!("{:col_w$}", ""))],
    }
}

fn bottom_rect(height: u16, area: Rect) -> Rect {
    Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(height),
        width: area.width,
        height: height.min(area.height),
    }
}
