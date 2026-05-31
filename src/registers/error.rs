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
pub enum RegisterError {
    /// Wraps a register-value parse failure.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// A register character was not recognised.
    #[error(transparent)]
    InvalidRegisterLetter(#[from] InvalidRegisterLetter),
    /// A register index was out of the valid `1`–`9` range.
    #[error(transparent)]
    InvalidRegisterIndex(#[from] InvalidRegisterIndex),
    /// The yank ring has fewer than two entries and cannot be cycled.
    #[error(transparent)]
    YankRingTooSmall(#[from] YankRingTooSmall),
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

/// Error returned when a `u8` is not a valid delete-history index (`1`–`9`).
///
/// # Examples
///
/// ```
/// use dps::registers::RegIndex;
///
/// let err = RegIndex::try_from(0u8).unwrap_err();
/// assert_eq!(err.to_string(), "`0` is not a valid register index (expected 1–9)");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a valid register index (expected 1–9)")]
pub struct InvalidRegisterIndex(pub(super) u8);

/// Error returned when a `char` is not a valid register name.
///
/// # Examples
///
/// ```
/// use dps::registers::{RegisterName, RegisterError};
///
/// let result = RegisterName::try_from('?');
/// assert!(matches!(result, Err(RegisterError::InvalidRegisterLetter(_))));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("`{0:?}` is not a valid register letter")]
pub struct InvalidRegisterLetter(pub(super) char);

/// Error returned when [`RegisterStore::cycle_yank`](super::RegisterStore::cycle_yank) is
/// called but the yank ring has fewer than two entries.
///
/// # Examples
///
/// ```
/// use dps::registers::{RegisterName, RegisterStore, RegisterValue, YankRingTooSmall};
/// use dps::gas::EANx;
/// use dps::units::Percent;
///
/// let mut store = RegisterStore::default();
/// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
///
/// store.push_yank(RegisterName::Unnamed, RegisterValue::EANx(ean32));
/// assert_eq!(store.cycle_yank(), Err(YankRingTooSmall.into()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("yank ring has fewer than two entries; nothing to cycle")]
pub struct YankRingTooSmall;
