//! ppO₂-by-depth table component.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::Color,
    widgets::{Cell, Paragraph, Row, StatefulWidget, TableState, Widget},
};

use crate::{
    action::Action,
    gas::Ean,
    theme::THEME,
    ui::{build_header_row, col_window_size, styled_table, trailing_constraints, window_start},
    units::Meters,
};

use super::{Component, KeyBinding};

const PPO2_TABLE_MIX_PERCENTS: &[u8] = &[10, 12, 14, 16, 18, 21, 28, 30, 32, 36, 40, 50, 80, 100];
const PPO2_TABLE_MIX_COUNT: usize = PPO2_TABLE_MIX_PERCENTS.len();
const PPO2_TABLE_DEPTH_MAX: usize = 80;
const PPO2_MIX_DEFAULT_IDX: usize = 5; // EAN21 (Air)

const FIXED_COL_COUNT: usize = 1;
const COL_DEPTH_W: u16 = 7;
const COL_PPO2_MIX_W: u16 = 7;
const PPO2_TABLE_OVERHEAD_W: u16 = 2 + 2 + COL_DEPTH_W + 1;

const PPO2_HYPOXIC_BELOW: f64 = 0.18;
const PPO2_CAUTION_FROM: f64 = 1.4;
const PPO2_DANGER_FROM: f64 = 1.6;

/// ppO₂-by-depth table: partial pressure of oxygen for each mix at each depth.
pub struct PpO2Tab {
    table_state: TableState,
    mix_idx: usize,
    selection: Option<(Meters, Ean)>,
}

impl Default for PpO2Tab {
    fn default() -> Self {
        Self::new()
    }
}

impl PpO2Tab {
    /// Creates a `PpO2Tab` pre-selected on Air (21%) at 0 m depth.
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            table_state,
            mix_idx: PPO2_MIX_DEFAULT_IDX,
            selection: None,
        }
    }

    fn selected_mix(&self) -> Ean {
        Ean::from_percent(PPO2_TABLE_MIX_PERCENTS[self.mix_idx])
            .expect("PPO2_TABLE_MIX_PERCENTS values are valid")
    }

    /// Mix columns for a sliding window of `window_size` columns centred on the selected index.
    fn visible_cols(&self, window_size: usize) -> Vec<Ean> {
        let start = window_start(self.mix_idx, PPO2_TABLE_MIX_COUNT, window_size);
        let count = window_size.min(PPO2_TABLE_MIX_COUNT);
        (0..count)
            .map(|i| {
                Ean::from_percent(PPO2_TABLE_MIX_PERCENTS[start + i])
                    .expect("PPO2_TABLE_MIX_PERCENTS values are valid")
            })
            .collect()
    }

    /// Column index of the selected mix within the visible window (0-based).
    fn mix_window_col(&self, window_size: usize) -> usize {
        self.mix_idx - window_start(self.mix_idx, PPO2_TABLE_MIX_COUNT, window_size)
    }

    fn move_row(&mut self, delta: isize) {
        let next = self
            .table_state
            .selected()
            .map(|i| (i as isize + delta).clamp(0, PPO2_TABLE_DEPTH_MAX as isize) as usize)
            .unwrap_or(0);
        self.table_state.select(Some(next));
    }

    fn build_rows(mixes: &[Ean]) -> Vec<Row<'static>> {
        (0..=PPO2_TABLE_DEPTH_MAX)
            .map(|d| PpO2Row { depth: d, mixes }.into())
            .collect()
    }
}

struct PpO2Row<'a> {
    depth: usize,
    mixes: &'a [Ean],
}

impl From<PpO2Row<'_>> for Row<'static> {
    fn from(r: PpO2Row<'_>) -> Row<'static> {
        let depth = Meters::new(r.depth as f64);
        let mut cells = vec![Cell::from(format!("{:>3} m", r.depth))];
        for mix in r.mixes {
            let ppo2 = mix.ppo2_at(depth);
            cells.push(
                Cell::from(format!("{:.2}", ppo2.value())).style(ppo2_cell_color(ppo2.value())),
            );
        }
        Row::new(cells)
    }
}

fn ppo2_cell_color(ppo2: f64) -> Color {
    if !(PPO2_HYPOXIC_BELOW..PPO2_DANGER_FROM).contains(&ppo2) {
        THEME.red
    } else if ppo2 >= PPO2_CAUTION_FROM {
        THEME.yellow
    } else {
        THEME.green
    }
}

impl Widget for &mut PpO2Tab {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let window_size = col_window_size(
            area.width,
            PPO2_TABLE_OVERHEAD_W,
            COL_PPO2_MIX_W,
            PPO2_TABLE_MIX_COUNT,
        );
        let col_in_window = self.mix_window_col(window_size);
        self.table_state
            .select_column(Some(col_in_window + FIXED_COL_COUNT));
        let mixes = self.visible_cols(window_size);
        let mix = self.selected_mix();
        let title = format!(" DPS — ppO\u{2082} by Depth   {}% ", mix.o2_percent());
        let constraints = trailing_constraints(
            &[Constraint::Length(COL_DEPTH_W)],
            mixes.len(),
            COL_PPO2_MIX_W,
        );
        let header = build_header_row(
            vec![Cell::from("Depth").style(THEME.header_cell())],
            mixes.iter().map(|m| format!("{:>3}%", m.o2_percent())),
            Some(col_in_window),
        );
        let table = styled_table(PpO2Tab::build_rows(&mixes), constraints, header, title);
        StatefulWidget::render(table, area, buf, &mut self.table_state);
    }
}

