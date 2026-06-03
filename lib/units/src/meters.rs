use crate::unit_newtype;

/// Depth in metres. Backed by f64 so mul/div never truncate.
///
/// ```no_run
/// use dps_units::Meters;
///
/// let a = Meters::new(30.0);
/// assert_eq!(a, Meters::new(30.0));
/// assert_eq!(a.to_string(), "30.0 m");
///
/// assert_eq!(a + Meters::new(10.0), Meters::new(40.0));
/// assert_eq!(a - Meters::new(10.0), Meters::new(20.0));
/// assert_eq!(-a, Meters::new(-30.0));
/// assert_eq!(a * 2.0, Meters::new(60.0));
/// assert_eq!(a / 2.0, Meters::new(15.0));
/// assert_eq!(a.max(Meters::new(50.0)), Meters::new(50.0));
///
/// let b: Meters = 30.0_f64.into();
/// assert_eq!(f64::from(b), 30.0);
///
/// // f64 × Meters (scalar-on-right is symmetric)
/// assert_eq!(2.0_f64 * a, Meters::new(60.0));
/// // Meters ÷ Meters → dimensionless ratio
/// let ratio: f64 = a / Meters::new(10.0);
/// assert_eq!(ratio, 3.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Meters(f64);

unit_newtype!(Meters, "m");
