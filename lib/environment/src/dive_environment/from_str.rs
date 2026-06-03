//! [`FromStr`](std::str::FromStr) implementation and [`ParseDiveEnvironmentError`] for
//! [`DiveEnvironment`](super::DiveEnvironment).
//!
//! ```
//! use dps_environment::DiveEnvironment;
//! assert_eq!("standard".parse::<DiveEnvironment>().unwrap(), DiveEnvironment::standard());
//! ```

use super::{DiveEnvironment, DiveEnvironmentError};
use crate::{Lake, Ocean};

use dps_units::{Bar, MetersPerBar};

use std::str::FromStr;

/// Error returned when a string cannot be parsed as a [`DiveEnvironment`].
///
/// Produced by [`DiveEnvironment::from_str`] when the input does not match any
/// format produced by [`Display`](std::fmt::Display).
///
/// ```
/// use dps_environment::DiveEnvironment;
///
/// assert!("invalid".parse::<DiveEnvironment>().is_err());
/// assert!("standard".parse::<DiveEnvironment>().is_ok());
/// ```
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("invalid dive environment string")]
pub struct ParseDiveEnvironmentError;

/// Parses a [`DiveEnvironment`] from its display representation.
///
/// Accepts `"standard"`, `"freshwater"`, `"ocean:Name"`, `"lake:Name"`, and the
/// `"surface_pressure=P,water_density=D"` format produced by
/// [`Display`](std::fmt::Display) for custom environments.
///
/// # Errors
///
/// Returns [`DiveEnvironmentError::Parse`] if the string does not match any
/// known format, or another [`DiveEnvironmentError`] variant if the raw
/// numeric values are out of range.
///
/// # Examples
///
/// ```
/// use dps_environment::{DiveEnvironment, Ocean};
///
/// assert_eq!("standard".parse::<DiveEnvironment>().unwrap(),   DiveEnvironment::standard());
/// assert_eq!("ocean:RedSea".parse::<DiveEnvironment>().unwrap(), DiveEnvironment::ocean(Ocean::RedSea));
/// assert!("invalid".parse::<DiveEnvironment>().is_err());
/// ```
impl FromStr for DiveEnvironment {
    type Err = DiveEnvironmentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "standard" => Ok(Self::standard()),
            "freshwater" => Ok(Self::freshwater()),
            _ if s.starts_with("ocean:") => {
                let name = s.strip_prefix("ocean:").ok_or(ParseDiveEnvironmentError)?;
                let ocean = Ocean::from_str(name).map_err(|_| ParseDiveEnvironmentError)?;

                Ok(Self::ocean(ocean))
            }
            _ if s.starts_with("lake:") => {
                let name = s.strip_prefix("lake:").ok_or(ParseDiveEnvironmentError)?;
                let lake = Lake::from_str(name).map_err(|_| ParseDiveEnvironmentError)?;

                Ok(Self::lake(lake))
            }
            _ => {
                let (sp_part, wd_part) = s
                    .split_once(",water_density=")
                    .ok_or(ParseDiveEnvironmentError)?;
                let p: f64 = sp_part
                    .strip_prefix("surface_pressure=")
                    .ok_or(ParseDiveEnvironmentError)?
                    .parse()
                    .map_err(|_| ParseDiveEnvironmentError)?;
                let d: f64 = wd_part.parse().map_err(|_| ParseDiveEnvironmentError)?;

                Self::new(Bar::new(p), MetersPerBar::new(d))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    use core::assert_matches;

    #[rstest]
    fn parses_standard() -> Result<(), DiveEnvironmentError> {
        let env = "standard".parse::<DiveEnvironment>()?;
        assert_eq!(env, DiveEnvironment::standard());
        Ok(())
    }

    #[rstest]
    fn parses_freshwater() -> Result<(), DiveEnvironmentError> {
        let env = "freshwater".parse::<DiveEnvironment>()?;
        assert_eq!(env, DiveEnvironment::freshwater());
        Ok(())
    }

    #[rstest]
    fn parses_ocean_format() -> Result<(), DiveEnvironmentError> {
        let env = "ocean:RedSea".parse::<DiveEnvironment>()?;
        assert_eq!(env, DiveEnvironment::ocean(Ocean::RedSea));
        Ok(())
    }

    #[rstest]
    fn parses_lake_format() -> Result<(), DiveEnvironmentError> {
        let env = "lake:Titicaca".parse::<DiveEnvironment>()?;
        assert_eq!(env, DiveEnvironment::lake(Lake::Titicaca));
        Ok(())
    }

    #[rstest]
    fn parses_key_value_format() -> Result<(), DiveEnvironmentError> {
        let env = "surface_pressure=0.95,water_density=10.1".parse::<DiveEnvironment>()?;
        assert_eq!(env.surface_pressure(), Bar::new(0.95));
        assert_eq!(env.water_density(), MetersPerBar::new(10.1));
        Ok(())
    }

    #[rstest]
    fn invalid_returns_error() {
        assert_matches!(
            "invalid".parse::<DiveEnvironment>(),
            Err(DiveEnvironmentError::Parse(_))
        );
    }

    #[rstest]
    fn custom_roundtrips() -> Result<(), DiveEnvironmentError> {
        let s = "surface_pressure=0.95,water_density=10.1";
        let env: DiveEnvironment = s.parse()?;
        assert_eq!(env.to_string().parse::<DiveEnvironment>()?, env);
        Ok(())
    }

    #[rstest]
    fn ocean_roundtrips() -> Result<(), DiveEnvironmentError> {
        let env = DiveEnvironment::ocean(Ocean::RedSea);
        assert_eq!(env.to_string().parse::<DiveEnvironment>()?, env);
        Ok(())
    }

    #[rstest]
    fn lake_roundtrips() -> Result<(), DiveEnvironmentError> {
        let env = DiveEnvironment::lake(Lake::Titicaca);
        assert_eq!(env.to_string().parse::<DiveEnvironment>()?, env);
        Ok(())
    }
}
