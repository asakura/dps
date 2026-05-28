//! Per-mode key-sequence-to-action map and its builder.

use super::seq::KeySeq;
use crate::action::Action;
use crossterm::event::KeyEvent;
use std::collections::HashMap;

/// Read-only key-sequence-to-action map for a single application [`Mode`](super::Mode).
///
/// Constructed via [`ModeMapBuilder`]; immutable thereafter.
/// Backed by a `HashMap` (O(1) exact lookup); excess bucket allocation is
/// eliminated by [`shrink_to_fit`](HashMap::shrink_to_fit) at build time.
#[derive(Clone, Debug, Default)]
pub struct ModeMap(HashMap<KeySeq, Action>);

impl ModeMap {
    /// Returns the action bound to `seq`, or `None` if unbound.
    #[must_use]
    pub fn get(&self, seq: &[KeyEvent]) -> Option<&Action> {
        self.0.get(seq)
    }

    /// Returns an iterator over `(&KeySeq, &Action)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&KeySeq, &Action)> {
        self.0.iter()
    }
}

/// Mutable accumulator that produces an immutable [`ModeMap`] on [`build`](Self::build).
#[derive(Clone, Debug, Default)]
pub struct ModeMapBuilder(HashMap<KeySeq, Action>);

impl ModeMapBuilder {
    /// Creates an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds `seq` to `action`, overwriting any existing binding for that sequence.
    pub fn bind(&mut self, seq: KeySeq, action: Action) -> &mut Self {
        self.0.insert(seq, action);

        self
    }

    /// Binds `seq` to `action` only if `seq` is not already bound.
    pub fn bind_default(&mut self, seq: KeySeq, action: Action) -> &mut Self {
        self.0.entry(seq).or_insert(action);

        self
    }

    /// Copies every binding from `other` that is not already present in `self`.
    pub(super) fn merge_defaults_from(&mut self, other: &Self) {
        for (seq, action) in &other.0 {
            self.0.entry(seq.clone()).or_insert_with(|| action.clone());
        }
    }

    /// Drains the builder and returns an immutable [`ModeMap`].
    ///
    /// Calls [`shrink_to_fit`](HashMap::shrink_to_fit) to release excess bucket
    /// allocation before freezing. The builder is left empty and may be reused.
    #[must_use]
    pub fn build(&mut self) -> ModeMap {
        let mut map = std::mem::take(&mut self.0);
        map.shrink_to_fit();

        ModeMap(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, Movement};
    use crossterm::event::KeyCode;
    use rstest::fixture;
    use rstest::rstest;

    use crate::keymap::testutil::{press, single};

    #[fixture]
    #[once]
    fn simple_map() -> ModeMap {
        ModeMapBuilder::new()
            .bind(single(KeyCode::Char('j')), Action::Move(Movement::Down))
            .bind(single(KeyCode::Char('k')), Action::Move(Movement::Up))
            .bind(
                KeySeq::from(vec![press(KeyCode::Char('g')), press(KeyCode::Char('g'))]),
                Action::Move(Movement::GotoTop),
            )
            .build()
    }

    mod mode_map {
        use super::*;

        #[rstest]
        fn bound_single_key_returns_action(simple_map: &ModeMap) {
            assert_eq!(
                simple_map.get(&[press(KeyCode::Char('j'))]),
                Some(&Action::Move(Movement::Down))
            );
        }

        #[rstest]
        fn unbound_key_returns_none(simple_map: &ModeMap) {
            assert_eq!(simple_map.get(&[press(KeyCode::Char('x'))]), None);
        }

        #[rstest]
        fn multi_key_chord_lookup(simple_map: &ModeMap) {
            let gg = &[press(KeyCode::Char('g')), press(KeyCode::Char('g'))];
            assert_eq!(simple_map.get(gg), Some(&Action::Move(Movement::GotoTop)));
        }

        #[rstest]
        fn iter_yields_all_bindings(simple_map: &ModeMap) {
            assert_eq!(simple_map.iter().count(), 3);
        }
    }

    mod mode_map_builder {
        use super::*;

        #[fixture]
        fn builder_j_down() -> ModeMapBuilder {
            let mut b = ModeMapBuilder::new();
            b.bind(single(KeyCode::Char('j')), Action::Move(Movement::Down));
            b
        }

        #[fixture]
        fn defaults_builder() -> ModeMapBuilder {
            let mut b = ModeMapBuilder::new();
            b.bind(single(KeyCode::Char('k')), Action::Move(Movement::Up))
                .bind(single(KeyCode::Char('j')), Action::Quit); // should not overwrite base
            b
        }

        #[rstest]
        fn bind_overwrites_existing_entry(mut builder_j_down: ModeMapBuilder) {
            let map = builder_j_down
                .bind(single(KeyCode::Char('j')), Action::Move(Movement::Up))
                .build();

            assert_eq!(
                map.get(&[press(KeyCode::Char('j'))]),
                Some(&Action::Move(Movement::Up))
            );
        }

        #[rstest]
        fn bind_default_preserves_existing_entry(mut builder_j_down: ModeMapBuilder) {
            let map = builder_j_down
                .bind_default(single(KeyCode::Char('j')), Action::Move(Movement::Up))
                .build();

            assert_eq!(
                map.get(&[press(KeyCode::Char('j'))]),
                Some(&Action::Move(Movement::Down))
            );
        }

        #[test]
        fn bind_default_fills_absent_key() {
            let map = ModeMapBuilder::new()
                .bind_default(single(KeyCode::Char('j')), Action::Move(Movement::Down))
                .build();

            assert_eq!(
                map.get(&[press(KeyCode::Char('j'))]),
                Some(&Action::Move(Movement::Down))
            );
        }

        #[rstest]
        fn merge_defaults_from_fills_missing_keys(
            mut builder_j_down: ModeMapBuilder,
            defaults_builder: ModeMapBuilder,
        ) {
            builder_j_down.merge_defaults_from(&defaults_builder);
            let map = builder_j_down.build();

            assert_eq!(
                map.get(&[press(KeyCode::Char('k'))]),
                Some(&Action::Move(Movement::Up))
            );
            assert_eq!(
                map.get(&[press(KeyCode::Char('j'))]),
                Some(&Action::Move(Movement::Down))
            );
        }

        #[rstest]
        #[case(0)]
        #[case(1)]
        #[case(10)]
        fn build_produces_map_with_correct_entry_count(#[case] n: u8) {
            let mut b = ModeMapBuilder::new();

            for i in 0..n {
                b.bind(single(KeyCode::Char(i as char)), Action::Quit);
            }

            assert_eq!(b.build().iter().count(), n as usize);
        }
    }
}
