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
//!     ChordResult::Exact(Action::Move(Movement::GotoTop), _),
//! ));
//! ```

use crossterm::event::{KeyCode, KeyEvent};

use crate::action::Action;

use super::ModeMap;

/// Result of advancing the chord engine by one key event.
///
/// The caller uses this to decide whether to dispatch an action, continue
/// buffering, or fall through to hardcoded global key handling.
#[derive(Debug)]
pub enum ChordResult {
    /// A configured binding matched exactly; the action is ready to dispatch.
    ///
    /// The `u32` is the repeat count extracted from a leading digit prefix
    /// (e.g. `5j` yields `count = 5`). It is always ≥ 1: no count typed
    /// means the action should fire once.
    Exact(Action, u32),
    /// The accumulated buffer is a prefix of at least one binding — or the
    /// engine is accumulating count-prefix digits.
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
///     ChordResult::Exact(Action::Quit, _),
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
///     ChordResult::Exact(Action::Move(Movement::Down), _),
/// ));
/// ```
#[derive(Debug, Default)]
pub struct SequenceEngine {
    buffer: Vec<KeyEvent>,
    count: u32,
    pending_register: Option<char>,
    awaiting_register: bool,
}

impl SequenceEngine {
    fn match_buffer(&self, bindings: &ModeMap) -> (Option<Action>, bool) {
        let mut exact: Option<Action> = None;
        let mut has_prefix = false;

        for (seq, action) in bindings.iter() {
            if seq.as_slice() == self.buffer.as_slice() {
                exact = Some(action.clone());
            } else if seq.starts_with(&self.buffer) {
                has_prefix = true;
            }
        }

        (exact, has_prefix)
    }

    fn take_count(&mut self) -> u32 {
        let c = self.count.max(1);
        self.count = 0;

        c
    }

