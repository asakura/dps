//! Error type for fallible [`DiveEnvironment`](crate::environment::DiveEnvironment) constructors.
//!
//! [`DiveEnvironmentError`] is returned whenever a constructor or builder receives an
//! out-of-range or non-finite value. Each variant carries the offending value so
//! callers can report it without re-inspecting the input.
//!
//! ```ignore
//! use dps::environment::{DiveEnvironment, DiveEnvironmentError};
//!
//! assert!(matches!(
//!     DiveEnvironment::at_altitude(-1.0),
//!     Err(DiveEnvironmentError::AltitudeOutOfRange(_))
//! ));
//! ```

use std::fmt;

use crate::units::{Bar, Celsius, Meters, MetersPerBar, PartsPerThousand};

/// Error returned by fallible [`DiveEnvironment`](super::DiveEnvironment) constructors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiveEnvironmentError {
    /// Surface pressure must be finite and positive.
    SurfacePressureNotPositive(Bar),
    /// Water density (m/bar) must be finite and positive.
    WaterDensityNotPositive(MetersPerBar),
    /// Altitude must be in $[\pu{0.0 m}, \pu{8849.0 m}]$.
    AltitudeOutOfRange(Meters),
    /// Salinity must be in $[\pu{0.0 ‰}, \pu{350.0 ‰}]$.
    SalinityOutOfRange(PartsPerThousand),
    /// Temperature must be in $[\pu{-2.0 ^\circ C}, \pu{40.0 ^\circ C}]$.
    TemperatureOutOfRange(Celsius),
}

impl fmt::Display for DiveEnvironmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SurfacePressureNotPositive(p) => {
                write!(f, "surface pressure must be finite and positive, got {p}")
            }
            Self::WaterDensityNotPositive(d) => {
                write!(f, "water density must be finite and positive, got {d}")
            }
            Self::AltitudeOutOfRange(h) => {
                write!(f, "altitude {h} is outside [0.0 m, 8 849.0 m]")
            }
            Self::SalinityOutOfRange(s) => {
                write!(f, "salinity {s} is outside [0.0 ‰, 350.0 ‰]")
            }
            Self::TemperatureOutOfRange(t) => {
                write!(f, "temperature {t} is outside [−2.0 °C, 40.0 °C]")
            }
        }
    }
}

impl std::error::Error for DiveEnvironmentError {}
