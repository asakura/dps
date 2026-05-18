//! ppO₂-by-depth table component.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Cell, Paragraph, Row, StatefulWidget, TableState, Widget},
};

use crate::{
    action::{Action, Movement},
    gas::Ean,
    theme::Theme,
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
#[derive(Debug, Clone, Copy)]
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
    #[must_use]
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
            .unwrap_or_else(|_| unreachable!("PPO2_TABLE_MIX_PERCENTS values are valid"))
    }

    /// Mix columns for a sliding window of `window_size` columns centred on the selected index.
    fn visible_cols(&self, window_size: usize) -> Vec<Ean> {
        let start = window_start(self.mix_idx, PPO2_TABLE_MIX_COUNT, window_size);
        let count = window_size.min(PPO2_TABLE_MIX_COUNT);
        (0..count)
            .map(|i| {
                Ean::from_percent(PPO2_TABLE_MIX_PERCENTS[start + i])
                    .unwrap_or_else(|_| unreachable!("PPO2_TABLE_MIX_PERCENTS values are valid"))
            })
            .collect()
    }

    /// Column index of the selected mix within the visible window (0-based).
    fn mix_window_col(&self, window_size: usize) -> usize {
        self.mix_idx - window_start(self.mix_idx, PPO2_TABLE_MIX_COUNT, window_size)
    }

    fn move_row(&mut self, delta: isize) {
        super::move_row(&mut self.table_state, delta, PPO2_TABLE_DEPTH_MAX);
    }

    fn move_up(&mut self) {
        self.move_row(-1);
    }

    fn move_down(&mut self) {
        self.move_row(1);
    }

    const fn move_left(&mut self) {
        self.mix_idx = self.mix_idx.saturating_sub(1);
    }

    fn move_right(&mut self) {
        self.mix_idx = (self.mix_idx + 1).min(PPO2_TABLE_MIX_COUNT - 1);
    }

    fn scroll_up(&mut self) {
        self.move_row(-super::SCROLL_DELTA);
    }

    fn scroll_down(&mut self) {
        self.move_row(super::SCROLL_DELTA);
    }

    fn page_up(&mut self) {
        self.move_row(-super::PAGE_DELTA);
    }

    fn page_down(&mut self) {
        self.move_row(super::PAGE_DELTA);
    }

    const fn goto_top(&mut self) {
        self.table_state.select(Some(0));
    }

    const fn goto_bottom(&mut self) {
        self.table_state.select(Some(PPO2_TABLE_DEPTH_MAX));
    }

    fn handle_movement(&mut self, mv: Movement) {
        match mv {
            Movement::Up => self.move_up(),
            Movement::Down => self.move_down(),
            Movement::Left => self.move_left(),
            Movement::Right => self.move_right(),
            Movement::ScrollUp => self.scroll_up(),
            Movement::ScrollDown => self.scroll_down(),
            Movement::PageUp => self.page_up(),
            Movement::PageDown => self.page_down(),
            Movement::GotoTop => self.goto_top(),
            Movement::GotoBottom => self.goto_bottom(),
        }
    }

    fn build_rows(mixes: &[Ean], theme: Theme) -> Vec<Row<'static>> {
        (0..=PPO2_TABLE_DEPTH_MAX)
            .map(|d| {
                PpO2Row {
                    depth: d,
                    mixes,
                    theme,
                }
                .into()
            })
            .collect()
    }
}

struct PpO2Row<'a> {
    depth: usize,
    mixes: &'a [Ean],
    theme: Theme,
}

impl From<PpO2Row<'_>> for Row<'static> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "depth is bounded by PPO2_TABLE_DEPTH_MAX = 80"
    )]
    fn from(r: PpO2Row<'_>) -> Self {
        let depth = Meters::new(r.depth as f64);
        let mut cells = vec![Cell::from(format!("{:>3} m", r.depth))];
        for mix in r.mixes {
            let ppo2 = mix.ppo2_at(depth);
            cells.push(
                Cell::from(format!("{:.2}", ppo2.value()))
                    .style(ppo2_cell_color(ppo2.value(), &r.theme)),
            );
        }
        Row::new(cells)
    }
}

fn ppo2_cell_color(ppo2: f64, theme: &Theme) -> Style {
    if !(PPO2_HYPOXIC_BELOW..PPO2_DANGER_FROM).contains(&ppo2) {
        theme.danger()
    } else if ppo2 >= PPO2_CAUTION_FROM {
        theme.caution()
    } else {
        theme.safe()
    }
}

