use crate::unit_newtype;

/// Depth-to-pressure conversion factor for seawater (metres per bar).
///
/// ```no_run
/// use dps_units::MetersPerBar;
/// let seawater = MetersPerBar::new(10.0);
/// assert_eq!(seawater, MetersPerBar::new(10.0));
/// assert_eq!(seawater.to_string(), "10.0 m/bar");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MetersPerBar(f64);

unit_newtype!(MetersPerBar, "m/bar");
