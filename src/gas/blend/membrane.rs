use super::{BlendMethod, sealed};
use crate::gas::components::GasComponents;
use crate::gas::constants::{AIR_DILUENT, AIR_OTHER};

/// Membrane separator blending.
///
/// Hollow-fibre membranes separate gases by differential permeability.
/// The N₂/Ar/CO₂ ratios in the output depend on the specific membrane
/// material and operating conditions and cannot be derived from FO₂ alone.
///
/// Construct via [`Membrane::from_analysis`] using a measured gas analysis, or
/// use [`Membrane::typical`] for an approximate model when no analyser is
/// available.
///
/// ```no_run
/// use dps::gas::{EANxBlend, Membrane};
/// use dps::units::Percent;
///
/// // From a gas-analyser reading at FO₂ 0.32
/// let mem = Membrane::from_analysis(0.32, 0.645, 0.030, 0.003).unwrap();
/// let mix = EANxBlend::new(Percent::new(0.32).unwrap(), mem).unwrap();
///
/// // The same membrane characterisation applies at any target FO₂
/// let mix40 = EANxBlend::new(Percent::new(0.40).unwrap(), mem).unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Membrane {
    // These are *diluent-normalised* ratios, not absolute mole fractions.
    //
    // A hollow-fibre membrane separates gases at a fixed N₂/Ar/CO₂ ratio
    // that is characteristic of the equipment, not of the target FO₂.
    // We store what fraction of the diluent (1 − FO₂) each gas occupies at
    // the analysis point, then scale by the actual diluent at any target FO₂.
    //
    // Example: if a gas-analyser reading at FO₂ 0.32 gives N₂ 0.65, then
    //   fn2 (stored) = 0.65 / (1 − 0.32) = 0.956 …
    //   n2  (output) = (1 − fo2_target) × 0.956
    //
    // Invariant: fn2 + far + fco2 + fother = 1.0 (they partition the diluent).
    fn2: f64,
    far: f64,
    fco2: f64,
    fother: f64,
}

impl Membrane {
    /// Constructs a `Membrane` from a gas-analyser reading.
    ///
    /// Pass the absolute mole fractions measured in the membrane output at
    /// any operating point. The diluent fractions are normalised internally
    /// and apply at all target FO₂ values for the same equipment settings.
    ///
    /// The `other` fraction (noble traces, etc.) is derived as the remainder.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidMembraneFractions`] if `fn2 + far + fco2 > 1 − fo2`.
    ///
    /// ```no_run
    /// use dps::gas::Membrane;
    /// let mem = Membrane::from_analysis(0.32, 0.645, 0.030, 0.003).unwrap();
    /// ```
    #[expect(
        clippy::similar_names,
        reason = "fo2/fn2/fco2 are standard gas-fraction notation; the 'f' prefix and gas suffix make each distinct in context"
    )]
    pub fn from_analysis(
        fo2: f64,
        fn2: f64,
        far: f64,
        fco2: f64,
    ) -> Result<Self, InvalidMembraneFractions> {
        let diluent = 1.0 - fo2;

        if diluent < 1e-9 {
            return Err(InvalidMembraneFractions);
        }

        let fother = diluent - fn2 - far - fco2;

        if fother < -1e-6 {
            return Err(InvalidMembraneFractions);
        }

        Ok(Self {
            fn2: fn2 / diluent,
            far: far / diluent,
            fco2: fco2 / diluent,
            fother: fother.max(0.0) / diluent,
        })
    }

    /// Returns an approximate `Membrane` when no gas analysis is available.
    ///
    /// Uses conservative estimates based on typical hollow-fibre membrane
    /// behaviour at common nitrox operating points:
    ///
    /// - Ar is enriched to ≈ 2.0 % of the diluent (≈ 1.7× air's ratio).
    /// - CO₂ is enriched to ≈ 0.1 % of the diluent (CO₂ permeates readily).
    /// - N₂ fills the remainder.
    ///
    /// These values are indicative only. Use [`Membrane::from_analysis`] for
    /// precision work.
    ///
    /// ```no_run
    /// use dps::gas::{EANxBlend, Membrane, EANx};
    /// use dps::units::Percent;
    ///
    /// let mem = Membrane::typical();
    /// let mix = EANxBlend::new(Percent::new(0.32).unwrap(), mem).unwrap();
    ///
    /// // Typical membrane output has more Ar than PP-blended gas
    /// let pp = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert!(mix.far() > pp.far());
    /// ```
    #[must_use]
    pub fn typical() -> Self {
        let far = 0.020;
        let fco2 = 0.001;
        let fother = AIR_OTHER / AIR_DILUENT;
        let fn2 = 1.0 - far - fco2 - fother;

        Self {
            fn2,
            far,
            fco2,
            fother,
        }
    }
}

