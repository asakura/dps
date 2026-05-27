//! Key-sequence matching and multi-key chord accumulation.
//!
//! The central abstraction is [`ChordEngine`], a trait for stateful
//! key-sequence matchers.  The bundled [`SequenceEngine`] is the buffer-based
//! implementation used by [`App`](crate::app::App).
//!
//! # Usage
//!
//! Build a [`ModeMap`], create a [`SequenceEngine`], then feed key events to
//! [`ChordEngine::advance`] one at a time and act on the returned
//! [`ChordResult`]:
//!
//! ```no_run
//! use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
//! use dps::action::{Action, Movement};
//! use dps::keymap::{ChordEngine, ChordResult, KeySeq, ModeMapBuilder, SequenceEngine};
//!
//! fn press(code: KeyCode) -> KeyEvent {
//!     KeyEvent::new(code, KeyModifiers::NONE)
//! }
//!
//! let mut builder = ModeMapBuilder::new();
//! builder.bind(
//!     KeySeq::from(vec![press(KeyCode::Char('g')), press(KeyCode::Char('g'))]),
//!     Action::Move(Movement::GotoTop),
//! );
//! let bindings = builder.build();
//!
//! let mut engine = SequenceEngine::default();
//!
//! // First 'g' is a prefix — keep accumulating.
//! assert!(matches!(engine.advance(press(KeyCode::Char('g')), &bindings), ChordResult::Prefix));
//!
//! // Second 'g' completes the chord.
//! assert!(matches!(
//!     engine.advance(press(KeyCode::Char('g')), &bindings),
//!     ChordResult::Exact(Action::Move(Movement::GotoTop)),
//! ));
//! ```

use crossterm::event::KeyEvent;

use crate::action::Action;

use super::ModeMap;

/// Result of advancing the chord engine by one key event.
///
/// The caller uses this to decide whether to dispatch an action, continue
/// buffering, or fall through to hardcoded global key handling.
#[derive(Debug)]
pub enum ChordResult {
    /// A configured binding matched exactly; the action is ready to dispatch.
    Exact(Action),
    /// The accumulated buffer is a prefix of at least one binding.
    ///
    /// The engine is still accumulating; the caller should wait for the next
    /// key without invoking any fallback handler.
    Prefix,
    /// No configured binding matched.
    ///
    /// The engine has cleared its buffer.  The caller should pass the original
    /// key to a global fallback handler.
    NoMatch,
}

/// A stateful key-sequence accumulator that resolves multi-key chords.
///
/// Each call to [`advance`](ChordEngine::advance) appends one key event,
/// tests the accumulated buffer against the provided binding map, and returns a
/// [`ChordResult`].  Implementations are responsible for managing internal
/// buffer state: clearing after an exact match and clearing (or retrying) on a
/// no-match.
///
/// # Implementing `ChordEngine`
///
/// The only requirement is `advance`.  A minimal pass-through that always
/// reports no match:
///
/// ```no_run
/// use crossterm::event::KeyEvent;
/// use dps::action::Action;
/// use dps::keymap::{ChordEngine, ChordResult, ModeMap};
///
/// struct AlwaysNoMatch;
///
/// impl ChordEngine for AlwaysNoMatch {
///     fn advance(
///         &mut self,
///         _key: KeyEvent,
///         _bindings: &ModeMap,
///     ) -> ChordResult {
///         ChordResult::NoMatch
///     }
/// }
/// ```
pub trait ChordEngine {
    /// Advance the engine by one key event and report the outcome.
    ///
    /// `bindings` is the active mode's key-sequence-to-action map.  The engine
    /// matches the accumulated buffer as a prefix or exact sequence; it does
    /// **not** look up the mode — the caller resolves the active mode and passes
    /// the corresponding [`ModeMap`].
    ///
    /// After an [`Exact`](ChordResult::Exact) match the engine resets so the
    /// next call begins a fresh sequence.
    fn advance(&mut self, key: KeyEvent, bindings: &ModeMap) -> ChordResult;
}

