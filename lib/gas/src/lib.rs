#![cfg_attr(
    test,
    expect(
        clippy::panic_in_result_fn,
        reason = "Tests legitimately combine Result return types with panic-inducing assertions"
    )
)]

//! Gas mix types and blending models for dive planning.
//!
//! # Overview
//!
//! The central type is [`EANxBlend`](crate::prelude::EANxBlend), parameterised by a [`BlendMethod`](crate::prelude::BlendMethod) that
//! determines the full gas composition ($\ce{N2}$, Ar, $\ce{CO2}$, trace gases) from the $\ce{O2}$
//! fraction alone — or from measured gas-analysis data for membrane systems.
//!
//! Three blend methods are provided:
//!
//! | Type | Diluent | $\ce{Ar}$ | $\ce{CO2}$ |
//! |---|---|---|---|
//! | [`PartialPressure`](crate::prelude::PartialPressure) | air-derived | ≈ air ratio | ≈ air ratio |
//! | [`Psa`](crate::prelude::Psa) | $\ce{N2}$ stripped | co-concentrates with $\ce{O2}$ | stripped by zeolite |
//! | [`Membrane`](crate::prelude::Membrane) | equipment-dependent | enriched vs air | enriched vs air |
//!
//! The [`EANx`](crate::prelude::EANx) type alias covers the common partial-pressure case.
//!
//! # Example
//!
//! ```
//! use dps_environment::DiveEnvironment;
//! use dps_gas::prelude::*;
//! use dps_units::{Bar, Meters, Percent};
//!
//! let ean32 = EANx::try_from(Percent::new(0.32).unwrap()).unwrap();
//!
//! // O₂ toxicity depth limit
//! let m = ean32.mod_at(Bar::new(1.4));
//! assert_eq!(m.to_string(), "33.4 m");
//!
//! // Narcotic equivalent depth at 30 m — shallower than actual for enriched air
//! let end = ean32.end_at(Meters::new(30.0));
//! assert!(end.depth() < Meters::new(30.0));
//!
//! // Best mix for a 30 m dive
//! let best = EANx::best_mix(Meters::new(30.0), Bar::new(1.4), DiveEnvironment::standard()).unwrap();
//! ```

mod blend;
mod components;
mod constants;
mod eanx;
mod error;

pub mod prelude;

#[cfg(test)]
mod tests {
    use crate::blend::{Membrane, Psa};
    use crate::eanx::{EANx, EANxBlend, InvalidEANxError};
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
