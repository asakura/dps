//! Salinity (and other concentrations) in parts per thousand.

use crate::unit_newtype;

/// Concentration in parts per thousand (‰).
///
/// Used for water salinity throughout the `environment` module.
///
/// ```
/// use dps_units::PartsPerThousand;
///
/// let s = PartsPerThousand::new(35.0);
/// assert_eq!(s, PartsPerThousand::new(35.0));
/// assert_eq!(s.to_string(), "35.0 ‰");
///
/// assert_eq!(s + PartsPerThousand::new(5.0), PartsPerThousand::new(40.0));
/// assert_eq!(s - PartsPerThousand::new(5.0), PartsPerThousand::new(30.0));
/// assert_eq!(s * 2.0, PartsPerThousand::new(70.0));
/// assert_eq!(s / 2.0, PartsPerThousand::new(17.5));
///
/// let b: PartsPerThousand = 10.0_f64.into();
/// assert_eq!(f64::from(b), 10.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
pub struct PartsPerThousand(f64);

unit_newtype!(PartsPerThousand, "‰");
