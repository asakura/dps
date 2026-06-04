#![cfg_attr(
    test,
    expect(
        clippy::panic_in_result_fn,
        reason = "Tests legitimately combine Result return types with panic-inducing assertions"
    )
)]
#![allow(
    rustdoc::private_doc_tests,
    reason = "Module-level doc examples reference crate paths that are private to rustdoc"
)]

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
//! use dps_environment::DiveEnvironment;
//! use dps_gas::{EANx, BlendMethod};
//! use dps_units::{Bar, Meters, Percent};
//!
//! let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
//!
//! // O₂ toxicity depth limit
//! let m = ean32.mod_at(Bar::new(1.4));
//! assert_eq!(m.to_string(), "33.4 m");
//!
//! // Narcotic equivalent depth at 30 m
//! let end = ean32.end_at(Meters::new(30.0));
//!
//! // Best mix for a 30 m dive
//! let best = EANx::best_mix(Meters::new(30.0), Bar::new(1.4), DiveEnvironment::standard()).unwrap();
//! ```

mod blend;
mod components;
pub mod constants;
mod eanx;
pub mod error;

pub use blend::{BlendMethod, InvalidMembraneFractionsError, Membrane, PartialPressure, Psa};
pub use components::GasComponents;
pub use constants::{
    AIR_AR, AIR_CO2, AIR_DILUENT, AIR_N2, AIR_NARCOTIC, AIR_O2, AIR_OTHER, EAN_MIN_O2,
};
pub use eanx::{
    EAD, EADSummary, EANx, EANxBlend, EANxDetail, END, ENDSummary, InvalidEANxError, MND,
    MNDSummary, MOD, MODSummary, MiniMOD, MiniMODSummary, PPO2, ParseEANxError, Ppo2Summary,
};
pub use error::Error as GasError;

#[cfg(test)]
mod tests {
    use super::*;
    use dps_units::{Meters, Percent};

    fn ean(fraction: f64) -> Result<EANx, InvalidEANxError> {
        let pct = Percent::new(fraction)?;
        EANx::try_from(pct)
    }

    fn ean_psa(fraction: f64) -> Result<EANxBlend<Psa>, InvalidEANxError> {
        let pct = Percent::new(fraction)?;
        EANxBlend::new(pct, Psa)
    }

    #[test]
    fn pp_and_psa_have_different_ar_at_ean32() -> Result<(), InvalidEANxError> {
        assert!(
            ean_psa(0.32)?.far() > ean(0.32)?.far(),
            "PSA should have more Ar than PP at fo2 = 0.32"
        );

        Ok(())
    }

    #[test]
    fn membrane_typical_has_higher_ar_than_pp() -> Result<(), InvalidEANxError> {
        let fo2 = Percent::new(0.32)?;
        let mem_mix = EANxBlend::new(fo2, Membrane::typical())?;

        assert!(
            mem_mix.far() > ean(0.32)?.far(),
            "typical membrane should have more Ar than PP"
        );

        Ok(())
    }

    #[test]
    fn psa_is_denser_than_pp_at_same_fo2() -> Result<(), InvalidEANxError> {
        let pp = ean(0.32)?.gas_density_at(Meters::new(0.0));
        let psa = ean_psa(0.32)?.gas_density_at(Meters::new(0.0));

        assert!(psa > pp, "PSA has more Ar (MW 40), so it should be denser");

        Ok(())
    }
}
