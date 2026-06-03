#![cfg_attr(
    test,
    expect(
        clippy::panic_in_result_fn,
        reason = "Tests legitimately combine Result return types with panic-inducing assertions"
    )
)]

//! End-to-end workflow tests — simulate real consumer usage patterns.
//!
//! Each test follows a complete scenario: receive input, construct an
//! environment, compute, and verify or serialise the result. These tests
//! validate the API as a whole rather than individual methods.

use dps_environment::{DiveEnvironment, DiveEnvironmentError, Lake, Ocean};
use dps_units::{Bar, Meters, MetersPerBar};

use approx::assert_relative_eq;
use rstest::rstest;

/// Register / clipboard workflow: store an environment as a string,
/// retrieve it, and use it for a depth calculation.
#[rstest]
fn clipboard_encode_decode_compute() -> Result<(), DiveEnvironmentError> {
    let original = DiveEnvironment::ocean(Ocean::Caribbean);
    let clipboard = original.to_clipboard_string();
    let retrieved: DiveEnvironment = clipboard.parse()?;
    let depth = Meters::new(30.0);

    assert_eq!(clipboard, "ocean:Caribbean");
    assert_eq!(retrieved, original);

    assert_relative_eq!(
        retrieved.absolute_pressure(depth),
        original.absolute_pressure(depth),
        epsilon = 1e-9,
    );

    Ok(())
}

/// Builder chain: Red Sea salinity at 500 m elevation (e.g. an elevated
/// saltwater pool or high-altitude saltwater site).
#[rstest]
fn builder_chain_ocean_preset_with_altitude_adjustment() -> Result<(), DiveEnvironmentError> {
    let sea_level = DiveEnvironment::ocean(Ocean::RedSea);
    let elevated = sea_level.with_altitude(Meters::new(500.0))?;

    // Salinity-derived density is untouched by altitude adjustment.
    assert_eq!(elevated.water_density(), sea_level.water_density());
    // Surface pressure decreases with altitude.
    assert!(elevated.surface_pressure() < sea_level.surface_pressure());

    // The adjusted environment is not a named preset, so it uses the key-value format.
    let s = elevated.to_clipboard_string();

    assert!(
        s.starts_with("surface_pressure="),
        "non-preset should use key-value format, got: {s}"
    );

    // Key-value format roundtrips correctly.
    let parsed: DiveEnvironment = s.parse()?;
    assert_eq!(parsed, elevated);

    Ok(())
}

/// Simulate receiving an environment string from a CLI flag and verifying
/// that freshwater produces less absolute pressure than seawater at the
/// same depth.
#[rstest]
fn parse_from_cli_arg_freshwater_vs_seawater() -> Result<(), DiveEnvironmentError> {
    let env: DiveEnvironment = "freshwater".parse()?;
    let depth = Meters::new(20.0);

    assert!(
        env.absolute_pressure(depth) < DiveEnvironment::standard().absolute_pressure(depth),
        "freshwater (less dense) must produce less absolute pressure than standard seawater"
    );

    Ok(())
}

/// Parse a custom environment from a saved config string, use it, and
/// verify the Display → `FromStr` roundtrip.
#[rstest]
fn custom_environment_parse_and_roundtrip() -> Result<(), DiveEnvironmentError> {
    let config = "surface_pressure=0.90,water_density=10.5";
    let env: DiveEnvironment = config.parse()?;
    let roundtripped: DiveEnvironment = env.to_string().parse()?;

    assert_relative_eq!(env.surface_pressure(), Bar::new(0.90), epsilon = 1e-9);
    assert_relative_eq!(env.water_density(), MetersPerBar::new(10.5), epsilon = 1e-9);
    assert_eq!(roundtripped, env);

    Ok(())
}

/// `From<Ocean>` conversion reaches the same state as the named constructor.
#[rstest]
fn from_ocean_conversion_matches_constructor() {
    assert_eq!(
        DiveEnvironment::from(Ocean::Mediterranean),
        DiveEnvironment::ocean(Ocean::Mediterranean),
    );
}

/// `From<Lake>` conversion reaches the same state as the named constructor.
#[rstest]
fn from_lake_conversion_matches_constructor() {
    assert_eq!(
        DiveEnvironment::from(Lake::Baikal),
        DiveEnvironment::lake(Lake::Baikal),
    );
}
