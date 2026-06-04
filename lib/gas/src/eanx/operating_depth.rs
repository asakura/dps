use super::error::InvalidEANxError;
use super::gas_name;

use crate::constants::EAN_MIN_O2;

use dps_environment::DiveEnvironment;
use dps_units::{Bar, Meters, Percent};

use std::fmt;

/// Maximum Operating Depth for a gas mix at a $\text{pp}\ce{O2}$ limit.
///
/// Produced by [`EANxBlend::mod_at`](crate::prelude::EANxBlend::mod_at). The blend method is erased at this boundary
/// because MOD depends only on $\text{F}\ce{O2}$ and `ppo2_max`.
///
/// ```no_run
/// use dps_gas::prelude::EANx;
/// use dps_units::{Bar, Percent};
/// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
/// let m = ean32.mod_at(Bar::new(1.4));
/// assert_eq!(m.to_string(), "33.4 m");
/// assert_eq!(m.summary().to_string(), "EANx 32  MOD 33.4 m  @ ppO₂ 1.4 bar");
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "MODShadow"))]
pub struct MOD {
    depth: Meters,
    fo2: Percent,
    ppo2_max: Bar,
}

#[cfg(feature = "serde")]
#[derive(::serde::Deserialize)]
struct MODShadow {
    depth: Meters,
    fo2: Percent,
    ppo2_max: Bar,
}

#[cfg(feature = "serde")]
impl TryFrom<MODShadow> for MOD {
    type Error = String;

    fn try_from(shadow: MODShadow) -> Result<Self, Self::Error> {
        if shadow.fo2 < EAN_MIN_O2 {
            return Err(format!(
                "O₂ fraction {} is below the 10% minimum",
                shadow.fo2
            ));
        }

        Ok(Self {
            depth: shadow.depth,
            fo2: shadow.fo2,
            ppo2_max: shadow.ppo2_max,
        })
    }
}

impl MOD {
    /// The computed depth.
    ///
    /// ```no_run
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{Bar, Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_relative_eq!(ean32.mod_at(Bar::new(1.4)).depth(), Meters::new(33.44), epsilon = 0.01);
    /// ```
    #[must_use]
    pub const fn depth(self) -> Meters {
        self.depth
    }

    /// The $\ce{O2}$ fraction of the gas that produced this MOD.
    ///
    /// ```no_run
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{Bar, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.mod_at(Bar::new(1.4)).fo2(), Percent::new(0.32).unwrap());
    /// ```
    #[must_use]
    pub const fn fo2(self) -> Percent {
        self.fo2
    }

    /// The $\text{pp}\ce{O2}$ limit used to compute this MOD.
    ///
    /// ```no_run
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{Bar, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.mod_at(Bar::new(1.4)).ppo2_max(), Bar::new(1.4));
    /// ```
    #[must_use]
    pub const fn ppo2_max(self) -> Bar {
        self.ppo2_max
    }

    /// Full-detail formatter: `{gas name}  MOD {depth}  @ ppO₂ {ppo2_max}`.
    ///
    /// ```no_run
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{Bar, Percent};
    /// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(
    ///     ean32.mod_at(Bar::new(1.4)).summary().to_string(),
    ///     "EANx 32  MOD 33.4 m  @ ppO₂ 1.4 bar",
    /// );
    /// ```
    #[must_use]
    pub const fn summary(self) -> MODSummary {
        MODSummary(self)
    }

    // Widened to `pub` under `#[cfg(test)]` so tests in sibling modules can
    // construct `MOD` directly without going through `EANxBlend::mod_at`.
    #[cfg(not(test))]
    pub(super) fn new(
        fo2: Percent,
        ppo2_max: Bar,
        env: DiveEnvironment,
    ) -> Result<Self, InvalidEANxError> {
        Self::new_inner(fo2, ppo2_max, env)
    }

    #[cfg(test)]
    #[expect(
        missing_docs,
        clippy::missing_errors_doc,
        reason = "test-only visibility widening"
    )]
    pub fn new(
        fo2: Percent,
        ppo2_max: Bar,
        env: DiveEnvironment,
    ) -> Result<Self, InvalidEANxError> {
        Self::new_inner(fo2, ppo2_max, env)
    }

    fn new_inner(
        fo2: Percent,
        ppo2_max: Bar,
        env: DiveEnvironment,
    ) -> Result<Self, InvalidEANxError> {
        if fo2 < EAN_MIN_O2 {
            return Err(InvalidEANxError::O2TooLow(fo2));
        }

        let depth = env.depth(ppo2_max / fo2);

        Ok(Self {
            depth,
            fo2,
            ppo2_max,
        })
    }
}

