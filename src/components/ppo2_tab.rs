//! ppO₂-by-depth table component.

use super::{Component, KeyBinding};

use crate::{
    action::{Action, Movement},
    gas::EANx,
    registers::RegisterStore,
    theme::Theme,
    ui::{build_header_row, col_window_size, styled_table, trailing_constraints, window_start},
    units::{Bar, Meters, Percent},
};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Cell, Paragraph, Row, StatefulWidget, TableState, Widget},
};

const PPO2_TABLE_MIX_PERCENTS: &[Percent] = [
    Percent::literal(0.10),
    Percent::literal(0.12),
    Percent::literal(0.14),
    Percent::literal(0.16),
    Percent::literal(0.18),
    Percent::literal(0.21),
    Percent::literal(0.28),
    Percent::literal(0.30),
    Percent::literal(0.32),
    Percent::literal(0.36),
    Percent::literal(0.40),
    Percent::literal(0.50),
    Percent::literal(0.80),
    Percent::literal(1.00),
]
.as_slice();

const PPO2_MIX_DEFAULT_IDX: usize = 5; // EAN21 (Air)
const PPO2_TABLE_DEPTH_MAX: usize = 80;
const PPO2_TABLE_MIX_COUNT: usize = PPO2_TABLE_MIX_PERCENTS.len();
const PPO2_TABLE_OVERHEAD_W: u16 = 2 + 2 + COL_DEPTH_W + 1;

const PPO2_CAUTION_FROM: Bar = Bar::new(1.4);
const PPO2_DANGER_FROM: Bar = Bar::new(1.6);
const PPO2_HYPOXIC_BELOW: Bar = Bar::new(0.18);

const COL_DEPTH_W: u16 = 7;
const COL_PPO2_MIX_W: u16 = 7;
const FIXED_COL_COUNT: usize = 1;

/// ppO₂-by-depth table: partial pressure of oxygen for each mix at each depth.
#[derive(Debug, Clone, Copy)]
pub struct PpO2Tab {
    table_state: TableState,
    mix_idx: usize,
    selection: Option<(Meters, EANx)>,
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

    fn selected_mix(&self) -> EANx {
        EANx::try_from(PPO2_TABLE_MIX_PERCENTS[self.mix_idx])
            .unwrap_or_else(|_| unreachable!("PPO2_TABLE_MIX_PERCENTS values are valid"))
    }

