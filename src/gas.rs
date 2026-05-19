//! Gas mix types and named presets for dive planning.

use color_eyre::Result;

use crate::errors::InvalidO2Percent;
use crate::units::{Bar, Meters, MetersPerBar};

const SURFACE_PRESSURE: Bar = Bar::new(1.0);
const SEAWATER: MetersPerBar = MetersPerBar::new(10.0);

/// Enriched Air Nitrox: modelled by oxygen fraction in [0.10, 1.0].
///
/// The remainder (1 − FO₂) is treated as inert diluent. In practice this is
/// mostly N₂ with ~1 % Ar and trace CO₂, none of which affect MOD calculations.
#[derive(Debug, Clone, Copy)]
pub struct Ean {
    fraction: f64,
}

impl Ean {
    /// Construct from whole-percent O₂ value (10–100).
    ///
    /// # Errors
    ///
    /// Returns `Err` if `o2_pct` is outside `[10, 100]`.
    ///
    /// ```no_run
    /// use dps::gas::Ean;
    /// assert!(Ean::from_percent(32).is_ok());
    /// assert!(Ean::from_percent(21).is_ok()); // air
    /// assert!(Ean::from_percent(9).is_err());
    /// assert!(Ean::from_percent(101).is_err());
    /// ```
    pub fn from_percent(o2_pct: u8) -> Result<Self, InvalidO2Percent> {
        if !(10..=100).contains(&o2_pct) {
            return Err(InvalidO2Percent(o2_pct));
        }

        Ok(Self {
            fraction: f64::from(o2_pct) / 100.0,
        })
    }

