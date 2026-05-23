use super::{BlendMethod, sealed};
use crate::gas::components::GasComponents;
use crate::gas::constants::{PSA_AR_PER_O2, PSA_OTHER_PER_O2};

/// Pressure-swing adsorption (PSA) blending.
///
/// Zeolite molecular sieves adsorb N₂ and CO₂ strongly; O₂, Ar, and noble
/// traces pass through essentially unretained. As a result:
///
/// - Ar and other noble traces scale with FO₂ (not with the diluent fraction).
/// - CO₂ is essentially absent from the output.
/// - N₂ is the remainder once O₂, Ar, and traces are accounted for.
///
/// The practical ceiling is FO₂ ≈ 95.7 % (where N₂ → 0); [`EANxBlend::new`]
/// rejects values above this ceiling with [`InvalidEANx::BlendCeilingExceeded`].
///
/// ```no_run
/// use dps::gas::{EANxBlend, Psa};
/// use dps::units::Percent;
///
/// let ean32 = EANxBlend::new(Percent::new(0.32).unwrap(), Psa).unwrap();
/// // PSA has no CO₂ in output
/// assert_eq!(ean32.fco2(), 0.0);
/// // Ar is higher than in PP-blended gas at the same FO₂
/// let pp = dps::gas::EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
/// assert!(ean32.far() > pp.far());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Psa;

impl sealed::Sealed for Psa {}

impl BlendMethod for Psa {
    fn components(&self, fo2: f64) -> GasComponents {
        let ar = fo2 * PSA_AR_PER_O2;
        let other = fo2 * PSA_OTHER_PER_O2;

        GasComponents::new(fo2, (1.0 - fo2 - ar - other).max(0.0), ar, 0.0, other)
    }

    fn is_valid_fo2(&self, fo2: f64) -> bool {
        let ar = fo2 * PSA_AR_PER_O2;
        let other = fo2 * PSA_OTHER_PER_O2;

        1.0 - fo2 - ar - other >= -1e-9
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gas::constants::{AIR_AR, AIR_O2};
    use approx::assert_relative_eq;

    #[test]
    fn fractions_sum_to_one() {
        assert_relative_eq!(Psa.components(0.32).sum(), 1.0, epsilon = 1e-9);
    }

    #[test]
    fn has_zero_co2() {
        assert_relative_eq!(Psa.components(0.32).co2(), 0.0);
    }

    #[test]
    fn ar_scales_with_fo2() {
        let expected_ratio = AIR_AR / AIR_O2;

        for &fo2 in &[0.21_f64, 0.32, 0.40] {
            assert_relative_eq!(
                Psa.components(fo2).ar() / fo2,
                expected_ratio,
                epsilon = 1e-9
            );
        }
    }

    #[test]
    fn rejects_fo2_above_ceiling() {
        assert!(!Psa.is_valid_fo2(0.99));
    }

    #[test]
    fn accepts_fo2_below_ceiling() {
        assert!(Psa.is_valid_fo2(0.40));
    }

    #[test]
    fn n2_approaches_zero_near_ceiling() {
        // At FO₂ ≈ 0.95 (near the ~95.7 % ceiling) N₂ must be close to zero —
        // the zeolite has adsorbed almost all of the available N₂.
        let c = Psa.components(0.95);

        assert!(
            c.n2() < 0.01,
            "expected n2 < 0.01 near ceiling, got {}",
            c.n2()
        );

        assert_relative_eq!(c.sum(), 1.0, epsilon = 1e-9);
    }
}
