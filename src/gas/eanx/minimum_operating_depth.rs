use std::fmt;

use crate::units::{Bar, Meters, Percent};

use super::error::InvalidEANx;
use super::gas_name;
use crate::gas::constants::{EAN_MIN_O2, SEAWATER, SURFACE_PRESSURE};

/// Minimum Operating Depth for a hypoxic gas mix at a ppO₂ floor.
///
/// Produced by [`EANxBlend::minimod_at`]. The blend method is erased at this
/// boundary because `MiniMOD` depends only on FO₂ and `ppO₂_min`.
///
/// ```no_run
/// use dps::gas::EANx;
/// use dps::units::{Bar, Percent};
/// let h10 = EANx::try_from(Percent::new(0.10).unwrap()).unwrap();
/// let m = h10.minimod_at(Bar::new(0.16));
/// assert_eq!(m.to_string(), "6.0 m");
/// assert_eq!(m.summary().to_string(), "Hypoxic 10  MiniMOD 6.0 m  @ ppO₂ 0.16 bar");
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MiniMOD {
    depth: Meters,
    fo2: Percent,
    ppo2_min: Bar,
}

impl MiniMOD {
    /// The minimum depth (0 m for normoxic mixes).
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Bar, Meters, Percent};
    /// # use approx::assert_relative_eq;
    /// let h10 = EANx::try_from(Percent::new(0.10).unwrap()).unwrap();
    /// assert_relative_eq!(h10.minimod_at(Bar::new(0.16)).depth(), Meters::new(6.0), epsilon = 1e-9);
    /// ```
    #[must_use]
    pub const fn depth(self) -> Meters {
        self.depth
    }

    /// The O₂ fraction of the gas that produced this `MiniMOD`.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Bar, Percent};
    /// let h10 = EANx::try_from(Percent::new(0.10).unwrap()).unwrap();
    /// assert_eq!(h10.minimod_at(Bar::new(0.16)).fo2(), Percent::new(0.10).unwrap());
    /// ```
    #[must_use]
    pub const fn fo2(self) -> Percent {
        self.fo2
    }

    /// The ppO₂ floor used to compute this `MiniMOD`.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Bar, Percent};
    /// let h10 = EANx::try_from(Percent::new(0.10).unwrap()).unwrap();
    /// assert_eq!(h10.minimod_at(Bar::new(0.16)).ppo2_min(), Bar::new(0.16));
    /// ```
    #[must_use]
    pub const fn ppo2_min(self) -> Bar {
        self.ppo2_min
    }

    /// Full-detail formatter: `{gas name}  MiniMOD {depth}  @ ppO₂ {ppo2_min}`.
    ///
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Bar, Percent};
    /// let h10 = EANx::try_from(Percent::new(0.10).unwrap()).unwrap();
    /// assert_eq!(
    ///     h10.minimod_at(Bar::new(0.16)).summary().to_string(),
    ///     "Hypoxic 10  MiniMOD 6.0 m  @ ppO₂ 0.16 bar",
    /// );
    /// ```
    #[must_use]
    pub const fn summary(self) -> MiniMODSummary {
        MiniMODSummary(self)
    }

    // Widened to `pub` under `#[cfg(test)]` so tests in sibling modules can
    // construct `MiniMOD` directly without going through `EANxBlend::minimod_at`.
    #[cfg(not(test))]
    pub(super) fn new(fo2: Percent, ppo2_min: Bar) -> Result<Self, InvalidEANx> {
        Self::new_inner(fo2, ppo2_min)
    }

    #[cfg(test)]
    #[expect(
        missing_docs,
        clippy::missing_errors_doc,
        reason = "test-only visibility widening"
    )]
    pub fn new(fo2: Percent, ppo2_min: Bar) -> Result<Self, InvalidEANx> {
        Self::new_inner(fo2, ppo2_min)
    }

    fn new_inner(fo2: Percent, ppo2_min: Bar) -> Result<Self, InvalidEANx> {
        if fo2 < EAN_MIN_O2 {
            return Err(InvalidEANx::O2TooLow(fo2));
        }

        let gauge = ppo2_min / fo2 - SURFACE_PRESSURE;
        let depth = gauge.max(Bar::new(0.0)) * SEAWATER;

        Ok(Self {
            depth,
            fo2,
            ppo2_min,
        })
    }
}

impl TryFrom<(Percent, Bar)> for MiniMOD {
    type Error = InvalidEANx;

    fn try_from((fo2, ppo2_min): (Percent, Bar)) -> Result<Self, Self::Error> {
        Self::new(fo2, ppo2_min)
    }
}

