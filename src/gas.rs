//! Gas mix types and named presets for dive planning.

use std::fmt;

use color_eyre::Result;

use crate::errors::InvalidO2Percent;
use crate::units::{Bar, Meters, MetersPerBar, Percent};

const SURFACE_PRESSURE: Bar = Bar::new(1.0);
const SEAWATER: MetersPerBar = MetersPerBar::new(10.0);
const EAN_MIN_O2: f64 = 0.10;

/// Maximum Operating Depth for a gas mix at a ppO₂ limit.
///
/// Produced exclusively by [`Ean::mod_at`].
///
/// ```no_run
/// use dps::gas::Ean;
/// use dps::units::{Bar, Percent};
/// let ean32 = Ean::try_from(Percent::new(0.32).unwrap()).unwrap();
/// let m = ean32.mod_at(Bar::new(1.4));
/// assert_eq!(m.to_string(), "33.8 m");
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mod {
    depth: Meters,
    gas: Ean,
    ppo2_max: Bar,
}

impl Mod {
    /// The computed depth.
    ///
    /// ```no_run
    /// # use approx::assert_relative_eq;
    /// use dps::gas::Ean;
    /// use dps::units::{Bar, Meters, Percent};
    /// let ean32 = Ean::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_relative_eq!(ean32.mod_at(Bar::new(1.4)).depth(), Meters::new(33.75), epsilon = 1e-6);
    /// ```
    #[must_use]
    pub const fn depth(self) -> Meters {
        self.depth
    }

    /// The gas mix that produced this MOD.
    ///
    /// ```no_run
    /// use dps::gas::Ean;
    /// use dps::units::{Bar, Percent};
    /// let ean32 = Ean::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.mod_at(Bar::new(1.4)).gas(), ean32);
    /// ```
    #[must_use]
    pub const fn gas(self) -> Ean {
        self.gas
    }

    /// The ppO₂ limit used to compute this MOD.
    ///
    /// ```no_run
    /// use dps::gas::Ean;
    /// use dps::units::{Bar, Percent};
    /// let ean32 = Ean::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(ean32.mod_at(Bar::new(1.4)).ppo2_max(), Bar::new(1.4));
    /// ```
    #[must_use]
    pub const fn ppo2_max(self) -> Bar {
        self.ppo2_max
    }

    /// Full-detail formatter: `{gas}  MOD {depth}  @ ppO₂ {ppo2_max}`.
    ///
    /// ```no_run
    /// use dps::gas::Ean;
    /// use dps::units::{Bar, Percent};
    /// let ean32 = Ean::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_eq!(
    ///     ean32.mod_at(Bar::new(1.4)).summary().to_string(),
    ///     "EANx 32  MOD 33.8 m  @ ppO₂ 1.4 bar",
    /// );
    /// ```
    #[must_use]
    pub const fn summary(self) -> ModSummary {
        ModSummary(self)
    }

    /// Constructs a [`Mod`] for testing without going through [`Ean::mod_at`].
    #[cfg(test)]
    #[must_use]
    pub fn new(gas: Ean, ppo2_max: Bar) -> Self {
        Self::from((gas, ppo2_max))
    }
}

impl From<(Ean, Bar)> for Mod {
    fn from((gas, ppo2_max): (Ean, Bar)) -> Self {
        let gauge = ppo2_max / gas.fraction - SURFACE_PRESSURE;
        let depth = (gauge * SEAWATER).max(Meters::new(0.0));
        Self {
            depth,
            gas,
            ppo2_max,
        }
    }
}

impl fmt::Display for Mod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.depth.fmt(f)
    }
}

impl From<Mod> for Meters {
    /// ```no_run
    /// use dps::gas::Ean;
    /// use dps::units::{Bar, Meters, Percent};
    /// let ean32 = Ean::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// let m = ean32.mod_at(Bar::new(1.4));
    /// assert_eq!(Meters::from(m), m.depth());
    /// ```
    fn from(m: Mod) -> Self {
        m.depth
    }
}

/// Full-detail display: `{gas}  MOD {depth}  @ ppO₂ {ppo2_max}`.
#[derive(Debug, Clone, Copy)]
pub struct ModSummary(Mod);

impl fmt::Display for ModSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}  MOD {}  @ ppO\u{2082} {}",
            self.0.gas, self.0, self.0.ppo2_max
        )
    }
}

#[cfg(test)]
impl approx::AbsDiffEq for Mod {
    type Epsilon = f64;

    fn default_epsilon() -> f64 {
        f64::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f64) -> bool {
        self.depth.abs_diff_eq(&other.depth, epsilon)
    }
}

