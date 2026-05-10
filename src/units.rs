//! Newtype wrappers for physical units used in dive calculations.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Generates standard impls for a newtype unit struct backed by f64.
///
/// Provides: `new`, `value`, `max`, `Display`, `From<f64>`, `From<T> for f64`,
/// `Add`, `Sub`, `Neg`, `Mul<f64>`, `Div<f64>`, `Mul<T> for f64`, and `Div` (ratio).
macro_rules! unit_newtype {
    ($ty:ident, $suffix:literal) => {
        impl $ty {
            /// Constructs a value from a raw `f64`.
            pub const fn new(val: f64) -> Self {
                Self(val)
            }
            /// Returns the underlying `f64`.
            pub fn value(self) -> f64 {
                self.0
            }
            /// Returns the greater of two values.
            pub fn max(self, other: Self) -> Self {
                Self(self.0.max(other.0))
            }
        }
        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:.1} {}", self.0, $suffix)
            }
        }
        impl From<f64> for $ty {
            fn from(v: f64) -> Self {
                Self(v)
            }
        }
        impl From<$ty> for f64 {
            fn from(v: $ty) -> Self {
                v.0
            }
        }
        impl Add for $ty {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }
        impl Sub for $ty {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }
        impl Mul<f64> for $ty {
            type Output = Self;
            fn mul(self, rhs: f64) -> Self {
                Self(self.0 * rhs)
            }
        }
        impl Div<f64> for $ty {
            type Output = Self;
            fn div(self, rhs: f64) -> Self {
                Self(self.0 / rhs)
            }
        }
        impl Mul<$ty> for f64 {
            type Output = $ty;
            fn mul(self, rhs: $ty) -> $ty {
                $ty(self * rhs.0)
            }
        }
        impl Div for $ty {
            type Output = f64;
            fn div(self, rhs: Self) -> f64 {
                self.0 / rhs.0
            }
        }
        impl Neg for $ty {
            type Output = Self;
            fn neg(self) -> Self {
                Self(-self.0)
            }
        }
    };
}

/// Depth in metres. Backed by f64 so mul/div never truncate.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Meters(f64);

unit_newtype!(Meters, "m");

/// Meters / MetersPerBar → Bar  (depth → gauge pressure)
impl Div<MetersPerBar> for Meters {
    type Output = Bar;
    fn div(self, rhs: MetersPerBar) -> Bar {
        Bar(self.0 / rhs.0)
    }
}

/// Pressure in bar.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Bar(f64);

unit_newtype!(Bar, "bar");

/// Bar × MetersPerBar → Meters  (gauge pressure → depth)
impl Mul<MetersPerBar> for Bar {
    type Output = Meters;
    fn mul(self, rhs: MetersPerBar) -> Meters {
        Meters(self.0 * rhs.0)
    }
}

/// Depth-to-pressure conversion factor for seawater (metres per bar).
#[derive(Debug, Clone, Copy)]
pub struct MetersPerBar(f64);

unit_newtype!(MetersPerBar, "m/bar");
