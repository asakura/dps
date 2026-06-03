//! Mutable keybinding accumulator [`KeyBindingsBuilder`].
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

use super::registry::KeyBindings;

use crate::action::Action;
use crate::keymap::{
    keys::parse_key_sequence,
    map::{ModeMap, ModeMapBuilder},
    mode::Mode,
    seq::KeySeq,
};

use serde::{Deserialize, Deserializer};

use std::collections::HashMap;

/// Mutable accumulator that produces an immutable [`KeyBindings`] on [`build`](Self::build).
///
/// Supports a configurable `<leader>` token in key sequence strings. Any
/// occurrence of the literal text `<leader>` in a config-file binding is
/// replaced with the leader string before the sequence is parsed. The
/// substitution happens in [`build_with_leader`](Self::build_with_leader);
/// [`build`](Self::build) is a shorthand that uses `<Space>` as the default.
///
/// Key sequences added via the programmatic [`bind`](Self::bind) /
/// [`bind_default`](Self::bind_default) API are already-parsed [`KeySeq`]
/// values and are never subject to leader substitution.
#[derive(Clone, Debug, Default)]
pub struct KeyBindingsBuilder {
    /// Entries added via the programmatic API — already resolved.
    explicit: HashMap<Mode, ModeMapBuilder>,
    /// Entries from deserialization — raw key strings awaiting leader resolution.
    pending: Vec<(Mode, String, Action)>,
    /// Raw defaults merged in via [`merge_defaults`](Self::merge_defaults).
    pending_defaults: Vec<(Mode, String, Action)>,
}

impl KeyBindingsBuilder {
    /// Creates an empty builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::keymap::KeyBindingsBuilder;
    ///
    /// let _builder = KeyBindingsBuilder::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds `seq` to `action` in `mode`, overwriting any existing binding.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::keymap::{KeyBindingsBuilder, KeySeq, Mode, keys::parse_key_sequence};
    /// use dps::action::{Action, Movement};
    ///
    /// let mut b = KeyBindingsBuilder::new();
    /// b.bind(Mode::Normal, KeySeq::from(parse_key_sequence("j").unwrap()), Action::Move(Movement::Down));
    /// assert!(b.build().get(&Mode::Normal).is_some());
    /// ```
    pub fn bind(&mut self, mode: Mode, seq: KeySeq, action: Action) -> &mut Self {
        self.explicit.entry(mode).or_default().bind(seq, action);

        self
    }

    /// Binds `seq` to `action` in `mode` only if `seq` is not already bound.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::keymap::{KeyBindingsBuilder, KeySeq, Mode, keys::parse_key_sequence};
    /// use dps::action::{Action, Movement};
    ///
    /// let mut b = KeyBindingsBuilder::new();
    /// b.bind(Mode::Normal, KeySeq::from(parse_key_sequence("j").unwrap()), Action::Move(Movement::Down));
    /// b.bind_default(Mode::Normal, KeySeq::from(parse_key_sequence("j").unwrap()), Action::Move(Movement::Up));
    /// // existing binding is kept
    /// assert_eq!(
    ///     b.build().get(&Mode::Normal).unwrap().get(&parse_key_sequence("j").unwrap()),
    ///     Some(&Action::Move(Movement::Down)),
    /// );
    /// ```
    pub fn bind_default(&mut self, mode: Mode, seq: KeySeq, action: Action) -> &mut Self {
        self.explicit
            .entry(mode)
            .or_default()
            .bind_default(seq, action);

        self
    }

    /// Copies every binding from `defaults` that is not already present in `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::keymap::{KeyBindingsBuilder, KeySeq, Mode, keys::parse_key_sequence};
    /// use dps::action::{Action, Movement};
    ///
    /// let mut defaults = KeyBindingsBuilder::new();
    /// defaults.bind(Mode::Normal, KeySeq::from(parse_key_sequence("j").unwrap()), Action::Move(Movement::Down));
    ///
    /// let bindings = KeyBindingsBuilder::new().merge_defaults(&defaults).build();
    /// assert!(bindings.get(&Mode::Normal).is_some());
    /// ```
    pub fn merge_defaults(&mut self, defaults: &Self) -> &mut Self {
        for (mode, def_map) in &defaults.explicit {
            self.explicit
                .entry(*mode)
                .or_default()
                .merge_defaults_from(def_map);
        }

        self.pending_defaults
            .extend(defaults.pending.iter().cloned());

        self.pending_defaults
            .extend(defaults.pending_defaults.iter().cloned());

        self
    }

