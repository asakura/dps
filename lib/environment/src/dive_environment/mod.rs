//! The [`DiveEnvironment`] struct: dive-site parameters for depth↔pressure conversion.
//!
//! Holds two values — surface pressure (varies with altitude) and water density
//! (varies with salinity and temperature) — from which all depth↔pressure calculations
//! are derived.
//!
//! Implements [`Display`](std::fmt::Display) and [`FromStr`](std::str::FromStr) for
//! clipboard round-trip serialisation used by the register system.
//!
//! Constructors fall into three families:
//!
//! - **Infallible presets** — [`DiveEnvironment::standard`], [`DiveEnvironment::freshwater`],
//!   [`DiveEnvironment::ocean`], [`DiveEnvironment::lake`]: cover the most common dive
//!   environments without error handling.
//! - **Fallible constructors** — [`DiveEnvironment::new`], [`DiveEnvironment::at_altitude`],
//!   [`DiveEnvironment::with_salinity`], and friends: validate their inputs and return
//!   [`DiveEnvironmentError`](crate::DiveEnvironmentError) for out-of-range values.
//! - **Builder refinements** — [`DiveEnvironment::with_altitude`],
//!   [`DiveEnvironment::with_surface_pressure`], [`DiveEnvironment::with_water_density`]:
//!   modify one field on an existing environment.
//!
//! ```
//! use dps_environment::{DiveEnvironment, Ocean, Lake};
//!
//! // Named preset: Red Sea at sea level
//! let env = DiveEnvironment::ocean(Ocean::RedSea);
//! assert!(env.water_density() < DiveEnvironment::standard().water_density());
//!
//! // High-altitude freshwater lake — lower surface pressure, lower mass density (more m/bar)
//! let alpine = DiveEnvironment::lake(Lake::Titicaca);
//! assert!(alpine.surface_pressure() < DiveEnvironment::standard().surface_pressure());
//! assert!(alpine.water_density() > DiveEnvironment::standard().water_density());
//! ```
//!
//! Two formulas underpin the calculations:
//!
//! - **Water density** — a linear approximation ($\rho \approx 1000 + 0.8S - 0.2T$) anchored
//!   at the ISO 19901-7 reference ($\pu{35 ‰}$, $\pu{15 ^\circ C}$ → $\pu{1025 kg/m^3}$),
//!   accurate to ${\pm}\pu{2 kg/m^3}$ across all practical diving conditions.
//! - **Altitude pressure** — the ICAO ISA barometric formula, valid for altitudes up to
//!   $\pu{8849 m}$ (Mt Everest summit).

mod default;
mod display;
mod error;
mod from_impls;
mod from_str;

pub use self::error::{DiveEnvironmentError, ParseDiveEnvironmentError};
use super::physics::{
    DENSITY_BASE, DENSITY_SALINITY_COEFF, DENSITY_TEMP_COEFF, FRESHWATER_TEMP_C,
    ICAO_PRESSURE_EXPONENT, ICAO_SEA_LEVEL_PA, ICAO_TEMP_GRADIENT, ISO_SEAWATER_DENSITY,
    MAX_ALTITUDE, MAX_SALINITY_PPT, MAX_TEMP_C, MIN_TEMP_C, PA_PER_BAR, SEA_LEVEL_PRESSURE_BAR,
    STANDARD_GRAVITY,
};
use super::{Lake, Ocean};

use dps_units::{Bar, Celsius, Meters, MetersPerBar, PartsPerThousand};

/// Dive environment parameters for depth↔pressure conversion.
///
/// Encapsulates surface pressure (varies with altitude) and water density
/// (varies with salinity and temperature). All [`dps_gas::EANxBlend`]
/// calculations use these values instead of fixed constants.
///
/// Use [`DiveEnvironment::standard`] for typical sea-level saltwater diving,
/// or one of the other constructors for altitude or freshwater environments.
/// Attach to a blend with [`dps_gas::EANxBlend::with_environment`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::{Bar, MetersPerBar};
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
    /// use dps_environment::DiveEnvironment;
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
    /// use dps_environment::{DiveEnvironment, Ocean};
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
    /// use dps_environment::{DiveEnvironment, Lake};
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
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::{Bar, MetersPerBar};
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
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::Meters;
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
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::Meters;
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
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::PartsPerThousand;
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
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::{Celsius, PartsPerThousand};
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
    /// use dps_environment::{DiveEnvironment, Ocean};
    /// use dps_units::Meters;
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
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::Bar;
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
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::MetersPerBar;
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
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::Bar;
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
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::MetersPerBar;
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
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::Meters;
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
    /// Returns [`Meters::new(0.0)`](dps_units::Meters) when `absolute_pressure ≤ surface_pressure`
    /// (at or above the surface).
    ///
    /// ```
    /// use dps_environment::DiveEnvironment;
    /// use dps_units::Meters;
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

