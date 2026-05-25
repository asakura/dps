use std::fmt;

use crate::units::{Bar, Meters, Percent};

use super::gas_name;
use crate::environment::DiveEnvironment;
use crate::gas::constants::AIR_N2;

/// Equivalent Air Depth at a given actual depth.
///
/// Produced by [`EANxBlend::ead_at`]. The blend method is erased at this
/// boundary; only FO₂ (for the gas name) and the N₂ fraction matter.
///
/// ```no_run
/// use dps::gas::EANx;
/// use dps::units::{Meters, Percent};
/// // Air: EAD always equals actual depth
/// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
/// let e = air.ead_at(Meters::new(30.0));
/// assert_eq!(e.to_string(), "30.0 m");
/// assert_eq!(e.summary().to_string(), "Air  EAD 30.0 m  @ 30.0 m");
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EAD {
    ead: Meters,
    fo2: Percent,
    actual_depth: Meters,
}

impl EAD {
    /// The equivalent air depth.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
    /// assert_relative_eq!(air.ead_at(Meters::new(30.0)).ead(), Meters::new(30.0), epsilon = 1e-9);
    /// ```
    #[must_use]
    pub const fn ead(self) -> Meters {
        self.ead
    }

    /// The O₂ fraction of the gas that produced this `EAD`.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.ead_at(Meters::new(30.0)).fo2(), Percent::new(0.32).unwrap());
    /// ```
    #[must_use]
    pub const fn fo2(self) -> Percent {
        self.fo2
    }

    /// The depth at which this EAD was evaluated.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.ead_at(Meters::new(30.0)).actual_depth(), Meters::new(30.0));
    /// ```
    #[must_use]
    pub const fn actual_depth(self) -> Meters {
        self.actual_depth
    }

    /// Full-detail formatter: `{gas name}  EAD {ead}  @ {actual_depth}`.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
    /// assert_eq!(
    ///     air.ead_at(Meters::new(30.0)).summary().to_string(),
    ///     "Air  EAD 30.0 m  @ 30.0 m",
    /// );
    /// ```
    #[must_use]
    pub const fn summary(self) -> EADSummary {
        EADSummary(self)
    }

    pub(super) fn new(fo2: Percent, fn2: f64, depth: Meters, env: DiveEnvironment) -> Self {
        let abs = depth / env.water_density() + env.surface_pressure();
        let ead_pressure = abs * (fn2 / f64::from(AIR_N2));
        let ead = (ead_pressure - env.surface_pressure()).max(Bar::new(0.0)) * env.water_density();

        Self {
            ead,
            fo2,
            actual_depth: depth,
        }
    }
}

impl fmt::Display for EAD {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ead.fmt(f)
    }
}

impl From<EAD> for Meters {
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// let e = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap().ead_at(Meters::new(30.0));
    /// assert_eq!(Meters::from(e), e.ead());
    /// ```
    fn from(e: EAD) -> Self {
        e.ead
    }
}

/// Full-detail display: `{gas name}  EAD {ead}  @ {actual_depth}`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EADSummary(EAD);

impl EADSummary {
    /// Unwraps the inner [`EAD`].
    #[must_use]
    pub const fn into_inner(self) -> EAD {
        self.0
    }
}

impl From<EAD> for EADSummary {
    fn from(e: EAD) -> Self {
        Self(e)
    }
}

impl fmt::Display for EADSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}  EAD {}  @ {}",
            gas_name(self.0.fo2),
            self.0,
            self.0.actual_depth
        )
    }
}

impl approx::AbsDiffEq for EAD {
    type Epsilon = f64;

    fn default_epsilon() -> f64 {
        f64::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f64) -> bool {
        self.ead.abs_diff_eq(&other.ead, epsilon)
    }
}

