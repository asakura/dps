use dps_units::Percent;

/// Error returned when a string cannot be parsed as an [`EANx`](crate::EANx) blend.
///
/// Produced by [`EANx::from_str`](std::str::FromStr) when the input does not match any known
/// gas-name format or the resulting $\ce{O2}$ fraction is outside the valid range.
///
/// ```
/// use dps_gas::EANx;
///
/// assert!("invalid".parse::<EANx>().is_err());
/// assert!("EANx 999".parse::<EANx>().is_err());
/// assert!("EANx 32".parse::<EANx>().is_ok());
/// ```
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("invalid EANx blend name")]
pub struct ParseEANxError;

/// Error returned when an [`EANxBlend`](crate::EANxBlend) cannot be constructed.
///
/// ```no_run
/// use dps_gas::{EANxBlend, InvalidEANxError, PartialPressure, Psa};
/// use dps_units::Percent;
///
/// // FO₂ below 10 % minimum
/// let too_low = Percent::new(0.09).unwrap();
/// assert!(matches!(
///     EANxBlend::new(too_low, PartialPressure),
///     Err(InvalidEANxError::O2TooLow(_))
/// ));
///
/// // PSA ceiling exceeded (fo2 ≈ 95.7 % max)
/// let too_high = Percent::new(0.99).unwrap();
/// assert!(matches!(
///     EANxBlend::new(too_high, Psa),
///     Err(InvalidEANxError::BlendCeilingExceeded(_))
/// ));
/// ```
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum InvalidEANxError {
    /// $\text{F}\ce{O2}$ is below the 10 % minimum for a breathable EAN mix.
    #[error("O₂ fraction {0} is below the 10 % minimum")]
    O2TooLow(Percent),
    /// $\text{F}\ce{O2}$ exceeds the physical ceiling for this blend method.
    ///
    /// For [`Psa`](crate::Psa) the ceiling is ≈ 95.7 %: the point at which all $\ce{N2}$
    /// would be depleted and the output is pure $\ce{O2}$ + Ar.
    #[error("O₂ fraction {0} exceeds the blend method ceiling")]
    BlendCeilingExceeded(Percent),
    /// The input string is not a recognised [`EANx`](crate::EANx) blend name.
    #[error(transparent)]
    ParseFailed(#[from] ParseEANxError),
    /// A unit value was outside the valid range during blend construction.
    #[error(transparent)]
    Unit(#[from] dps_units::UnitError),
}

#[cfg(test)]
mod tests {
    use super::*;

    use dps_units::{Percent, UnitError};

    use rstest::rstest;

    mod display {
        use super::*;

        #[rstest]
        fn o2_too_low_contains_fraction() -> Result<(), UnitError> {
            let p = Percent::new(0.05)?;

            assert!(
                InvalidEANxError::O2TooLow(p)
                    .to_string()
                    .contains(&p.to_string())
            );

            Ok(())
        }

        #[rstest]
        fn blend_ceiling_exceeded_contains_ceiling() -> Result<(), UnitError> {
            let p = Percent::new(0.99)?;

            assert!(
                InvalidEANxError::BlendCeilingExceeded(p)
                    .to_string()
                    .contains("ceiling")
            );

            Ok(())
        }
    }
}
