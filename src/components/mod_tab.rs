//! MOD-by-ppO₂ table component.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Cell, Paragraph, Row, StatefulWidget, TableState, Widget},
};

use crate::{
    action::{Action, Movement},
    gas::{EANx, MOD},
    theme::Theme,
    ui::{build_header_row, col_window_size, styled_table, trailing_constraints, window_start},
    units::{Bar, Meters, Percent},
};

use super::{Component, KeyBinding};

const PPO2_MIN: Bar = Bar::new(0.8);
const PPO2_STEP: Bar = Bar::new(0.1);
const PPO2_MAX_IDX: usize = 8;
const PPO2_DEFAULT_IDX: usize = 6;
const PPO2_COUNT: usize = PPO2_MAX_IDX + 1;

const O2_PCT_MIN: u8 = 10;
const O2_PCT_MAX: u8 = 100;
const DEFAULT_MIX: Percent = Percent::new(0.32).expect("valid fraction literal");

const COL_NAME_W: u16 = 12;
const COL_O2_W: u16 = 6;
const COL_MOD_W: u16 = 9;
const FIXED_COL_COUNT: usize = 2;
const TABLE_OVERHEAD_W: u16 = 2 + 2 + COL_NAME_W + 1 + COL_O2_W + 1;

const MOD_RED_BELOW: Meters = Meters::new(10.0);
const MOD_YELLOW_BELOW: Meters = Meters::new(20.0);

/// MOD-by-ppO₂ table: maximum operating depth for each nitrox mix at the selected ppO₂ limit.
#[derive(Debug)]
pub struct ModTab {
    mixes: Vec<EANx>,
    table_state: TableState,
    ppo2_idx: usize,
    selection: Option<MOD>,
}

impl Default for ModTab {
    fn default() -> Self {
        Self::new()
    }
}

impl ModTab {
    /// Creates a `ModTab` pre-selected on EAN32 at 1.4 bar ppO₂.
    #[must_use]
    pub fn new() -> Self {
        let mixes: Vec<EANx> = (O2_PCT_MIN..=O2_PCT_MAX)
            .filter_map(|p| Percent::new(f64::from(p) / 100.0))
            .filter_map(|pct| EANx::try_from(pct).ok())
            .collect();
        let start_idx = mixes
            .iter()
            .position(|m| m.fo2() == DEFAULT_MIX)
            .unwrap_or(0);
        let mut table_state = TableState::default();
        table_state.select(Some(start_idx));
        Self {
            mixes,
            table_state,
            ppo2_idx: PPO2_DEFAULT_IDX,
            selection: None,
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "ppo2_idx is bounded by PPO2_MAX_IDX = 8"
    )]
    const fn ppo2(&self) -> Bar {
        PPO2_STEP.mul_add(self.ppo2_idx as f64, PPO2_MIN)
    }

    /// ppO₂ column values for a sliding window of `window_size` columns centred on the selected index.
    #[expect(
        clippy::cast_precision_loss,
        reason = "start + i is bounded by PPO2_COUNT = 9"
    )]
    fn visible_columns(&self, window_size: usize) -> Vec<Bar> {
        let start = window_start(self.ppo2_idx, PPO2_COUNT, window_size);
        let count = window_size.min(PPO2_COUNT);
        (0..count)
            .map(|i| PPO2_STEP.mul_add((start + i) as f64, PPO2_MIN))
            .collect()
    }

    /// Column index of the selected ppO₂ within the visible window (0-based).
    fn ppo2_window_col(&self, window_size: usize) -> usize {
        self.ppo2_idx - window_start(self.ppo2_idx, PPO2_COUNT, window_size)
    }

    fn move_row(&mut self, delta: isize) {
        let max = self.mixes.len().saturating_sub(1);
        super::move_row(&mut self.table_state, delta, max);
    }

    fn move_up(&mut self) {
        self.move_row(-1);
    }

    fn move_down(&mut self) {
        self.move_row(1);
    }

    const fn move_left(&mut self) {
        self.ppo2_idx = self.ppo2_idx.saturating_sub(1);
    }

    fn move_right(&mut self) {
        self.ppo2_idx = (self.ppo2_idx + 1).min(PPO2_MAX_IDX);
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
        self.table_state.select(Some(self.mixes.len() - 1));
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
            Movement::None => {}
        }
    }

    fn build_rows(&self, cols: &[Bar], theme: &Theme) -> Vec<Row<'static>> {
        self.mixes
            .iter()
            .map(|mix| ModRow { mix, cols, theme }.into())
            .collect()
    }
}

