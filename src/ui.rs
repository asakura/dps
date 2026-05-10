//! Terminal UI rendering.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::{
    app::{ActiveTab, App, PPO2_COUNT, PPO2_TABLE_DEPTH_MAX, PPO2_TABLE_MIX_COUNT},
    gas::Ean,
    units::{Bar, Meters},
};

const COL_NAME_W: u16 = 12;
const COL_O2_W: u16 = 6;
const COL_MOD_W: u16 = 9;
// Fixed columns (Name, O₂%) that precede the ppO₂ data columns.
const FIXED_COL_COUNT: usize = 2;
// Width consumed by borders, highlight symbol, fixed columns, and their spacings.
const TABLE_OVERHEAD_W: u16 = 2 + 2 + COL_NAME_W + 1 + COL_O2_W + 1;

// MOD depth thresholds for colour coding.
const MOD_RED_BELOW_M: f64 = 10.0;
const MOD_YELLOW_BELOW_M: f64 = 20.0;

const COL_DEPTH_W: u16 = 7;
const COL_PPO2_MIX_W: u16 = 7;
// Width consumed by borders, highlight symbol, depth column, and its spacing.
const PPO2_TABLE_OVERHEAD_W: u16 = 2 + 2 + COL_DEPTH_W + 1;

// ppO₂ thresholds for colour coding in the ppO₂ table.
const PPO2_HYPOXIC_BELOW: f64 = 0.18;
const PPO2_CAUTION_FROM: f64 = 1.4;
const PPO2_DANGER_FROM: f64 = 1.6;

/// Draws the full UI: active table, status bar, and help line.
pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    match app.active_tab {
        ActiveTab::Mod => {
            let window_size = ppo2_window_size(area.width);
            let col_in_window = app.ppo2_window_col(window_size);
            app.table_state
                .select_column(Some(col_in_window + FIXED_COL_COUNT));
            let cols = app.visible_columns(window_size);
            let title = format!(" DPS — MOD Table   ppO\u{2082} {} ", app.ppo2());
            let table = build_table(&app.mixes, &cols, Some(col_in_window), title);
            f.render_stateful_widget(table, chunks[0], &mut app.table_state);
            f.render_widget(selection_bar(app.selection), chunks[1]);
        }
        ActiveTab::PpO2 => {
            let window_size = ppo2_mix_window_size(area.width);
            let col_in_window = app.ppo2_mix_window_col(window_size);
            app.ppo2_table_state.select_column(Some(col_in_window + 1));
            let mixes = app.ppo2_mix_visible_cols(window_size);
            let mix = app.ppo2_selected_mix();
            let title = format!(" DPS — ppO\u{2082} by Depth   {}% ", mix.o2_percent());
            let table = build_ppo2_table(&mixes, Some(col_in_window), title);
            let depth_m = app.ppo2_table_state.selected().unwrap_or(0);
            f.render_stateful_widget(table, chunks[0], &mut app.ppo2_table_state);
            f.render_widget(ppo2_status_bar(depth_m, mix), chunks[1]);
        }
    }
    f.render_widget(help_bar(&app.active_tab), chunks[2]);
}

fn trailing_constraints(fixed: &[Constraint], n: usize, col_w: u16) -> Vec<Constraint> {
    let mut c = fixed.to_vec();
    for i in 0..n {
        c.push(if i + 1 < n { Constraint::Length(col_w) } else { Constraint::Fill(1) });
    }
    c
}

fn build_table(mixes: &[Ean], cols: &[Bar], highlighted: Option<usize>, title: String) -> Table<'static> {
    let constraints = trailing_constraints(
        &[Constraint::Length(COL_NAME_W), Constraint::Length(COL_O2_W)],
        cols.len(),
        COL_MOD_W,
    );

    Table::new(build_rows(mixes, cols), constraints)
        .header(build_header(cols, highlighted))
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .column_highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ")
}

fn build_header(cols: &[Bar], highlighted: Option<usize>) -> Row<'static> {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let bold_ul = Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

    let mut cells = vec![
        Cell::from("Name").style(bold),
        Cell::from("O\u{2082}%").style(bold),
    ];
    for (i, col) in cols.iter().enumerate() {
        let style = if highlighted == Some(i) { bold_ul } else { bold };
        cells.push(Cell::from(format!("{}", col)).style(style));
    }
    Row::new(cells).style(Style::default().fg(Color::Cyan))
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

