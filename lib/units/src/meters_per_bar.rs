use crate::unit_newtype;

/// Depth-to-pressure conversion factor for water ($\pu{m/bar}$).
///
/// ISO standard seawater ($\pu{35 ‰}$, $\pu{15 ^\circ C}$) gives $\pu{9.950 m/bar}$;
/// fresh water ($\pu{0 ‰}$) gives $\approx \pu{10.197 m/bar}$.
///
/// ```
/// use dps_units::MetersPerBar;
/// let seawater = MetersPerBar::new(10.0);
/// assert_eq!(seawater, MetersPerBar::new(10.0));
/// assert_eq!(seawater.to_string(), "10.0 m/bar");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
pub struct MetersPerBar(f64);

unit_newtype!(MetersPerBar, "m/bar");
