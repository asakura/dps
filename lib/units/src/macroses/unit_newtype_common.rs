//! `unit_newtype_common!` macro: `From`, arithmetic, `Default`, `Sum`, and `approx` impls.
//!
//! ```
//! use dps_units::Bar;
//! use approx::assert_relative_eq;
//! let a = Bar::new(3.0) + Bar::new(2.0);
//! assert_relative_eq!(a, Bar::new(5.0));
//! ```

/// Generates trait impls common to all units.
#[doc(hidden)]
#[macro_export]
macro_rules! unit_newtype_common {
    ($ty:ident) => {
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
        mod unit_newtype_common_tests {
            use super::*;

            use ::rstest::rstest;

            mod from {
                use super::*;

                #[rstest]
                fn from_f64() {
                    ::approx::assert_relative_eq!($ty::from(5.0_f64), $ty::from(5.0));
                }

                #[rstest]
                fn f64_from() {
                    ::approx::assert_relative_eq!(f64::from($ty::from(5.0)), 5.0);
                }
            }

            mod default {
                use super::*;

                #[rstest]
                fn default_is_zero() {
                    assert_eq!($ty::default(), $ty::from(0.0));
                }
            }

            mod sum {
                use super::*;

                #[rstest]
                fn sums_iterator() {
                    let vals = vec![$ty::from(1.0), $ty::from(2.0), $ty::from(3.0)];
                    assert_eq!(vals.into_iter().sum::<$ty>(), $ty::from(6.0));
                }
            }

            mod add {
                use super::*;

                #[rstest]
                fn adds_values() {
                    ::approx::assert_relative_eq!($ty::from(1.0) + $ty::from(2.0), $ty::from(3.0));
                }
            }

            mod sub {
                use super::*;

                #[rstest]
                fn subtracts_values() {
                    ::approx::assert_relative_eq!($ty::from(5.0) - $ty::from(2.0), $ty::from(3.0));
                }
            }

            mod mul {
                use super::*;

                #[rstest]
                fn ty_mul_f64() {
                    ::approx::assert_relative_eq!($ty::from(5.0) * 2.0_f64, $ty::from(10.0));
                }

                #[rstest]
                fn f64_mul_ty() {
                    ::approx::assert_relative_eq!(2.0_f64 * $ty::from(5.0), $ty::from(10.0));
                }
            }

            mod div {
                use super::*;

                #[rstest]
                fn ty_div_f64() {
                    ::approx::assert_relative_eq!($ty::from(10.0) / 2.0_f64, $ty::from(5.0));
                }

                #[rstest]
                fn ratio_div() {
                    let ratio: f64 = $ty::from(10.0) / $ty::from(2.0);
                    ::approx::assert_relative_eq!(ratio, 5.0);
                }
            }

            mod neg {
                use super::*;

                #[rstest]
                fn negates_value() {
                    ::approx::assert_relative_eq!(-$ty::from(5.0), $ty::from(-5.0));
                }
            }
        }
    };
}
