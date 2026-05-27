//! Entry point for the dps application.

use std::path::Path;

use clap::Parser;
use color_eyre::Result;
use dps::app::AppNew;
use dps::cli::Cli;
use dps::config::get_data_dir;
use dps::errors;
use dps::logging;

#[tokio::main]
async fn main() -> Result<()> {
    errors::init()?;

    let args = Cli::parse();

    args.validate().unwrap_or_else(|e| e.exit());

    let effective_data_dir = args
        .data_dir
        .as_deref()
        .map_or_else(get_data_dir, Path::to_path_buf);

    logging::init(&effective_data_dir)?;

    let mut app = AppNew::new(
        args.tick_rate,
        args.frame_rate,
        args.config_dir.as_deref(),
        args.data_dir.as_deref(),
    )
    .map_err(color_eyre::Report::from)?;
    app.run().await?;

    Ok(())
}
