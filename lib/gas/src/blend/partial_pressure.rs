use super::{BlendMethod, sealed};

use crate::components::GasComponents;
use crate::constants::{AIR_AR, AIR_CO2, AIR_DILUENT, AIR_N2, AIR_OTHER};

/// Partial-pressure blending: pure O₂ added to air.
///
/// The diluent is always air-derived, so N₂, Ar, CO₂, and trace gases appear
/// in the same ratios as in dry air regardless of the target O₂ fraction.
///
/// ```no_run
/// use dps_gas::{EANxBlend, PartialPressure};
/// use dps_units::Percent;
///
/// let ean32 = EANxBlend::new(Percent::new(0.32).unwrap(), PartialPressure).unwrap();
/// let c = ean32.components();
/// // N₂/Ar ratio equals the air ratio for any FO₂.
/// let air_ratio = 0.78084_f64 / 0.00934;
/// assert!((c.n2() / c.ar() - air_ratio).abs() < 0.01);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
pub struct PartialPressure;

impl sealed::Sealed for PartialPressure {}

impl BlendMethod for PartialPressure {
    fn blend_name(&self) -> &'static str {
        "partial pressure"
    }

    fn components(&self, fo2: f64) -> GasComponents {
        let d = 1.0 - fo2;

        GasComponents::new(
            fo2,
            d * (AIR_N2 / AIR_DILUENT),
            d * (AIR_AR / AIR_DILUENT),
            d * (AIR_CO2 / AIR_DILUENT),
            d * (AIR_OTHER / AIR_DILUENT),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::constants::{AIR_AR, AIR_CO2, AIR_N2, AIR_O2};

    use approx::assert_relative_eq;

    #[test]
    fn air_fractions_sum_to_one() {
        assert_relative_eq!(
            PartialPressure.components(f64::from(AIR_O2)).sum(),
            1.0,
            epsilon = 1e-12
        );
    }

    #[test]
    fn ean32_fractions_sum_to_one() {
        assert_relative_eq!(PartialPressure.components(0.32).sum(), 1.0, epsilon = 1e-12);
    }

    #[test]
    fn air_recovers_standard_air_composition() {
        let c = PartialPressure.components(f64::from(AIR_O2));

        assert_relative_eq!(c.o2(), f64::from(AIR_O2), epsilon = 1e-9);
        assert_relative_eq!(c.n2(), f64::from(AIR_N2), epsilon = 1e-6);
        assert_relative_eq!(c.ar(), f64::from(AIR_AR), epsilon = 1e-6);
        assert_relative_eq!(c.co2(), f64::from(AIR_CO2), epsilon = 1e-9);
    }

    #[test]
    fn n2_ar_ratio_matches_air() {
        let c = PartialPressure.components(0.32);
        assert_relative_eq!(c.n2() / c.ar(), AIR_N2 / AIR_AR, epsilon = 1e-6);
    }

    #[test]
    fn pure_o2_has_zero_diluent() {
        let c = PartialPressure.components(1.0);

        assert_relative_eq!(c.o2(), 1.0, epsilon = 1e-12);
        assert_relative_eq!(c.n2(), 0.0, epsilon = 1e-12);
        assert_relative_eq!(c.ar(), 0.0, epsilon = 1e-12);
        assert_relative_eq!(c.co2(), 0.0, epsilon = 1e-12);
        assert_relative_eq!(c.other(), 0.0, epsilon = 1e-12);
    }

    #[test]
    fn blend_name_is_partial_pressure() {
        assert_eq!(PartialPressure.blend_name(), "partial pressure");
    }
}
