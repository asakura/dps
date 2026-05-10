//! Shared rendering utilities used by all components.

use ratatui::{
    layout::Constraint,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};

/// First visible index that keeps `idx` centred in the window without scrolling past either end.
pub(crate) fn window_start(idx: usize, total: usize, window_size: usize) -> usize {
    let half = window_size / 2;
    let max_start = total.saturating_sub(window_size);
    idx.saturating_sub(half).min(max_start)
}

/// How many data columns of `col_w` fit in `width` given fixed `overhead` already consumed.
pub(crate) fn col_window_size(width: u16, overhead: u16, col_w: u16, max: usize) -> usize {
    let n = 1 + width.saturating_sub(overhead + col_w) / (col_w + 1);
    (n as usize).min(max)
}

/// Constraint list for `fixed` columns followed by `n` data columns of `col_w`;
/// the last data column uses `Fill(1)` to absorb leftover terminal width.
pub(crate) fn trailing_constraints(fixed: &[Constraint], n: usize, col_w: u16) -> Vec<Constraint> {
    let mut c = fixed.to_vec();
    for i in 0..n {
        c.push(if i + 1 < n { Constraint::Length(col_w) } else { Constraint::Fill(1) });
    }
    c
}

/// Wraps `rows` in the application's standard table style: bordered block, bold/dark-gray
/// row highlight, bold column highlight, and a `▶` cursor symbol.
pub(crate) fn styled_table(
    rows: Vec<Row<'static>>,
    constraints: Vec<Constraint>,
    header: Row<'static>,
    title: String,
) -> Table<'static> {
    Table::new(rows, constraints)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .column_highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ")
}

/// Header row from `fixed` cells and dynamic `labels`, with the `highlighted` column underlined.
pub(crate) fn build_header_row(
    fixed: Vec<Cell<'static>>,
    labels: impl Iterator<Item = String>,
    highlighted: Option<usize>,
) -> Row<'static> {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let bold_ul = Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    let mut cells = fixed;
    for (i, label) in labels.enumerate() {
        cells.push(Cell::from(label).style(if highlighted == Some(i) { bold_ul } else { bold }));
    }
    Row::new(cells).style(Style::default().fg(Color::Cyan))
}