struct PpO2TabStatus<'a>(&'a PpO2Tab);

impl Widget for PpO2TabStatus<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let depth_m = self.0.table_state.selected().unwrap_or(0);
        let mix = self.0.selected_mix();
        let depth = Meters::new(depth_m as f64);
        let ppo2 = mix.ppo2_at(depth);
        let name = mix.label().map(|s| format!("{} ", s)).unwrap_or_default();
        let text = format!(
            " \u{25c6} {}({}%)  @ {} m  \u{2192}  ppO\u{2082} {:.2} bar",
            name,
            mix.o2_percent(),
            depth_m,
            ppo2.value(),
        );
        Paragraph::new(text)
            .style(THEME.status_active())
            .render(area, buf);
    }
}

const SCROLL_DELTA: isize = 10;
const PAGE_DELTA: isize = 20;

impl Component for PpO2Tab {
    fn title(&self) -> &'static str {
        "ppO₂ by Depth"
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Down => self.move_row(1),
            Action::Up => self.move_row(-1),
            Action::Right => self.mix_idx = (self.mix_idx + 1).min(PPO2_TABLE_MIX_COUNT - 1),
            Action::Left => self.mix_idx = self.mix_idx.saturating_sub(1),
            Action::ScrollDown => self.move_row(SCROLL_DELTA),
            Action::ScrollUp => self.move_row(-SCROLL_DELTA),
            Action::PageDown => self.move_row(PAGE_DELTA),
            Action::PageUp => self.move_row(-PAGE_DELTA),
            Action::GotoTop => self.table_state.select(Some(0)),
            Action::GotoBottom => self.table_state.select(Some(PPO2_TABLE_DEPTH_MAX)),
            Action::Select => {
                if let Some(depth_m) = self.table_state.selected() {
                    self.selection = Some((Meters::new(depth_m as f64), self.selected_mix()));
                }
            }
            _ => {}
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        Widget::render(self, area, buf);
    }

    fn render_status(&self, area: Rect, buf: &mut Buffer) {
        PpO2TabStatus(self).render(area, buf);
    }

    fn key_bindings(&self) -> &'static [KeyBinding] {
        static BINDINGS: &[KeyBinding] = &[
            KeyBinding {
                key: "j/k",
                desc: "navigate depth",
            },
            KeyBinding {
                key: "h/l",
                desc: "change mix",
            },
        ];
        BINDINGS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn widget_text(widget: impl Widget, width: u16) -> String {
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);
        buf.content.iter().map(|c| c.symbol()).collect()
    }

    mod initial_state {
        use super::*;

        #[test]
        fn selected_depth_is_zero() {
            let tab = PpO2Tab::new();
            assert_eq!(tab.table_state.selected().unwrap(), 0);
        }

        #[test]
        fn selected_mix_is_air() {
            let tab = PpO2Tab::new();
            assert_eq!(tab.selected_mix().o2_percent(), 21);
        }

        #[test]
        fn no_selection() {
            assert!(PpO2Tab::new().selection.is_none());
        }
    }

    mod enter_key {
        use super::*;

        #[test]
        fn stores_current_depth_and_mix() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Select);
            let (depth, mix) = tab.selection.unwrap();
            assert!((depth.value() - 0.0).abs() < 1e-9);
            assert_eq!(mix.o2_percent(), 21);
        }

        #[test]
        fn selection_updates_after_moving_row() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Select);
            let first_depth = tab.selection.unwrap().0.value();
            tab.handle_action(Action::Down);
            tab.handle_action(Action::Select);
            let second_depth = tab.selection.unwrap().0.value();
            assert_ne!(first_depth, second_depth);
        }
    }

    mod ppo2_cell_color_fn {
        use super::*;

        #[test]
        fn hypoxic_below_threshold_is_red() {
            assert_eq!(ppo2_cell_color(0.10), THEME.red);
        }

        #[test]
        fn at_hypoxic_threshold_is_green() {
            assert_eq!(ppo2_cell_color(0.18), THEME.green);
        }

        #[test]
        fn normal_range_is_green() {
            assert_eq!(ppo2_cell_color(1.0), THEME.green);
        }

        #[test]
        fn at_caution_threshold_is_yellow() {
            assert_eq!(ppo2_cell_color(1.4), THEME.yellow);
        }

        #[test]
        fn caution_range_is_yellow() {
            assert_eq!(ppo2_cell_color(1.5), THEME.yellow);
        }

        #[test]
        fn at_danger_threshold_is_red() {
            assert_eq!(ppo2_cell_color(1.6), THEME.red);
        }

        #[test]
        fn above_danger_is_red() {
            assert_eq!(ppo2_cell_color(2.0), THEME.red);
        }
    }

    mod status_bar_fn {
        use super::*;

        #[test]
        fn shows_air_at_surface() {
            let tab = PpO2Tab::new();
            let text = widget_text(PpO2TabStatus(&tab), 60);
            assert!(text.contains("21"));
            assert!(text.contains("@ 0 m"));
        }

        #[test]
        fn shows_updated_depth_after_navigation() {
            let mut tab = PpO2Tab::new();
            for _ in 0..10 {
                tab.handle_action(Action::Down);
            }
            let text = widget_text(PpO2TabStatus(&tab), 60);
            assert!(text.contains("@ 10 m"));
        }
    }

    mod action_dispatch {
        use super::*;
        use crate::action::Action;

        #[test]
        fn down_advances_depth() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Down);
            assert_eq!(tab.table_state.selected().unwrap(), 1);
        }

        #[test]
        fn down_clamped_at_max_depth() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::GotoBottom);
            tab.handle_action(Action::Down);
            assert_eq!(tab.table_state.selected().unwrap(), PPO2_TABLE_DEPTH_MAX);
        }

        #[test]
        fn up_at_zero_stays_at_zero() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Up);
            assert_eq!(tab.table_state.selected().unwrap(), 0);
        }

        #[test]
        fn goto_top_selects_depth_zero() {
            let mut tab = PpO2Tab::new();
            for _ in 0..10 {
                tab.handle_action(Action::Down);
            }
            tab.handle_action(Action::GotoTop);
            assert_eq!(tab.table_state.selected().unwrap(), 0);
        }

        #[test]
        fn goto_bottom_selects_max_depth() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::GotoBottom);
            assert_eq!(tab.table_state.selected().unwrap(), PPO2_TABLE_DEPTH_MAX);
        }

        #[test]
        fn scroll_down_moves_by_delta() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::ScrollDown);
            assert_eq!(
                tab.table_state.selected().unwrap(),
                SCROLL_DELTA as usize,
            );
        }

        #[test]
        fn scroll_up_moves_by_delta() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::GotoBottom);
            tab.handle_action(Action::ScrollUp);
            assert_eq!(
                tab.table_state.selected().unwrap(),
                PPO2_TABLE_DEPTH_MAX - SCROLL_DELTA as usize,
            );
        }

        #[test]
        fn page_down_moves_by_page_delta() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::PageDown);
            assert_eq!(
                tab.table_state.selected().unwrap(),
                PAGE_DELTA as usize,
            );
        }

        #[test]
        fn page_up_moves_by_page_delta() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::GotoBottom);
            tab.handle_action(Action::PageUp);
            assert_eq!(
                tab.table_state.selected().unwrap(),
                PPO2_TABLE_DEPTH_MAX - PAGE_DELTA as usize,
            );
        }

        #[test]
        fn right_increments_mix() {
            let mut tab = PpO2Tab::new();
            let before = tab.mix_idx;
            tab.handle_action(Action::Right);
            assert_eq!(tab.mix_idx, before + 1);
        }

        #[test]
        fn right_clamped_at_last_mix() {
            let mut tab = PpO2Tab::new();
            for _ in 0..=PPO2_TABLE_MIX_COUNT {
                tab.handle_action(Action::Right);
            }
            assert_eq!(tab.mix_idx, PPO2_TABLE_MIX_COUNT - 1);
        }

        #[test]
        fn left_decrements_mix() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Right);
            let before = tab.mix_idx;
            tab.handle_action(Action::Left);
            assert_eq!(tab.mix_idx, before - 1);
        }

        #[test]
        fn left_clamped_at_zero_mix() {
            let mut tab = PpO2Tab::new();
            for _ in 0..=PPO2_MIX_DEFAULT_IDX {
                tab.handle_action(Action::Left);
            }
            assert_eq!(tab.mix_idx, 0);
        }
    }
}
