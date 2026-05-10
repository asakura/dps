//! Newtype wrappers for physical units used in dive calculations.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Depth in metres. Backed by f64 so mul/div never truncate.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Meters(f64);

impl Meters {
    /// Constructs a depth value from a raw metre value.
    pub const fn new(val: f64) -> Self {
        Self(val)
    }

    /// Returns the raw metre value.
    pub fn value(self) -> f64 {
        self.0
    }

    /// Returns the greater of two depths.
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    /// Absolute pressure in bar using the 10 m/bar seawater approximation.
    pub fn abs_pressure_bar(self) -> f64 {
        self.0 / 10.0 + 1.0
    }
}

impl fmt::Display for Meters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} m", self.0)
    }
}

impl From<f64> for Meters {
    fn from(v: f64) -> Self {
        Self(v)
    }
}

impl From<Meters> for f64 {
    fn from(m: Meters) -> Self {
        m.0
    }
}

impl Add for Meters {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Meters {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Neg for Meters {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

/// Meters × scalar → Meters
impl Mul<f64> for Meters {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self(self.0 * rhs)
    }
}

/// scalar × Meters → Meters
impl Mul<Meters> for f64 {
    type Output = Meters;
    fn mul(self, rhs: Meters) -> Meters {
        Meters(self * rhs.0)
    }
}

/// Meters / scalar → Meters
impl Div<f64> for Meters {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        Self(self.0 / rhs)
    }
}

/// Meters / Meters → dimensionless ratio
impl Div for Meters {
    type Output = f64;
    fn div(self, rhs: Self) -> f64 {
        self.0 / rhs.0
    }
}

/// Pressure in bar.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Bar(f64);

impl Bar {
    /// Constructs a pressure value from a raw bar value.
    pub const fn new(val: f64) -> Self {
        Self(val)
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

impl fmt::Display for Bar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} bar", self.0)
    }
}

/// Bar − Bar → Bar
impl Sub for Bar {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

/// Bar / dimensionless FO₂ fraction → Bar
impl Div<f64> for Bar {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        Self(self.0 / rhs)
    }
}

/// Depth-to-pressure conversion factor for seawater (metres per bar).
#[derive(Debug, Clone, Copy)]
pub struct MetersPerBar(f64);

impl MetersPerBar {
    /// Constructs a conversion factor from a raw m/bar value.
    pub const fn new(val: f64) -> Self {
        Self(val)
    }
}

/// Bar × MetersPerBar → Meters  (gauge pressure → depth)
impl Mul<MetersPerBar> for Bar {
    type Output = Meters;
    fn mul(self, rhs: MetersPerBar) -> Meters {
        Meters(self.0 * rhs.0)
    }
}