#[cfg(test)]
impl approx::RelativeEq for Mod {
    fn default_max_relative() -> f64 {
        f64::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: f64, max_relative: f64) -> bool {
        self.depth.relative_eq(&other.depth, epsilon, max_relative)
    }
}

/// Enriched Air Nitrox: modelled by oxygen fraction in [0.10, 1.0].
///
/// The remainder (1 − FO₂) is treated as inert diluent. In practice this is
/// mostly N₂ with ~1 % Ar and trace CO₂, none of which affect MOD calculations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ean {
    fraction: Percent,
}

impl Ean {
    /// Returns the O₂ fraction as a `Percent` in [0.10, 1.0].
    ///
    /// ```no_run
    /// # use approx::assert_relative_eq;
    /// use dps::gas::Ean;
    /// use dps::units::Percent;
    /// let ean32 = Ean::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_relative_eq!(ean32.fo2().value(), 0.32);
    /// ```
    #[must_use]
    pub const fn fo2(self) -> Percent {
        self.fraction
    }

    /// Maximum Operating Depth for a given ppO₂ limit.
    ///
    /// Formula: MOD = (`ppO₂_max` / FO₂ − 1 atm) × 10 m/bar  (seawater approximation)
    ///
    /// ```no_run
    /// # use approx::assert_relative_eq;
    /// use dps::gas::Ean;
    /// use dps::units::{Bar, Meters, Percent};
    /// let ean32 = Ean::try_from(Percent::new(0.32).unwrap()).unwrap();
    /// assert_relative_eq!(ean32.mod_at(Bar::new(1.4)).depth(), Meters::new(33.75), epsilon = 1e-6);
    ///
    /// // Clamps to 0.0 when the ppO₂ limit is below the surface partial pressure.
    /// let o2 = Ean::try_from(Percent::new(1.0).unwrap()).unwrap();
    /// assert_relative_eq!(o2.mod_at(Bar::new(0.5)).depth(), Meters::new(0.0), epsilon = 1e-9);
    /// ```
    #[must_use]
    pub fn mod_at(self, ppo2_max: Bar) -> Mod {
        Mod::from((self, ppo2_max))
    }

    /// ppO₂ for this mix at the given depth.
    ///
    /// ```no_run
    /// # use approx::assert_relative_eq;
    /// use dps::gas::Ean;
    /// use dps::units::{Meters, Percent};
    /// let air = Ean::try_from(Percent::new(0.21).unwrap()).unwrap();
    /// // Air at 30 m: (30/10 + 1) × 0.21 = 0.84 bar
    /// assert_relative_eq!(air.ppo2_at(Meters::new(30.0)).value(), 0.84, epsilon = 1e-9);
    /// ```
    #[must_use]
    pub fn ppo2_at(self, depth: Meters) -> Bar {
        (depth / SEAWATER + SURFACE_PRESSURE) * self.fraction
    }
}

impl fmt::Display for Ean {
    /// Named mixes display their label; unnamed mixes display their O₂ fraction.
    ///
    /// ```no_run
    /// use dps::gas::Ean;
    /// use dps::units::Percent;
    /// let try_ean = |f| Ean::try_from(Percent::new(f).unwrap()).unwrap();
    /// assert_eq!(try_ean(0.21).to_string(), "Air");
    /// assert_eq!(try_ean(0.32).to_string(), "EANx 32");
    /// assert_eq!(try_ean(0.25).to_string(), "25 %");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ceil(x - 0.5) floors half-integers (21.5 → 21), unlike f64::round which rounds them up.
        let pct = self.fraction.value().mul_add(100.0, -0.5).ceil() as u8;
        let name = match pct {
            10 => "Hypoxic 10",
            12 => "Hypoxic 12",
            14 => "Hypoxic 14",
            16 => "Hypoxic 16",
            18 => "Hypoxic 18",
            21 => "Air",
            28 => "EANx 28",
            30 => "EANx 30",
            32 => "EANx 32",
            36 => "EANx 36",
            40 => "EANx 40",
            50 => "O₂ 50%",
            80 => "O₂ 80%",
            100 => "Pure O₂",
            _ => return write!(f, "{}", self.fraction),
        };

        write!(f, "{name}")
    }
}

/// Constructs an [`Ean`] from an oxygen fraction.
///
/// # Errors
///
/// Returns [`InvalidO2Percent`] if `pct` is below 10 % (i.e. below 0.10). The upper bound of
/// 100 % is already enforced by [`Percent`] itself.
///
/// ```no_run
/// use dps::gas::Ean;
/// use dps::units::Percent;
/// assert!(Ean::try_from(Percent::new(0.32).unwrap()).is_ok());
/// assert!(Ean::try_from(Percent::new(0.10).unwrap()).is_ok()); // minimum
/// assert!(Ean::try_from(Percent::new(0.09).unwrap()).is_err()); // below minimum
/// ```
impl TryFrom<Percent> for Ean {
    type Error = InvalidO2Percent;