/// Linear water density approximation valid for diving conditions.
///
/// $$
/// \rho(S, T) \approx 1000 + 0.8 \times S - 0.2 \times T \; [\text{kg/m}^3]
/// $$
///
/// Anchored at the ISO standard seawater reference ($\pu{35 ‰}$, $\pu{15 ^\circ C}$ → $\pu{1025 kg/m^3}$),
/// which is consistent with the hardcoded value used by [`super::DiveEnvironment::standard`].
/// Accuracy: within ${\pm}\pu{2 kg/m^3}$ for $S \in [\pu{0 ‰}, \pu{45 ‰}]$ and $T \in [\pu{0 ^\circ C}, \pu{35 ^\circ C}]$,
/// which covers all practical dive environments.
///
/// # Examples
///
/// ```ignore
/// use dps_units::{Celsius, PartsPerThousand};
///
/// // ISO reference point: 35 ‰ salinity, 15 °C → exactly 1025 kg/m³
/// assert_eq!(
///     density_kg_m3(PartsPerThousand::new(35.0), Celsius::new(15.0)),
///     1025.0,
/// );
///
/// // Fresh water at 20 °C → 996 kg/m³
/// assert_eq!(
///     density_kg_m3(PartsPerThousand::new(0.0), Celsius::new(20.0)),
///     996.0,
/// );
/// ```
fn density_kg_m3(salinity: PartsPerThousand, temperature: Celsius) -> f64 {
    DENSITY_TEMP_COEFF.mul_add(
        temperature.into(),
        DENSITY_SALINITY_COEFF.mul_add(salinity.into(), DENSITY_BASE),
    )
}

/// Converts salinity and temperature to the water-column height that equals one bar of pressure.
///
/// Divides the Pa→bar conversion factor by the product of [`density_kg_m3`] and standard gravity
/// to produce a [`MetersPerBar`] value — the depth change corresponding to one bar of gauge
/// pressure in this water body. Denser water (higher salinity, lower temperature) produces a
/// smaller value; lighter water produces a larger one.
///
/// # Examples
///
/// ```ignore
/// use approx::assert_relative_eq;
/// use dps_units::{MetersPerBar, PartsPerThousand, Celsius};
///
/// // ISO standard seawater (35 ‰, 15 °C) → ≈ 9.948 m/bar
/// let seawater = water_density_from(PartsPerThousand::new(35.0), Celsius::new(15.0));
/// assert_relative_eq!(seawater, MetersPerBar::new(9.948), max_relative = 1e-3);
///
/// // Fresh water (0 ‰, 20 °C) → ≈ 10.239 m/bar — less dense, more metres per bar
/// let fresh = water_density_from(PartsPerThousand::new(0.0), Celsius::new(20.0));
/// assert!(fresh > seawater);
/// ```
fn water_density_from(salinity: PartsPerThousand, temperature: Celsius) -> MetersPerBar {
    MetersPerBar::new(PA_PER_BAR / (density_kg_m3(salinity, temperature) * STANDARD_GRAVITY))
}

