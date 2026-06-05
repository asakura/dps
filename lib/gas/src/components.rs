//! Full mole-fraction breakdown of a breathing gas.
//!
//! Provides [`GasComponents`], the five-component ($\ce{O2}$, $\ce{N2}$, $\ce{Ar}$, $\ce{CO2}$, and lumped traces)
//! output of [`BlendMethod::components`](crate::prelude::BlendMethod::components).
//!
//! Instances are constructed exclusively inside the blend machinery and carry the
//! invariant that all five fractions sum to 1.0. Physical properties — molar mass for
//! gas-density calculations, and narcotic fraction for EAD/END/MND — are derived from
//! the components.

use super::constants::AR_NARCOTIC_POTENCY;

// Molecular weights (g/mol)

const MW_O2: f64 = 31.9988;
const MW_N2: f64 = 28.0134;
const MW_AR: f64 = 39.948;
const MW_CO2: f64 = 44.0095;
const MW_OTHER: f64 = 20.1797; // Neon — dominant trace noble gas by mole fraction

/// Complete mole-fraction breakdown of a breathing gas.
///
/// Invariant: `o2() + n2() + ar() + co2() + other() = 1.0`
/// (within floating-point precision).
///
/// Produced exclusively by [`EANxBlend::components`](crate::prelude::EANxBlend::components); the fields are private to
/// prevent construction of invalid mixes outside the blend-method machinery.
///
/// ```
/// use dps_gas::prelude::EANx;
/// use dps_units::Percent;
///
/// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
/// let c = air.components();
/// assert!((c.sum() - 1.0).abs() < 1e-12);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "GasComponentsShadow"))]
pub struct GasComponents {
    o2: f64,
    n2: f64,
    ar: f64,
    co2: f64,
    other: f64,
}

#[cfg(feature = "serde")]
#[derive(::serde::Deserialize)]
struct GasComponentsShadow {
    o2: f64,
    n2: f64,
    ar: f64,
    co2: f64,
    other: f64,
}

#[cfg(feature = "serde")]
impl TryFrom<GasComponentsShadow> for GasComponents {
    type Error = &'static str;

    fn try_from(shadow: GasComponentsShadow) -> Result<Self, Self::Error> {
        let sum = shadow.o2 + shadow.n2 + shadow.ar + shadow.co2 + shadow.other;

        if (sum - 1.0).abs() > 1e-6 {
            return Err("GasComponents fractions must sum to 1.0");
        }

        Ok(Self {
            o2: shadow.o2,
            n2: shadow.n2,
            ar: shadow.ar,
            co2: shadow.co2,
            other: shadow.other,
        })
    }
}

impl GasComponents {
    pub(super) fn new(o2: f64, n2: f64, ar: f64, co2: f64, other: f64) -> Self {
        debug_assert!(
            (o2 + n2 + ar + co2 + other - 1.0).abs() < 1e-6,
            "GasComponents must sum to 1.0, got {}",
            o2 + n2 + ar + co2 + other
        );

        Self {
            o2,
            n2,
            ar,
            co2,
            other,
        }
    }

    /// Oxygen fraction.
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
    /// # use approx::assert_relative_eq;
    /// let c = EANx::try_from(Percent::new(0.32).unwrap()).unwrap().components();
    /// assert_relative_eq!(c.o2(), 0.32, epsilon = 1e-9);
    /// ```
    #[must_use]
    pub const fn o2(self) -> f64 {
        self.o2
    }

    /// Nitrogen fraction.
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
    /// let c = EANx::try_from(Percent::new(0.32).unwrap()).unwrap().components();
    /// // N₂ is the dominant diluent in partial-pressure nitrox
    /// assert!(c.n2() > 0.0);
    /// ```
    #[must_use]
    pub const fn n2(self) -> f64 {
        self.n2
    }

    /// Argon fraction.
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
    /// let c = EANx::try_from(Percent::new(0.32).unwrap()).unwrap().components();
    /// // Ar is present as a trace component of air-derived diluent
    /// assert!(c.ar() > 0.0 && c.ar() < 0.01);
    /// ```
    #[must_use]
    pub const fn ar(self) -> f64 {
        self.ar
    }

    /// Carbon dioxide fraction.
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
    /// let c = EANx::try_from(Percent::new(0.32).unwrap()).unwrap().components();
    /// // CO₂ is present at air-trace concentrations (< 500 ppm)
    /// assert!(c.co2() > 0.0 && c.co2() < 0.001);
    /// ```
    #[must_use]
    pub const fn co2(self) -> f64 {
        self.co2
    }

    /// Lumped trace-gas fraction: $\ce{Ne}$, $\ce{He}$, $\ce{CH4}$, $\ce{Kr}$, $\ce{H2}$, $\ce{N2O}$, $\ce{Xe}$, …
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
    /// let c = EANx::try_from(Percent::new(0.32).unwrap()).unwrap().components();
    /// assert!(c.other() >= 0.0 && c.other() < c.co2());
    /// ```
    #[must_use]
    pub const fn other(self) -> f64 {
        self.other
    }

