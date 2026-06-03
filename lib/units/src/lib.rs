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
//! - [`Bar`]: Pressure, used for both ambient and tank pressure.
//! - [`Meters`]: Depth and distance.
//! - [`Percent`]: Fractional proportions (e.g., gas mix fractions).
//! - [`Celsius`]: Temperature.
//!
//! ## Derived Units
//!
//! - [`MetersPerBar`]: Depth-to-pressure conversion factor (water density).
//! - [`CnsRatePerMinute`]: CNS O₂ toxicity accumulation rate.
//! - [`OTUPerMinute`]: Oxygen Tolerance Unit accumulation rate.
//! - [`GramsPerLitre`]: Gas density.
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

/// Generates standard impls for a newtype unit struct backed by f64.
///
/// Provides: `new`, `max`, `Display`, `FromStr`, `From<f64>`, `From<T> for f64`,
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

            /// Returns the underlying `f64` value.
            ///
            /// # Warning
            ///
            /// This method returns a unitless value, bypassing type safety. Use only when
            /// strictly necessary for external API compatibility.
            #[deprecated(
                since = "0.1.0",
                note = "returns unitless value; use only for external API compatibility"
            )]
            pub const fn as_f64(self) -> f64 {
                self.0
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

        impl ::std::str::FromStr for $ty {
            type Err = $crate::UnitError;

            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                let num_str = s
                    .strip_suffix(::std::concat!(" ", $suffix))
                    .ok_or_else(|| {
                        $crate::UnitError::Parse($crate::error::ParseError::$ty(s.to_owned()))
                    })?;
                let val: f64 = num_str.parse().map_err(|_| {
                    $crate::UnitError::Parse($crate::error::ParseError::$ty(num_str.to_owned()))
                })?;
                ::std::result::Result::Ok(Self(val))
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

        impl ::std::default::Default for $ty {
            fn default() -> Self {
                Self(0.0)
            }
        }

        impl ::std::iter::Sum for $ty {
            fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                Self(iter.map(|v| v.0).sum())
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

        impl ::std::ops::Mul<$crate::Percent> for $ty {
            type Output = Self;

            fn mul(self, rhs: $crate::Percent) -> Self {
                Self(self.0 * f64::from(rhs))
            }
        }

        impl ::std::ops::Div<$crate::Percent> for $ty {
            type Output = Self;

            fn div(self, rhs: $crate::Percent) -> Self {
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
            use ::rstest::rstest;

            use super::*;

            mod is_finite {
                use super::*;

                #[rstest]
                fn finite_value() {
                    assert!($ty::new(5.0).is_finite());
                }

                #[rstest]
                fn infinity_is_not_finite() {
                    assert!(!$ty::new(f64::INFINITY).is_finite());
                }

                #[rstest]
                fn nan_is_not_finite() {
                    assert!(!$ty::new(f64::NAN).is_finite());
                }
            }

            mod is_positive_finite {
                use super::*;

                #[rstest]
                fn small_positive_value() {
                    assert!($ty::new(0.001).is_positive_finite());
                }

                #[rstest]
                fn zero_is_not_positive_finite() {
                    assert!(!$ty::new(0.0).is_positive_finite());
                }

                #[rstest]
                fn negative_is_not_positive_finite() {
                    assert!(!$ty::new(-1.0).is_positive_finite());
                }

                #[rstest]
                fn infinity_is_not_positive_finite() {
                    assert!(!$ty::new(f64::INFINITY).is_positive_finite());
                }
            }

            mod contains {
                use super::*;

                #[rstest]
                fn inclusive_range_contains_interior() {
                    assert!($ty::new(5.0).contains($ty::new(0.0)..=$ty::new(10.0)));
                }

                #[rstest]
                fn inclusive_range_contains_boundary() {
                    assert!($ty::new(5.0).contains($ty::new(5.0)..=$ty::new(5.0)));
                }

                #[rstest]
                fn inclusive_range_rejects_exterior() {
                    assert!(!$ty::new(5.0).contains($ty::new(6.0)..=$ty::new(10.0)));
                }

                #[rstest]
                fn exclusive_range_contains_interior() {
                    assert!($ty::new(5.0).contains($ty::new(0.0)..$ty::new(10.0)));
                }

                #[rstest]
                fn exclusive_range_rejects_boundary() {
                    assert!(!$ty::new(5.0).contains($ty::new(0.0)..$ty::new(5.0)));
                }
            }

            mod display {
                use super::*;

                #[rstest]
                fn formats_with_suffix() {
                    assert_eq!($ty::new(10.0).to_string(), concat!("10.0 ", $suffix));
                }
            }

            mod from_str {
                use super::*;

                #[rstest]
                fn roundtrip() -> Result<(), $crate::UnitError> {
                    let v = $ty::new(1.5);

                    assert_eq!(v.to_string().parse::<$ty>()?, v);

                    ::std::result::Result::Ok(())
                }

                #[rstest]
                #[case("1.5", "1.5")]
                #[case("", "")]
                fn missing_suffix_reports_full_input(#[case] input: &str, #[case] expected: &str) {
                    assert_eq!(
                        input.parse::<$ty>(),
                        Err($crate::UnitError::Parse($crate::error::ParseError::$ty(
                            expected.to_owned()
                        ))),
                    );
                }

                #[rstest]
                fn non_numeric_reports_numeric_part() {
                    let input = ::std::concat!("abc ", $suffix);

                    assert_eq!(
                        input.parse::<$ty>(),
                        Err($crate::UnitError::Parse($crate::error::ParseError::$ty(
                            "abc".to_owned()
                        ))),
                    );
                }
            }

            mod from {
                use super::*;

                #[rstest]
                fn from_f64() {
                    ::approx::assert_relative_eq!($ty::from(5.0_f64), $ty::new(5.0));
                }

                #[rstest]
                fn f64_from() {
                    ::approx::assert_relative_eq!(f64::from($ty::new(5.0)), 5.0);
                }
            }

            mod default {
                use super::*;

                #[rstest]
                fn default_is_zero() {
                    assert_eq!($ty::default(), $ty::new(0.0));
                }
            }

            mod sum {
                use super::*;

                #[rstest]
                fn sums_iterator() {
                    let vals = vec![$ty::new(1.0), $ty::new(2.0), $ty::new(3.0)];
                    assert_eq!(vals.into_iter().sum::<$ty>(), $ty::new(6.0));
                }
            }

            mod mul {
                use super::*;

                #[rstest]
                fn f64_mul() {
                    ::approx::assert_relative_eq!(2.0_f64 * $ty::new(5.0), $ty::new(10.0));
                }
            }

            mod div {
                use super::*;

                #[rstest]
                fn ratio_div() {
                    let ratio: f64 = $ty::new(10.0) / $ty::new(2.0);
                    ::approx::assert_relative_eq!(ratio, 5.0);
                }
            }

            mod neg {
                use super::*;

                #[rstest]
                fn negates_value() {
                    ::approx::assert_relative_eq!(-$ty::new(5.0), $ty::new(-5.0));
                }
            }
        }
    };
}

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
