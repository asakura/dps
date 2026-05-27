//! UI action types dispatched through the event loop.
//!
//! Two enums are provided:
//!
//! - [`Movement`] — directional navigation commands (Up, Down, scroll, page, go-to-top/bottom).
//!   Implements [`Display`](std::fmt::Display), [`FromStr`],
//!   [`Serialize`], and [`Deserialize`] so that
//!   key-binding configuration can reference movements by name (`"Down"`, `"GotoBottom"`, …).
//!
//! - [`Action`] — the outcome returned by component event handlers. Wraps [`Movement`] as
//!   `Action::Move(mv)` and adds `Quit`, `None`, and `Select`. Serialises as a flat string:
//!   `"Quit"`, `"None"`, `"Select"`, or `"Move(Down)"` for movement variants.

mod movement;

pub use movement::Movement;

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use strum::{ParseError, VariantNames};

/// Outcome returned by [`crate::components::Component::handle_action`] and [`crate::app::App::handle_key`].
///
/// TODO: wire `Display` output up once the `WhichKey` widget and status bar exist.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, strum::VariantNames)]
pub enum Action {
    /// Exit the application.
    Quit,
    /// Key was handled internally; no further action required.
    #[default]
    None,
    /// A directional or positional navigation command.
    Move(Movement),
    /// Confirm or activate the highlighted item.
    Select,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quit => f.write_str("Quit"),
            Self::None => f.write_str("None"),
            Self::Select => f.write_str("Select"),
            Self::Move(mv) => write!(f, "Move({mv})"),
        }
    }
}

impl FromStr for Action {
    type Err = ParseError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if let Some(inner) = s.strip_prefix("Move(").and_then(|s| s.strip_suffix(")")) {
            return Movement::from_str(inner).map(Self::Move);
        }

        match s {
            "Quit" => Ok(Self::Quit),
            "None" => Ok(Self::None),
            "Select" => Ok(Self::Select),
            _ => Err(ParseError::VariantNotFound),
        }
    }
}

impl Serialize for Action {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;

        Self::from_str(&s).map_err(|_| de::Error::unknown_variant(&s, Self::VARIANTS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::Result;
    use rstest::rstest;
    use std::str::FromStr;

    mod display {
        use super::*;

        #[rstest]
        #[case(Action::Quit, "Quit")]
        #[case(Action::None, "None")]
        #[case(Action::Select, "Select")]
        fn non_move_actions_display(#[case] action: Action, #[case] expected: &str) {
            assert_eq!(action.to_string(), expected);
        }

        #[rstest]
        #[case(Movement::Down, "Move(Down)")]
        #[case(Movement::GotoTop, "Move(GotoTop)")]
        #[case(Movement::ScrollUp, "Move(ScrollUp)")]
        fn move_wraps_movement_in_parens(#[case] mv: Movement, #[case] expected: &str) {
            assert_eq!(Action::Move(mv).to_string(), expected);
        }
    }

    mod from_str {
        use super::*;

        #[rstest]
        #[case("Quit", Action::Quit)]
        #[case("None", Action::None)]
        #[case("Select", Action::Select)]
        #[case("Move(Up)", Action::Move(Movement::Up))]
        #[case("Move(Down)", Action::Move(Movement::Down))]
        #[case("Move(GotoBottom)", Action::Move(Movement::GotoBottom))]
        fn known_variants_parse(#[case] input: &str, #[case] expected: Action) -> Result<()> {
            assert_eq!(Action::from_str(input)?, expected);

            Ok(())
        }

        #[rstest]
        #[case("Down")]
        #[case("Unknown")]
        fn invalid_input_returns_err(#[case] input: &str) {
            assert!(Action::from_str(input).is_err());
        }
    }

    mod serde_roundtrip {
        use super::*;

        fn roundtrip(action: Action) -> Result<Action> {
            let json = serde_json::to_string(&action)?;

            Ok(serde_json::from_str(&json)?)
        }

        #[rstest]
        #[case(Action::Quit)]
        #[case(Action::None)]
        #[case(Action::Select)]
        fn non_movement_actions_roundtrip(#[case] action: Action) -> Result<()> {
            assert_eq!(roundtrip(action)?, action);

            Ok(())
        }

        #[rstest]
        #[case(Movement::Up)]
        #[case(Movement::Down)]
        #[case(Movement::Left)]
        #[case(Movement::Right)]
        #[case(Movement::ScrollUp)]
        #[case(Movement::ScrollDown)]
        #[case(Movement::PageUp)]
        #[case(Movement::PageDown)]
        #[case(Movement::GotoTop)]
        #[case(Movement::GotoBottom)]
        fn movement_roundtrips(#[case] mv: Movement) -> Result<()> {
            let action = Action::Move(mv);
            assert_eq!(roundtrip(action)?, action);

            Ok(())
        }

        #[rstest]
        fn movement_serializes_as_move_parens_string() -> Result<()> {
            assert_eq!(
                serde_json::to_string(&Action::Move(Movement::Down))?,
                "\"Move(Down)\""
            );

            Ok(())
        }

        #[rstest]
        fn unknown_variant_returns_error() {
            assert!(serde_json::from_str::<Action>("\"NotAnAction\"").is_err());
        }
    }
}
