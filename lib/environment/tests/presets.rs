#![cfg_attr(
    test,
    expect(
        clippy::panic_in_result_fn,
        reason = "Tests legitimately combine Result return types with panic-inducing assertions"
    )
)]

//! Exhaustive preset coverage — every `Ocean` and `Lake` variant, plus
//! direct `Ocean`/`Lake` API items that consumers use independently of
//! `DiveEnvironment`.

use dps_environment::{DiveEnvironment, DiveEnvironmentError, Lake, Ocean};

use rstest::rstest;
use strum::{IntoEnumIterator, VariantNames};

mod ocean {
    use super::*;

    // Every Ocean variant must survive a Display → FromStr roundtrip back to
    // the same DiveEnvironment. Catches strum rename-attribute divergences
    // between Display and FromStr that are invisible from inside the crate.
    #[rstest]
    fn all_roundtrip_through_display_and_fromstr() -> Result<(), DiveEnvironmentError> {
        for ocean in Ocean::iter() {
            let env = DiveEnvironment::ocean(ocean);
            let s = env.to_string();
            let parsed: DiveEnvironment = s.parse()?;

            assert_eq!(parsed, env, "roundtrip mismatch for {ocean:?}");
        }
        Ok(())
    }

    // `Display` and `to_clipboard_string` must agree for every preset.
    // They use different float formatters internally; this verifies they
    // produce the same output for the named-preset branch.
    #[rstest]
    fn display_equals_clipboard_string_for_all_presets() {
        for ocean in Ocean::iter() {
            let env = DiveEnvironment::ocean(ocean);

            assert_eq!(
                env.to_string(),
                env.to_clipboard_string(),
                "Display/to_clipboard_string mismatch for {ocean:?}"
            );
        }
    }

    // Every ocean preset must produce `"ocean:{name}"` — including the five
    // variants whose physical values coincide with another preset under the
    // linear density model.  The tag stored at construction time is used
    // directly, so no value-equality scan is needed and no variant is
    // stolen by a collision sibling.
    #[rstest]
    fn all_produce_named_clipboard_string() {
        for ocean in Ocean::iter() {
            let s = DiveEnvironment::ocean(ocean).to_clipboard_string();

            assert_eq!(
                s,
                format!("ocean:{ocean}"),
                "clipboard mismatch for {ocean:?}"
            );
        }
    }

    // Every ocean variant displays as `"ocean:{name}"` via `Display`.
    #[rstest]
    fn all_display_as_ocean_name() {
        for ocean in Ocean::iter() {
            assert_eq!(
                DiveEnvironment::ocean(ocean).to_string(),
                format!("ocean:{ocean}"),
                "Display mismatch for {ocean:?}"
            );
        }
    }

    // Presets that share identical physical values still round-trip through
    // their own names because the construction-time tag drives serialisation.
    #[rstest]
    fn value_equal_presets_roundtrip_independently() -> Result<(), DiveEnvironmentError> {
        // Atlantic (35.5 ‰, 17 °C) is value-equal to DiveEnvironment::standard()
        // but must serialise as "ocean:Atlantic", not "standard".
        let atlantic = DiveEnvironment::ocean(Ocean::Atlantic);

        assert_eq!(
            atlantic,
            DiveEnvironment::standard(),
            "Atlantic must be value-equal to standard"
        );

        assert_eq!(atlantic.to_string(), "ocean:Atlantic");
        assert_eq!("ocean:Atlantic".parse::<DiveEnvironment>()?, atlantic);

        // Four ocean-to-ocean collision pairs.
        let pairs: &[(Ocean, Ocean)] = [
            (Ocean::RedSea, Ocean::Southern),
            (Ocean::NorthSea, Ocean::PersianGulf),
            (Ocean::SouthChinaSea, Ocean::AndamanSea),
            (Ocean::CelebesSea, Ocean::BandaSea),
        ]
        .as_slice();

        for &(a, b) in pairs {
            let env_a = DiveEnvironment::ocean(a);
            let env_b = DiveEnvironment::ocean(b);

            assert_eq!(env_a, env_b, "{a:?} and {b:?} should be value-equal");
            assert_ne!(
                env_a.to_string(),
                env_b.to_string(),
                "{a:?} and {b:?} must not share a display string"
            );

            let parsed_a: DiveEnvironment = env_a.to_string().parse()?;
            let parsed_b: DiveEnvironment = env_b.to_string().parse()?;

            assert_eq!(parsed_a, env_a, "roundtrip failed for {a:?}");
            assert_eq!(parsed_b, env_b, "roundtrip failed for {b:?}");
        }

        Ok(())
    }

