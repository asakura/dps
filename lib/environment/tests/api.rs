#![cfg_attr(
    test,
    expect(
        clippy::panic_in_result_fn,
        reason = "Tests legitimately combine Result return types with panic-inducing assertions"
    )
)]

//! Public API accessibility tests — constants as validation bounds and error
//! type discoverability.
//!
//! These tests verify that exported constants correctly gate the same ranges
//! that the constructors enforce, and that every error variant is pattern-
//! matchable and carries a non-empty Display message.

use dps_environment::{
    DiveEnvironment, DiveEnvironmentError, MAX_ALTITUDE, MAX_SALINITY_PPT, MAX_TEMP_C,
    MIN_ALTITUDE, MIN_SALINITY_PPT, MIN_TEMP_C, ParseDiveEnvironmentError,
};
use dps_units::{Bar, Celsius, Meters, MetersPerBar, PartsPerThousand};

use rstest::rstest;

use core::assert_matches;

// Constants as consumer-side validation bounds
mod constants {
    use super::*;

    mod altitude {
        use super::*;

        #[rstest]
        fn min_altitude_is_accepted() {
            assert!(DiveEnvironment::at_altitude(MIN_ALTITUDE).is_ok());
        }

        #[rstest]
        fn max_altitude_is_accepted() {
            assert!(DiveEnvironment::at_altitude(MAX_ALTITUDE).is_ok());
        }

        #[rstest]
        fn below_min_altitude_is_rejected() {
            let below = Meters::new(f64::from(MIN_ALTITUDE) - 1.0);

            assert_matches!(
                DiveEnvironment::at_altitude(below),
                Err(DiveEnvironmentError::AltitudeOutOfRange(_))
            );
        }

        #[rstest]
        fn above_max_altitude_is_rejected() {
            let above = Meters::new(f64::from(MAX_ALTITUDE) + 1.0);

            assert_matches!(
                DiveEnvironment::at_altitude(above),
                Err(DiveEnvironmentError::AltitudeOutOfRange(_))
            );
        }
    }

    mod salinity {
        use super::*;

        #[rstest]
        fn min_salinity_is_accepted() {
            assert!(DiveEnvironment::with_salinity(MIN_SALINITY_PPT).is_ok());
        }

        #[rstest]
        fn max_salinity_is_accepted() {
            assert!(DiveEnvironment::with_salinity(MAX_SALINITY_PPT).is_ok());
        }

        #[rstest]
        fn above_max_salinity_is_rejected() {
            let above = PartsPerThousand::new(f64::from(MAX_SALINITY_PPT) + 1.0);

            assert_matches!(
                DiveEnvironment::with_salinity(above),
                Err(DiveEnvironmentError::SalinityOutOfRange(_))
            );
        }
    }

    mod temperature {
        use super::*;

        #[rstest]
        fn min_temperature_is_accepted() {
            assert!(
                DiveEnvironment::with_salinity_at_temperature(
                    PartsPerThousand::new(35.0),
                    MIN_TEMP_C
                )
                .is_ok()
            );
        }

        #[rstest]
        fn max_temperature_is_accepted() {
            assert!(
                DiveEnvironment::with_salinity_at_temperature(
                    PartsPerThousand::new(35.0),
                    MAX_TEMP_C
                )
                .is_ok()
            );
        }

        #[rstest]
        fn below_min_temperature_is_rejected() {
            let below = Celsius::new(f64::from(MIN_TEMP_C) - 1.0);

            assert_matches!(
                DiveEnvironment::with_salinity_at_temperature(PartsPerThousand::new(35.0), below),
                Err(DiveEnvironmentError::TemperatureOutOfRange(_))
            );
        }

        #[rstest]
        fn above_max_temperature_is_rejected() {
            let above = Celsius::new(f64::from(MAX_TEMP_C) + 1.0);

            assert_matches!(
                DiveEnvironment::with_salinity_at_temperature(PartsPerThousand::new(35.0), above),
                Err(DiveEnvironmentError::TemperatureOutOfRange(_))
            );
        }
    }
}

// Error type accessibility
mod errors {
    use super::*;

    mod dive_environment_error {
        use super::*;

        #[rstest]
        fn surface_pressure_not_positive_is_matchable_with_non_empty_message()
        -> Result<(), &'static str> {
            let err = DiveEnvironment::new(Bar::new(0.0), MetersPerBar::new(10.0))
                .err()
                .ok_or("expected Err")?;

