//! Error type for fallible [`DiveEnvironment`](crate::environment::DiveEnvironment) constructors.
//!
//! [`DiveEnvironmentError`] is returned whenever a constructor, builder, or [`FromStr`](std::str::FromStr)
//! receives an out-of-range, non-finite, or unparseable value. Each variant carries the offending
//! value so callers can report it without re-inspecting the input.
//!
//! ```
//! use dps::environment::{DiveEnvironment, DiveEnvironmentError};
//! use dps::units::Meters;
//!
//! assert!(matches!(
//!     DiveEnvironment::at_altitude(Meters::new(-1.0)),
//!     Err(DiveEnvironmentError::AltitudeOutOfRange(_))
//! ));
//! ```

use super::from_str::ParseDiveEnvironmentError;

use crate::units::{Bar, Celsius, Meters, MetersPerBar, PartsPerThousand};

/// Error returned by fallible [`DiveEnvironment`](super::DiveEnvironment) constructors and [`FromStr`](std::str::FromStr).
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
    /// The input string did not match any known [`DiveEnvironment`](super::DiveEnvironment) format.
    #[error(transparent)]
    Parse(#[from] ParseDiveEnvironmentError),
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::units::{Bar, Celsius, Meters, MetersPerBar, PartsPerThousand};

    use rstest::rstest;

    mod display {
        use super::*;

        #[rstest]
        fn surface_pressure_not_positive_contains_value() {
            let msg = DiveEnvironmentError::SurfacePressureNotPositive(Bar::new(-1.0)).to_string();
            assert!(msg.contains("surface pressure"));
            assert!(msg.contains("-1.0 bar"));
        }

        #[rstest]
        fn water_density_not_positive_contains_value() {
            let msg =
                DiveEnvironmentError::WaterDensityNotPositive(MetersPerBar::new(0.0)).to_string();
            assert!(msg.contains("water density"));
            assert!(msg.contains("0.0 m/bar"));
        }

        #[rstest]
        fn altitude_out_of_range_contains_value() {
            let msg = DiveEnvironmentError::AltitudeOutOfRange(Meters::new(-1.0)).to_string();
            assert!(msg.contains("altitude"));
            assert!(msg.contains("-1.0 m"));
        }

        #[rstest]
        fn salinity_out_of_range_contains_value() {
            let msg =
                DiveEnvironmentError::SalinityOutOfRange(PartsPerThousand::new(-1.0)).to_string();
            assert!(msg.contains("salinity"));
            assert!(msg.contains("-1.0 ‰"));
        }

        #[rstest]
        fn temperature_out_of_range_contains_value() {
            let msg = DiveEnvironmentError::TemperatureOutOfRange(Celsius::new(-5.0)).to_string();
            assert!(msg.contains("temperature"));
            assert!(msg.contains("-5.0 °C"));
        }

        #[rstest]
        fn parse_delegates_to_inner_message() {
            let msg = DiveEnvironmentError::Parse(ParseDiveEnvironmentError).to_string();
            assert_eq!(msg, ParseDiveEnvironmentError.to_string());
        }
    }
}
