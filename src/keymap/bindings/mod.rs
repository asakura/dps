//! Mode-indexed keybinding registry and its builder.
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

mod builder;
mod registry;

pub use self::builder::KeyBindingsBuilder;
pub use self::registry::KeyBindings;
