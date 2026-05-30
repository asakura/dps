//! Key-handling primitives: modes, sequences, mode maps, and chord accumulation.
//!
//! # Architecture
//!
//! The module is split into six focused sub-modules. Data flows through
//! them in a single direction during each iteration of the application event
//! loop:
//!
//! ```text
//!  Config (TOML/JSON)
//!      │  KeyBindingsBuilder::deserialize
//!      ▼
//!  KeyBindings          ← mode-indexed registry (immutable)
//!      │  .get(&mode)
//!      ▼
//!  ModeMap              ← sequence-to-action map for one mode (immutable)
//!      │
//!      ▼
//!  SequenceEngine::advance(key, &ModeMap)
//!      │
//!      ├─ ChordResult::Prefix   → buffer growing, wait for next key
//!      ├─ ChordResult::Exact    → dispatch Action to the component
//!      └─ ChordResult::NoMatch  → pass key to the global fallback handler
//! ```
//!
//! # Quick start
//!
//! Build a binding table at startup, then feed key events one at a time:
//!
//! ```no_run
//! use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
//! use dps::action::{Action, Movement};
//! use dps::keymap::{
//!     ChordEngine, ChordResult, KeyBindingsBuilder, KeySeq, Mode, SequenceEngine,
//! };
//!
//! fn press(code: KeyCode) -> KeyEvent {
//!     KeyEvent::new(code, KeyModifiers::NONE)
//! }
//!
//! // build once at startup
//! let mut builder = KeyBindingsBuilder::new();
//! builder.bind(
//!     Mode::Normal,
//!     KeySeq::from(vec![press(KeyCode::Char('j'))]),
//!     Action::Move(Movement::Down),
//! );
//! builder.bind(
//!     Mode::Normal,
//!     KeySeq::from(vec![press(KeyCode::Char('g')), press(KeyCode::Char('g'))]),
//!     Action::Move(Movement::GotoTop),
//! );
//! let bindings = builder.build();
//!
//! // inside the event loop
//! let mode = Mode::Normal;
//! let mut engine = SequenceEngine::default();
//!
//! // first 'g' → prefix, second 'g' → exact match
//! if let Some(map) = bindings.get(&mode) {
//!     let _ = engine.advance(press(KeyCode::Char('g')), map); // Prefix
//!
//!     match engine.advance(press(KeyCode::Char('g')), map) {
//!         ChordResult::Exact(action, _) => { /* dispatch action */ let _ = action; }
//!         ChordResult::Prefix           => { /* keep waiting */ }
//!         ChordResult::NoMatch          => { /* global fallback */ }
//!     }
//! }
//! ```
//!
//! # Module overview
//!
//! - `mode` — [`crate::keymap::mode::Mode`]: enumerates application modes that scope keybindings.
//! - `seq` — [`crate::keymap::seq::KeySeq`]: newtype wrapping a boxed `[KeyEvent]` slice; hash key for maps.
//! - `map` — [`crate::keymap::map::ModeMap`], [`crate::keymap::map::ModeMapBuilder`]: immutable sequence→action map for one mode and its builder.
//! - `bindings` — [`crate::keymap::bindings::KeyBindings`], [`crate::keymap::bindings::KeyBindingsBuilder`]: mode-indexed registry; supports TOML/JSON deserialization.
//! - `chord` — [`crate::keymap::chord::SequenceEngine`], [`crate::keymap::chord::ChordResult`]: stateful multi-key chord accumulator.
//! - `keys` — [`crate::keymap::keys::parse_key_sequence`], [`crate::keymap::keys::key_event_to_string`]: Vim-notation ↔ [`crossterm::event::KeyEvent`] conversion.
//! - `error` — [`crate::keymap::KeyMapError`]: errors produced by key-spec parsing.

pub mod bindings;
pub mod chord;
mod error;
pub mod keys;
pub mod map;
pub mod mode;
pub mod seq;

pub use self::bindings::{KeyBindings, KeyBindingsBuilder};
pub use self::chord::{ChordEngine, ChordResult, SequenceEngine};
pub use self::error::Error as KeyMapError;
pub use self::keys::{key_event_to_string, parse_key_sequence};
pub use self::map::{ModeMap, ModeMapBuilder};
pub use self::mode::Mode;
pub use self::seq::KeySeq;

#[cfg(test)]
pub(crate) mod testutil {
    use super::seq::KeySeq;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    pub fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    pub fn single(code: KeyCode) -> KeySeq {
        KeySeq::from(vec![press(code)])
    }
}