    /// Mix columns for a sliding window of `window_size` columns centred on the selected index.
    fn visible_cols(&self, window_size: usize) -> Vec<EANx> {
        let start = window_start(self.mix_idx, PPO2_TABLE_MIX_COUNT, window_size);
        let count = window_size.min(PPO2_TABLE_MIX_COUNT);

        (0..count)
            .map(|i| {
                EANx::try_from(PPO2_TABLE_MIX_PERCENTS[start + i])
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
            // TODO: implement single-line scroll; for now falls back to one-row cursor move
            Movement::Up | Movement::LineUp => self.move_up(),
            Movement::Down | Movement::LineDown => self.move_down(),
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

    fn build_rows(mixes: &[EANx], theme: &Theme) -> Vec<Row<'static>> {
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
    mixes: &'a [EANx],
    theme: &'a Theme,
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
            let ppo2 = mix.ppo2_at(depth).pressure();

            cells.push(
                Cell::from(format!("{:.2}", f64::from(ppo2))).style(ppo2_cell_color(ppo2, r.theme)),
            );
        }

        Row::new(cells)
    }
}

fn ppo2_cell_color(ppo2: Bar, theme: &Theme) -> Style {
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
                let ppo2 = mix.ppo2_at(depth).pressure();
                let text = format!(
                    " \u{25c6} {}  @ {}  \u{2192}  ppO\u{2082} {:.2} bar",
                    mix,
                    depth,
                    f64::from(ppo2),
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
    fn handle_action(&mut self, action: Action, _registers: &mut RegisterStore) {
        match action {
            Action::Move(mv) => self.handle_movement(mv),
            Action::Select => {
                if let Some(depth_m) = self.table_state.selected() {
                    self.selection = Some((Meters::new(depth_m as f64), self.selected_mix()));
                }
            }
            _ => {}
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
        let title = format!(" DPS — ppO\u{2082} by Depth   {} ", mix.fo2());

        let constraints = trailing_constraints(
            [Constraint::Length(COL_DEPTH_W)].as_slice(),
            mixes.len(),
            COL_PPO2_MIX_W,
        );

        let header = build_header_row(
            vec![Cell::from("Depth").style(theme.header_cell())],
            mixes.iter().map(|m| m.fo2().to_string()),
            Some(col_in_window),
            theme,
        );

        let table = styled_table(
            Self::build_rows(&mixes, theme),
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
        static BINDINGS: &[KeyBinding] = [
            KeyBinding {
                key: "j/k",
                desc: "navigate depth",
            },
            KeyBinding {
                key: "h/l",
                desc: "change mix",
            },
        ]
        .as_slice();

        BINDINGS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::action::{Action, Movement};
    use crate::components::test_utils::widget_text;
    use crate::components::{PAGE_DELTA, SCROLL_DELTA};

    use rstest::rstest;

    mod constants {
        use super::*;

        #[rstest]
        fn ppo2_table_overhead_w_is_twelve() {
            // 2 + 2 + 7(COL_DEPTH_W) + 1 = 12
            assert_eq!(PPO2_TABLE_OVERHEAD_W, 12);
        }
    }

    mod visible_cols_fn {
        use super::*;

        #[rstest]
        fn full_window_returns_all_fourteen_mixes() {
            let tab = PpO2Tab::new();
            assert_eq!(tab.visible_cols(20).len(), 14);
        }

        #[rstest]
        fn returns_mixes_at_correct_offsets_from_start() {
            let mut tab = PpO2Tab::new();

            // mix_idx: 5 → 6
            tab.handle_action(Action::Move(Movement::Right), &mut RegisterStore::default());

            // window_start(6, 14, 3) = 5; percents at indices [5],[6],[7]
            let cols = tab.visible_cols(3);

            assert_eq!(cols[0].fo2(), Percent::literal(0.21)); // [5]
            assert_eq!(cols[1].fo2(), Percent::literal(0.28)); // [6]
            assert_eq!(cols[2].fo2(), Percent::literal(0.30)); // [7]
        }
    }

    mod mix_window_col_fn {
        use super::*;

        #[rstest]
        fn at_max_mix_idx_with_small_window() {
            let mut tab = PpO2Tab::new();

            for _ in 0..PPO2_TABLE_MIX_COUNT {
                tab.handle_action(Action::Move(Movement::Right), &mut RegisterStore::default());
            }

            // = 13
            assert_eq!(tab.mix_idx, PPO2_TABLE_MIX_COUNT - 1);
            // window_start(13, 14, 3): half=1, max_start=11, (13-1).min(11)=11 → col=13-11=2
            assert_eq!(tab.mix_window_col(3), 2);
        }
    }

    mod component_trait {
        use super::*;

        #[rstest]
        fn title_is_correct() {
            assert_eq!(PpO2Tab::new().title(), "ppO\u{2082} by Depth");
        }

        #[rstest]
        fn key_bindings_is_non_empty() {
            assert!(!PpO2Tab::new().key_bindings().is_empty());
        }
    }

    mod initial_state {
        use super::*;

        #[rstest]
        fn selected_depth_is_zero() {
            assert_eq!(PpO2Tab::new().table_state.selected(), Some(0));
        }

        #[rstest]
        fn selected_mix_is_air() {
            assert_eq!(PpO2Tab::new().selected_mix().fo2(), Percent::literal(0.21));
        }

        #[rstest]
        fn no_selection() {
            assert!(PpO2Tab::new().selection.is_none());
        }
    }

    mod select_action {
        use super::*;

        #[rstest]
        fn stores_current_depth_and_mix() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(Action::Select, &mut RegisterStore::default());

            assert_eq!(tab.selection.map(|(d, _)| d), Some(Meters::new(0.0)));
            assert_eq!(
                tab.selection.map(|(_, m)| m.fo2()),
                Some(Percent::literal(0.21))
            );
        }

        #[rstest]
        fn selection_updates_after_moving_row() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(Action::Select, &mut RegisterStore::default());
            let first_depth = tab.selection.map(|(d, _)| d);

            tab.handle_action(Action::Move(Movement::Down), &mut RegisterStore::default());
            tab.handle_action(Action::Select, &mut RegisterStore::default());

            assert_ne!(tab.selection.map(|(d, _)| d), first_depth);
        }
    }

    mod ppo2_cell_color {
        use super::*;

        #[rstest]
        fn hypoxic_below_threshold_is_red() {
            assert_eq!(
                ppo2_cell_color(Bar::new(0.10), &Theme::default()),
                Theme::default().danger()
            );
        }

        #[rstest]
        fn at_hypoxic_threshold_is_green() {
            assert_eq!(
                ppo2_cell_color(Bar::new(0.18), &Theme::default()),
                Theme::default().safe()
            );
        }

        #[rstest]
        fn normal_range_is_green() {
            assert_eq!(
                ppo2_cell_color(Bar::new(1.0), &Theme::default()),
                Theme::default().safe()
            );
        }

        #[rstest]
        fn at_caution_threshold_is_yellow() {
            assert_eq!(
                ppo2_cell_color(Bar::new(1.4), &Theme::default()),
                Theme::default().caution()
            );
        }

        #[rstest]
        fn caution_range_is_yellow() {
            assert_eq!(
                ppo2_cell_color(Bar::new(1.5), &Theme::default()),
                Theme::default().caution()
            );
        }

        #[rstest]
        fn at_danger_threshold_is_red() {
            assert_eq!(
                ppo2_cell_color(Bar::new(1.6), &Theme::default()),
                Theme::default().danger()
            );
        }

        #[rstest]
        fn above_danger_is_red() {
            assert_eq!(
                ppo2_cell_color(Bar::new(2.0), &Theme::default()),
                Theme::default().danger()
            );
        }
    }

    mod status_bar {
        use super::*;

        #[rstest]
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

        #[rstest]
        fn selection_shows_depth_mix_and_ppo2() {
            let mut tab = PpO2Tab::new();

            for _ in 0..10 {
                tab.handle_action(Action::Move(Movement::Down), &mut RegisterStore::default());
            }

            tab.handle_action(Action::Select, &mut RegisterStore::default());

            let text = widget_text(
                PpO2TabStatus {
                    tab: &tab,
                    theme: Theme::default(),
                },
                80,
            );

            assert!(text.contains("10.0 m"));
            assert!(text.contains("Air"));
        }
    }

    mod action_dispatch {
        use super::*;

        #[rstest]
        fn down_advances_depth() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(Action::Move(Movement::Down), &mut RegisterStore::default());

            assert_eq!(tab.table_state.selected(), Some(1));
        }

        #[rstest]
        fn down_clamped_at_max_depth() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(
                Action::Move(Movement::GotoBottom),
                &mut RegisterStore::default(),
            );
            tab.handle_action(Action::Move(Movement::Down), &mut RegisterStore::default());

            assert_eq!(tab.table_state.selected(), Some(PPO2_TABLE_DEPTH_MAX));
        }

        #[rstest]
        fn up_retreats_depth() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(Action::Move(Movement::Down), &mut RegisterStore::default());
            tab.handle_action(Action::Move(Movement::Up), &mut RegisterStore::default());

            assert_eq!(tab.table_state.selected(), Some(0));
        }

        #[rstest]
        fn up_at_zero_stays_at_zero() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(Action::Move(Movement::Up), &mut RegisterStore::default());

            assert_eq!(tab.table_state.selected(), Some(0));
        }

