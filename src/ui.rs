//! Shared rendering utilities used by all components.

use ratatui::{
    layout::Constraint,
    text::Span,
    widgets::{Block, Borders, Cell, Row, Table},
};

use crate::theme::Theme;

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
    fixed
        .iter()
        .copied()
        .chain(std::iter::repeat_n(
            Constraint::Length(col_w),
            n.saturating_sub(1),
        ))
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
    theme: &Theme,
) -> Table<'static> {
    Table::new(rows, constraints)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.border())
                .title(Span::styled(title, theme.title())),
        )
        .row_highlight_style(theme.selection())
        .column_highlight_style(theme.column_focus())
        .highlight_symbol("▶ ")
}

/// Header row from `fixed` cells and dynamic `labels`, with the `highlighted` column underlined.
pub(crate) fn build_header_row(
    fixed: Vec<Cell<'static>>,
    labels: impl Iterator<Item = String>,
    highlighted: Option<usize>,
    theme: &Theme,
) -> Row<'static> {
    let mut cells = fixed;

    for (i, label) in labels.enumerate() {
        let style = if highlighted == Some(i) {
            theme.header_cell_active()
        } else {
            theme.header_cell()
        };

        cells.push(Cell::from(label).style(style));
    }

    Row::new(cells).style(theme.header())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    enum TestError {
        #[error("{0}")]
        Assert(&'static str),
    }

    type Result<T> = std::result::Result<T, TestError>;

    mod window_start_fn {
        use super::*;

        #[test]
        fn at_start_stays_at_zero() {
            assert_eq!(window_start(0, 10, 3), 0);
        }

        #[test]
        fn centres_on_selected_index() {
            assert_eq!(window_start(5, 10, 3), 4);
        }

        #[test]
        fn clamps_at_end() {
            assert_eq!(window_start(9, 10, 3), 7);
        }

        #[test]
        fn full_window_always_starts_at_zero() {
            assert_eq!(window_start(5, 10, 10), 0);
        }

        #[test]
        fn even_window_uses_integer_half_not_remainder() {
            // window size 4: half = 4/2 = 2; 5.saturating_sub(2).min(6) = 3
            assert_eq!(window_start(5, 10, 4), 3);
        }
    }

    mod build_header_row_fn {
        use super::*;
        use ratatui::{
            buffer::Buffer,
            layout::{Position, Rect},
            style::Modifier,
            widgets::Widget,
        };

        #[test]
        fn highlighted_column_is_underlined_others_are_not() -> Result<()> {
            let header = build_header_row(
                vec![],
                ["A".to_string(), "B".to_string()].into_iter(),
                Some(0),
                &Theme::default(),
            );

            let table = Table::new(
                Vec::<Row<'static>>::new(),
                [Constraint::Length(3), Constraint::Length(3)],
            )
            .header(header);

            let area = Rect::new(0, 0, 6, 2);
            let mut buf = Buffer::empty(area);

            Widget::render(table, area, &mut buf);

            // Header is row 0; 'A' at x=0, 'B' at x=3.
            assert!(
                buf.cell(Position::new(0, 0))
                    .ok_or(TestError::Assert("cell out of bounds"))?
                    .modifier
                    .contains(Modifier::UNDERLINED)
            );
            assert!(
                !buf.cell(Position::new(3, 0))
                    .ok_or(TestError::Assert("cell out of bounds"))?
                    .modifier
                    .contains(Modifier::UNDERLINED)
            );

            Ok(())
        }
    }

    mod col_window_size_fn {
        use super::*;

        #[test]
        fn minimum_one_column() {
            assert_eq!(col_window_size(10, 10, 9, 20), 1);
        }

        #[test]
        fn counts_additional_columns_by_width() {
            // 1 + (80 - 10 - 9) / (9 + 1) = 1 + 6 = 7
            assert_eq!(col_window_size(80, 10, 9, 20), 7);
        }

        #[test]
        fn capped_at_max() {
            assert_eq!(col_window_size(80, 10, 9, 5), 5);
        }

        #[test]
        fn exactly_two_columns_fit() {
            // 1 + (29 - 10 - 9) / (9 + 1) = 1 + 1 = 2
            assert_eq!(col_window_size(29, 10, 9, 20), 2);
        }

        #[test]
        fn divides_by_col_w_plus_one_not_col_w() {
            // 1 + (20 - 2 - 4) / (4 + 1) = 1 + 2 = 3; the +1 accounts for the separator
            assert_eq!(col_window_size(20, 2, 4, 10), 3);
        }
    }

    mod trailing_constraints_fn {
        use super::*;

        #[test]
        fn zero_data_columns_returns_only_fixed() {
            let c = trailing_constraints([Constraint::Length(12)].as_slice(), 0, 9);
            assert_eq!(c, vec![Constraint::Length(12)]);
        }

        #[test]
        fn single_data_column_is_fill() {
            let c = trailing_constraints([].as_slice(), 1, 9);
            assert_eq!(c, vec![Constraint::Fill(1)]);
        }

        #[test]
        fn multiple_data_columns_last_is_fill() {
            let c = trailing_constraints([].as_slice(), 3, 9);
            assert_eq!(
                c,
                vec![
                    Constraint::Length(9),
                    Constraint::Length(9),
                    Constraint::Fill(1),
                ]
            );
        }

        #[test]
        fn fixed_columns_prepended() {
            let c = trailing_constraints(
                [Constraint::Length(12), Constraint::Length(6)].as_slice(),
                2,
                9,
            );
            assert_eq!(
                c,
                vec![
                    Constraint::Length(12),
                    Constraint::Length(6),
                    Constraint::Length(9),
                    Constraint::Fill(1),
                ]
            );
        }
    }
}