impl approx::RelativeEq for EAD {
    fn default_max_relative() -> f64 {
        f64::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: f64, max_relative: f64) -> bool {
        self.ead.relative_eq(&other.ead, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::DiveEnvironment;
    use crate::gas::EANx;
    use crate::gas::constants::{AIR_N2, AIR_O2};
    use crate::units::{Meters, Percent};
    use approx::assert_relative_eq;
    use color_eyre::{Result, eyre::eyre};

    fn ean(fraction: f64) -> Result<EANx> {
        let pct =
            Percent::new(fraction).ok_or_else(|| eyre!("fraction {fraction} out of [0.0, 1.0]"))?;

        Ok(EANx::try_from(pct)?)
    }

    mod equivalent_air_depth {
        use super::*;

        #[test]
        fn display_shows_ead_depth() -> Result<()> {
            // Air at 30 m: EAD == actual depth == 30.0 m
            let e = ean(f64::from(AIR_O2))?.ead_at(Meters::new(30.0));

            assert_eq!(e.to_string(), "30.0 m");

            Ok(())
        }

        #[test]
        fn ead_accessor_returns_meters() -> Result<()> {
            let e = ean(f64::from(AIR_O2))?.ead_at(Meters::new(30.0));

            assert_relative_eq!(e.ead(), Meters::new(30.0), epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn fo2_is_preserved() -> Result<()> {
            let fo2 = Percent::new(0.32).ok_or_else(|| eyre!("invalid"))?;
            let e = ean(0.32)?.ead_at(Meters::new(30.0));

            assert_eq!(e.fo2(), fo2);

            Ok(())
        }

        #[test]
        fn depth_is_preserved() -> Result<()> {
            let depth = Meters::new(30.0);
            let e = ean(0.32)?.ead_at(depth);

            assert_eq!(e.actual_depth(), depth);

            Ok(())
        }

        #[test]
        fn from_gives_meters() -> Result<()> {
            let e = ean(f64::from(AIR_O2))?.ead_at(Meters::new(30.0));

            assert_eq!(Meters::from(e), e.ead());

            Ok(())
        }

        #[test]
        fn enriched_air_gives_shallower_ead() -> Result<()> {
            let depth = Meters::new(30.0);
            let e = ean(0.32)?.ead_at(depth);

            assert!(
                e.ead() < depth,
                "EANx 32 should have a shallower EAD than 30 m"
            );

            Ok(())
        }

        #[test]
        fn surface_depth_clamps_to_zero() -> Result<()> {
            // EANx 32 at surface: less N₂ than air → clamped to 0
            let e = ean(0.32)?.ead_at(Meters::new(0.0));

            assert_relative_eq!(e.ead(), Meters::new(0.0), epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn ean32_pp_formula_at_30m() -> Result<()> {
            let env = DiveEnvironment::standard();
            let mix = ean(0.32)?;
            let depth = Meters::new(30.0);
            let abs = depth / env.water_density() + env.surface_pressure();
            let ead_pressure = abs * (mix.fn2() / f64::from(AIR_N2));
            let expected =
                (ead_pressure - env.surface_pressure()).max(Bar::new(0.0)) * env.water_density();

            assert_relative_eq!(
                mix.ead_at(Meters::new(30.0)).ead(),
                expected,
                epsilon = 1e-9
            );

            Ok(())
        }
    }

    mod ead_summary {
        use super::*;

        #[test]
        fn summary_formats_full_detail() -> Result<()> {
            // Air at 30 m: EAD == 30.0 m
            let e = ean(f64::from(AIR_O2))?.ead_at(Meters::new(30.0));

            assert_eq!(e.summary().to_string(), "Air  EAD 30.0 m  @ 30.0 m");

            Ok(())
        }

        #[test]
        fn summary_shows_ead_not_actual_depth() -> Result<()> {
            // EANx 32 at 30 m: EAD ≈ 24.4 m ≠ 30.0 m — verifies the two depth
            // fields are not interchangeable in the format string.
            let e = ean(0.32)?.ead_at(Meters::new(30.0));

            assert_eq!(e.summary().to_string(), "EANx 32  EAD 24.4 m  @ 30.0 m");

            Ok(())
        }

        #[test]
        fn into_inner_recovers_ead() -> Result<()> {
            let e = ean(0.32)?.ead_at(Meters::new(30.0));

            assert_eq!(e.summary().into_inner(), e);

            Ok(())
        }

        #[test]
        fn from_impl_matches_summary_method() -> Result<()> {
            let e = ean(0.32)?.ead_at(Meters::new(30.0));

            assert_eq!(EADSummary::from(e).to_string(), e.summary().to_string());

            Ok(())
        }
    }
}