/// Converts altitude above sea level to atmospheric pressure using the ICAO ISA barometric formula.
///
/// $$
/// P(h) = 101325 \times \bigl(1 - 2.25577 \times 10^{-5} \cdot h\bigr)^{5.25588} \; [\text{Pa}]
/// $$
///
/// Valid for $h \in [\pu{0 m}, \pu{8849 m}]$ (sea level to Mt Everest summit). At sea level the
/// result is exactly $\pu{1.01325 bar}$; at $\pu{3812 m}$ (Lake Titicaca) it drops to roughly
/// $\pu{0.632 bar}$.
///
/// # Examples
///
/// ```ignore
/// # use approx::assert_relative_eq;
/// use dps_units::{Bar, Meters};
///
/// // Sea level → exactly 1.01325 bar
/// assert_relative_eq!(
///     altitude_to_pressure_bar(Meters::new(0.0)),
///     Bar::new(1.013_25),
///     max_relative = 1e-6,
/// );
///
/// // Lake Titicaca (3812 m) → ≈ 0.632 bar
/// assert_relative_eq!(
///     altitude_to_pressure_bar(Meters::new(3812.0)),
///     Bar::new(0.632),
///     max_relative = 5e-3,
/// );
/// ```
fn altitude_to_pressure_bar(altitude: Meters) -> Bar {
    Bar::new(
        ICAO_SEA_LEVEL_PA
            * (ICAO_TEMP_GRADIENT.mul_add(-f64::from(altitude), 1.0)).powf(ICAO_PRESSURE_EXPONENT)
            / PA_PER_BAR,
    )
}

#[cfg(test)]
mod tests {
    use super::DiveEnvironment;

    use crate::{DiveEnvironmentError, Lake, Ocean};
    use dps_units::{Bar, Celsius, Meters, MetersPerBar, PartsPerThousand};

    use approx::assert_relative_eq;
    use rstest::rstest;

    use core::assert_matches;

    mod presets {
        use super::*;

        #[rstest]
        fn standard_matches_legacy_constants() {
            let env = DiveEnvironment::standard();

            assert_relative_eq!(env.surface_pressure(), Bar::new(1.013_25), epsilon = 1e-6);
            assert_relative_eq!(
                env.water_density(),
                MetersPerBar::new(1e5 / (1025.0 * 9.806_65)),
                epsilon = 1e-6
            );
        }

        #[rstest]
        fn standard_and_iso_formula_agree() -> Result<(), DiveEnvironmentError> {
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

        #[rstest]
        fn freshwater_is_less_dense_than_seawater() {
            let fresh = DiveEnvironment::freshwater();
            let salt = DiveEnvironment::standard();

            assert!(fresh.water_density() > salt.water_density());
        }

        #[rstest]
        fn red_sea_is_denser_than_standard() {
            let red_sea = DiveEnvironment::ocean(Ocean::RedSea);
            let std = DiveEnvironment::standard();

            assert!(red_sea.water_density() < std.water_density());
        }

        #[rstest]
        fn titicaca_matches_freshwater_at_altitude() -> Result<(), DiveEnvironmentError> {
            let preset = DiveEnvironment::lake(Lake::Titicaca);
            let manual = DiveEnvironment::freshwater_at_altitude(Meters::new(3_812.0))?;

            assert_relative_eq!(
                preset.surface_pressure(),
                manual.surface_pressure(),
                epsilon = 1e-6
            );

            Ok(())
        }

        #[rstest]
        fn from_ocean_matches_ocean_constructor() {
            assert_eq!(
                DiveEnvironment::from(Ocean::Caribbean),
                DiveEnvironment::ocean(Ocean::Caribbean)
            );
        }

        #[rstest]
        fn from_lake_matches_lake_constructor() {
            assert_eq!(
                DiveEnvironment::from(Lake::Titicaca),
                DiveEnvironment::lake(Lake::Titicaca)
            );
        }

        #[rstest]
        fn default_is_standard() {
            assert_eq!(DiveEnvironment::default(), DiveEnvironment::standard());
        }
    }

    mod new {
        use super::*;

        #[rstest]
        fn valid_construction() -> Result<(), DiveEnvironmentError> {
            let env = DiveEnvironment::new(Bar::new(1.013), MetersPerBar::new(9.95))?;

            assert_eq!(env.surface_pressure(), Bar::new(1.013));
            assert_eq!(env.water_density(), MetersPerBar::new(9.95));

            Ok(())
        }

        #[rstest]
        fn rejects_zero_surface_pressure() {
            assert_matches!(
                DiveEnvironment::new(Bar::new(0.0), MetersPerBar::new(10.0)),
                Err(DiveEnvironmentError::SurfacePressureNotPositive(_))
            );
        }

