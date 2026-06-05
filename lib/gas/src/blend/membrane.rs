use super::{BlendMethod, sealed};

use crate::components::GasComponents;
use crate::constants::{AIR_DILUENT, AIR_OTHER};

/// Membrane separator blending.
///
/// Hollow-fibre membranes separate gases by differential permeability.
/// The $\ce{N2}$/$\ce{Ar}$/$\ce{CO2}$ ratios in the output depend on the specific membrane
/// material and operating conditions and cannot be derived from $\text{F}\ce{O2}$ alone.
///
/// Construct via [`Membrane::from_analysis`] using a measured gas analysis, or
/// use [`Membrane::typical`] for an approximate model when no analyser is
/// available.
///
/// ```
/// use dps_gas::prelude::{EANxBlend, Membrane};
/// use dps_units::Percent;
///
/// // From a gas-analyser reading at FO₂ 0.32
/// let mem = Membrane::from_analysis(0.32, 0.645, 0.030, 0.003).unwrap();
/// let mix = EANxBlend::new(Percent::new(0.32).unwrap(), mem).unwrap();
///
/// // The same membrane characterisation applies at any target FO₂
/// let mix40 = EANxBlend::new(Percent::new(0.40).unwrap(), mem).unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "MembraneShadow"))]
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

impl Default for Membrane {
    /// Returns [`Membrane::typical()`].
    fn default() -> Self {
        Self::typical()
    }
}

#[cfg(feature = "serde")]
#[derive(::serde::Deserialize)]
struct MembraneShadow {
    fn2: f64,
    far: f64,
    fco2: f64,
    fother: f64,
}

#[cfg(feature = "serde")]
impl TryFrom<MembraneShadow> for Membrane {
    type Error = &'static str;

    fn try_from(shadow: MembraneShadow) -> Result<Self, Self::Error> {
        let sum = shadow.fn2 + shadow.far + shadow.fco2 + shadow.fother;

        if (sum - 1.0).abs() > 1e-6 {
            return Err("Membrane diluent ratios must sum to 1.0");
        }

        Ok(Self {
            fn2: shadow.fn2,
            far: shadow.far,
            fco2: shadow.fco2,
            fother: shadow.fother,
        })
    }
}

impl Membrane {
    /// Constructs a `Membrane` from a gas-analyser reading.
    ///
    /// Pass the absolute mole fractions measured in the membrane output at
    /// any operating point. The diluent fractions are normalised internally
    /// and apply at all target $\text{F}\ce{O2}$ values for the same equipment settings.
    ///
    /// The `other` fraction (noble traces, etc.) is derived as the remainder.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidMembraneFractionsError`] if `fn2 + far + fco2 > 1 − fo2`.
    ///
    /// ```
    /// use dps_gas::prelude::Membrane;
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
    ) -> Result<Self, InvalidMembraneFractionsError> {
        let diluent = 1.0 - fo2;

        if diluent < 1e-9 {
            return Err(InvalidMembraneFractionsError);
        }

        let fother = diluent - fn2 - far - fco2;

        if fother < -1e-6 {
            return Err(InvalidMembraneFractionsError);
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
    /// - $\ce{Ar}$ is enriched to $\approx 2.0\\%$ of the diluent ($\approx 1.7\\times$ air's ratio).
    /// - $\ce{CO2}$ is enriched to $\approx 0.1\\%$ of the diluent ($\ce{CO2}$ permeates readily).
    /// - $\ce{N2}$ fills the remainder.
    ///
    /// These values are indicative only. Use [`Membrane::from_analysis`] for
    /// precision work.
    ///
    /// ```
    /// use dps_gas::prelude::{EANxBlend, Membrane, EANx};
    /// use dps_units::Percent;
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
    fn blend_name(&self) -> &'static str {
        "membrane"
    }

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
/// ```
/// use dps_gas::prelude::{Membrane, InvalidMembraneFractionsError};
/// // fn2 + far + fco2 > 1 − fo2
/// assert!(Membrane::from_analysis(0.32, 0.60, 0.10, 0.005).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
#[error("membrane diluent fractions are invalid: fn2 + far + fco2 must not exceed (1 − fo2)")]
pub struct InvalidMembraneFractionsError;

#[cfg(test)]
mod tests {
    use super::*;

    use approx::assert_relative_eq;

    #[test]
    #[expect(
        clippy::similar_names,
        reason = "fo2/fn2/fco2 are standard gas-fraction notation; the 'f' prefix and gas suffix make each distinct in context"
    )]
    fn from_analysis_roundtrips_components() -> Result<(), InvalidMembraneFractionsError> {
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
    fn diluent_ratios_are_fo2_independent() -> Result<(), InvalidMembraneFractionsError> {
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
        let msg = InvalidMembraneFractionsError.to_string();

        assert!(
            msg.contains("fn2") || msg.contains("far") || msg.contains("fco2"),
            "expected fraction names in message, got: {msg}"
        );
    }

    #[test]
    fn blend_name_is_membrane() {
        assert_eq!(Membrane::typical().blend_name(), "membrane");
    }
}
