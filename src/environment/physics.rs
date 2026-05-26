//! Physical constants and computation helpers for water density and altitude pressure.
//!
//! All items are `pub(super)` — implementation details of the `environment` module,
//! not part of the public API.
//!
//! Two formulas are implemented here:
//!
//! - A linear water-density approximation anchored at the ISO 19901-7 reference
//!   point ($\pu{35 ‰}$, $\pu{15 ^\circ C}$, $\pu{1025 kg/m^3}$), accurate to
//!   ${\pm}\pu{2 kg/m^3}$ across all practical diving conditions.
//! - The ICAO International Standard Atmosphere barometric formula, valid for
//!   altitudes up to $\pu{8849 m}$ (Mt Everest summit).

use crate::units::{Bar, Celsius, Meters, MetersPerBar, PartsPerThousand};

/// ISO standard atmosphere sea-level pressure.
pub(super) const SEA_LEVEL_PRESSURE_BAR: Bar = Bar::new(1.013_25);

/// Conversion factor from pascals to bar (100 000 Pa = 1 bar).
pub(super) const PA_PER_BAR: f64 = 1e5;

/// Standard acceleration due to gravity per ISO 80000-3, in m/s².
pub(super) const STANDARD_GRAVITY: f64 = 9.806_65;

/// ISO standard seawater density at 35 ‰ salinity, 15 °C, 0 dbar (ISO 19901-7), in kg/m³.
pub(super) const ISO_SEAWATER_DENSITY: f64 = 1025.0;

/// Pure-water baseline in the linear density approximation ρ(S,T) ≈ 1000 + 0.8S − 0.2T, in kg/m³.
pub(super) const DENSITY_BASE: f64 = 1000.0;

/// Salinity coefficient in the linear density approximation, in kg/(m³·‰).
pub(super) const DENSITY_SALINITY_COEFF: f64 = 0.8;

/// Temperature coefficient in the linear density approximation, in kg/(m³·°C).
pub(super) const DENSITY_TEMP_COEFF: f64 = -0.2;

/// ICAO ISA sea-level pressure used in the barometric altitude formula, in Pa.
pub(super) const ICAO_SEA_LEVEL_PA: f64 = 101_325.0;

/// Normalized temperature lapse rate L/T₀ in m⁻¹, where L = 0.0065 K/m and T₀ = 288.15 K.
pub(super) const ICAO_TEMP_GRADIENT: f64 = 2.255_77e-5;

/// Barometric exponent g·M/(R·L) in the ICAO ISA formula (dimensionless).
pub(super) const ICAO_PRESSURE_EXPONENT: f64 = 5.255_88;

/// Upper altitude bound (Mt Everest summit).
pub(super) const MAX_ALTITUDE: Meters = Meters::new(8_849.0);

/// Upper salinity bound accepted by the density model.
pub(super) const MAX_SALINITY_PPT: PartsPerThousand = PartsPerThousand::new(350.0);

/// Lower temperature bound accepted by the density model (seawater freezing point).
pub(super) const MIN_TEMP_C: Celsius = Celsius::new(-2.0);

/// Upper temperature bound accepted by the density model.
pub(super) const MAX_TEMP_C: Celsius = Celsius::new(40.0);

/// Default water temperature used for freshwater and salinity-only constructors.
pub(super) const FRESHWATER_TEMP_C: Celsius = Celsius::new(20.0);

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
pub(super) fn density_kg_m3(salinity: PartsPerThousand, temperature: Celsius) -> f64 {
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
///
/// // ISO standard seawater (35 ‰, 15 °C) → ≈ 9.948 m/bar
/// let seawater = water_density_from(PartsPerThousand::new(35.0), Celsius::new(15.0));
/// assert_relative_eq!(seawater, MetersPerBar::new(9.948), max_relative = 1e-3);
///
/// // Fresh water (0 ‰, 20 °C) → ≈ 10.239 m/bar — less dense, more metres per bar
/// let fresh = water_density_from(PartsPerThousand::new(0.0), Celsius::new(20.0));
/// assert!(fresh > seawater);
/// ```
pub(super) fn water_density_from(salinity: PartsPerThousand, temperature: Celsius) -> MetersPerBar {
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
/// use approx::assert_relative_eq;
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
pub(super) fn altitude_to_pressure_bar(altitude: Meters) -> Bar {
    Bar::new(
        ICAO_SEA_LEVEL_PA
            * (ICAO_TEMP_GRADIENT.mul_add(-f64::from(altitude), 1.0)).powf(ICAO_PRESSURE_EXPONENT)
            / PA_PER_BAR,
    )
}