    fn try_from(pct: Percent) -> Result<Self, Self::Error> {
        if pct.value() < EAN_MIN_O2 {
            return Err(InvalidO2Percent(pct));
        }

        Ok(Self { fraction: pct })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{Bar, Meters, Percent};
    use approx::assert_relative_eq;
    use color_eyre::{Result, eyre::eyre};
    use rstest::*;

    fn ean(fraction: f64) -> Result<Ean> {
        let pct =
            Percent::new(fraction).ok_or_else(|| eyre!("fraction {fraction} out of [0.0, 1.0]"))?;
        Ok(Ean::try_from(pct)?)
    }

    mod fo2 {
        use super::*;

        #[rstest]
        fn fo2_matches_fraction() -> Result<()> {
            assert_relative_eq!(
                ean(0.21)?.fo2(),
                Percent::new(0.21).ok_or_else(|| eyre!("invalid"))?
            );
            assert_relative_eq!(
                ean(0.32)?.fo2(),
                Percent::new(0.32).ok_or_else(|| eyre!("invalid"))?
            );
            assert_relative_eq!(
                ean(1.0)?.fo2(),
                Percent::new(1.0).ok_or_else(|| eyre!("invalid"))?
            );

            Ok(())
        }
    }

    mod mod_at {
        use super::*;

        // Formula: MOD = (ppO2_max / FO2 − 1 bar) × 10 m/bar

        #[test]
        fn mod_at_eanx32_1_4_bar() -> Result<()> {
            let expected = Meters::new((1.4_f64 / 0.32 - 1.0) * 10.0); // ≈ 33.75 m

            assert_relative_eq!(ean(0.32)?.mod_at(Bar::new(1.4)).depth(), expected);

            Ok(())
        }

        #[test]
        fn mod_at_eanx40_1_4_bar() -> Result<()> {
            let expected = Meters::new((1.4_f64 / 0.40 - 1.0) * 10.0); // 25.0 m

            assert_relative_eq!(ean(0.40)?.mod_at(Bar::new(1.4)).depth(), expected);

            Ok(())
        }

        #[test]
        fn mod_at_pure_o2_1_6_bar() -> Result<()> {
            let expected = Meters::new((1.6_f64 / 1.0 - 1.0) * 10.0); // 6.0 m

            assert_relative_eq!(ean(1.0)?.mod_at(Bar::new(1.6)).depth(), expected);

            Ok(())
        }

        #[test]
        fn mod_at_clamps_to_zero_when_negative() -> Result<()> {
            // Pure O2 at 0.5 bar ppO2 limit: (0.5/1.0 - 1) * 10 = -5.0 m → 0.0
            assert_relative_eq!(
                ean(1.0)?.mod_at(Bar::new(0.5)).depth(),
                Meters::new(0.0),
                epsilon = 1e-9
            );

            Ok(())
        }

        #[test]
        fn gas_is_preserved() -> Result<()> {
            let gas = ean(0.32)?;
            assert_eq!(gas.mod_at(Bar::new(1.4)).gas(), gas);

            Ok(())
        }

        #[test]
        fn ppo2_max_is_preserved() -> Result<()> {
            let ppo2 = Bar::new(1.6);
            assert_eq!(ean(0.32)?.mod_at(ppo2).ppo2_max(), ppo2);

            Ok(())
        }
    }

    mod ppo2_at {
        use super::*;

        // Formula: ppO2 = (depth_m / 10 + 1) × FO2

        #[test]
        fn ppo2_at_surface_equals_fo2() -> Result<()> {
            assert_relative_eq!(ean(0.32)?.ppo2_at(Meters::new(0.0)), Bar::new(0.32));

            Ok(())
        }

        #[test]
        fn ppo2_at_air_30m() -> Result<()> {
            let expected = Bar::new((30.0_f64 / 10.0 + 1.0) * 0.21); // 0.84 bar

            assert_relative_eq!(ean(0.21)?.ppo2_at(Meters::new(30.0)), expected);

            Ok(())
        }

        #[test]
        fn ppo2_at_eanx40_10m() -> Result<()> {
            let expected = Bar::new((10.0_f64 / 10.0 + 1.0) * 0.40); // 0.80 bar

            assert_relative_eq!(ean(0.40)?.ppo2_at(Meters::new(10.0)), expected);

            Ok(())
        }
    }

    mod display {
        use super::*;

        #[test]
        fn display_air() -> Result<()> {
            assert_eq!(ean(0.21)?.to_string(), "Air");
            // 21.5 % floors to 21 (half-integers floor, not round-up), so still Air
            assert_eq!(ean(0.215)?.to_string(), "Air");
            // 20.5 % floors to 20, no longer Air
            assert_ne!(ean(0.205)?.to_string(), "Air");

            Ok(())
        }

        #[test]
        fn display_named_nitrox_mixes() -> Result<()> {
            assert_eq!(ean(0.28)?.to_string(), "EANx 28");
            assert_eq!(ean(0.30)?.to_string(), "EANx 30");
            assert_eq!(ean(0.32)?.to_string(), "EANx 32");
            assert_eq!(ean(0.36)?.to_string(), "EANx 36");
            assert_eq!(ean(0.40)?.to_string(), "EANx 40");

            Ok(())
        }

        #[test]
        fn display_high_o2_mixes() -> Result<()> {
            assert_eq!(ean(0.50)?.to_string(), "O₂ 50%");
            assert_eq!(ean(0.80)?.to_string(), "O₂ 80%");

            Ok(())
        }

        #[test]
        fn display_hypoxic_mixes() -> Result<()> {
            assert_eq!(ean(0.10)?.to_string(), "Hypoxic 10");
            assert_eq!(ean(0.12)?.to_string(), "Hypoxic 12");
            assert_eq!(ean(0.14)?.to_string(), "Hypoxic 14");
            assert_eq!(ean(0.16)?.to_string(), "Hypoxic 16");
            assert_eq!(ean(0.18)?.to_string(), "Hypoxic 18");

            Ok(())
        }

        #[test]
        fn display_pure_o2() -> Result<()> {
            assert_eq!(ean(1.0)?.to_string(), "Pure O₂");

            Ok(())
        }

        #[test]
        fn display_unnamed_mix_shows_fraction() -> Result<()> {
            assert_eq!(ean(0.25)?.to_string(), "25 %");
            assert_eq!(ean(0.33)?.to_string(), "33 %");

            Ok(())
        }
    }

    mod r#mod {
        use super::*;

        #[test]
        fn display_shows_depth() -> Result<()> {
            assert_eq!(ean(0.32)?.mod_at(Bar::new(1.4)).to_string(), "33.8 m");

            Ok(())
        }

        #[test]
        fn into_meters_gives_depth() -> Result<()> {
            let m = ean(0.32)?.mod_at(Bar::new(1.4));
            assert_eq!(Meters::from(m), m.depth());

            Ok(())
        }
    }

