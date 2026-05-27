//! Parse error for Vim-style key sequences.

/// Error returned when a Vim-style key-sequence string cannot be parsed.
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
