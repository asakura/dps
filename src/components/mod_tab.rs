//! MOD-by-$\text{pp}\ce{O2}$ table component.
//!
//! # Examples
//!
//! ```
//! use dps::components::mod_tab::ModTab;
//!
//! let _tab = ModTab::new();
//! ```

use super::{Component, Result};

use crate::{
    action::{Action, EditOp, Movement},
    config::Config,
    registers::{RegisterName, RegisterStore, RegisterValue},
    theme::Theme,
    ui::{build_header_row, col_window_size, styled_table, trailing_constraints, window_start},
    units::{Bar, Meters, Percent},
};

use dps_gas::prelude::{EANx, MOD};

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Cell, Row, StatefulWidget, TableState},
};

const PPO2_MIN: Bar = Bar::new(0.8);
const PPO2_STEP: Bar = Bar::new(0.1);
const PPO2_MAX_IDX: usize = 8;
const PPO2_DEFAULT_IDX: usize = 6;
const PPO2_COUNT: usize = PPO2_MAX_IDX + 1;

const O2_PCT_MIN: u8 = 10;
const O2_PCT_MAX: u8 = 100;
const DEFAULT_MIX: Percent = Percent::literal(0.32);

const COL_NAME_W: u16 = 12;
const COL_O2_W: u16 = 6;
const COL_MOD_W: u16 = 9;
const FIXED_COL_COUNT: usize = 2;
const TABLE_OVERHEAD_W: u16 = 2 + 2 + COL_NAME_W + 1 + COL_O2_W + 1;

const MOD_RED_BELOW: Meters = Meters::new(10.0);
const MOD_YELLOW_BELOW: Meters = Meters::new(20.0);

/// MOD-by-$\text{pp}\ce{O2}$ table: maximum operating depth for each nitrox mix at the selected $\text{pp}\ce{O2}$ limit.
#[derive(Debug)]
pub struct ModTab {
    theme: Theme,
    mixes: Vec<EANx>,
    table_state: TableState,
    ppo2_idx: usize,
    selection: Option<MOD>,
    last_paste_row: Option<usize>,
}

impl Default for ModTab {
    fn default() -> Self {
        Self::new()
    }
}

impl ModTab {
    pub(crate) const TITLE: &'static str = "MOD Table";

