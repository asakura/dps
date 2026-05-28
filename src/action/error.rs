//! Parse errors for action types.
//!
//! ```
//! use dps::action::{Action, ActionError};
//! use std::str::FromStr;
//!
//! assert!(matches!(Action::from_str("Unknown"), Err(ActionError::Parse(_))));
//! ```

/// Parse error for action string representations.
///
/// # Examples
///
/// ```
/// use dps::action::{Action, ActionError};
/// use std::str::FromStr;
///
/// assert!(matches!(Action::from_str("Unknown"), Err(ActionError::Parse(_))));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// The input string did not match any known variant name or payload format.
    #[error("variant not found")]
    VariantNotFound,
}

impl From<strum::ParseError> for ParseError {
    fn from(_: strum::ParseError) -> Self {
        Self::VariantNotFound
    }
}

/// Module-level error for action parsing.
///
/// # Examples
///
/// ```
/// use dps::action::{Action, ActionError};
/// use std::str::FromStr;
///
/// assert!(matches!(Action::from_str("Unknown"), Err(ActionError::Parse(_))));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Parse failure (unknown variant name, malformed payload).
    #[error(transparent)]
    Parse(#[from] ParseError),
}
