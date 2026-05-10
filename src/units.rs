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
///
/// ```
/// use dps::units::Meters;
///
/// let a = Meters::new(30.0);
/// assert_eq!(a.value(), 30.0);
/// assert_eq!(a.to_string(), "30.0 m");
///
/// assert_eq!((a + Meters::new(10.0)).value(), 40.0);
/// assert_eq!((a - Meters::new(10.0)).value(), 20.0);
/// assert_eq!((-a).value(), -30.0);
/// assert_eq!((a * 2.0).value(), 60.0);
/// assert_eq!((a / 2.0).value(), 15.0);
/// assert_eq!(a.max(Meters::new(50.0)).value(), 50.0);
///
/// let b: Meters = 30.0_f64.into();
/// assert_eq!(f64::from(b), 30.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Meters(f64);

unit_newtype!(Meters, "m");

/// Meters / MetersPerBar → Bar  (depth → gauge pressure)
///
/// ```
/// use dps::units::{Bar, Meters, MetersPerBar};
/// let gauge: Bar = Meters::new(30.0) / MetersPerBar::new(10.0);
/// assert_eq!(gauge.value(), 3.0);
/// ```
impl Div<MetersPerBar> for Meters {
    type Output = Bar;
    fn div(self, rhs: MetersPerBar) -> Bar {
        Bar(self.0 / rhs.0)
    }
}

/// Pressure in bar.
///
/// ```
/// use dps::units::Bar;
///
/// let p = Bar::new(1.5);
/// assert_eq!(p.value(), 1.5);
/// assert_eq!(p.to_string(), "1.5 bar");
///
/// assert_eq!((p + Bar::new(0.5)).value(), 2.0);
/// assert_eq!((p - Bar::new(0.5)).value(), 1.0);
/// assert_eq!((p * 2.0).value(), 3.0);
/// assert_eq!((p / 2.0).value(), 0.75);
///
/// // Ratio between two Bar values is dimensionless.
/// let ratio: f64 = Bar::new(3.0) / Bar::new(1.5);
/// assert_eq!(ratio, 2.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Bar(f64);

unit_newtype!(Bar, "bar");

/// Bar × MetersPerBar → Meters  (gauge pressure → depth)
///
/// ```
/// use dps::units::{Bar, Meters, MetersPerBar};
/// let depth: Meters = Bar::new(3.0) * MetersPerBar::new(10.0);
/// assert_eq!(depth.value(), 30.0);
/// ```
impl Mul<MetersPerBar> for Bar {
    type Output = Meters;
    fn mul(self, rhs: MetersPerBar) -> Meters {
        Meters(self.0 * rhs.0)
    }
}

/// Depth-to-pressure conversion factor for seawater (metres per bar).
///
/// ```
/// use dps::units::MetersPerBar;
/// let seawater = MetersPerBar::new(10.0);
/// assert_eq!(seawater.value(), 10.0);
/// assert_eq!(seawater.to_string(), "10.0 m/bar");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct MetersPerBar(f64);

unit_newtype!(MetersPerBar, "m/bar");
