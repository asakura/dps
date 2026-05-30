//! [`RegisterValue`]: the set of domain values that can live in a named register.
//!
//! ```
//! use dps::registers::RegisterValue;
//! use dps::environment::DiveEnvironment;
//!
//! let env: RegisterValue = "standard".parse().unwrap();
//! assert_eq!(env, RegisterValue::DiveEnvironment(DiveEnvironment::standard()));
//! ```

use super::{RegisterError, error::ParseError};

use crate::environment::DiveEnvironment;
use crate::gas::EANx;

use std::{fmt, str::FromStr};

/// A value that can be stored in a named register.
///
/// Both variants are [`Copy`] so the store can pass them by value without cloning.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegisterValue {
    /// An [`EANx`] partial-pressure nitrox blend.
    EANx(EANx),
    /// A dive-site environment (surface pressure + water density).
    DiveEnvironment(DiveEnvironment),
}

/// Delegates to the inner type's [`Display`](std::fmt::Display).
///
/// # Examples
///
/// ```
/// use dps::registers::RegisterValue;
/// use dps::environment::DiveEnvironment;
/// use dps::gas::EANx;
/// use dps::units::Percent;
///
/// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
/// assert_eq!(RegisterValue::EANx(ean32).to_string(), "EANx 32");
///
/// assert_eq!(
///     RegisterValue::DiveEnvironment(DiveEnvironment::standard()).to_string(),
///     "standard",
/// );
/// ```
impl fmt::Display for RegisterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EANx(b) => b.fmt(f),
            Self::DiveEnvironment(e) => e.fmt(f),
        }
    }
}

/// Tries [`EANx`] first, then [`DiveEnvironment`].
///
/// Returns [`RegisterError`] when neither parse succeeds.
///
/// # Examples
///
/// ```
/// use dps::registers::RegisterValue;
/// use dps::environment::DiveEnvironment;
/// use dps::gas::EANx;
/// use dps::units::Percent;
///
/// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
/// assert_eq!("EANx 32".parse::<RegisterValue>().unwrap(), RegisterValue::EANx(ean32));
///
/// assert_eq!(
///     "standard".parse::<RegisterValue>().unwrap(),
///     RegisterValue::DiveEnvironment(DiveEnvironment::standard()),
/// );
///
/// assert!("nonsense".parse::<RegisterValue>().is_err());
/// ```
impl FromStr for RegisterValue {
    type Err = RegisterError;

    fn from_str(s: &str) -> Result<Self, RegisterError> {
        if let Ok(b) = s.parse::<EANx>() {
            return Ok(Self::EANx(b));
        }

        if let Ok(e) = s.parse::<DiveEnvironment>() {
            return Ok(Self::DiveEnvironment(e));
        }

        Err(ParseError::UnknownValue(s.to_owned()).into())
    }
}

#[cfg(test)]
mod tests {
    use super::{RegisterError, RegisterValue};

    use crate::environment::DiveEnvironment;
    use crate::gas::EANx;
    use crate::gas::InvalidEANxError;
    use crate::units::Percent;

    use rstest::rstest;

    mod display {
        use super::*;

        #[rstest]
        fn eanx_uses_gas_name() -> Result<(), InvalidEANxError> {
            let ean32 = EANx::try_from(Percent::new(0.32)?)?;

            assert_eq!(RegisterValue::EANx(ean32).to_string(), "EANx 32");

            Ok(())
        }

        #[rstest]
        fn dive_environment_uses_preset_name() {
            assert_eq!(
                RegisterValue::DiveEnvironment(DiveEnvironment::standard()).to_string(),
                "standard",
            );
        }
    }

    mod from_str {
        use super::*;

        #[rstest]
        #[case("EANx 32")]
        #[case("EANx 36")]
        #[case("Air")]
        #[case("Pure O₂")]
        #[case("standard")]
        #[case("freshwater")]
        fn known_string_roundtrips(#[case] s: &str) -> Result<(), RegisterError> {
            let parsed: RegisterValue = s.parse()?;

            assert_eq!(parsed.to_string(), s);

            Ok(())
        }

        #[rstest]
        fn unknown_string_returns_err() {
            assert!("nonsense".parse::<RegisterValue>().is_err());
        }
    }
}
