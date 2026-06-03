#![cfg_attr(
    test,
    expect(
        clippy::panic_in_result_fn,
        reason = "Tests legitimately combine Result return types with panic-inducing assertions"
    )
)]
#![cfg(feature = "serde")]

//! Serde roundtrip tests — only compiled when `--features serde` is active.
//!
//! Verifies that the optional `serde` feature correctly serialises and
//! deserialises all public gas types without data loss.

use approx::assert_relative_eq;
use dps_gas::{
    EAD, EADSummary, EANx, EANxBlend, EANxDetail, END, ENDSummary, GasComponents, MND,
    MNDSummary, MOD, MODSummary, Membrane, MiniMOD, MiniMODSummary, PPO2, PartialPressure,
    Ppo2Summary, Psa,
};
use dps_units::{Bar, Meters, Percent};
use rstest::rstest;

mod eanx {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let ean32 = EANx::try_from(Percent::new(0.32)?)?;
        let json = serde_json::to_string(&ean32)?;
        let parsed: EANx = serde_json::from_str(&json)?;

        assert_eq!(ean32, parsed);
        assert_eq!(parsed.fo2(), Percent::new(0.32)?);

        Ok(())
    }

    #[rstest]
    fn validation_rejects_invalid_fo2() -> Result<(), Box<dyn std::error::Error>> {
        let ean = EANx::try_from(Percent::new(0.32)?)?;
        let mut value: serde_json::Value = serde_json::from_str(&serde_json::to_string(&ean)?)?;

        value["fo2"] = serde_json::Value::String("9%".to_string());

        let result: Result<EANx, _> = serde_json::from_value(value);

        assert!(result.is_err());

        Ok(())
    }
}

mod eanx_blend {
    use super::*;

    #[rstest]
    fn psa_json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let psa32 = EANxBlend::new(Percent::new(0.32)?, Psa)?;
        let json = serde_json::to_string(&psa32)?;
        let parsed: EANxBlend<Psa> = serde_json::from_str(&json)?;

        assert_eq!(psa32, parsed);

        Ok(())
    }
}

mod gas_components {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let ean32 = EANx::try_from(Percent::new(0.32)?)?;
        let components = ean32.components();
        let json = serde_json::to_string(&components)?;
        let parsed: GasComponents = serde_json::from_str(&json)?;

        assert_eq!(components, parsed);
        assert_relative_eq!(parsed.sum(), 1.0);

        Ok(())
    }

    #[rstest]
    fn validation_rejects_fractions_not_summing_to_one() -> Result<(), &'static str> {
        let json = r#"{"o2": 0.32, "n2": 0.6, "ar": 0.01, "co2": 0.0, "other": 0.0}"#;
        let err = serde_json::from_str::<GasComponents>(json)
            .err()
            .ok_or("expected GasComponents deserialization to fail")?;

        assert!(
            err.to_string()
                .contains("GasComponents fractions must sum to 1.0")
        );

        Ok(())
    }
}

mod maximum_operating_depth {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let ean32 = EANx::try_from(Percent::new(0.32)?)?;
        let m = ean32.mod_at(Bar::new(1.4));
        let json = serde_json::to_string(&m)?;
        let parsed: MOD = serde_json::from_str(&json)?;

        assert_eq!(m, parsed);
        assert_eq!(parsed.depth(), m.depth());

        Ok(())
    }

    #[rstest]
    fn validation_rejects_fo2_below_minimum() -> Result<(), Box<dyn std::error::Error>> {
        let m = EANx::try_from(Percent::new(0.32)?)?.mod_at(Bar::new(1.4));
        let mut value: serde_json::Value = serde_json::from_str(&serde_json::to_string(&m)?)?;

        value["fo2"] = serde_json::Value::String("9%".to_string());

        let result: Result<MOD, _> = serde_json::from_value(value);

        assert!(result.is_err());

        Ok(())
    }
}

mod mod_summary {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let m = EANx::try_from(Percent::new(0.32)?)?.mod_at(Bar::new(1.4));
        let summary = m.summary();
        let json = serde_json::to_string(&summary)?;
        let parsed: MODSummary = serde_json::from_str(&json)?;

        assert_eq!(summary, parsed);
        assert_eq!(parsed.into_inner(), m);

        Ok(())
    }
}

mod minimum_operating_depth {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let h10 = EANx::try_from(Percent::new(0.10)?)?.minimod_at(Bar::new(0.16));
        let json = serde_json::to_string(&h10)?;
        let parsed: MiniMOD = serde_json::from_str(&json)?;

        assert_eq!(h10, parsed);
        assert_eq!(parsed.depth(), h10.depth());

        Ok(())
    }

    #[rstest]
    fn validation_rejects_fo2_below_minimum() -> Result<(), Box<dyn std::error::Error>> {
        let h10 = EANx::try_from(Percent::new(0.10)?)?.minimod_at(Bar::new(0.16));
        let mut value: serde_json::Value = serde_json::from_str(&serde_json::to_string(&h10)?)?;

        value["fo2"] = serde_json::Value::String("9%".to_string());

        let result: Result<MiniMOD, _> = serde_json::from_value(value);

        assert!(result.is_err());

        Ok(())
    }
}

mod minimod_summary {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let h10 = EANx::try_from(Percent::new(0.10)?)?.minimod_at(Bar::new(0.16));
        let summary = h10.summary();
        let json = serde_json::to_string(&summary)?;
        let parsed: MiniMODSummary = serde_json::from_str(&json)?;

