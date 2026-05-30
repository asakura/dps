//! Mode-indexed keybinding registry and its builder.

use std::collections::HashMap;

use serde::{Deserialize, Deserializer};

use crate::action::Action;

use super::keys::parse_key_sequence;
use super::map::{ModeMap, ModeMapBuilder};
use super::mode::Mode;
use super::seq::KeySeq;

/// Read-only mode-indexed keybinding registry.
///
/// Constructed via [`KeyBindingsBuilder`]; immutable thereafter.
/// Stored as a flat boxed slice sorted by [`Mode`] for compact memory.
#[derive(Clone, Debug, Default)]
pub struct KeyBindings(Box<[(Mode, ModeMap)]>);

impl KeyBindings {
    /// Returns the binding map for `mode`, or `None` if none is registered.
    #[must_use]
    pub fn get(&self, mode: &Mode) -> Option<&ModeMap> {
        self.0.iter().find(|(m, _)| m == mode).map(|(_, map)| map)
    }

    /// Returns an iterator over `(&Mode, &ModeMap)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Mode, &ModeMap)> {
        self.0.iter().map(|(m, map)| (m, map))
    }
}

/// Mutable accumulator that produces an immutable [`KeyBindings`] on [`build`](Self::build).
#[derive(Clone, Debug, Default)]
pub struct KeyBindingsBuilder(HashMap<Mode, ModeMapBuilder>);

impl KeyBindingsBuilder {
    /// Creates an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds `seq` to `action` in `mode`, overwriting any existing binding.
    pub fn bind(&mut self, mode: Mode, seq: KeySeq, action: Action) -> &mut Self {
        self.0.entry(mode).or_default().bind(seq, action);

        self
    }

    /// Binds `seq` to `action` in `mode` only if `seq` is not already bound.
    pub fn bind_default(&mut self, mode: Mode, seq: KeySeq, action: Action) -> &mut Self {
        self.0.entry(mode).or_default().bind_default(seq, action);

        self
    }

    /// Copies every binding from `defaults` that is not already present in `self`.
    pub fn merge_defaults(&mut self, defaults: &Self) -> &mut Self {
        for (mode, def_map) in &defaults.0 {
            self.0
                .entry(*mode)
                .or_default()
                .merge_defaults_from(def_map);
        }

        self
    }

    /// Returns an immutable [`KeyBindings`], draining the builder.
    ///
    /// Modes are stored in sorted order so iteration is deterministic.
    #[must_use]
    pub fn build(&mut self) -> KeyBindings {
        let mut pairs: Vec<(Mode, ModeMap)> = std::mem::take(&mut self.0)
            .into_iter()
            .map(|(mode, mut builder)| (mode, builder.build()))
            .collect();

        pairs.sort_by_key(|(m, _)| *m);

        KeyBindings(pairs.into_boxed_slice())
    }
}

impl<'de> Deserialize<'de> for KeyBindingsBuilder {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let parsed_map = HashMap::<Mode, HashMap<String, Action>>::deserialize(deserializer)?;

