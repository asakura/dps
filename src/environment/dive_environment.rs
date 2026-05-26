//! The [`DiveEnvironment`] struct: dive site parameters for depth↔pressure conversion.
//!
//! A [`DiveEnvironment`] holds exactly two values — surface pressure (varies with
//! altitude) and water density (varies with salinity and temperature) — and derives
//! everything else from them.
//!
//! ```ignore
//! use dps::environment::{DiveEnvironment, Ocean, Lake};
//!
//! // Named preset: Red Sea at sea level
//! let env = DiveEnvironment::ocean(Ocean::RedSea);
//! assert!(env.water_density() < DiveEnvironment::standard().water_density());
//!
//! // Named preset: high-altitude freshwater lake
//! let alpine = DiveEnvironment::lake(Lake::Titicaca);
//! assert!(alpine.surface_pressure() < DiveEnvironment::standard().surface_pressure());
//! ```

use crate::units::{Bar, Celsius, Meters, MetersPerBar, PartsPerThousand};

use super::error::DiveEnvironmentError;
use super::physics::{
    FRESHWATER_TEMP_C, ISO_SEAWATER_DENSITY, MAX_ALTITUDE, MAX_SALINITY_PPT, MAX_TEMP_C,
    MIN_TEMP_C, PA_PER_BAR, SEA_LEVEL_PRESSURE_BAR, STANDARD_GRAVITY, altitude_to_pressure_bar,
    water_density_from,
};
use super::{Lake, Ocean};

/// Dive environment parameters for depth↔pressure conversion.
///
/// Encapsulates surface pressure (varies with altitude) and water density
/// (varies with salinity and temperature). All [`crate::gas::EANxBlend`]
/// calculations use these values instead of fixed constants.
///
/// Use [`DiveEnvironment::standard`] for typical sea-level saltwater diving,
/// or one of the other constructors for altitude or freshwater environments.
/// Attach to a blend with [`crate::gas::EANxBlend::with_environment`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiveEnvironment {
    surface_pressure: Bar,
    water_density: MetersPerBar,
}

impl DiveEnvironment {
    // Infallible presets

