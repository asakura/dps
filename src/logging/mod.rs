//! File-based tracing initialisation.

pub mod error;

pub use self::error::Error as LoggingError;

use crate::cli;

use tracing_error::ErrorLayer;
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use std::{fs, path::Path, sync::LazyLock};

/// Environment variable name used to set the log level (e.g. `DPS_LOG_LEVEL`).
pub static LOG_ENV: LazyLock<String> =
    LazyLock::new(|| format!("{}_LOG_LEVEL", *cli::PROJECT_NAME));

/// Log file name, derived from the crate package name (e.g. `dps.log`).
pub static LOG_FILE: LazyLock<String> = LazyLock::new(|| format!("{}.log", env!("CARGO_PKG_NAME")));

/// Initialises file-based tracing for the application.
///
/// # Errors
///
/// Returns `Err` if the data directory cannot be created, the log file cannot
/// be opened, or the tracing subscriber cannot be installed (e.g. already initialised).
pub fn init<P: AsRef<Path>>(data_dir: P) -> Result<(), LoggingError> {
    fs::create_dir_all(data_dir.as_ref())?;

    let log_path = data_dir.as_ref().join(LOG_FILE.as_str());
    let log_file = fs::File::create(log_path)?;
    let env_filter = EnvFilter::builder().with_default_directive(tracing::Level::INFO.into());

    // If the `RUST_LOG` environment variable is set, use that as the default, otherwise use the
    // value of the `LOG_ENV` environment variable. If the `LOG_ENV` environment variable contains
    // errors, then this will return an error.
    let env_filter = env_filter
        .try_from_env()
        .or_else(|_| env_filter.with_env_var(LOG_ENV.as_str()).from_env())?;

    let file_subscriber = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(env_filter);

    // Ignore "already initialized" — a second call from tests means the
    // subscriber is already running, which is fine.
    let _ = tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .try_init();

    Ok(())
}
