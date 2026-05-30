use std::fmt;
use std::str::FromStr;

use super::UnitError;
use super::error::ParseError;

/// Fractional proportion in [0.0, 1.0], displayed as a percentage.
///
/// ```no_run
/// # use approx::assert_relative_eq;
/// use dps::units::Percent;
/// let p = Percent::new(0.32).unwrap();
/// assert_relative_eq!(f64::from(p), 0.32);
/// assert_eq!(p.to_string(), "32%");
/// assert_eq!(Percent::new(1.0).unwrap().to_string(), "100%");
/// assert_eq!(Percent::new(0.999).unwrap().to_string(), "99.9%");
/// assert!(Percent::new(1.1).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Percent(f64);

impl Percent {
    /// Constructs a `Percent` from a fraction in [0.0, 1.0].
    ///
    /// # Errors
    ///
    /// Returns [`UnitError::OutOfRange`] if `val` is outside `[0.0, 1.0]`.
    ///
    /// ```no_run
    /// use dps::units::Percent;
    /// assert!(Percent::new(0.32).is_ok());
    /// assert!(Percent::new(0.0).is_ok());
    /// assert!(Percent::new(1.0).is_ok());
    /// assert!(Percent::new(1.1).is_err());
    /// assert!(Percent::new(-0.1).is_err());
    /// ```
    pub const fn new(val: f64) -> Result<Self, UnitError> {
        if val >= 0.0 && val <= 1.0 {
            Ok(Self(val))
        } else {
            Err(UnitError::OutOfRange(val))
        }
    }

    /// Constructs a `Percent` from a compile-time-known fraction, panicking if
    /// out of range.
    ///
    /// Intended exclusively for `const` items where the value is a literal
    /// guaranteed to lie in `[0.0, 1.0]`. For runtime construction, prefer
    /// [`Percent::new`].
    ///
    /// # Panics
    ///
    /// Panics if `val` is outside `[0.0, 1.0]`. When the call site is a
    /// `const` item the compiler evaluates this at compile time, turning an
    /// out-of-range value into a **compile-time error**. Calling this function
    /// at runtime with an out-of-range value still panics at runtime, which is
    /// why this function should only appear in `const` item initialisers.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use dps::units::Percent;
    /// const P: Percent = Percent::literal(0.32);
    /// assert_eq!(P.to_string(), "32%");
    /// ```
    #[expect(
        clippy::panic,
        reason = "only called from `const` item initialisers; out-of-range values there become compile-time errors rather than runtime panics"
    )]
    #[must_use]
    pub const fn literal(val: f64) -> Self {
        if val >= 0.0 && val <= 1.0 {
            Self(val)
        } else {
            panic!("Percent value is outside [0.0, 1.0]")
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
            write!(f, "{rounded:.0}%")
        } else {
            write!(f, "{rounded:.1}%")
        }
    }
}

/// Parses a [`Percent`] from its display representation.
///
/// Accepts the format produced by [`Display`](std::fmt::Display): a number
/// (integer or one decimal place) followed by `"%"`.
///
/// # Errors
///
/// Returns [`UnitError`] if the suffix is not `"%"`, the numeric part cannot
/// be parsed as `f64`, or the resulting fraction is outside `[0.0, 1.0]`.
///
/// # Examples
///
/// ```
/// use dps::units::Percent;
/// use approx::assert_relative_eq;
///
/// assert_relative_eq!(f64::from("32%".parse::<Percent>().unwrap()), 0.32);
/// assert_relative_eq!(f64::from("99.9%".parse::<Percent>().unwrap()), 0.999);
/// assert_relative_eq!(f64::from("100%".parse::<Percent>().unwrap()), 1.0);
/// assert!("invalid".parse::<Percent>().is_err());
/// ```
impl FromStr for Percent {
    type Err = UnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num_str = s
            .strip_suffix("%")
            .ok_or_else(|| UnitError::Parse(ParseError::Percent(s.to_owned())))?;
        let pct: f64 = num_str
            .parse()
            .map_err(|_| UnitError::Parse(ParseError::Percent(num_str.to_owned())))?;

        Self::new(pct / 100.0).map_err(|_| UnitError::Parse(ParseError::Percent(s.to_owned())))
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
    use approx::assert_relative_eq;
    use rstest::rstest;

    use super::*;
    use crate::units::UnitError;
    use crate::units::error::ParseError;

    mod new {
        use super::*;

        #[rstest]
        fn rejects_above_one() {
            assert!(Percent::new(1.1).is_err());
        }

        #[rstest]
        fn rejects_negative() {
            assert!(Percent::new(-0.1).is_err());
        }
    }

    mod from {
        use super::*;

        #[rstest]
        fn gives_inner_value() -> Result<(), UnitError> {
            let p = Percent::new(0.40)?;
            assert_relative_eq!(f64::from(p), 0.40);
            Ok(())
        }
    }

    mod div {
        use super::*;

        #[rstest]
        fn gives_dimensionless_ratio() -> Result<(), UnitError> {
            let a = Percent::new(0.32)?;
            let b = Percent::new(0.68)?;

            assert_relative_eq!(a / b, 0.32 / 0.68);

            Ok(())
        }
    }

    mod display {
        use super::*;

        #[rstest]
        #[case(0.32, "32%")]
        #[case(0.999, "99.9%")]
        #[case(1.0, "100%")]
        fn formats_correctly(#[case] val: f64, #[case] expected: &str) -> Result<(), UnitError> {
            let p = Percent::new(val)?;

            assert_eq!(p.to_string(), expected);

            Ok(())
        }
    }

    mod from_str {
        use super::*;

        #[rstest]
        fn roundtrip() -> Result<(), UnitError> {
            let v = Percent::new(0.32)?;

            assert_eq!(v.to_string().parse::<Percent>()?, v);

            Ok(())
        }

        #[rstest]
        #[case("101%",    UnitError::Parse(ParseError::Percent("101%".to_owned())))]
        #[case("abc%",    UnitError::Parse(ParseError::Percent("abc".to_owned())))]
        #[case("invalid", UnitError::Parse(ParseError::Percent("invalid".to_owned())))]
        fn error_carries_offending_input(#[case] input: &str, #[case] expected: UnitError) {
            assert_eq!(input.parse::<Percent>(), Err(expected));
        }
    }
}