    fn inject_register(&mut self, action: Action) -> Action {
        let Some(reg) = self.pending_register.take() else {
            return action;
        };
        match action {
            Action::Edit(op) => Action::Edit(op.with_register(Some(reg))),
            other => other,
        }
    }
}

impl ChordEngine for SequenceEngine {
    fn advance(&mut self, key: KeyEvent, bindings: &ModeMap) -> ChordResult {
        if self.buffer.is_empty() {
            // Register-select: consume the char typed after `"`
            if self.awaiting_register {
                if let KeyCode::Char(c) = key.code {
                    self.pending_register = Some(c);
                }
                self.awaiting_register = false;
                return ChordResult::Prefix;
            }

            match key.code {
                KeyCode::Char(c @ '1'..='9') => {
                    let digit = (c as u32) - ('0' as u32);
                    self.count = self.count.saturating_mul(10).saturating_add(digit);

                    return ChordResult::Prefix;
                }
                KeyCode::Char('0') if self.count > 0 => {
                    self.count = self.count.saturating_mul(10);

                    return ChordResult::Prefix;
                }
                KeyCode::Char('"') => {
                    self.awaiting_register = true;
                    return ChordResult::Prefix;
                }
                _ => {}
            }
        }

        self.buffer.push(key);

        match self.match_buffer(bindings) {
            (Some(action), _) => {
                self.buffer.clear();
                let count = self.take_count();
                let action = self.inject_register(action);

                ChordResult::Exact(action, count)
            }
            (None, true) => ChordResult::Prefix,
            (None, false) => {
                let was_chord = self.buffer.len() > 1;
                self.buffer.clear();

                if was_chord {
                    self.buffer.push(key);

                    match self.match_buffer(bindings) {
                        (Some(action), _) => {
                            self.buffer.clear();
                            let count = self.take_count();
                            let action = self.inject_register(action);

                            return ChordResult::Exact(action, count);
                        }
                        (None, true) => return ChordResult::Prefix,
                        (None, false) => {
                            self.buffer.clear();
                            self.count = 0;
                            self.pending_register = None;
                            self.awaiting_register = false;
                        }
                    }
                } else {
                    self.count = 0;
                    self.pending_register = None;
                    self.awaiting_register = false;
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
    use crate::keymap::testutil::press;
    use crate::keymap::{KeySeq, ModeMapBuilder};
    use crossterm::event::KeyCode;
    use rstest::{fixture, rstest};

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

    /// Single key `q → Quit`.
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

    /// Two-key chord `gg → GotoTop`.
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

    /// Three-key chord `abc → GotoTop`.
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

    /// `gg → GotoTop` and `j → Down` — used for chord-break and count tests.
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

    /// `gg → GotoTop` and `jk → ScrollUp`.
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

    /// Single key `j → Down` — used for count tests.
    #[fixture]
    #[once]
    fn j_bindings() -> ModeMap {
        bindings(
            [(
                [KeyCode::Char('j')].as_slice(),
                Action::Move(Movement::Down),
            )]
            .as_slice(),
        )
    }

    /// `0 → GotoTop` and `j → Down` — used to verify bare-zero disambiguation.
    #[fixture]
    #[once]
    fn zero_j_bindings() -> ModeMap {
        bindings(
            [
                (
                    [KeyCode::Char('0')].as_slice(),
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

    mod single_key {
        use super::*;

        #[rstest]
        fn exact_match_returns_exact(mut engine: SequenceEngine, q_bindings: &ModeMap) {
            assert!(matches!(
                engine.advance(press(KeyCode::Char('q')), q_bindings),
                ChordResult::Exact(Action::Quit, _)
            ));
        }

        #[rstest]
        fn unrecognised_key_returns_no_match(mut engine: SequenceEngine, empty_bindings: &ModeMap) {
            assert!(matches!(
                engine.advance(press(KeyCode::Char('x')), empty_bindings),
                ChordResult::NoMatch
            ));
        }
    }

    mod chord {
        use super::*;

        #[rstest]
        fn first_key_returns_prefix(mut engine: SequenceEngine, gg_bindings: &ModeMap) {
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
                ChordResult::Exact(Action::Move(Movement::GotoTop), _)
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
                ChordResult::Exact(Action::Move(Movement::GotoTop), _)
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
    }

    mod chord_break {
        use super::*;

        #[rstest]
        fn breaking_key_retried_as_exact_binding(
            mut engine: SequenceEngine,
            gg_j_bindings: &ModeMap,
        ) {
            engine.advance(press(KeyCode::Char('g')), gg_j_bindings); // Prefix

            // 'j' breaks "gg"; retried alone it matches → Exact(Down).
            assert!(matches!(
                engine.advance(press(KeyCode::Char('j')), gg_j_bindings),
                ChordResult::Exact(Action::Move(Movement::Down), _)
            ));
        }

        #[rstest]
        fn breaking_key_retried_as_new_prefix(
            mut engine: SequenceEngine,
            gg_jk_bindings: &ModeMap,
        ) {
            engine.advance(press(KeyCode::Char('g')), gg_jk_bindings); // Prefix of "gg"

            // 'j' breaks "gg"; retried alone it is a prefix of "jk".
            assert!(matches!(
                engine.advance(press(KeyCode::Char('j')), gg_jk_bindings),
                ChordResult::Prefix
            ));

            // 'k' completes "jk".
            assert!(matches!(
                engine.advance(press(KeyCode::Char('k')), gg_jk_bindings),
                ChordResult::Exact(Action::Move(Movement::ScrollUp), _)
            ));
        }

        #[rstest]
        fn unbound_retry_returns_no_match(mut engine: SequenceEngine, gg_bindings: &ModeMap) {
            engine.advance(press(KeyCode::Char('g')), gg_bindings); // Prefix

            // 'q' breaks "gg"; retry of 'q' also has no binding → NoMatch.
            assert!(matches!(
                engine.advance(press(KeyCode::Char('q')), gg_bindings),
                ChordResult::NoMatch
            ));
        }
    }

    mod count {
        use super::*;

        #[rstest]
        fn single_digit_prefix_fires_action_with_count(
            mut engine: SequenceEngine,
            j_bindings: &ModeMap,
        ) {
            engine.advance(press(KeyCode::Char('5')), j_bindings); // count digit → Prefix

            assert!(matches!(
                engine.advance(press(KeyCode::Char('j')), j_bindings),
                ChordResult::Exact(Action::Move(Movement::Down), 5)
            ));
        }

        #[rstest]
        fn multi_digit_count_accumulates(mut engine: SequenceEngine, j_bindings: &ModeMap) {
            engine.advance(press(KeyCode::Char('1')), j_bindings);
            engine.advance(press(KeyCode::Char('2')), j_bindings);

            assert!(matches!(
                engine.advance(press(KeyCode::Char('j')), j_bindings),
                ChordResult::Exact(Action::Move(Movement::Down), 12)
            ));
        }

        #[rstest]
        fn zero_extends_count_after_nonzero_digit(
            mut engine: SequenceEngine,
            j_bindings: &ModeMap,
        ) {
            engine.advance(press(KeyCode::Char('1')), j_bindings);
            engine.advance(press(KeyCode::Char('0')), j_bindings); // extends count to 10

            assert!(matches!(
                engine.advance(press(KeyCode::Char('j')), j_bindings),
                ChordResult::Exact(Action::Move(Movement::Down), 10)
            ));
        }

        #[rstest]
        fn bare_zero_resolves_as_binding_not_count(
            mut engine: SequenceEngine,
            zero_j_bindings: &ModeMap,
        ) {
            // '0' with no prior count digit must fire its binding, not accumulate count.
            assert!(matches!(
                engine.advance(press(KeyCode::Char('0')), zero_j_bindings),
                ChordResult::Exact(Action::Move(Movement::GotoTop), 1)
            ));
        }

        #[rstest]
        fn count_clears_after_exact_match(mut engine: SequenceEngine, j_bindings: &ModeMap) {
            engine.advance(press(KeyCode::Char('5')), j_bindings);
            engine.advance(press(KeyCode::Char('j')), j_bindings); // fires with count=5, clears

            // Next 'j' must have count=1 (no prior count).
            assert!(matches!(
                engine.advance(press(KeyCode::Char('j')), j_bindings),
                ChordResult::Exact(Action::Move(Movement::Down), 1)
            ));
        }

        #[rstest]
        fn count_clears_after_no_match(mut engine: SequenceEngine, j_bindings: &ModeMap) {
            engine.advance(press(KeyCode::Char('5')), j_bindings);
            engine.advance(press(KeyCode::Char('x')), j_bindings); // unbound → NoMatch, clears

            // Next 'j' must have count=1 (no prior count).
            assert!(matches!(
                engine.advance(press(KeyCode::Char('j')), j_bindings),
                ChordResult::Exact(Action::Move(Movement::Down), 1)
            ));
        }

        #[rstest]
        fn count_survives_chord_break_into_exact(
            mut engine: SequenceEngine,
            gg_j_bindings: &ModeMap,
        ) {
            engine.advance(press(KeyCode::Char('3')), gg_j_bindings); // count=3
            engine.advance(press(KeyCode::Char('g')), gg_j_bindings); // prefix of "gg"

            // 'j' breaks "gg"; retried as 'j' alone it matches — count must survive.
            assert!(matches!(
                engine.advance(press(KeyCode::Char('j')), gg_j_bindings),
                ChordResult::Exact(Action::Move(Movement::Down), 3)
            ));
        }
    }

    mod register {
        use super::*;
        use crate::action::EditOp;

        /// `dd → Edit(Delete)` binding used for register tests.
        #[fixture]
        #[once]
        fn dd_bindings() -> ModeMap {
            bindings(
                [(
                    [KeyCode::Char('d'), KeyCode::Char('d')].as_slice(),
                    Action::Edit(EditOp::Delete(None)),
                )]
                .as_slice(),
            )
        }

        #[rstest]
        fn quote_then_char_then_action_injects_register(
            mut engine: SequenceEngine,
            dd_bindings: &ModeMap,
        ) {
            engine.advance(press(KeyCode::Char('"')), dd_bindings); // awaiting register
            engine.advance(press(KeyCode::Char('a')), dd_bindings); // register = 'a'
            engine.advance(press(KeyCode::Char('d')), dd_bindings); // prefix

            assert!(matches!(
                engine.advance(press(KeyCode::Char('d')), dd_bindings),
                ChordResult::Exact(Action::Edit(EditOp::Delete(Some('a'))), 1)
            ));
        }

        #[rstest]
        fn count_then_quote_then_char_also_works(
            mut engine: SequenceEngine,
            dd_bindings: &ModeMap,
        ) {
            engine.advance(press(KeyCode::Char('3')), dd_bindings); // count = 3
            engine.advance(press(KeyCode::Char('"')), dd_bindings); // awaiting register
            engine.advance(press(KeyCode::Char('a')), dd_bindings); // register = 'a'
            engine.advance(press(KeyCode::Char('d')), dd_bindings); // prefix

            assert!(matches!(
                engine.advance(press(KeyCode::Char('d')), dd_bindings),
                ChordResult::Exact(Action::Edit(EditOp::Delete(Some('a'))), 3)
            ));
        }

        #[rstest]
        fn register_clears_after_no_match(mut engine: SequenceEngine, dd_bindings: &ModeMap) {
            engine.advance(press(KeyCode::Char('"')), dd_bindings);
            engine.advance(press(KeyCode::Char('a')), dd_bindings);
            engine.advance(press(KeyCode::Char('x')), dd_bindings); // no binding → NoMatch

            // Next dd must have no register.
            engine.advance(press(KeyCode::Char('d')), dd_bindings);
            assert!(matches!(
                engine.advance(press(KeyCode::Char('d')), dd_bindings),
                ChordResult::Exact(Action::Edit(EditOp::Delete(None)), 1)
            ));
        }

        #[rstest]
        fn non_edit_action_discards_register(mut engine: SequenceEngine, j_bindings: &ModeMap) {
            engine.advance(press(KeyCode::Char('"')), j_bindings);
            engine.advance(press(KeyCode::Char('a')), j_bindings);

            // 'j' fires Move(Down) — register 'a' is silently discarded.
            assert!(matches!(
                engine.advance(press(KeyCode::Char('j')), j_bindings),
                ChordResult::Exact(Action::Move(Movement::Down), 1)
            ));
        }
    }
}
