use std::fmt;

use crate::units::{Meters, Percent};

use super::gas_name;
use crate::environment::DiveEnvironment;
use crate::gas::constants::AIR_NARCOTIC;

/// Equivalent Narcotic Depth at a given actual depth.
///
/// Produced by [`EANxBlend::end_at`](crate::gas::EANxBlend::end_at). The blend method is erased at this
/// boundary; only FO₂ (for the gas name) and the narcotic fraction matter.
///
/// ```no_run
/// use dps::gas::EANx;
/// use dps::units::{Meters, Percent};
/// // Air: END always equals actual depth
/// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
/// let e = air.end_at(Meters::new(30.0));
/// assert_eq!(e.to_string(), "30.0 m");
/// assert_eq!(e.summary().to_string(), "Air  END 30.0 m  @ 30.0 m");
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct END {
    end: Meters,
    fo2: Percent,
    actual_depth: Meters,
}

impl END {
    /// The equivalent narcotic depth.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
    /// assert_relative_eq!(air.end_at(Meters::new(30.0)).end(), Meters::new(30.0), epsilon = 1e-9);
    /// ```
    #[must_use]
    pub const fn end(self) -> Meters {
        self.end
    }

    /// The O₂ fraction of the gas that produced this `END`.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.end_at(Meters::new(30.0)).fo2(), Percent::new(0.32).unwrap());
    /// ```
    #[must_use]
    pub const fn fo2(self) -> Percent {
        self.fo2
    }

    /// The depth at which this END was evaluated.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.end_at(Meters::new(30.0)).actual_depth(), Meters::new(30.0));
    /// ```
    #[must_use]
    pub const fn actual_depth(self) -> Meters {
        self.actual_depth
    }

    /// Full-detail formatter: `{gas name}  END {end}  @ {actual_depth}`.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
    /// assert_eq!(
    ///     air.end_at(Meters::new(30.0)).summary().to_string(),
    ///     "Air  END 30.0 m  @ 30.0 m",
    /// );
    /// ```
    #[must_use]
    pub const fn summary(self) -> ENDSummary {
        ENDSummary(self)
    }

    pub(super) fn new(fo2: Percent, narcotic: f64, depth: Meters, env: DiveEnvironment) -> Self {
        let abs = env.absolute_pressure(depth);
        let end_pressure = abs * (narcotic / f64::from(AIR_NARCOTIC));
        let end = env.depth(end_pressure);

        Self {
            end,
            fo2,
            actual_depth: depth,
        }
    }
}

impl fmt::Display for END {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.end.fmt(f)
    }
}

impl From<END> for Meters {
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Meters, Percent};
    /// let e = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap().end_at(Meters::new(30.0));
    /// assert_eq!(Meters::from(e), e.end());
    /// ```
    fn from(e: END) -> Self {
        e.end
    }
}

/// Full-detail display: `{gas name}  END {end}  @ {actual_depth}`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ENDSummary(END);

impl ENDSummary {
    /// Unwraps the inner [`END`].
    #[must_use]
    pub const fn into_inner(self) -> END {
        self.0
    }
}

impl From<END> for ENDSummary {
    fn from(e: END) -> Self {
        Self(e)
    }
}

impl fmt::Display for ENDSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}  END {}  @ {}",
            gas_name(self.0.fo2),
            self.0,
            self.0.actual_depth
        )
    }
}

impl approx::AbsDiffEq for END {
    type Epsilon = f64;

    fn default_epsilon() -> f64 {
        f64::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f64) -> bool {
        self.end.abs_diff_eq(&other.end, epsilon)
    }
}

