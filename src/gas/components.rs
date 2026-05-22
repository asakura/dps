use super::constants::{AR_NARCOTIC_POTENCY, MW_AR, MW_CO2, MW_N2, MW_O2, MW_OTHER};

/// Complete mole-fraction breakdown of a breathing gas.
///
/// Invariant: `o2() + n2() + ar() + co2() + other() = 1.0`
/// (within floating-point precision).
///
/// Produced exclusively by [`EANxBlend::components`]; the fields are private to
/// prevent construction of invalid mixes outside the blend-method machinery.
///
/// ```no_run
/// use dps::gas::EANx;
/// use dps::units::Percent;
///
/// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
/// let c = air.components();
/// assert!((c.sum() - 1.0).abs() < 1e-12);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GasComponents {
    o2: f64,
    n2: f64,
    ar: f64,
    co2: f64,
    other: f64,
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
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
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
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
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
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
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
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    /// let c = EANx::try_from(Percent::new(0.32).unwrap()).unwrap().components();
    /// // CO₂ is present at air-trace concentrations (< 500 ppm)
    /// assert!(c.co2() > 0.0 && c.co2() < 0.001);
    /// ```
    #[must_use]
    pub const fn co2(self) -> f64 {
        self.co2
    }

    /// Lumped trace-gas fraction: Ne, He, CH₄, Kr, H₂, N₂O, Xe, …
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    /// let c = EANx::try_from(Percent::new(0.32).unwrap()).unwrap().components();
    /// assert!(c.other() >= 0.0 && c.other() < c.co2());
    /// ```
    #[must_use]
    pub const fn other(self) -> f64 {
        self.other
    }

    /// Sum of all component fractions; equals 1.0 for a valid mix.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    /// # use approx::assert_relative_eq;
    /// let c = EANx::try_from(Percent::new(0.32).unwrap()).unwrap().components();
    /// assert_relative_eq!(c.sum(), 1.0, epsilon = 1e-12);
    /// ```
    #[must_use]
    pub fn sum(self) -> f64 {
        self.o2 + self.n2 + self.ar + self.co2 + self.other
    }

    /// Mean molar mass in g/mol (used for gas density).
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
    /// // Air molar mass ≈ 28.97 g/mol
    /// let c = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap().components();
    /// assert!((c.molar_mass() - 28.97).abs() < 0.01);
    /// ```
    #[must_use]
    pub fn molar_mass(self) -> f64 {
        self.other.mul_add(
            MW_OTHER,
            self.co2.mul_add(
                MW_CO2,
                self.ar
                    .mul_add(MW_AR, self.o2.mul_add(MW_O2, self.n2 * MW_N2)),
            ),
        )
    }

    /// Narcotic fraction under the NOAA model: N₂ + 1.5 × Ar.
    ///
    /// O₂ is treated as non-narcotic; CO₂ narcosis from inspired gas at
    /// air-trace concentrations is negligible and excluded.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::Percent;
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
    use crate::gas::EANx;
    use crate::units::Percent;
    use approx::assert_relative_eq;
    use color_eyre::Result;

    fn air() -> Result<GasComponents, Box<dyn std::error::Error>> {
        Ok(EANx::try_from(Percent::new(0.20946).ok_or("0.20946 is in [0.0, 1.0]")?)?.components())
    }

    fn ean32() -> Result<GasComponents, Box<dyn std::error::Error>> {
        Ok(EANx::try_from(Percent::new(0.32).ok_or("0.32 is in [0.0, 1.0]")?)?.components())
    }

    #[test]
    fn molar_mass_of_air_is_approximately_28_97() -> Result<(), Box<dyn std::error::Error>> {
        assert_relative_eq!(air()?.molar_mass(), 28.97, epsilon = 0.01);

        Ok(())
    }

    #[test]
    fn molar_mass_increases_with_fo2() -> Result<(), Box<dyn std::error::Error>> {
        // Pure O₂ (MW 32) raises the mean molar mass above air (MW ≈ 28.97)
        assert!(ean32()?.molar_mass() > air()?.molar_mass());

        Ok(())
    }

    #[test]
    fn narcotic_fraction_equals_n2_plus_1_5_ar() -> Result<(), Box<dyn std::error::Error>> {
        let air = air()?;
        assert_relative_eq!(
            air.narcotic(),
            1.5f64.mul_add(air.ar(), air.n2()),
            epsilon = 1e-12
        );

        Ok(())
    }

    #[test]
    fn narcotic_fraction_of_ean32_is_less_than_air() -> Result<(), Box<dyn std::error::Error>> {
        // Higher FO₂ → less N₂/Ar → lower narcotic load
        assert!(ean32()?.narcotic() < air()?.narcotic());

        Ok(())
    }
}
