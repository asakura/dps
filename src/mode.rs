//! Application interaction modes used to scope keybindings.

use serde::{Deserialize, Serialize};

/// Defines which keybinding context is active.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    /// The default mode: top-level navigation between tabs.
    #[default]
    Home,
}
