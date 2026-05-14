//! File-based tracing initialisation.

use std::path::Path;

use tracing_error::ErrorLayer;
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config;

pub static LOG_ENV: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| format!("{}_LOG_LEVEL", *config::PROJECT_NAME));
pub static LOG_FILE: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| format!("{}.log", env!("CARGO_PKG_NAME")));

/// Initialises file-based tracing, writing to `<data_dir>/<crate>.log`.
///
/// Call this after resolving the effective data directory (CLI flag →
/// `DPS_DATA` env var → platform default) so the log file lands in the
/// right place regardless of which override is active.
pub fn init(data_dir: &Path) -> color_eyre::Result<()> {
    std::fs::create_dir_all(data_dir)?;

    let log_path = data_dir.join(LOG_FILE.clone());
    let log_file = std::fs::File::create(log_path)?;
    let env_filter = EnvFilter::builder().with_default_directive(tracing::Level::INFO.into());

    // If the `RUST_LOG` environment variable is set, use that as the default, otherwise use the
    // value of the `LOG_ENV` environment variable. If the `LOG_ENV` environment variable contains
    // errors, then this will return an error.
    let env_filter = env_filter
        .try_from_env()
        .or_else(|_| env_filter.with_env_var(LOG_ENV.clone()).from_env())?;

    let file_subscriber = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(env_filter);

    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .try_init()?;

    Ok(())
}
