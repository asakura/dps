use super::gas_name;

use crate::constants::AIR_NARCOTIC;

use dps_environment::DiveEnvironment;
use dps_units::{Meters, Percent};

use std::fmt;

/// Maximum Narcotic Depth for a given END limit.
///
/// Produced by [`EANxBlend::mnd_at`](crate::EANxBlend::mnd_at). The blend method is erased at this
/// boundary; only $\text{F}\ce{O2}$ (for the gas name) and the narcotic fraction matter.
///
/// ```no_run
/// use dps_gas::EANx;
/// use dps_units::{Meters, Percent};
/// // Air: MND always equals the END limit
/// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
/// let m = air.mnd_at(Meters::new(30.0));
/// assert_eq!(m.to_string(), "30.0 m");
/// assert_eq!(m.summary().to_string(), "Air  MND 30.0 m  @ END 30.0 m");
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "MNDShadow"))]
pub struct MND {
    mnd: Meters,
    fo2: Percent,
    end_limit: Meters,
}

#[cfg(feature = "serde")]
#[derive(::serde::Deserialize)]
struct MNDShadow {
    mnd: Meters,
    fo2: Percent,
    end_limit: Meters,
}

#[cfg(feature = "serde")]
impl From<MNDShadow> for MND {
    fn from(shadow: MNDShadow) -> Self {
        Self {
            mnd: shadow.mnd,
            fo2: shadow.fo2,
            end_limit: shadow.end_limit,
        }
    }
}

impl MND {
    /// The maximum narcotic depth.
    ///
    /// ```no_run
    /// use dps_gas::EANx;
    /// use dps_units::{Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
    /// assert_relative_eq!(air.mnd_at(Meters::new(30.0)).mnd(), Meters::new(30.0), epsilon = 1e-9);
    /// ```
    #[must_use]
    pub const fn mnd(self) -> Meters {
        self.mnd
    }

    /// The $\ce{O2}$ fraction of the gas that produced this `MND`.
    ///
    /// ```no_run
    /// use dps_gas::EANx;
    /// use dps_units::{Meters, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.mnd_at(Meters::new(30.0)).fo2(), Percent::new(0.32).unwrap());
    /// ```
    #[must_use]
    pub const fn fo2(self) -> Percent {
        self.fo2
    }

    /// The END limit used to compute this `MND`.
    ///
    /// ```no_run
    /// use dps_gas::EANx;
    /// use dps_units::{Meters, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.mnd_at(Meters::new(30.0)).end_limit(), Meters::new(30.0));
    /// ```
    #[must_use]
    pub const fn end_limit(self) -> Meters {
        self.end_limit
    }

    /// Full-detail formatter: `{gas name}  MND {mnd}  @ END {end_limit}`.
    ///
    /// ```no_run
    /// use dps_gas::EANx;
    /// use dps_units::{Meters, Percent};
    /// let air = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap();
    /// assert_eq!(
    ///     air.mnd_at(Meters::new(30.0)).summary().to_string(),
    ///     "Air  MND 30.0 m  @ END 30.0 m",
    /// );
    /// ```
    #[must_use]
    pub const fn summary(self) -> MNDSummary {
        MNDSummary(self)
    }

    pub(super) fn new(
        fo2: Percent,
        narcotic: f64,
        end_limit: Meters,
        env: DiveEnvironment,
    ) -> Self {
        debug_assert!(
            narcotic >= 1e-9,
            "narcotic fraction is zero — impossible for any EAN mix"
        );

        let end_abs = env.absolute_pressure(end_limit);
        let mnd_pressure = end_abs / (narcotic / f64::from(AIR_NARCOTIC));
        let mnd = env.depth(mnd_pressure);

        Self {
            mnd,
            fo2,
            end_limit,
        }
    }
}

impl fmt::Display for MND {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.mnd.fmt(f)
    }
}

impl From<MND> for Meters {
    /// ```no_run
    /// use dps_gas::EANx;
    /// use dps_units::{Meters, Percent};
    /// let m = EANx::try_from(Percent::new(0.20946).unwrap()).unwrap().mnd_at(Meters::new(30.0));
    /// assert_eq!(Meters::from(m), m.mnd());
    /// ```
    fn from(m: MND) -> Self {
        m.mnd
    }
}

/// Full-detail display: `{gas name}  MND {mnd}  @ END {end_limit}`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct MNDSummary(MND);

impl MNDSummary {
    /// Unwraps the inner [`MND`].
    #[must_use]
    pub const fn into_inner(self) -> MND {
        self.0
    }
}

impl From<MND> for MNDSummary {
    fn from(m: MND) -> Self {
        Self(m)
    }
}

