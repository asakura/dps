//! Parse errors for register values.
//!
//! ```
//! use dps::registers::{RegisterValue, RegisterError};
//!
//! assert!(matches!("nonsense".parse::<RegisterValue>(), Err(RegisterError::Parse(_))));
//! ```

/// Module-level parse error for register values.
///
/// # Examples
///
/// ```
/// use dps::registers::{RegisterValue, RegisterError};
///
/// assert!(matches!("nonsense".parse::<RegisterValue>(), Err(RegisterError::Parse(_))));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Wraps a register-value parse failure.
    #[error(transparent)]
    Parse(#[from] ParseError),
}

/// Error returned when a string cannot be parsed as a [`super::RegisterValue`].
///
/// # Examples
///
/// ```
/// use dps::registers::{RegisterValue, RegisterError};
///
/// assert!(matches!("nonsense".parse::<RegisterValue>(), Err(RegisterError::Parse(_))));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// Input did not match any known register value format.
    #[error("unable to parse `{0}` as a register value")]
    UnknownValue(String),
}
