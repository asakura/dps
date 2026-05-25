/// Major oceans and seas, keyed by representative surface salinity and temperature.
///
/// Use [`crate::environment::DiveEnvironment::ocean`] to obtain a [`crate::environment::DiveEnvironment`]
/// for a given body of water.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Ocean {
    // Five oceans
    /// Pacific Ocean — $\pu{34.5 ‰}$, $\pu{17 ^\circ C}$.
    Pacific,
    /// Atlantic Ocean — $\pu{35.5 ‰}$, $\pu{17 ^\circ C}$.
    Atlantic,
    /// Indian Ocean — $\pu{34.5 ‰}$, $\pu{26 ^\circ C}$.
    Indian,
    /// Arctic Ocean — $\pu{28.0 ‰}$, $\pu{2 ^\circ C}$.
    Arctic,
    /// Southern Ocean — $\pu{34.0 ‰}$, $\pu{2 ^\circ C}$.
    Southern,

    // Major diving seas
    /// Mediterranean Sea — $\pu{38.0 ‰}$, $\pu{18 ^\circ C}$.
    Mediterranean,
    /// Red Sea — $\pu{40.0 ‰}$, $\pu{26 ^\circ C}$.
    RedSea,
    /// Caribbean Sea — $\pu{36.0 ‰}$, $\pu{27 ^\circ C}$.
    Caribbean,
    /// Baltic Sea — $\pu{7.0 ‰}$, $\pu{10 ^\circ C}$.
    BalticSea,
    /// Black Sea — $\pu{18.0 ‰}$, $\pu{14 ^\circ C}$.
    BlackSea,
    /// Persian Gulf — $\pu{40.0 ‰}$, $\pu{28 ^\circ C}$.
    PersianGulf,
    /// North Sea — $\pu{34.5 ‰}$, $\pu{10 ^\circ C}$.
    NorthSea,
    /// Coral Sea — $\pu{35.5 ‰}$, $\pu{26 ^\circ C}$.
    CoralSea,
    /// Andaman Sea — $\pu{33.0 ‰}$, $\pu{28 ^\circ C}$.
    AndamanSea,
    /// South China Sea — $\pu{33.0 ‰}$, $\pu{28 ^\circ C}$.
    SouthChinaSea,
    /// Banda Sea — $\pu{34.0 ‰}$, $\pu{28 ^\circ C}$.
    BandaSea,
    /// Celebes Sea — $\pu{34.0 ‰}$, $\pu{28 ^\circ C}$.
    CelebesSea,
}

impl Ocean {
    /// Representative surface salinity in parts per thousand ($\text{‰}$).
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

    /// Representative surface temperature in $^\circ\text{C}$.
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
