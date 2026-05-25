/// Major oceans and seas, keyed by representative surface salinity and temperature.
///
/// Use [`crate::environment::DiveEnvironment::ocean`] to obtain a [`crate::environment::DiveEnvironment`]
/// for a given body of water.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Ocean {
    // Five oceans
    /// Pacific Ocean — 34.5 ‰, 17 °C.
    Pacific,
    /// Atlantic Ocean — 35.5 ‰, 17 °C.
    Atlantic,
    /// Indian Ocean — 34.5 ‰, 26 °C.
    Indian,
    /// Arctic Ocean — 28.0 ‰, 2 °C.
    Arctic,
    /// Southern Ocean — 34.0 ‰, 2 °C.
    Southern,

    // Major diving seas
    /// Mediterranean Sea — 38.0 ‰, 18 °C.
    Mediterranean,
    /// Red Sea — 40.0 ‰, 26 °C.
    RedSea,
    /// Caribbean Sea — 36.0 ‰, 27 °C.
    Caribbean,
    /// Baltic Sea — 7.0 ‰, 10 °C.
    BalticSea,
    /// Black Sea — 18.0 ‰, 14 °C.
    BlackSea,
    /// Persian Gulf — 40.0 ‰, 28 °C.
    PersianGulf,
    /// North Sea — 34.5 ‰, 10 °C.
    NorthSea,
    /// Coral Sea — 35.5 ‰, 26 °C.
    CoralSea,
    /// Andaman Sea — 33.0 ‰, 28 °C.
    AndamanSea,
    /// South China Sea — 33.0 ‰, 28 °C.
    SouthChinaSea,
    /// Banda Sea — 34.0 ‰, 28 °C.
    BandaSea,
    /// Celebes Sea — 34.0 ‰, 28 °C.
    CelebesSea,
}

impl Ocean {
    /// Representative surface salinity in parts per thousand (‰).
    ///
    /// ```
    /// use dps::environment::Ocean;
    ///
    /// assert_eq!(Ocean::RedSea.salinity_ppt(), 40.0);
    /// assert_eq!(Ocean::BalticSea.salinity_ppt(), 7.0);
    /// ```
    #[must_use]
    pub const fn salinity_ppt(self) -> f64 {
        match self {
            Self::AndamanSea | Self::SouthChinaSea => 33.0,
            Self::Arctic => 28.0,
            Self::Atlantic | Self::CoralSea | Self::NorthSea => 35.5,
            Self::BalticSea => 7.0,
            Self::BandaSea | Self::CelebesSea | Self::Southern => 34.0,
            Self::BlackSea => 18.0,
            Self::Caribbean => 36.0,
            Self::Indian | Self::Pacific => 34.5,
            Self::Mediterranean => 38.0,
            Self::PersianGulf | Self::RedSea => 40.0,
        }
    }

    /// Representative surface temperature in °C.
    ///
    /// ```
    /// use dps::environment::Ocean;
    ///
    /// assert_eq!(Ocean::Mediterranean.typical_temperature_c(), 18.0);
    /// assert_eq!(Ocean::Arctic.typical_temperature_c(), 2.0);
    /// ```
    #[must_use]
    pub const fn typical_temperature_c(self) -> f64 {
        match self {
            Self::AndamanSea
            | Self::PersianGulf
            | Self::BandaSea
            | Self::CelebesSea
            | Self::SouthChinaSea => 28.0,
            Self::Arctic | Self::Southern => 2.0,
            Self::Atlantic | Self::Pacific => 17.0,
            Self::BalticSea | Self::NorthSea => 10.0,
            Self::BlackSea => 14.0,
            Self::Caribbean => 27.0,
            Self::CoralSea | Self::Indian | Self::RedSea => 26.0,
            Self::Mediterranean => 18.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn red_sea_is_saltiest() {
        assert_relative_eq!(Ocean::RedSea.salinity_ppt(), 40.0);
        assert!(Ocean::RedSea.salinity_ppt() > Ocean::Atlantic.salinity_ppt());
    }

    #[test]
    fn baltic_is_least_salty() {
        assert_relative_eq!(Ocean::BalticSea.salinity_ppt(), 7.0);
        assert!(Ocean::BalticSea.salinity_ppt() < Ocean::Mediterranean.salinity_ppt());
    }

    #[test]
    fn arctic_and_southern_are_coldest() {
        assert_relative_eq!(Ocean::Arctic.typical_temperature_c(), 2.0);
        assert_relative_eq!(Ocean::Southern.typical_temperature_c(), 2.0);
        assert!(Ocean::Arctic.typical_temperature_c() < Ocean::Caribbean.typical_temperature_c());
    }

    #[test]
    fn tropical_seas_are_warmest() {
        let warm = [
            Ocean::AndamanSea,
            Ocean::PersianGulf,
            Ocean::BandaSea,
            Ocean::CelebesSea,
            Ocean::SouthChinaSea,
        ];
        for ocean in warm {
            assert_relative_eq!(ocean.typical_temperature_c(), 28.0);
        }
    }
}
