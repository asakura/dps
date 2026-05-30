use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use strum::{Display, VariantNames};

use super::ActionError;
use super::error::ParseError;

/// Directional and positional navigation commands.
///
/// `Movement` is the payload of [`Action::Move`](crate::action::Action::Move). Key bindings
/// produce a `Move(movement)` action; `App::dispatch` unwraps it and forwards the `Movement`
/// to the active component's
/// [`Component::handle_action`](crate::components::Component::handle_action), which maps it
/// to a cursor offset, scroll position, or absolute row index.
///
/// ## Serialisation
///
/// [`Display`](std::fmt::Display) and [`FromStr`] encode each variant as its name — the same
/// format used in key-binding configuration files and inside the `Move(…)` payload of
/// [`Action`](crate::action::Action):
///
/// ```text
/// Up           →  "Up"
/// GotoBottom   →  "GotoBottom"
/// ```
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use dps::action::Movement;
///
/// assert_eq!(Movement::Down.to_string(), "Down");
/// assert_eq!(Movement::from_str("GotoTop").unwrap(), Movement::GotoTop);
/// ```
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Display, VariantNames)]
pub enum Movement {
    /// Move the cursor or selection up by one row.
    Up,
    /// Move the cursor or selection down by one row.
    Down,
    /// Move the cursor or selection left by one column.
    Left,
    /// Move the cursor or selection right by one column.
    Right,

    /// Scroll up by one row without moving the selection.
    LineUp,
    /// Scroll down by one row without moving the selection.
    LineDown,

    /// Scroll up by [`crate::components::SCROLL_DELTA`] rows without moving the selection.
    ScrollUp,
    /// Scroll down by [`crate::components::SCROLL_DELTA`] rows without moving the selection.
    ScrollDown,

    /// Jump the selection up by [`crate::components::PAGE_DELTA`] rows.
    PageUp,
    /// Jump the selection down by [`crate::components::PAGE_DELTA`] rows.
    PageDown,

    /// Jump to the first row.
    GotoTop,
    /// Jump to the last row.
    GotoBottom,

    /// No movement; a no-op sentinel used as the default when no direction has been set.
    #[default]
    None,
}

impl Serialize for Movement {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Movement {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;

        Self::from_str(&s).map_err(|_| de::Error::unknown_variant(&s, Self::VARIANTS))
    }
}

/// Parses a [`Movement`] from its display name.
///
/// # Errors
///
/// Returns [`ActionError`] if the string does not match any known variant name.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use dps::action::{Movement, ActionError};
///
/// assert_eq!(Movement::from_str("Down").unwrap(), Movement::Down);
/// assert!(matches!(Movement::from_str("Unknown"), Err(ActionError::Parse(_))));
/// ```
impl FromStr for Movement {
    type Err = ActionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Up" => Ok(Self::Up),
            "Down" => Ok(Self::Down),
            "Left" => Ok(Self::Left),
            "Right" => Ok(Self::Right),
            "LineUp" => Ok(Self::LineUp),
            "LineDown" => Ok(Self::LineDown),
            "ScrollUp" => Ok(Self::ScrollUp),
            "ScrollDown" => Ok(Self::ScrollDown),
            "PageUp" => Ok(Self::PageUp),
            "PageDown" => Ok(Self::PageDown),
            "GotoTop" => Ok(Self::GotoTop),
            "GotoBottom" => Ok(Self::GotoBottom),
            "None" => Ok(Self::None),
            _ => Err(ParseError::VariantNotFound.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::str::FromStr;

    mod display {
        use super::*;

        #[rstest]
        #[case(Movement::Up, "Up")]
        #[case(Movement::Down, "Down")]
        #[case(Movement::Left, "Left")]
        #[case(Movement::Right, "Right")]
        #[case(Movement::LineUp, "LineUp")]
        #[case(Movement::LineDown, "LineDown")]
        #[case(Movement::ScrollUp, "ScrollUp")]
        #[case(Movement::ScrollDown, "ScrollDown")]
        #[case(Movement::PageUp, "PageUp")]
        #[case(Movement::PageDown, "PageDown")]
        #[case(Movement::GotoTop, "GotoTop")]
        #[case(Movement::GotoBottom, "GotoBottom")]
        #[case(Movement::None, "None")]
        fn variant_displays_as_name(#[case] mv: Movement, #[case] expected: &str) {
            assert_eq!(mv.to_string(), expected);
        }
    }

    mod from_str {
        use super::*;

        #[rstest]
        #[case("Up", Movement::Up)]
        #[case("Down", Movement::Down)]
        #[case("Left", Movement::Left)]
        #[case("Right", Movement::Right)]
        #[case("LineUp", Movement::LineUp)]
        #[case("LineDown", Movement::LineDown)]
        #[case("ScrollUp", Movement::ScrollUp)]
        #[case("ScrollDown", Movement::ScrollDown)]
        #[case("PageUp", Movement::PageUp)]
        #[case("PageDown", Movement::PageDown)]
        #[case("GotoTop", Movement::GotoTop)]
        #[case("GotoBottom", Movement::GotoBottom)]
        #[case("None", Movement::None)]
        fn known_variants_parse(
            #[case] input: &str,
            #[case] expected: Movement,
        ) -> Result<(), ActionError> {
            assert_eq!(Movement::from_str(input)?, expected);

            Ok(())
        }

        #[rstest]
        #[case("Unknown")]
        #[case("")]
        fn invalid_input_returns_err(#[case] input: &str) {
            assert!(Movement::from_str(input).is_err());
        }
    }

    mod serde_roundtrip {
        use super::*;

        fn roundtrip(mv: Movement) -> Result<Movement, serde_json::Error> {
            let json = serde_json::to_string(&mv)?;

            serde_json::from_str(&json)
        }

        #[rstest]
        #[case(Movement::Up)]
        #[case(Movement::Down)]
        #[case(Movement::Left)]
        #[case(Movement::Right)]
        #[case(Movement::LineUp)]
        #[case(Movement::LineDown)]
        #[case(Movement::ScrollUp)]
        #[case(Movement::ScrollDown)]
        #[case(Movement::PageUp)]
        #[case(Movement::PageDown)]
        #[case(Movement::GotoTop)]
        #[case(Movement::GotoBottom)]
        #[case(Movement::None)]
        fn all_variants_roundtrip(#[case] mv: Movement) -> Result<(), serde_json::Error> {
            assert_eq!(roundtrip(mv)?, mv);

            Ok(())
        }

        #[rstest]
        fn serializes_as_variant_name() -> Result<(), serde_json::Error> {
            assert_eq!(serde_json::to_string(&Movement::Down)?, "\"Down\"");

            Ok(())
        }

        #[rstest]
        fn unknown_variant_returns_error() {
            assert!(serde_json::from_str::<Movement>("\"NotAMovement\"").is_err());
        }
    }
}
