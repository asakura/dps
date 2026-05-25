//! Depth↔pressure conversion for variable dive environments.
//!
//! Standard dive tables are built on two fixed constants: 1.013 25 bar at the
//! surface and roughly 10.0 m of seawater per bar of gauge pressure. That is
//! fine for a generic ocean dive, but the moment you leave those conditions —
//! high altitude, fresh water, the hypersaline Red Sea — both numbers shift,
//! and any deco calculation that ignores the shift is wrong.
//!
//! This module models those two variable parameters in [`DiveEnvironment`] and
//! provides the formulas and presets to compute them correctly.
//!
//! # The core equation
//!
//! Every depth↔pressure conversion in the library comes down to:
//!
//! ```text
//! P_abs = P_surface + depth / water_density
//! ```
//!
//! `P_surface` is the atmospheric pressure at the dive site (bar).
//! `water_density` is the column height that produces one bar of gauge
//! pressure, expressed as metres per bar. [`DiveEnvironment`] holds exactly
//! these two values — nothing more.
//!
//! # Water density
//!
//! The mass of a one-metre water column determines how quickly pressure builds
//! with depth. ISO standard seawater (35 ‰ salinity, 15 °C) has a density of
//! 1025 kg/m³, giving roughly 9.95 m/bar. Real conditions span 8 ‰ in the
//! Baltic to 41 ‰ in the Red Sea, and temperatures from 2 °C polar water to
//! 28 °C tropical reefs. Both shift density enough to affect deco margins:
//! salt makes water heavier, heat makes it lighter.
//!
//! Rather than the full TEOS-10 equation of state, this module uses a linear
//! approximation anchored at the ISO reference point:
//!
//! ```text
//! ρ(S, T) ≈ 1000 + 0.8 × S − 0.2 × T  [kg/m³]
//! ```
//!
//! At (35 ‰, 15 °C) this yields exactly 1025 kg/m³. Across all practical
//! dive conditions (S ∈ [0, 45] ‰, T ∈ [0, 35] °C) the error stays within
//! ±2 kg/m³ — smaller than the uncertainty budget of any real deco model.
//!
//! # Surface pressure and altitude
//!
//! Atmosphere thins with altitude. The ICAO International Standard Atmosphere
//! gives the pressure at height h as:
//!
//! ```text
//! P(h) = 101 325 × (1 − 2.255 77×10⁻⁵ × h)^5.255 88  [Pa]
//! ```
//!
//! At sea level this evaluates to 101 325 Pa (1.013 25 bar). At Lake Titicaca
//! (3 812 m) it drops to roughly 0.63 bar — a surface pressure that pushes a
//! diver well into altitude-adjustment territory before they hit the water.
//!
//! The two coefficients come from atmospheric physics: `2.255 77×10⁻⁵` is
//! L/T₀, the temperature lapse rate (0.0065 K/m) divided by ISA sea-level
//! temperature (288.15 K); `5.255 88` is g·M/(R·L), standard gravity times
//! molar mass of dry air divided by the universal gas constant times the lapse
//! rate.
//!
//! # Presets
//!
//! | Constructor | Salinity | Temperature | Altitude |
//! |---|---|---|---|
//! | [`DiveEnvironment::standard`] | 35 ‰ ISO seawater | 15 °C | sea level |
//! | [`DiveEnvironment::freshwater`] | 0 ‰ | 20 °C | sea level |
//! | [`DiveEnvironment::ocean`] | from [`Ocean`] variant | from [`Ocean`] variant | sea level |
//! | [`DiveEnvironment::lake`] | 0 ‰ | from [`Lake`] variant | from [`Lake`] variant |
//!
//! Seventeen [`Ocean`] variants cover the five major oceans and twelve popular
//! dive seas, each keyed to representative surface salinity and temperature.
//! Fifteen [`Lake`] variants span sea-level systems (Florida springs, Yucatán
//! cenotes) through extreme-altitude sites (Licancabur at 5 916 m, Ojos del
//! Salado at 6 390 m).
//!
//! For anything not covered by a preset, the fallible constructors
//! [`DiveEnvironment::new`], [`DiveEnvironment::at_altitude`], and
//! [`DiveEnvironment::with_salinity_at_temperature`] accept validated explicit
//! values. The builder methods [`DiveEnvironment::with_altitude`],
//! [`DiveEnvironment::with_surface_pressure`], and
//! [`DiveEnvironment::with_water_density`] let you adjust a preset in place.
//!
//! ```
//! use dps::environment::{DiveEnvironment, Ocean, Lake};
//! use dps::units::{Bar, MetersPerBar};
//!
//! // Named ocean preset at sea level
//! let red_sea = DiveEnvironment::ocean(Ocean::RedSea);
//!
//! // High-altitude freshwater lake
//! let titicaca = DiveEnvironment::lake(Lake::Titicaca);
//!
//! // Red Sea salinity at 500 m altitude — adjust a preset with a builder
//! let elevated = DiveEnvironment::ocean(Ocean::RedSea)
//!     .with_altitude(500.0)
//!     .unwrap();
//!
//! // Fully custom via validated constructor
//! let custom = DiveEnvironment::new(Bar::new(0.95), MetersPerBar::new(10.1)).unwrap();
//! ```