struct ModRow<'a> {
    mix: &'a EANx,
    cols: &'a [Bar],
    theme: &'a Theme,
}

impl From<ModRow<'_>> for Row<'static> {
    fn from(r: ModRow<'_>) -> Self {
        let mut cells = vec![
            Cell::from(r.mix.to_string()),
            Cell::from(r.mix.fo2().to_string()),
        ];

        for &col in r.cols {
            let m = r.mix.mod_at(col);
            cells.push(Cell::from(format!("{m}")).style(mod_color(m.into(), r.theme)));
        }

        Row::new(cells)
    }
}

fn mod_color(depth: Meters, theme: &Theme) -> Style {
    if depth < MOD_RED_BELOW {
        theme.danger()
    } else if depth < MOD_YELLOW_BELOW {
        theme.caution()
    } else {
        theme.safe()
    }
}

struct ModTabStatus<'a> {
    tab: &'a ModTab,
    theme: Theme,
}

impl Widget for ModTabStatus<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.tab.selection {
            Some(m) => {
                let text = format!(" \u{25c6} {}", m.summary());
                Paragraph::new(text)
                    .style(self.theme.status_active())
                    .render(area, buf);
            }
            None => Paragraph::new(" No gas selected — press Enter to select")
                .style(self.theme.status_empty())
                .render(area, buf),
        }
    }
}

impl Component for ModTab {
    fn title(&self) -> &'static str {
        "MOD Table"
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Move(mv) => self.handle_movement(mv),
            Action::Select => {
                if let Some(row) = self.table_state.selected() {
                    self.selection = Some(self.mixes[row].mod_at(self.ppo2()));
                }
            }
            _ => {}
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let window_size = col_window_size(area.width, TABLE_OVERHEAD_W, COL_MOD_W, PPO2_COUNT);
        let col_in_window = self.ppo2_window_col(window_size);

        self.table_state
            .select_column(Some(col_in_window + FIXED_COL_COUNT));

        let cols = self.visible_columns(window_size);
        let title = format!(" DPS — MOD Table   ppO\u{2082} {} ", self.ppo2());

        let constraints = trailing_constraints(
            [Constraint::Length(COL_NAME_W), Constraint::Length(COL_O2_W)].as_slice(),
            cols.len(),
            COL_MOD_W,
        );

        let header = build_header_row(
            vec![
                Cell::from("Name").style(theme.header_cell()),
                Cell::from("O\u{2082}%").style(theme.header_cell()),
            ],
            cols.iter().map(ToString::to_string),
            Some(col_in_window),
            theme,
        );

        let table = styled_table(
            self.build_rows(&cols, theme),
            constraints,
            header,
            title,
            theme,
        );