        #[rstest]
        fn rejects_negative_water_density() {
            assert_matches!(
                DiveEnvironment::new(Bar::new(1.0), MetersPerBar::new(-1.0)),
                Err(DiveEnvironmentError::WaterDensityNotPositive(_))
            );
        }

        #[rstest]
        fn rejects_nan_surface_pressure() {
            assert_matches!(
                DiveEnvironment::new(Bar::new(f64::NAN), MetersPerBar::new(10.0)),
                Err(DiveEnvironmentError::SurfacePressureNotPositive(_))
            );
        }

        #[rstest]
        fn rejects_nan_water_density() {
            assert_matches!(
                DiveEnvironment::new(Bar::new(1.0), MetersPerBar::new(f64::NAN)),
                Err(DiveEnvironmentError::WaterDensityNotPositive(_))
            );
        }
    }

    mod at_altitude {
        use super::*;

        #[rstest]
        fn reduces_surface_pressure() -> Result<(), DiveEnvironmentError> {
            let high = DiveEnvironment::at_altitude(Meters::new(3_812.0))?;
            let sea = DiveEnvironment::standard();

            assert!(high.surface_pressure() < sea.surface_pressure());

            Ok(())
        }

        #[rstest]
        fn preserves_seawater_density() -> Result<(), DiveEnvironmentError> {
            let high = DiveEnvironment::at_altitude(Meters::new(1_000.0))?;

            assert_eq!(
                high.water_density(),
                DiveEnvironment::standard().water_density()
            );

            Ok(())
        }

        #[rstest]
        fn freshwater_reduces_pressure_and_preserves_density() -> Result<(), DiveEnvironmentError> {
            let alpine = DiveEnvironment::freshwater_at_altitude(Meters::new(1_000.0))?;
            let sea_level = DiveEnvironment::freshwater();

            assert!(alpine.surface_pressure() < sea_level.surface_pressure());
            assert_eq!(alpine.water_density(), sea_level.water_density());

            Ok(())
        }

        #[rstest]
        fn out_of_range_rejected() {
            assert_matches!(
                DiveEnvironment::at_altitude(Meters::new(-1.0)),
                Err(DiveEnvironmentError::AltitudeOutOfRange(_))
            );

            assert_matches!(
                DiveEnvironment::at_altitude(Meters::new(9_000.0)),
                Err(DiveEnvironmentError::AltitudeOutOfRange(_))
            );
        }

        #[rstest]
        fn nan_rejected() {
            assert_matches!(
                DiveEnvironment::at_altitude(Meters::new(f64::NAN)),
                Err(DiveEnvironmentError::AltitudeOutOfRange(_))
            );
        }
    }

    mod with_salinity {
        use super::*;

        #[rstest]
        fn constructs_valid_env() -> Result<(), DiveEnvironmentError> {
            let brackish = DiveEnvironment::with_salinity(PartsPerThousand::new(10.0))?;

            assert_eq!(
                brackish.surface_pressure(),
                DiveEnvironment::standard().surface_pressure()
            );

            assert!(brackish.water_density() > DiveEnvironment::standard().water_density());
            assert!(brackish.water_density() < DiveEnvironment::freshwater().water_density());

            Ok(())
        }

        #[rstest]
        fn at_temperature_constructs_valid_env() -> Result<(), DiveEnvironmentError> {
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

        #[rstest]
        fn out_of_range_rejected() {
            assert_matches!(
                DiveEnvironment::with_salinity(PartsPerThousand::new(-1.0)),
                Err(DiveEnvironmentError::SalinityOutOfRange(_))
            );

            assert_matches!(
                DiveEnvironment::with_salinity(PartsPerThousand::new(400.0)),
                Err(DiveEnvironmentError::SalinityOutOfRange(_))
            );
        }

        #[rstest]
        fn nan_rejected() {
            assert_matches!(
                DiveEnvironment::with_salinity(PartsPerThousand::new(f64::NAN)),
                Err(DiveEnvironmentError::SalinityOutOfRange(_))
            );
        }

        #[rstest]
        fn temperature_out_of_range_rejected() {
            assert_matches!(
                DiveEnvironment::with_salinity_at_temperature(
                    PartsPerThousand::new(35.0),
                    Celsius::new(-5.0)
                ),
                Err(DiveEnvironmentError::TemperatureOutOfRange(_))
            );

            assert_matches!(
                DiveEnvironment::with_salinity_at_temperature(
                    PartsPerThousand::new(35.0),
                    Celsius::new(50.0)
                ),
                Err(DiveEnvironmentError::TemperatureOutOfRange(_))
            );
        }

        #[rstest]
        fn temperature_nan_rejected() {
            assert_matches!(
                DiveEnvironment::with_salinity_at_temperature(
                    PartsPerThousand::new(35.0),
                    Celsius::new(f64::NAN)
                ),
                Err(DiveEnvironmentError::TemperatureOutOfRange(_))
            );
        }
    }

