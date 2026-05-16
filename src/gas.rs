//! Gas mix types and named presets for dive planning.

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
    /// ```
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
    /// ```
    /// use dps::gas::Ean;
    /// assert_eq!(Ean::from_percent(32).unwrap().o2_percent(), 32);
    /// ```
    #[must_use]
    pub fn o2_percent(self) -> u8 {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "fraction is in [0.10, 1.0] so rounding gives a value in [10, 100] which fits u8")]
        let pct = (self.fraction * 100.0).round() as u8;
        pct
    }

    /// Returns the O₂ fraction as a decimal in [0.10, 1.0].
    ///
    /// ```
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
    /// ```
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
    /// ```
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
    /// ```
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

    // --- from_percent ---

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
        let err = Ean::from_percent(5).unwrap_err();
        assert_eq!(err.0, 5);
    }

    // --- o2_percent / fo2 ---

    #[test]
    fn o2_percent_roundtrips() {
        for pct in [10u8, 21, 32, 36, 40, 80, 100] {
            assert_eq!(Ean::from_percent(pct).unwrap().o2_percent(), pct);
        }
    }

    #[test]
    fn fo2_matches_fraction() {
        assert_relative_eq!(Ean::from_percent(21).unwrap().fo2(), 0.21);
        assert_relative_eq!(Ean::from_percent(32).unwrap().fo2(), 0.32);
        assert_relative_eq!(Ean::from_percent(100).unwrap().fo2(), 1.0);
    }

    // --- mod_at ---
    // Formula: MOD = (ppO2_max / FO2 − 1 bar) × 10 m/bar

    #[test]
    fn mod_at_eanx32_1_4_bar() {
        let ean32 = Ean::from_percent(32).unwrap();
        let expected = (1.4_f64 / 0.32 - 1.0) * 10.0; // ≈ 33.75 m
        assert_relative_eq!(ean32.mod_at(Bar::new(1.4)).value(), expected);
    }

    #[test]
    fn mod_at_eanx40_1_4_bar() {
        let ean40 = Ean::from_percent(40).unwrap();
        let expected = (1.4_f64 / 0.40 - 1.0) * 10.0; // 25.0 m
        assert_relative_eq!(ean40.mod_at(Bar::new(1.4)).value(), expected);
    }

    #[test]
    fn mod_at_pure_o2_1_6_bar() {
        let o2 = Ean::from_percent(100).unwrap();
        let expected = (1.6_f64 / 1.0 - 1.0) * 10.0; // 6.0 m
        assert_relative_eq!(o2.mod_at(Bar::new(1.6)).value(), expected);
    }

    #[test]
    fn mod_at_clamps_to_zero_when_negative() {
        // Pure O2 at 0.5 bar ppO2 limit: (0.5/1.0 - 1) * 10 = -5.0 m → 0.0
        let o2 = Ean::from_percent(100).unwrap();
        assert_relative_eq!(o2.mod_at(Bar::new(0.5)).value(), 0.0, epsilon = 1e-9);
    }

    // --- ppo2_at ---
    // Formula: ppO2 = (depth_m / 10 + 1) × FO2

    #[test]
    fn ppo2_at_surface_equals_fo2() {
        let ean32 = Ean::from_percent(32).unwrap();
        assert_relative_eq!(ean32.ppo2_at(Meters::new(0.0)).value(), 0.32);
    }

    #[test]
    fn ppo2_at_air_30m() {
        let air = Ean::from_percent(21).unwrap();
        let expected = (30.0_f64 / 10.0 + 1.0) * 0.21; // 0.84 bar
        assert_relative_eq!(air.ppo2_at(Meters::new(30.0)).value(), expected);
    }

    #[test]
    fn ppo2_at_eanx40_10m() {
        let ean40 = Ean::from_percent(40).unwrap();
        let expected = (10.0_f64 / 10.0 + 1.0) * 0.40; // 0.80 bar
        assert_relative_eq!(ean40.ppo2_at(Meters::new(10.0)).value(), expected);
    }

    // --- label ---

    #[test]
    fn label_air() {
        assert_eq!(Ean::from_percent(21).unwrap().label(), Some("Air"));
    }

    #[test]
    fn label_named_nitrox_mixes() {
        assert_eq!(Ean::from_percent(28).unwrap().label(), Some("EANx 28"));
        assert_eq!(Ean::from_percent(30).unwrap().label(), Some("EANx 30"));
        assert_eq!(Ean::from_percent(32).unwrap().label(), Some("EANx 32"));
        assert_eq!(Ean::from_percent(36).unwrap().label(), Some("EANx 36"));
        assert_eq!(Ean::from_percent(40).unwrap().label(), Some("EANx 40"));
    }

    #[test]
    fn label_high_o2_mixes() {
        assert_eq!(Ean::from_percent(50).unwrap().label(), Some("O₂ 50%"));
        assert_eq!(Ean::from_percent(80).unwrap().label(), Some("O₂ 80%"));
    }

    #[test]
    fn label_hypoxic_mixes() {
        assert_eq!(Ean::from_percent(10).unwrap().label(), Some("Hypoxic 10"));
        assert_eq!(Ean::from_percent(12).unwrap().label(), Some("Hypoxic 12"));
        assert_eq!(Ean::from_percent(14).unwrap().label(), Some("Hypoxic 14"));
        assert_eq!(Ean::from_percent(16).unwrap().label(), Some("Hypoxic 16"));
        assert_eq!(Ean::from_percent(18).unwrap().label(), Some("Hypoxic 18"));
    }

    #[test]
    fn label_pure_o2() {
        assert_eq!(Ean::from_percent(100).unwrap().label(), Some("Pure O₂"));
    }

    #[test]
    fn label_unlabelled_mix_returns_none() {
        assert_eq!(Ean::from_percent(25).unwrap().label(), None);
        assert_eq!(Ean::from_percent(33).unwrap().label(), None);
    }

    // --- Display for InvalidO2Percent ---

    #[test]
    fn invalid_o2_percent_display() {
        let msg = format!("{}", InvalidO2Percent(5));
        assert!(msg.contains("5"));
        assert!(msg.contains("10"));
        assert!(msg.contains("100"));
    }
}