impl fmt::Display for MNDSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}  MND {}  @ END {}",
            gas_name(self.0.fo2),
            self.0,
            self.0.end_limit
        )
    }
}

impl approx::AbsDiffEq for MND {
    type Epsilon = f64;

    fn default_epsilon() -> f64 {
        f64::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f64) -> bool {
        self.mnd.abs_diff_eq(&other.mnd, epsilon)
    }
}

impl approx::RelativeEq for MND {
    fn default_max_relative() -> f64 {
        f64::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: f64, max_relative: f64) -> bool {
        self.mnd.relative_eq(&other.mnd, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::EANx;
    use crate::constants::AIR_O2;
    use crate::eanx::InvalidEANxError;

    use dps_units::{Meters, Percent};

    use approx::assert_relative_eq;

    fn ean(fraction: f64) -> Result<EANx, InvalidEANxError> {
        let pct = Percent::new(fraction)?;
        EANx::try_from(pct)
    }

    mod maximum_narcotic_depth {
        use super::*;

        #[test]
        fn display_shows_mnd_depth() -> Result<(), InvalidEANxError> {
            // Air: MND == END limit == 30.0 m
            let m = ean(f64::from(AIR_O2))?.mnd_at(Meters::new(30.0));

            assert_eq!(m.to_string(), "30.0 m");

            Ok(())
        }

        #[test]
        fn mnd_accessor_returns_meters() -> Result<(), InvalidEANxError> {
            let m = ean(f64::from(AIR_O2))?.mnd_at(Meters::new(30.0));

            assert_relative_eq!(m.mnd(), Meters::new(30.0), epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn fo2_is_preserved() -> Result<(), InvalidEANxError> {
            let fo2 = Percent::new(0.32)?;
            let m = ean(0.32)?.mnd_at(Meters::new(30.0));

            assert_eq!(m.fo2(), fo2);

            Ok(())
        }

        #[test]
        fn end_limit_is_preserved() -> Result<(), InvalidEANxError> {
            let limit = Meters::new(30.0);
            let m = ean(0.32)?.mnd_at(limit);

            assert_eq!(m.end_limit(), limit);

            Ok(())
        }

        #[test]
        fn from_gives_meters() -> Result<(), InvalidEANxError> {
            let m = ean(f64::from(AIR_O2))?.mnd_at(Meters::new(30.0));

            assert_eq!(Meters::from(m), m.mnd());

            Ok(())
        }

        #[test]
        fn enriched_air_gives_deeper_mnd() -> Result<(), InvalidEANxError> {
            let limit = Meters::new(30.0);
            let m = ean(0.32)?.mnd_at(limit);

            assert!(
                m.mnd() > limit,
                "EANx 32 can dive deeper than 30 m before reaching END 30 m"
            );

            Ok(())
        }

        #[test]
        fn mnd_is_inverse_of_end() -> Result<(), InvalidEANxError> {
            let mix = ean(0.32)?;
            let end_limit = Meters::new(30.0);
            let mnd = mix.mnd_at(end_limit);

            assert_relative_eq!(
                mix.end_at(Meters::from(mnd)).end(),
                end_limit,
                epsilon = 1e-6
            );

            Ok(())
        }
    }

    mod mnd_summary {
        use super::*;

        #[test]
        fn summary_formats_full_detail() -> Result<(), InvalidEANxError> {
            // Air: MND == 30.0 m at END limit 30.0 m
            let m = ean(f64::from(AIR_O2))?.mnd_at(Meters::new(30.0));

            assert_eq!(m.summary().to_string(), "Air  MND 30.0 m  @ END 30.0 m");

            Ok(())
        }

        #[test]
        fn summary_shows_mnd_not_end_limit() -> Result<(), InvalidEANxError> {
            // EANx 32 at END limit 30 m: MND ≈ 36.5 m ≠ 30.0 m — verifies the two
            // depth fields are not interchangeable in the format string.
            let m = ean(0.32)?.mnd_at(Meters::new(30.0));

            assert_eq!(m.summary().to_string(), "EANx 32  MND 36.5 m  @ END 30.0 m");

            Ok(())
        }

        #[test]
        fn into_inner_recovers_mnd() -> Result<(), InvalidEANxError> {
            let m = ean(0.32)?.mnd_at(Meters::new(30.0));

            assert_eq!(m.summary().into_inner(), m);

            Ok(())
        }

        #[test]
        fn from_impl_matches_summary_method() -> Result<(), InvalidEANxError> {
            let m = ean(0.32)?.mnd_at(Meters::new(30.0));

            assert_eq!(MNDSummary::from(m).to_string(), m.summary().to_string());

            Ok(())
        }
    }
}
