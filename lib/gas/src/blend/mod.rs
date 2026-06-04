mod membrane;
mod partial_pressure;
mod psa;

mod sealed {
    pub trait Sealed {}
}

pub use self::membrane::{InvalidMembraneFractionsError, Membrane};
pub use self::partial_pressure::PartialPressure;
pub use self::psa::Psa;
use super::components::GasComponents;

use std::fmt;

/// Describes how a nitrox mix was blended, determining the full gas composition
/// from the $\ce{O2}$ fraction.
///
/// # Implementations
///
/// Three concrete types implement this trait:
///
/// | Type | Description |
/// |------|-------------|
/// | [`PartialPressure`] | Bank gases mixed by partial pressure — canonical nitrox blending |
/// | [`Psa`] | Pressure-swing adsorption unit — adds residual Ar, caps $\ce{O2}$ at $\approx 95.7\\%$ |
/// | [`Membrane`] | Hollow-fibre membrane unit — arbitrary Ar/N₂ residuals from bench analysis |
///
/// # Sealed trait
///
/// This trait is **sealed**: it cannot be implemented outside this crate.
/// The supertrait `sealed::Sealed` is private, so the compiler rejects any
/// external `impl BlendMethod for MyType`.
///
/// The seal is intentional. Each blend method encodes a physical gas model
/// (residual inert fractions, production ceilings, component ratios) that
/// feeds into CNS/OTU oxygen-toxicity and nitrogen-narcosis calculations.
/// Allowing arbitrary external implementations would make it impossible to
/// audit those models for correctness and physical plausibility as a unit.
/// If you need a blend method not covered here, open an issue or PR so it
/// can be validated against the rest of the crate.
pub trait BlendMethod: sealed::Sealed + Copy + fmt::Debug {
    /// Full gas composition for a mix with the given $\ce{O2}$ fraction.
    ///
    /// ```no_run
    /// use dps_gas::{BlendMethod, PartialPressure};
    /// # use approx::assert_relative_eq;
    /// let c = PartialPressure.components(0.32);
    /// assert_relative_eq!(c.sum(), 1.0, epsilon = 1e-12);
    /// assert_relative_eq!(c.o2(), 0.32, epsilon = 1e-9);
    /// ```
    fn components(&self, fo2: f64) -> GasComponents;

    /// Short human-readable name for this blend method.
    ///
    /// ```no_run
    /// use dps_gas::{BlendMethod, PartialPressure, Psa};
    /// assert_eq!(PartialPressure.blend_name(), "partial pressure");
    /// assert_eq!(Psa.blend_name(), "PSA");
    /// ```
    fn blend_name(&self) -> &'static str;

    /// Returns `true` if `fo2` is physically achievable with this blend method.
    ///
    /// Defaults to `true`; overridden by [`Psa`] to enforce the $\approx 95.7\\%$ ceiling.
    ///
    /// ```no_run
    /// use dps_gas::{BlendMethod, PartialPressure, Psa};
    /// // PartialPressure has no physical ceiling
    /// assert!(PartialPressure.is_valid_fo2(0.99));
    /// // Psa cannot produce mixes above ~95.7% O₂
    /// assert!(!Psa.is_valid_fo2(0.99));
    /// assert!(Psa.is_valid_fo2(0.40));
    /// ```
    fn is_valid_fo2(&self, _fo2: f64) -> bool {
        true
    }
}
