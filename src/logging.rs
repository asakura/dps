//! File-based tracing initialisation.

use std::path::PathBuf;

use directories::ProjectDirs;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
}

/// Returns the path to the application's local data directory.
///
/// Priority: `DPS_DATA` env var → platform data dir (`directories` crate) → `.data` fallback.
pub fn get_data_dir() -> PathBuf {
    let data_env = format!("{}_DATA", env!("CARGO_PKG_NAME").to_uppercase());
    if let Ok(s) = std::env::var(data_env) {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".data")
    }
}

/// Initialises a file-based `tracing` subscriber and registers it globally.
///
/// Creates the data directory if absent, opens `dps.log` inside it, and sets
/// the default log level to `INFO`. Override with the `DPS_LOGLEVEL` env var.
pub fn initialize_logging() -> color_eyre::Result<()> {
    let directory = get_data_dir();
    std::fs::create_dir_all(&directory)?;
    let log_path = directory.join(concat!(env!("CARGO_PKG_NAME"), ".log"));
    let log_file = std::fs::File::create(log_path)?;

    let log_env = format!("{}_LOGLEVEL", env!("CARGO_PKG_NAME").to_uppercase());
    let env_filter = tracing_subscriber::filter::EnvFilter::builder()
        .with_default_directive(tracing::Level::INFO.into())
        .with_env_var(log_env)
        .from_env_lossy();

    let file_subscriber = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(env_filter);

    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .init();

    Ok(())
}
