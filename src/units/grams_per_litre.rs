use crate::unit_newtype;

/// Gas density in grams per litre (g/L).
///
/// ```no_run
/// use dps::units::GramsPerLitre;
///
/// let d = GramsPerLitre::new(1.188);
/// assert_eq!(d, GramsPerLitre::new(1.188));
/// assert_eq!(d.to_string(), "1.2 g/L");
///
/// assert_eq!(d + GramsPerLitre::new(0.5), GramsPerLitre::new(1.688));
/// assert_eq!(d - GramsPerLitre::new(0.5), GramsPerLitre::new(0.688));
/// assert_eq!(d * 2.0, GramsPerLitre::new(2.376));
/// assert_eq!(d / 2.0, GramsPerLitre::new(0.594));
///
/// // Ratio between two densities is dimensionless.
/// let ratio: f64 = GramsPerLitre::new(2.376) / GramsPerLitre::new(1.188);
/// assert_eq!(ratio, 2.0);
///
/// let e: GramsPerLitre = 1.188_f64.into();
/// assert_eq!(f64::from(e), 1.188);
///
/// assert_eq!(-d, GramsPerLitre::new(-1.188));
/// assert_eq!(2.0_f64 * d, GramsPerLitre::new(2.376));
/// assert_eq!(d.max(GramsPerLitre::new(2.0)), GramsPerLitre::new(2.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct GramsPerLitre(f64);

unit_newtype!(GramsPerLitre, "g/L");
