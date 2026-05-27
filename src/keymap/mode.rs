//! Application interaction modes used to scope keybindings.
//!
//! A [`Mode`] names the keybinding context that is active at a given moment.
//! The application event loop selects the current mode, looks up the matching
//! [`ModeMap`](super::ModeMap) in the [`KeyBindings`](super::KeyBindings)
//! registry, and passes it to the chord engine.  Each mode therefore has its
//! own fully independent binding table — the same key sequence can map to
//! different actions in different modes.
//!
//! # Extending modes
//!
//! Add a new variant here, then populate its bindings via
//! [`KeyBindingsBuilder::bind`](super::KeyBindingsBuilder::bind).  The
//! application component responsible for that UI section drives mode
//! transitions by updating the active-mode field in shared app state.

use serde::{Deserialize, Serialize};

/// The active keybinding context.
///
/// Each variant represents a distinct UI state where the user interacts with
/// the application.  Keybindings are registered per-mode so the same key can
/// trigger different actions depending on context.
///
/// `Mode` is `Copy`, `Ord`, and serializable so it can be used as a
/// [`HashMap`](std::collections::HashMap) key and deserialized from
/// configuration files.
#[derive(
    Default, Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum Mode {
    /// The default mode: top-level navigation between tabs.
    ///
    /// Active when no overlay or detail panel is open.  All navigation
    /// commands (move, scroll, quit) are registered here by default.
    #[default]
    Normal,

    /// Confirmation prompt mode.
    ///
    /// Active when the application is waiting for the user to confirm or
    /// cancel a destructive action.
    Confirm,
}
