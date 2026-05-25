//! Actions produced by components and consumed by the event loop.

use std::{fmt, str::FromStr};

use color_eyre::Result;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use strum::{Display, EnumString};

/// Directional and positional navigation commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
pub enum Movement {
    /// Move the cursor or selection up by one row.
    Up,
    /// Move the cursor or selection down by one row.
    Down,
    /// Move the cursor or selection left.
    Left,
    /// Move the cursor or selection right.
    Right,
    /// Scroll up by [`crate::components::SCROLL_DELTA`] rows.
    ScrollUp,
    /// Scroll down by [`crate::components::SCROLL_DELTA`] rows.
    ScrollDown,
    /// Jump up by [`crate::components::PAGE_DELTA`] rows.
    PageUp,
    /// Jump down by [`crate::components::PAGE_DELTA`] rows.
    PageDown,
    /// Jump to the first row.
    GotoTop,
    /// Jump to the last row.
    GotoBottom,
}

/// Outcome returned by [`crate::components::Component::handle_action`] and [`crate::app::App::handle_key`].
///
/// `Display` is implemented manually so that `Move(Down)` renders as `"Down"` rather than `"Move"`,
/// keeping the display output consistent with the flat config format.
/// TODO: wire `Display` output up once the `WhichKey` widget and status bar exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Exit the application.
    Quit,
    /// Key was handled internally; no further action required.
    None,
    /// A directional or positional navigation command.
    Move(Movement),
    /// Confirm or activate the highlighted item.
    Select,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quit => write!(f, "Quit"),
            Self::None => write!(f, "None"),
            Self::Select => write!(f, "Select"),
            Self::Move(mv) => write!(f, "{mv}"),
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
        match s.as_str() {
            "Quit" => Ok(Self::Quit),
            "None" => Ok(Self::None),
            "Select" => Ok(Self::Select),
            s => Movement::from_str(s).map(Self::Move).map_err(|_| {
                de::Error::unknown_variant(
                    s,
                    &[
                        "Quit",
                        "None",
                        "Select",
                        "Up",
                        "Down",
                        "Left",
                        "Right",
                        "ScrollUp",
                        "ScrollDown",
                        "PageUp",
                        "PageDown",
                        "GotoTop",
                        "GotoBottom",
                    ],
                )
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod display {
        use super::*;
        use rstest::rstest;

        #[test]
        fn quit() {
            assert_eq!(Action::Quit.to_string(), "Quit");
        }

        #[test]
        fn none() {
            assert_eq!(Action::None.to_string(), "None");
        }

        #[test]
        fn select() {
            assert_eq!(Action::Select.to_string(), "Select");
        }

        #[rstest]
        #[case(Movement::Down, "Down")]
        #[case(Movement::GotoTop, "GotoTop")]
        #[case(Movement::ScrollUp, "ScrollUp")]
        fn move_shows_movement_name_not_variant_name(#[case] mv: Movement, #[case] expected: &str) {
            assert_eq!(Action::Move(mv).to_string(), expected);
        }
    }

    mod serde_roundtrip {
        use super::*;
        use rstest::rstest;

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

        #[test]
        fn movement_serializes_as_flat_string_not_nested_object() -> Result<()> {
            assert_eq!(
                serde_json::to_string(&Action::Move(Movement::Down))?,
                "\"Down\""
            );

            Ok(())
        }

        #[test]
        fn unknown_variant_returns_error() {
            assert!(serde_json::from_str::<Action>("\"NotAnAction\"").is_err());
        }
    }
}