    /// ISO standard seawater at sea level: $\pu{35 ‰}$, $\pu{15 ^\circ C}$, $\pu{1.01325 bar}$.
    ///
    /// This is the baseline used by dive tables, certification agencies, and
    /// dive computers.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::{Bar, MetersPerBar};
    /// use approx::assert_relative_eq;
    ///
    /// let env = DiveEnvironment::standard();
    /// assert_relative_eq!(env.surface_pressure(), Bar::new(1.013_25), epsilon = 1e-9);
    /// assert_relative_eq!(
    ///     env.water_density(),
    ///     MetersPerBar::new(1e5 / (1025.0 * 9.806_65)),
    ///     epsilon = 1e-9
    /// );
    /// ```
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            surface_pressure: SEA_LEVEL_PRESSURE_BAR,
            water_density: MetersPerBar::new(
                PA_PER_BAR / (ISO_SEAWATER_DENSITY * STANDARD_GRAVITY),
            ),
        }
    }

    /// Fresh water at sea level: $\pu{0 ‰}$ salinity, $\pu{20 ^\circ C}$.
    ///
    /// Suitable for quarry, river, and cave diving at sea level.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    ///
    /// let env = DiveEnvironment::freshwater();
    /// // fresh water is less dense than standard seawater — more metres per bar
    /// assert!(env.water_density() > DiveEnvironment::standard().water_density());
    /// assert_eq!(env.surface_pressure(), DiveEnvironment::standard().surface_pressure());
    /// ```
    #[must_use]
    pub fn freshwater() -> Self {
        Self {
            surface_pressure: SEA_LEVEL_PRESSURE_BAR,
            water_density: water_density_from(PartsPerThousand::new(0.0), FRESHWATER_TEMP_C),
        }
    }

    /// Sea-level environment for a named ocean or sea.
    ///
    /// Density is computed from the [`Ocean`] variant's representative salinity
    /// and temperature.
    ///
    /// ```
    /// use dps::environment::{DiveEnvironment, Ocean};
    ///
    /// let red_sea = DiveEnvironment::ocean(Ocean::RedSea);
    /// // Red Sea (41 ‰) is saltier than ISO standard (35 ‰) — denser, fewer m/bar
    /// assert!(red_sea.water_density() < DiveEnvironment::standard().water_density());
    /// assert_eq!(red_sea.surface_pressure(), DiveEnvironment::standard().surface_pressure());
    /// ```
    #[must_use]
    pub fn ocean(ocean: Ocean) -> Self {
        Self {
            surface_pressure: SEA_LEVEL_PRESSURE_BAR,
            water_density: water_density_from(ocean.salinity(), ocean.typical_temperature()),
        }
    }

    /// Environment for a named freshwater dive lake.
    ///
    /// Surface pressure is derived from the [`Lake`] variant's altitude via the
    /// ICAO barometric formula. Density is freshwater at the lake's typical
    /// temperature.
    ///
    /// ```
    /// use dps::environment::{DiveEnvironment, Lake};
    ///
    /// let titicaca = DiveEnvironment::lake(Lake::Titicaca);
    /// // high altitude → lower surface pressure than sea level
    /// assert!(titicaca.surface_pressure() < DiveEnvironment::standard().surface_pressure());
    /// // freshwater → greater m/bar than seawater
    /// assert!(titicaca.water_density() > DiveEnvironment::standard().water_density());
    /// ```
    #[must_use]
    pub fn lake(lake: Lake) -> Self {
        Self {
            surface_pressure: altitude_to_pressure_bar(lake.altitude()),
            water_density: water_density_from(
                PartsPerThousand::new(0.0),
                lake.typical_temperature(),
            ),
        }
    }

    // Fallible constructors

    /// Constructs from explicit surface pressure and water density.
    ///
    /// Both values must be finite and positive. Passing zero or a negative value
    /// would cause division by zero or sign inversion in all depth calculations.
    ///
    /// # Errors
    ///
    /// - [`DiveEnvironmentError::SurfacePressureNotPositive`] if `surface_pressure ≤ 0` or non-finite.
    /// - [`DiveEnvironmentError::WaterDensityNotPositive`] if `water_density ≤ 0` or non-finite.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::{Bar, MetersPerBar};
    ///
    /// let env = DiveEnvironment::new(Bar::new(0.95), MetersPerBar::new(10.1)).unwrap();
    /// assert_eq!(env.surface_pressure(), Bar::new(0.95));
    /// assert_eq!(env.water_density(), MetersPerBar::new(10.1));
    /// ```
    pub fn new(
        surface_pressure: Bar,
        water_density: MetersPerBar,
    ) -> Result<Self, DiveEnvironmentError> {
        if !surface_pressure.is_positive_finite() {
            return Err(DiveEnvironmentError::SurfacePressureNotPositive(
                surface_pressure,
            ));
        }

        if !water_density.is_positive_finite() {
            return Err(DiveEnvironmentError::WaterDensityNotPositive(water_density));
        }

        Ok(Self {
            surface_pressure,
            water_density,
        })
    }

    /// Seawater environment at the given altitude above sea level.
    ///
    /// Surface pressure is derived via the ICAO barometric formula. Water density
    /// uses ISO standard seawater ($\pu{35 ‰}$, $\pu{15 ^\circ C}$).
    ///
    /// # Errors
    ///
    /// [`DiveEnvironmentError::AltitudeOutOfRange`] if `altitude` is outside
    /// $[\pu{0.0 m}, \pu{8849.0 m}]$ or non-finite.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::Meters;
    ///
    /// let high = DiveEnvironment::at_altitude(Meters::new(1_000.0)).unwrap();
    /// assert!(high.surface_pressure() < DiveEnvironment::standard().surface_pressure());
    /// // water density is unchanged — still ISO seawater
    /// assert_eq!(high.water_density(), DiveEnvironment::standard().water_density());
    /// ```
    pub fn at_altitude(altitude: Meters) -> Result<Self, DiveEnvironmentError> {
        validate_altitude(altitude)?;

        Ok(Self {
            surface_pressure: altitude_to_pressure_bar(altitude),
            water_density: Self::standard().water_density,
        })
    }

    /// Freshwater environment at the given altitude above sea level.
    ///
    /// Surface pressure is derived via the ICAO barometric formula. Water density
    /// uses fresh water at $\pu{20 ^\circ C}$.
    ///
    /// # Errors
    ///
    /// [`DiveEnvironmentError::AltitudeOutOfRange`] if `altitude` is outside
    /// $[\pu{0.0 m}, \pu{8849.0 m}]$ or non-finite.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::Meters;
    ///
    /// let alpine = DiveEnvironment::freshwater_at_altitude(Meters::new(1_000.0)).unwrap();
    /// assert!(alpine.surface_pressure() < DiveEnvironment::standard().surface_pressure());
    /// // freshwater is less dense than seawater — more metres per bar
    /// assert!(alpine.water_density() > DiveEnvironment::standard().water_density());
    /// ```
    pub fn freshwater_at_altitude(altitude: Meters) -> Result<Self, DiveEnvironmentError> {
        validate_altitude(altitude)?;

        Ok(Self {
            surface_pressure: altitude_to_pressure_bar(altitude),
            water_density: Self::freshwater().water_density,
        })
    }

    /// Sea-level environment for the given salinity at $\pu{20 ^\circ C}$.
    ///
    /// # Errors
    ///
    /// [`DiveEnvironmentError::SalinityOutOfRange`] if `salinity` is outside
    /// $[\pu{0.0 ‰}, \pu{350.0 ‰}]$ or non-finite.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::PartsPerThousand;
    ///
    /// let brackish = DiveEnvironment::with_salinity(PartsPerThousand::new(10.0)).unwrap();
    /// // 10 ‰ is denser than fresh water but less dense than standard seawater
    /// assert!(brackish.water_density() < DiveEnvironment::freshwater().water_density());
    /// assert!(brackish.water_density() > DiveEnvironment::standard().water_density());
    /// ```
    pub fn with_salinity(salinity: PartsPerThousand) -> Result<Self, DiveEnvironmentError> {
        validate_salinity(salinity)?;

        Ok(Self {
            surface_pressure: SEA_LEVEL_PRESSURE_BAR,
            water_density: water_density_from(salinity, FRESHWATER_TEMP_C),
        })
    }

    /// Sea-level environment for the given salinity and water temperature.
    ///
    /// Passing `(35.0 ‰, 15.0 °C)` reproduces the ISO standard seawater reference
    /// and gives the same water density as [`DiveEnvironment::standard`].
    ///
    /// # Errors
    ///
    /// - [`DiveEnvironmentError::SalinityOutOfRange`] if `salinity` is outside $[\pu{0.0 ‰}, \pu{350.0 ‰}]$.
    /// - [`DiveEnvironmentError::TemperatureOutOfRange`] if `temperature` is outside $[\pu{-2.0 ^\circ C}, \pu{40.0 ^\circ C}]$.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::{Celsius, PartsPerThousand};
    /// use approx::assert_relative_eq;
    ///
    /// let iso = DiveEnvironment::with_salinity_at_temperature(
    ///     PartsPerThousand::new(35.0),
    ///     Celsius::new(15.0),
    /// ).unwrap();
    /// assert_relative_eq!(
    ///     iso.water_density(),
    ///     DiveEnvironment::standard().water_density(),
    ///     epsilon = 1e-9
    /// );
    /// ```
    pub fn with_salinity_at_temperature(
        salinity: PartsPerThousand,
        temperature: Celsius,
    ) -> Result<Self, DiveEnvironmentError> {
        validate_salinity(salinity)?;
        validate_temperature(temperature)?;

        Ok(Self {
            surface_pressure: SEA_LEVEL_PRESSURE_BAR,
            water_density: water_density_from(salinity, temperature),
        })
    }

    // Builder refinements

    /// Returns a copy with surface pressure recomputed for the given altitude.
    ///
    /// Water density is unchanged — use this to combine an ocean preset with a
    /// non-sea-level pressure (e.g. an elevated saltwater pool).
    ///
    /// # Errors
    ///
    /// [`DiveEnvironmentError::AltitudeOutOfRange`] if `altitude` is outside
    /// $[\pu{0.0 m}, \pu{8849.0 m}]$ or non-finite.
    ///
    /// ```
    /// use dps::environment::{DiveEnvironment, Ocean};
    /// use dps::units::Meters;
    ///
    /// let elevated = DiveEnvironment::ocean(Ocean::RedSea)
    ///     .with_altitude(Meters::new(500.0))
    ///     .unwrap();
    /// let sea_level = DiveEnvironment::ocean(Ocean::RedSea);
    ///
    /// assert!(elevated.surface_pressure() < sea_level.surface_pressure());
    /// // salinity-derived density is untouched
    /// assert_eq!(elevated.water_density(), sea_level.water_density());
    /// ```
    pub fn with_altitude(self, altitude: Meters) -> Result<Self, DiveEnvironmentError> {
        validate_altitude(altitude)?;

        Ok(Self {
            surface_pressure: altitude_to_pressure_bar(altitude),
            ..self
        })
    }

    /// Returns a copy with the given surface pressure.
    ///
    /// # Errors
    ///
    /// [`DiveEnvironmentError::SurfacePressureNotPositive`] if `p ≤ 0` or non-finite.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::Bar;
    ///
    /// let custom = DiveEnvironment::standard()
    ///     .with_surface_pressure(Bar::new(0.90))
    ///     .unwrap();
    /// assert_eq!(custom.surface_pressure(), Bar::new(0.90));
    /// assert_eq!(custom.water_density(), DiveEnvironment::standard().water_density());
    /// ```
    pub fn with_surface_pressure(
        self,
        surface_pressure: Bar,
    ) -> Result<Self, DiveEnvironmentError> {
        if !surface_pressure.is_positive_finite() {
            return Err(DiveEnvironmentError::SurfacePressureNotPositive(
                surface_pressure,
            ));
        }

        Ok(Self {
            surface_pressure,
            ..self
        })
    }

    /// Returns a copy with the given water density.
    ///
    /// # Errors
    ///
    /// [`DiveEnvironmentError::WaterDensityNotPositive`] if `d ≤ 0` or non-finite.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::MetersPerBar;
    ///
    /// let custom = DiveEnvironment::standard()
    ///     .with_water_density(MetersPerBar::new(10.2))
    ///     .unwrap();
    /// assert_eq!(custom.water_density(), MetersPerBar::new(10.2));
    /// assert_eq!(custom.surface_pressure(), DiveEnvironment::standard().surface_pressure());
    /// ```
    pub fn with_water_density(
        self,
        water_density: MetersPerBar,
    ) -> Result<Self, DiveEnvironmentError> {
        if !water_density.is_positive_finite() {
            return Err(DiveEnvironmentError::WaterDensityNotPositive(water_density));
        }

        Ok(Self {
            water_density,
            ..self
        })
    }

    // Accessors

    /// Surface pressure at the dive site.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::Bar;
    /// use approx::assert_relative_eq;
    ///
    /// // ISO sea-level pressure: 1.013 25 bar
    /// assert_relative_eq!(
    ///     DiveEnvironment::standard().surface_pressure(),
    ///     Bar::new(1.013_25),
    ///     epsilon = 1e-9
    /// );
    /// ```
    #[must_use]
    pub const fn surface_pressure(self) -> Bar {
        self.surface_pressure
    }

    /// Water density expressed as metres per bar of gauge pressure.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::MetersPerBar;
    /// use approx::assert_relative_eq;
    ///
    /// // ISO seawater (1025 kg/m³): ~9.948 m per bar
    /// assert_relative_eq!(
    ///     DiveEnvironment::standard().water_density(),
    ///     MetersPerBar::new(1e5 / (1025.0 * 9.806_65)),
    ///     epsilon = 1e-9
    /// );
    /// ```
    #[must_use]
    pub const fn water_density(self) -> MetersPerBar {
        self.water_density
    }

    // Depth↔pressure

    /// Absolute pressure at the given depth.
    ///
    /// $$
    /// P_\text{abs} = P_\text{surface} + \frac{\text{depth}}{\text{water density}}
    /// $$
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::Meters;
    /// use approx::assert_relative_eq;
    ///
    /// let env = DiveEnvironment::standard();
    /// // At the surface, absolute pressure equals surface pressure.
    /// assert_relative_eq!(
    ///     env.absolute_pressure(Meters::new(0.0)),
    ///     env.surface_pressure(),
    ///     epsilon = 1e-9
    /// );
    /// // Deeper means higher absolute pressure.
    /// assert!(env.absolute_pressure(Meters::new(30.0)) > env.surface_pressure());
    /// ```
    #[must_use]
    pub fn absolute_pressure(self, depth: Meters) -> Bar {
        depth / self.water_density + self.surface_pressure
    }

    /// Depth at which the given absolute pressure occurs.
    ///
    /// $$
    /// \text{depth} = (P_\text{abs} - P_\text{surface}) \times \text{water density}
    /// $$
    ///
    /// Returns [`Meters::new(0.0)`](crate::units::Meters) when `absolute_pressure ≤ surface_pressure`
    /// (at or above the surface).
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::Meters;
    /// use approx::assert_relative_eq;
    ///
    /// let env = DiveEnvironment::standard();
    /// let depth = Meters::new(30.0);
    /// // Roundtrip: depth → pressure → depth.
    /// assert_relative_eq!(env.depth(env.absolute_pressure(depth)), depth, epsilon = 1e-9);
    /// // Surface pressure or lower maps to the surface.
    /// assert_eq!(env.depth(env.surface_pressure()), Meters::new(0.0));
    /// ```
    #[must_use]
    pub fn depth(self, absolute_pressure: Bar) -> Meters {
        (absolute_pressure - self.surface_pressure).max(Bar::new(0.0)) * self.water_density
    }
}

