//! Configuration error types produced by [`Config::from_dirs`].
//!
//! [`Config::from_dirs`]: crate::config::Config::from_dirs
//!
//! ```
//! use dps::config::{Config, ConfigError};
//!
//! fn load(dir: &std::path::Path) -> Result<Config, ConfigError> {
//!     Config::from_dirs(dir, dir)
//! }
//! ```

/// Key-sequence parse error.
pub use crate::keymap::KeyMapError as KeyResolutionError;

use std::num::ParseIntError;

/// Error from a theme-resolution operation.
#[derive(Debug, thiserror::Error)]
pub enum ThemeResolutionError {
    /// A theme key has no palette with the same name.
    #[error("theme '{0}' has no matching palette")]
    MissingPalette(String),
    /// A slot references a colour name not present in the palette.
    #[error("unknown palette colour '{0}'")]
    UnknownColour(String),
    /// A hex colour string is missing the leading `#`.
    #[error("expected '#' prefix in colour '{0}'")]
    MissingHexPrefix(String),
    /// A hex colour string does not contain exactly 6 hex digits.
    #[error("expected 6 hex digits in '{value}', got {len}")]
    InvalidHexLength {
        /// The full colour value that was being parsed.
        value: String,
        /// Actual number of hex digits found.
        len: usize,
    },
    /// A hex colour string contains a non-hex character.
    #[error("bad hex in '{value}': {source}")]
    InvalidHexDigit {
        /// The full colour value that was being parsed.
        value: String,
        /// The underlying parse error.
        #[source]
        source: ParseIntError,
    },
}

/// Error from a configuration operation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Configuration file could not be read.
    #[error("failed to read config file: {0}")]
    Read(#[from] std::io::Error),
    /// Configuration file could not be parsed as JSON or JSON5.
    #[error("failed to parse config file as JSON5: {0}")]
    ParseJson(json5::Error),
    /// Configuration file could not be parsed as YAML.
    #[error("failed to parse config file as YAML: {0}")]
    ParseYaml(serde_saphyr::Error),
    /// Configuration file could not be parsed as TOML.
    #[error("failed to parse config file as TOML: {0}")]
    ParseToml(toml::de::Error),
    /// The embedded `config.json5` default could not be parsed.
    #[error("embedded config.json5 is invalid and could not be parsed: {0}")]
    EmbeddedConfig(json5::Error),
    /// Theme resolution failed (unknown colour, missing palette, …).
    #[error(transparent)]
    ThemeResolution(#[from] ThemeResolutionError),
    /// Key-sequence parse failure.
    #[error(transparent)]
    KeyResolution(#[from] KeyResolutionError),
    /// The `defaultTheme` name does not match any resolved theme.
    #[error("defaultTheme '{0}' does not match any resolved theme")]
    UnknownTheme(String),
}