/// Buffer-based implementation of [`ChordEngine`].
///
/// Key events are accumulated in an internal `Vec`.  The engine tries three
/// outcomes on each call to [`advance`](ChordEngine::advance):
///
/// 1. **Exact match** — the buffer equals a binding key sequence.  The buffer
///    is cleared and [`ChordResult::Exact`] is returned.
/// 2. **Prefix match** — the buffer is a strict prefix of at least one binding.
///    The buffer is kept and [`ChordResult::Prefix`] is returned.
/// 3. **No match** — neither exact nor prefix.  If the buffer held more than
///    one key (*chord break*), the breaking key is automatically retried as the
///    start of a fresh sequence before returning [`ChordResult::NoMatch`], so
///    the caller always receives the most informative result for that key.
///
/// # Examples
///
/// Single-key binding:
///
/// ```no_run
/// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
/// use dps::action::Action;
/// use dps::keymap::{ChordEngine, ChordResult, KeySeq, ModeMapBuilder, SequenceEngine};
///
/// fn press(code: KeyCode) -> KeyEvent {
///     KeyEvent::new(code, KeyModifiers::NONE)
/// }
///
/// let mut builder = ModeMapBuilder::new();
/// builder.bind(KeySeq::from(vec![press(KeyCode::Char('q'))]), Action::Quit);
/// let bindings = builder.build();
///
/// let mut engine = SequenceEngine::default();
/// assert!(matches!(
///     engine.advance(press(KeyCode::Char('q')), &bindings),
///     ChordResult::Exact(Action::Quit),
/// ));
/// ```
///
/// Chord break with automatic retry — `'j'` breaks `"gg"` and is immediately
/// retried against the binding map, where it matches `Action::Move(Down)`:
///
/// ```no_run
/// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
/// use dps::action::{Action, Movement};
/// use dps::keymap::{ChordEngine, ChordResult, KeySeq, ModeMapBuilder, SequenceEngine};
///
/// fn press(code: KeyCode) -> KeyEvent {
///     KeyEvent::new(code, KeyModifiers::NONE)
/// }
///
/// let mut builder = ModeMapBuilder::new();
/// builder.bind(
///     KeySeq::from(vec![press(KeyCode::Char('g')), press(KeyCode::Char('g'))]),
///     Action::Move(Movement::GotoTop),
/// );
/// builder.bind(
///     KeySeq::from(vec![press(KeyCode::Char('j'))]),
///     Action::Move(Movement::Down),
/// );
/// let bindings = builder.build();
///
/// let mut engine = SequenceEngine::default();
/// engine.advance(press(KeyCode::Char('g')), &bindings); // Prefix
///
/// // 'j' breaks "gg"; retried alone it matches "j" → Down.
/// assert!(matches!(
///     engine.advance(press(KeyCode::Char('j')), &bindings),
///     ChordResult::Exact(Action::Move(Movement::Down)),
/// ));
/// ```
#[derive(Debug, Default)]
pub struct SequenceEngine {
    buffer: Vec<KeyEvent>,
}

impl SequenceEngine {
    fn match_buffer(&self, bindings: &ModeMap) -> ChordResult {
        let mut exact: Option<Action> = None;
        let mut has_prefix = false;

        for (seq, action) in bindings.iter() {
            if seq.as_slice() == self.buffer.as_slice() {
                exact = Some(action.clone());
            } else if seq.starts_with(&self.buffer) {
                has_prefix = true;
            }
        }

        match (exact, has_prefix) {
            (Some(action), _) => ChordResult::Exact(action),
            (None, true) => ChordResult::Prefix,
            (None, false) => ChordResult::NoMatch,
        }
    }
}

