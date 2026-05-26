//! Newtype wrappers for physical units used in dive calculations.

mod bar;
mod celsius;
mod cns_rate_per_minute;
mod grams_per_litre;
mod meters;
mod meters_per_bar;
mod otu_per_minute;
mod parts_per_thousand;
mod percent;

pub use bar::Bar;
pub use celsius::Celsius;
pub use cns_rate_per_minute::CnsRatePerMinute;
pub use grams_per_litre::GramsPerLitre;
pub use meters::Meters;
pub use meters_per_bar::MetersPerBar;
pub use otu_per_minute::OTUPerMinute;
pub use parts_per_thousand::PartsPerThousand;
pub use percent::Percent;

use std::ops::{Div, Mul};

/// Generates standard impls for a newtype unit struct backed by f64.
///
/// Provides: `new`, `max`, `Display`, `From<f64>`, `From<T> for f64`,
/// `Add`, `Sub`, `Neg`, `Mul<f64>`, `Div<f64>`, `Mul<T> for f64`, `Div` (ratio),
/// `Mul<Percent>`, and `Div<Percent>`.
#[doc(hidden)]
#[macro_export]
macro_rules! unit_newtype {
    ($ty:ident, $suffix:literal) => {
        impl $ty {
            /// Constructs a value from a raw `f64`.
            pub const fn new(val: f64) -> Self {
                Self(val)
            }

            /// Returns the greater of two values.
            #[must_use]
            pub const fn max(self, other: Self) -> Self {
                Self(self.0.max(other.0))
            }

            /// Computes `self * scalar + addend` with a single rounding.
            #[must_use]
            pub const fn mul_add(self, scalar: f64, addend: Self) -> Self {
                Self(self.0.mul_add(scalar, addend.0))
            }

            /// Returns `true` if the underlying value is finite (not infinite or `NaN`).
            #[must_use]
            pub const fn is_finite(self) -> bool {
                self.0.is_finite()
            }

            /// Returns `true` if the underlying value is strictly positive (`> 0`) and finite.
            #[must_use]
            pub fn is_positive_finite(self) -> bool {
                self.0 > 0.0 && self.0.is_finite()
            }

            /// Returns `true` if this value falls within `range`.
            ///
            /// Accepts any standard range expression.
            #[must_use]
            pub fn contains(self, range: impl ::std::ops::RangeBounds<Self>) -> bool {
                range.contains(&self)
            }
        }

        impl ::std::fmt::Display for $ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{:.1} {}", self.0, $suffix)
            }
        }

        impl ::std::convert::From<f64> for $ty {
            fn from(v: f64) -> Self {
                Self(v)
            }
        }

        impl ::std::convert::From<$ty> for f64 {
            fn from(v: $ty) -> Self {
                v.0
            }
        }

        impl ::std::ops::Add for $ty {
            type Output = Self;

            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }

        impl ::std::ops::Sub for $ty {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }

        impl ::std::ops::Mul<f64> for $ty {
            type Output = Self;

            fn mul(self, rhs: f64) -> Self {
                Self(self.0 * rhs)
            }
        }

        impl ::std::ops::Div<f64> for $ty {
            type Output = Self;

            fn div(self, rhs: f64) -> Self {
                Self(self.0 / rhs)
            }
        }

        impl ::std::ops::Mul<$ty> for f64 {
            type Output = $ty;

            fn mul(self, rhs: $ty) -> $ty {
                $ty(self * rhs.0)
            }
        }

        impl ::std::ops::Div for $ty {
            type Output = f64;

            fn div(self, rhs: Self) -> f64 {
                self.0 / rhs.0
            }
        }

        impl ::std::ops::Neg for $ty {
            type Output = Self;

            fn neg(self) -> Self {
                Self(-self.0)
            }
        }

        impl ::std::ops::Mul<$crate::units::Percent> for $ty {
            type Output = Self;

            fn mul(self, rhs: $crate::units::Percent) -> Self {
                Self(self.0 * f64::from(rhs))
            }
        }

        impl ::std::ops::Div<$crate::units::Percent> for $ty {
            type Output = Self;

            fn div(self, rhs: $crate::units::Percent) -> Self {
                Self(self.0 / f64::from(rhs))
            }
        }

        impl ::approx::AbsDiffEq for $ty {
            type Epsilon = f64;

            fn default_epsilon() -> f64 {
                f64::default_epsilon()
            }

            fn abs_diff_eq(&self, other: &Self, epsilon: f64) -> bool {
                self.0.abs_diff_eq(&other.0, epsilon)
            }
        }

        impl ::approx::RelativeEq for $ty {
            fn default_max_relative() -> f64 {
                f64::default_max_relative()
            }

            fn relative_eq(&self, other: &Self, epsilon: f64, max_relative: f64) -> bool {
                self.0.relative_eq(&other.0, epsilon, max_relative)
            }
        }

        #[cfg(test)]
        mod unit_newtype_tests {
            use super::*;

            #[test]
            fn display() {
                assert_eq!($ty::new(10.0).to_string(), concat!("10.0 ", $suffix));
            }

            #[test]
            fn from_f64() {
                ::approx::assert_relative_eq!($ty::from(5.0_f64), $ty::new(5.0));
            }

            #[test]
            fn f64_from() {
                ::approx::assert_relative_eq!(f64::from($ty::new(5.0)), 5.0);
            }

            #[test]
            fn f64_mul() {
                ::approx::assert_relative_eq!(2.0_f64 * $ty::new(5.0), $ty::new(10.0));
            }

            #[test]
            fn ratio_div() {
                let ratio: f64 = $ty::new(10.0) / $ty::new(2.0);
                ::approx::assert_relative_eq!(ratio, 5.0);
            }

            #[test]
            fn neg() {
                ::approx::assert_relative_eq!(-$ty::new(5.0), $ty::new(-5.0));
            }

            #[test]
            fn is_finite_true() {
                assert!($ty::new(5.0).is_finite());
            }

            #[test]
            fn is_finite_false() {
                assert!(!$ty::new(f64::INFINITY).is_finite());
                assert!(!$ty::new(f64::NAN).is_finite());
            }

            #[test]
            fn is_positive_finite_true() {
                assert!($ty::new(0.001).is_positive_finite());
            }

            #[test]
            fn is_positive_finite_false() {
                assert!(!$ty::new(0.0).is_positive_finite());
                assert!(!$ty::new(-1.0).is_positive_finite());
                assert!(!$ty::new(f64::INFINITY).is_positive_finite());
            }

            #[test]
            fn contains_inclusive_range() {
                let v = $ty::new(5.0);
                assert!(v.contains($ty::new(0.0)..=$ty::new(10.0)));
                assert!(v.contains($ty::new(5.0)..=$ty::new(5.0)));
                assert!(!v.contains($ty::new(6.0)..=$ty::new(10.0)));
            }

            #[test]
            fn contains_exclusive_range() {
                let v = $ty::new(5.0);
                assert!(v.contains($ty::new(0.0)..$ty::new(10.0)));
                assert!(!v.contains($ty::new(0.0)..$ty::new(5.0)));
            }
        }
    };
}

/// Meters / `MetersPerBar` → Bar  (depth → gauge pressure)
///
/// ```no_run
/// use dps::units::{Bar, Meters, MetersPerBar};
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
/// ```no_run
/// use dps::units::{Bar, Meters, MetersPerBar};
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

    #[test]
    fn meters_div_meters_per_bar_gives_bar() {
        let gauge: Bar = Meters::new(30.0) / MetersPerBar::new(10.0);
        assert_relative_eq!(gauge, Bar::new(3.0));
    }

    #[test]
    fn bar_mul_meters_per_bar_gives_meters() {
        let depth: Meters = Bar::new(3.0) * MetersPerBar::new(10.0);
        assert_relative_eq!(depth, Meters::new(30.0));
    }
}