    mod builders {
        use super::*;

        #[rstest]
        fn with_altitude_overrides_pressure_only() -> Result<(), DiveEnvironmentError> {
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

        #[rstest]
        fn with_altitude_out_of_range_rejected() {
            assert_matches!(
                DiveEnvironment::standard().with_altitude(Meters::new(-1.0)),
                Err(DiveEnvironmentError::AltitudeOutOfRange(_))
            );
        }

        #[rstest]
        fn with_surface_pressure_overrides_pressure_only() -> Result<(), DiveEnvironmentError> {
            let custom = DiveEnvironment::standard().with_surface_pressure(Bar::new(0.90))?;

            assert_eq!(custom.surface_pressure(), Bar::new(0.90));
            assert_eq!(
                custom.water_density(),
                DiveEnvironment::standard().water_density()
            );

            Ok(())
        }

        #[rstest]
        fn with_surface_pressure_zero_rejected() {
            assert_matches!(
                DiveEnvironment::standard().with_surface_pressure(Bar::new(0.0)),
                Err(DiveEnvironmentError::SurfacePressureNotPositive(_))
            );
        }

        #[rstest]
        fn with_water_density_overrides_density_only() -> Result<(), DiveEnvironmentError> {
            let custom = DiveEnvironment::standard().with_water_density(MetersPerBar::new(10.2))?;

            assert_eq!(custom.water_density(), MetersPerBar::new(10.2));
            assert_eq!(
                custom.surface_pressure(),
                DiveEnvironment::standard().surface_pressure()
            );

            Ok(())
        }

        #[rstest]
        fn with_water_density_zero_rejected() {
            assert_matches!(
                DiveEnvironment::standard().with_water_density(MetersPerBar::new(0.0)),
                Err(DiveEnvironmentError::WaterDensityNotPositive(_))
            );
        }
    }

    mod depth_pressure {
        use super::*;

        #[rstest]
        fn absolute_pressure_at_surface_equals_surface_pressure() {
            let env = DiveEnvironment::standard();
            assert_eq!(
                env.absolute_pressure(Meters::new(0.0)),
                env.surface_pressure()
            );
        }

        #[rstest]
        fn absolute_pressure_increases_with_depth() {
            let env = DiveEnvironment::standard();

            assert!(
                env.absolute_pressure(Meters::new(30.0)) > env.absolute_pressure(Meters::new(10.0))
            );
        }

        #[rstest]
        fn depth_roundtrip_standard() {
            let env = DiveEnvironment::standard();
            let d = Meters::new(30.0);

            assert_relative_eq!(env.depth(env.absolute_pressure(d)), d, epsilon = 1e-9);
        }

        #[rstest]
        fn depth_roundtrip_freshwater_at_altitude() -> Result<(), DiveEnvironmentError> {
            let env = DiveEnvironment::freshwater_at_altitude(Meters::new(2_000.0))?;
            let d = Meters::new(18.0);

            assert_relative_eq!(env.depth(env.absolute_pressure(d)), d, epsilon = 1e-9);

            Ok(())
        }

        #[rstest]
        fn depth_at_surface_pressure_is_zero() {
            let env = DiveEnvironment::standard();
            assert_eq!(env.depth(env.surface_pressure()), Meters::new(0.0));
        }

        #[rstest]
        fn depth_clamps_below_surface_pressure() {
            let env = DiveEnvironment::standard();
            assert_eq!(env.depth(Bar::new(0.5)), Meters::new(0.0));
        }
    }
}