    mod mod_summary {
        use super::*;

        #[test]
        fn summary_formats_full_detail() -> Result<()> {
            let m = ean(0.32)?.mod_at(Bar::new(1.4));
            assert_eq!(
                m.summary().to_string(),
                "EANx 32  MOD 33.8 m  @ ppO₂ 1.4 bar"
            );

            Ok(())
        }
    }

    mod invalid_o2_percent {
        use super::*;

        #[test]
        fn invalid_o2_percent_display() -> Result<()> {
            let msg = format!(
                "{}",
                InvalidO2Percent(Percent::new(0.5).ok_or_else(|| eyre!("invalid"))?)
            );

            assert!(msg.contains('5'));
            assert!(msg.contains("10"));
            assert!(msg.contains("100"));

            Ok(())
        }
    }

    mod try_from_percent {
        use super::*;
        use rstest::rstest;

        #[rstest]
        #[case(0.21)]
        #[case(0.32)]
        #[case(0.40)]
        #[case(1.0)]
        fn try_from_percent_preserves_fraction(#[case] fraction: f64) -> Result<()> {
            let pct = Percent::new(fraction)
                .ok_or_else(|| eyre!("fraction {fraction} out of [0.0, 1.0]"))?;

            assert_eq!(Ean::try_from(pct)?.fo2(), pct);

            Ok(())
        }

        #[test]
        fn try_from_percent_rejects_below_minimum() -> Result<()> {
            // 0.09 = 9 % is below the 10 % floor for Ean
            assert!(Ean::try_from(Percent::new(0.09).ok_or_else(|| eyre!("invalid"))?).is_err());

            Ok(())
        }

        #[test]
        fn try_from_percent_accepts_fraction_that_rounds_into_valid_range() -> Result<()> {
            // 0.316 × 100 = 31.6, rounds to 32 — accepted
            assert!(Ean::try_from(Percent::new(0.316).ok_or_else(|| eyre!("invalid"))?).is_ok());

            Ok(())
        }
    }
}
