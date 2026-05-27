//! Vim-style key sequence parsing and serialization.
//!
//! Implementation lives in [`crate::keymap::keys`]; this module re-exports
//! the public surface.
pub use crate::keymap::keys::{key_event_to_string, parse_key_sequence};