impl Default for DiveEnvironment {
    fn default() -> Self {
        Self::standard()
    }
}

impl From<Ocean> for DiveEnvironment {
    fn from(ocean: Ocean) -> Self {
        Self::ocean(ocean)
    }
}

impl From<Lake> for DiveEnvironment {
    fn from(lake: Lake) -> Self {
        Self::lake(lake)
    }
}

fn validate_altitude(altitude: Meters) -> Result<(), DiveEnvironmentError> {
    if !altitude.is_finite() || !altitude.contains(Meters::new(0.0)..=MAX_ALTITUDE) {
        Err(DiveEnvironmentError::AltitudeOutOfRange(altitude))
    } else {
        Ok(())
    }
}

fn validate_salinity(salinity: PartsPerThousand) -> Result<(), DiveEnvironmentError> {
    if !salinity.is_finite() || !salinity.contains(PartsPerThousand::new(0.0)..=MAX_SALINITY_PPT) {
        Err(DiveEnvironmentError::SalinityOutOfRange(salinity))
    } else {
        Ok(())
    }
}

fn validate_temperature(temperature: Celsius) -> Result<(), DiveEnvironmentError> {
    if !temperature.is_finite() || !temperature.contains(MIN_TEMP_C..=MAX_TEMP_C) {
        Err(DiveEnvironmentError::TemperatureOutOfRange(temperature))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::DiveEnvironment;
    use crate::environment::{DiveEnvironmentError, Lake, Ocean};
    use crate::units::{Bar, Celsius, Meters, MetersPerBar, PartsPerThousand};
    use approx::assert_relative_eq;
    use color_eyre::Result;

    mod presets {
        use super::*;

        #[test]
        fn standard_matches_legacy_constants() {
            let env = DiveEnvironment::standard();

            assert_relative_eq!(env.surface_pressure(), Bar::new(1.013_25), epsilon = 1e-6);
            assert_relative_eq!(
                env.water_density(),
                MetersPerBar::new(1e5 / (1025.0 * 9.806_65)),
                epsilon = 1e-6
            );
        }

        #[test]
        fn standard_and_iso_formula_agree() -> Result<()> {
            let iso = DiveEnvironment::with_salinity_at_temperature(
                PartsPerThousand::new(35.0),
                Celsius::new(15.0),
            )?;

            assert_relative_eq!(
                iso.water_density(),
                DiveEnvironment::standard().water_density(),
                epsilon = 1e-9
            );

            Ok(())
        }

        #[test]
        fn freshwater_is_less_dense_than_seawater() {
            let fresh = DiveEnvironment::freshwater();
            let salt = DiveEnvironment::standard();

            assert!(fresh.water_density() > salt.water_density());
        }

        #[test]
        fn red_sea_is_denser_than_standard() {
            let red_sea = DiveEnvironment::ocean(Ocean::RedSea);
            let std = DiveEnvironment::standard();

            assert!(red_sea.water_density() < std.water_density());
        }

        #[test]
        fn titicaca_matches_freshwater_at_altitude() -> Result<()> {
            let preset = DiveEnvironment::lake(Lake::Titicaca);
            let manual = DiveEnvironment::freshwater_at_altitude(Meters::new(3_812.0))?;

            assert_relative_eq!(
                preset.surface_pressure(),
                manual.surface_pressure(),
                epsilon = 1e-6
            );

            Ok(())
        }

        #[test]
        fn from_ocean_matches_ocean_constructor() {
            assert_eq!(
                DiveEnvironment::from(Ocean::Caribbean),
                DiveEnvironment::ocean(Ocean::Caribbean)
            );
        }

        #[test]
        fn from_lake_matches_lake_constructor() {
            assert_eq!(
                DiveEnvironment::from(Lake::Titicaca),
                DiveEnvironment::lake(Lake::Titicaca)
            );
        }

        #[test]
        fn default_is_standard() {
            assert_eq!(DiveEnvironment::default(), DiveEnvironment::standard());
        }
    }

    mod new {
        use super::*;

        #[test]
        fn valid_construction() -> Result<()> {
            let env = DiveEnvironment::new(Bar::new(1.013), MetersPerBar::new(9.95))?;

            assert_eq!(env.surface_pressure(), Bar::new(1.013));
            assert_eq!(env.water_density(), MetersPerBar::new(9.95));

            Ok(())
        }

        #[test]
        fn rejects_zero_surface_pressure() {
            assert!(matches!(
                DiveEnvironment::new(Bar::new(0.0), MetersPerBar::new(10.0)),
                Err(DiveEnvironmentError::SurfacePressureNotPositive(_))
            ));
        }

        #[test]
        fn rejects_negative_water_density() {
            assert!(matches!(
                DiveEnvironment::new(Bar::new(1.0), MetersPerBar::new(-1.0)),
                Err(DiveEnvironmentError::WaterDensityNotPositive(_))
            ));
        }

        #[test]
        fn rejects_nan_surface_pressure() {
            assert!(matches!(
                DiveEnvironment::new(Bar::new(f64::NAN), MetersPerBar::new(10.0)),
                Err(DiveEnvironmentError::SurfacePressureNotPositive(_))
            ));
        }
    }

    mod at_altitude {
        use super::*;

        #[test]
        fn reduces_surface_pressure() -> Result<()> {
            let high = DiveEnvironment::at_altitude(Meters::new(3_812.0))?;
            let sea = DiveEnvironment::standard();

            assert!(high.surface_pressure() < sea.surface_pressure());

            Ok(())
        }

        #[test]
        fn preserves_seawater_density() -> Result<()> {
            let high = DiveEnvironment::at_altitude(Meters::new(1_000.0))?;

            assert_eq!(
                high.water_density(),
                DiveEnvironment::standard().water_density()
            );

            Ok(())
        }

        #[test]
        fn freshwater_reduces_pressure_and_preserves_density() -> Result<()> {
            let alpine = DiveEnvironment::freshwater_at_altitude(Meters::new(1_000.0))?;
            let sea_level = DiveEnvironment::freshwater();

            assert!(alpine.surface_pressure() < sea_level.surface_pressure());
            assert_eq!(alpine.water_density(), sea_level.water_density());

            Ok(())
        }

        #[test]
        fn out_of_range_rejected() {
            assert!(matches!(
                DiveEnvironment::at_altitude(Meters::new(-1.0)),
                Err(DiveEnvironmentError::AltitudeOutOfRange(_))
            ));

            assert!(matches!(
                DiveEnvironment::at_altitude(Meters::new(9_000.0)),
                Err(DiveEnvironmentError::AltitudeOutOfRange(_))
            ));
        }

        #[test]
        fn nan_rejected() {
            assert!(matches!(
                DiveEnvironment::at_altitude(Meters::new(f64::NAN)),
                Err(DiveEnvironmentError::AltitudeOutOfRange(_))
            ));
        }
    }

    mod with_salinity {
        use super::*;

        #[test]
        fn constructs_valid_env() -> Result<()> {
            let brackish = DiveEnvironment::with_salinity(PartsPerThousand::new(10.0))?;

            assert_eq!(
                brackish.surface_pressure(),
                DiveEnvironment::standard().surface_pressure()
            );

            assert!(brackish.water_density() > DiveEnvironment::standard().water_density());
            assert!(brackish.water_density() < DiveEnvironment::freshwater().water_density());

            Ok(())
        }

        #[test]
        fn at_temperature_constructs_valid_env() -> Result<()> {
            let env = DiveEnvironment::with_salinity_at_temperature(
                PartsPerThousand::new(35.0),
                Celsius::new(15.0),
            )?;

            assert_relative_eq!(
                env.water_density(),
                DiveEnvironment::standard().water_density(),
                epsilon = 1e-9
            );

            Ok(())
        }

        #[test]
        fn out_of_range_rejected() {
            assert!(matches!(
                DiveEnvironment::with_salinity(PartsPerThousand::new(-1.0)),
                Err(DiveEnvironmentError::SalinityOutOfRange(_))
            ));

            assert!(matches!(
                DiveEnvironment::with_salinity(PartsPerThousand::new(400.0)),
                Err(DiveEnvironmentError::SalinityOutOfRange(_))
            ));
        }

        #[test]
        fn nan_rejected() {
            assert!(matches!(
                DiveEnvironment::with_salinity(PartsPerThousand::new(f64::NAN)),
                Err(DiveEnvironmentError::SalinityOutOfRange(_))
            ));
        }

        #[test]
        fn temperature_out_of_range_rejected() {
            assert!(matches!(
                DiveEnvironment::with_salinity_at_temperature(
                    PartsPerThousand::new(35.0),
                    Celsius::new(-5.0)
                ),
                Err(DiveEnvironmentError::TemperatureOutOfRange(_))
            ));

            assert!(matches!(
                DiveEnvironment::with_salinity_at_temperature(
                    PartsPerThousand::new(35.0),
                    Celsius::new(50.0)
                ),
                Err(DiveEnvironmentError::TemperatureOutOfRange(_))
            ));
        }

        #[test]
        fn temperature_nan_rejected() {
            assert!(matches!(
                DiveEnvironment::with_salinity_at_temperature(
                    PartsPerThousand::new(35.0),
                    Celsius::new(f64::NAN)
                ),
                Err(DiveEnvironmentError::TemperatureOutOfRange(_))
            ));
        }
    }

    mod builders {
        use super::*;

        #[test]
        fn with_altitude_overrides_pressure_only() -> Result<()> {
            let red_sea = DiveEnvironment::ocean(Ocean::RedSea);
            let elevated = red_sea.with_altitude(Meters::new(500.0))?;

            assert!(elevated.surface_pressure() < red_sea.surface_pressure());
            assert_relative_eq!(
                elevated.water_density(),
                red_sea.water_density(),
                epsilon = 1e-9
            );

            Ok(())
        }

        #[test]
        fn with_altitude_out_of_range_rejected() {
            assert!(matches!(
                DiveEnvironment::standard().with_altitude(Meters::new(-1.0)),
                Err(DiveEnvironmentError::AltitudeOutOfRange(_))
            ));
        }

        #[test]
        fn with_surface_pressure_overrides_pressure_only() -> Result<()> {
            let custom = DiveEnvironment::standard().with_surface_pressure(Bar::new(0.90))?;

            assert_eq!(custom.surface_pressure(), Bar::new(0.90));
            assert_eq!(
                custom.water_density(),
                DiveEnvironment::standard().water_density()
            );

            Ok(())
        }

        #[test]
        fn with_surface_pressure_zero_rejected() {
            assert!(matches!(
                DiveEnvironment::standard().with_surface_pressure(Bar::new(0.0)),
                Err(DiveEnvironmentError::SurfacePressureNotPositive(_))
            ));
        }

        #[test]
        fn with_water_density_overrides_density_only() -> Result<()> {
            let custom = DiveEnvironment::standard().with_water_density(MetersPerBar::new(10.2))?;

            assert_eq!(custom.water_density(), MetersPerBar::new(10.2));
            assert_eq!(
                custom.surface_pressure(),
                DiveEnvironment::standard().surface_pressure()
            );

            Ok(())
        }

        #[test]
        fn with_water_density_zero_rejected() {
            assert!(matches!(
                DiveEnvironment::standard().with_water_density(MetersPerBar::new(0.0)),
                Err(DiveEnvironmentError::WaterDensityNotPositive(_))
            ));
        }
    }

    mod depth_pressure {
        use super::*;

        #[test]
        fn absolute_pressure_at_surface_equals_surface_pressure() {
            let env = DiveEnvironment::standard();
            assert_eq!(
                env.absolute_pressure(Meters::new(0.0)),
                env.surface_pressure()
            );
        }

        #[test]
        fn absolute_pressure_increases_with_depth() {
            let env = DiveEnvironment::standard();

            assert!(
                env.absolute_pressure(Meters::new(30.0)) > env.absolute_pressure(Meters::new(10.0))
            );
        }

        #[test]
        fn depth_roundtrip_standard() {
            let env = DiveEnvironment::standard();
            let d = Meters::new(30.0);

            assert_relative_eq!(env.depth(env.absolute_pressure(d)), d, epsilon = 1e-9);
        }

        #[test]
        fn depth_roundtrip_freshwater_at_altitude() -> Result<()> {
            let env = DiveEnvironment::freshwater_at_altitude(Meters::new(2_000.0))?;
            let d = Meters::new(18.0);

            assert_relative_eq!(env.depth(env.absolute_pressure(d)), d, epsilon = 1e-9);

            Ok(())
        }

        #[test]
        fn depth_at_surface_pressure_is_zero() {
            let env = DiveEnvironment::standard();
            assert_eq!(env.depth(env.surface_pressure()), Meters::new(0.0));
        }

        #[test]
        fn depth_clamps_below_surface_pressure() {
            let env = DiveEnvironment::standard();
            assert_eq!(env.depth(Bar::new(0.5)), Meters::new(0.0));
        }
    }
}
