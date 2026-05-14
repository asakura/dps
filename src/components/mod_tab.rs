//! MOD-by-ppO₂ table component.

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
    units::Bar,
};

use super::{Component, KeyBinding};

const PPO2_MIN: f64 = 0.8;
const PPO2_STEP: f64 = 0.1;
const PPO2_MAX_IDX: usize = 8;
const PPO2_DEFAULT_IDX: usize = 6;
const PPO2_COUNT: usize = PPO2_MAX_IDX + 1;

const O2_PCT_MIN: u8 = 10;
const O2_PCT_MAX: u8 = 100;
const DEFAULT_MIX_O2_PCT: u8 = 32;

const COL_NAME_W: u16 = 12;
const COL_O2_W: u16 = 6;
const COL_MOD_W: u16 = 9;
const FIXED_COL_COUNT: usize = 2;
const TABLE_OVERHEAD_W: u16 = 2 + 2 + COL_NAME_W + 1 + COL_O2_W + 1;

const MOD_RED_BELOW_M: f64 = 10.0;
const MOD_YELLOW_BELOW_M: f64 = 20.0;

/// MOD-by-ppO₂ table: maximum operating depth for each nitrox mix at the selected ppO₂ limit.
pub struct ModTab {
    mixes: Vec<Ean>,
    table_state: TableState,
    ppo2_idx: usize,
    selection: Option<(Ean, Bar)>,
}

impl Default for ModTab {
    fn default() -> Self {
        Self::new()
    }
}

impl ModTab {
    /// Creates a `ModTab` pre-selected on EAN32 at 1.4 bar ppO₂.
    pub fn new() -> Self {
        let mixes: Vec<Ean> = (O2_PCT_MIN..=O2_PCT_MAX)
            .map(|p| Ean::from_percent(p).expect("10..=100 is always valid"))
            .collect();
        let start_idx = mixes
            .iter()
            .position(|m| m.o2_percent() == DEFAULT_MIX_O2_PCT)
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

    fn ppo2(&self) -> Bar {
        Bar::new(PPO2_MIN + self.ppo2_idx as f64 * PPO2_STEP)
    }

    /// ppO₂ column values for a sliding window of `window_size` columns centred on the selected index.
    fn visible_columns(&self, window_size: usize) -> Vec<Bar> {
        let start = window_start(self.ppo2_idx, PPO2_COUNT, window_size);
        let count = window_size.min(PPO2_COUNT);
        (0..count)
            .map(|i| Bar::new(PPO2_MIN + (start + i) as f64 * PPO2_STEP))
            .collect()
    }

    /// Column index of the selected ppO₂ within the visible window (0-based).
    fn ppo2_window_col(&self, window_size: usize) -> usize {
        self.ppo2_idx - window_start(self.ppo2_idx, PPO2_COUNT, window_size)
    }

    fn move_row(&mut self, delta: isize) {
        let last = self.mixes.len().saturating_sub(1) as isize;
        let next = self
            .table_state
            .selected()
            .map(|i| (i as isize + delta).clamp(0, last) as usize)
            .unwrap_or(0);
        self.table_state.select(Some(next));
    }

    fn build_rows(&self, cols: &[Bar]) -> Vec<Row<'static>> {
        self.mixes
            .iter()
            .map(|mix| ModRow { mix, cols }.into())
            .collect()
    }
}

struct ModRow<'a> {
    mix: &'a Ean,
    cols: &'a [Bar],
}

