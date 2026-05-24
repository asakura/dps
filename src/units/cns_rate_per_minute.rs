use crate::unit_newtype;

/// CNS O₂ toxicity rate in percent of the single-dive CNS exposure limit per minute.
///
/// Computed from the NOAA single-dive CNS table. Multiply by exposure time in
/// minutes to get the percentage of the CNS limit consumed. A value of
/// [`f64::INFINITY`] indicates a ppO₂ above 1.6 bar (not recommended).
///
/// ```no_run
/// use dps::units::CnsRatePerMinute;
///
/// // At 1.4 bar limit (150 min): rate = 100/150 ≈ 0.667 CNS%/min
/// let rate = CnsRatePerMinute::new(100.0 / 150.0);
/// assert_eq!(rate.to_string(), "0.7 CNS%/min");
///
/// assert_eq!(rate + CnsRatePerMinute::new(100.0 / 150.0), CnsRatePerMinute::new(200.0 / 150.0));
/// assert_eq!(rate * 150.0, CnsRatePerMinute::new(100.0));
/// assert_eq!(rate / 2.0, CnsRatePerMinute::new(100.0 / 300.0));
///
/// // Ratio between two rates is dimensionless.
/// let ratio: f64 = CnsRatePerMinute::new(2.0) / CnsRatePerMinute::new(1.0);
/// assert_eq!(ratio, 2.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct CnsRatePerMinute(f64);

unit_newtype!(CnsRatePerMinute, "CNS%/min");
