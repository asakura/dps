//! ppO₂-by-depth table component.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Cell, Paragraph, Row, TableState},
};

use crate::{
    action::Action,
    gas::Ean,
    ui::{build_header_row, col_window_size, styled_table, trailing_constraints, window_start},
    units::Meters,
};

use super::Component;

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
        }
    }

    fn selected_mix(&self) -> Ean {
        Ean::from_percent(PPO2_TABLE_MIX_PERCENTS[self.mix_idx])
            .expect("PPO2_TABLE_MIX_PERCENTS values are valid")
    }

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

    fn mix_window_col(&self, window_size: usize) -> usize {
        self.mix_idx - window_start(self.mix_idx, PPO2_TABLE_MIX_COUNT, window_size)
    }
}

fn ppo2_cell_color(ppo2: f64) -> Color {
    if !(PPO2_HYPOXIC_BELOW..PPO2_DANGER_FROM).contains(&ppo2) {
        Color::Red
    } else if ppo2 >= PPO2_CAUTION_FROM {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn build_rows(mixes: &[Ean]) -> Vec<Row<'static>> {
    (0..=PPO2_TABLE_DEPTH_MAX)
        .map(|d| {
            let depth = Meters::new(d as f64);
            let mut cells = vec![Cell::from(format!("{:>3} m", d))];
            for mix in mixes {
                let ppo2 = mix.ppo2_at(depth);
                cells.push(
                    Cell::from(format!("{:.2}", ppo2.value()))
                        .style(Style::default().fg(ppo2_cell_color(ppo2.value()))),
                );
            }
            Row::new(cells)
        })
        .collect()
}

impl Component for PpO2Tab {
    fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let next = self.table_state.selected()
                    .map(|i| (i + 1).min(PPO2_TABLE_DEPTH_MAX))
                    .unwrap_or(0);
                self.table_state.select(Some(next));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let prev = self.table_state.selected()
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(0);
                self.table_state.select(Some(prev));
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.mix_idx = (self.mix_idx + 1).min(PPO2_TABLE_MIX_COUNT - 1);
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.mix_idx = self.mix_idx.saturating_sub(1);
            }
            _ => {}
        }
        Action::None
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        let window_size = col_window_size(
            area.width, PPO2_TABLE_OVERHEAD_W, COL_PPO2_MIX_W, PPO2_TABLE_MIX_COUNT,
        );
        let col_in_window = self.mix_window_col(window_size);
        self.table_state.select_column(Some(col_in_window + FIXED_COL_COUNT));
        let mixes = self.visible_cols(window_size);
        let mix = self.selected_mix();
        let title = format!(" DPS — ppO\u{2082} by Depth   {}% ", mix.o2_percent());
        let constraints = trailing_constraints(
            &[Constraint::Length(COL_DEPTH_W)],
            mixes.len(),
            COL_PPO2_MIX_W,
        );
        let bold = Style::default().add_modifier(Modifier::BOLD);
        let header = build_header_row(
            vec![Cell::from("Depth").style(bold)],
            mixes.iter().map(|m| format!("{:>3}%", m.o2_percent())),
            Some(col_in_window),
        );
        let table = styled_table(build_rows(&mixes), constraints, header, title);
        f.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn status_bar(&self) -> Paragraph<'static> {
        let depth_m = self.table_state.selected().unwrap_or(0);
        let mix = self.selected_mix();
        let depth = Meters::new(depth_m as f64);
        let ppo2 = mix.ppo2_at(depth);
        let name = mix.label()
            .map(|s| format!("{} ", s))
            .unwrap_or_default();
        let text = format!(
            " \u{25c6} {}({}%)  @ {} m  \u{2192}  ppO\u{2082} {:.2} bar",
            name, mix.o2_percent(), depth_m, ppo2.value()
        );
        Paragraph::new(text)
            .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
    }

    fn help_text(&self) -> &'static str {
        " \u{2191}\u{2193}/jk depth   \u{2190}\u{2192}/hl mix   Tab next table   q quit"
    }
}
