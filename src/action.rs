//! Actions produced by components and consumed by the event loop.

use serde::{Deserialize, Serialize};
use strum::Display;

/// Outcome returned by [`crate::components::Component::handle_key`] and [`crate::app::App::handle_key`].
///
/// `Display` (via `strum`) is derived for future use in the WhichKey widget and status bar.
/// TODO: wire `Display` output up once those surfaces exist.
#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    /// Exit the application.
    Quit,
    /// Key was handled internally; no further action required.
    None,
    Up,
    Down,
    Left,
    Right,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    GotoTop,
    GotoBottom,
    Select,
}
