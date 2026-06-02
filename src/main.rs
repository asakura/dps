//! Entry point for the dps application.

use dps::app::App;
use dps::cli::{Args, Cli};
use dps::errors;

use clap::Parser;
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    errors::init()?;

    let args: Args = Cli::parse().try_into()?;
    let mut app = App::new(&args).map_err(color_eyre::Report::from)?;

    app.run().await?;

    Ok(())
}
