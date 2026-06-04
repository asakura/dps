//! Error type for fallible [`DiveEnvironment`](crate::DiveEnvironment) constructors.
//!
//! [`DiveEnvironmentError`] is returned whenever a constructor, builder, or [`FromStr`](std::str::FromStr)
//! receives an out-of-range, non-finite, or unparseable value. Each variant carries the offending
//! value so callers can report it without re-inspecting the input.
//!
//! ```
//! use dps_environment::{DiveEnvironment, DiveEnvironmentError};
//! use dps_units::Meters;
//!
//! assert!(matches!(
//!     DiveEnvironment::at_altitude(Meters::new(-1.0)),
//!     Err(DiveEnvironmentError::AltitudeOutOfRange(_))
//! ));
//! ```

use dps_units::{Bar, Celsius, Meters, MetersPerBar, PartsPerThousand};

/// Error returned when a string cannot be parsed as a [`DiveEnvironment`](super::DiveEnvironment).
///
/// Produced by [`DiveEnvironment`](super::DiveEnvironment)'s [`FromStr`](std::str::FromStr) impl when the input does not match any
/// format produced by [`Display`](std::fmt::Display).
///
/// ```
/// use dps_environment::DiveEnvironment;
///
/// assert!("invalid".parse::<DiveEnvironment>().is_err());
/// assert!("standard".parse::<DiveEnvironment>().is_ok());
/// ```
#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum ParseDiveEnvironmentError {
    /// The string is an ocean preset, but the name is not recognised.
    #[error("unknown ocean preset '{0}'")]
    UnknownOcean(String),
    /// The string is a lake preset, but the name is not recognised.
    #[error("unknown lake preset '{0}'")]
    UnknownLake(String),
    /// The custom format string is missing required keys or delimiters.
    #[error("invalid custom format: expected 'surface_pressure=P,water_density=D'")]
    InvalidCustomFormat,
    /// The custom format contains an unparseable surface pressure value.
    #[error("invalid surface pressure: {0}")]
    InvalidSurfacePressure(String),
    /// The custom format contains an unparseable water density value.
    #[error("invalid water density: {0}")]
    InvalidWaterDensity(String),
    /// The string does not match any known environment format.
    #[error("unrecognised environment format: '{0}'")]
    UnrecognisedFormat(String),
}

/// Error returned by fallible [`DiveEnvironment`](super::DiveEnvironment) constructors and [`FromStr`](std::str::FromStr).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum DiveEnvironmentError {
    /// Surface pressure must be finite and positive.
    #[error("surface pressure must be finite and positive, got {0}")]
    SurfacePressureNotPositive(Bar),
    /// Water density (m/bar) must be finite and positive.
    #[error("water density must be finite and positive, got {0}")]
    WaterDensityNotPositive(MetersPerBar),
    /// Altitude must be in $[\pu{0.0 m}, \pu{8849.0 m}]$.
    #[error("altitude {0} is outside [0.0 m, 8 849.0 m]")]
    AltitudeOutOfRange(Meters),
    /// Salinity must be in $[\pu{0.0 ‰}, \pu{350.0 ‰}]$.
    #[error("salinity {0} is outside [0.0 ‰, 350.0 ‰]")]
    SalinityOutOfRange(PartsPerThousand),
    /// Temperature must be in $[\pu{-2.0 ^\circ C}, \pu{40.0 ^\circ C}]$.
    #[error("temperature {0} is outside [−2.0 °C, 40.0 °C]")]
    TemperatureOutOfRange(Celsius),
    /// The input string did not match any known [`DiveEnvironment`](super::DiveEnvironment) format.
    #[error(transparent)]
    Parse(#[from] ParseDiveEnvironmentError),
}
