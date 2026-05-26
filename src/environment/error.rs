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

use crate::units::{Bar, Celsius, Meters, MetersPerBar, PartsPerThousand};

/// Error returned by fallible [`DiveEnvironment`](super::DiveEnvironment) constructors.
#[derive(Debug, Clone, Copy, thiserror::Error)]
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
}