struct PpO2TabStatus<'a> {
    tab: &'a PpO2Tab,
    theme: Theme,
}

impl Widget for PpO2TabStatus<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.tab.selection {
            Some((depth, mix)) => {
                let ppo2 = mix.ppo2_at(depth);
                let name = mix.label().map(|s| format!("{s} ")).unwrap_or_default();
                let text = format!(
                    " \u{25c6} {}({}%)  @ {}  \u{2192}  ppO\u{2082} {:.2} bar",
                    name,
                    mix.o2_percent(),
                    depth,
                    ppo2.value(),
                );
                Paragraph::new(text)
                    .style(self.theme.status_active())
                    .render(area, buf);
            }
            None => Paragraph::new(" No depth selected — press Enter to select")
                .style(self.theme.status_empty())
                .render(area, buf),
        }
    }
}

impl Component for PpO2Tab {
    fn title(&self) -> &'static str {
        "ppO₂ by Depth"
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "depth_m is bounded by PPO2_TABLE_DEPTH_MAX = 80"
    )]
    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Move(mv) => self.handle_movement(mv),
            Action::Select => {
                if let Some(depth_m) = self.table_state.selected() {
                    self.selection = Some((Meters::new(depth_m as f64), self.selected_mix()));
                }
            }
            Action::Quit | Action::None => {}
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme) {
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
            vec![Cell::from("Depth").style(Theme::header_cell())],
            mixes.iter().map(|m| format!("{:>3}%", m.o2_percent())),
            Some(col_in_window),
            theme,
        );
        let table = styled_table(
            Self::build_rows(&mixes, *theme),
            constraints,
            header,
            title,
            theme,
        );
        StatefulWidget::render(table, area, buf, &mut self.table_state);
    }

    fn render_status(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        PpO2TabStatus {
            tab: self,
            theme: *theme,
        }
        .render(area, buf);
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
    use crate::components::test_utils::widget_text;

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

    mod select_action {
        use super::*;
        use crate::action::Movement;

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
            tab.handle_action(Action::Move(Movement::Down));
            tab.handle_action(Action::Select);
            let second_depth = tab.selection.unwrap().0.value();
            assert_ne!(first_depth, second_depth);
        }
    }

    mod ppo2_cell_color {
        use super::*;

        #[test]
        fn hypoxic_below_threshold_is_red() {
            assert_eq!(
                ppo2_cell_color(0.10, &Theme::default()),
                Theme::default().danger()
            );
        }

        #[test]
        fn at_hypoxic_threshold_is_green() {
            assert_eq!(
                ppo2_cell_color(0.18, &Theme::default()),
                Theme::default().safe()
            );
        }

        #[test]
        fn normal_range_is_green() {
            assert_eq!(
                ppo2_cell_color(1.0, &Theme::default()),
                Theme::default().safe()
            );
        }

        #[test]
        fn at_caution_threshold_is_yellow() {
            assert_eq!(
                ppo2_cell_color(1.4, &Theme::default()),
                Theme::default().caution()
            );
        }

        #[test]
        fn caution_range_is_yellow() {
            assert_eq!(
                ppo2_cell_color(1.5, &Theme::default()),
                Theme::default().caution()
            );
        }

        #[test]
        fn at_danger_threshold_is_red() {
            assert_eq!(
                ppo2_cell_color(1.6, &Theme::default()),
                Theme::default().danger()
            );
        }

        #[test]
        fn above_danger_is_red() {
            assert_eq!(
                ppo2_cell_color(2.0, &Theme::default()),
                Theme::default().danger()
            );
        }
    }

    mod status_bar {
        use super::*;
        use crate::action::Movement;

        #[test]
        fn no_selection_shows_prompt() {
            let tab = PpO2Tab::new();
            let text = widget_text(
                PpO2TabStatus {
                    tab: &tab,
                    theme: Theme::default(),
                },
                60,
            );
            assert!(text.contains("No depth selected"));
        }

        #[test]
        fn selection_shows_depth_mix_and_ppo2() {
            let mut tab = PpO2Tab::new();
            for _ in 0..10 {
                tab.handle_action(Action::Move(Movement::Down));
            }
            tab.handle_action(Action::Select);
            let text = widget_text(
                PpO2TabStatus {
                    tab: &tab,
                    theme: Theme::default(),
                },
                80,
            );
            assert!(text.contains("10.0 m"));
            assert!(text.contains("21")); // Air (EAN21)
        }
    }

    mod action_dispatch {
        use super::*;
        use crate::action::{Action, Movement};
        use crate::components::{PAGE_DELTA, SCROLL_DELTA};

        #[test]
        fn down_advances_depth() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Move(Movement::Down));
            assert_eq!(tab.table_state.selected().unwrap(), 1);
        }

        #[test]
        fn down_clamped_at_max_depth() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Move(Movement::GotoBottom));
            tab.handle_action(Action::Move(Movement::Down));
            assert_eq!(tab.table_state.selected().unwrap(), PPO2_TABLE_DEPTH_MAX);
        }

        #[test]
        fn up_retreats_depth() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Move(Movement::Down));
            let after = tab.table_state.selected().unwrap();
            tab.handle_action(Action::Move(Movement::Up));
            assert_eq!(tab.table_state.selected().unwrap(), after - 1);
        }

        #[test]
        fn up_at_zero_stays_at_zero() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Move(Movement::Up));
            assert_eq!(tab.table_state.selected().unwrap(), 0);
        }

        #[test]
        fn goto_top_selects_depth_zero() {
            let mut tab = PpO2Tab::new();
            for _ in 0..10 {
                tab.handle_action(Action::Move(Movement::Down));
            }
            tab.handle_action(Action::Move(Movement::GotoTop));
            assert_eq!(tab.table_state.selected().unwrap(), 0);
        }

        #[test]
        fn goto_bottom_selects_max_depth() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Move(Movement::GotoBottom));
            assert_eq!(tab.table_state.selected().unwrap(), PPO2_TABLE_DEPTH_MAX);
        }

        #[test]
        fn scroll_down_moves_by_delta() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Move(Movement::ScrollDown));
            assert_eq!(tab.table_state.selected().unwrap(), SCROLL_DELTA as usize,);
        }

        #[test]
        fn scroll_up_moves_by_delta() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Move(Movement::GotoBottom));
            tab.handle_action(Action::Move(Movement::ScrollUp));
            assert_eq!(
                tab.table_state.selected().unwrap(),
                PPO2_TABLE_DEPTH_MAX - SCROLL_DELTA as usize,
            );
        }

        #[test]
        fn page_down_moves_by_page_delta() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Move(Movement::PageDown));
            assert_eq!(tab.table_state.selected().unwrap(), PAGE_DELTA as usize,);
        }

        #[test]
        fn page_up_moves_by_page_delta() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Move(Movement::GotoBottom));
            tab.handle_action(Action::Move(Movement::PageUp));
            assert_eq!(
                tab.table_state.selected().unwrap(),
                PPO2_TABLE_DEPTH_MAX - PAGE_DELTA as usize,
            );
        }

        #[test]
        fn right_increments_mix() {
            let mut tab = PpO2Tab::new();
            let before = tab.mix_idx;
            tab.handle_action(Action::Move(Movement::Right));
            assert_eq!(tab.mix_idx, before + 1);
        }

        #[test]
        fn right_clamped_at_last_mix() {
            let mut tab = PpO2Tab::new();
            for _ in 0..=PPO2_TABLE_MIX_COUNT {
                tab.handle_action(Action::Move(Movement::Right));
            }
            assert_eq!(tab.mix_idx, PPO2_TABLE_MIX_COUNT - 1);
        }

        #[test]
        fn left_decrements_mix() {
            let mut tab = PpO2Tab::new();
            tab.handle_action(Action::Move(Movement::Right));
            let before = tab.mix_idx;
            tab.handle_action(Action::Move(Movement::Left));
            assert_eq!(tab.mix_idx, before - 1);
        }

        #[test]
        fn left_clamped_at_zero_mix() {
            let mut tab = PpO2Tab::new();
            for _ in 0..=PPO2_MIX_DEFAULT_IDX {
                tab.handle_action(Action::Move(Movement::Left));
            }
            assert_eq!(tab.mix_idx, 0);
        }

        #[test]
        fn none_is_a_noop() {
            let mut tab = PpO2Tab::new();
            let before = tab.table_state.selected().unwrap();
            tab.handle_action(Action::None);
            assert_eq!(tab.table_state.selected().unwrap(), before);
        }

        #[test]
        fn quit_is_a_noop() {
            let mut tab = PpO2Tab::new();
            let before = tab.table_state.selected().unwrap();
            tab.handle_action(Action::Quit);
            assert_eq!(tab.table_state.selected().unwrap(), before);
        }
    }
}