    /// Returns the O₂ fraction as a whole percentage (10–100).
    ///
    /// ```no_run
    /// use dps::gas::Ean;
    /// assert_eq!(Ean::from_percent(32).unwrap().o2_percent(), 32);
    /// ```
    #[must_use]
    pub fn o2_percent(self) -> u8 {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "fraction is in [0.10, 1.0] so rounding gives a value in [10, 100] which fits u8"
        )]
        let pct = (self.fraction * 100.0).round() as u8;
        pct
    }

    /// Returns the O₂ fraction as a decimal in [0.10, 1.0].
    ///
    /// ```no_run
    /// # use approx::assert_relative_eq;
    /// use dps::gas::Ean;
    /// assert_relative_eq!(Ean::from_percent(32).unwrap().fo2(), 0.32);
    /// ```
    #[must_use]
    pub const fn fo2(self) -> f64 {
        self.fraction
    }

    /// Maximum Operating Depth for a given ppO₂ limit.
    ///
    /// Formula: MOD = (`ppO₂_max` / FO₂ − 1 atm) × 10 m/bar  (seawater approximation)
    ///
    /// ```no_run
    /// # use approx::assert_relative_eq;
    /// use dps::gas::Ean;
    /// use dps::units::Bar;
    /// let ean32 = Ean::from_percent(32).unwrap();
    /// assert_relative_eq!(ean32.mod_at(Bar::new(1.4)).value(), 33.75, epsilon = 1e-6);
    ///
    /// // Clamps to 0.0 when the ppO₂ limit is below the surface partial pressure.
    /// let o2 = Ean::from_percent(100).unwrap();
    /// assert_relative_eq!(o2.mod_at(Bar::new(0.5)).value(), 0.0, epsilon = 1e-9);
    /// ```
    #[must_use]
    pub fn mod_at(self, ppo2_max: Bar) -> Meters {
        let gauge = ppo2_max / self.fo2() - SURFACE_PRESSURE;
        (gauge * SEAWATER).max(Meters::new(0.0))
    }

    /// ppO₂ for this mix at the given depth.
    ///
    /// ```no_run
    /// # use approx::assert_relative_eq;
    /// use dps::gas::Ean;
    /// use dps::units::Meters;
    /// let air = Ean::from_percent(21).unwrap();
    /// // Air at 30 m: (30/10 + 1) × 0.21 = 0.84 bar
    /// assert_relative_eq!(air.ppo2_at(Meters::new(30.0)).value(), 0.84, epsilon = 1e-9);
    /// ```
    #[must_use]
    pub fn ppo2_at(self, depth: Meters) -> Bar {
        (depth / SEAWATER + SURFACE_PRESSURE) * self.fraction
    }

    /// Named label for this mix, if one exists.
    ///
    /// ```no_run
    /// use dps::gas::Ean;
    /// assert_eq!(Ean::from_percent(21).unwrap().label(), Some("Air"));
    /// assert_eq!(Ean::from_percent(32).unwrap().label(), Some("EANx 32"));
    /// assert_eq!(Ean::from_percent(25).unwrap().label(), None);
    /// ```
    #[must_use]
    pub fn label(self) -> Option<&'static str> {
        match self.o2_percent() {
            10 => Some("Hypoxic 10"),
            12 => Some("Hypoxic 12"),
            14 => Some("Hypoxic 14"),
            16 => Some("Hypoxic 16"),
            18 => Some("Hypoxic 18"),
            21 => Some("Air"),
            28 => Some("EANx 28"),
            30 => Some("EANx 30"),
            32 => Some("EANx 32"),
            36 => Some("EANx 36"),
            40 => Some("EANx 40"),
            50 => Some("O₂ 50%"),
            80 => Some("O₂ 80%"),
            100 => Some("Pure O₂"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{Bar, Meters};
    use approx::assert_relative_eq;

    mod from_percent {
        use super::*;

        #[test]
        fn from_percent_accepts_boundary_values() {
            assert!(Ean::from_percent(10).is_ok());
            assert!(Ean::from_percent(100).is_ok());
        }

        #[test]
        fn from_percent_accepts_typical_mixes() {
            assert!(Ean::from_percent(21).is_ok());
            assert!(Ean::from_percent(32).is_ok());
            assert!(Ean::from_percent(40).is_ok());
        }

        #[test]
        fn from_percent_rejects_zero() {
            assert!(Ean::from_percent(0).is_err());
        }

        #[test]
        fn from_percent_rejects_below_minimum() {
            assert!(Ean::from_percent(9).is_err());
        }

        #[test]
        fn from_percent_rejects_above_maximum() {
            assert!(Ean::from_percent(101).is_err());
        }

        #[test]
        fn from_percent_error_carries_the_bad_value() {
            let result = Ean::from_percent(5);

            assert!(matches!(result, Err(InvalidO2Percent(5))));
        }
    }

    mod o2_percent {
        use super::*;
        use rstest::rstest;

        #[rstest]
        #[case(10)]
        #[case(21)]
        #[case(32)]
        #[case(36)]
        #[case(40)]
        #[case(80)]
        #[case(100)]
        fn o2_percent_roundtrips(#[case] pct: u8) -> Result<()> {
            assert_eq!(Ean::from_percent(pct)?.o2_percent(), pct);

            Ok(())
        }
    }

    mod fo2 {
        use super::*;

        #[test]
        fn fo2_matches_fraction() -> Result<()> {
            assert_relative_eq!(Ean::from_percent(21)?.fo2(), 0.21);
            assert_relative_eq!(Ean::from_percent(32)?.fo2(), 0.32);
            assert_relative_eq!(Ean::from_percent(100)?.fo2(), 1.0);

            Ok(())
        }
    }

    mod mod_at {
        use super::*;

        // Formula: MOD = (ppO2_max / FO2 − 1 bar) × 10 m/bar

        #[test]
        fn mod_at_eanx32_1_4_bar() -> Result<()> {
            let ean32 = Ean::from_percent(32)?;
            let expected = (1.4_f64 / 0.32 - 1.0) * 10.0; // ≈ 33.75 m

            assert_relative_eq!(ean32.mod_at(Bar::new(1.4)).value(), expected);

            Ok(())
        }

        #[test]
        fn mod_at_eanx40_1_4_bar() -> Result<()> {
            let ean40 = Ean::from_percent(40)?;
            let expected = (1.4_f64 / 0.40 - 1.0) * 10.0; // 25.0 m

            assert_relative_eq!(ean40.mod_at(Bar::new(1.4)).value(), expected);

            Ok(())
        }

        #[test]
        fn mod_at_pure_o2_1_6_bar() -> Result<()> {
            let o2 = Ean::from_percent(100)?;
            let expected = (1.6_f64 / 1.0 - 1.0) * 10.0; // 6.0 m

            assert_relative_eq!(o2.mod_at(Bar::new(1.6)).value(), expected);

            Ok(())
        }

        #[test]
        fn mod_at_clamps_to_zero_when_negative() -> Result<()> {
            // Pure O2 at 0.5 bar ppO2 limit: (0.5/1.0 - 1) * 10 = -5.0 m → 0.0
            let o2 = Ean::from_percent(100)?;

            assert_relative_eq!(o2.mod_at(Bar::new(0.5)).value(), 0.0, epsilon = 1e-9);

            Ok(())
        }
    }

    mod ppo2_at {
        use super::*;

        // Formula: ppO2 = (depth_m / 10 + 1) × FO2

        #[test]
        fn ppo2_at_surface_equals_fo2() -> Result<()> {
            let ean32 = Ean::from_percent(32)?;

            assert_relative_eq!(ean32.ppo2_at(Meters::new(0.0)).value(), 0.32);

            Ok(())
        }

        #[test]
        fn ppo2_at_air_30m() -> Result<()> {
            let air = Ean::from_percent(21)?;
            let expected = (30.0_f64 / 10.0 + 1.0) * 0.21; // 0.84 bar

            assert_relative_eq!(air.ppo2_at(Meters::new(30.0)).value(), expected);

            Ok(())
        }

        #[test]
        fn ppo2_at_eanx40_10m() -> Result<()> {
            let ean40 = Ean::from_percent(40)?;
            let expected = (10.0_f64 / 10.0 + 1.0) * 0.40; // 0.80 bar

            assert_relative_eq!(ean40.ppo2_at(Meters::new(10.0)).value(), expected);

            Ok(())
        }
    }

    mod label {
        use super::*;

        #[test]
        fn label_air() -> Result<()> {
            assert_eq!(Ean::from_percent(21)?.label(), Some("Air"));

            Ok(())
        }

        #[test]
        fn label_named_nitrox_mixes() -> Result<()> {
            assert_eq!(Ean::from_percent(28)?.label(), Some("EANx 28"));
            assert_eq!(Ean::from_percent(30)?.label(), Some("EANx 30"));
            assert_eq!(Ean::from_percent(32)?.label(), Some("EANx 32"));
            assert_eq!(Ean::from_percent(36)?.label(), Some("EANx 36"));
            assert_eq!(Ean::from_percent(40)?.label(), Some("EANx 40"));

            Ok(())
        }

        #[test]
        fn label_high_o2_mixes() -> Result<()> {
            assert_eq!(Ean::from_percent(50)?.label(), Some("O₂ 50%"));
            assert_eq!(Ean::from_percent(80)?.label(), Some("O₂ 80%"));

            Ok(())
        }

        #[test]
        fn label_hypoxic_mixes() -> Result<()> {
            assert_eq!(Ean::from_percent(10)?.label(), Some("Hypoxic 10"));
            assert_eq!(Ean::from_percent(12)?.label(), Some("Hypoxic 12"));
            assert_eq!(Ean::from_percent(14)?.label(), Some("Hypoxic 14"));
            assert_eq!(Ean::from_percent(16)?.label(), Some("Hypoxic 16"));
            assert_eq!(Ean::from_percent(18)?.label(), Some("Hypoxic 18"));

            Ok(())
        }

        #[test]
        fn label_pure_o2() -> Result<()> {
            assert_eq!(Ean::from_percent(100)?.label(), Some("Pure O₂"));

            Ok(())
        }

        #[test]
        fn label_unlabelled_mix_returns_none() -> Result<()> {
            assert_eq!(Ean::from_percent(25)?.label(), None);
            assert_eq!(Ean::from_percent(33)?.label(), None);

            Ok(())
        }
    }

    mod invalid_o2_percent {
        use super::*;

        #[test]
        fn invalid_o2_percent_display() {
            let msg = format!("{}", InvalidO2Percent(5));
            assert!(msg.contains('5'));
            assert!(msg.contains("10"));
            assert!(msg.contains("100"));
        }
    }
}
