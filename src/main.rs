use std::{io, panic, time::Duration};

use clap::Parser;
use color_eyre::Result;
use crossterm::{
    cursor::Show,
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use dps::action::Action;
use dps::app::App;
use dps::cli::Cli;
use dps::logging::initialize_logging;

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, Show)?;
    Ok(())
}

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
        Self::install_panic_hook();
        Ok(Self { terminal })
    }

    /// Wraps the existing panic hook (color-eyre's) so the terminal is restored before it runs.
    fn install_panic_hook() {
        let hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let _ = restore_terminal();
            hook(info);
        }));
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    initialize_logging()?;
    let cli = Cli::parse();
    let poll_interval = Duration::from_secs_f64(1.0 / cli.frame_rate);
    let mut tui = Tui::new()?;
    let mut app = App::new();

    loop {
        tui.terminal.draw(|f| app.render(f))?;

        if !event::poll(poll_interval)? {
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
