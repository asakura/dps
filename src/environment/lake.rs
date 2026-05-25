/// Notable freshwater dive sites, keyed by altitude and typical water temperature.
///
/// All variants are freshwater (salinity ≈ 0 ‰). Use
/// [`crate::environment::DiveEnvironment::lake`] to obtain a
/// [`crate::environment::DiveEnvironment`] for a given lake.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Lake {
    // Extreme altitude > 5 000 m
    /// Ojos del Salado crater lake, Argentina/Chile — 6 390 m, ~0 °C.
    /// World's highest crater lake; world-record dive by Marcel Korkus in 2019.
    OjosDeSalado,
    /// Licancabur volcano crater lake, Bolivia/Chile — 5 916 m, ~2 °C.
    /// Explored for NASA astrobiology research (Cabrol/SETI, 2006).
    Licancabur,

    // High altitude > 1 500 m
    /// Lake Titicaca, Bolivia/Peru — 3 812 m, ~12 °C.
    Titicaca,
    /// Lake Tahoe, USA — 1 897 m, ~10 °C.
    Tahoe,
    /// Crater Lake, Oregon, USA — 1 882 m, ~4 °C.
    CraterLake,
    /// Lake Atitlán, Guatemala — 1 562 m, ~18 °C.
    Atitlan,

    // Mid altitude 500–1 500 m
    /// Lake Tanganyika, East Africa — 773 m, ~25 °C.
    Tanganyika,
    /// Lake Ohrid, North Macedonia/Albania — 693 m, ~14 °C.
    Ohrid,

    // Low altitude < 500 m
    /// Lake Malawi, East Africa — 474 m, ~24 °C.
    Malawi,
    /// Lake Baikal, Russia — 456 m, ~6 °C.
    Baikal,
    /// Lake Bled, Slovenia — 475 m, ~12 °C.
    Bled,
    /// Lake Constance, Germany/Austria/Switzerland — 395 m, ~10 °C.
    Constance,
    /// Lake Taupō, New Zealand — 357 m, ~15 °C.
    Taupo,

    // Sea-level freshwater dive sites
    /// Yucatán cenote systems, Mexico — 0 m, ~24 °C.
    /// Cave/sinkhole groundwater systems; freshwater density applies.
    Cenotes,
    /// Florida spring systems, USA — 0 m, ~22 °C.
    /// Constant-temperature groundwater springs.
    FloridaSprings,
}

impl Lake {
    /// Altitude above sea level in metres.
    ///
    /// ```
    /// use dps::environment::Lake;
    ///
    /// assert_eq!(Lake::Titicaca.altitude_m(), 3_812.0);
    /// assert_eq!(Lake::Cenotes.altitude_m(), 0.0);
    /// ```
    #[must_use]
    pub const fn altitude_m(self) -> f64 {
        match self {
            Self::Atitlan => 1_562.0,
            Self::Baikal => 456.0,
            Self::Bled => 475.0,
            Self::Cenotes | Self::FloridaSprings => 0.0,
            Self::Constance => 395.0,
            Self::CraterLake => 1_882.0,
            Self::Licancabur => 5_916.0,
            Self::Malawi => 474.0,
            Self::Ohrid => 693.0,
            Self::OjosDeSalado => 6_390.0,
            Self::Tahoe => 1_897.0,
            Self::Tanganyika => 773.0,
            Self::Taupo => 357.0,
            Self::Titicaca => 3_812.0,
        }
    }

    /// Typical water temperature in °C.
    ///
    /// ```
    /// use dps::environment::Lake;
    ///
    /// assert_eq!(Lake::Titicaca.typical_temperature_c(), 12.0);
    /// assert_eq!(Lake::CraterLake.typical_temperature_c(), 4.0);
    /// ```
    #[must_use]
    pub const fn typical_temperature_c(self) -> f64 {
        match self {
            Self::Atitlan => 18.0,
            Self::Baikal => 6.0,
            Self::Bled | Self::Titicaca => 12.0,
            Self::Cenotes | Self::Malawi => 24.0,
            Self::Constance | Self::Tahoe => 10.0,
            Self::CraterLake => 4.0,
            Self::FloridaSprings => 22.0,
            Self::Licancabur => 2.0,
            Self::Ohrid => 14.0,
            Self::OjosDeSalado => 0.0,
            Self::Tanganyika => 25.0,
            Self::Taupo => 15.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn titicaca_altitude_and_temperature() {
        assert_relative_eq!(Lake::Titicaca.altitude_m(), 3_812.0);
        assert_relative_eq!(Lake::Titicaca.typical_temperature_c(), 12.0);
    }

    #[test]
    fn sea_level_lakes_have_zero_altitude() {
        assert_relative_eq!(Lake::Cenotes.altitude_m(), 0.0);
        assert_relative_eq!(Lake::FloridaSprings.altitude_m(), 0.0);
    }

    #[test]
    fn ojos_del_salado_is_highest() {
        assert!(Lake::OjosDeSalado.altitude_m() > Lake::Licancabur.altitude_m());
        assert!(Lake::Licancabur.altitude_m() > Lake::Titicaca.altitude_m());
    }

    #[test]
    fn licancabur_is_coldest() {
        assert_relative_eq!(Lake::Licancabur.typical_temperature_c(), 2.0);
        assert!(Lake::Licancabur.typical_temperature_c() < Lake::Titicaca.typical_temperature_c());
    }
}