    /// Sum of all component fractions; equals 1.0 for a valid mix.
    ///
    /// ```
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
    /// # use approx::assert_relative_eq;
    /// let c = EANx::try_from(Percent::new(0.32).unwrap()).unwrap().components();
    /// assert_relative_eq!(c.sum(), 1.0, epsilon = 1e-12);
    /// ```
    #[must_use]
    pub const fn sum(self) -> f64 {
        self.o2 + self.n2 + self.ar + self.co2 + self.other
    }

    /// Mean molar mass in $\pu{g/mol}$ (used for gas density).
    ///
    /// ```no_run
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
    /// // Air molar mass ≈ 28.97 g/mol
    /// let c = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap().components();
    /// assert!((c.molar_mass() - 28.97).abs() < 0.01);
    /// ```
    #[must_use]
    pub const fn molar_mass(self) -> f64 {
        self.other.mul_add(
            MW_OTHER,
            self.co2.mul_add(
                MW_CO2,
                self.ar
                    .mul_add(MW_AR, self.o2.mul_add(MW_O2, self.n2 * MW_N2)),
            ),
        )
    }

    /// Narcotic fraction under the NOAA model: $\ce{N2} + 1.5 \times \ce{Ar}$.
    ///
    /// $\ce{O2}$ is treated as non-narcotic; $\ce{CO2}$ narcosis from inspired gas at
    /// air-trace concentrations is negligible and excluded.
    ///
    /// ```no_run
    /// use dps_gas::prelude::EANx;
    /// use dps_units::Percent;
    /// // Air narcotic fraction ≈ 0.7948
    /// let c = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap().components();
    /// assert!((c.narcotic() - 0.7948).abs() < 0.001);
    /// ```
    #[must_use]
    pub const fn narcotic(self) -> f64 {
        AR_NARCOTIC_POTENCY.mul_add(self.ar, self.n2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::eanx::{EANx, InvalidEANxError};
    use dps_units::Percent;

    use approx::assert_relative_eq;

    fn air() -> Result<GasComponents, InvalidEANxError> {
        Ok(EANx::try_from(Percent::new(0.20946)?)?.components())
    }

    fn ean32() -> Result<GasComponents, InvalidEANxError> {
        Ok(EANx::try_from(Percent::new(0.32)?)?.components())
    }

    mod accessors {
        use super::*;

        #[test]
        fn o2_returns_oxygen_fraction() -> Result<(), InvalidEANxError> {
            assert_relative_eq!(air()?.o2(), 0.20946, epsilon = 1e-9);
            Ok(())
        }

        #[test]
        fn n2_is_dominant_diluent_in_air() -> Result<(), InvalidEANxError> {
            let c = air()?;
            assert!(c.n2() > 0.78 && c.n2() < 0.79);
            Ok(())
        }

        #[test]
        fn ar_matches_noaa_air_composition() -> Result<(), InvalidEANxError> {
            assert_relative_eq!(air()?.ar(), 0.00934, epsilon = 1e-4);
            Ok(())
        }

        #[test]
        fn co2_is_sub_ppt_trace() -> Result<(), InvalidEANxError> {
            let c = air()?;
            assert!(c.co2() > 0.0 && c.co2() < 0.001);
            Ok(())
        }

        #[test]
        fn other_is_smaller_than_co2() -> Result<(), InvalidEANxError> {
            let c = air()?;
            assert!(c.other() >= 0.0 && c.other() < c.co2());
            Ok(())
        }

        #[test]
        fn sum_equals_one_for_air() -> Result<(), InvalidEANxError> {
            assert_relative_eq!(air()?.sum(), 1.0, epsilon = 1e-12);
            Ok(())
        }
    }

    #[test]
    fn molar_mass_of_air_is_approximately_28_97() -> Result<(), InvalidEANxError> {
        assert_relative_eq!(air()?.molar_mass(), 28.97, epsilon = 0.01);

        Ok(())
    }

    #[test]
    fn molar_mass_increases_with_fo2() -> Result<(), InvalidEANxError> {
        // Pure O₂ (MW 32) raises the mean molar mass above air (MW ≈ 28.97)
        assert!(ean32()?.molar_mass() > air()?.molar_mass());

        Ok(())
    }

    #[test]
    fn narcotic_fraction_equals_n2_plus_1_5_ar() -> Result<(), InvalidEANxError> {
        let air = air()?;

        assert_relative_eq!(
            air.narcotic(),
            1.5f64.mul_add(air.ar(), air.n2()),
            epsilon = 1e-12
        );

        Ok(())
    }

    #[test]
    fn narcotic_fraction_of_ean32_is_less_than_air() -> Result<(), InvalidEANxError> {
        // Higher FO₂ → less N₂/Ar → lower narcotic load
        assert!(ean32()?.narcotic() < air()?.narcotic());

        Ok(())
    }
}
