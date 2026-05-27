use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use strum::VariantNames;

/// Directional and positional navigation commands.
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    strum::Display,
    strum::EnumString,
    strum::VariantNames,
)]
pub enum Movement {
    #[default]
    /// No movement; a no-op sentinel used as the enum default.
    None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::Result;
    use rstest::rstest;
    use std::str::FromStr;

    mod display {
        use super::*;

        #[rstest]
        #[case(Movement::None, "None")]
        #[case(Movement::Up, "Up")]
        #[case(Movement::Down, "Down")]
        #[case(Movement::Left, "Left")]
        #[case(Movement::Right, "Right")]
        #[case(Movement::ScrollUp, "ScrollUp")]
        #[case(Movement::ScrollDown, "ScrollDown")]
        #[case(Movement::PageUp, "PageUp")]
        #[case(Movement::PageDown, "PageDown")]
        #[case(Movement::GotoTop, "GotoTop")]
        #[case(Movement::GotoBottom, "GotoBottom")]
        fn variant_displays_as_name(#[case] mv: Movement, #[case] expected: &str) {
            assert_eq!(mv.to_string(), expected);
        }
    }

    mod from_str {
        use super::*;

        #[rstest]
        #[case("None", Movement::None)]
        #[case("Up", Movement::Up)]
        #[case("Down", Movement::Down)]
        #[case("Left", Movement::Left)]
        #[case("Right", Movement::Right)]
        #[case("ScrollUp", Movement::ScrollUp)]
        #[case("ScrollDown", Movement::ScrollDown)]
        #[case("PageUp", Movement::PageUp)]
        #[case("PageDown", Movement::PageDown)]
        #[case("GotoTop", Movement::GotoTop)]
        #[case("GotoBottom", Movement::GotoBottom)]
        fn known_variants_parse(#[case] input: &str, #[case] expected: Movement) -> Result<()> {
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

        fn roundtrip(mv: Movement) -> Result<Movement> {
            let json = serde_json::to_string(&mv)?;

            Ok(serde_json::from_str(&json)?)
        }

        #[rstest]
        #[case(Movement::None)]
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
        fn all_variants_roundtrip(#[case] mv: Movement) -> Result<()> {
            assert_eq!(roundtrip(mv)?, mv);

            Ok(())
        }

        #[rstest]
        fn serializes_as_variant_name() -> Result<()> {
            assert_eq!(serde_json::to_string(&Movement::Down)?, "\"Down\"");

            Ok(())
        }

        #[rstest]
        fn unknown_variant_returns_error() {
            assert!(serde_json::from_str::<Movement>("\"NotAMovement\"").is_err());
        }
    }
}
