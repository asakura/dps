//! Named freshwater lake presets for [`DiveEnvironment`](crate::DiveEnvironment).
//!
//! Each [`Lake`] variant encodes the altitude and typical water temperature of a
//! notable freshwater dive site. All variants are treated as fresh water (salinity
//! $\approx \pu{0 ‰}$). Pass a variant to
//! [`DiveEnvironment::lake`](crate::DiveEnvironment::lake) to obtain an
//! environment with the correct surface pressure and freshwater density.
//!
//! ```
//! use dps_environment::{DiveEnvironment, Lake};
//!
//! let titicaca = DiveEnvironment::lake(Lake::Titicaca);
//! // High altitude → lower surface pressure than sea level
//! assert!(titicaca.surface_pressure() < DiveEnvironment::standard().surface_pressure());
//! // Fresh water → less dense than seawater, more metres per bar
//! assert!(titicaca.water_density() > DiveEnvironment::standard().water_density());
//! ```

use dps_units::{Celsius, Meters};

/// Notable freshwater dive sites, keyed by altitude and typical water temperature.
///
/// All variants are freshwater (salinity $\approx \pu{0 ‰}$). Use
/// [`crate::DiveEnvironment::lake`] to obtain a
/// [`crate::DiveEnvironment`] for a given lake.
#[derive(
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Hash,
    strum::Display,
    strum::EnumString,
    strum::VariantNames,
    strum::EnumIter,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Lake {
    // Extreme altitude > 5 000 m
    /// Ojos del Salado crater lake, Argentina/Chile — $\pu{6390 m}$, ${\approx}\pu{0 ^\circ C}$.
    /// World's highest crater lake; world-record dive by Marcel Korkus in 2019.
    OjosDeSalado,
    /// Licancabur volcano crater lake, Bolivia/Chile — $\pu{5916 m}$, ${\approx}\pu{2 ^\circ C}$.
    /// Explored for NASA astrobiology research (Cabrol/SETI, 2006).
    Licancabur,

    // High altitude > 1 500 m
    /// Lake Titicaca, Bolivia/Peru — $\pu{3812 m}$, ${\approx}\pu{12 ^\circ C}$.
    Titicaca,
    /// Lake Tahoe, USA — $\pu{1897 m}$, ${\approx}\pu{10 ^\circ C}$.
    Tahoe,
    /// Crater Lake, Oregon, USA — $\pu{1882 m}$, ${\approx}\pu{4 ^\circ C}$.
    CraterLake,
    /// Lake Atitlán, Guatemala — $\pu{1562 m}$, ${\approx}\pu{18 ^\circ C}$.
    Atitlan,

    // Mid altitude 500–1 500 m
    /// Lake Tanganyika, East Africa — $\pu{773 m}$, ${\approx}\pu{25 ^\circ C}$.
    Tanganyika,
    /// Lake Ohrid, North Macedonia/Albania — $\pu{693 m}$, ${\approx}\pu{14 ^\circ C}$.
    Ohrid,

    // Low altitude < 500 m
    /// Lake Malawi, East Africa — $\pu{474 m}$, ${\approx}\pu{24 ^\circ C}$.
    Malawi,
    /// Lake Baikal, Russia — $\pu{456 m}$, ${\approx}\pu{6 ^\circ C}$.
    Baikal,
    /// Lake Bled, Slovenia — $\pu{475 m}$, ${\approx}\pu{12 ^\circ C}$.
    Bled,
    /// Lake Constance, Germany/Austria/Switzerland — $\pu{395 m}$, ${\approx}\pu{10 ^\circ C}$.
    Constance,
    /// Lake Taupō, New Zealand — $\pu{357 m}$, ${\approx}\pu{15 ^\circ C}$.
    Taupo,

    // Sea-level freshwater dive sites
    /// Yucatán cenote systems, Mexico — $\pu{0 m}$, ${\approx}\pu{24 ^\circ C}$.
    /// Cave/sinkhole groundwater systems; freshwater density applies.
    Cenotes,
    /// Florida spring systems, USA — $\pu{0 m}$, ${\approx}\pu{22 ^\circ C}$.
    /// Constant-temperature groundwater springs.
    FloridaSprings,
}

impl Lake {
    /// Altitude above sea level.
    ///
    /// ```
    /// use dps_environment::Lake;
    /// use dps_units::Meters;
    ///
    /// assert_eq!(Lake::Titicaca.altitude(), Meters::new(3_812.0));
    /// assert_eq!(Lake::Cenotes.altitude(), Meters::new(0.0));
    /// ```
    #[must_use]
    pub const fn altitude(self) -> Meters {
        Meters::new(match self {
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
        })
    }

    /// Typical water temperature in $^\circ\text{C}$.
    ///
    /// ```
    /// use dps_environment::Lake;
    /// use dps_units::Celsius;
    ///
    /// assert_eq!(Lake::Titicaca.typical_temperature(), Celsius::new(12.0));
    /// assert_eq!(Lake::CraterLake.typical_temperature(), Celsius::new(4.0));
    /// ```
    #[must_use]
    pub const fn typical_temperature(self) -> Celsius {
        Celsius::new(match self {
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dps_units::Meters;

    use approx::assert_relative_eq;
    use rstest::rstest;

    mod altitude {
        use super::*;

        #[rstest]
        fn titicaca() {
            assert_relative_eq!(Lake::Titicaca.altitude(), Meters::new(3_812.0));
        }

        #[rstest]
        fn sea_level_lakes_have_zero_altitude() {
            assert_relative_eq!(Lake::Cenotes.altitude(), Meters::new(0.0));
            assert_relative_eq!(Lake::FloridaSprings.altitude(), Meters::new(0.0));
        }

        #[rstest]
        fn ojos_del_salado_is_highest() {
            assert!(Lake::OjosDeSalado.altitude() > Lake::Licancabur.altitude());
            assert!(Lake::Licancabur.altitude() > Lake::Titicaca.altitude());
        }
    }

    mod typical_temperature {
        use super::*;

        #[rstest]
        fn titicaca() {
            assert_relative_eq!(Lake::Titicaca.typical_temperature(), Celsius::new(12.0));
        }

        #[rstest]
        fn licancabur_is_coldest() {
            assert_relative_eq!(Lake::Licancabur.typical_temperature(), Celsius::new(2.0));
            assert!(Lake::Licancabur.typical_temperature() < Lake::Titicaca.typical_temperature());
        }

        #[rstest]
        fn ojos_del_salado_is_at_freezing() {
            assert_relative_eq!(Lake::OjosDeSalado.typical_temperature(), Celsius::new(0.0));
            assert!(
                Lake::OjosDeSalado.typical_temperature() < Lake::Licancabur.typical_temperature()
            );
        }
    }
}
