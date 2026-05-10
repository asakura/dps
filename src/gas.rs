//! Gas mix types and named presets for dive planning.

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
    pub fn from_percent(o2_pct: u8) -> Self {
        debug_assert!((10..=100).contains(&o2_pct));

        Self {
            fraction: o2_pct as f64 / 100.0,
        }
    }

    /// Returns the O₂ fraction as a whole percentage (10–100).
    pub fn o2_percent(self) -> u8 {
        (self.fraction * 100.0).round() as u8
    }

    /// Returns the O₂ fraction as a decimal in [0.10, 1.0].
    pub fn fo2(self) -> f64 {
        self.fraction
    }

    /// Maximum Operating Depth for a given ppO₂ limit.
    ///
    /// Formula: MOD = (ppO₂_max / FO₂ − 1 atm) × 10 m/bar  (seawater approximation)
    pub fn mod_at(self, ppo2_max: Bar) -> Meters {
        let gauge = ppo2_max / self.fo2() - SURFACE_PRESSURE;
        (gauge * SEAWATER).max(Meters::new(0.0))
    }

    /// ppO₂ for this mix at the given depth.
    pub fn ppo2_at(self, depth: Meters) -> Bar {
        (depth / SEAWATER + SURFACE_PRESSURE) * self.fraction
    }

    /// Named label for this mix, if one exists.
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