mod lake;
mod ocean;

pub use lake::Lake;
pub use ocean::Ocean;

use std::fmt;

use crate::units::{Bar, Meters, MetersPerBar};

/// ISO standard atmosphere sea-level pressure.
const SEA_LEVEL_PRESSURE_BAR: Bar = Bar::new(1.013_25);

/// Conversion factor from pascals to bar (100 000 Pa = 1 bar).
const PA_PER_BAR: f64 = 1e5;

/// Standard acceleration due to gravity per ISO 80000-3, in m/s².
const STANDARD_GRAVITY: f64 = 9.806_65;

/// ISO standard seawater density at 35 ‰ salinity, 15 °C, 0 dbar (ISO 19901-7), in kg/m³.
const ISO_SEAWATER_DENSITY: f64 = 1025.0;

/// Pure-water baseline in the linear density approximation ρ(S,T) ≈ 1000 + 0.8S − 0.2T, in kg/m³.
const DENSITY_BASE: f64 = 1000.0;

/// Salinity coefficient in the linear density approximation, in kg/(m³·‰).
const DENSITY_SALINITY_COEFF: f64 = 0.8;

/// Temperature coefficient in the linear density approximation, in kg/(m³·°C).
const DENSITY_TEMP_COEFF: f64 = -0.2;

/// ICAO ISA sea-level pressure used in the barometric altitude formula, in Pa.
const ICAO_SEA_LEVEL_PA: f64 = 101_325.0;

/// Normalized temperature lapse rate L/T₀ in m⁻¹, where L = 0.0065 K/m and T₀ = 288.15 K.
const ICAO_TEMP_GRADIENT: f64 = 2.255_77e-5;

/// Barometric exponent g·M/(R·L) in the ICAO ISA formula (dimensionless).
const ICAO_PRESSURE_EXPONENT: f64 = 5.255_88;

/// Upper altitude bound (Mt Everest summit), in metres.
const MAX_ALTITUDE_M: f64 = 8_849.0;

/// Upper salinity bound accepted by the density model, in ‰.
const MAX_SALINITY_PPT: f64 = 350.0;

/// Lower temperature bound accepted by the density model (seawater freezing point), in °C.
const MIN_TEMP_C: f64 = -2.0;

/// Upper temperature bound accepted by the density model, in °C.
const MAX_TEMP_C: f64 = 40.0;

/// Default water temperature used for freshwater and salinity-only constructors, in °C.
const FRESHWATER_TEMP_C: f64 = 20.0;

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

/// Error returned by fallible [`DiveEnvironment`] constructors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiveEnvironmentError {
    /// Surface pressure must be finite and positive.
    SurfacePressureNotPositive(Bar),
    /// Water density (m/bar) must be finite and positive.
    WaterDensityNotPositive(MetersPerBar),
    /// Altitude must be in \[0.0, 8 849.0\] m.
    AltitudeOutOfRange(f64),
    /// Salinity must be in \[0.0, 350.0\] ‰.
    SalinityOutOfRange(f64),
    /// Temperature must be in \[−2.0, 40.0\] °C.
    TemperatureOutOfRange(f64),
}

