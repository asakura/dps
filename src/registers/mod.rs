//! Vim-style register store for yank, paste, and delete operations.
//!
//! The central type is [`RegisterStore`], which holds a named register map
//! keyed by [`RegisterName`]. Special registers follow Vim semantics:
//!
//! | Register | Behaviour |
//! |---|---|
//! | `'"'` | Unnamed — always receives the most recent yank or delete |
//! | `'0'` | Yank register — head of the internal yank ring (most recent yank) |
//! | `'1'`–`'9'` (`Numbered`) | Delete history stack (oldest entry falls off `'9'`) |
//! | `'a'`–`'z'` | Named registers (persist until overwritten) |
//! | `'A'`–`'Z'` | Append to lowercase partner (last-write wins for typed values) |
//! | `'_'` | Black hole — writes discarded, reads always `None` |
//! | `'+'` / `'*'` | OS clipboard registers via [`arboard`] |
//!
//! ## Yank ring
//!
//! [`RegisterStore`] maintains an internal [`VecDeque`](std::collections::VecDeque)
//! yank ring (capacity 9). Every [`push_yank`](RegisterStore::push_yank) call
//! prepends to the ring and resets the ring cursor to 0; `'0'` always reads
//! from the head. [`cycle_yank`](RegisterStore::cycle_yank) advances the
//! cursor so components can walk older entries in response to
//! [`EditOp::CyclePaste`](crate::action::EditOp).
//!
//! ## Future work: multi-value yank sequences (Design C)
//!
//! A possible future extension would add batch-yank support so that a count
//! prefix like `3yy` collects N consecutive rows into a single register value
//! rather than repeating a single-row yank N times (which is a no-op beyond
//! the first because the cursor does not advance).
//!
//! The design requires three coordinated changes:
//!
//! 1. **`RegisterValue::Sequence(Box<[RegisterValue]>)`** — a new variant
//!    holding an ordered slice of values. This drops `RegisterValue: Copy`
//!    (since `Box<[_]>` is not `Copy`), requiring all `self.slots.insert` /
//!    `.copied()` call sites in [`RegisterStore`] to switch to `clone()`.
//!
//! 2. **`EditOp::YankRows { reg: Option<RegisterName>, count: usize }`** — a separate
//!    action variant that carries the count directly. The current dispatch
//!    architecture implements count by repeating an action N times, which only
//!    works when each repetition has a distinct side-effect (e.g. `Delete`
//!    removes the focused row so the cursor naturally advances). `YankRow` has
//!    no cursor side-effect, so dispatch repetition is useless; the count must
//!    reach the component as data. `YankRows` opts out of `accepts_count` and
//!    is emitted by the [`SequenceEngine`](crate::keymap::SequenceEngine) when
//!    count > 1 for a yank binding.
//!
//! 3. **Paste semantics for `Sequence`** — a component receiving
//!    `Edit(Paste(_))` where the register holds a `Sequence` would insert each
//!    element as a new row. Paste semantics are currently undefined
//!    (`EditOp::Paste` has a TODO); `Sequence` paste should be designed
//!    alongside single-value paste to keep the two paths consistent.
//!
//! # Examples
//!
//! ```
//! use dps::registers::{RegisterName, RegisterStore, RegisterValue};
//! use dps::gas::EANx;
//! use dps::units::Percent;
//!
//! let mut store = RegisterStore::default();
//! let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
//! let reg_0 = RegisterName::Yank;
//! let reg_a = RegisterName::try_from('a').unwrap();
//!
//! // Yank into the unnamed register; '0' also receives a copy.
//! store.push_yank(RegisterName::Unnamed, RegisterValue::EANx(ean32));
//! assert_eq!(store.read(RegisterName::Unnamed), Some(RegisterValue::EANx(ean32)));
//! assert_eq!(store.read(reg_0), Some(RegisterValue::EANx(ean32)));
//!
//! // Paste from '0' into a named register.
//! if let Some(val) = store.read(reg_0) {
//!     store.write(reg_a, val);
//! }
//! assert_eq!(store.read(reg_a), Some(RegisterValue::EANx(ean32)));
//! ```

mod clipboard;
mod error;
mod name;
mod store;
mod value;

pub use self::error::{
    InvalidRegisterIndex, InvalidRegisterLetter, RegisterError, YankRingTooSmall,
};
pub use self::name::RegisterName;
#[doc(hidden)]
pub use self::name::{RegIndex, RegLetter};
pub use self::store::RegisterStore;
pub use self::value::RegisterValue;
