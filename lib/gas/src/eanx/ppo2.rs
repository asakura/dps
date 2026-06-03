use super::gas_name;

use dps_environment::DiveEnvironment;
use dps_units::{Bar, Meters, Percent};

use std::fmt;

/// Partial pressure of O₂ at a given depth.
///
/// Produced by [`EANxBlend::ppo2_at`](crate::EANxBlend::ppo2_at). The blend method is erased at this
/// boundary because ppO₂ depends only on FO₂ and depth.
///
/// ```no_run
/// use dps_gas::EANx;
/// use dps_units::{Meters, Percent};
/// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
/// let p = ean32.ppo2_at(Meters::new(33.75));
/// assert_eq!(p.to_string(), "1.4 bar");
/// assert_eq!(p.summary().to_string(), "EANx 32  ppO₂ 1.4 bar  @ 33.8 m");
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "PPO2Shadow"))]
pub struct PPO2 {
    ppo2: Bar,
    fo2: Percent,
    depth: Meters,
}

#[cfg(feature = "serde")]
#[derive(::serde::Deserialize)]
struct PPO2Shadow {
    ppo2: Bar,
    fo2: Percent,
    depth: Meters,
}

#[cfg(feature = "serde")]
impl From<PPO2Shadow> for PPO2 {
    fn from(shadow: PPO2Shadow) -> Self {
        Self {
            ppo2: shadow.ppo2,
            fo2: shadow.fo2,
            depth: shadow.depth,
        }
    }
}

impl PPO2 {
    /// The computed ppO₂ as a [`Bar`] value.
    ///
    /// ```no_run
    /// use dps_gas::EANx;
    /// use dps_units::{Bar, Meters, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.ppo2_at(Meters::new(33.75)).pressure(), Bar::new(1.4));
    /// ```
    #[must_use]
    pub const fn pressure(self) -> Bar {
        self.ppo2
    }

    /// The O₂ fraction of the gas that produced this `Ppo2`.
    ///
    /// ```no_run
    /// use dps_gas::EANx;
    /// use dps_units::{Meters, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.ppo2_at(Meters::new(30.0)).fo2(), Percent::new(0.32).unwrap());
    /// ```
    #[must_use]
    pub const fn fo2(self) -> Percent {
        self.fo2
    }

    /// The depth at which this ppO₂ was evaluated.
    ///
    /// ```no_run
    /// use dps_gas::EANx;
    /// use dps_units::{Meters, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.ppo2_at(Meters::new(30.0)).depth(), Meters::new(30.0));
    /// ```
    #[must_use]
    pub const fn depth(self) -> Meters {
        self.depth
    }

    /// Full-detail formatter: `{gas name}  ppO₂ {ppo2}  @ {depth}`.
    ///
    /// ```no_run
    /// use dps_gas::EANx;
    /// use dps_units::{Meters, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(
    ///     ean32.ppo2_at(Meters::new(33.75)).summary().to_string(),
    ///     "EANx 32  ppO₂ 1.4 bar  @ 33.8 m",
    /// );
    /// ```
    #[must_use]
    pub const fn summary(self) -> Ppo2Summary {
        Ppo2Summary(self)
    }

    pub(super) fn new(fo2: Percent, depth: Meters, env: DiveEnvironment) -> Self {
        let ppo2 = env.absolute_pressure(depth) * fo2;
        Self { ppo2, fo2, depth }
    }
}

impl fmt::Display for PPO2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ppo2.fmt(f)
    }
}

impl From<PPO2> for Bar {
    /// ```no_run
    /// use dps_gas::EANx;
    /// use dps_units::{Bar, Meters, Percent};
    /// let p = EANx::try_from(Percent::new(0.32).unwrap()).unwrap().ppo2_at(Meters::new(30.0));
    /// assert_eq!(Bar::from(p), p.pressure());
    /// ```
    fn from(p: PPO2) -> Self {
        p.ppo2
    }
}

/// Full-detail display: `{gas name}  ppO₂ {ppo2}  @ {depth}`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Ppo2Summary(PPO2);

impl Ppo2Summary {
    /// Unwraps the inner [`PPO2`].
    #[must_use]
    pub const fn into_inner(self) -> PPO2 {
        self.0
    }
}

impl From<PPO2> for Ppo2Summary {
    fn from(p: PPO2) -> Self {
        Self(p)
    }
}

impl fmt::Display for Ppo2Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}  ppO\u{2082} {}  @ {}",
            gas_name(self.0.fo2),
            self.0,
            self.0.depth
        )
    }
}

impl approx::AbsDiffEq for PPO2 {
    type Epsilon = f64;

