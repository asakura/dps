//! DPS — interactive terminal MOD table for nitrox dive planning.
mod action;
mod app;
mod components;
mod gas;
mod ui;
mod units;

use std::{io, time::Duration};

use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use action::Action;
use app::App;

/// How long to block waiting for a terminal event before redrawing.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// RAII guard that owns the terminal and restores it on drop.
struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl Tui {
    /// Switches the terminal into raw mode and the alternate screen, then creates the backend.
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
        Ok(Self { terminal })
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn main() -> io::Result<()> {
    let mut tui = Tui::new()?;
    let mut app = App::new();

    loop {
        tui.terminal.draw(|f| app.render(f))?;

        if !event::poll(POLL_INTERVAL)? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        if matches!(app.handle_key(key), Action::Quit) {
            break;
        }
    }

    Ok(())
}
