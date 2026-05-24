use crate::unit_newtype;

/// Oxygen Tolerance Unit accumulation rate in OTU/min.
///
/// Computed via the NOAA formula: `(ppO₂ − 0.5)^0.83` when `ppO₂ > 0.5 bar`,
/// else `0.0`. The daily limit is approximately 850 OTU.
///
/// ```no_run
/// use dps::units::OTUPerMinute;
///
/// let rate = OTUPerMinute::new(0.918);
/// assert_eq!(rate, OTUPerMinute::new(0.918));
/// assert_eq!(rate.to_string(), "0.9 OTU/min");
///
/// assert_eq!(rate + OTUPerMinute::new(0.082), OTUPerMinute::new(1.0));
/// assert_eq!(rate - OTUPerMinute::new(0.082), OTUPerMinute::new(0.836));
/// assert_eq!(rate * 60.0, OTUPerMinute::new(0.918 * 60.0));
/// assert_eq!(rate / 2.0, OTUPerMinute::new(0.459));
///
/// // Ratio between two rates is dimensionless.
/// let ratio: f64 = OTUPerMinute::new(2.0) / OTUPerMinute::new(1.0);
/// assert_eq!(ratio, 2.0);
///
/// assert_eq!(-rate, OTUPerMinute::new(-0.918));
/// assert_eq!(2.0_f64 * rate, OTUPerMinute::new(1.836));
/// assert_eq!(rate.max(OTUPerMinute::new(1.0)), OTUPerMinute::new(1.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct OTUPerMinute(f64);

unit_newtype!(OTUPerMinute, "OTU/min");
