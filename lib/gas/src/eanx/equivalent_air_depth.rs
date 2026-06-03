use super::gas_name;

use crate::constants::AIR_N2;

use dps_environment::DiveEnvironment;
use dps_units::{Meters, Percent};

use std::fmt;

/// Equivalent Air Depth at a given actual depth.
///
/// Produced by [`EANxBlend::ead_at`](crate::EANxBlend::ead_at). The blend method is erased at this
/// boundary; only FO₂ (for the gas name) and the N₂ fraction matter.
///
/// ```no_run
/// use dps_gas::EANx;
/// use dps_units::{Meters, Percent};
/// // Air: EAD always equals actual depth
/// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
/// let e = air.ead_at(Meters::new(30.0));
/// assert_eq!(e.to_string(), "30.0 m");
/// assert_eq!(e.summary().to_string(), "Air  EAD 30.0 m  @ 30.0 m");
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "EADShadow"))]
pub struct EAD {
    ead: Meters,
    fo2: Percent,
    actual_depth: Meters,
}

#[cfg(feature = "serde")]
#[derive(::serde::Deserialize)]
struct EADShadow {
    ead: Meters,
    fo2: Percent,
    actual_depth: Meters,
}

#[cfg(feature = "serde")]
impl From<EADShadow> for EAD {
    fn from(shadow: EADShadow) -> Self {
        Self {
            ead: shadow.ead,
            fo2: shadow.fo2,
            actual_depth: shadow.actual_depth,
        }
    }
}

impl EAD {
    /// The equivalent air depth.
    ///
    /// ```no_run
    /// use dps_gas::EANx;
    /// use dps_units::{Meters, Percent};
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
    /// use dps_gas::EANx;
    /// use dps_units::{Meters, Percent};
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
    /// use dps_gas::EANx;
    /// use dps_units::{Meters, Percent};
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
    /// use dps_gas::EANx;
    /// use dps_units::{Meters, Percent};
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
        let abs = env.absolute_pressure(depth);
        let ead_pressure = abs * (fn2 / f64::from(AIR_N2));
        let ead = env.depth(ead_pressure);

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
    /// use dps_gas::EANx;
    /// use dps_units::{Meters, Percent};
    /// let e = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap().ead_at(Meters::new(30.0));
    /// assert_eq!(Meters::from(e), e.ead());
    /// ```
    fn from(e: EAD) -> Self {
        e.ead
    }
}

/// Full-detail display: `{gas name}  EAD {ead}  @ {actual_depth}`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
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

    use crate::EANx;
    use crate::constants::{AIR_N2, AIR_O2};
    use crate::eanx::InvalidEANxError;

    use dps_environment::DiveEnvironment;
    use dps_units::{Meters, Percent};

    use approx::assert_relative_eq;

    fn ean(fraction: f64) -> Result<EANx, InvalidEANxError> {
        let pct = Percent::new(fraction)?;
        EANx::try_from(pct)
    }

    mod equivalent_air_depth {
        use super::*;

        #[test]
        fn display_shows_ead_depth() -> Result<(), InvalidEANxError> {
            // Air at 30 m: EAD == actual depth == 30.0 m
            let e = ean(f64::from(AIR_O2))?.ead_at(Meters::new(30.0));

            assert_eq!(e.to_string(), "30.0 m");

            Ok(())
        }

        #[test]
        fn ead_accessor_returns_meters() -> Result<(), InvalidEANxError> {
            let e = ean(f64::from(AIR_O2))?.ead_at(Meters::new(30.0));

            assert_relative_eq!(e.ead(), Meters::new(30.0), epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn fo2_is_preserved() -> Result<(), InvalidEANxError> {
            let fo2 = Percent::new(0.32)?;
            let e = ean(0.32)?.ead_at(Meters::new(30.0));

            assert_eq!(e.fo2(), fo2);

            Ok(())
        }

        #[test]
        fn depth_is_preserved() -> Result<(), InvalidEANxError> {
            let depth = Meters::new(30.0);
            let e = ean(0.32)?.ead_at(depth);

            assert_eq!(e.actual_depth(), depth);

            Ok(())
        }

        #[test]
        fn from_gives_meters() -> Result<(), InvalidEANxError> {
            let e = ean(f64::from(AIR_O2))?.ead_at(Meters::new(30.0));

            assert_eq!(Meters::from(e), e.ead());

            Ok(())
        }

        #[test]
        fn enriched_air_gives_shallower_ead() -> Result<(), InvalidEANxError> {
            let depth = Meters::new(30.0);
            let e = ean(0.32)?.ead_at(depth);

            assert!(
                e.ead() < depth,
                "EANx 32 should have a shallower EAD than 30 m"
            );

            Ok(())
        }

        #[test]
        fn surface_depth_clamps_to_zero() -> Result<(), InvalidEANxError> {
            // EANx 32 at surface: less N₂ than air → clamped to 0
            let e = ean(0.32)?.ead_at(Meters::new(0.0));

            assert_relative_eq!(e.ead(), Meters::new(0.0), epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn ean32_pp_formula_at_30m() -> Result<(), InvalidEANxError> {
            let env = DiveEnvironment::standard();
            let mix = ean(0.32)?;
            let depth = Meters::new(30.0);
            let abs = env.absolute_pressure(depth);
            let ead_pressure = abs * (mix.fn2() / f64::from(AIR_N2));
            let expected = env.depth(ead_pressure);

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
        fn summary_formats_full_detail() -> Result<(), InvalidEANxError> {
            // Air at 30 m: EAD == 30.0 m
            let e = ean(f64::from(AIR_O2))?.ead_at(Meters::new(30.0));

            assert_eq!(e.summary().to_string(), "Air  EAD 30.0 m  @ 30.0 m");

            Ok(())
        }

        #[test]
        fn summary_shows_ead_not_actual_depth() -> Result<(), InvalidEANxError> {
            // EANx 32 at 30 m: EAD ≈ 24.4 m ≠ 30.0 m — verifies the two depth
            // fields are not interchangeable in the format string.
            let e = ean(0.32)?.ead_at(Meters::new(30.0));

            assert_eq!(e.summary().to_string(), "EANx 32  EAD 24.4 m  @ 30.0 m");

            Ok(())
        }

        #[test]
        fn into_inner_recovers_ead() -> Result<(), InvalidEANxError> {
            let e = ean(0.32)?.ead_at(Meters::new(30.0));

            assert_eq!(e.summary().into_inner(), e);

            Ok(())
        }

        #[test]
        fn from_impl_matches_summary_method() -> Result<(), InvalidEANxError> {
            let e = ean(0.32)?.ead_at(Meters::new(30.0));

            assert_eq!(EADSummary::from(e).to_string(), e.summary().to_string());

            Ok(())
        }
    }
}
