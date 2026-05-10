//! DPS — interactive terminal MOD table for nitrox dive planning.
mod app;
mod gas;
mod ui;
mod units;

use std::{io, time::Duration};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use app::{ActiveTab, App};

const POLL_INTERVAL: Duration = Duration::from_millis(50);

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new();

    loop {
        terminal.draw(|f| ui::render(f, &mut app))?;

        if !event::poll(POLL_INTERVAL)? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => break,
            KeyCode::Tab => app.switch_tab(),
            KeyCode::Down | KeyCode::Char('j') => match app.active_tab {
                ActiveTab::Mod => app.move_down(),
                ActiveTab::PpO2 => app.ppo2_table_move_down(),
            },
            KeyCode::Up | KeyCode::Char('k') => match app.active_tab {
                ActiveTab::Mod => app.move_up(),
                ActiveTab::PpO2 => app.ppo2_table_move_up(),
            },
            KeyCode::Right | KeyCode::Char('l') => match app.active_tab {
                ActiveTab::Mod => app.ppo2_next(),
                ActiveTab::PpO2 => app.ppo2_mix_next(),
            },
            KeyCode::Left | KeyCode::Char('h') => match app.active_tab {
                ActiveTab::Mod => app.ppo2_prev(),
                ActiveTab::PpO2 => app.ppo2_mix_prev(),
            },
            KeyCode::Enter => {
                if app.active_tab == ActiveTab::Mod {
                    app.select();
                }
            }
            _ => {}
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
