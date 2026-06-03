use super::{UnitError, error::ParseError};

/// Fractional proportion in [0.0, 1.0], displayed as a percentage.
///
/// ```
/// # use approx::assert_relative_eq;
/// use dps_units::Percent;
/// let p = Percent::new(0.32).unwrap();
/// assert_relative_eq!(f64::from(p), 0.32);
/// assert_eq!(p.to_string(), "32%");
/// assert_eq!(Percent::new(1.0).unwrap().to_string(), "100%");
/// assert_eq!(Percent::new(0.999).unwrap().to_string(), "99.9%");
/// assert!(Percent::new(1.1).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
pub struct Percent(f64);

crate::unit_newtype!(
    Percent,
    bounds = 0.0..=1.0,
    to_clipboard_string = |p| {
        let mut buffer = ::ryu::Buffer::new();
        ::std::format!("{}%", buffer.format(p.0 * 100.0))
    },
    display = |p, f| {
        let pct = p.0 * 100.0;
        // Round to 1 decimal, then drop the decimal point for whole numbers.
        // n is the rounded value × 10 as an exact integer; if it's a multiple
        // of 10 the division by 10.0 is exact and fract() is 0.0.
        let rounded = (pct * 10.0).round() / 10.0;

        if rounded.fract() == 0.0 {
            write!(f, "{rounded:.0}%")
        } else {
            write!(f, "{rounded:.1}%")
        }
    },
    from_str = |s| {
        let num_str = s
            .strip_suffix("%")
            .ok_or_else(|| UnitError::Parse(ParseError::Percent(s.to_owned())))?;
        let pct: f64 = num_str
            .parse()
            .map_err(|_| UnitError::Parse(ParseError::Percent(num_str.to_owned())))?;

        Percent::new(pct / 100.0).map_err(|_| UnitError::Parse(ParseError::Percent(s.to_owned())))
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::UnitError;
    use crate::error::ParseError;

    use approx::assert_relative_eq;
    use rstest::rstest;

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
        fn roundtrip_clipboard() -> Result<(), UnitError> {
            let v = Percent::new(0.3211)?;

            // clipboard string must be bit-perfect
            assert_eq!(v.to_clipboard_string().parse::<Percent>()?, v);

            Ok(())
        }

        #[rstest]
        fn display_is_lossy_for_precision() -> Result<(), UnitError> {
            let v = Percent::new(0.3211)?;

            assert_eq!(v.to_string(), "32.1%");

            Ok(())
        }

        #[rstest]
        fn roundtrip_simple() -> Result<(), UnitError> {
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
