use std::fmt;

use super::components::GasComponents;

mod sealed {
    pub trait Sealed {}
}

mod membrane;
mod partial_pressure;
mod psa;

pub use membrane::{InvalidMembraneFractionsError, Membrane};
pub use partial_pressure::PartialPressure;
pub use psa::Psa;

/// Describes how a nitrox mix was blended, determining the full gas composition
/// from the O₂ fraction.
///
/// Sealed: only [`PartialPressure`], [`Psa`], and [`Membrane`] are valid.
pub trait BlendMethod: sealed::Sealed + Copy + fmt::Debug {
    /// Full gas composition for a mix with the given O₂ fraction.
    ///
    /// ```no_run
    /// use dps::gas::{BlendMethod, PartialPressure};
    /// # use approx::assert_relative_eq;
    /// let c = PartialPressure.components(0.32);
    /// assert_relative_eq!(c.sum(), 1.0, epsilon = 1e-12);
    /// assert_relative_eq!(c.o2(), 0.32, epsilon = 1e-9);
    /// ```
    fn components(&self, fo2: f64) -> GasComponents;

    /// Short human-readable name for this blend method.
    ///
    /// ```no_run
    /// use dps::gas::{BlendMethod, PartialPressure, Psa};
    /// assert_eq!(PartialPressure.blend_name(), "partial pressure");
    /// assert_eq!(Psa.blend_name(), "PSA");
    /// ```
    fn blend_name(&self) -> &'static str;

    /// Returns `true` if `fo2` is physically achievable with this blend method.
    ///
    /// Defaults to `true`; overridden by [`Psa`] to enforce the ~95.7 % ceiling.
    ///
    /// ```no_run
    /// use dps::gas::{BlendMethod, PartialPressure, Psa};
    /// // PartialPressure has no physical ceiling
    /// assert!(PartialPressure.is_valid_fo2(0.99));
    /// // Psa cannot produce mixes above ~95.7 % O₂
    /// assert!(!Psa.is_valid_fo2(0.99));
    /// assert!(Psa.is_valid_fo2(0.40));
    /// ```
    fn is_valid_fo2(&self, _fo2: f64) -> bool {
        true
    }
}
