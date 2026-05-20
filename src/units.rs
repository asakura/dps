//! Newtype wrappers for physical units used in dive calculations.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Generates standard impls for a newtype unit struct backed by f64.
///
/// Provides: `new`, `value`, `max`, `Display`, `From<f64>`, `From<T> for f64`,
/// `Add`, `Sub`, `Neg`, `Mul<f64>`, `Div<f64>`, `Mul<T> for f64`, `Div` (ratio),
/// `Mul<Percent>`, and `Div<Percent>`.
macro_rules! unit_newtype {
    ($ty:ident, $suffix:literal) => {
        impl $ty {
            /// Constructs a value from a raw `f64`.
            pub const fn new(val: f64) -> Self {
                Self(val)
            }

            /// Returns the underlying `f64`.
            pub const fn value(self) -> f64 {
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

        impl Mul<Percent> for $ty {
            type Output = Self;

            fn mul(self, rhs: Percent) -> Self {
                Self(self.0 * rhs.value())
            }
        }

        impl Div<Percent> for $ty {
            type Output = Self;

            fn div(self, rhs: Percent) -> Self {
                Self(self.0 / rhs.value())
            }
        }

        #[cfg(test)]
        impl approx::AbsDiffEq for $ty {
            type Epsilon = f64;

            fn default_epsilon() -> f64 {
                f64::default_epsilon()
            }

            fn abs_diff_eq(&self, other: &Self, epsilon: f64) -> bool {
                self.0.abs_diff_eq(&other.0, epsilon)
            }
        }

        #[cfg(test)]
        impl approx::RelativeEq for $ty {
            fn default_max_relative() -> f64 {
                f64::default_max_relative()
            }

            fn relative_eq(&self, other: &Self, epsilon: f64, max_relative: f64) -> bool {
                self.0.relative_eq(&other.0, epsilon, max_relative)
            }
        }
    };
}

/// Depth in metres. Backed by f64 so mul/div never truncate.
///
/// ```no_run
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
///
/// // f64 × Meters (scalar-on-right is symmetric)
/// assert_eq!((2.0_f64 * a).value(), 60.0);
/// // Meters ÷ Meters → dimensionless ratio
/// let ratio: f64 = a / Meters::new(10.0);
/// assert_eq!(ratio, 3.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Meters(f64);

unit_newtype!(Meters, "m");

/// Meters / `MetersPerBar` → Bar  (depth → gauge pressure)
///
/// ```no_run
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
/// ```no_run
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
///
/// assert_eq!((-p).value(), -1.5);
/// assert_eq!((2.0_f64 * p).value(), 3.0);
/// assert_eq!(p.max(Bar::new(2.0)).value(), 2.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Bar(f64);

unit_newtype!(Bar, "bar");

/// Bar × `MetersPerBar` → Meters  (gauge pressure → depth)
///
/// ```no_run
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
/// ```no_run
/// use dps::units::MetersPerBar;
/// let seawater = MetersPerBar::new(10.0);
/// assert_eq!(seawater.value(), 10.0);
/// assert_eq!(seawater.to_string(), "10.0 m/bar");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MetersPerBar(f64);

unit_newtype!(MetersPerBar, "m/bar");

/// Fractional proportion in [0.0, 1.0], displayed as a percentage.
///
/// ```no_run
/// # use approx::assert_relative_eq;
/// use dps::units::Percent;
/// let p = Percent::new(0.32).unwrap();
/// assert_relative_eq!(p.value(), 0.32);
/// assert_eq!(p.to_string(), "32 %");
/// assert_eq!(Percent::new(1.0).unwrap().to_string(), "100 %");
/// assert_eq!(Percent::new(0.999).unwrap().to_string(), "99.9 %");
/// assert!(Percent::new(1.1).is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Percent(f64);

impl Percent {
    /// Constructs a `Percent` from a fraction in [0.0, 1.0].
    ///
    /// Returns `None` if `val` is outside [0.0, 1.0].
    ///
    /// ```no_run
    /// use dps::units::Percent;
    /// assert!(Percent::new(0.32).is_some());
    /// assert!(Percent::new(0.0).is_some());
    /// assert!(Percent::new(1.0).is_some());
    /// assert!(Percent::new(1.1).is_none());
    /// assert!(Percent::new(-0.1).is_none());
    /// ```
    #[must_use]
    pub const fn new(val: f64) -> Option<Self> {
        if val >= 0.0 && val <= 1.0 {
            Some(Self(val))
        } else {
            None
        }
    }

    /// Returns the underlying fraction in [0.0, 1.0].
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }
}

impl From<Percent> for f64 {
    fn from(p: Percent) -> Self {
        p.0
    }
}

impl fmt::Display for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pct = self.0 * 100.0;
        // Round to 1 decimal, then drop the decimal point for whole numbers.
        // n is the rounded value × 10 as an exact integer; if it's a multiple
        // of 10 the division by 10.0 is exact and fract() is 0.0.
        let rounded = (pct * 10.0).round() / 10.0;
        if rounded.fract() == 0.0 {
            write!(f, "{rounded:.0} %")
        } else {
            write!(f, "{rounded:.1} %")
        }
    }
}

#[cfg(test)]
impl approx::AbsDiffEq for Percent {
    type Epsilon = f64;

    fn default_epsilon() -> f64 {
        f64::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f64) -> bool {
        self.0.abs_diff_eq(&other.0, epsilon)
    }
}

#[cfg(test)]
impl approx::RelativeEq for Percent {
    fn default_max_relative() -> f64 {
        f64::default_max_relative()
    }

    fn relative_eq(&self, other: &Self, epsilon: f64, max_relative: f64) -> bool {
        self.0.relative_eq(&other.0, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn display_meters() {
        assert_eq!(Meters::new(10.0).to_string(), "10.0 m");
    }

    #[test]
    fn from_f64_meters() {
        assert_relative_eq!(Meters::from(5.0_f64), Meters::new(5.0));
    }

    #[test]
    fn f64_from_meters() {
        assert_relative_eq!(f64::from(Meters::new(5.0)), 5.0);
    }

    #[test]
    fn f64_mul_meters() {
        assert_relative_eq!(2.0_f64 * Meters::new(5.0), Meters::new(10.0));
    }

    #[test]
    fn meters_ratio_div() {
        let ratio: f64 = Meters::new(10.0) / Meters::new(2.0);
        assert_relative_eq!(ratio, 5.0);
    }

    #[test]
    fn neg_meters() {
        assert_relative_eq!(-Meters::new(5.0), Meters::new(-5.0));
    }

    #[test]
    fn display_percent_whole_number() -> Result<(), &'static str> {
        assert_eq!(Percent::new(0.32).ok_or("invalid")?.to_string(), "32 %");
        Ok(())
    }

    #[test]
    fn display_percent_one_decimal() -> Result<(), &'static str> {
        assert_eq!(Percent::new(0.999).ok_or("invalid")?.to_string(), "99.9 %");
        Ok(())
    }

    #[test]
    fn display_percent_one() -> Result<(), &'static str> {
        assert_eq!(Percent::new(1.0).ok_or("invalid")?.to_string(), "100 %");
        Ok(())
    }

    #[test]
    fn new_percent_rejects_above_one() {
        assert!(Percent::new(1.1).is_none());
    }

    #[test]
    fn new_percent_rejects_negative() {
        assert!(Percent::new(-0.1).is_none());
    }

    #[test]
    fn f64_from_percent() -> Result<(), &'static str> {
        assert_relative_eq!(f64::from(Percent::new(0.40).ok_or("invalid")?), 0.40);
        Ok(())
    }
}