        parsed_map
            .into_iter()
            .flat_map(|(mode, inner_map)| {
                inner_map
                    .into_iter()
                    .map(move |(key_str, action)| (mode, key_str, action))
            })
            .try_fold(Self::new(), |mut b, (mode, key_str, action)| {
                let seq = parse_key_sequence(&key_str).map_err(serde::de::Error::custom)?;
                b.bind(mode, KeySeq::from(seq), action);
                Ok(b)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, Movement};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use rstest::fixture;
    use rstest::rstest;

    use crate::keymap::testutil::{press, single};

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
                lookup(simple_bindings, Mode::Normal, &[press(KeyCode::Char('j'))]),
                Some(&Action::Move(Movement::Down))
            );
        }

        #[rstest]
        fn iter_yields_registered_modes(simple_bindings: &KeyBindings) {
            let modes: Vec<&Mode> = simple_bindings.iter().map(|(m, _)| m).collect();
            assert_eq!(modes, vec![&Mode::Normal]);
        }
    }

    mod key_bindings_builder {
        use super::*;

        #[fixture]
        #[once]
        fn overwritten_binding() -> KeyBindings {
            KeyBindingsBuilder::new()
                .bind(
                    Mode::Normal,
                    single(KeyCode::Char('j')),
                    Action::Move(Movement::Down),
                )
                .bind(
                    Mode::Normal,
                    single(KeyCode::Char('j')),
                    Action::Move(Movement::Up),
                )
                .build()
        }

        #[fixture]
        #[once]
        fn default_not_overwritten() -> KeyBindings {
            KeyBindingsBuilder::new()
                .bind(
                    Mode::Normal,
                    single(KeyCode::Char('j')),
                    Action::Move(Movement::Down),
                )
                .bind_default(
                    Mode::Normal,
                    single(KeyCode::Char('j')),
                    Action::Move(Movement::Up),
                )
                .build()
        }

        #[fixture]
        #[once]
        fn merged_with_defaults() -> KeyBindings {
            let mut defaults = KeyBindingsBuilder::new();

            defaults.bind(
                Mode::Normal,
                single(KeyCode::Char('j')),
                Action::Move(Movement::Down),
            );

            KeyBindingsBuilder::new()
                .bind(Mode::Normal, single(KeyCode::Char('x')), Action::Quit)
                .merge_defaults(&defaults)
                .build()
        }

        #[fixture]
        #[once]
        fn user_overrides_merged() -> KeyBindings {
            let mut defaults = KeyBindingsBuilder::new();

            defaults.bind(
                Mode::Normal,
                single(KeyCode::Char('j')),
                Action::Move(Movement::Down),
            );

            KeyBindingsBuilder::new()
                .bind(
                    Mode::Normal,
                    single(KeyCode::Char('j')),
                    Action::Move(Movement::Up),
                )
                .merge_defaults(&defaults)
                .build()
        }

        #[rstest]
        fn bind_overwrites_in_same_mode(overwritten_binding: &KeyBindings) {
            assert_eq!(
                lookup(overwritten_binding, Mode::Normal, &[press(KeyCode::Char('j'))]),
                Some(&Action::Move(Movement::Up))
            );
        }

        #[rstest]
        fn bind_default_does_not_overwrite(default_not_overwritten: &KeyBindings) {
            assert_eq!(
                lookup(default_not_overwritten, Mode::Normal, &[press(KeyCode::Char('j'))]),
                Some(&Action::Move(Movement::Down))
            );
        }

        #[rstest]
        fn build_produces_sorted_deterministic_order() {
            // Insert Confirm before Normal — reverse of sorted order — to verify sort.
            let bindings = KeyBindingsBuilder::new()
                .bind(Mode::Confirm, single(KeyCode::Char('y')), Action::Quit)
                .bind(
                    Mode::Normal,
                    single(KeyCode::Char('j')),
                    Action::Move(Movement::Down),
                )
                .build();

            let modes: Vec<Mode> = bindings.iter().map(|(m, _)| *m).collect();
            assert_eq!(modes, vec![Mode::Normal, Mode::Confirm]);
        }

        #[rstest]
        fn merge_defaults_fills_missing_bindings(merged_with_defaults: &KeyBindings) {
            assert_eq!(
                lookup(merged_with_defaults, Mode::Normal, &[press(KeyCode::Char('j'))]),
                Some(&Action::Move(Movement::Down))
            );
            assert_eq!(
                lookup(merged_with_defaults, Mode::Normal, &[press(KeyCode::Char('x'))]),
                Some(&Action::Quit)
            );
        }

        #[rstest]
        fn merge_defaults_does_not_overwrite_user_binding(user_overrides_merged: &KeyBindings) {
            assert_eq!(
                lookup(user_overrides_merged, Mode::Normal, &[press(KeyCode::Char('j'))]),
                Some(&Action::Move(Movement::Up))
            );
        }
    }

    mod deserialize {
        use super::*;
        #[rstest]
        fn single_binding_deserializes() -> Result<(), serde_json::Error> {
            let json = r#"{ "Normal": { "j": "Move(Down)" } }"#;
            let mut builder: KeyBindingsBuilder = serde_json::from_str(json)?;
            let bindings = builder.build();

            assert_eq!(
                lookup(&bindings, Mode::Normal, &[press(KeyCode::Char('j'))]),
                Some(&Action::Move(Movement::Down))
            );

            Ok(())
        }

        #[rstest]
        fn multi_key_chord_deserializes() -> Result<(), serde_json::Error> {
            let json = r#"{ "Normal": { "gg": "Move(GotoTop)" } }"#;
            let mut builder: KeyBindingsBuilder = serde_json::from_str(json)?;
            let bindings = builder.build();

            assert_eq!(
                lookup(
                    &bindings,
                    Mode::Normal,
                    &[press(KeyCode::Char('g')), press(KeyCode::Char('g'))]
                ),
                Some(&Action::Move(Movement::GotoTop))
            );

            Ok(())
        }

        #[rstest]
        #[case(
            r#"{ "Normal": { "j": "Move(Down)", "k": "Move(Up)", "gg": "Move(GotoTop)" } }"#,
            3
        )]
        #[case(r#"{ "Normal": {} }"#, 0)]
        fn binding_count_matches(
            #[case] json: &str,
            #[case] expected: usize,
        ) -> Result<(), serde_json::Error> {
            let mut builder: KeyBindingsBuilder = serde_json::from_str(json)?;
            let bindings = builder.build();
            let count = bindings.get(&Mode::Normal).map_or(0, |m| m.iter().count());

            assert_eq!(count, expected);

            Ok(())
        }

        #[rstest]
        fn unknown_key_spec_is_rejected() {
            let json = r#"{ "Normal": { "<nope>": "Quit" } }"#;
            assert!(serde_json::from_str::<KeyBindingsBuilder>(json).is_err());
        }

        #[rstest]
        fn unknown_action_is_rejected() {
            let json = r#"{ "Normal": { "j": "NotAnAction" } }"#;
            assert!(serde_json::from_str::<KeyBindingsBuilder>(json).is_err());
        }

        #[rstest]
        fn special_key_deserializes() -> Result<(), serde_json::Error> {
            let json = r#"{ "Normal": { "<C-d>": "Move(ScrollDown)" } }"#;
            let bindings: KeyBindings = serde_json::from_str::<KeyBindingsBuilder>(json)?.build();

            assert_eq!(
                lookup(
                    &bindings,
                    Mode::Normal,
                    &[KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)]
                ),
                Some(&Action::Move(Movement::ScrollDown))
            );

            Ok(())
        }
    }
}
