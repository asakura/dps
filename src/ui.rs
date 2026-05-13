//! Shared rendering utilities used by all components.

use ratatui::{
    layout::Constraint,
    text::Span,
    widgets::{Block, Borders, Cell, Row, Table},
};

use crate::theme::THEME;

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
    fixed.iter().copied()
        .chain(std::iter::repeat_n(Constraint::Length(col_w), n.saturating_sub(1)))
        .chain((n > 0).then_some(Constraint::Fill(1)))
        .collect()
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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(THEME.border())
                .title(Span::styled(title, THEME.title())),
        )
        .row_highlight_style(THEME.selection())
        .column_highlight_style(THEME.column_focus())
        .highlight_symbol("▶ ")
}

/// Header row from `fixed` cells and dynamic `labels`, with the `highlighted` column underlined.
pub(crate) fn build_header_row(
    fixed: Vec<Cell<'static>>,
    labels: impl Iterator<Item = String>,
    highlighted: Option<usize>,
) -> Row<'static> {
    let mut cells = fixed;
    for (i, label) in labels.enumerate() {
        let style = if highlighted == Some(i) { THEME.header_cell_active() } else { THEME.header_cell() };
        cells.push(Cell::from(label).style(style));
    }
    Row::new(cells).style(THEME.header())
}
