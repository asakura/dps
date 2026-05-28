//! Vim-style register store for yank, paste, and delete operations.
//!
//! The central type is [`RegisterStore`], which holds a named register map
//! keyed by a single `char`. Special registers follow Vim semantics:
//!
//! | Register | Behaviour |
//! |---|---|
//! | `'"'` | Unnamed — always receives the most recent yank or delete |
//! | `'0'` | Yank register — most recent yank only |
//! | `'1'`–`'9'` | Delete history stack (oldest entry falls off `'9'`) |
//! | `'a'`–`'z'` | Named registers (persist until overwritten) |
//! | `'A'`–`'Z'` | Append to lowercase partner (last-write wins for typed values) |
//! | `'_'` | Black hole — writes discarded, reads always `None` |
//! | `'+'` / `'*'` | OS clipboard registers via [`arboard`] |
//!
//! # Examples
//!
//! ```
//! use dps::registers::{RegisterStore, RegisterValue};
//! use dps::gas::EANx;
//! use dps::units::Percent;
//!
//! let mut store = RegisterStore::default();
//! let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
//!
//! // Yank into the unnamed register; '0' also receives a copy.
//! store.push_yank(None, RegisterValue::EANx(ean32));
//! assert_eq!(store.read('"'), Some(RegisterValue::EANx(ean32)));
//! assert_eq!(store.read('0'), Some(RegisterValue::EANx(ean32)));
//!
//! // Paste from '0' into a named register.
//! if let Some(val) = store.read('0') {
//!     store.write('a', val);
//! }
//! assert_eq!(store.read('a'), Some(RegisterValue::EANx(ean32)));
//! ```

mod clipboard;
mod store;
mod value;

pub use store::RegisterStore;
pub use value::RegisterValue;