fn selection_bar(selection: Option<(Ean, Bar)>) -> Paragraph<'static> {
    match selection {
        Some((mix, ppo2)) => {
            let depth = mix.mod_at(ppo2);
            let name = mix.label()
                .map(|s| format!("{} ", s))
                .unwrap_or_default();
            let text = format!(
                " \u{25c6} {}({}%)  MOD {}  @ ppO\u{2082} {}",
                name,
                mix.o2_percent(),
                depth,
                ppo2
            );
            Paragraph::new(text)
                .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        }
        None => Paragraph::new(" No gas selected — press Enter to select")
            .style(Style::default().fg(Color::DarkGray)),
    }
}

fn help_bar(active_tab: &ActiveTab) -> Paragraph<'static> {
    let text = match active_tab {
        ActiveTab::Mod => {
            " \u{2191}\u{2193}/jk navigate   \u{2190}\u{2192}/hl ppO\u{2082}   Enter select   Tab next table   q quit"
        }
        ActiveTab::PpO2 => {
            " \u{2191}\u{2193}/jk depth   \u{2190}\u{2192}/hl mix   Tab next table   q quit"
        }
    };
    Paragraph::new(text).style(Style::default().fg(Color::DarkGray))
}

/// How many ppO₂ columns fit in `width` terminal columns.
/// Each ppO₂ column is COL_MOD_W wide; each adds COL_MOD_W+1 beyond the first (extra spacing).
///
/// WARNING: TABLE_OVERHEAD_W must reflect the exact fixed-column layout in `build_table`
/// (borders, highlight symbol, Name, O₂%, and their spacings). If that layout changes,
/// update TABLE_OVERHEAD_W or this function will silently return the wrong count.
fn ppo2_window_size(width: u16) -> usize {
    let n = 1 + width.saturating_sub(TABLE_OVERHEAD_W + COL_MOD_W) / (COL_MOD_W + 1);
    (n as usize).min(PPO2_COUNT)
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

/// How many EAN mix columns fit in `width` terminal columns for the ppO₂ table.
fn ppo2_mix_window_size(width: u16) -> usize {
    let n = 1 + width.saturating_sub(PPO2_TABLE_OVERHEAD_W + COL_PPO2_MIX_W) / (COL_PPO2_MIX_W + 1);
    (n as usize).min(PPO2_TABLE_MIX_COUNT)
}

fn build_ppo2_table(mixes: &[Ean], highlighted: Option<usize>, title: String) -> Table<'static> {
    let constraints = trailing_constraints(
        &[Constraint::Length(COL_DEPTH_W)],
        mixes.len(),
        COL_PPO2_MIX_W,
    );
    Table::new(build_ppo2_rows(mixes), constraints)
        .header(build_ppo2_header(mixes, highlighted))
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .column_highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ")
}

fn build_ppo2_header(mixes: &[Ean], highlighted: Option<usize>) -> Row<'static> {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let bold_ul = Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    let mut cells = vec![Cell::from("Depth").style(bold)];
    for (i, mix) in mixes.iter().enumerate() {
        let style = if highlighted == Some(i) { bold_ul } else { bold };
        cells.push(Cell::from(format!("{:>3}%", mix.o2_percent())).style(style));
    }
    Row::new(cells).style(Style::default().fg(Color::Cyan))
}

fn build_ppo2_rows(mixes: &[Ean]) -> Vec<Row<'static>> {
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

fn ppo2_status_bar(depth_m: usize, mix: Ean) -> Paragraph<'static> {
    let depth = Meters::new(depth_m as f64);
    let ppo2 = mix.ppo2_at(depth);
    let name = mix.label()
        .map(|s| format!("{} ", s))
        .unwrap_or_default();
    let text = format!(
        " \u{25c6} {}({}%)  @ {} m  \u{2192}  ppO\u{2082} {:.2} bar",
        name,
        mix.o2_percent(),
        depth_m,
        ppo2.value()
    );
    Paragraph::new(text).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
}

fn ppo2_cell_color(ppo2: f64) -> Color {
    if ppo2 < PPO2_HYPOXIC_BELOW || ppo2 >= PPO2_DANGER_FROM {
        Color::Red
    } else if ppo2 >= PPO2_CAUTION_FROM {
        Color::Yellow
    } else {
        Color::Green
    }
}
