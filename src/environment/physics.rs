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

use crate::units::{Bar, MetersPerBar};

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

/// Upper altitude bound (Mt Everest summit), in metres.
pub(super) const MAX_ALTITUDE_M: f64 = 8_849.0;

/// Upper salinity bound accepted by the density model, in ‰.
pub(super) const MAX_SALINITY_PPT: f64 = 350.0;

/// Lower temperature bound accepted by the density model (seawater freezing point), in °C.
pub(super) const MIN_TEMP_C: f64 = -2.0;

/// Upper temperature bound accepted by the density model, in °C.
pub(super) const MAX_TEMP_C: f64 = 40.0;

/// Default water temperature used for freshwater and salinity-only constructors, in °C.
pub(super) const FRESHWATER_TEMP_C: f64 = 20.0;

pub(super) trait PositiveFinite {
    fn is_positive_finite(&self) -> bool;
}

impl PositiveFinite for f64 {
    fn is_positive_finite(&self) -> bool {
        self.is_finite() && *self > 0.0
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
/// Accuracy: within ${\pm}\pu{2 kg/m^3}$ for $S \in [0, 45]\ \text{‰}$ and $T \in [0, 35]\ ^\circ\text{C}$,
/// which covers all practical dive environments.
pub(super) const fn density_kg_m3(salinity_ppt: f64, temp_c: f64) -> f64 {
    DENSITY_TEMP_COEFF.mul_add(
        temp_c,
        DENSITY_SALINITY_COEFF.mul_add(salinity_ppt, DENSITY_BASE),
    )
}

pub(super) const fn water_density_from(salinity_ppt: f64, temp_c: f64) -> MetersPerBar {
    MetersPerBar::new(PA_PER_BAR / (density_kg_m3(salinity_ppt, temp_c) * STANDARD_GRAVITY))
}

/// ICAO barometric formula: $P(h) = 101325 \times (1 - 2.25577 \times 10^{-5} \cdot h)^{5.25588}\ \text{Pa}$
pub(super) fn altitude_to_pressure_bar(meters_asl: f64) -> Bar {
    Bar::new(
        ICAO_SEA_LEVEL_PA
            * (ICAO_TEMP_GRADIENT.mul_add(-meters_asl, 1.0)).powf(ICAO_PRESSURE_EXPONENT)
            / PA_PER_BAR,
    )
}
