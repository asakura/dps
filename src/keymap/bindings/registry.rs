//! Immutable mode-indexed keybinding registry [`KeyBindings`].
//!
//! # Examples
//!
//! ```
//! use dps::keymap::{KeyBindingsBuilder, KeySeq, Mode, keys::parse_key_sequence};
//! use dps::action::{Action, Movement};
//!
//! let mut b = KeyBindingsBuilder::new();
//! b.bind(
//!     Mode::Normal,
//!     KeySeq::from(parse_key_sequence("j").unwrap()),
//!     Action::Move(Movement::Down),
//! );
//! let bindings = b.build();
//! assert!(bindings.get(&Mode::Normal).is_some());
//! ```

use crate::keymap::{map::ModeMap, mode::Mode};

/// Read-only mode-indexed keybinding registry.
///
/// Constructed via [`KeyBindingsBuilder`]; immutable thereafter.
/// Stored as a flat boxed slice sorted by [`Mode`] for compact memory.
#[derive(Clone, Debug, Default)]
pub struct KeyBindings(Box<[(Mode, ModeMap)]>);

impl KeyBindings {
    /// Constructs from a sorted, boxed slice of mode–map pairs.
    pub(in crate::keymap::bindings) const fn from_sorted_pairs(
        pairs: Box<[(Mode, ModeMap)]>,
    ) -> Self {
        Self(pairs)
    }

    /// Returns the binding map for `mode`, or `None` if none is registered.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::keymap::{KeyBindings, KeyBindingsBuilder, Mode};
    ///
    /// let bindings = KeyBindingsBuilder::new().build();
    /// assert!(bindings.get(&Mode::Normal).is_none());
    /// ```
    #[must_use]
    pub fn get(&self, mode: &Mode) -> Option<&ModeMap> {
        self.0.iter().find(|(m, _)| m == mode).map(|(_, map)| map)
    }

    /// Returns an iterator over `(&Mode, &ModeMap)` pairs.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::keymap::{KeyBindingsBuilder, KeySeq, Mode, keys::parse_key_sequence};
    /// use dps::action::{Action, Movement};
    ///
    /// let mut b = KeyBindingsBuilder::new();
    /// b.bind(Mode::Normal, KeySeq::from(parse_key_sequence("j").unwrap()), Action::Move(Movement::Down));
    /// let bindings = b.build();
    /// assert_eq!(bindings.iter().count(), 1);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&Mode, &ModeMap)> {
        self.0.iter().map(|(m, map)| (m, map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::action::{Action, Movement};
    use crate::keymap::{
        KeyBindingsBuilder,
        testutil::{press, single},
    };

    use crossterm::event::{KeyCode, KeyEvent};
    use rstest::fixture;
    use rstest::rstest;

    fn lookup<'a>(b: &'a KeyBindings, mode: Mode, keys: &[KeyEvent]) -> Option<&'a Action> {
        b.get(&mode).and_then(|m| m.get(keys))
    }

    #[fixture]
    #[once]
    fn simple_bindings() -> KeyBindings {
        KeyBindingsBuilder::new()
            .bind(
                Mode::Normal,
                single(KeyCode::Char('j')),
                Action::Move(Movement::Down),
            )
            .bind(
                Mode::Normal,
                single(KeyCode::Char('k')),
                Action::Move(Movement::Up),
            )
            .build()
    }

    mod key_bindings {
        use super::*;

        #[rstest]
        fn registered_mode_is_found(simple_bindings: &KeyBindings) {
            assert!(simple_bindings.get(&Mode::Normal).is_some());
        }

        #[rstest]
        fn binding_in_registered_mode_resolves(simple_bindings: &KeyBindings) {
            assert_eq!(
                lookup(
                    simple_bindings,
                    Mode::Normal,
                    [press(KeyCode::Char('j'))].as_slice()
                ),
                Some(&Action::Move(Movement::Down))
            );
        }

        #[rstest]
        fn iter_yields_registered_modes(simple_bindings: &KeyBindings) {
            let modes: Vec<&Mode> = simple_bindings.iter().map(|(m, _)| m).collect();
            assert_eq!(modes, vec![&Mode::Normal]);
        }
    }
}
