//! Application state and input logic.

use ratatui::widgets::TableState;

use crate::{gas::Ean, units::Bar};

// ppO₂ cursor step size and range.
pub const PPO2_MIN: f64 = 0.8;
pub const PPO2_STEP: f64 = 0.1;
const PPO2_MAX_IDX: usize = 8;        // 8 steps × 0.1 from 0.8 → 1.6 bar
const PPO2_DEFAULT_IDX: usize = 6;    // → 1.4 bar
pub const PPO2_COUNT: usize = PPO2_MAX_IDX + 1;

const O2_PCT_MIN: u8 = 10;
const O2_PCT_MAX: u8 = 100;
const DEFAULT_MIX_O2_PCT: u8 = 32; // EAN32

pub const PPO2_TABLE_MIX_PERCENTS: &[u8] = &[10, 12, 14, 16, 18, 21, 28, 30, 32, 36, 40, 50, 80, 100];
pub const PPO2_TABLE_MIX_COUNT: usize = 14;
pub const PPO2_TABLE_DEPTH_MAX: usize = 80;
const PPO2_MIX_DEFAULT_IDX: usize = 5; // EAN21 (Air)

#[derive(PartialEq)]
pub enum ActiveTab {
    Mod,
    PpO2,
}

/// Top-level application state.
pub struct App {
    /// All nitrox mixes from EAN10 to pure O₂, one per percent.
    pub mixes: Vec<Ean>,
    /// Ratatui cursor tracking which mix row is highlighted.
    pub table_state: TableState,
    /// Index into the ppO₂ range 0.8–1.6 bar (step 0.1); see `ppo2()`.
    pub ppo2_idx: usize,
    /// Gas and ppO₂ limit confirmed by the user via Enter, if any.
    pub selection: Option<(Ean, Bar)>,
    /// Which table is currently displayed.
    pub active_tab: ActiveTab,
    /// Ratatui cursor tracking which depth row is highlighted in the ppO₂ table.
    pub ppo2_table_state: TableState,
    /// Index into PPO2_TABLE_MIX_PERCENTS (column cursor for the ppO₂ table).
    pub ppo2_mix_idx: usize,
}

impl App {
    /// Creates a new `App` pre-selected on EAN32 at 1.4 bar ppO₂.
    pub fn new() -> Self {
        let mixes: Vec<Ean> = (O2_PCT_MIN..=O2_PCT_MAX).map(Ean::from_percent).collect();
        let start_idx = mixes
            .iter()
            .position(|m| m.o2_percent() == DEFAULT_MIX_O2_PCT)
            .unwrap_or(0);
        let mut table_state = TableState::default();
        table_state.select(Some(start_idx));
        let mut ppo2_table_state = TableState::default();
        ppo2_table_state.select(Some(0));
        Self {
            mixes,
            table_state,
            ppo2_idx: PPO2_DEFAULT_IDX,
            selection: None,
            active_tab: ActiveTab::Mod,
            ppo2_table_state,
            ppo2_mix_idx: PPO2_MIX_DEFAULT_IDX,
        }
    }

    /// Returns the currently selected ppO₂ limit.
    pub fn ppo2(&self) -> Bar {
        Bar::new(PPO2_MIN + self.ppo2_idx as f64 * PPO2_STEP)
    }

    fn window_start(&self, window_size: usize) -> usize {
        let half = window_size / 2;
        let max_start = PPO2_COUNT.saturating_sub(window_size);
        self.ppo2_idx.saturating_sub(half).min(max_start)
    }

    /// The ppO₂ values to show as columns for the given window size.
    pub fn visible_columns(&self, window_size: usize) -> Vec<Bar> {
        let start = self.window_start(window_size);
        let count = window_size.min(PPO2_COUNT);
        (0..count)
            .map(|i| Bar::new(PPO2_MIN + (start + i) as f64 * PPO2_STEP))
            .collect()
    }

