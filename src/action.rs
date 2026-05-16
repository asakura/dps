//! Actions produced by components and consumed by the event loop.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use strum::{Display, EnumString};

/// Directional and positional navigation commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
pub enum Movement {
    Up,
    Down,
    Left,
    Right,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    GotoTop,
    GotoBottom,
}

/// Outcome returned by [`crate::components::Component::handle_key`] and [`crate::app::App::handle_key`].
///
/// `Display` is implemented manually so that `Move(Down)` renders as `"Down"` rather than `"Move"`,
/// keeping the display output consistent with the flat config format.
/// TODO: wire `Display` output up once the WhichKey widget and status bar exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Exit the application.
    Quit,
    /// Key was handled internally; no further action required.
    None,
    /// A directional or positional navigation command.
    Move(Movement),
    Select,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Quit => write!(f, "Quit"),
            Action::None => write!(f, "None"),
            Action::Select => write!(f, "Select"),
            Action::Move(mv) => write!(f, "{mv}"),
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
            "Quit" => Ok(Action::Quit),
            "None" => Ok(Action::None),
            "Select" => Ok(Action::Select),
            s => Movement::from_str(s).map(Action::Move).map_err(|_| {
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

        #[test]
        fn move_shows_movement_name_not_variant_name() {
            assert_eq!(Action::Move(Movement::Down).to_string(), "Down");
            assert_eq!(Action::Move(Movement::GotoTop).to_string(), "GotoTop");
            assert_eq!(Action::Move(Movement::ScrollUp).to_string(), "ScrollUp");
        }
    }

    mod serde_roundtrip {
        use super::*;

        fn roundtrip(action: &Action) -> Action {
            let json = serde_json::to_string(action).unwrap();
            serde_json::from_str(&json).unwrap()
        }

        #[test]
        fn quit() {
            assert_eq!(roundtrip(&Action::Quit), Action::Quit);
        }

        #[test]
        fn none() {
            assert_eq!(roundtrip(&Action::None), Action::None);
        }

        #[test]
        fn select() {
            assert_eq!(roundtrip(&Action::Select), Action::Select);
        }

        #[test]
        fn all_movements() {
            for mv in [
                Movement::Up,
                Movement::Down,
                Movement::Left,
                Movement::Right,
                Movement::ScrollUp,
                Movement::ScrollDown,
                Movement::PageUp,
                Movement::PageDown,
                Movement::GotoTop,
                Movement::GotoBottom,
            ] {
                let action = Action::Move(mv);
                assert_eq!(roundtrip(&action), action, "round-trip failed for {mv}");
            }
        }

        #[test]
        fn movement_serializes_as_flat_string_not_nested_object() {
            assert_eq!(
                serde_json::to_string(&Action::Move(Movement::Down)).unwrap(),
                "\"Down\""
            );
        }

        #[test]
        fn unknown_variant_returns_error() {
            assert!(serde_json::from_str::<Action>("\"NotAnAction\"").is_err());
        }
    }
}