        #[rstest]
        fn goto_top_selects_depth_zero() {
            let mut tab = PpO2Tab::new();

            for _ in 0..10 {
                tab.handle_action(Action::Move(Movement::Down), &mut RegisterStore::default());
            }

            tab.handle_action(
                Action::Move(Movement::GotoTop),
                &mut RegisterStore::default(),
            );

            assert_eq!(tab.table_state.selected(), Some(0));
        }

        #[rstest]
        fn goto_bottom_selects_max_depth() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(
                Action::Move(Movement::GotoBottom),
                &mut RegisterStore::default(),
            );

            assert_eq!(tab.table_state.selected(), Some(PPO2_TABLE_DEPTH_MAX));
        }

        #[rstest]
        fn scroll_down_moves_by_delta() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(
                Action::Move(Movement::ScrollDown),
                &mut RegisterStore::default(),
            );

            assert_eq!(tab.table_state.selected(), Some(SCROLL_DELTA as usize));
        }

        #[rstest]
        fn scroll_up_moves_by_delta() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(
                Action::Move(Movement::GotoBottom),
                &mut RegisterStore::default(),
            );
            tab.handle_action(
                Action::Move(Movement::ScrollUp),
                &mut RegisterStore::default(),
            );

            assert_eq!(
                tab.table_state.selected(),
                Some(PPO2_TABLE_DEPTH_MAX - SCROLL_DELTA as usize),
            );
        }

        #[rstest]
        fn page_down_moves_by_page_delta() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(
                Action::Move(Movement::PageDown),
                &mut RegisterStore::default(),
            );

            assert_eq!(tab.table_state.selected(), Some(PAGE_DELTA as usize));
        }

        #[rstest]
        fn page_up_moves_by_page_delta() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(
                Action::Move(Movement::GotoBottom),
                &mut RegisterStore::default(),
            );
            tab.handle_action(
                Action::Move(Movement::PageUp),
                &mut RegisterStore::default(),
            );

            assert_eq!(
                tab.table_state.selected(),
                Some(PPO2_TABLE_DEPTH_MAX - PAGE_DELTA as usize),
            );
        }

        #[rstest]
        fn right_increments_mix() {
            let mut tab = PpO2Tab::new();
            let before = tab.mix_idx;

            tab.handle_action(Action::Move(Movement::Right), &mut RegisterStore::default());

            assert_eq!(tab.mix_idx, before + 1);
        }

        #[rstest]
        fn right_clamped_at_last_mix() {
            let mut tab = PpO2Tab::new();

            for _ in 0..=PPO2_TABLE_MIX_COUNT {
                tab.handle_action(Action::Move(Movement::Right), &mut RegisterStore::default());
            }

            assert_eq!(tab.mix_idx, PPO2_TABLE_MIX_COUNT - 1);
        }

        #[rstest]
        fn left_decrements_mix() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(Action::Move(Movement::Right), &mut RegisterStore::default());

            let before = tab.mix_idx;

            tab.handle_action(Action::Move(Movement::Left), &mut RegisterStore::default());

            assert_eq!(tab.mix_idx, before - 1);
        }

        #[rstest]
        fn left_clamped_at_zero_mix() {
            let mut tab = PpO2Tab::new();

            for _ in 0..=PPO2_MIX_DEFAULT_IDX {
                tab.handle_action(Action::Move(Movement::Left), &mut RegisterStore::default());
            }

            assert_eq!(tab.mix_idx, 0);
        }

        #[rstest]
        fn none_is_a_noop() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(Action::None, &mut RegisterStore::default());

            assert_eq!(tab.table_state.selected(), Some(0));
        }

        #[rstest]
        fn quit_is_a_noop() {
            let mut tab = PpO2Tab::new();

            tab.handle_action(Action::Quit, &mut RegisterStore::default());

            assert_eq!(tab.table_state.selected(), Some(0));
        }
    }

    mod render {
        use super::*;

        #[rstest]
        fn selected_column_is_mix_window_col_plus_fixed_col_count() {
            // width 123 fits all 14 mix columns (window_size=14), so col_in_window = PPO2_MIX_DEFAULT_IDX(5).
            // selected_column = col_in_window(5) + FIXED_COL_COUNT(1) = 6
            let mut tab = PpO2Tab::new();
            let area = Rect::new(0, 0, 123, 40);

            tab.render(area, &mut Buffer::empty(area), &Theme::default());

            assert_eq!(tab.table_state.selected_column(), Some(6));
        }
    }
}
