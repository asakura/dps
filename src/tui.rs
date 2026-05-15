use std::{io, panic};

use crossterm::{
    cursor::Show,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, Show)?;
    Ok(())
}

/// RAII guard that owns the terminal and restores it on drop.
pub struct Tui {
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl Tui {
    /// Switches the terminal into raw mode and the alternate screen, then creates the backend.
    pub fn new() -> io::Result<Self> {
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
