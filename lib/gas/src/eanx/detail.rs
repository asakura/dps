use super::EANxBlend;

use crate::blend::BlendMethod;

use std::fmt;

/// Display wrapper that prints extended information about a gas mix.
///
/// Shows the gas name, blend method, and full component breakdown as mole
/// fractions. Produced by [`EANxBlend::detail`].
///
/// ```no_run
/// use dps_gas::EANx;
/// use dps_units::Percent;
///
/// let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
/// println!("{}", ean32.detail());
/// // EANx 32 (partial pressure)
/// //   O₂      32.000 %
/// //   N₂      67.159 %
/// //   Ar       0.803 %
/// //   CO₂      0.035 %
/// //   other    0.002 %
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EANxDetail<M: BlendMethod>(EANxBlend<M>);

impl<M: BlendMethod> EANxDetail<M> {
    /// Unwraps the inner [`EANxBlend`].
    #[must_use]
    pub const fn into_inner(self) -> EANxBlend<M> {
        self.0
    }
}

impl<M: BlendMethod> From<EANxBlend<M>> for EANxDetail<M> {
    fn from(blend: EANxBlend<M>) -> Self {
        Self(blend)
    }
}

impl<M: BlendMethod> fmt::Display for EANxDetail<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = self.0.components();

        // Each label prefix is exactly 8 terminal columns so the decimal points
        // in the {:7.3} value field align across all five component lines.
        write!(f, "{} ({})", self.0, self.0.blend_name())?;
        write!(f, "\n  O₂    {:7.3} %", c.o2() * 100.0)?;
        write!(f, "\n  N₂    {:7.3} %", c.n2() * 100.0)?;
        write!(f, "\n  Ar    {:7.3} %", c.ar() * 100.0)?;
        write!(f, "\n  CO₂   {:7.3} %", c.co2() * 100.0)?;
        write!(f, "\n  other {:7.3} %", c.other() * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::eanx::InvalidEANxError;
    use crate::{EANx, EANxBlend, Membrane, Psa};
    use dps_units::Percent;

    fn ean(fraction: f64) -> Result<EANx, InvalidEANxError> {
        let pct = Percent::new(fraction)?;
        EANx::try_from(pct)
    }

    fn ean_psa(fraction: f64) -> Result<EANxBlend<Psa>, InvalidEANxError> {
        let pct = Percent::new(fraction)?;
        EANxBlend::new(pct, Psa)
    }

    fn ean_membrane(fraction: f64) -> Result<EANxBlend<Membrane>, InvalidEANxError> {
        let pct = Percent::new(fraction)?;
        EANxBlend::new(pct, Membrane::typical())
    }

    mod display {
        use super::*;

        #[test]
        fn first_line_contains_gas_name_and_blend_method() -> Result<(), InvalidEANxError> {
            let first = ean(0.32)?
                .detail()
                .to_string()
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned();

            assert_eq!(first, "EANx 32 (partial pressure)");

            Ok(())
        }

        #[test]
        fn psa_blend_shows_psa_name() -> Result<(), InvalidEANxError> {
            let first = ean_psa(0.32)?
                .detail()
                .to_string()
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned();

            assert_eq!(first, "EANx 32 (PSA)");

            Ok(())
        }

        #[test]
        fn membrane_blend_shows_membrane_name() -> Result<(), InvalidEANxError> {
            let first = ean_membrane(0.32)?
                .detail()
                .to_string()
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned();

            assert_eq!(first, "EANx 32 (membrane)");

            Ok(())
        }

        #[test]
        fn air_shows_gas_name_and_blend_method() -> Result<(), InvalidEANxError> {
            let first = ean(0.21)?
                .detail()
                .to_string()
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned();

            assert_eq!(first, "Air (partial pressure)");

            Ok(())
        }

        #[test]
        fn pure_o2_shows_gas_name_and_blend_method() -> Result<(), InvalidEANxError> {
            let first = ean(1.0)?
                .detail()
                .to_string()
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned();

            assert_eq!(first, "Pure O₂ (partial pressure)");

            Ok(())
        }

        #[test]
        fn all_component_labels_appear() -> Result<(), InvalidEANxError> {
            let output = ean(0.32)?.detail().to_string();

            assert!(output.contains("O₂"), "missing O₂ in: {output}");
            assert!(output.contains("N₂"), "missing N₂ in: {output}");
            assert!(output.contains("Ar"), "missing Ar in: {output}");
            assert!(output.contains("CO₂"), "missing CO₂ in: {output}");
            assert!(output.contains("other"), "missing other in: {output}");

            Ok(())
        }

        #[test]
        fn o2_fraction_value_appears() -> Result<(), InvalidEANxError> {
            let output = ean(0.32)?.detail().to_string();
            assert!(output.contains("32.000"), "expected 32.000 in: {output}");

            Ok(())
        }

        #[test]
        fn pure_o2_shows_100_percent_o2() -> Result<(), InvalidEANxError> {
            let output = ean(1.0)?.detail().to_string();
            assert!(output.contains("100.000"), "expected 100.000 in: {output}");

            Ok(())
        }

        #[test]
        fn has_five_component_lines() -> Result<(), InvalidEANxError> {
            let output = ean(0.32)?.detail().to_string();
            assert_eq!(output.lines().count(), 6);

            Ok(())
        }

        #[test]
        fn from_impl_matches_detail_method() -> Result<(), InvalidEANxError> {
            let mix = ean(0.32)?;
            assert_eq!(EANxDetail::from(mix).to_string(), mix.detail().to_string());

            Ok(())
        }

        #[test]
        fn into_inner_recovers_original_blend() -> Result<(), InvalidEANxError> {
            let mix = ean(0.32)?;
            assert_eq!(mix.detail().into_inner(), mix);

            Ok(())
        }
    }
}