impl sealed::Sealed for Membrane {}

impl BlendMethod for Membrane {
    fn components(&self, fo2: f64) -> GasComponents {
        let d = 1.0 - fo2;

        GasComponents::new(
            fo2,
            d * self.fn2,
            d * self.far,
            d * self.fco2,
            d * self.fother,
        )
    }
}

/// Error returned when membrane diluent fractions are inconsistent.
///
/// ```no_run
/// use dps::gas::{Membrane, InvalidMembraneFractions};
/// // fn2 + far + fco2 > 1 − fo2
/// assert!(Membrane::from_analysis(0.32, 0.60, 0.10, 0.005).is_err());
/// ```
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("membrane diluent fractions are invalid: fn2 + far + fco2 must not exceed (1 − fo2)")]
pub struct InvalidMembraneFractions;

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use color_eyre::Result;

    #[test]
    #[expect(
        clippy::similar_names,
        reason = "fo2/fn2/fco2 are standard gas-fraction notation; the 'f' prefix and gas suffix make each distinct in context"
    )]
    fn from_analysis_roundtrips_components() -> Result<()> {
        let fo2 = 0.32_f64;
        let fn2 = 0.65_f64;
        let far = 0.025_f64;
        let fco2 = 0.003_f64;
        let mem = Membrane::from_analysis(fo2, fn2, far, fco2)?;
        let c = mem.components(fo2);

        assert_relative_eq!(c.o2(), fo2, epsilon = 1e-12);
        assert_relative_eq!(c.n2(), fn2, epsilon = 1e-12);
        assert_relative_eq!(c.ar(), far, epsilon = 1e-12);
        assert_relative_eq!(c.co2(), fco2, epsilon = 1e-12);
        assert_relative_eq!(c.sum(), 1.0, epsilon = 1e-12);

        Ok(())
    }

    #[test]
    fn from_analysis_rejects_over_full() {
        assert!(Membrane::from_analysis(0.32, 0.60, 0.10, 0.005).is_err());
    }

    #[test]
    fn from_analysis_rejects_pure_o2() {
        // fo2 = 1.0 → diluent = 0 → no N₂/Ar ratios can be defined
        assert!(Membrane::from_analysis(1.0, 0.0, 0.0, 0.0).is_err());
    }

    #[test]
    fn typical_components_sum_to_one() {
        assert_relative_eq!(
            Membrane::typical().components(0.32).sum(),
            1.0,
            epsilon = 1e-12
        );
    }

    #[test]
    fn diluent_ratios_are_fo2_independent() -> Result<()> {
        // A membrane is characterised at one FO₂ but the diluent N₂/Ar ratio must
        // hold at any target FO₂ for the same equipment settings.
        let fo2_analysis = 0.32_f64;
        let fn2 = 0.65_f64;
        let far = 0.025_f64;
        let fco2 = 0.003_f64;
        let mem = Membrane::from_analysis(fo2_analysis, fn2, far, fco2)?;

        let c32 = mem.components(fo2_analysis);
        let c40 = mem.components(0.40);

        // N₂/Ar ratio must be identical at both FO₂ values
        assert_relative_eq!(c32.n2() / c32.ar(), c40.n2() / c40.ar(), epsilon = 1e-9);

        // Components still sum to 1 at the new FO₂
        assert_relative_eq!(c40.sum(), 1.0, epsilon = 1e-12);

        Ok(())
    }

    #[test]
    fn invalid_membrane_fractions_display_mentions_constraint() {
        let msg = InvalidMembraneFractions.to_string();

        assert!(
            msg.contains("fn2") || msg.contains("far") || msg.contains("fco2"),
            "expected fraction names in message, got: {msg}"
        );
    }
}