    /// Returns an immutable [`KeyBindings`] using `<Space>` as the leader key.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::keymap::KeyBindingsBuilder;
    ///
    /// let _bindings = KeyBindingsBuilder::new().build();
    /// ```
    #[must_use]
    pub fn build(&mut self) -> KeyBindings {
        self.build_with_leader("<Space>")
    }

    /// Returns an immutable [`KeyBindings`], substituting `leader` for every
    /// occurrence of `<leader>` in deserialized key strings before parsing them.
    ///
    /// User-config pending entries are added with [`bind`](ModeMapBuilder::bind)
    /// (override priority); default pending entries are added with
    /// [`bind_default`](ModeMapBuilder::bind_default) (fill-in priority).
    /// Modes are stored in sorted order so iteration is deterministic.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::keymap::KeyBindingsBuilder;
    ///
    /// let _bindings = KeyBindingsBuilder::new().build_with_leader(",");
    /// ```
    #[must_use]
    pub fn build_with_leader(&mut self, leader: &str) -> KeyBindings {
        let pending = std::mem::take(&mut self.pending);
        let pending_defaults = std::mem::take(&mut self.pending_defaults);

        for (mode, key_str, action) in pending {
            let resolved = key_str.replace("<leader>", leader);

            if let Ok(seq) = parse_key_sequence(&resolved) {
                self.explicit
                    .entry(mode)
                    .or_default()
                    .bind(KeySeq::from(seq), action);
            }
        }

        for (mode, key_str, action) in pending_defaults {
            let resolved = key_str.replace("<leader>", leader);

            if let Ok(seq) = parse_key_sequence(&resolved) {
                self.explicit
                    .entry(mode)
                    .or_default()
                    .bind_default(KeySeq::from(seq), action);
            }
        }

        let mut pairs: Vec<(Mode, ModeMap)> = std::mem::take(&mut self.explicit)
            .into_iter()
            .map(|(mode, mut builder)| (mode, builder.build()))
            .collect();

        pairs.sort_by_key(|(m, _)| *m);

        KeyBindings::from_sorted_pairs(pairs.into_boxed_slice())
    }
}

impl<'de> Deserialize<'de> for KeyBindingsBuilder {
    /// Deserializes from a `Mode → { key_string → action }` map.
    ///
    /// Key strings use Vim notation. `<leader>` is a placeholder resolved
    /// at [`build`](Self::build) / [`build_with_leader`](Self::build_with_leader) time.
    ///
    /// # Errors
    ///
    /// Returns a deserialization error if any key spec (with `<leader>`
    /// replaced by `<Space>`) fails to parse, or if any action string is
    /// unrecognised.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::keymap::{KeyBindingsBuilder, Mode};
    ///
    /// let mut b: KeyBindingsBuilder =
    ///     serde_json::from_str(r#"{"Normal": {"j": "Move(Down)"}}"#).unwrap();
    /// let bindings = b.build();
    /// assert!(bindings.get(&Mode::Normal).is_some());
    /// ```
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let parsed_map = HashMap::<Mode, HashMap<String, Action>>::deserialize(deserializer)?;

        let pending = parsed_map
            .into_iter()
            .flat_map(|(mode, inner_map)| {
                inner_map
                    .into_iter()
                    .map(move |(key_str, action)| (mode, key_str, action))
            })
            .map(|(mode, key_str, action)| {
                // Validate early: substitute <Space> for <leader> so bad key specs
                // surface at config-load time rather than silently at build time.
                let validation_str = key_str.replace("<leader>", "<Space>");

                parse_key_sequence(&validation_str).map_err(serde::de::Error::custom)?;

                Ok((mode, key_str, action))
            })
            .collect::<Result<Vec<_>, D::Error>>()?;

