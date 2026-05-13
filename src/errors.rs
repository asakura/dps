//! Domain-specific error types for DPS.

use std::fmt;

/// Error returned when an O₂ percentage is outside the valid range [10, 100].
///
/// ```
/// use dps::errors::InvalidO2Percent;
/// let msg = InvalidO2Percent(5).to_string();
/// assert!(msg.contains("5") && msg.contains("10") && msg.contains("100"));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct InvalidO2Percent(pub u8);

impl fmt::Display for InvalidO2Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "O₂ percentage {} is outside valid range [10, 100]", self.0)
    }
}

impl std::error::Error for InvalidO2Percent {}