impl fmt::Display for DiveEnvironmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SurfacePressureNotPositive(p) => {
                write!(f, "surface pressure must be finite and positive, got {p}")
            }
            Self::WaterDensityNotPositive(d) => {
                write!(f, "water density must be finite and positive, got {d}")
            }
            Self::AltitudeOutOfRange(h) => {
                write!(f, "altitude {h} m is outside [0.0, 8 849.0] m")
            }
            Self::SalinityOutOfRange(s) => {
                write!(f, "salinity {s} ‰ is outside [0.0, 350.0] ‰")
            }
            Self::TemperatureOutOfRange(t) => {
                write!(f, "temperature {t} °C is outside [−2.0, 40.0] °C")
            }
        }
    }
}

impl std::error::Error for DiveEnvironmentError {}

impl DiveEnvironment {
    // Infallible presets

    /// ISO standard seawater at sea level: 35 ‰, 15 °C, 1.01325 bar.
    ///
    /// This is the baseline used by dive tables, certification agencies, and
    /// dive computers.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use approx::assert_relative_eq;
    ///
    /// let env = DiveEnvironment::standard();
    /// assert_relative_eq!(f64::from(env.surface_pressure()), 1.013_25, epsilon = 1e-9);
    /// assert_relative_eq!(
    ///     f64::from(env.water_density()),
    ///     1e5 / (1025.0 * 9.806_65),
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

