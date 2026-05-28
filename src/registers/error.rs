//! Parse errors for register values.
//!
//! ```
//! use dps::registers::RegisterValue;
//! use dps::registers::error::ParseError;
//!
//! assert!(matches!("nonsense".parse::<RegisterValue>(), Err(ParseError::UnknownValue(_))));
//! ```

/// Error returned when a string cannot be parsed as a [`super::RegisterValue`].
///
/// # Examples
///
/// ```
/// use dps::registers::RegisterValue;
/// use dps::registers::error::ParseError;
///
/// assert!(matches!("nonsense".parse::<RegisterValue>(), Err(ParseError::UnknownValue(_))));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// Input did not match any known register value format.
    #[error("unable to parse `{0}` as a register value")]
    UnknownValue(String),
}
