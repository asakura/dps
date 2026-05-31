//! Tab-navigation motion dispatched through the event loop.
//!
//! ```
//! use std::str::FromStr;
//! use dps::action::TabMotion;
//!
//! assert_eq!(TabMotion::Next.to_string(), "Next");
//! assert_eq!(TabMotion::from_str("GoTo(3)").unwrap(), TabMotion::GoTo(3));
//! ```

use super::{ActionError, error::ParseError};

use std::{fmt, str::FromStr};

/// Motion or destination for a tab-switch action.
///
/// Carried by [`Action::Tab`](crate::action::Action::Tab) and consumed by
/// the tab-pane component to select the active tab.
///
/// ## Serialisation
///
/// ```
/// use std::str::FromStr;
/// use dps::action::TabMotion;
///
/// assert_eq!(TabMotion::Next.to_string(),     "Next");
/// assert_eq!(TabMotion::Prev.to_string(),     "Prev");
/// assert_eq!(TabMotion::GoTo(3).to_string(),  "GoTo(3)");
///
/// assert_eq!(TabMotion::from_str("Next").unwrap(),    TabMotion::Next);
/// assert_eq!(TabMotion::from_str("Prev").unwrap(),    TabMotion::Prev);
/// assert_eq!(TabMotion::from_str("GoTo(3)").unwrap(), TabMotion::GoTo(3));
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabMotion {
    /// Move to the next tab, wrapping at the end.
    Next,
    /// Move to the previous tab, wrapping at the start.
    Prev,
    /// Jump directly to the 1-indexed tab number.
    ///
    /// Count repetition is not meaningful for this variant —
    /// [`Action::accepts_count`](crate::action::Action::accepts_count) returns
    /// `false` for `Tab(GoTo(_))`.
    GoTo(usize),
}

impl fmt::Display for TabMotion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Next => f.write_str("Next"),
            Self::Prev => f.write_str("Prev"),
            Self::GoTo(n) => write!(f, "GoTo({n})"),
        }
    }
}

/// Parses a `TabMotion` from its flat-string representation.
///
/// # Errors
///
/// Returns [`ActionError`] if the string does not match any known variant.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use dps::action::TabMotion;
///
/// assert_eq!(TabMotion::from_str("Next").unwrap(),    TabMotion::Next);
/// assert_eq!(TabMotion::from_str("Prev").unwrap(),    TabMotion::Prev);
/// assert_eq!(TabMotion::from_str("GoTo(1)").unwrap(), TabMotion::GoTo(1));
/// assert!(TabMotion::from_str("Unknown").is_err());
/// assert!(TabMotion::from_str("GoTo(abc)").is_err());
/// ```
impl FromStr for TabMotion {
    type Err = ActionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Next" => Ok(Self::Next),
            "Prev" => Ok(Self::Prev),
            s => s
                .strip_prefix("GoTo(")
                .and_then(|t| t.strip_suffix(")"))
                .and_then(|n| n.parse::<usize>().ok())
                .map(Self::GoTo)
                .ok_or_else(|| ParseError::VariantNotFound.into()),
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
        #[case(TabMotion::Next, "Next")]
        #[case(TabMotion::Prev, "Prev")]
        #[case(TabMotion::GoTo(1), "GoTo(1)")]
        #[case(TabMotion::GoTo(42), "GoTo(42)")]
        fn formats_correctly(#[case] motion: TabMotion, #[case] expected: &str) {
            assert_eq!(motion.to_string(), expected);
        }
    }

    mod from_str {
        use super::*;

        #[rstest]
        #[case("Next", TabMotion::Next)]
        #[case("Prev", TabMotion::Prev)]
        #[case("GoTo(1)", TabMotion::GoTo(1))]
        #[case("GoTo(99)", TabMotion::GoTo(99))]
        fn parses_correctly(
            #[case] input: &str,
            #[case] expected: TabMotion,
        ) -> Result<(), ActionError> {
            assert_eq!(TabMotion::from_str(input)?, expected);

            Ok(())
        }

        #[rstest]
        #[case("Unknown")]
        #[case("")]
        #[case("next")]
        #[case("GoTo(abc)")]
        #[case("GoTo()")]
        fn unknown_variants_return_err(#[case] input: &str) {
            assert!(TabMotion::from_str(input).is_err());
        }
    }

    mod roundtrip {
        use super::*;

        #[rstest]
        #[case(TabMotion::Next)]
        #[case(TabMotion::Prev)]
        #[case(TabMotion::GoTo(1))]
        #[case(TabMotion::GoTo(7))]
        fn display_then_from_str_is_identity(
            #[case] motion: TabMotion,
        ) -> Result<(), ActionError> {
            assert_eq!(TabMotion::from_str(&motion.to_string())?, motion);

            Ok(())
        }
    }
}