    /// Position of the ppO₂ cursor within the visible window (for column highlighting).
    pub fn ppo2_window_col(&self, window_size: usize) -> usize {
        self.ppo2_idx - self.window_start(window_size)
    }

    /// Moves the row cursor down by one, clamped to the last mix.
    pub fn move_down(&mut self) {
        let next = self
            .table_state
            .selected()
            .map(|i| (i + 1).min(self.mixes.len() - 1))
            .unwrap_or(0);
        self.table_state.select(Some(next));
    }

    /// Moves the row cursor up by one, clamped to the first mix.
    pub fn move_up(&mut self) {
        let prev = self
            .table_state
            .selected()
            .map(|i| i.saturating_sub(1))
            .unwrap_or(0);
        self.table_state.select(Some(prev));
    }

    /// Increments the ppO₂ cursor by 0.1 bar, clamped to 1.6 bar.
    pub fn ppo2_next(&mut self) {
        self.ppo2_idx = (self.ppo2_idx + 1).min(PPO2_MAX_IDX);
    }

    /// Decrements the ppO₂ cursor by 0.1 bar, clamped to 0.8 bar.
    pub fn ppo2_prev(&mut self) {
        self.ppo2_idx = self.ppo2_idx.saturating_sub(1);
    }

    /// Stores the currently highlighted mix and ppO₂ as the active selection.
    pub fn select(&mut self) {
        if let Some(row) = self.table_state.selected() {
            self.selection = Some((self.mixes[row], self.ppo2()));
        }
    }

    /// Switches to the other table.
    pub fn switch_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Mod => ActiveTab::PpO2,
            ActiveTab::PpO2 => ActiveTab::Mod,
        };
    }

    /// Moves the ppO₂ table depth cursor down by one.
    pub fn ppo2_table_move_down(&mut self) {
        let next = self
            .ppo2_table_state
            .selected()
            .map(|i| (i + 1).min(PPO2_TABLE_DEPTH_MAX))
            .unwrap_or(0);
        self.ppo2_table_state.select(Some(next));
    }

    /// Moves the ppO₂ table depth cursor up by one.
    pub fn ppo2_table_move_up(&mut self) {
        let prev = self
            .ppo2_table_state
            .selected()
            .map(|i| i.saturating_sub(1))
            .unwrap_or(0);
        self.ppo2_table_state.select(Some(prev));
    }

    /// Advances the ppO₂ table mix column cursor right.
    pub fn ppo2_mix_next(&mut self) {
        self.ppo2_mix_idx = (self.ppo2_mix_idx + 1).min(PPO2_TABLE_MIX_COUNT - 1);
    }

    /// Retreats the ppO₂ table mix column cursor left.
    pub fn ppo2_mix_prev(&mut self) {
        self.ppo2_mix_idx = self.ppo2_mix_idx.saturating_sub(1);
    }

    fn ppo2_mix_window_start(&self, window_size: usize) -> usize {
        let half = window_size / 2;
        let max_start = PPO2_TABLE_MIX_COUNT.saturating_sub(window_size);
        self.ppo2_mix_idx.saturating_sub(half).min(max_start)
    }

    /// The EAN mixes to show as columns for the given window size.
    pub fn ppo2_mix_visible_cols(&self, window_size: usize) -> Vec<Ean> {
        let start = self.ppo2_mix_window_start(window_size);
        let count = window_size.min(PPO2_TABLE_MIX_COUNT);
        (0..count)
            .map(|i| Ean::from_percent(PPO2_TABLE_MIX_PERCENTS[start + i]))
            .collect()
    }

    /// Position of the mix column cursor within the visible window.
    pub fn ppo2_mix_window_col(&self, window_size: usize) -> usize {
        self.ppo2_mix_idx - self.ppo2_mix_window_start(window_size)
    }

    /// Returns the currently selected mix in the ppO₂ table.
    pub fn ppo2_selected_mix(&self) -> Ean {
        Ean::from_percent(PPO2_TABLE_MIX_PERCENTS[self.ppo2_mix_idx])
    }
}
