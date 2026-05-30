//! Configuration error type.

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
    /// Configuration file load or parse failure.
    #[error(transparent)]
    Load(#[from] config::ConfigError),
    /// The embedded `config.json5` default could not be parsed.
    #[error("embedded config.json5 is invalid: {0}")]
    EmbeddedConfig(#[from] json5::Error),
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
