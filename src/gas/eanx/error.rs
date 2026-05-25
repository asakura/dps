use crate::units::Percent;

/// Error returned when an [`EANxBlend`](crate::gas::EANxBlend) cannot be constructed.
///
/// ```no_run
/// use dps::gas::{EANxBlend, InvalidEANx, PartialPressure, Psa};
/// use dps::units::Percent;
///
/// // FO₂ below 10 % minimum
/// let too_low = Percent::new(0.09).unwrap();
/// assert!(matches!(
///     EANxBlend::new(too_low, PartialPressure),
///     Err(InvalidEANx::O2TooLow(_))
/// ));
///
/// // PSA ceiling exceeded (fo2 ≈ 95.7 % max)
/// let too_high = Percent::new(0.99).unwrap();
/// assert!(matches!(
///     EANxBlend::new(too_high, Psa),
///     Err(InvalidEANx::BlendCeilingExceeded(_))
/// ));
/// ```
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[non_exhaustive]
pub enum InvalidEANx {
    /// FO₂ is below the 10 % minimum for a breathable EAN mix.
    #[error("O₂ fraction {0} is below the 10 % minimum")]
    O2TooLow(Percent),
    /// FO₂ exceeds the physical ceiling for this blend method.
    ///
    /// For [`Psa`](crate::gas::Psa) the ceiling is ≈ 95.7 %: the point at which all N₂
    /// would be depleted and the output is pure O₂ + Ar.
    #[error("O₂ fraction {0} exceeds the blend method ceiling")]
    BlendCeilingExceeded(Percent),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Percent;
    use color_eyre::Result;

    #[test]
    fn o2_too_low_display_contains_fraction() -> Result<(), Box<dyn std::error::Error>> {
        let p = Percent::new(0.05).ok_or("invalid")?;
        let msg = InvalidEANx::O2TooLow(p).to_string();

        assert!(msg.contains('5') || msg.contains("0.05"));

        Ok(())
    }

    #[test]
    fn blend_ceiling_exceeded_display() -> Result<(), Box<dyn std::error::Error>> {
        let p = Percent::new(0.99).ok_or("invalid")?;
        let msg = InvalidEANx::BlendCeilingExceeded(p).to_string();

        assert!(msg.contains("ceiling"));

        Ok(())
    }
}
