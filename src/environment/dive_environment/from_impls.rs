//! [`From`] conversions from [`Ocean`] and [`Lake`] into
//! [`DiveEnvironment`](super::DiveEnvironment).
//!
//! ```
//! use dps::environment::{DiveEnvironment, Ocean};
//! assert_eq!(DiveEnvironment::from(Ocean::Caribbean), DiveEnvironment::ocean(Ocean::Caribbean));
//! ```

use crate::environment::{Lake, Ocean};

use super::DiveEnvironment;

/// Converts a named [`Ocean`] into a sea-level [`DiveEnvironment`].
///
/// Equivalent to [`DiveEnvironment::ocean`].
///
/// # Examples
///
/// ```
/// use dps::environment::{DiveEnvironment, Ocean};
///
/// assert_eq!(
///     DiveEnvironment::from(Ocean::RedSea),
///     DiveEnvironment::ocean(Ocean::RedSea),
/// );
/// ```
impl From<Ocean> for DiveEnvironment {
    fn from(ocean: Ocean) -> Self {
        Self::ocean(ocean)
    }
}

/// Converts a named [`Lake`] into a [`DiveEnvironment`].
///
/// Equivalent to [`DiveEnvironment::lake`].
///
/// # Examples
///
/// ```
/// use dps::environment::{DiveEnvironment, Lake};
///
/// assert_eq!(
///     DiveEnvironment::from(Lake::Titicaca),
///     DiveEnvironment::lake(Lake::Titicaca),
/// );
/// ```
impl From<Lake> for DiveEnvironment {
    fn from(lake: Lake) -> Self {
        Self::lake(lake)
    }
}
