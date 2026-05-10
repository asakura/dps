//! Actions produced by components and consumed by the event loop.

/// Outcome returned by [`crate::components::Component::handle_key`] and [`crate::app::App::handle_key`].
pub enum Action {
    /// Exit the application.
    Quit,
    /// Key was handled internally; no further action required.
    None,
}
