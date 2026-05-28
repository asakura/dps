//! Parse errors for Vim-style key sequences.
//!
//! ```
//! use dps::keymap::{KeyMapError, parse_key_sequence};
//!
//! assert!(matches!(parse_key_sequence(""), Err(KeyMapError::Parse(_))));
//! ```

/// Module-level parse error for key-map operations.
///
/// # Examples
///
/// ```
/// use dps::keymap::{KeyMapError, parse_key_sequence};
///
/// assert!(matches!(parse_key_sequence(""), Err(KeyMapError::Parse(_))));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Wraps a key-sequence parse failure.
    #[error(transparent)]
    Parse(#[from] ParseError),
}

/// Error returned when a Vim-style key-sequence string cannot be parsed.
///
/// # Examples
///
/// ```
/// use dps::keymap::{KeyMapError, parse_key_sequence};
///
/// assert!(matches!(parse_key_sequence(""), Err(KeyMapError::Parse(_))));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// The input key sequence was empty.
    #[error("empty key sequence")]
    EmptySequence,
    /// An opening `<` has no matching `>`.
    #[error("unclosed `<` in `{0}`")]
    UnclosedAngleBracket(String),
    /// A `<…>` spec contained an unrecognised key name.
    #[error("unable to parse `{0}`")]
    UnknownKey(String),
}