    /// Creates a `ModTab` pre-selected on EAN32 at 1.4 bar $\text{pp}\ce{O2}$.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::components::mod_tab::ModTab;
    ///
    /// let tab = ModTab::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        let mixes: Vec<EANx> = (O2_PCT_MIN..=O2_PCT_MAX)
            .filter_map(|p| Percent::new(f64::from(p) / 100.0).ok())
            .filter_map(|pct| EANx::try_from(pct).ok())
            .collect();
        let start_idx = mixes
            .iter()
            .position(|m| m.fo2() == DEFAULT_MIX)
            .unwrap_or(0);
        let mut table_state = TableState::default();
        table_state.select(Some(start_idx));
        Self {
            theme: Theme::default(),
            mixes,
            table_state,
            ppo2_idx: PPO2_DEFAULT_IDX,
            selection: None,
            last_paste_row: None,
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "ppo2_idx is bounded by PPO2_MAX_IDX = 8"
    )]
    const fn ppo2(&self) -> Bar {
        PPO2_STEP.mul_add(self.ppo2_idx as f64, PPO2_MIN)
    }

    /// $\text{pp}\ce{O2}$ column values for a sliding window of `window_size` columns centred on the selected index.
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

    /// Column index of the selected $\text{pp}\ce{O2}$ within the visible window (0-based).
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
        }
    }

    fn build_rows(&self, cols: &[Bar], theme: &Theme) -> Vec<Row<'static>> {
        self.mixes
            .iter()
            .map(|mix| ModRow { mix, cols, theme }.into())
            .collect()
    }

    fn handle_action(&mut self, action: &Action, registers: &mut RegisterStore) {
        if !matches!(action, Action::Edit(EditOp::CyclePaste)) {
            self.last_paste_row = None;
            registers.reset_ring_cursor();
        }

        match *action {
            Action::Move(mv) => self.handle_movement(mv),
            Action::Select => {
                if let Some(row) = self.table_state.selected() {
                    self.selection = Some(self.mixes[row].mod_at(self.ppo2()));
                }
            }
            Action::Edit(EditOp::YankRow(reg)) => {
                if let Some(row) = self.table_state.selected() {
                    registers.push_yank(
                        reg.unwrap_or(RegisterName::Unnamed),
                        RegisterValue::EANx(self.mixes[row]),
                    );
                }
            }
            Action::Edit(EditOp::Paste(reg)) => {
                let r = reg.unwrap_or(RegisterName::Unnamed);
                if let Some(RegisterValue::EANx(mix)) = registers.read(r) {
                    let insert_at = self.table_state.selected().map_or(0, |r| r + 1);
                    self.mixes.insert(insert_at, mix);
                    self.table_state.select(Some(insert_at));
                    self.last_paste_row = Some(insert_at);
                }
            }
            Action::Edit(EditOp::PasteAbove(reg)) => {
                let r = reg.unwrap_or(RegisterName::Unnamed);
                if let Some(RegisterValue::EANx(mix)) = registers.read(r) {
                    let insert_at = self.table_state.selected().unwrap_or(0);
                    self.mixes.insert(insert_at, mix);
                    self.table_state.select(Some(insert_at));
                    self.last_paste_row = Some(insert_at);
                }
            }
            Action::Edit(EditOp::CyclePaste) => {
                if let Some(row) = self.last_paste_row
                    && let Ok(RegisterValue::EANx(mix)) = registers.cycle_yank()
                {
                    self.mixes[row] = mix;
                }
            }
            Action::Edit(EditOp::Delete(reg)) => {
                if let Some(row) = self.table_state.selected() {
                    let value = RegisterValue::EANx(self.mixes.remove(row));
                    let new_sel = (!self.mixes.is_empty()).then(|| row.min(self.mixes.len() - 1));
                    self.table_state.select(new_sel);
                    match reg {
                        Some(
                            r @ (RegisterName::Named(_)
                            | RegisterName::Clipboard
                            | RegisterName::Selection
                            | RegisterName::BlackHole),
                        ) => {
                            registers.write(r, value);
                        }
                        _ => {
                            registers.push_delete(value);
                        }
                    }
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

impl Component for ModTab {
    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.theme = *config.active_theme();
        Ok(())
    }

    fn update(&mut self, action: Action, registers: &mut RegisterStore) -> Result<Option<Action>> {
        self.handle_action(&action, registers);
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()> {
        let theme = self.theme;
        self.render(area, frame.buffer_mut(), &theme);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::action::EditOp;
    use crate::components::{PAGE_DELTA, SCROLL_DELTA};
    use crate::registers::{RegisterError, RegisterName};

    use approx::assert_relative_eq;
    use rstest::{fixture, rstest};

    use std::result::Result;

    fn reg(c: char) -> Result<RegisterName, RegisterError> {
        RegisterName::try_from(c)
    }

    #[fixture]
    fn regs() -> RegisterStore {
        RegisterStore::default()
    }

    mod constants {
        use super::*;

        #[rstest]
        fn ppo2_count_is_nine() {
            // PPO2_MAX_IDX is 8; count = 8 + 1
            assert_eq!(PPO2_COUNT, 9);
        }

        #[rstest]
        fn table_overhead_w_is_twenty_four() {
            // 2 + 2 + 12(COL_NAME_W) + 1 + 6(COL_O2_W) + 1 = 24
            assert_eq!(TABLE_OVERHEAD_W, 24);
        }
    }

    mod visible_columns_fn {
        use super::*;

        #[rstest]
        fn full_window_returns_all_nine_columns() {
            let tab = ModTab::new();
            assert_eq!(tab.visible_columns(20).len(), 9);
        }

        #[rstest]
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

        #[rstest]
        fn at_max_idx_with_small_window() {
            let mut tab = ModTab::new();

            for _ in 0..PPO2_MAX_IDX {
                tab.handle_action(
                    &Action::Move(Movement::Right),
                    &mut RegisterStore::default(),
                );
            }

            // = 8
            assert_eq!(tab.ppo2_idx, PPO2_MAX_IDX);
            // window_start(8, 9, 3): half=1, max_start=6, (8-1).min(6)=6 → col=8-6=2
            assert_eq!(tab.ppo2_window_col(3), 2);
        }
    }

    mod initial_state {
        use super::*;

        #[rstest]
        fn selected_row_is_ean32() {
            let tab = ModTab::new();
            assert_eq!(
                tab.table_state.selected().map(|i| tab.mixes[i].fo2()),
                Some(DEFAULT_MIX),
            );
        }

        #[rstest]
        fn ppo2_is_1_4_bar() {
            assert_relative_eq!(ModTab::new().ppo2(), Bar::new(1.4));
        }

        #[rstest]
        fn no_selection() {
            assert!(ModTab::new().selection.is_none());
        }
    }

    mod select_action {
        use super::*;

        #[rstest]
        fn stores_current_mix_and_ppo2() {
            let mut tab = ModTab::new();
            let expected_fo2 = tab.table_state.selected().map(|i| tab.mixes[i].fo2());
            let expected_ppo2 = tab.ppo2();

            tab.handle_action(&Action::Select, &mut RegisterStore::default());

            assert_eq!(tab.selection.map(MOD::fo2), expected_fo2);
            assert_eq!(tab.selection.map(MOD::ppo2_max), Some(expected_ppo2));
        }

        #[rstest]
        fn selection_updates_after_moving_row() {
            let mut tab = ModTab::new();
            tab.handle_action(&Action::Select, &mut RegisterStore::default());
            let first_fo2 = tab.selection.map(MOD::fo2);

            tab.handle_action(&Action::Move(Movement::Down), &mut RegisterStore::default());
            tab.handle_action(&Action::Select, &mut RegisterStore::default());

            assert_ne!(tab.selection.map(MOD::fo2), first_fo2);
        }
    }

    mod yank_row {
        use super::*;

        #[rstest]
        fn unnamed_reg_writes_mix_to_unnamed_and_yank_register(
            mut regs: RegisterStore,
        ) -> Result<(), RegisterError> {
            let mut tab = ModTab::new();
            let expected = tab
                .table_state
                .selected()
                .map(|r| RegisterValue::EANx(tab.mixes[r]));

            tab.handle_action(&Action::Edit(EditOp::YankRow(None)), &mut regs);

            assert_eq!(regs.read(RegisterName::Unnamed), expected);
            assert_eq!(regs.read(reg('0')?), expected);

            Ok(())
        }

        #[rstest]
        fn named_reg_writes_mix_to_named_and_yank_register(
            mut regs: RegisterStore,
        ) -> Result<(), RegisterError> {
            let mut tab = ModTab::new();
            let expected = tab
                .table_state
                .selected()
                .map(|r| RegisterValue::EANx(tab.mixes[r]));

            tab.handle_action(
                &Action::Edit(EditOp::YankRow(RegisterName::try_from('a').ok())),
                &mut regs,
            );

            assert_eq!(regs.read(reg('a')?), expected);
            assert_eq!(regs.read(reg('0')?), expected);
            Ok(())
        }
    }

    mod delete {
        use super::*;

        #[rstest]
        fn removes_focused_row_from_mixes() {
            let mut tab = ModTab::new();
            let before_len = tab.mixes.len();

            tab.handle_action(
                &Action::Edit(EditOp::Delete(None)),
                &mut RegisterStore::default(),
            );

            assert_eq!(tab.mixes.len(), before_len - 1);
        }

        #[rstest]
        fn unnamed_routes_to_delete_stack(mut regs: RegisterStore) -> Result<(), RegisterError> {
            let mut tab = ModTab::new();
            let expected = tab
                .table_state
                .selected()
                .map(|r| RegisterValue::EANx(tab.mixes[r]));

            tab.handle_action(&Action::Edit(EditOp::Delete(None)), &mut regs);

            assert_eq!(regs.read(reg('1')?), expected);
            assert_eq!(regs.read(RegisterName::Unnamed), expected);
            assert!(regs.read(reg('a')?).is_none());
            Ok(())
        }

        #[rstest]
        fn named_reg_bypasses_delete_stack(mut regs: RegisterStore) -> Result<(), RegisterError> {
            let mut tab = ModTab::new();
            let expected = tab
                .table_state
                .selected()
                .map(|r| RegisterValue::EANx(tab.mixes[r]));

            tab.handle_action(
                &Action::Edit(EditOp::Delete(RegisterName::try_from('a').ok())),
                &mut regs,
            );

            assert_eq!(regs.read(reg('a')?), expected);
            assert_eq!(regs.read(RegisterName::Unnamed), expected);
            assert!(regs.read(reg('1')?).is_none());
            Ok(())
        }

        #[rstest]
        fn blackhole_reg_discards_value(mut regs: RegisterStore) -> Result<(), RegisterError> {
            let mut tab = ModTab::new();

            tab.handle_action(
                &Action::Edit(EditOp::Delete(RegisterName::try_from('_').ok())),
                &mut regs,
            );

            assert!(regs.read(reg('_')?).is_none());
            assert!(regs.read(RegisterName::Unnamed).is_none());
            assert!(regs.read(reg('1')?).is_none());
            Ok(())
        }

        #[rstest]
        fn digit_reg_routes_to_delete_stack(mut regs: RegisterStore) -> Result<(), RegisterError> {
            let mut tab = ModTab::new();
            let expected = tab
                .table_state
                .selected()
                .map(|r| RegisterValue::EANx(tab.mixes[r]));

            tab.handle_action(
                &Action::Edit(EditOp::Delete(RegisterName::try_from('3').ok())),
                &mut regs,
            );

            assert_eq!(regs.read(reg('1')?), expected);
            assert!(regs.read(reg('3')?).is_none());
            Ok(())
        }

        #[rstest]
        fn selection_clamps_when_last_row_deleted() {
            let mut tab = ModTab::new();
            tab.handle_action(
                &Action::Move(Movement::GotoBottom),
                &mut RegisterStore::default(),
            );
            let last = tab.mixes.len() - 1;

            tab.handle_action(
                &Action::Edit(EditOp::Delete(None)),
                &mut RegisterStore::default(),
            );

            assert_eq!(tab.table_state.selected(), Some(last - 1));
        }

        #[rstest]
        fn selection_stays_stable_when_mid_row_deleted() {
            let mut tab = ModTab::new();
            tab.handle_action(
                &Action::Move(Movement::GotoTop),
                &mut RegisterStore::default(),
            );

            tab.handle_action(
                &Action::Edit(EditOp::Delete(None)),
                &mut RegisterStore::default(),
            );

            assert_eq!(tab.table_state.selected(), Some(0));
        }
    }

    mod paste {
        use super::*;

        #[rstest]
        fn inserts_below_focused_row(mut regs: RegisterStore) {
            let mut tab = ModTab::new();
            let before_len = tab.mixes.len();
            let cursor = tab.table_state.selected();

            tab.handle_action(&Action::Edit(EditOp::YankRow(None)), &mut regs);
            tab.handle_action(&Action::Edit(EditOp::Paste(None)), &mut regs);

            assert_eq!(tab.mixes.len(), before_len + 1);
            assert_eq!(tab.table_state.selected(), cursor.map(|c| c + 1));
        }

        #[rstest]
        fn inserts_from_named_register(mut regs: RegisterStore) {
            let mut tab = ModTab::new();
            let cursor = tab.table_state.selected();
            let yanked = cursor.map(|c| tab.mixes[c]);

            tab.handle_action(
                &Action::Edit(EditOp::YankRow(RegisterName::try_from('a').ok())),
                &mut regs,
            );

            tab.handle_action(
                &Action::Edit(EditOp::Paste(RegisterName::try_from('a').ok())),
                &mut regs,
            );

            assert_eq!(cursor.and_then(|c| tab.mixes.get(c + 1)).copied(), yanked);
        }

        #[rstest]
        fn no_op_when_register_empty() {
            let mut tab = ModTab::new();
            let before_len = tab.mixes.len();

            tab.handle_action(
                &Action::Edit(EditOp::Paste(None)),
                &mut RegisterStore::default(),
            );

            assert_eq!(tab.mixes.len(), before_len);
        }
    }

    mod paste_above {
        use super::*;

        #[rstest]
        fn inserts_at_focused_row(mut regs: RegisterStore) {
            let mut tab = ModTab::new();
            let before_len = tab.mixes.len();
            let cursor = tab.table_state.selected();

            tab.handle_action(&Action::Edit(EditOp::YankRow(None)), &mut regs);
            tab.handle_action(&Action::Edit(EditOp::PasteAbove(None)), &mut regs);

            assert_eq!(tab.mixes.len(), before_len + 1);
            assert_eq!(tab.table_state.selected(), cursor);
        }
    }

    mod cycle_paste {
        use super::*;

        #[rstest]
        fn replaces_pasted_row_with_older_yank(mut regs: RegisterStore) {
            let mut tab = ModTab::new();
            let cursor = tab.table_state.selected();

            // Yank row A (older), move down, yank row B (newer).
            let mix_a = cursor.map(|c| tab.mixes[c]);
            tab.handle_action(&Action::Edit(EditOp::YankRow(None)), &mut regs);
            tab.handle_action(&Action::Move(Movement::Down), &mut regs);

            let mix_b = cursor.map(|c| tab.mixes[c + 1]);
            tab.handle_action(&Action::Edit(EditOp::YankRow(None)), &mut regs);

            // Paste inserts B (most recent) below cursor.
            tab.handle_action(&Action::Edit(EditOp::Paste(None)), &mut regs);
            let paste_row = tab.table_state.selected();
            assert_eq!(paste_row.and_then(|r| tab.mixes.get(r)).copied(), mix_b);

            // CyclePaste replaces that row with A (older entry).
            tab.handle_action(&Action::Edit(EditOp::CyclePaste), &mut regs);
            assert_eq!(paste_row.and_then(|r| tab.mixes.get(r)).copied(), mix_a);
        }

        #[rstest]
        fn no_op_without_prior_paste(mut regs: RegisterStore) {
            let mut tab = ModTab::new();
            let before_len = tab.mixes.len();

            tab.handle_action(&Action::Edit(EditOp::YankRow(None)), &mut regs);
            tab.handle_action(&Action::Edit(EditOp::YankRow(None)), &mut regs);
            tab.handle_action(&Action::Edit(EditOp::CyclePaste), &mut regs);

            assert_eq!(tab.mixes.len(), before_len);
        }

        #[rstest]
        fn no_op_when_ring_has_single_entry(mut regs: RegisterStore) {
            let mut tab = ModTab::new();
            let cursor = tab.table_state.selected();

            tab.handle_action(&Action::Edit(EditOp::YankRow(None)), &mut regs);
            tab.handle_action(&Action::Edit(EditOp::Paste(None)), &mut regs);

            let paste_row = tab.table_state.selected();
            let pasted_mix = paste_row.and_then(|r| tab.mixes.get(r)).copied();

            tab.handle_action(&Action::Edit(EditOp::CyclePaste), &mut regs);

            assert_eq!(
                paste_row.and_then(|r| tab.mixes.get(r)).copied(),
                pasted_mix
            );
            assert_eq!(
                paste_row.and_then(|r| tab.mixes.get(r)).copied(),
                cursor.and_then(|c| tab.mixes.get(c)).copied(),
            );
        }

        #[rstest]
        fn intervening_move_breaks_chain(mut regs: RegisterStore) {
            let mut tab = ModTab::new();
            let cursor = tab.table_state.selected();

            let mix_a = cursor.map(|c| tab.mixes[c]);
            tab.handle_action(&Action::Edit(EditOp::YankRow(None)), &mut regs);
            tab.handle_action(&Action::Move(Movement::Down), &mut regs);
            tab.handle_action(&Action::Edit(EditOp::YankRow(None)), &mut regs);
            tab.handle_action(&Action::Edit(EditOp::Paste(None)), &mut regs);
            let paste_row = tab.table_state.selected();

            // A Move between Paste and CyclePaste clears last_paste_row.
            tab.handle_action(&Action::Move(Movement::Up), &mut regs);
            tab.handle_action(&Action::Edit(EditOp::CyclePaste), &mut regs);

            // Row at paste_row is unchanged (CyclePaste was a no-op).
            assert_ne!(paste_row.and_then(|r| tab.mixes.get(r)).copied(), mix_a);
        }
    }

    mod mod_color {
        use super::*;

        #[rstest]
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

        #[rstest]
        fn exactly_10m_is_yellow() {
            assert_eq!(
                mod_color(Meters::new(10.0), &Theme::default()),
                Theme::default().caution()
            );
        }

        #[rstest]
        fn between_thresholds_is_yellow() {
            assert_eq!(
                mod_color(Meters::new(15.0), &Theme::default()),
                Theme::default().caution()
            );
        }

        #[rstest]
        fn exactly_20m_is_green() {
            assert_eq!(
                mod_color(Meters::new(20.0), &Theme::default()),
                Theme::default().safe()
            );
        }

        #[rstest]
        fn above_20m_is_green() {
            assert_eq!(
                mod_color(Meters::new(33.75), &Theme::default()),
                Theme::default().safe()
            );
        }
    }

    mod action_dispatch {
        use super::*;

        #[rstest]
        fn down_advances_row() {
            let mut tab = ModTab::new();
            let start = tab.table_state.selected();

            tab.handle_action(&Action::Move(Movement::Down), &mut RegisterStore::default());

            assert_eq!(tab.table_state.selected(), start.map(|s| s + 1));
        }

        #[rstest]
        fn down_clamped_at_last_mix() {
            let mut tab = ModTab::new();

            tab.handle_action(
                &Action::Move(Movement::GotoBottom),
                &mut RegisterStore::default(),
            );
            tab.handle_action(&Action::Move(Movement::Down), &mut RegisterStore::default());

            assert_eq!(tab.table_state.selected(), Some(tab.mixes.len() - 1));
        }

        #[rstest]
        fn up_retreats_row() {
            let mut tab = ModTab::new();
            let start = tab.table_state.selected();

            tab.handle_action(&Action::Move(Movement::Down), &mut RegisterStore::default());
            tab.handle_action(&Action::Move(Movement::Up), &mut RegisterStore::default());

            assert_eq!(tab.table_state.selected(), start);
        }

        #[rstest]
        fn up_clamped_at_zero() {
            let mut tab = ModTab::new();

            tab.handle_action(
                &Action::Move(Movement::GotoTop),
                &mut RegisterStore::default(),
            );
            tab.handle_action(&Action::Move(Movement::Up), &mut RegisterStore::default());

            assert_eq!(tab.table_state.selected(), Some(0));
        }

        #[rstest]
        fn goto_top_selects_first_row() {
            let mut tab = ModTab::new();

            for _ in 0..10 {
                tab.handle_action(&Action::Move(Movement::Down), &mut RegisterStore::default());
            }

            tab.handle_action(
                &Action::Move(Movement::GotoTop),
                &mut RegisterStore::default(),
            );

            assert_eq!(tab.table_state.selected(), Some(0));
        }

        #[rstest]
        fn goto_bottom_selects_last_row() {
            let mut tab = ModTab::new();

            tab.handle_action(
                &Action::Move(Movement::GotoBottom),
                &mut RegisterStore::default(),
            );

            assert_eq!(tab.table_state.selected(), Some(tab.mixes.len() - 1));
        }

        #[rstest]
        fn scroll_down_moves_by_delta() {
            let mut tab = ModTab::new();
            let start = tab.table_state.selected();

            tab.handle_action(
                &Action::Move(Movement::ScrollDown),
                &mut RegisterStore::default(),
            );

            assert_eq!(
                tab.table_state.selected(),
                start.map(|s| s + SCROLL_DELTA as usize),
            );
        }

        #[rstest]
        fn scroll_up_moves_by_delta() {
            let mut tab = ModTab::new();
            tab.handle_action(
                &Action::Move(Movement::GotoBottom),
                &mut RegisterStore::default(),
            );

            let start = tab.table_state.selected();
            tab.handle_action(
                &Action::Move(Movement::ScrollUp),
                &mut RegisterStore::default(),
            );

            assert_eq!(
                tab.table_state.selected(),
                start.map(|s| s - SCROLL_DELTA as usize),
            );
        }

        #[rstest]
        fn page_down_moves_by_page_delta() {
            let mut tab = ModTab::new();
            let start = tab.table_state.selected();

            tab.handle_action(
                &Action::Move(Movement::PageDown),
                &mut RegisterStore::default(),
            );

            assert_eq!(
                tab.table_state.selected(),
                start.map(|s| s + PAGE_DELTA as usize),
            );
        }

        #[rstest]
        fn page_up_moves_by_page_delta() {
            let mut tab = ModTab::new();
            tab.handle_action(
                &Action::Move(Movement::GotoBottom),
                &mut RegisterStore::default(),
            );

            let start = tab.table_state.selected();
            tab.handle_action(
                &Action::Move(Movement::PageUp),
                &mut RegisterStore::default(),
            );

            assert_eq!(
                tab.table_state.selected(),
                start.map(|s| s - PAGE_DELTA as usize),
            );
        }

        #[rstest]
        fn right_increments_ppo2() {
            let mut tab = ModTab::new();
            let before = tab.ppo2_idx;

            tab.handle_action(
                &Action::Move(Movement::Right),
                &mut RegisterStore::default(),
            );

            assert_eq!(tab.ppo2_idx, before + 1);
        }

        #[rstest]
        fn right_clamped_at_max_ppo2() {
            let mut tab = ModTab::new();

            for _ in 0..=PPO2_MAX_IDX {
                tab.handle_action(
                    &Action::Move(Movement::Right),
                    &mut RegisterStore::default(),
                );
            }

            assert_eq!(tab.ppo2_idx, PPO2_MAX_IDX);
        }

        #[rstest]
        fn left_decrements_ppo2() {
            let mut tab = ModTab::new();
            tab.handle_action(
                &Action::Move(Movement::Right),
                &mut RegisterStore::default(),
            );

            let before = tab.ppo2_idx;
            tab.handle_action(&Action::Move(Movement::Left), &mut RegisterStore::default());

            assert_eq!(tab.ppo2_idx, before - 1);
        }

        #[rstest]
        fn left_clamped_at_zero_ppo2() {
            let mut tab = ModTab::new();

            for _ in 0..=PPO2_DEFAULT_IDX {
                tab.handle_action(&Action::Move(Movement::Left), &mut RegisterStore::default());
            }

            assert_eq!(tab.ppo2_idx, 0);
        }

        #[rstest]
        fn none_is_a_noop() {
            let mut tab = ModTab::new();
            let before = tab.table_state.selected();

            tab.handle_action(&Action::None, &mut RegisterStore::default());

            assert_eq!(tab.table_state.selected(), before);
        }

        #[rstest]
        fn quit_is_a_noop() {
            let mut tab = ModTab::new();
            let before = tab.table_state.selected();

            tab.handle_action(&Action::Quit, &mut RegisterStore::default());

            assert_eq!(tab.table_state.selected(), before);
        }
    }

    mod render {
        use super::*;

        #[rstest]
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