        Ok(Self {
            explicit: HashMap::new(),
            pending,
            pending_defaults: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::action::{Action, Movement, TabMotion};
    use crate::keymap::{
        KeyBindings,
        testutil::{press, single},
    };

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use rstest::fixture;
    use rstest::rstest;

    fn lookup<'a>(b: &'a KeyBindings, mode: Mode, keys: &[KeyEvent]) -> Option<&'a Action> {
        b.get(&mode).and_then(|m| m.get(keys))
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
                lookup(
                    overwritten_binding,
                    Mode::Normal,
                    [press(KeyCode::Char('j'))].as_slice()
                ),
                Some(&Action::Move(Movement::Up))
            );
        }

        #[rstest]
        fn bind_default_does_not_overwrite(default_not_overwritten: &KeyBindings) {
            assert_eq!(
                lookup(
                    default_not_overwritten,
                    Mode::Normal,
                    [press(KeyCode::Char('j'))].as_slice()
                ),
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
                lookup(
                    merged_with_defaults,
                    Mode::Normal,
                    [press(KeyCode::Char('j'))].as_slice()
                ),
                Some(&Action::Move(Movement::Down))
            );
            assert_eq!(
                lookup(
                    merged_with_defaults,
                    Mode::Normal,
                    [press(KeyCode::Char('x'))].as_slice()
                ),
                Some(&Action::Quit)
            );
        }

        #[rstest]
        fn merge_defaults_does_not_overwrite_user_binding(user_overrides_merged: &KeyBindings) {
            assert_eq!(
                lookup(
                    user_overrides_merged,
                    Mode::Normal,
                    [press(KeyCode::Char('j'))].as_slice()
                ),
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
                lookup(
                    &bindings,
                    Mode::Normal,
                    [press(KeyCode::Char('j'))].as_slice()
                ),
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
                    [press(KeyCode::Char('g')), press(KeyCode::Char('g'))].as_slice()
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
                    [KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)].as_slice()
                ),
                Some(&Action::Move(Movement::ScrollDown))
            );

            Ok(())
        }
    }

    mod leader {
        use super::*;

        #[rstest]
        fn leader_token_resolves_to_space_by_default() -> Result<(), serde_json::Error> {
            let json = r#"{ "Normal": { "<leader>j": "Tab(Next)" } }"#;
            let bindings: KeyBindings = serde_json::from_str::<KeyBindingsBuilder>(json)?.build();

            assert_eq!(
                lookup(
                    &bindings,
                    Mode::Normal,
                    [
                        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
                        press(KeyCode::Char('j')),
                    ]
                    .as_slice()
                ),
                Some(&Action::Tab(TabMotion::Next))
            );

            Ok(())
        }

        #[rstest]
        fn custom_leader_substitutes_correctly() -> Result<(), serde_json::Error> {
            let json = r#"{ "Normal": { "<leader>j": "Tab(Next)" } }"#;
            let bindings: KeyBindings =
                serde_json::from_str::<KeyBindingsBuilder>(json)?.build_with_leader("<C-a>");

            assert_eq!(
                lookup(
                    &bindings,
                    Mode::Normal,
                    [
                        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
                        press(KeyCode::Char('j')),
                    ]
                    .as_slice()
                ),
                Some(&Action::Tab(TabMotion::Next))
            );

            Ok(())
        }

        #[rstest]
        fn invalid_key_spec_still_rejected_at_deserialize_time() {
            let json = r#"{ "Normal": { "<leader><nope>": "Quit" } }"#;
            assert!(serde_json::from_str::<KeyBindingsBuilder>(json).is_err());
        }

        #[rstest]
        fn leader_in_default_binding_is_resolved_when_merged() -> Result<(), serde_json::Error> {
            let default_json = r#"{ "Normal": { "<leader>]": "Tab(Next)" } }"#;
            let defaults: KeyBindingsBuilder = serde_json::from_str(default_json)?;
            let mut user = KeyBindingsBuilder::new();

            user.merge_defaults(&defaults);

            let bindings = user.build_with_leader("<C-a>");

            assert_eq!(
                lookup(
                    &bindings,
                    Mode::Normal,
                    [
                        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
                        press(KeyCode::Char(']')),
                    ]
                    .as_slice()
                ),
                Some(&Action::Tab(TabMotion::Next))
            );

            Ok(())
        }
    }
}
