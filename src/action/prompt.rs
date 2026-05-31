//! Prompt-response actions for modal confirmation dialogs.
//!
//! ```
//! use std::str::FromStr;
//! use dps::action::PromptOp;
//!
//! assert_eq!(PromptOp::Confirm.to_string(), "Confirm");
//! assert_eq!(PromptOp::from_str("Cancel").unwrap(), PromptOp::Cancel);
//! ```

use super::{ActionError, error::ParseError};

use std::{fmt, str::FromStr};

/// Response to a modal confirmation prompt.
///
/// Carried by [`Action::Prompt`](crate::action::Action::Prompt). Produced
/// when the user answers an in-flight confirmation dialog —
/// `y` / Enter → [`Confirm`](PromptOp::Confirm),
/// `n` / Esc / `q` → [`Cancel`](PromptOp::Cancel).
///
/// ## Serialisation
///
/// ```
/// use std::str::FromStr;
/// use dps::action::PromptOp;
///
/// assert_eq!(PromptOp::Confirm.to_string(),          "Confirm");
/// assert_eq!(PromptOp::Cancel.to_string(),           "Cancel");
///
/// assert_eq!(PromptOp::from_str("Confirm").unwrap(), PromptOp::Confirm);
/// assert_eq!(PromptOp::from_str("Cancel").unwrap(),  PromptOp::Cancel);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptOp {
    /// Affirmative answer — proceed with the guarded operation.
    Confirm,
    /// Negative answer — dismiss without acting.
    Cancel,
}

impl fmt::Display for PromptOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Confirm => f.write_str("Confirm"),
            Self::Cancel => f.write_str("Cancel"),
        }
    }
}

/// Parses a `PromptOp` from its flat-string representation.
///
/// # Errors
///
/// Returns [`ActionError`] if the string does not match any known variant.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use dps::action::PromptOp;
///
/// assert_eq!(PromptOp::from_str("Confirm").unwrap(), PromptOp::Confirm);
/// assert_eq!(PromptOp::from_str("Cancel").unwrap(),  PromptOp::Cancel);
/// assert!(PromptOp::from_str("Unknown").is_err());
/// ```
impl FromStr for PromptOp {
    type Err = ActionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Confirm" => Ok(Self::Confirm),
            "Cancel" => Ok(Self::Cancel),
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
        #[case(PromptOp::Confirm, "Confirm")]
        #[case(PromptOp::Cancel, "Cancel")]
        fn formats_correctly(#[case] op: PromptOp, #[case] expected: &str) {
            assert_eq!(op.to_string(), expected);
        }
    }

    mod from_str {
        use super::*;

        #[rstest]
        #[case("Confirm", PromptOp::Confirm)]
        #[case("Cancel", PromptOp::Cancel)]
        fn parses_correctly(
            #[case] input: &str,
            #[case] expected: PromptOp,
        ) -> Result<(), ActionError> {
            assert_eq!(PromptOp::from_str(input)?, expected);

            Ok(())
        }

        #[rstest]
        #[case("Unknown")]
        #[case("")]
        #[case("confirm")]
        fn unknown_variants_return_err(#[case] input: &str) {
            assert!(PromptOp::from_str(input).is_err());
        }
    }

    mod roundtrip {
        use super::*;

        #[rstest]
        #[case(PromptOp::Confirm)]
        #[case(PromptOp::Cancel)]
        fn display_then_from_str_is_identity(#[case] op: PromptOp) -> Result<(), ActionError> {
            assert_eq!(PromptOp::from_str(&op.to_string())?, op);

            Ok(())
        }
    }
}