    /// Direct `Ocean::Display` and `Ocean::from_str` — used independently of
    /// `DiveEnvironment` (e.g. for dropdown population or logging).
    #[rstest]
    fn direct_display_fromstr_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        for ocean in Ocean::iter() {
            let displayed = ocean.to_string();
            let parsed: Ocean = displayed.parse()?;

            assert_eq!(parsed, ocean);
        }

        Ok(())
    }

    /// `Ocean::VARIANTS` must list exactly one entry per variant, with the
    /// same name that `Display` and `FromStr` use.
    #[rstest]
    fn variants_constant_matches_iter_and_display() {
        let variants = Ocean::VARIANTS;

        assert_eq!(
            variants.len(),
            Ocean::iter().count(),
            "VARIANTS length must match variant count"
        );

        for ocean in Ocean::iter() {
            let name = ocean.to_string();

            assert!(
                variants.contains(&name.as_str()),
                "VARIANTS missing '{name}' (from {ocean:?})"
            );
        }
    }
}

mod lake {
    use super::*;

    #[rstest]
    fn all_roundtrip_through_display_and_fromstr() -> Result<(), DiveEnvironmentError> {
        for lake in Lake::iter() {
            let env = DiveEnvironment::lake(lake);
            let s = env.to_string();
            let parsed: DiveEnvironment = s.parse()?;

            assert_eq!(parsed, env, "roundtrip mismatch for {lake:?}");
        }

        Ok(())
    }

    /// Every Lake preset must produce a named `"lake:..."` string — no lake
    /// variant collides with `standard`, `freshwater`, or another lake.
    #[rstest]
    fn all_produce_named_clipboard_string() {
        for lake in Lake::iter() {
            let s = DiveEnvironment::lake(lake).to_clipboard_string();

            assert!(
                s.starts_with("lake:"),
                "expected 'lake:...' for {lake:?}, got: {s}"
            );
        }
    }

    #[rstest]
    fn display_equals_clipboard_string_for_all_presets() {
        for lake in Lake::iter() {
            let env = DiveEnvironment::lake(lake);

            assert_eq!(
                env.to_string(),
                env.to_clipboard_string(),
                "Display/to_clipboard_string mismatch for {lake:?}"
            );
        }
    }

    #[rstest]
    fn direct_display_fromstr_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        for lake in Lake::iter() {
            let displayed = lake.to_string();
            let parsed: Lake = displayed.parse()?;

            assert_eq!(parsed, lake);
        }

        Ok(())
    }

    #[rstest]
    fn variants_constant_matches_iter_and_display() {
        let variants = Lake::VARIANTS;

        assert_eq!(
            variants.len(),
            Lake::iter().count(),
            "VARIANTS length must match variant count"
        );

        for lake in Lake::iter() {
            let name = lake.to_string();

            assert!(
                variants.contains(&name.as_str()),
                "VARIANTS missing '{name}' (from {lake:?})"
            );
        }
    }

    #[rstest]
    fn all_display_as_lake_name() {
        for lake in Lake::iter() {
            assert_eq!(
                DiveEnvironment::lake(lake).to_string(),
                format!("lake:{lake}"),
                "format mismatch for {lake:?}"
            );
        }
    }
}