        assert_eq!(summary, parsed);
        assert_eq!(parsed.into_inner(), h10);

        Ok(())
    }
}

mod membrane {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let mem = Membrane::typical();
        let json = serde_json::to_string(&mem)?;
        let parsed: Membrane = serde_json::from_str(&json)?;

        // Fields are private; compare blend output at a fixed FO₂.
        let fo2 = Percent::new(0.32)?;
        let orig_mix = EANxBlend::new(fo2, mem)?;
        let parsed_mix = EANxBlend::new(fo2, parsed)?;

        assert_relative_eq!(orig_mix.fn2(), parsed_mix.fn2(), epsilon = 1e-12);
        assert_relative_eq!(orig_mix.far(), parsed_mix.far(), epsilon = 1e-12);

        Ok(())
    }

    #[rstest]
    fn validation_rejects_ratios_not_summing_to_one() -> Result<(), &'static str> {
        let json = r#"{"fn2": 0.5, "far": 0.3, "fco2": 0.1, "fother": 0.0}"#;
        let err = serde_json::from_str::<Membrane>(json)
            .err()
            .ok_or("expected Membrane deserialization to fail")?;

        assert!(
            err.to_string()
                .contains("Membrane diluent ratios must sum to 1.0")
        );

        Ok(())
    }
}

mod eanx_detail {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let ean32 = EANx::try_from(Percent::new(0.32)?)?;
        let detail = ean32.detail();
        let json = serde_json::to_string(&detail)?;
        let parsed: EANxDetail<PartialPressure> = serde_json::from_str(&json)?;

        assert_eq!(detail, parsed);
        assert_eq!(parsed.into_inner(), ean32);

        Ok(())
    }
}

mod equivalent_air_depth {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let e = EANx::try_from(Percent::new(0.32)?)?.ead_at(Meters::new(30.0));
        let json = serde_json::to_string(&e)?;
        let parsed: EAD = serde_json::from_str(&json)?;

        assert_eq!(e, parsed);
        assert_eq!(parsed.ead(), e.ead());
        assert_eq!(parsed.actual_depth(), e.actual_depth());

        Ok(())
    }
}

mod ead_summary {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let e = EANx::try_from(Percent::new(0.32)?)?.ead_at(Meters::new(30.0));
        let summary = e.summary();
        let json = serde_json::to_string(&summary)?;
        let parsed: EADSummary = serde_json::from_str(&json)?;

        assert_eq!(summary, parsed);
        assert_eq!(parsed.into_inner(), e);

        Ok(())
    }
}

mod equivalent_narcotic_depth {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let e = EANx::try_from(Percent::new(0.32)?)?.end_at(Meters::new(30.0));
        let json = serde_json::to_string(&e)?;
        let parsed: END = serde_json::from_str(&json)?;

        assert_eq!(e, parsed);
        assert_eq!(parsed.end(), e.end());
        assert_eq!(parsed.actual_depth(), e.actual_depth());

        Ok(())
    }
}

mod end_summary {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let e = EANx::try_from(Percent::new(0.32)?)?.end_at(Meters::new(30.0));
        let summary = e.summary();
        let json = serde_json::to_string(&summary)?;
        let parsed: ENDSummary = serde_json::from_str(&json)?;

        assert_eq!(summary, parsed);
        assert_eq!(parsed.into_inner(), e);

        Ok(())
    }
}

mod maximum_narcotic_depth {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let m = EANx::try_from(Percent::new(0.32)?)?.mnd_at(Meters::new(30.0));
        let json = serde_json::to_string(&m)?;
        let parsed: MND = serde_json::from_str(&json)?;

        assert_eq!(m, parsed);
        assert_eq!(parsed.mnd(), m.mnd());
        assert_eq!(parsed.end_limit(), m.end_limit());

        Ok(())
    }
}

mod mnd_summary {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let m = EANx::try_from(Percent::new(0.32)?)?.mnd_at(Meters::new(30.0));
        let summary = m.summary();
        let json = serde_json::to_string(&summary)?;
        let parsed: MNDSummary = serde_json::from_str(&json)?;

        assert_eq!(summary, parsed);
        assert_eq!(parsed.into_inner(), m);

        Ok(())
    }
}

mod ppo2 {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        // Use a depth where ppo2 is an exact decimal to avoid 1-ULP round-trip loss.
        // EANx 32 at 33.75 m → ppO₂ = 1.4 bar.
        let p = EANx::try_from(Percent::new(0.32)?)?.ppo2_at(Meters::new(33.75));
        let json = serde_json::to_string(&p)?;
        let parsed: PPO2 = serde_json::from_str(&json)?;

        assert_eq!(p, parsed);
        assert_eq!(parsed.fo2(), p.fo2());
        assert_eq!(parsed.depth(), p.depth());

        Ok(())
    }
}

mod ppo2_summary {
    use super::*;

    #[rstest]
    fn json_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let p = EANx::try_from(Percent::new(0.32)?)?.ppo2_at(Meters::new(33.75));
        let summary = p.summary();
        let json = serde_json::to_string(&summary)?;
        let parsed: Ppo2Summary = serde_json::from_str(&json)?;

        assert_eq!(summary, parsed);
        assert_eq!(parsed.into_inner(), p);

        Ok(())
    }
}
