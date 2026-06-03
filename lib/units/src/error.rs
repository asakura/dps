//! Parse errors for unit types.
//!
//! ```
//! use dps_units::{Bar, Meters, Percent};
//! assert!("invalid".parse::<Bar>().is_err());
//! assert!("1.5 bar".parse::<Bar>().is_ok());
//! assert!("invalid".parse::<Percent>().is_err());
//! assert!("32%".parse::<Percent>().is_ok());
//! ```

/// Module-level parse error for unit types.
///
/// Wraps the lower-level `ParseError` variants behind a stable boundary.
///
/// # Examples
///
/// ```
/// use dps_units::{Bar, UnitError};
///
/// assert!(matches!("bad".parse::<Bar>(), Err(UnitError::Parse(_))));
/// ```
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    /// Wraps a specific unit-parse failure.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// The supplied value is outside the valid range `[0.0, 1.0]`.
    #[error("value {0} is outside the valid range [0.0, 1.0]")]
    OutOfRange(f64),
}

/// Error returned when a string cannot be parsed as a unit value.
///
/// Each variant corresponds to one unit type and carries the specific
/// substring that could not be interpreted.
///
/// # Examples
///
/// ```
/// use dps_units::{Bar, Celsius, Meters, Percent, UnitError};
///
/// assert!(matches!("nope".parse::<Bar>(), Err(UnitError::Parse(_))));
/// assert!(matches!("nope".parse::<Celsius>(), Err(UnitError::Parse(_))));
/// assert!(matches!("nope".parse::<Meters>(), Err(UnitError::Parse(_))));
/// assert!(matches!("nope".parse::<Percent>(), Err(UnitError::Parse(_))));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// Input could not be parsed as a [`super::Bar`] value.
    #[error("unable to parse `{0}` as a bar value")]
    Bar(String),
    /// Input could not be parsed as a [`super::Celsius`] value.
    #[error("unable to parse `{0}` as a celsius value")]
    Celsius(String),
    /// Input could not be parsed as a [`super::CnsRatePerMinute`] value.
    #[error("unable to parse `{0}` as a CNS rate per minute value")]
    CnsRatePerMinute(String),
    /// Input could not be parsed as a [`super::GramsPerLitre`] value.
    #[error("unable to parse `{0}` as a grams per litre value")]
    GramsPerLitre(String),
    /// Input could not be parsed as a [`super::Meters`] value.
    #[error("unable to parse `{0}` as a meters value")]
    Meters(String),
    /// Input could not be parsed as a [`super::MetersPerBar`] value.
    #[error("unable to parse `{0}` as a meters per bar value")]
    MetersPerBar(String),
    /// Input could not be parsed as a [`super::OTUPerMinute`] value.
    #[error("unable to parse `{0}` as an OTU per minute value")]
    OTUPerMinute(String),
    /// Input could not be parsed as a [`super::PartsPerThousand`] value.
    #[error("unable to parse `{0}` as a parts per thousand value")]
    PartsPerThousand(String),
    /// Input could not be parsed as a [`super::Percent`] value.
    #[error("unable to parse `{0}` as a percent value")]
    Percent(String),
}