impl From<ModRow<'_>> for Row<'static> {
    fn from(r: ModRow<'_>) -> Row<'static> {
        let mut cells = vec![
            Cell::from(r.mix.label().unwrap_or("")),
            Cell::from(format!("{:>4}%", r.mix.o2_percent())),
        ];
        for &col in r.cols {
            let depth = r.mix.mod_at(col);
            cells.push(Cell::from(format!("{}", depth)).style(mod_color(depth.value())));
        }
        Row::new(cells)
    }
}

fn mod_color(depth_m: f64) -> Color {
    if depth_m < MOD_RED_BELOW_M {
        THEME.red
    } else if depth_m < MOD_YELLOW_BELOW_M {
        THEME.yellow
    } else {
        THEME.green
    }
}

impl Widget for &mut ModTab {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let window_size = col_window_size(area.width, TABLE_OVERHEAD_W, COL_MOD_W, PPO2_COUNT);
        let col_in_window = self.ppo2_window_col(window_size);
        self.table_state
            .select_column(Some(col_in_window + FIXED_COL_COUNT));
        let cols = self.visible_columns(window_size);
        let title = format!(" DPS — MOD Table   ppO\u{2082} {} ", self.ppo2());
        let constraints = trailing_constraints(
            &[Constraint::Length(COL_NAME_W), Constraint::Length(COL_O2_W)],
            cols.len(),
            COL_MOD_W,
        );
        let header = build_header_row(
            vec![
                Cell::from("Name").style(THEME.header_cell()),
                Cell::from("O\u{2082}%").style(THEME.header_cell()),
            ],
            cols.iter().map(|c| c.to_string()),
            Some(col_in_window),
        );
        let table = styled_table(self.build_rows(&cols), constraints, header, title);
        StatefulWidget::render(table, area, buf, &mut self.table_state);
    }
}

struct ModTabStatus<'a>(&'a ModTab);

impl Widget for ModTabStatus<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.0.selection {
            Some((mix, ppo2)) => {
                let depth = mix.mod_at(ppo2);
                let name = mix.label().map(|s| format!("{} ", s)).unwrap_or_default();
                let text = format!(
                    " \u{25c6} {}({}%)  MOD {}  @ ppO\u{2082} {}",
                    name,
                    mix.o2_percent(),
                    depth,
                    ppo2,
                );
                Paragraph::new(text)
                    .style(THEME.status_active())
                    .render(area, buf);
            }
            None => Paragraph::new(" No gas selected — press Enter to select")
                .style(THEME.status_empty())
                .render(area, buf),
        }
    }
}

const SCROLL_DELTA: isize = 10;
const PAGE_DELTA: isize = 20;

