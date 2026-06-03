//! [`Default`] implementation for [`DiveEnvironment`](super::DiveEnvironment).
//!
//! ```
//! use dps_environment::DiveEnvironment;
//! assert_eq!(DiveEnvironment::default(), DiveEnvironment::standard());
//! ```

use super::DiveEnvironment;

/// Returns [`DiveEnvironment::standard`].
///
/// # Examples
///
/// ```
/// use dps_environment::DiveEnvironment;
///
/// assert_eq!(DiveEnvironment::default(), DiveEnvironment::standard());
/// ```
impl Default for DiveEnvironment {
    fn default() -> Self {
        Self::standard()
    }
}
