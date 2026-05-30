//! Error type for the [`app`](super) module.
//!
//! # Examples
//!
//! ```
//! use dps::app::AppError;
//! use dps::components::ComponentError;
//!
//! let e = AppError::from(ComponentError::InvalidState("boom"));
//! assert!(matches!(e, AppError::Component(_)));
//! ```

use crate::{action::Action, components::ComponentError};

use tokio::sync::mpsc::error::SendError;

/// Errors produced by the [`App`](super::App) event loop.
///
/// # Examples
///
/// ```
/// use dps::app::AppError;
/// use dps::components::ComponentError;
///
/// let e = AppError::from(ComponentError::InvalidState("boom"));
/// assert!(matches!(e, AppError::Component(_)));
/// ```
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// A component method returned an error.
    #[error(transparent)]
    Component(#[from] ComponentError),
    /// Sending an [`Action`] to the event channel failed.
    #[error(transparent)]
    ActionSend(#[from] SendError<Action>),
    /// A terminal I/O operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