impl TryFrom<(Percent, Bar)> for MOD {
    type Error = InvalidEANxError;

    fn try_from((fo2, ppo2_max): (Percent, Bar)) -> Result<Self, Self::Error> {
        Self::new(fo2, ppo2_max, DiveEnvironment::standard())
    }
}

impl fmt::Display for MOD {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.depth.fmt(f)
    }
}

impl From<MOD> for Meters {
    /// ```no_run
    /// use dps_gas::prelude::EANx;
    /// use dps_units::{Bar, Meters, Percent};
    /// let m = EANx::try_from(Percent::new(0.32).unwrap()).unwrap().mod_at(Bar::new(1.4));
    /// assert_eq!(Meters::from(m), m.depth());
    /// ```
    fn from(m: MOD) -> Self {
        m.depth
    }
}

impl approx::AbsDiffEq for MOD {
    type Epsilon = f64;

    fn default_epsilon() -> f64 {
        f64::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f64) -> bool {
        self.depth.abs_diff_eq(&other.depth, epsilon)
    }
}

impl approx::RelativeEq for MOD {
    fn default_max_relative() -> f64 {
        f64::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: f64, max_relative: f64) -> bool {
        self.depth.relative_eq(&other.depth, epsilon, max_relative)
    }
}

/// Full-detail display: `{gas name}  MOD {depth}  @ ppO₂ {ppo2_max}`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct MODSummary(MOD);

impl MODSummary {
    /// Unwraps the inner [`MOD`].
    #[must_use]
    pub const fn into_inner(self) -> MOD {
        self.0
    }
}

impl From<MOD> for MODSummary {
    fn from(m: MOD) -> Self {
        Self(m)
    }
}

impl fmt::Display for MODSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}  MOD {}  @ ppO\u{2082} {}",
            gas_name(self.0.fo2),
            self.0,
            self.0.ppo2_max
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::eanx::InvalidEANxError;

    use dps_units::{Bar, Meters, Percent};

    use core::assert_matches;

    mod operating_depth {
        use super::*;

        #[test]
        fn display_shows_depth() -> Result<(), InvalidEANxError> {
            let m = MOD::try_from((Percent::new(0.32)?, Bar::new(1.4)))?;

            assert_eq!(m.to_string(), "33.4 m");

            Ok(())
        }

        #[test]
        fn into_meters_gives_depth() -> Result<(), InvalidEANxError> {
            let m = MOD::try_from((Percent::new(0.32)?, Bar::new(1.4)))?;
            assert_eq!(Meters::from(m), m.depth());

            Ok(())
        }

        #[test]
        fn try_from_rejects_fo2_below_minimum() -> Result<(), InvalidEANxError> {
            let fo2 = Percent::new(0.09)?;

            assert_matches!(
                MOD::try_from((fo2, Bar::new(1.4))),
                Err(InvalidEANxError::O2TooLow(_))
            );

            Ok(())
        }
    }

    mod mod_summary {
        use super::*;

        #[test]
        fn summary_formats_full_detail() -> Result<(), InvalidEANxError> {
            let m = MOD::try_from((Percent::new(0.32)?, Bar::new(1.4)))?;

            assert_eq!(
                m.summary().to_string(),
                "EANx 32  MOD 33.4 m  @ ppO₂ 1.4 bar"
            );

            Ok(())
        }

        #[test]
        fn into_inner_recovers_original_mod() -> Result<(), InvalidEANxError> {
            let m = MOD::try_from((Percent::new(0.32)?, Bar::new(1.4)))?;

            assert_eq!(m.summary().into_inner(), m);

            Ok(())
        }

        #[test]
        fn from_impl_matches_summary_method() -> Result<(), InvalidEANxError> {
            let m = MOD::try_from((Percent::new(0.32)?, Bar::new(1.4)))?;

            assert_eq!(MODSummary::from(m).to_string(), m.summary().to_string());

            Ok(())
        }
    }
}