impl fmt::Display for MiniMOD {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.depth.fmt(f)
    }
}

impl From<MiniMOD> for Meters {
    /// ```no_run
    /// use dps::gas::EANx;
    /// use dps::units::{Bar, Meters, Percent};
    /// let m = EANx::try_from(Percent::new(0.10).unwrap()).unwrap().minimod_at(Bar::new(0.16));
    /// assert_eq!(Meters::from(m), m.depth());
    /// ```
    fn from(m: MiniMOD) -> Self {
        m.depth
    }
}

/// Full-detail display: `{gas name}  MiniMOD {depth}  @ ppO₂ {ppo2_min}`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MiniMODSummary(MiniMOD);

impl MiniMODSummary {
    /// Unwraps the inner [`MiniMOD`].
    #[must_use]
    pub const fn into_inner(self) -> MiniMOD {
        self.0
    }
}

impl From<MiniMOD> for MiniMODSummary {
    fn from(m: MiniMOD) -> Self {
        Self(m)
    }
}

impl fmt::Display for MiniMODSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}  MiniMOD {}  @ ppO\u{2082} {}",
            gas_name(self.0.fo2),
            self.0,
            self.0.ppo2_min
        )
    }
}

#[cfg(test)]
impl approx::AbsDiffEq for MiniMOD {
    type Epsilon = f64;

    fn default_epsilon() -> f64 {
        f64::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f64) -> bool {
        self.depth.abs_diff_eq(&other.depth, epsilon)
    }
}

#[cfg(test)]
impl approx::RelativeEq for MiniMOD {
    fn default_max_relative() -> f64 {
        f64::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: f64, max_relative: f64) -> bool {
        self.depth.relative_eq(&other.depth, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gas::InvalidEANx;
    use crate::units::{Bar, Percent};
    use color_eyre::Result;
    use color_eyre::eyre::eyre;

    mod minimum_operating_depth {
        use super::*;

        #[test]
        fn display_shows_depth() -> Result<()> {
            let m = MiniMOD::try_from((
                Percent::new(0.10).ok_or_else(|| eyre!("invalid"))?,
                Bar::new(0.16),
            ))?;

            assert_eq!(m.to_string(), "6.0 m");

            Ok(())
        }

        #[test]
        fn try_from_rejects_fo2_below_minimum() -> Result<()> {
            let fo2 = Percent::new(0.09).ok_or_else(|| eyre!("0.09 is in [0.0, 1.0]"))?;

            assert!(matches!(
                MiniMOD::try_from((fo2, Bar::new(0.16))),
                Err(InvalidEANx::O2TooLow(_))
            ));

            Ok(())
        }

        #[test]
        fn summary_formats_full_detail() -> Result<()> {
            // Bar displays with one decimal: 0.16 → "0.2 bar". Use 0.2 for a clean round-trip.
            // minimod depth = (0.2 / 0.10 − 1) × 10 = 10.0 m
            let m = MiniMOD::try_from((
                Percent::new(0.10).ok_or_else(|| eyre!("invalid"))?,
                Bar::new(0.2),
            ))?;

            assert_eq!(
                m.summary().to_string(),
                "Hypoxic 10  MiniMOD 10.0 m  @ ppO₂ 0.2 bar"
            );

            Ok(())
        }
    }

    mod mod_summary {
        use super::*;

        #[test]
        fn summary_formats_full_detail() -> Result<()> {
            let m = MiniMOD::try_from((
                Percent::new(0.32).ok_or_else(|| eyre!("invalid"))?,
                Bar::new(1.4),
            ))?;

            assert_eq!(
                m.summary().to_string(),
                "EANx 32  MiniMOD 33.8 m  @ ppO₂ 1.4 bar"
            );

            Ok(())
        }

        #[test]
        fn into_inner_recovers_original_minimod() -> Result<()> {
            let m = MiniMOD::try_from((
                Percent::new(0.10).ok_or_else(|| eyre!("invalid"))?,
                Bar::new(0.16),
            ))?;

            assert_eq!(m.summary().into_inner(), m);

            Ok(())
        }

        #[test]
        fn from_impl_matches_summary_method() -> Result<()> {
            let m = MiniMOD::try_from((
                Percent::new(0.10).ok_or_else(|| eyre!("invalid"))?,
                Bar::new(0.16),
            ))?;

            assert_eq!(MiniMODSummary::from(m).to_string(), m.summary().to_string());

            Ok(())
        }
    }
}
