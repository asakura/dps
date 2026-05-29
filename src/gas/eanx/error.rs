use crate::units::Percent;

/// Error returned when a string cannot be parsed as an [`EANx`](crate::gas::EANx) blend.
///
/// Produced by [`EANx::from_str`](std::str::FromStr) when the input does not match any known
/// gas-name format or the resulting O₂ fraction is outside the valid range.
///
/// ```
/// use dps::gas::EANx;
///
/// assert!("invalid".parse::<EANx>().is_err());
/// assert!("EANx 999".parse::<EANx>().is_err());
/// assert!("EANx 32".parse::<EANx>().is_ok());
/// ```
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("invalid EANx blend name")]
pub struct ParseEANxError;

/// Error returned when an [`EANxBlend`](crate::gas::EANxBlend) cannot be constructed.
///
/// ```no_run
/// use dps::gas::{EANxBlend, InvalidEANxError, PartialPressure, Psa};
/// use dps::units::Percent;
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
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[non_exhaustive]
pub enum InvalidEANxError {
    /// FO₂ is below the 10 % minimum for a breathable EAN mix.
    #[error("O₂ fraction {0} is below the 10 % minimum")]
    O2TooLow(Percent),
    /// FO₂ exceeds the physical ceiling for this blend method.
    ///
    /// For [`Psa`](crate::gas::Psa) the ceiling is ≈ 95.7 %: the point at which all N₂
    /// would be depleted and the output is pure O₂ + Ar.
    #[error("O₂ fraction {0} exceeds the blend method ceiling")]
    BlendCeilingExceeded(Percent),
    /// The input string is not a recognised [`EANx`](crate::gas::EANx) blend name.
    #[error(transparent)]
    ParseFailed(#[from] ParseEANxError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Percent;

    #[test]
    fn o2_too_low_display_contains_fraction() -> Result<(), Box<dyn std::error::Error>> {
        let p = Percent::new(0.05)?;
        let msg = InvalidEANxError::O2TooLow(p).to_string();

        assert!(msg.contains('5') || msg.contains("0.05"));

        Ok(())
    }

    #[test]
    fn blend_ceiling_exceeded_display() -> Result<(), Box<dyn std::error::Error>> {
        let p = Percent::new(0.99)?;
        let msg = InvalidEANxError::BlendCeilingExceeded(p).to_string();

        assert!(msg.contains("ceiling"));

        Ok(())
    }
}
