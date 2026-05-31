//! UI control actions dispatched through the event loop.
//!
//! ```
//! use std::str::FromStr;
//! use dps::action::UiOp;
//!
//! assert_eq!(UiOp::Help.to_string(), "Help");
//! assert_eq!(UiOp::from_str("Help").unwrap(), UiOp::Help);
//! ```

use super::{ActionError, error::ParseError};

use std::{fmt, str::FromStr};

/// UI-layer control operations.
///
/// Carried by [`Action::Ui`](crate::action::Action::Ui). Drives overlay
/// panels, toggles, and other display-layer controls that are not tied to a
/// specific component's data model.
///
/// ## Serialisation
///
/// ```
/// use std::str::FromStr;
/// use dps::action::UiOp;
///
/// assert_eq!(UiOp::Help.to_string(),          "Help");
/// assert_eq!(UiOp::from_str("Help").unwrap(), UiOp::Help);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiOp {
    /// Toggle the which-key / help overlay.
    ///
    /// Wired to `?` by default.
    Help,
}

impl fmt::Display for UiOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Help => f.write_str("Help"),
        }
    }
}

/// Parses a `UiOp` from its flat-string representation.
///
/// # Errors
///
/// Returns [`ActionError`] if the string does not match any known variant.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use dps::action::UiOp;
///
/// assert_eq!(UiOp::from_str("Help").unwrap(), UiOp::Help);
/// assert!(UiOp::from_str("Unknown").is_err());
/// ```
impl FromStr for UiOp {
    type Err = ActionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Help" => Ok(Self::Help),
            _ => Err(ParseError::VariantNotFound.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    mod display {
        use super::*;

        #[rstest]
        fn help_formats_correctly() {
            assert_eq!(UiOp::Help.to_string(), "Help");
        }
    }

    mod from_str {
        use super::*;

        #[rstest]
        fn help_parses_correctly() -> Result<(), ActionError> {
            assert_eq!(UiOp::from_str("Help")?, UiOp::Help);

            Ok(())
        }

        #[rstest]
        #[case("Unknown")]
        #[case("")]
        #[case("help")]
        fn unknown_variants_return_err(#[case] input: &str) {
            assert!(UiOp::from_str(input).is_err());
        }
    }

    mod roundtrip {
        use super::*;

        #[rstest]
        fn display_then_from_str_is_identity() -> Result<(), ActionError> {
            assert_eq!(UiOp::from_str(&UiOp::Help.to_string())?, UiOp::Help);

            Ok(())
        }
    }
}
