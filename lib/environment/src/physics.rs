//! Physical constants for water density and altitude pressure.
//!
//! These constants underpin all depth↔pressure calculations in
//! [`DiveEnvironment`](crate::DiveEnvironment). For instance,
//! the ICAO standard atmosphere constant fixes sea-level surface pressure at
//! exactly $\pu{1.01325 bar}$:
//!
//! ```
//! use dps_environment::DiveEnvironment;
//! use dps_units::Bar;
//! use approx::assert_relative_eq;
//!
//! assert_relative_eq!(
//!     DiveEnvironment::standard().surface_pressure(),
//!     Bar::new(1.013_25),
//!     epsilon = 1e-9,
//! );
//! ```

use dps_units::{Bar, Celsius, Meters, PartsPerThousand};

/// ISO standard atmosphere sea-level pressure.
pub const SEA_LEVEL_PRESSURE_BAR: Bar = Bar::new(1.013_25);

/// Conversion factor from pascals to bar ($\pu{100000 Pa} = \pu{1 bar}$).
pub const PA_PER_BAR: f64 = 1e5;

/// Standard acceleration due to gravity per ISO $\pu{80000-3 m/s^2}$.
pub const STANDARD_GRAVITY: f64 = 9.806_65;

/// ISO standard seawater density at $\pu{35 ‰} salinity, $\pu{15 ^\circ C}$, $\pu{0 dbar}$ (ISO 19901-7), in $\pu{kg/m^3}$.
pub const ISO_SEAWATER_DENSITY: f64 = 1025.0;

/// Pure-water baseline in the linear density approximation ρ(S,T) ≈ 1000 + 0.8S − 0.2T, in $\pu{kg/m^3}$.
pub const DENSITY_BASE: f64 = 1000.0;

/// Salinity coefficient in the linear density approximation, in $\pu{kg/(m^3 \times ‰)}$.
pub const DENSITY_SALINITY_COEFF: f64 = 0.8;

/// Temperature coefficient in the linear density approximation, in $\pu{kg/(m^3 \times \circ C)}$.
pub const DENSITY_TEMP_COEFF: f64 = -0.2;

/// ICAO ISA sea-level pressure used in the barometric altitude formula, in $\pu{Pa}$.
pub const ICAO_SEA_LEVEL_PA: f64 = 101_325.0;

/// Normalized temperature lapse rate L/T₀ in m⁻¹, where L = 0.0065 K/m and T₀ = 288.15 K.
pub const ICAO_TEMP_GRADIENT: f64 = 2.255_77e-5;

/// Barometric exponent g·M/(R·L) in the ICAO ISA formula (dimensionless).
pub const ICAO_PRESSURE_EXPONENT: f64 = 5.255_88;

/// Lower altitude bound (sea level).
pub const MIN_ALTITUDE: Meters = Meters::new(0.0);

/// Upper altitude bound (Mt Everest summit).
pub const MAX_ALTITUDE: Meters = Meters::new(8_849.0);

/// Lower salinity bound accepted by the density model (pure fresh water).
pub const MIN_SALINITY_PPT: PartsPerThousand = PartsPerThousand::new(0.0);

/// Upper salinity bound accepted by the density model.
pub const MAX_SALINITY_PPT: PartsPerThousand = PartsPerThousand::new(350.0);

/// Lower temperature bound accepted by the density model (seawater freezing point).
pub const MIN_TEMP_C: Celsius = Celsius::new(-2.0);

/// Upper temperature bound accepted by the density model.
pub const MAX_TEMP_C: Celsius = Celsius::new(40.0);

/// Default water temperature used for freshwater and salinity-only constructors.
pub const FRESHWATER_TEMP_C: Celsius = Celsius::new(20.0);