impl Component for ModTab {
    fn title(&self) -> &'static str {
        "MOD Table"
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Down => self.move_row(1),
            Action::Up => self.move_row(-1),
            Action::Right => self.ppo2_idx = (self.ppo2_idx + 1).min(PPO2_MAX_IDX),
            Action::Left => self.ppo2_idx = self.ppo2_idx.saturating_sub(1),
            Action::ScrollDown => self.move_row(SCROLL_DELTA),
            Action::ScrollUp => self.move_row(-SCROLL_DELTA),
            Action::PageDown => self.move_row(PAGE_DELTA),
            Action::PageUp => self.move_row(-PAGE_DELTA),
            Action::GotoTop => self.table_state.select(Some(0)),
            Action::GotoBottom => self.table_state.select(Some(self.mixes.len() - 1)),
            Action::Select => {
                if let Some(row) = self.table_state.selected() {
                    self.selection = Some((self.mixes[row], self.ppo2()));
                }
            }
            _ => {}
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        Widget::render(self, area, buf);
    }

    fn render_status(&self, area: Rect, buf: &mut Buffer) {
        ModTabStatus(self).render(area, buf);
    }

    fn key_bindings(&self) -> &'static [KeyBinding] {
        static BINDINGS: &[KeyBinding] = &[
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
        fn selected_row_is_ean32() {
            let tab = ModTab::new();
            let idx = tab.table_state.selected().unwrap();
            assert_eq!(tab.mixes[idx].o2_percent(), DEFAULT_MIX_O2_PCT);
        }

        #[test]
        fn ppo2_is_1_4_bar() {
            let tab = ModTab::new();
            assert!((tab.ppo2().value() - 1.4).abs() < 1e-9);
        }

        #[test]
        fn no_selection() {
            assert!(ModTab::new().selection.is_none());
        }
    }

    mod enter_key {
        use super::*;

        #[test]
        fn stores_current_mix_and_ppo2() {
            let mut tab = ModTab::new();
            let row = tab.table_state.selected().unwrap();
            let expected_pct = tab.mixes[row].o2_percent();
            let expected_ppo2 = tab.ppo2().value();
            tab.handle_action(Action::Select);
            let (mix, ppo2) = tab.selection.unwrap();
            assert_eq!(mix.o2_percent(), expected_pct);
            assert!((ppo2.value() - expected_ppo2).abs() < 1e-9);
        }

        #[test]
        fn selection_updates_after_moving_row() {
            let mut tab = ModTab::new();
            tab.handle_action(Action::Select);
            let first_pct = tab.selection.unwrap().0.o2_percent();
            tab.handle_action(Action::Down);
            tab.handle_action(Action::Select);
            let second_pct = tab.selection.unwrap().0.o2_percent();
            assert_ne!(first_pct, second_pct);
        }
    }

    mod mod_color_fn {
        use super::*;

        #[test]
        fn below_10m_is_red() {
            assert_eq!(mod_color(9.9), THEME.red);
            assert_eq!(mod_color(0.0), THEME.red);
        }

        #[test]
        fn exactly_10m_is_yellow() {
            assert_eq!(mod_color(10.0), THEME.yellow);
        }

        #[test]
        fn between_thresholds_is_yellow() {
            assert_eq!(mod_color(15.0), THEME.yellow);
        }

        #[test]
        fn exactly_20m_is_green() {
            assert_eq!(mod_color(20.0), THEME.green);
        }

        #[test]
        fn above_20m_is_green() {
            assert_eq!(mod_color(33.75), THEME.green);
        }
    }

    mod status_bar_fn {
        use super::*;

        #[test]
        fn no_selection_shows_prompt() {
            let tab = ModTab::new();
            let text = widget_text(ModTabStatus(&tab), 60);
            assert!(text.contains("No gas"));
        }

        #[test]
        fn selection_shows_mix_percent_and_mod() {
            let mut tab = ModTab::new();
            tab.handle_action(Action::Select);
            let text = widget_text(ModTabStatus(&tab), 60);
            assert!(text.contains("32"));
            assert!(text.contains("MOD"));
        }
    }

    mod action_dispatch {
        use super::*;
        use crate::action::Action;

        #[test]
        fn down_advances_row() {
            let mut tab = ModTab::new();
            let start = tab.table_state.selected().unwrap();
            tab.handle_action(Action::Down);
            assert_eq!(tab.table_state.selected().unwrap(), start + 1);
        }

        #[test]
        fn up_retreats_row() {
            let mut tab = ModTab::new();
            tab.handle_action(Action::Down);
            let after = tab.table_state.selected().unwrap();
            tab.handle_action(Action::Up);
            assert_eq!(tab.table_state.selected().unwrap(), after - 1);
        }

        #[test]
        fn goto_top_selects_first_row() {
            let mut tab = ModTab::new();
            for _ in 0..10 {
                tab.handle_action(Action::Down);
            }
            tab.handle_action(Action::GotoTop);
            assert_eq!(tab.table_state.selected().unwrap(), 0);
        }

        #[test]
        fn goto_bottom_selects_last_row() {
            let mut tab = ModTab::new();
            tab.handle_action(Action::GotoBottom);
            assert_eq!(tab.table_state.selected().unwrap(), tab.mixes.len() - 1);
        }

        #[test]
        fn scroll_down_moves_by_delta() {
            let mut tab = ModTab::new();
            tab.handle_action(Action::ScrollDown);
            assert_eq!(
                tab.table_state.selected().unwrap(),
                tab.mixes
                    .iter()
                    .position(|m| m.o2_percent() == DEFAULT_MIX_O2_PCT)
                    .unwrap()
                    + SCROLL_DELTA as usize,
            );
        }

        #[test]
        fn page_down_moves_by_page_delta() {
            let mut tab = ModTab::new();
            let start = tab.table_state.selected().unwrap();
            tab.handle_action(Action::PageDown);
            assert_eq!(
                tab.table_state.selected().unwrap(),
                start + PAGE_DELTA as usize,
            );
        }

        #[test]
        fn right_increments_ppo2() {
            let mut tab = ModTab::new();
            let before = tab.ppo2_idx;
            tab.handle_action(Action::Right);
            assert_eq!(tab.ppo2_idx, before + 1);
        }

        #[test]
        fn left_decrements_ppo2() {
            let mut tab = ModTab::new();
            tab.handle_action(Action::Right);
            let before = tab.ppo2_idx;
            tab.handle_action(Action::Left);
            assert_eq!(tab.ppo2_idx, before - 1);
        }
    }
}