    fn default_epsilon() -> f64 {
        f64::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f64) -> bool {
        self.ppo2.abs_diff_eq(&other.ppo2, epsilon)
    }
}

impl approx::RelativeEq for PPO2 {
    fn default_max_relative() -> f64 {
        f64::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: f64, max_relative: f64) -> bool {
        self.ppo2.relative_eq(&other.ppo2, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{EANx, eanx::InvalidEANxError};

    use dps_environment::DiveEnvironment;
    use dps_units::{Bar, Meters, Percent};

    use approx::assert_relative_eq;

    fn ean(fraction: f64) -> Result<EANx, InvalidEANxError> {
        let pct = Percent::new(fraction)?;
        EANx::try_from(pct)
    }

    mod ppo2 {
        use super::*;

        #[test]
        fn display_shows_ppo2_bar_value() -> Result<(), InvalidEANxError> {
            // EANx 32 at 33.75 m: (33.75/9.948 + 1.013) × 0.32 ≈ 1.410 bar → displays as "1.4 bar"
            let p = ean(0.32)?.ppo2_at(Meters::new(33.75));

            assert_eq!(p.to_string(), "1.4 bar");

            Ok(())
        }

        #[test]
        fn ppo2_accessor_returns_bar() -> Result<(), InvalidEANxError> {
            // Use the MOD depth so ppO₂ = 1.4 bar exactly by construction
            let env = DiveEnvironment::standard();
            let fo2 = Percent::new(0.32)?;
            let ppo2_target = Bar::new(1.4);
            let mod_depth = env.depth(ppo2_target / fo2);
            let p = ean(0.32)?.ppo2_at(mod_depth);

            assert_relative_eq!(p.pressure(), ppo2_target, epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn fo2_is_preserved() -> Result<(), InvalidEANxError> {
            let fo2 = Percent::new(0.32)?;
            let p = ean(0.32)?.ppo2_at(Meters::new(30.0));

            assert_eq!(p.fo2(), fo2);

            Ok(())
        }

        #[test]
        fn depth_is_preserved() -> Result<(), InvalidEANxError> {
            let depth = Meters::new(30.0);
            let p = ean(0.32)?.ppo2_at(depth);

            assert_eq!(p.depth(), depth);

            Ok(())
        }

        #[test]
        fn from_gives_bar() -> Result<(), InvalidEANxError> {
            let p = ean(0.32)?.ppo2_at(Meters::new(30.0));

            assert_eq!(Bar::from(p), p.pressure());

            Ok(())
        }

        #[test]
        fn ppo2_at_surface_equals_surface_pressure_times_fo2() -> Result<(), InvalidEANxError> {
            let env = DiveEnvironment::standard();
            let fo2 = Percent::new(0.32)?;

            assert_relative_eq!(
                ean(0.32)?.ppo2_at(Meters::new(0.0)).pressure(),
                env.surface_pressure() * fo2
            );

            Ok(())
        }

        #[test]
        fn ppo2_at_air_30m() -> Result<(), InvalidEANxError> {
            let env = DiveEnvironment::standard();
            let expected = env.absolute_pressure(Meters::new(30.0)) * Percent::new(0.21)?;

            assert_relative_eq!(ean(0.21)?.ppo2_at(Meters::new(30.0)).pressure(), expected);

            Ok(())
        }

        #[test]
        fn ppo2_at_eanx40_10m() -> Result<(), InvalidEANxError> {
            let env = DiveEnvironment::standard();
            let expected = env.absolute_pressure(Meters::new(10.0)) * Percent::new(0.40)?;

            assert_relative_eq!(ean(0.40)?.ppo2_at(Meters::new(10.0)).pressure(), expected);

            Ok(())
        }
    }

    mod ppo2_summary {
        use super::*;

        #[test]
        fn summary_formats_full_detail() -> Result<(), InvalidEANxError> {
            // EANx 32 at 33.75 m → ppO₂ = 1.4 bar; depth displays as "33.8 m"
            let p = ean(0.32)?.ppo2_at(Meters::new(33.75));

            assert_eq!(p.summary().to_string(), "EANx 32  ppO₂ 1.4 bar  @ 33.8 m");

            Ok(())
        }

        #[test]
        fn into_inner_recovers_ppo2() -> Result<(), InvalidEANxError> {
            let p = ean(0.32)?.ppo2_at(Meters::new(30.0));

            assert_eq!(p.summary().into_inner(), p);

            Ok(())
        }

        #[test]
        fn from_impl_matches_summary_method() -> Result<(), InvalidEANxError> {
            let p = ean(0.32)?.ppo2_at(Meters::new(30.0));

            assert_eq!(Ppo2Summary::from(p).to_string(), p.summary().to_string());

            Ok(())
        }
    }
}
