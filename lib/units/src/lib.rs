#![cfg_attr(
    test,
    expect(
        clippy::panic_in_result_fn,
        reason = "Tests legitimately combine Result return types with panic-inducing assertions"
    )
)]
#![allow(
    rustdoc::private_doc_tests,
    reason = "Module-level doc examples reference crate paths that are private to rustdoc"
)]

//! Newtype wrappers for physical units used in dive calculations.
//!
//! This crate provides a collection of newtype wrappers around `f64` to provide
//! type safety and prevent accidental mixing of different units.
//!
//! ## Core Units
//!
//! - [`Bar`][]: Pressure, used for both ambient and tank pressure.
//! - [`Meters`][]: Depth and distance.
//! - [`Percent`][]: Fractional proportions (e.g., gas mix fractions).
//! - [`Celsius`][]: Temperature.
//!
//! ## Derived Units
//!
//! - [`MetersPerBar`][]: Depth-to-pressure conversion factor (water density).
//! - [`CnsRatePerMinute`][]: CNS O₂ toxicity accumulation rate.
//! - [`OTUPerMinute`][]: Oxygen Tolerance Unit accumulation rate.
//! - [`GramsPerLitre`][]: Gas density.
//!
//! ## Unit Interactions
//!
//! The units are designed to work together through standard operator
//! implementations. For example, dividing a depth by a conversion factor
//! yields pressure:
//!
//! ```
//! use dps_units::{Bar, Meters, MetersPerBar};
//!
//! let depth = Meters::new(30.0);
//! let density = MetersPerBar::new(10.0); // Seawater
//! let gauge_pressure: Bar = depth / density;
//!
//! assert_eq!(gauge_pressure, Bar::new(3.0));
//! ```
//!
//! Similarly, multiplying pressure by the conversion factor yields depth:
//!
//! ```
//! use dps_units::{Bar, Meters, MetersPerBar};
//!
//! let gauge_pressure = Bar::new(3.0);
//! let density = MetersPerBar::new(10.0);
//! let depth: Meters = gauge_pressure * density;
//!
//! assert_eq!(depth, Meters::new(30.0));
//! ```

mod bar;
mod celsius;
mod cns_rate_per_minute;
mod error;
mod grams_per_litre;
mod macroses;
mod meters;
mod meters_per_bar;
mod otu_per_minute;
mod parts_per_thousand;
mod percent;

pub use self::bar::Bar;
pub use self::celsius::Celsius;
pub use self::cns_rate_per_minute::CnsRatePerMinute;
pub use self::error::Error as UnitError;
pub use self::grams_per_litre::GramsPerLitre;
pub use self::meters::Meters;
pub use self::meters_per_bar::MetersPerBar;
pub use self::otu_per_minute::OTUPerMinute;
pub use self::parts_per_thousand::PartsPerThousand;
pub use self::percent::Percent;

use std::ops::{Div, Mul};

/// Meters / `MetersPerBar` → Bar  (depth → gauge pressure)
///
/// ```
/// use dps_units::{Bar, Meters, MetersPerBar};
/// let gauge: Bar = Meters::new(30.0) / MetersPerBar::new(10.0);
/// assert_eq!(gauge, Bar::new(3.0));
/// ```
impl Div<MetersPerBar> for Meters {
    type Output = Bar;

    fn div(self, rhs: MetersPerBar) -> Bar {
        Bar::new(f64::from(self) / f64::from(rhs))
    }
}

/// Bar × `MetersPerBar` → Meters  (gauge pressure → depth)
///
/// ```
/// use dps_units::{Bar, Meters, MetersPerBar};
/// let depth: Meters = Bar::new(3.0) * MetersPerBar::new(10.0);
/// assert_eq!(depth, Meters::new(30.0));
/// ```
impl Mul<MetersPerBar> for Bar {
    type Output = Meters;

    fn mul(self, rhs: MetersPerBar) -> Meters {
        Meters::new(f64::from(self) * f64::from(rhs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use approx::assert_relative_eq;
    use rstest::rstest;

    #[rstest]
    fn meters_div_meters_per_bar_gives_bar() {
        let gauge: Bar = Meters::new(30.0) / MetersPerBar::new(10.0);
        assert_relative_eq!(gauge, Bar::new(3.0));
    }

    #[rstest]
    fn bar_mul_meters_per_bar_gives_meters() {
        let depth: Meters = Bar::new(3.0) * MetersPerBar::new(10.0);
        assert_relative_eq!(depth, Meters::new(30.0));
    }
}