            assert!(!err.to_string().is_empty());
            assert_matches!(err, DiveEnvironmentError::SurfacePressureNotPositive(_));

            Ok(())
        }

        #[rstest]
        fn water_density_not_positive_is_matchable_with_non_empty_message()
        -> Result<(), &'static str> {
            let err = DiveEnvironment::new(Bar::new(1.0), MetersPerBar::new(0.0))
                .err()
                .ok_or("expected Err")?;

            assert!(!err.to_string().is_empty());
            assert_matches!(err, DiveEnvironmentError::WaterDensityNotPositive(_));

            Ok(())
        }

        #[rstest]
        fn altitude_out_of_range_is_matchable_with_non_empty_message() -> Result<(), &'static str> {
            let err = DiveEnvironment::at_altitude(Meters::new(-1.0))
                .err()
                .ok_or("expected Err")?;

            assert!(!err.to_string().is_empty());
            assert_matches!(err, DiveEnvironmentError::AltitudeOutOfRange(_));

            Ok(())
        }

        #[rstest]
        fn salinity_out_of_range_is_matchable_with_non_empty_message() -> Result<(), &'static str> {
            let err = DiveEnvironment::with_salinity(PartsPerThousand::new(-1.0))
                .err()
                .ok_or("expected Err")?;

            assert!(!err.to_string().is_empty());
            assert_matches!(err, DiveEnvironmentError::SalinityOutOfRange(_));

            Ok(())
        }

        #[rstest]
        fn temperature_out_of_range_is_matchable_with_non_empty_message() -> Result<(), &'static str>
        {
            let err = DiveEnvironment::with_salinity_at_temperature(
                PartsPerThousand::new(35.0),
                Celsius::new(-10.0),
            )
            .err()
            .ok_or("expected Err")?;

            assert!(!err.to_string().is_empty());
            assert_matches!(err, DiveEnvironmentError::TemperatureOutOfRange(_));

            Ok(())
        }

        #[rstest]
        fn error_is_clone_and_eq() -> Result<(), &'static str> {
            let err = DiveEnvironment::at_altitude(Meters::new(-1.0))
                .err()
                .ok_or("expected Err")?;

            assert_eq!(err.clone(), err);

            Ok(())
        }
    }

    mod parse_dive_environment_error {
        use super::*;

        #[rstest]
        fn parse_unrecognised_format_is_matchable() -> Result<(), &'static str> {
            let err = "garbage"
                .parse::<DiveEnvironment>()
                .err()
                .ok_or("expected Err")?;

            assert!(!err.to_string().is_empty());
            assert_matches!(
                err,
                DiveEnvironmentError::Parse(ParseDiveEnvironmentError::UnrecognisedFormat(_))
            );

            Ok(())
        }

        #[rstest]
        fn parse_unknown_ocean_is_matchable() {
            assert_matches!(
                "ocean:Atlantis".parse::<DiveEnvironment>(),
                Err(DiveEnvironmentError::Parse(
                    ParseDiveEnvironmentError::UnknownOcean(_)
                ))
            );
        }

        #[rstest]
        fn parse_unknown_lake_is_matchable() {
            assert_matches!(
                "lake:LochNess".parse::<DiveEnvironment>(),
                Err(DiveEnvironmentError::Parse(
                    ParseDiveEnvironmentError::UnknownLake(_)
                ))
            );
        }

        #[rstest]
        fn parse_invalid_custom_format_is_matchable() {
            assert_matches!(
                "surface_pressure=0.9,invalid=10.0".parse::<DiveEnvironment>(),
                Err(DiveEnvironmentError::Parse(
                    ParseDiveEnvironmentError::InvalidCustomFormat
                ))
            );
        }

        #[rstest]
        fn parse_invalid_surface_pressure_value_is_matchable() {
            assert_matches!(
                "surface_pressure=bad,water_density=10.0".parse::<DiveEnvironment>(),
                Err(DiveEnvironmentError::Parse(
                    ParseDiveEnvironmentError::InvalidSurfacePressure(_)
                ))
            );
        }

        #[rstest]
        fn parse_invalid_water_density_value_is_matchable() {
            assert_matches!(
                "surface_pressure=0.9,water_density=bad".parse::<DiveEnvironment>(),
                Err(DiveEnvironmentError::Parse(
                    ParseDiveEnvironmentError::InvalidWaterDensity(_)
                ))
            );
        }
    }
}
