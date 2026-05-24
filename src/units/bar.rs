use crate::unit_newtype;

/// Pressure in bar.
///
/// ```no_run
/// use dps::units::Bar;
///
/// let p = Bar::new(1.5);
/// assert_eq!(p.value(), 1.5);
/// assert_eq!(p.to_string(), "1.5 bar");
///
/// assert_eq!((p + Bar::new(0.5)).value(), 2.0);
/// assert_eq!((p - Bar::new(0.5)).value(), 1.0);
/// assert_eq!((p * 2.0).value(), 3.0);
/// assert_eq!((p / 2.0).value(), 0.75);
///
/// // Ratio between two Bar values is dimensionless.
/// let ratio: f64 = Bar::new(3.0) / Bar::new(1.5);
/// assert_eq!(ratio, 2.0);
///
/// assert_eq!((-p).value(), -1.5);
/// assert_eq!((2.0_f64 * p).value(), 3.0);
/// assert_eq!(p.max(Bar::new(2.0)).value(), 2.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Bar(f64);

unit_newtype!(Bar, "bar");