    /// Fresh water at sea level: 0 ‰ salinity, 20 °C.
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
    pub const fn freshwater() -> Self {
        Self {
            surface_pressure: SEA_LEVEL_PRESSURE_BAR,
            water_density: water_density_from(0.0, FRESHWATER_TEMP_C),
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
    pub const fn ocean(ocean: Ocean) -> Self {
        Self {
            surface_pressure: SEA_LEVEL_PRESSURE_BAR,
            water_density: water_density_from(ocean.salinity_ppt(), ocean.typical_temperature_c()),
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
            surface_pressure: altitude_to_pressure_bar(lake.altitude_m()),
            water_density: water_density_from(0.0, lake.typical_temperature_c()),
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
        let sp = f64::from(surface_pressure);

        if !sp.is_positive_finite() {
            return Err(DiveEnvironmentError::SurfacePressureNotPositive(
                surface_pressure,
            ));
        }

        let wd = f64::from(water_density);

        if !wd.is_positive_finite() {
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
    /// uses ISO standard seawater (35 ‰, 15 °C).
    ///
    /// # Errors
    ///
    /// [`DiveEnvironmentError::AltitudeOutOfRange`] if `meters_asl` is outside
    /// `[0.0, 8 849.0]` or non-finite.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    ///
    /// let high = DiveEnvironment::at_altitude(1_000.0).unwrap();
    /// assert!(high.surface_pressure() < DiveEnvironment::standard().surface_pressure());
    /// // water density is unchanged — still ISO seawater
    /// assert_eq!(high.water_density(), DiveEnvironment::standard().water_density());
    /// ```
    pub fn at_altitude(meters_asl: f64) -> Result<Self, DiveEnvironmentError> {
        validate_altitude(meters_asl)?;

        Ok(Self {
            surface_pressure: altitude_to_pressure_bar(meters_asl),
            water_density: Self::standard().water_density,
        })
    }

    /// Freshwater environment at the given altitude above sea level.
    ///
    /// Surface pressure is derived via the ICAO barometric formula. Water density
    /// uses fresh water at 20 °C.
    ///
    /// # Errors
    ///
    /// [`DiveEnvironmentError::AltitudeOutOfRange`] if `meters_asl` is outside
    /// `[0.0, 8 849.0]` or non-finite.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    ///
    /// let alpine = DiveEnvironment::freshwater_at_altitude(1_000.0).unwrap();
    /// assert!(alpine.surface_pressure() < DiveEnvironment::standard().surface_pressure());
    /// // freshwater is less dense than seawater — more metres per bar
    /// assert!(alpine.water_density() > DiveEnvironment::standard().water_density());
    /// ```
    pub fn freshwater_at_altitude(meters_asl: f64) -> Result<Self, DiveEnvironmentError> {
        validate_altitude(meters_asl)?;

        Ok(Self {
            surface_pressure: altitude_to_pressure_bar(meters_asl),
            water_density: Self::freshwater().water_density,
        })
    }

    /// Sea-level environment for the given salinity at 20 °C.
    ///
    /// # Errors
    ///
    /// [`DiveEnvironmentError::SalinityOutOfRange`] if `salinity_ppt` is outside
    /// `[0.0, 350.0]` or non-finite.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    ///
    /// let brackish = DiveEnvironment::with_salinity(10.0).unwrap();
    /// // 10 ‰ is denser than fresh water but less dense than standard seawater
    /// assert!(brackish.water_density() < DiveEnvironment::freshwater().water_density());
    /// assert!(brackish.water_density() > DiveEnvironment::standard().water_density());
    /// ```
    pub fn with_salinity(salinity_ppt: f64) -> Result<Self, DiveEnvironmentError> {
        validate_salinity(salinity_ppt)?;

        Ok(Self {
            surface_pressure: SEA_LEVEL_PRESSURE_BAR,
            water_density: water_density_from(salinity_ppt, FRESHWATER_TEMP_C),
        })
    }

    /// Sea-level environment for the given salinity and water temperature.
    ///
    /// Passing `(35.0, 15.0)` reproduces the ISO standard seawater reference
    /// and gives the same water density as [`DiveEnvironment::standard`].
    ///
    /// # Errors
    ///
    /// - [`DiveEnvironmentError::SalinityOutOfRange`] if `salinity_ppt` is outside `[0.0, 350.0]`.
    /// - [`DiveEnvironmentError::TemperatureOutOfRange`] if `temp_c` is outside `[−2.0, 40.0]`.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use approx::assert_relative_eq;
    ///
    /// let iso = DiveEnvironment::with_salinity_at_temperature(35.0, 15.0).unwrap();
    /// assert_relative_eq!(
    ///     f64::from(iso.water_density()),
    ///     f64::from(DiveEnvironment::standard().water_density()),
    ///     epsilon = 1e-9
    /// );
    /// ```
    pub fn with_salinity_at_temperature(
        salinity_ppt: f64,
        temp_c: f64,
    ) -> Result<Self, DiveEnvironmentError> {
        validate_salinity(salinity_ppt)?;
        validate_temperature(temp_c)?;

        Ok(Self {
            surface_pressure: SEA_LEVEL_PRESSURE_BAR,
            water_density: water_density_from(salinity_ppt, temp_c),
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
    /// [`DiveEnvironmentError::AltitudeOutOfRange`] if `meters_asl` is outside
    /// `[0.0, 8 849.0]` or non-finite.
    ///
    /// ```
    /// use dps::environment::{DiveEnvironment, Ocean};
    ///
    /// let elevated = DiveEnvironment::ocean(Ocean::RedSea)
    ///     .with_altitude(500.0)
    ///     .unwrap();
    /// let sea_level = DiveEnvironment::ocean(Ocean::RedSea);
    ///
    /// assert!(elevated.surface_pressure() < sea_level.surface_pressure());
    /// // salinity-derived density is untouched
    /// assert_eq!(elevated.water_density(), sea_level.water_density());
    /// ```
    pub fn with_altitude(self, meters_asl: f64) -> Result<Self, DiveEnvironmentError> {
        validate_altitude(meters_asl)?;

        Ok(Self {
            surface_pressure: altitude_to_pressure_bar(meters_asl),
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
    pub fn with_surface_pressure(self, p: Bar) -> Result<Self, DiveEnvironmentError> {
        let v = f64::from(p);

        if !v.is_positive_finite() {
            return Err(DiveEnvironmentError::SurfacePressureNotPositive(p));
        }

        Ok(Self {
            surface_pressure: p,
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
    pub fn with_water_density(self, d: MetersPerBar) -> Result<Self, DiveEnvironmentError> {
        let v = f64::from(d);

        if !v.is_positive_finite() {
            return Err(DiveEnvironmentError::WaterDensityNotPositive(d));
        }

        Ok(Self {
            water_density: d,
            ..self
        })
    }

    // Accessors

    /// Surface pressure at the dive site.
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use approx::assert_relative_eq;
    ///
    /// // ISO sea-level pressure: 1.013 25 bar
    /// assert_relative_eq!(
    ///     f64::from(DiveEnvironment::standard().surface_pressure()),
    ///     1.013_25,
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
    /// use approx::assert_relative_eq;
    ///
    /// // ISO seawater (1025 kg/m³): ~9.948 m per bar
    /// assert_relative_eq!(
    ///     f64::from(DiveEnvironment::standard().water_density()),
    ///     1e5 / (1025.0 * 9.806_65),
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
    /// `P_abs = P_surface + depth / water_density`
    ///
    /// ```
    /// use dps::environment::DiveEnvironment;
    /// use dps::units::Meters;
    /// use approx::assert_relative_eq;
    ///
    /// let env = DiveEnvironment::standard();
    /// // At the surface, absolute pressure equals surface pressure.
    /// assert_relative_eq!(
    ///     f64::from(env.absolute_pressure(Meters::new(0.0))),
    ///     f64::from(env.surface_pressure()),
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
    /// `depth = (P_abs − P_surface) × water_density`
    ///
    /// Returns [`Meters::new(0.0)`] when `absolute_pressure ≤ surface_pressure`
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

trait PositiveFinite {
    fn is_positive_finite(&self) -> bool;
}

impl PositiveFinite for f64 {
    fn is_positive_finite(&self) -> bool {
        self.is_finite() && *self > 0.0
    }
}

/// Linear water density approximation valid for diving conditions.
///
/// ρ(S, T) ≈ 1000 + 0.8 × S − 0.2 × T  kg/m³
///
/// Anchored at the ISO standard seawater reference (35 ‰, 15 °C → 1025 kg/m³),
/// which is consistent with the hardcoded value used by [`DiveEnvironment::standard`].
/// Accuracy: within ±2 kg/m³ for S ∈ [0, 45] ‰ and T ∈ [0, 35] °C,
/// which covers all practical dive environments.
const fn density_kg_m3(salinity_ppt: f64, temp_c: f64) -> f64 {
    DENSITY_TEMP_COEFF.mul_add(
        temp_c,
        DENSITY_SALINITY_COEFF.mul_add(salinity_ppt, DENSITY_BASE),
    )
}

const fn water_density_from(salinity_ppt: f64, temp_c: f64) -> MetersPerBar {
    MetersPerBar::new(PA_PER_BAR / (density_kg_m3(salinity_ppt, temp_c) * STANDARD_GRAVITY))
}

/// ICAO barometric formula: P(h) = 101325 × (1 − 2.25577×10⁻⁵ × h)^5.25588 Pa
fn altitude_to_pressure_bar(meters_asl: f64) -> Bar {
    Bar::new(
        ICAO_SEA_LEVEL_PA
            * (ICAO_TEMP_GRADIENT.mul_add(-meters_asl, 1.0)).powf(ICAO_PRESSURE_EXPONENT)
            / PA_PER_BAR,
    )
}

fn validate_altitude(meters_asl: f64) -> Result<(), DiveEnvironmentError> {
    if !meters_asl.is_finite() || !(0.0..=MAX_ALTITUDE_M).contains(&meters_asl) {
        Err(DiveEnvironmentError::AltitudeOutOfRange(meters_asl))
    } else {
        Ok(())
    }
}

fn validate_salinity(salinity_ppt: f64) -> Result<(), DiveEnvironmentError> {
    if !salinity_ppt.is_finite() || !(0.0..=MAX_SALINITY_PPT).contains(&salinity_ppt) {
        Err(DiveEnvironmentError::SalinityOutOfRange(salinity_ppt))
    } else {
        Ok(())
    }
}

fn validate_temperature(temp_c: f64) -> Result<(), DiveEnvironmentError> {
    if !temp_c.is_finite() || !(MIN_TEMP_C..=MAX_TEMP_C).contains(&temp_c) {
        Err(DiveEnvironmentError::TemperatureOutOfRange(temp_c))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use color_eyre::Result;

    use crate::units::{Bar, MetersPerBar};

    #[test]
    fn standard_matches_legacy_constants() {
        let env = DiveEnvironment::standard();

        assert_relative_eq!(f64::from(env.surface_pressure()), 1.013_25, epsilon = 1e-6);
        assert_relative_eq!(
            f64::from(env.water_density()),
            1e5 / (1025.0 * 9.806_65),
            epsilon = 1e-6
        );
    }

    #[test]
    fn standard_and_iso_formula_agree() -> Result<()> {
        let iso = DiveEnvironment::with_salinity_at_temperature(35.0, 15.0)?;

        assert_relative_eq!(
            f64::from(iso.water_density()),
            f64::from(DiveEnvironment::standard().water_density()),
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
    fn altitude_reduces_surface_pressure() -> Result<()> {
        let high = DiveEnvironment::at_altitude(3_812.0)?;
        let sea = DiveEnvironment::standard();

        assert!(high.surface_pressure() < sea.surface_pressure());

        Ok(())
    }

    #[test]
    fn at_altitude_preserves_seawater_density() -> Result<()> {
        let high = DiveEnvironment::at_altitude(1_000.0)?;
        assert_eq!(
            high.water_density(),
            DiveEnvironment::standard().water_density()
        );
        Ok(())
    }

    #[test]
    fn freshwater_at_altitude_reduces_pressure_and_preserves_freshwater_density() -> Result<()> {
        let alpine = DiveEnvironment::freshwater_at_altitude(1_000.0)?;
        let sea_level = DiveEnvironment::freshwater();

        assert!(alpine.surface_pressure() < sea_level.surface_pressure());
        assert_eq!(alpine.water_density(), sea_level.water_density());

        Ok(())
    }

    #[test]
    fn titicaca_lake_preset_matches_freshwater_at_altitude() -> Result<()> {
        let preset = DiveEnvironment::lake(Lake::Titicaca);
        let manual = DiveEnvironment::freshwater_at_altitude(3_812.0)?;

        assert_relative_eq!(
            f64::from(preset.surface_pressure()),
            f64::from(manual.surface_pressure()),
            epsilon = 1e-6
        );

        Ok(())
    }

    #[test]
    fn with_salinity_constructs_valid_env() -> Result<()> {
        let brackish = DiveEnvironment::with_salinity(10.0)?;

        assert_eq!(
            brackish.surface_pressure(),
            DiveEnvironment::standard().surface_pressure()
        );
        // 10 ‰ less dense than standard — more m/bar
        assert!(brackish.water_density() > DiveEnvironment::standard().water_density());
        // 10 ‰ denser than freshwater — fewer m/bar
        assert!(brackish.water_density() < DiveEnvironment::freshwater().water_density());

        Ok(())
    }

    #[test]
    fn with_salinity_at_temperature_constructs_valid_env() -> Result<()> {
        let env = DiveEnvironment::with_salinity_at_temperature(35.0, 15.0)?;
        assert_relative_eq!(
            f64::from(env.water_density()),
            f64::from(DiveEnvironment::standard().water_density()),
            epsilon = 1e-9
        );
        Ok(())
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
    fn new_valid_construction() -> Result<()> {
        let env = DiveEnvironment::new(Bar::new(1.013), MetersPerBar::new(9.95))?;

        assert_eq!(env.surface_pressure(), Bar::new(1.013));
        assert_eq!(env.water_density(), MetersPerBar::new(9.95));

        Ok(())
    }

    #[test]
    fn new_rejects_zero_surface_pressure() {
        assert!(matches!(
            DiveEnvironment::new(Bar::new(0.0), MetersPerBar::new(10.0)),
            Err(DiveEnvironmentError::SurfacePressureNotPositive(_))
        ));
    }

    #[test]
    fn new_rejects_negative_water_density() {
        assert!(matches!(
            DiveEnvironment::new(Bar::new(1.0), MetersPerBar::new(-1.0)),
            Err(DiveEnvironmentError::WaterDensityNotPositive(_))
        ));
    }

    #[test]
    fn new_rejects_nan_surface_pressure() {
        assert!(matches!(
            DiveEnvironment::new(Bar::new(f64::NAN), MetersPerBar::new(10.0)),
            Err(DiveEnvironmentError::SurfacePressureNotPositive(_))
        ));
    }

    #[test]
    fn altitude_out_of_range_rejected() {
        assert!(matches!(
            DiveEnvironment::at_altitude(-1.0),
            Err(DiveEnvironmentError::AltitudeOutOfRange(_))
        ));

        assert!(matches!(
            DiveEnvironment::at_altitude(9_000.0),
            Err(DiveEnvironmentError::AltitudeOutOfRange(_))
        ));
    }

    #[test]
    fn altitude_nan_rejected() {
        assert!(matches!(
            DiveEnvironment::at_altitude(f64::NAN),
            Err(DiveEnvironmentError::AltitudeOutOfRange(_))
        ));
    }

    #[test]
    fn with_altitude_out_of_range_rejected() {
        assert!(matches!(
            DiveEnvironment::standard().with_altitude(-1.0),
            Err(DiveEnvironmentError::AltitudeOutOfRange(_))
        ));
    }

    #[test]
    fn with_surface_pressure_zero_rejected() {
        assert!(matches!(
            DiveEnvironment::standard().with_surface_pressure(Bar::new(0.0)),
            Err(DiveEnvironmentError::SurfacePressureNotPositive(_))
        ));
    }

    #[test]
    fn with_water_density_zero_rejected() {
        assert!(matches!(
            DiveEnvironment::standard().with_water_density(MetersPerBar::new(0.0)),
            Err(DiveEnvironmentError::WaterDensityNotPositive(_))
        ));
    }

    #[test]
    fn salinity_out_of_range_rejected() {
        assert!(matches!(
            DiveEnvironment::with_salinity(-1.0),
            Err(DiveEnvironmentError::SalinityOutOfRange(_))
        ));

        assert!(matches!(
            DiveEnvironment::with_salinity(400.0),
            Err(DiveEnvironmentError::SalinityOutOfRange(_))
        ));
    }

    #[test]
    fn salinity_nan_rejected() {
        assert!(matches!(
            DiveEnvironment::with_salinity(f64::NAN),
            Err(DiveEnvironmentError::SalinityOutOfRange(_))
        ));
    }

    #[test]
    fn temperature_out_of_range_rejected() {
        assert!(matches!(
            DiveEnvironment::with_salinity_at_temperature(35.0, -5.0),
            Err(DiveEnvironmentError::TemperatureOutOfRange(_))
        ));

        assert!(matches!(
            DiveEnvironment::with_salinity_at_temperature(35.0, 50.0),
            Err(DiveEnvironmentError::TemperatureOutOfRange(_))
        ));
    }

    #[test]
    fn temperature_nan_rejected() {
        assert!(matches!(
            DiveEnvironment::with_salinity_at_temperature(35.0, f64::NAN),
            Err(DiveEnvironmentError::TemperatureOutOfRange(_))
        ));
    }

    #[test]
    fn with_altitude_overrides_pressure_only() -> Result<()> {
        let red_sea = DiveEnvironment::ocean(Ocean::RedSea);
        let elevated = red_sea.with_altitude(500.0)?;

        assert!(elevated.surface_pressure() < red_sea.surface_pressure());

        assert_relative_eq!(
            f64::from(elevated.water_density()),
            f64::from(red_sea.water_density()),
            epsilon = 1e-9
        );

        Ok(())
    }

    #[test]
    fn from_ocean_matches_ocean_constructor() {
        let a = DiveEnvironment::from(Ocean::Caribbean);
        let b = DiveEnvironment::ocean(Ocean::Caribbean);

        assert_eq!(a, b);
    }

    #[test]
    fn from_lake_matches_lake_constructor() {
        let a = DiveEnvironment::from(Lake::Titicaca);
        let b = DiveEnvironment::lake(Lake::Titicaca);

        assert_eq!(a, b);
    }

    #[test]
    fn default_is_standard() {
        assert_eq!(DiveEnvironment::default(), DiveEnvironment::standard());
    }

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
        let env = DiveEnvironment::freshwater_at_altitude(2_000.0)?;
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
