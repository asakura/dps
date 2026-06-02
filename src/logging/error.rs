//! Logging initialisation error type.
//!
//! # Examples
//!
//! ```
//! use dps::logging::LoggingError;
//!
//! let io_err = std::io::Error::from(std::io::ErrorKind::NotFound);
//! let err = LoggingError::from(io_err);
//! assert!(matches!(err, LoggingError::Io(_)));
//! ```

use tracing_subscriber::filter::FromEnvError;

/// Error from a logging initialisation operation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The data directory could not be created, or the log file could not be opened.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The log-level environment variable contains an invalid filter directive.
    #[error(transparent)]
    EnvFilter(#[from] FromEnvError),
}
