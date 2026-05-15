use std::time::{Duration, Instant};

use clap::Parser;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyEventKind};

use dps::action::Action;
use dps::app::App;
use dps::cli::Cli;
use dps::config::{Config, CONFIG_FOLDER, DATA_FOLDER, get_config_dir, get_data_dir};
use dps::errors;
use dps::logging;
use dps::tui::Tui;

fn main() -> Result<()> {
    errors::init()?;

    // Parse CLI before logging so --data-dir reaches logging::init.
    let cli = Cli::parse();

    let data_dir = cli.data_dir.as_deref().map_or_else(get_data_dir, |p| p.to_path_buf());
    logging::init(&data_dir)?;

    tracing::debug!(
        data_dir = ?DATA_FOLDER.as_deref(),
        config_dir = ?CONFIG_FOLDER.as_deref(),
        "env directory overrides (None = using platform default)"
    );

    let tick_interval = Duration::from_secs_f64(1.0 / cli.tick_rate);
    let frame_interval = Duration::from_secs_f64(1.0 / cli.frame_rate);

    let config = Config::from_dirs(cli.config_dir.as_deref(), cli.data_dir.as_deref())
        .unwrap_or_else(|_| {
            // from_dirs sets paths via set_default; failure leaves them empty.
            let mut c = Config::default();
            c.config.data_dir = data_dir;
            c.config.config_dir = cli.config_dir.unwrap_or_else(get_config_dir);
            c
        });

    tracing::debug!(
        data_dir = %config.config.data_dir.display(),
        config_dir = %config.config.config_dir.display(),
        "effective directories"
    );

    let mut last_frame = Instant::now();
    let mut tui = Tui::new()?;
    let mut app = App::new(config);

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