impl approx::RelativeEq for END {
    fn default_max_relative() -> f64 {
        f64::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: f64, max_relative: f64) -> bool {
        self.end.relative_eq(&other.end, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::DiveEnvironment;
    use crate::gas::EANx;
    use crate::gas::constants::{AIR_NARCOTIC, AIR_O2, AR_NARCOTIC_POTENCY};
    use crate::gas::eanx::InvalidEANxError;
    use crate::units::{Meters, Percent};
    use approx::assert_relative_eq;

    fn ean(fraction: f64) -> Result<EANx, InvalidEANxError> {
        let pct = Percent::new(fraction)?;
        EANx::try_from(pct)
    }

    mod equivalent_narcotic_depth {
        use super::*;

        #[test]
        fn display_shows_end_depth() -> Result<(), InvalidEANxError> {
            // Air at 30 m: END == actual depth == 30.0 m
            let e = ean(f64::from(AIR_O2))?.end_at(Meters::new(30.0));

            assert_eq!(e.to_string(), "30.0 m");

            Ok(())
        }

        #[test]
        fn end_accessor_returns_meters() -> Result<(), InvalidEANxError> {
            let e = ean(f64::from(AIR_O2))?.end_at(Meters::new(30.0));

            assert_relative_eq!(e.end(), Meters::new(30.0), epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn fo2_is_preserved() -> Result<(), InvalidEANxError> {
            let fo2 = Percent::new(0.32)?;
            let e = ean(0.32)?.end_at(Meters::new(30.0));

            assert_eq!(e.fo2(), fo2);

            Ok(())
        }

        #[test]
        fn depth_is_preserved() -> Result<(), InvalidEANxError> {
            let depth = Meters::new(30.0);
            let e = ean(0.32)?.end_at(depth);

            assert_eq!(e.actual_depth(), depth);

            Ok(())
        }

        #[test]
        fn from_gives_meters() -> Result<(), InvalidEANxError> {
            let e = ean(f64::from(AIR_O2))?.end_at(Meters::new(30.0));

            assert_eq!(Meters::from(e), e.end());

            Ok(())
        }

        #[test]
        fn enriched_air_gives_shallower_end() -> Result<(), InvalidEANxError> {
            let depth = Meters::new(30.0);
            let e = ean(0.32)?.end_at(depth);

            assert!(
                e.end() < depth,
                "EANx 32 should have a shallower END than 30 m"
            );

            Ok(())
        }

        #[test]
        fn surface_depth_clamps_to_zero() -> Result<(), InvalidEANxError> {
            // EANx 32 at surface: narcotic load < air narcotic → clamped to 0
            let e = ean(0.32)?.end_at(Meters::new(0.0));

            assert_relative_eq!(e.end(), Meters::new(0.0), epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn ean32_pp_formula_at_30m() -> Result<(), InvalidEANxError> {
            let env = DiveEnvironment::standard();
            let mix = ean(0.32)?;
            let c = mix.components();
            let narcotic_mix = AR_NARCOTIC_POTENCY.mul_add(c.ar(), c.n2());
            let depth = Meters::new(30.0);
            let abs = env.absolute_pressure(depth);
            let end_pressure = abs * (narcotic_mix / f64::from(AIR_NARCOTIC));
            let expected = env.depth(end_pressure);

            assert_relative_eq!(
                mix.end_at(Meters::new(30.0)).end(),
                expected,
                epsilon = 1e-9
            );

            Ok(())
        }

        #[test]
        fn higher_fo2_gives_lower_end() -> Result<(), InvalidEANxError> {
            let depth = Meters::new(40.0);
            assert!(ean(0.32)?.end_at(depth).end() < ean(0.21)?.end_at(depth).end());

            Ok(())
        }
    }

    mod end_summary {
        use super::*;

        #[test]
        fn summary_formats_full_detail() -> Result<(), InvalidEANxError> {
            // Air at 30 m: END == 30.0 m
            let e = ean(f64::from(AIR_O2))?.end_at(Meters::new(30.0));

            assert_eq!(e.summary().to_string(), "Air  END 30.0 m  @ 30.0 m");

            Ok(())
        }

        #[test]
        fn summary_shows_end_not_actual_depth() -> Result<(), InvalidEANxError> {
            // EANx 32 at 30 m: END ≈ 24.4 m ≠ 30.0 m — verifies the two depth
            // fields are not interchangeable in the format string.
            let e = ean(0.32)?.end_at(Meters::new(30.0));

            assert_eq!(e.summary().to_string(), "EANx 32  END 24.4 m  @ 30.0 m");

            Ok(())
        }

        #[test]
        fn into_inner_recovers_end() -> Result<(), InvalidEANxError> {
            let e = ean(0.32)?.end_at(Meters::new(30.0));

            assert_eq!(e.summary().into_inner(), e);

            Ok(())
        }

        #[test]
        fn from_impl_matches_summary_method() -> Result<(), InvalidEANxError> {
            let e = ean(0.32)?.end_at(Meters::new(30.0));

            assert_eq!(ENDSummary::from(e).to_string(), e.summary().to_string());

            Ok(())
        }
    }
}
