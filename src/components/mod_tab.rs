//! MOD-by-ppO₂ table component.

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
    units::Bar,
};

use super::Component;

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

pub struct ModTab {
    mixes: Vec<Ean>,
    table_state: TableState,
    ppo2_idx: usize,
    selection: Option<(Ean, Bar)>,
}

impl ModTab {
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

    fn visible_columns(&self, window_size: usize) -> Vec<Bar> {
        let start = window_start(self.ppo2_idx, PPO2_COUNT, window_size);
        let count = window_size.min(PPO2_COUNT);
        (0..count)
            .map(|i| Bar::new(PPO2_MIN + (start + i) as f64 * PPO2_STEP))
            .collect()
    }

    fn ppo2_window_col(&self, window_size: usize) -> usize {
        self.ppo2_idx - window_start(self.ppo2_idx, PPO2_COUNT, window_size)
    }
}

fn mod_color(depth_m: f64) -> Color {
    if depth_m < MOD_RED_BELOW_M {
        Color::Red
    } else if depth_m < MOD_YELLOW_BELOW_M {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn build_rows(mixes: &[Ean], cols: &[Bar]) -> Vec<Row<'static>> {
    mixes
        .iter()
        .map(|mix| {
            let mut cells = vec![
                Cell::from(mix.label().unwrap_or("")),
                Cell::from(format!("{:>4}%", mix.o2_percent())),
            ];
            for &col in cols.iter() {
                let depth = mix.mod_at(col);
                cells.push(
                    Cell::from(format!("{}", depth))
                        .style(Style::default().fg(mod_color(depth.value()))),
                );
            }
            Row::new(cells)
        })
        .collect()
}

impl Component for ModTab {
    fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let next = self.table_state.selected()
                    .map(|i| (i + 1).min(self.mixes.len() - 1))
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
                self.ppo2_idx = (self.ppo2_idx + 1).min(PPO2_MAX_IDX);
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.ppo2_idx = self.ppo2_idx.saturating_sub(1);
            }
            KeyCode::Enter => {
                if let Some(row) = self.table_state.selected() {
                    self.selection = Some((self.mixes[row], self.ppo2()));
                }
            }
            _ => {}
        }
        Action::None
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        let window_size = col_window_size(area.width, TABLE_OVERHEAD_W, COL_MOD_W, PPO2_COUNT);
        let col_in_window = self.ppo2_window_col(window_size);
        self.table_state.select_column(Some(col_in_window + FIXED_COL_COUNT));
        let cols = self.visible_columns(window_size);
        let title = format!(" DPS — MOD Table   ppO\u{2082} {} ", self.ppo2());
        let constraints = trailing_constraints(
            &[Constraint::Length(COL_NAME_W), Constraint::Length(COL_O2_W)],
            cols.len(),
            COL_MOD_W,
        );
        let bold = Style::default().add_modifier(Modifier::BOLD);
        let header = build_header_row(
            vec![Cell::from("Name").style(bold), Cell::from("O\u{2082}%").style(bold)],
            cols.iter().map(|c| format!("{}", c)),
            Some(col_in_window),
        );
        let table = styled_table(build_rows(&self.mixes, &cols), constraints, header, title);
        f.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn status_bar(&self) -> Paragraph<'static> {
        match self.selection {
            Some((mix, ppo2)) => {
                let depth = mix.mod_at(ppo2);
                let name = mix.label()
                    .map(|s| format!("{} ", s))
                    .unwrap_or_default();
                let text = format!(
                    " \u{25c6} {}({}%)  MOD {}  @ ppO\u{2082} {}",
                    name, mix.o2_percent(), depth, ppo2
                );
                Paragraph::new(text)
                    .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
            }
            None => Paragraph::new(" No gas selected — press Enter to select")
                .style(Style::default().fg(Color::DarkGray)),
        }
    }

    fn help_text(&self) -> &'static str {
        " \u{2191}\u{2193}/jk navigate   \u{2190}\u{2192}/hl ppO\u{2082}   Enter select   Tab next table   q quit"
    }
}
