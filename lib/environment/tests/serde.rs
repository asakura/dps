#![cfg_attr(
    test,
    expect(
        clippy::panic_in_result_fn,
        reason = "Tests legitimately combine Result return types with panic-inducing assertions"
    )
)]
#![cfg(feature = "serde")]

//! Serde roundtrip tests — only compiled when `--features serde` is active.
//!
//! Verifies that the optional `serde` feature wires up `dps-units/serde`
//! correctly and that all three public types produce sensible JSON that
//! roundtrips without data loss.
//!
//! Note on float precision: values computed via `mul_add` chains (e.g. ocean
//! preset water densities) can differ by ≤ 2 ULP after a JSON roundtrip due
//! to `serde_json`'s float parser.  Preset tests use `assert_relative_eq!` with
//! `epsilon = 1e-12`; tests on exact constants (standard, freshwater, custom)
//! use exact equality.

use dps_environment::{DiveEnvironment, Lake, Ocean};
use dps_units::{Bar, MetersPerBar};

use approx::assert_relative_eq;
use rstest::rstest;

mod dive_environment {
    use super::*;

    #[rstest]
    fn standard_preset_json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let env = DiveEnvironment::standard();
        let json = serde_json::to_string(&env)?;
        let parsed: DiveEnvironment = serde_json::from_str(&json)?;

        assert_eq!(env, parsed);

        Ok(())
    }

    #[rstest]
    fn freshwater_preset_json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let env = DiveEnvironment::freshwater();
        let json = serde_json::to_string(&env)?;
        let parsed: DiveEnvironment = serde_json::from_str(&json)?;

        assert_eq!(env, parsed);

        Ok(())
    }

    #[rstest]
    fn ocean_preset_json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        // Values computed via mul_add may round-trip with ≤ 2 ULP error in
        // serde_json's float parser; use approximate equality.
        let env = DiveEnvironment::ocean(Ocean::BalticSea);
        let json = serde_json::to_string(&env)?;
        let parsed: DiveEnvironment = serde_json::from_str(&json)?;

        assert_relative_eq!(
            parsed.surface_pressure(),
            env.surface_pressure(),
            epsilon = 1e-12
        );

        assert_relative_eq!(parsed.water_density(), env.water_density(), epsilon = 1e-12);

        Ok(())
    }

    #[rstest]
    fn lake_preset_json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let env = DiveEnvironment::lake(Lake::Titicaca);
        let json = serde_json::to_string(&env)?;
        let parsed: DiveEnvironment = serde_json::from_str(&json)?;

        assert_relative_eq!(
            parsed.surface_pressure(),
            env.surface_pressure(),
            epsilon = 1e-12
        );

        assert_relative_eq!(parsed.water_density(), env.water_density(), epsilon = 1e-12);

        Ok(())
    }

    #[rstest]
    fn custom_environment_json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        // Exact f64 values (1.0, 10.5) roundtrip exactly.
        let env = DiveEnvironment::new(Bar::new(1.0), MetersPerBar::new(10.5))?;
        let json = serde_json::to_string(&env)?;
        let parsed: DiveEnvironment = serde_json::from_str(&json)?;

        assert_eq!(env, parsed);

        Ok(())
    }

    /// The JSON object must expose `surface_pressure` and `water_density` as
    /// numeric fields. This pins the schema so consumers can rely on field names.
    #[rstest]
    fn json_schema_exposes_expected_fields() -> Result<(), Box<dyn std::error::Error>> {
        let env = DiveEnvironment::standard();
        let json = serde_json::to_string(&env)?;
        let value: serde_json::Value = serde_json::from_str(&json)?;

        assert!(
            value.get("surface_pressure").is_some(),
            "missing 'surface_pressure' field in: {json}"
        );
        assert!(
            value.get("water_density").is_some(),
            "missing 'water_density' field in: {json}"
        );

        Ok(())
    }
}

mod ocean {
    use super::*;

    #[rstest]
    #[case(Ocean::Pacific)]
    #[case(Ocean::BalticSea)]
    #[case(Ocean::Mediterranean)]
    #[case(Ocean::Caribbean)]
    fn json_roundtrip(#[case] ocean: Ocean) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(&ocean)?;
        let parsed: Ocean = serde_json::from_str(&json)?;

        assert_eq!(ocean, parsed);

        Ok(())
    }
}

mod lake {
    use super::*;

    #[rstest]
    #[case(Lake::Titicaca)]
    #[case(Lake::Cenotes)]
    #[case(Lake::OjosDeSalado)]
    fn json_roundtrip(#[case] lake: Lake) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(&lake)?;
        let parsed: Lake = serde_json::from_str(&json)?;

        assert_eq!(lake, parsed);

        Ok(())
    }
}
