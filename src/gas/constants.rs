use crate::units::Percent;

pub(super) const EAN_MIN_O2: Percent = Percent::literal(0.10);

// Dry air mole fractions (NOAA standard atmosphere)
//
// O₂, Ar, CO₂, and other traces are taken from NOAA/ICAO published values.
// N₂ is derived as the exact remainder so all five fractions sum to exactly 1.0.
// The resulting N₂ value (≈ 78.077 %) differs from the independently published
// 78.084 % by ~0.007 %; this is within the rounding of the source data.

// The _RAW constants exist solely so that derived constants (AIR_N2, AIR_DILUENT,
// AIR_NARCOTIC, PSA_*) can be computed in const context.  Ideally these would
// chain off the typed Percent constants via From<Percent> for f64, but trait
// method calls in const expressions require const trait impls, which are still
// unstable as of Rust 1.88 (tracking issue rust-lang/rust#67792).  Once
// `#![feature(const_trait_impl)]` is stabilised the _RAW layer can be removed
// and the derivations rewritten as e.g. `f64::from(AIR_O2)`.
pub(super) const AIR_O2_RAW: f64 = 0.209_46;
pub(super) const AIR_AR_RAW: f64 = 0.009_34;
const AIR_CO2_RAW: f64 = 0.000_407;
pub(super) const AIR_OTHER_RAW: f64 = 0.000_027_4;
const AIR_N2_RAW: f64 = 1.0 - AIR_O2_RAW - AIR_AR_RAW - AIR_CO2_RAW - AIR_OTHER_RAW;

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "derived constants use AIR_O2_RAW for const arithmetic until const_trait_impl stabilises; AIR_O2 replaces it once f64::from(Percent) is usable in const context"
    )
)]
pub(super) const AIR_O2: Percent = Percent::literal(AIR_O2_RAW);
pub(super) const AIR_AR: Percent = Percent::literal(AIR_AR_RAW);
pub(super) const AIR_CO2: Percent = Percent::literal(AIR_CO2_RAW); // NOAA GML 2017 annual mean (≈ 406.6 ppm); fixed for model consistency
pub(super) const AIR_OTHER: Percent = Percent::literal(AIR_OTHER_RAW); // Ne, He, CH₄, Kr, H₂, N₂O, Xe, …
pub(super) const AIR_N2: Percent = Percent::literal(AIR_N2_RAW);
pub(super) const AIR_DILUENT: Percent = Percent::literal(1.0 - AIR_O2_RAW); // non-O₂ total

// Narcosis
//
// NOAA model: O₂ is not narcotic; Ar is 1.5× more narcotic than N₂ per unit
// partial pressure. CO₂ narcosis from inspired gas at air-trace concentrations
// is negligible and excluded.

pub(super) const AR_NARCOTIC_POTENCY: f64 = 1.5;
pub(super) const AIR_NARCOTIC: Percent =
    Percent::literal(AIR_N2_RAW + AR_NARCOTIC_POTENCY * AIR_AR_RAW);

// Gas density
//
// ρ [g/L] = P [Pa] × M [g/mol] / (R [Pa·L/(mol·K)] × T [K])

pub(super) const GAS_CONSTANT: f64 = 8314.46; // Pa·L/(mol·K)
pub(super) const STANDARD_TEMP_K: f64 = 293.15; // 20 °C reference temperature

// CNS clock

pub(super) fn cns_limit_minutes(ppo2: f64) -> f64 {
    if ppo2 <= 0.50 {
        return f64::INFINITY; // no CNS effect below 0.5 bar
    }

    // NOAA single-dive CNS exposure limits
    if ppo2 <= 0.60 {
        return 720.0;
    }

    if ppo2 <= 0.70 {
        return 570.0;
    }

    if ppo2 <= 0.80 {
        return 450.0;
    }

    if ppo2 <= 0.90 {
        return 360.0;
    }

    if ppo2 <= 1.00 {
        return 300.0;
    }

    if ppo2 <= 1.10 {
        return 240.0;
    }

    if ppo2 <= 1.20 {
        return 210.0;
    }

    if ppo2 <= 1.30 {
        return 180.0;
    }

    if ppo2 <= 1.40 {
        return 150.0;
    }

    if ppo2 <= 1.50 {
        return 120.0;
    }

    if ppo2 <= 1.60 {
        return 45.0;
    }

    0.0 // above 1.6 bar: no exposure permitted
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use rstest::rstest;

    #[rstest]
    #[case(0.0, f64::INFINITY)]
    #[case(0.50, f64::INFINITY)]
    #[case(0.51, 720.0)]
    #[case(0.60, 720.0)]
    #[case(0.61, 570.0)]
    #[case(0.70, 570.0)]
    #[case(0.71, 450.0)]
    #[case(0.80, 450.0)]
    #[case(0.81, 360.0)]
    #[case(0.90, 360.0)]
    #[case(0.91, 300.0)]
    #[case(1.00, 300.0)]
    #[case(1.01, 240.0)]
    #[case(1.10, 240.0)]
    #[case(1.11, 210.0)]
    #[case(1.20, 210.0)]
    #[case(1.21, 180.0)]
    #[case(1.30, 180.0)]
    #[case(1.31, 150.0)]
    #[case(1.40, 150.0)]
    #[case(1.41, 120.0)]
    #[case(1.50, 120.0)]
    #[case(1.51, 45.0)]
    #[case(1.60, 45.0)]
    #[case(1.61, 0.0)]
    #[case(2.00, 0.0)]
    fn cns_limit_boundaries(#[case] ppo2: f64, #[case] expected: f64) {
        assert_relative_eq!(cns_limit_minutes(ppo2), expected);
    }
}
