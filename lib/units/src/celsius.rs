//! Temperature in degrees Celsius.

use crate::unit_newtype;

/// Temperature in degrees Celsius (°C).
///
/// ```
/// use dps_units::Celsius;
///
/// let t = Celsius::new(25.0);
/// assert_eq!(t, Celsius::new(25.0));
/// assert_eq!(t.to_string(), "25.0 °C");
///
/// assert_eq!(t + Celsius::new(5.0), Celsius::new(30.0));
/// assert_eq!(t - Celsius::new(5.0), Celsius::new(20.0));
/// assert_eq!(t * 2.0, Celsius::new(50.0));
/// assert_eq!(t / 2.0, Celsius::new(12.5));
///
/// let b: Celsius = 20.0_f64.into();
/// assert_eq!(f64::from(b), 20.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
pub struct Celsius(f64);

unit_newtype!(Celsius, "°C");
