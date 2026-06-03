//! `impl_percent_scaling!` macro: `Mul<Percent>` and `Div<Percent>` for unit newtypes.
//!
//! ```
//! use dps_units::{Bar, Percent};
//! use approx::assert_relative_eq;
//! let half = Percent::new(0.5).unwrap();
//! assert_relative_eq!(Bar::new(10.0) * half, Bar::new(5.0));
//! assert_relative_eq!(Bar::new(5.0) / half, Bar::new(10.0));
//! ```

/// Conditionally implements Mul/Div<Percent> for scaling operations.
#[doc(hidden)]
#[macro_export]
macro_rules! impl_percent_scaling {
    (Percent) => {
        impl ::std::ops::Mul<Percent> for Percent {
            type Output = Percent;

            fn mul(self, rhs: Percent) -> Percent {
                Percent(self.0 * rhs.0)
            }
        }

        #[cfg(test)]
        mod unit_newtype_scaling_tests {
            use super::*;

            use ::rstest::rstest;

            mod mul {
                use super::*;

                #[rstest]
                fn multiplies_self() {
                    ::approx::assert_relative_eq!(
                        Percent::from(0.5) * Percent::from(0.5),
                        Percent::from(0.25)
                    );
                }
            }
        }
    };
    ($ty:ident) => {
        impl ::std::ops::Mul<$crate::Percent> for $ty {
            type Output = Self;

            fn mul(self, rhs: $crate::Percent) -> Self {
                Self(self.0 * f64::from(rhs))
            }
        }

        impl ::std::ops::Mul<$ty> for $crate::Percent {
            type Output = $ty;

            fn mul(self, rhs: $ty) -> $ty {
                $ty(f64::from(self) * rhs.0)
            }
        }

        impl ::std::ops::Div<$crate::Percent> for $ty {
            type Output = Self;

            fn div(self, rhs: $crate::Percent) -> Self {
                Self(self.0 / f64::from(rhs))
            }
        }

        #[cfg(test)]
        mod unit_newtype_scaling_tests {
            use super::*;

            use ::rstest::rstest;

            mod mul {
                use super::*;

                #[rstest]
                fn multiplies_percent() {
                    ::approx::assert_relative_eq!(
                        $ty::from(10.0) * $crate::Percent::from(0.5),
                        $ty::from(5.0)
                    );
                }

                #[rstest]
                fn multiplies_percent_commutative() {
                    ::approx::assert_relative_eq!(
                        $crate::Percent::from(0.5) * $ty::from(10.0),
                        $ty::from(5.0)
                    );
                }
            }

            mod div {
                use super::*;

                #[rstest]
                fn divides_percent() {
                    ::approx::assert_relative_eq!(
                        $ty::from(5.0) / $crate::Percent::from(0.5),
                        $ty::from(10.0)
                    );
                }
            }
        }
    };
}
