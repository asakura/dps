//! `unit_newtype!` macro: constructors, `Display`/`FromStr`, and method impls for unit newtypes.
//!
//! ```
//! use dps_units::Meters;
//! let d = Meters::new(30.0);
//! assert_eq!(d.to_string(), "30.0 m");
//! assert_eq!("30.0 m".parse::<Meters>().unwrap(), d);
//! ```

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

            /// Returns a lossless string representation suitable for copy-pasting.
            ///
            /// Guaranteed to roundtrip perfectly via [`FromStr`](std::str::FromStr).
            #[must_use]
            pub fn to_clipboard_string(&self) -> String {
                let mut buffer = ::ryu::Buffer::new();
                ::std::format!("{} {}", buffer.format(self.0), $suffix)
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
                ::std::write!(f, "{:.1} {}", self.0, $suffix)
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

        $crate::unit_newtype_common!($ty);

        $crate::impl_percent_scaling!($ty);

        #[cfg(test)]
        mod unit_newtype_tests {
            use super::*;

            use ::rstest::rstest;

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
                fn roundtrip_clipboard() -> ::std::result::Result<(), $crate::UnitError> {
                    let v = $ty::new(1.555);

                    // clipboard string must be bit-perfect
                    assert_eq!(v.to_clipboard_string().parse::<$ty>()?, v);

                    ::std::result::Result::Ok(())
                }

                #[rstest]
                fn roundtrip_simple() -> ::std::result::Result<(), $crate::UnitError> {
                    let v = $ty::new(3.0);
                    assert_eq!(v.to_string().parse::<$ty>()?, v);
                    ::std::result::Result::Ok(())
                }

                #[rstest]
                fn display_is_lossy_for_precision() {
                    let v = $ty::new(1.55);
                    assert_eq!(v.to_string(), ::std::concat!("1.6 ", $suffix));
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
        }
    };
    (
        $ty:ident,
        bounds = $min:literal..=$max:literal,
        to_clipboard_string = $clipboard_expr:expr,
        display = $display_expr:expr,
        from_str = $from_str_expr:expr
    ) => {
        impl $ty {
            /// Constructs a value from a raw `f64`.
            ///
            /// # Errors
            ///
            /// Returns [`UnitError::OutOfRange`] if `val` is outside the valid range.
            pub const fn new(val: f64) -> ::std::result::Result<Self, $crate::UnitError> {
                if val >= $min && val <= $max {
                    ::std::result::Result::Ok(Self(val))
                } else {
                    ::std::result::Result::Err($crate::UnitError::OutOfRange(val))
                }
            }

            /// Constructs a value from a compile-time-known fraction, panicking if
            /// out of range.
            ///
            /// Intended exclusively for `const` items where the value is a literal
            /// guaranteed to lie in the valid range. For runtime construction, prefer
            /// `new`.
            ///
            /// # Panics
            ///
            /// Panics if `val` is outside the valid range. When the call site is a
            /// `const` item the compiler evaluates this at compile time, turning an
            /// out-of-range value into a **compile-time error**.
            #[must_use]
            pub const fn literal(val: f64) -> Self {
                if val >= $min && val <= $max {
                    Self(val)
                } else {
                    panic!(::std::concat!(
                        ::std::stringify!($ty),
                        " value is outside valid range"
                    ))
                }
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

            /// Returns a lossless string representation suitable for copy-pasting.
            ///
            /// Guaranteed to roundtrip perfectly via [`FromStr`](std::str::FromStr).
            #[must_use]
            pub fn to_clipboard_string(&self) -> String {
                let formatter: fn(&$ty) -> String = $clipboard_expr;
                formatter(self)
            }
        }

        impl ::std::fmt::Display for $ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                let formatter: fn(&$ty, &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result =
                    $display_expr;
                formatter(self, f)
            }
        }

        impl ::std::str::FromStr for $ty {
            type Err = $crate::UnitError;

            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                let parser: fn(&str) -> ::std::result::Result<Self, Self::Err> = $from_str_expr;
                parser(s)
            }
        }

        $crate::unit_newtype_common!($ty);

        $crate::impl_percent_scaling!($ty);

        #[cfg(test)]
        mod unit_newtype_bounded_tests {
            use super::*;

            use ::rstest::rstest;

            mod new {
                use super::*;

                #[rstest]
                fn rejects_out_of_range() {
                    assert!($ty::new($min - 0.1).is_err());
                    assert!($ty::new($max + 0.1).is_err());
                }

                #[rstest]
                fn accepts_in_range() {
                    assert!($ty::new($min).is_ok());
                    assert!($ty::new($max).is_ok());
                    assert!($ty::new(($min + $max) / 2.0).is_ok());
                }
            }

            mod literal {
                use super::*;

                #[rstest]
                fn accepts_in_range() {
                    let _ = $ty::literal($min);
                    let _ = $ty::literal($max);
                }
            }
        }
    };
}
