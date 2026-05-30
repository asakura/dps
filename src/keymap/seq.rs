//! Key-sequence newtype: a boxed slice of key events forming a binding key.

use super::keys::key_event_to_string;

use crossterm::event::KeyEvent;

use std::{borrow::Borrow, fmt, ops::Deref};

/// A parsed key sequence: one or more [`KeyEvent`]s that together form a binding key.
///
/// Stored as a boxed slice — no excess allocation after construction.
/// Build one with [`From<Vec<KeyEvent>>`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeySeq(Box<[KeyEvent]>);

impl KeySeq {
    /// Returns the sequence as a slice of [`KeyEvent`]s.
    #[must_use]
    pub fn as_slice(&self) -> &[KeyEvent] {
        &self.0
    }
}

/// Converts an owned `Vec<KeyEvent>` into a [`KeySeq`], boxing the slice to
/// eliminate excess allocation.
impl From<Vec<KeyEvent>> for KeySeq {
    fn from(v: Vec<KeyEvent>) -> Self {
        Self(v.into_boxed_slice())
    }
}

/// Allows `&KeySeq` to coerce to `&[KeyEvent]` transparently, enabling
/// slice methods (e.g. [`starts_with`](slice::starts_with)) directly on a
/// `KeySeq` reference.
impl Deref for KeySeq {
    type Target = [KeyEvent];

    fn deref(&self) -> &[KeyEvent] {
        &self.0
    }
}

/// Allows a `HashMap<KeySeq, _>` to be queried with a plain `&[KeyEvent]`
/// without allocating a `KeySeq`.  This is the key that makes
/// [`ModeMap::get`](super::ModeMap::get) accept a raw slice.
impl Borrow<[KeyEvent]> for KeySeq {
    fn borrow(&self) -> &[KeyEvent] {
        &self.0
    }
}

/// Renders the sequence in Vim notation (e.g. `"gg"`, `"<C-d>"`, `"<C-w>j"`).
impl fmt::Display for KeySeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for key in &self.0 {
            f.write_str(&key_event_to_string(key))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::keymap::testutil::press;

    use crossterm::event::{KeyCode, KeyModifiers};
    use rstest::rstest;

    use std::borrow::Borrow;
    use std::collections::HashMap;

    mod from_vec {
        use super::*;

        #[test]
        fn empty_vec_produces_empty_seq() {
            let seq = KeySeq::from(vec![]);
            assert_eq!(seq.as_slice(), &[]);
        }

        #[rstest]
        #[case(KeyCode::Char('a'))]
        #[case(KeyCode::Esc)]
        #[case(KeyCode::Enter)]
        fn single_key_stored_correctly(#[case] code: KeyCode) {
            let key = press(code);
            let seq = KeySeq::from(vec![key]);

            assert_eq!(seq.as_slice(), &[key]);
        }

        #[test]
        fn multi_key_seq_preserves_order() {
            let keys = vec![press(KeyCode::Char('g')), press(KeyCode::Char('g'))];
            let seq = KeySeq::from(keys.clone());

            assert_eq!(seq.as_slice(), keys.as_slice());
        }
    }

    mod deref_and_borrow {
        use super::*;

        #[test]
        fn deref_yields_key_slice() {
            let keys = vec![press(KeyCode::Char('j'))];
            let seq = KeySeq::from(keys.clone());
            let slice: &[KeyEvent] = &seq;

            assert_eq!(slice, keys.as_slice());
        }

        #[test]
        fn borrow_yields_key_slice() {
            let keys = vec![press(KeyCode::Char('k'))];
            let seq = KeySeq::from(keys.clone());
            let borrowed: &[KeyEvent] = seq.borrow();

            assert_eq!(borrowed, keys.as_slice());
        }
    }

    mod equality_and_hash {
        use super::*;

        #[test]
        fn equal_seqs_compare_equal() {
            let a = KeySeq::from(vec![press(KeyCode::Char('g')), press(KeyCode::Char('g'))]);
            let b = KeySeq::from(vec![press(KeyCode::Char('g')), press(KeyCode::Char('g'))]);

            assert_eq!(a, b);
        }

        #[test]
        fn different_seqs_compare_unequal() {
            let a = KeySeq::from(vec![press(KeyCode::Char('j'))]);
            let b = KeySeq::from(vec![press(KeyCode::Char('k'))]);

            assert_ne!(a, b);
        }

        #[test]
        fn seq_usable_as_hashmap_key() {
            let mut map: HashMap<KeySeq, u32> = HashMap::new();
            let seq = KeySeq::from(vec![press(KeyCode::Char('g')), press(KeyCode::Char('g'))]);

            map.insert(seq.clone(), 42);
            assert_eq!(map.get(&seq), Some(&42));
        }

        #[test]
        fn hashmap_lookup_by_borrowed_slice() {
            let mut map: HashMap<KeySeq, u32> = HashMap::new();
            let seq = KeySeq::from(vec![press(KeyCode::Char('g'))]);

            map.insert(seq, 1);

            let slice = [press(KeyCode::Char('g'))];
            assert_eq!(map.get(slice.as_slice()), Some(&1));
        }
    }

    mod display {
        use super::*;

        #[rstest]
        #[case(vec![press(KeyCode::Char('j'))], "j")]
        #[case(vec![press(KeyCode::Char('g')), press(KeyCode::Char('g'))], "gg")]
        #[case(vec![KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)], "<C-d>")]
        #[case(vec![KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL), press(KeyCode::Char('j'))], "<C-w>j")]
        #[case(vec![press(KeyCode::Esc)], "<Esc>")]
        #[case(vec![press(KeyCode::F(5))], "<F5>")]
        fn formats_as_vim_notation(#[case] events: Vec<KeyEvent>, #[case] expected: &str) {
            assert_eq!(KeySeq::from(events).to_string(), expected);
        }

        #[test]
        fn empty_seq_displays_as_empty_string() {
            assert_eq!(KeySeq::from(vec![]).to_string(), "");
        }
    }
}