impl ChordEngine for SequenceEngine {
    fn advance(&mut self, key: KeyEvent, bindings: &ModeMap) -> ChordResult {
        self.buffer.push(key);

        match self.match_buffer(bindings) {
            ChordResult::Exact(action) => {
                self.buffer.clear();
                ChordResult::Exact(action)
            }
            ChordResult::Prefix => ChordResult::Prefix,
            ChordResult::NoMatch => {
                let was_chord = self.buffer.len() > 1;
                self.buffer.clear();

                if was_chord {
                    self.buffer.push(key);

                    match self.match_buffer(bindings) {
                        ChordResult::Exact(action) => {
                            self.buffer.clear();
                            return ChordResult::Exact(action);
                        }
                        ChordResult::Prefix => return ChordResult::Prefix,
                        ChordResult::NoMatch => self.buffer.clear(),
                    }
                }

                ChordResult::NoMatch
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Movement;
    use crate::keymap::{KeySeq, ModeMapBuilder};
    use crossterm::event::KeyCode;
    use rstest::fixture;
    use rstest::rstest;

    use crate::keymap::testutil::press;

    fn bindings(pairs: &[(&[KeyCode], Action)]) -> ModeMap {
        let mut builder = ModeMapBuilder::new();

        for (codes, action) in pairs {
            builder.bind(
                KeySeq::from(codes.iter().map(|&c| press(c)).collect::<Vec<_>>()),
                action.clone(),
            );
        }

        builder.build()
    }

    #[fixture]
    fn engine() -> SequenceEngine {
        SequenceEngine::default()
    }

    #[fixture]
    #[once]
    fn q_bindings() -> ModeMap {
        bindings([([KeyCode::Char('q')].as_slice(), Action::Quit)].as_slice())
    }

    #[fixture]
    #[once]
    fn empty_bindings() -> ModeMap {
        bindings([].as_slice())
    }

    /// Single binding `gg → GotoTop`, shared by tests that only need that map.
    #[fixture]
    #[once]
    fn gg_bindings() -> ModeMap {
        bindings(
            [(
                [KeyCode::Char('g'), KeyCode::Char('g')].as_slice(),
                Action::Move(Movement::GotoTop),
            )]
            .as_slice(),
        )
    }

    #[fixture]
    #[once]
    fn abc_bindings() -> ModeMap {
        bindings(
            [(
                [KeyCode::Char('a'), KeyCode::Char('b'), KeyCode::Char('c')].as_slice(),
                Action::Move(Movement::GotoTop),
            )]
            .as_slice(),
        )
    }

    #[fixture]
    #[once]
    fn gg_j_bindings() -> ModeMap {
        bindings(
            [
                (
                    [KeyCode::Char('g'), KeyCode::Char('g')].as_slice(),
                    Action::Move(Movement::GotoTop),
                ),
                (
                    [KeyCode::Char('j')].as_slice(),
                    Action::Move(Movement::Down),
                ),
            ]
            .as_slice(),
        )
    }

    #[fixture]
    #[once]
    fn gg_jk_bindings() -> ModeMap {
        bindings(
            [
                (
                    [KeyCode::Char('g'), KeyCode::Char('g')].as_slice(),
                    Action::Move(Movement::GotoTop),
                ),
                (
                    [KeyCode::Char('j'), KeyCode::Char('k')].as_slice(),
                    Action::Move(Movement::ScrollUp),
                ),
            ]
            .as_slice(),
        )
    }

    #[rstest]
    fn single_key_exact_match_returns_exact(mut engine: SequenceEngine, q_bindings: &ModeMap) {
        assert!(matches!(
            engine.advance(press(KeyCode::Char('q')), q_bindings),
            ChordResult::Exact(Action::Quit)
        ));
    }

    #[rstest]
    fn unrecognised_key_returns_no_match(mut engine: SequenceEngine, empty_bindings: &ModeMap) {
        assert!(matches!(
            engine.advance(press(KeyCode::Char('x')), empty_bindings),
            ChordResult::NoMatch
        ));
    }

    #[rstest]
    fn first_key_of_chord_returns_prefix(mut engine: SequenceEngine, gg_bindings: &ModeMap) {
        assert!(matches!(
            engine.advance(press(KeyCode::Char('g')), gg_bindings),
            ChordResult::Prefix
        ));
    }

    #[rstest]
    fn completing_chord_returns_exact(mut engine: SequenceEngine, gg_bindings: &ModeMap) {
        engine.advance(press(KeyCode::Char('g')), gg_bindings);

        assert!(matches!(
            engine.advance(press(KeyCode::Char('g')), gg_bindings),
            ChordResult::Exact(Action::Move(Movement::GotoTop))
        ));
    }

    #[rstest]
    fn three_key_chord_accumulates_prefix_then_fires(
        mut engine: SequenceEngine,
        abc_bindings: &ModeMap,
    ) {
        assert!(matches!(
            engine.advance(press(KeyCode::Char('a')), abc_bindings),
            ChordResult::Prefix
        ));
        assert!(matches!(
            engine.advance(press(KeyCode::Char('b')), abc_bindings),
            ChordResult::Prefix
        ));
        assert!(matches!(
            engine.advance(press(KeyCode::Char('c')), abc_bindings),
            ChordResult::Exact(Action::Move(Movement::GotoTop))
        ));
    }

    #[rstest]
    fn exact_match_clears_buffer_for_next_sequence(
        mut engine: SequenceEngine,
        gg_bindings: &ModeMap,
    ) {
        engine.advance(press(KeyCode::Char('g')), gg_bindings);
        engine.advance(press(KeyCode::Char('g')), gg_bindings); // exact → buffer cleared

        // Fresh 'g' is a prefix again, not a misfire.
        assert!(matches!(
            engine.advance(press(KeyCode::Char('g')), gg_bindings),
            ChordResult::Prefix
        ));
    }

    #[rstest]
    fn chord_break_retries_breaking_key_as_exact_binding(
        mut engine: SequenceEngine,
        gg_j_bindings: &ModeMap,
    ) {
        engine.advance(press(KeyCode::Char('g')), gg_j_bindings); // Prefix

        // 'j' breaks "gg"; retried alone it matches → Exact(Down).
        assert!(matches!(
            engine.advance(press(KeyCode::Char('j')), gg_j_bindings),
            ChordResult::Exact(Action::Move(Movement::Down))
        ));
    }

    #[rstest]
    fn chord_break_retry_starts_new_prefix(mut engine: SequenceEngine, gg_jk_bindings: &ModeMap) {
        // Prefix of "gg"
        engine.advance(press(KeyCode::Char('g')), gg_jk_bindings);

        // 'j' breaks "gg"; retried alone it is a prefix of "jk".
        assert!(matches!(
            engine.advance(press(KeyCode::Char('j')), gg_jk_bindings),
            ChordResult::Prefix
        ));

        // 'k' completes "jk".
        assert!(matches!(
            engine.advance(press(KeyCode::Char('k')), gg_jk_bindings),
            ChordResult::Exact(Action::Move(Movement::ScrollUp))
        ));
    }

    #[rstest]
    fn chord_break_with_unbound_retry_returns_no_match(
        mut engine: SequenceEngine,
        gg_bindings: &ModeMap,
    ) {
        engine.advance(press(KeyCode::Char('g')), gg_bindings); // Prefix

        // 'q' breaks "gg"; retry of 'q' also has no binding → NoMatch.
        assert!(matches!(
            engine.advance(press(KeyCode::Char('q')), gg_bindings),
            ChordResult::NoMatch
        ));
    }
}