        StatefulWidget::render(table, area, buf, &mut self.table_state);
    }

    fn render_status(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        ModTabStatus {
            tab: self,
            theme: *theme,
        }
        .render(area, buf);
    }

    fn key_bindings(&self) -> &'static [KeyBinding] {
        static BINDINGS: &[KeyBinding] = [
            KeyBinding {
                key: "j/k",
                desc: "navigate rows",
            },
            KeyBinding {
                key: "h/l",
                desc: "change ppO\u{2082} limit",
            },
            KeyBinding {
                key: "Enter",
                desc: "select gas",
            },
        ]
        .as_slice();
        BINDINGS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::test_utils::widget_text;
    use color_eyre::{Result, eyre::eyre};

    mod constants {
        use super::*;

        #[test]
        fn ppo2_count_is_nine() {
            // PPO2_MAX_IDX is 8; count = 8 + 1
            assert_eq!(PPO2_COUNT, 9);
        }

        #[test]
        fn table_overhead_w_is_twenty_four() {
            // 2 + 2 + 12(COL_NAME_W) + 1 + 6(COL_O2_W) + 1 = 24
            assert_eq!(TABLE_OVERHEAD_W, 24);
        }
    }

    mod visible_columns_fn {
        use super::*;
        use approx::assert_relative_eq;

        #[test]
        fn full_window_returns_all_nine_columns() {
            let tab = ModTab::new();
            assert_eq!(tab.visible_columns(20).len(), 9);
        }

        #[test]
        fn returns_bars_at_correct_offsets_from_start() {
            // ppo2_idx = PPO2_DEFAULT_IDX = 6
            let tab = ModTab::new();
            // window_start(6, 9, 3) = 5; values: (5+i)*0.1 + 0.8
            let cols = tab.visible_columns(3);

            assert_eq!(cols.len(), 3);

            assert_relative_eq!(cols[0], Bar::new(1.3), epsilon = 1e-9); // (5+0)*0.1+0.8
            assert_relative_eq!(cols[1], Bar::new(1.4), epsilon = 1e-9); // (5+1)*0.1+0.8
            assert_relative_eq!(cols[2], Bar::new(1.5), epsilon = 1e-9); // (5+2)*0.1+0.8
        }
    }

    mod ppo2_window_col_fn {
        use super::*;

        #[test]
        fn at_max_idx_with_small_window() {
            let mut tab = ModTab::new();

            for _ in 0..PPO2_MAX_IDX {
                tab.handle_action(Action::Move(Movement::Right));
            }

            // = 8
            assert_eq!(tab.ppo2_idx, PPO2_MAX_IDX);
            // window_start(8, 9, 3): half=1, max_start=6, (8-1).min(6)=6 → col=8-6=2
            assert_eq!(tab.ppo2_window_col(3), 2);
        }
    }

    mod component_trait {
        use super::*;

        #[test]
        fn title_is_mod_table() {
            assert_eq!(ModTab::new().title(), "MOD Table");
        }

        #[test]
        fn key_bindings_is_non_empty() {
            assert!(!ModTab::new().key_bindings().is_empty());
        }
    }

    mod initial_state {
        use super::*;

        #[test]
        fn selected_row_is_ean32() -> Result<()> {
            let tab = ModTab::new();
            let idx = tab
                .table_state
                .selected()
                .ok_or_else(|| eyre!("no row selected"))?;

            assert_eq!(tab.mixes[idx].fo2(), DEFAULT_MIX);

            Ok(())
        }

        #[test]
        fn ppo2_is_1_4_bar() {
            use approx::assert_relative_eq;
            let tab = ModTab::new();
            assert_relative_eq!(tab.ppo2(), Bar::new(1.4));
        }

        #[test]
        fn no_selection() {
            assert!(ModTab::new().selection.is_none());
        }
    }

    mod select_action {
        use super::*;
        use crate::action::Movement;

        #[test]
        fn stores_current_mix_and_ppo2() -> Result<()> {
            let mut tab = ModTab::new();
            let row = tab
                .table_state
                .selected()
                .ok_or_else(|| eyre!("no row selected"))?;
            let expected_fo2 = tab.mixes[row].fo2();
            let expected_ppo2 = tab.ppo2();

            tab.handle_action(Action::Select);

            let m = tab
                .selection
                .ok_or_else(|| eyre!("no selection after Select action"))?;

            assert_eq!(m.fo2(), expected_fo2);
            assert_eq!(m.ppo2_max(), expected_ppo2);

            Ok(())
        }

        #[test]
        fn selection_updates_after_moving_row() -> Result<()> {
            let mut tab = ModTab::new();
            tab.handle_action(Action::Select);

            let first_fo2 = tab
                .selection
                .ok_or_else(|| eyre!("no selection after first Select"))?
                .fo2();

            tab.handle_action(Action::Move(Movement::Down));
            tab.handle_action(Action::Select);

            let second_fo2 = tab
                .selection
                .ok_or_else(|| eyre!("no selection after second Select"))?
                .fo2();

            assert_ne!(first_fo2, second_fo2);

            Ok(())
        }
    }

    mod mod_color {
        use super::*;

        #[test]
        fn below_10m_is_red() {
            assert_eq!(
                mod_color(Meters::new(9.9), &Theme::default()),
                Theme::default().danger()
            );
            assert_eq!(
                mod_color(Meters::new(0.0), &Theme::default()),
                Theme::default().danger()
            );
        }

        #[test]
        fn exactly_10m_is_yellow() {
            assert_eq!(
                mod_color(Meters::new(10.0), &Theme::default()),
                Theme::default().caution()
            );
        }

        #[test]
        fn between_thresholds_is_yellow() {
            assert_eq!(
                mod_color(Meters::new(15.0), &Theme::default()),
                Theme::default().caution()
            );
        }

        #[test]
        fn exactly_20m_is_green() {
            assert_eq!(
                mod_color(Meters::new(20.0), &Theme::default()),
                Theme::default().safe()
            );
        }

        #[test]
        fn above_20m_is_green() {
            assert_eq!(
                mod_color(Meters::new(33.75), &Theme::default()),
                Theme::default().safe()
            );
        }
    }

    mod status_bar {
        use super::*;

        #[test]
        fn no_selection_shows_prompt() {
            let tab = ModTab::new();
            let text = widget_text(
                ModTabStatus {
                    tab: &tab,
                    theme: Theme::default(),
                },
                60,
            );

            assert!(text.contains("No gas"));
        }

        #[test]
        fn selection_shows_mix_percent_and_mod() {
            let mut tab = ModTab::new();

            tab.handle_action(Action::Select);

            let text = widget_text(
                ModTabStatus {
                    tab: &tab,
                    theme: Theme::default(),
                },
                60,
            );

            assert!(text.contains("32"));
            assert!(text.contains("MOD"));
        }
    }

    mod action_dispatch {
        use super::*;
        use crate::action::{Action, Movement};
        use crate::components::{PAGE_DELTA, SCROLL_DELTA};

        #[test]
        fn down_advances_row() -> Result<()> {
            let mut tab = ModTab::new();
            let start = tab
                .table_state
                .selected()
                .ok_or_else(|| eyre!("no row selected"))?;

            tab.handle_action(Action::Move(Movement::Down));

            assert_eq!(
                tab.table_state
                    .selected()
                    .ok_or_else(|| eyre!("no row selected"))?,
                start + 1
            );

            Ok(())
        }

        #[test]
        fn down_clamped_at_last_mix() -> Result<()> {
            let mut tab = ModTab::new();

            tab.handle_action(Action::Move(Movement::GotoBottom));
            tab.handle_action(Action::Move(Movement::Down));

            assert_eq!(
                tab.table_state
                    .selected()
                    .ok_or_else(|| eyre!("no row selected"))?,
                tab.mixes.len() - 1
            );

            Ok(())
        }

        #[test]
        fn up_retreats_row() -> Result<()> {
            let mut tab = ModTab::new();
            tab.handle_action(Action::Move(Movement::Down));

            let after = tab
                .table_state
                .selected()
                .ok_or_else(|| eyre!("no row selected"))?;
            tab.handle_action(Action::Move(Movement::Up));

            assert_eq!(
                tab.table_state
                    .selected()
                    .ok_or_else(|| eyre!("no row selected"))?,
                after - 1
            );

            Ok(())
        }

        #[test]
        fn up_clamped_at_zero() -> Result<()> {
            let mut tab = ModTab::new();

            tab.handle_action(Action::Move(Movement::GotoTop));
            tab.handle_action(Action::Move(Movement::Up));

            assert_eq!(
                tab.table_state
                    .selected()
                    .ok_or_else(|| eyre!("no row selected"))?,
                0
            );

            Ok(())
        }

        #[test]
        fn goto_top_selects_first_row() -> Result<()> {
            let mut tab = ModTab::new();

            for _ in 0..10 {
                tab.handle_action(Action::Move(Movement::Down));
            }

            tab.handle_action(Action::Move(Movement::GotoTop));

            assert_eq!(
                tab.table_state
                    .selected()
                    .ok_or_else(|| eyre!("no row selected"))?,
                0
            );

            Ok(())
        }

        #[test]
        fn goto_bottom_selects_last_row() -> Result<()> {
            let mut tab = ModTab::new();

            tab.handle_action(Action::Move(Movement::GotoBottom));

            assert_eq!(
                tab.table_state
                    .selected()
                    .ok_or_else(|| eyre!("no row selected"))?,
                tab.mixes.len() - 1
            );

            Ok(())
        }

        #[test]
        fn scroll_down_moves_by_delta() -> Result<()> {
            let mut tab = ModTab::new();

            tab.handle_action(Action::Move(Movement::ScrollDown));

            assert_eq!(
                tab.table_state
                    .selected()
                    .ok_or_else(|| eyre!("no row selected"))?,
                tab.mixes
                    .iter()
                    .position(|m| m.fo2() == DEFAULT_MIX)
                    .ok_or_else(|| eyre!("EAN32 not found in mixes"))?
                    + SCROLL_DELTA as usize,
            );

            Ok(())
        }

        #[test]
        fn scroll_up_moves_by_delta() -> Result<()> {
            let mut tab = ModTab::new();

            tab.handle_action(Action::Move(Movement::GotoBottom));
            tab.handle_action(Action::Move(Movement::ScrollUp));

            assert_eq!(
                tab.table_state
                    .selected()
                    .ok_or_else(|| eyre!("no row selected"))?,
                tab.mixes.len() - 1 - SCROLL_DELTA as usize,
            );

            Ok(())
        }

        #[test]
        fn page_down_moves_by_page_delta() -> Result<()> {
            let mut tab = ModTab::new();
            let start = tab
                .table_state
                .selected()
                .ok_or_else(|| eyre!("no row selected"))?;

            tab.handle_action(Action::Move(Movement::PageDown));

            assert_eq!(
                tab.table_state
                    .selected()
                    .ok_or_else(|| eyre!("no row selected"))?,
                start + PAGE_DELTA as usize,
            );

            Ok(())
        }

        #[test]
        fn page_up_moves_by_page_delta() -> Result<()> {
            let mut tab = ModTab::new();

            tab.handle_action(Action::Move(Movement::GotoBottom));
            tab.handle_action(Action::Move(Movement::PageUp));

            assert_eq!(
                tab.table_state
                    .selected()
                    .ok_or_else(|| eyre!("no row selected"))?,
                tab.mixes.len() - 1 - PAGE_DELTA as usize,
            );

            Ok(())
        }

        #[test]
        fn right_increments_ppo2() {
            let mut tab = ModTab::new();
            let before = tab.ppo2_idx;

            tab.handle_action(Action::Move(Movement::Right));

            assert_eq!(tab.ppo2_idx, before + 1);
        }

        #[test]
        fn right_clamped_at_max_ppo2() {
            let mut tab = ModTab::new();

            for _ in 0..=PPO2_MAX_IDX {
                tab.handle_action(Action::Move(Movement::Right));
            }

            assert_eq!(tab.ppo2_idx, PPO2_MAX_IDX);
        }

        #[test]
        fn left_decrements_ppo2() {
            let mut tab = ModTab::new();
            tab.handle_action(Action::Move(Movement::Right));

            let before = tab.ppo2_idx;
            tab.handle_action(Action::Move(Movement::Left));

            assert_eq!(tab.ppo2_idx, before - 1);
        }

        #[test]
        fn left_clamped_at_zero_ppo2() {
            let mut tab = ModTab::new();

            for _ in 0..=PPO2_DEFAULT_IDX {
                tab.handle_action(Action::Move(Movement::Left));
            }

            assert_eq!(tab.ppo2_idx, 0);
        }

        #[test]
        fn none_is_a_noop() -> Result<()> {
            let mut tab = ModTab::new();
            let before = tab
                .table_state
                .selected()
                .ok_or_else(|| eyre!("no row selected"))?;

            tab.handle_action(Action::None);

            assert_eq!(
                tab.table_state
                    .selected()
                    .ok_or_else(|| eyre!("no row selected"))?,
                before
            );

            Ok(())
        }

        #[test]
        fn quit_is_a_noop() -> Result<()> {
            let mut tab = ModTab::new();
            let before = tab
                .table_state
                .selected()
                .ok_or_else(|| eyre!("no row selected"))?;

            tab.handle_action(Action::Quit);

            assert_eq!(
                tab.table_state
                    .selected()
                    .ok_or_else(|| eyre!("no row selected"))?,
                before
            );

            Ok(())
        }
    }

    mod render {
        use super::*;

        #[test]
        fn selected_column_is_ppo2_window_col_plus_fixed_col_count() {
            // width 113 fits all 9 ppO₂ columns (window_size=9), so col_in_window = PPO2_DEFAULT_IDX(6).
            // selected_column = col_in_window(6) + FIXED_COL_COUNT(2) = 8
            let mut tab = ModTab::new();
            let area = Rect::new(0, 0, 113, 40);

            tab.render(area, &mut Buffer::empty(area), &Theme::default());

            assert_eq!(tab.table_state.selected_column(), Some(8));
        }
    }
}
