use clap::Parser;
use color_eyre::Result;
use dps::app::App;
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
        .map_or_else(get_data_dir, |p| p.to_path_buf());
    logging::init(&effective_data_dir)?;
    let mut app = App::new(
        args.tick_rate,
        args.frame_rate,
        args.config_dir.as_deref(),
        args.data_dir.as_deref(),
    )?;
    app.run().await?;
    Ok(())
}
