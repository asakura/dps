use std::fmt;

/// Fractional proportion in [0.0, 1.0], displayed as a percentage.
///
/// ```no_run
/// # use approx::assert_relative_eq;
/// use dps::units::Percent;
/// let p = Percent::new(0.32).unwrap();
/// assert_relative_eq!(f64::from(p), 0.32);
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
}

impl From<Percent> for f64 {
    fn from(p: Percent) -> Self {
        p.0
    }
}

/// Ratio of two fractions; the result is dimensionless.
///
/// ```no_run
/// use dps::units::Percent;
/// # use approx::assert_relative_eq;
/// let n2 = Percent::new(0.7808).unwrap();
/// let diluent = Percent::new(0.7906).unwrap();
/// let ratio: f64 = n2 / diluent;
/// assert_relative_eq!(ratio, 0.7808 / 0.7906, epsilon = 1e-9);
/// ```
impl std::ops::Div for Percent {
    type Output = f64;

    fn div(self, rhs: Self) -> f64 {
        self.0 / rhs.0
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

impl approx::AbsDiffEq for Percent {
    type Epsilon = f64;

    fn default_epsilon() -> f64 {
        f64::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f64) -> bool {
        self.0.abs_diff_eq(&other.0, epsilon)
    }
}

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

    #[test]
    fn div_gives_dimensionless_ratio() -> Result<(), &'static str> {
        let a = Percent::new(0.32).ok_or("invalid")?;
        let b = Percent::new(0.68).ok_or("invalid")?;

        assert_relative_eq!(a / b, 0.32 / 0.68);

        Ok(())
    }
}
