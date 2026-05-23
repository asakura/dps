//! Gas mix types and blending models for dive planning.
//!
//! # Overview
//!
//! The central type is [`EANxBlend<M>`], parameterised by a [`BlendMethod`] that
//! determines the full gas composition (N₂, Ar, CO₂, trace gases) from the O₂
//! fraction alone — or from measured gas-analysis data for membrane systems.
//!
//! Three blend methods are provided:
//!
//! | Type | Diluent | Ar | CO₂ |
//! |---|---|---|---|
//! | [`PartialPressure`] | air-derived | ≈ air ratio | ≈ air ratio |
//! | [`Psa`] | N₂ stripped | co-concentrates with O₂ | stripped by zeolite |
//! | [`Membrane`] | equipment-dependent | enriched vs air | enriched vs air |
//!
//! The [`EANx`] type alias covers the common partial-pressure case.
//!
//! # Example
//!
//! ```no_run
//! use dps::gas::{EANx, BlendMethod};
//! use dps::units::{Bar, Meters, Percent};
//!
//! let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
//!
//! // O₂ toxicity depth limit
//! let m = ean32.mod_at(Bar::new(1.4));
//! assert_eq!(m.to_string(), "33.8 m");
//!
//! // Narcotic equivalent depth at 30 m
//! let end = ean32.end_at(Meters::new(30.0));
//!
//! // Best mix for a 30 m dive
//! let best = EANx::best_mix(Meters::new(30.0), Bar::new(1.4)).unwrap();
//! ```

mod blend;
mod components;
mod constants;
mod eanx;
pub mod error;

pub use blend::{BlendMethod, InvalidMembraneFractions, Membrane, PartialPressure, Psa};
pub use components::GasComponents;
pub use eanx::InvalidEANx;
pub use eanx::{EANx, EANxBlend, MOD, MODSummary, MiniMOD, MiniMODSummary};
pub use error::Error as GasError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{Meters, Percent};
    use color_eyre::{Result, eyre::eyre};

    fn ean(fraction: f64) -> Result<EANx> {
        let pct =
            Percent::new(fraction).ok_or_else(|| eyre!("fraction {fraction} out of [0.0, 1.0]"))?;

        Ok(EANx::try_from(pct)?)
    }

    fn ean_psa(fraction: f64) -> Result<EANxBlend<Psa>> {
        let pct =
            Percent::new(fraction).ok_or_else(|| eyre!("fraction {fraction} out of [0.0, 1.0]"))?;

        Ok(EANxBlend::new(pct, Psa)?)
    }

    #[test]
    fn pp_and_psa_have_different_ar_at_ean32() -> Result<()> {
        assert!(
            ean_psa(0.32)?.far() > ean(0.32)?.far(),
            "PSA should have more Ar than PP at fo2 = 0.32"
        );

        Ok(())
    }

    #[test]
    fn membrane_typical_has_higher_ar_than_pp() -> Result<()> {
        let fo2 = Percent::new(0.32).ok_or_else(|| eyre!("0.32 is in [0.0, 1.0]"))?;
        let mem_mix = EANxBlend::new(fo2, Membrane::typical())?;

        assert!(
            mem_mix.far() > ean(0.32)?.far(),
            "typical membrane should have more Ar than PP"
        );

        Ok(())
    }

    #[test]
    fn psa_is_denser_than_pp_at_same_fo2() -> Result<()> {
        let pp = ean(0.32)?.gas_density_at(Meters::new(0.0));
        let psa = ean_psa(0.32)?.gas_density_at(Meters::new(0.0));

        assert!(psa > pp, "PSA has more Ar (MW 40), so it should be denser");

        Ok(())
    }
}
