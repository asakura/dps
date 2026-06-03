#![cfg_attr(
    test,
    expect(
        clippy::panic_in_result_fn,
        reason = "Tests legitimately combine Result return types with panic-inducing assertions"
    )
)]

//! Physical correctness tests — absolute values against published constants.
//!
//! These tests verify depth↔pressure conversions against independently
//! verifiable physical reference points, not just internal consistency.
//! A sign flip or wrong ICAO exponent would pass every unit test but fail here.

use dps_environment::{DiveEnvironment, DiveEnvironmentError, Lake, MAX_ALTITUDE, Ocean};
use dps_units::{Bar, Meters};

use approx::assert_relative_eq;
use rstest::rstest;

// Surface pressure: ICAO ISA barometric formula
mod surface_pressure {
    use super::*;

    #[rstest]
    fn sea_level_pressure_matches_iso_standard_atmosphere() {
        // ISO standard atmosphere defines P₀ = 101 325 Pa = 1.01325 bar exactly.
        assert_relative_eq!(
            DiveEnvironment::standard().surface_pressure(),
            Bar::new(1.013_25),
            epsilon = 1e-9,
        );
    }

    #[rstest]
    fn titicaca_surface_pressure_matches_icao_isa() {
        // ICAO ISA barometric tables: 3812 m → ~0.632 bar.
        // Published in aviation pressure-altitude charts and independently
        // verifiable from the ICAO formula: 101325×(1−2.25577×10⁻⁵×3812)^5.25588 Pa.
        assert_relative_eq!(
            DiveEnvironment::lake(Lake::Titicaca).surface_pressure(),
            Bar::new(0.632),
            max_relative = 5e-3,
        );
    }

    #[rstest]
    fn max_altitude_surface_pressure_matches_icao_isa() -> Result<(), DiveEnvironmentError> {
        // ICAO ISA at 8849 m (Everest summit) → ~0.314 bar.
        // Uses MAX_ALTITUDE constant directly as a consumer would.
        let env = DiveEnvironment::at_altitude(MAX_ALTITUDE)?;

        assert_relative_eq!(env.surface_pressure(), Bar::new(0.314), max_relative = 5e-3,);

        Ok(())
    }

    #[rstest]
    fn altitude_pressure_decreases_monotonically() -> Result<(), DiveEnvironmentError> {
        let sea_level = DiveEnvironment::standard().surface_pressure();
        let titicaca = DiveEnvironment::lake(Lake::Titicaca).surface_pressure();
        let everest = DiveEnvironment::at_altitude(MAX_ALTITUDE)?.surface_pressure();

        assert!(
            sea_level > titicaca && titicaca > everest,
            "pressure must decrease with altitude: {sea_level} > {titicaca} > {everest}"
        );

        Ok(())
    }
}

// Absolute pressure: ISO seawater density and Archimedes principle
mod absolute_pressure {
    use super::*;

    #[rstest]
    fn standard_seawater_absolute_pressure_at_10m() {
        // ISO 19901-7: seawater at (35 ‰, 15 °C) has density 1025 kg/m³.
        // ISO 80000-3: g = 9.80665 m/s².  1 bar = 100_000 Pa.
        // Gauge at 10 m = 1025 × 9.80665 × 10 / 100_000 ≈ 1.005 bar.
        // Absolute = 1.01325 + 1.005 ≈ 2.018 bar.
        let expected_bar = 1.013_25 + 1025.0 * 9.806_65 * 10.0 / 1e5;

        assert_relative_eq!(
            DiveEnvironment::standard().absolute_pressure(Meters::new(10.0)),
            Bar::new(expected_bar),
            epsilon = 1e-6,
        );
    }

    #[rstest]
    fn freshwater_absolute_pressure_at_10m_is_less_than_seawater() {
        // Fresh water (0 ‰) is less dense than ISO seawater (35 ‰) — same depth
        // produces less gauge pressure, so absolute pressure is lower.
        let fresh = DiveEnvironment::freshwater().absolute_pressure(Meters::new(10.0));
        let salt = DiveEnvironment::standard().absolute_pressure(Meters::new(10.0));

        assert!(fresh < salt);
    }

    #[rstest]
    fn red_sea_absolute_pressure_at_30m() {
        // Red Sea: 40 ‰ salinity, 26 °C → ρ = 1000 + 0.8×40 − 0.2×26 = 1026.8 kg/m³.
        // Gauge at 30 m = 1026.8 × 9.80665 × 30 / 100 000 ≈ 3.021 bar.
        // Absolute ≈ 1.013 + 3.021 = 4.034 bar.
        let rho = 0.2f64.mul_add(-26.0, 0.8f64.mul_add(40.0, 1000.0_f64)); // 1026.8 kg/m³
        let expected_bar = 1.013_25 + rho * 9.806_65 * 30.0 / 1e5;

        assert_relative_eq!(
            DiveEnvironment::ocean(Ocean::RedSea).absolute_pressure(Meters::new(30.0)),
            Bar::new(expected_bar),
            epsilon = 1e-6,
        );
    }

    #[rstest]
    fn red_sea_is_denser_than_baltic_at_same_depth() {
        // Red Sea (40 ‰) is saltier than Baltic (7 ‰) — more pressure at the same depth.
        let depth = Meters::new(20.0);
        let red_sea = DiveEnvironment::ocean(Ocean::RedSea).absolute_pressure(depth);
        let baltic = DiveEnvironment::ocean(Ocean::BalticSea).absolute_pressure(depth);

        assert!(red_sea > baltic);
    }
}

// Depth ↔ pressure roundtrip
mod depth_pressure_roundtrip {
    use super::*;

    #[rstest]
    #[case(DiveEnvironment::standard())]
    #[case(DiveEnvironment::freshwater())]
    #[case(DiveEnvironment::ocean(Ocean::RedSea))]
    #[case(DiveEnvironment::ocean(Ocean::BalticSea))]
    #[case(DiveEnvironment::lake(Lake::Titicaca))]
    #[case(DiveEnvironment::lake(Lake::OjosDeSalado))]
    fn depth_pressure_roundtrip_is_identity(#[case] env: DiveEnvironment) {
        let depth = Meters::new(30.0);

        assert_relative_eq!(
            env.depth(env.absolute_pressure(depth)),
            depth,
            epsilon = 1e-9,
        );
    }

    #[rstest]
    fn depth_at_surface_pressure_is_zero() {
        let env = DiveEnvironment::lake(Lake::Titicaca);

        assert_relative_eq!(
            env.depth(env.surface_pressure()),
            Meters::new(0.0),
            epsilon = 1e-9,
        );
    }
}
