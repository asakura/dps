use std::{
    io, panic,
    time::{Duration, Instant},
};

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
use dps::errors;
use dps::logging;

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
        let _ = restore_terminal();
    }
}

fn main() -> Result<()> {
    errors::init()?;
    logging::init()?;

    let cli = Cli::parse();
    let tick_interval = Duration::from_secs_f64(1.0 / cli.tick_rate);
    let frame_interval = Duration::from_secs_f64(1.0 / cli.frame_rate);

    let mut last_frame = Instant::now();
    let mut tui = Tui::new()?;
    let mut app = App::new();

    loop {
        if last_frame.elapsed() >= frame_interval {
            tui.terminal.draw(|f| app.render(f))?;
            last_frame = Instant::now();
        }

        let timeout = tick_interval.min(frame_interval.saturating_sub(last_frame.elapsed()));

        if event::poll(timeout)? {
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
    }

    Ok(())
}
