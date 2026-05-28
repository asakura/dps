//! [`Display`](std::fmt::Display) implementation for [`DiveEnvironment`](super::DiveEnvironment).
//!
//! ```
//! use dps::environment::DiveEnvironment;
//! assert_eq!(DiveEnvironment::standard().to_string(), "standard");
//! ```

use std::fmt;

use super::DiveEnvironment;

/// Serialises a [`DiveEnvironment`] as a human-readable string.
///
/// Named presets serialise to their short names; custom environments use a
/// `"surface_pressure=P,water_density=D"` key-value format with raw `f64` values.
///
/// ```
/// use dps::environment::DiveEnvironment;
///
/// assert_eq!(DiveEnvironment::standard().to_string(),   "standard");
/// assert_eq!(DiveEnvironment::freshwater().to_string(), "freshwater");
/// ```
impl fmt::Display for DiveEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::standard() {
            f.write_str("standard")
        } else if *self == Self::freshwater() {
            f.write_str("freshwater")
        } else {
            write!(
                f,
                "surface_pressure={},water_density={}",
                f64::from(self.surface_pressure),
                f64::from(self.water_density),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::{DiveEnvironmentError, Ocean};
    use crate::units::{Bar, MetersPerBar};
    use rstest::rstest;

    #[rstest]
    fn standard_serialises_to_keyword() {
        assert_eq!(DiveEnvironment::standard().to_string(), "standard");
    }

    #[rstest]
    fn freshwater_serialises_to_keyword() {
        assert_eq!(DiveEnvironment::freshwater().to_string(), "freshwater");
    }

    #[rstest]
    fn custom_env_serialises_to_key_value() -> Result<(), DiveEnvironmentError> {
        let env = DiveEnvironment::new(Bar::new(0.95), MetersPerBar::new(10.1))?;
        assert_eq!(env.to_string(), "surface_pressure=0.95,water_density=10.1");
        Ok(())
    }

    #[rstest]
    fn ocean_preset_falls_through_to_key_value() {
        let s = DiveEnvironment::ocean(Ocean::RedSea).to_string();
        assert!(s.starts_with("surface_pressure="));
        assert!(s.contains(",water_density="));
    }
}
