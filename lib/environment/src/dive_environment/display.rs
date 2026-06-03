//! [`Display`](std::fmt::Display) implementation for [`DiveEnvironment`](super::DiveEnvironment).
//!
//! ```
//! use dps_environment::DiveEnvironment;
//! assert_eq!(DiveEnvironment::standard().to_string(), "standard");
//! ```

use super::DiveEnvironment;
use crate::{Lake, Ocean};

use strum::IntoEnumIterator;

use std::fmt;

/// Serialises a [`DiveEnvironment`] as a human-readable string.
///
/// Named presets serialise to their short names; custom environments use a
/// `"surface_pressure=P,water_density=D"` key-value format with raw `f64` values.
///
/// ```
/// use dps_environment::{DiveEnvironment, Ocean};
///
/// assert_eq!(DiveEnvironment::standard().to_string(),   "standard");
/// assert_eq!(DiveEnvironment::freshwater().to_string(), "freshwater");
/// assert_eq!(DiveEnvironment::ocean(Ocean::RedSea).to_string(), "ocean:RedSea");
/// ```
impl fmt::Display for DiveEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::standard() {
            return f.write_str("standard");
        }

        if *self == Self::freshwater() {
            return f.write_str("freshwater");
        }

        for ocean in Ocean::iter() {
            if *self == Self::ocean(ocean) {
                return write!(f, "ocean:{ocean}");
            }
        }

        for lake in Lake::iter() {
            if *self == Self::lake(lake) {
                return write!(f, "lake:{lake}");
            }
        }

        write!(
            f,
            "surface_pressure={},water_density={}",
            f64::from(self.surface_pressure),
            f64::from(self.water_density),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::DiveEnvironmentError;
    use dps_units::{Bar, MetersPerBar};

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
    fn ocean_preset_serialises_to_named_format() {
        assert_eq!(
            DiveEnvironment::ocean(Ocean::BalticSea).to_string(),
            "ocean:BalticSea"
        );
    }

    #[rstest]
    fn lake_preset_serialises_to_named_format() {
        assert_eq!(
            DiveEnvironment::lake(Lake::Titicaca).to_string(),
            "lake:Titicaca"
        );
    }
}
