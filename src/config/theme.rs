//! Theme and palette configuration types, and the [`resolve_theme`] resolver.

use std::collections::HashMap;

use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

use super::error::{Error, ThemeResolutionError};
use crate::theme::Theme;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum ModifierName {
    Bold,
    Dim,
    Italic,
    Underlined,
    SlowBlink,
    RapidBlink,
    Reversed,
    Hidden,
    CrossedOut,
}

impl From<ModifierName> for Modifier {
    fn from(m: ModifierName) -> Self {
        match m {
            ModifierName::Bold => Self::BOLD,
            ModifierName::Dim => Self::DIM,
            ModifierName::Italic => Self::ITALIC,
            ModifierName::Underlined => Self::UNDERLINED,
            ModifierName::SlowBlink => Self::SLOW_BLINK,
            ModifierName::RapidBlink => Self::RAPID_BLINK,
            ModifierName::Reversed => Self::REVERSED,
            ModifierName::Hidden => Self::HIDDEN,
            ModifierName::CrossedOut => Self::CROSSED_OUT,
        }
    }
}

/// Foreground (and optional background) colour name for one semantic theme slot.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ThemeSlotConfig {
    fg: Option<String>,
    bg: Option<String>,
    modifiers: Option<Vec<ModifierName>>,
}

/// One named theme entry: maps every semantic slot to palette colour names.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ThemeConfig {
    popup_surface: ThemeSlotConfig,
    key_label: ThemeSlotConfig,
    border: ThemeSlotConfig,
    title: ThemeSlotConfig,
    header: ThemeSlotConfig,
    header_cell: ThemeSlotConfig,
    header_cell_active: ThemeSlotConfig,
    selection: ThemeSlotConfig,
    column_focus: ThemeSlotConfig,
    nav_bar: ThemeSlotConfig,
    status_active: ThemeSlotConfig,
    status_empty: ThemeSlotConfig,
    safe: ThemeSlotConfig,
    caution: ThemeSlotConfig,
    danger: ThemeSlotConfig,
    body_text: ThemeSlotConfig,
    hint: ThemeSlotConfig,
}

/// One named palette entry: an open map of colour name → CSS hex value (`#rrggbb`).
///
/// Any colour names may be used; only those referenced by the active theme's
/// slot mappings need to be present.
pub(super) type PaletteConfig = HashMap<String, String>;

/// Resolves every entry in `themes` against its matching palette, returning a
/// map of name → fully constructed [`Theme`].
///
/// Each theme key must have a corresponding palette with the same key.
///
/// # Errors
///
/// Returns `Err` if any theme references a missing palette, or if any colour
/// name in a slot mapping is absent from the palette.
pub(super) fn resolve_theme(
    themes: &HashMap<String, ThemeConfig>,
    palettes: &HashMap<String, PaletteConfig>,
) -> Result<HashMap<String, Theme>, Error> {
    themes
        .iter()
        .map(|(name, tc)| {
            let pc = palettes
                .get(name)
                .ok_or_else(|| ThemeResolutionError::MissingPalette(name.clone()))?;
            Ok((name.clone(), resolve_single(tc, pc)?))
        })
        .collect()
}

fn resolve_single(tc: &ThemeConfig, pc: &PaletteConfig) -> Result<Theme, Error> {
    let pal = parse_palette(pc)?;
    let resolve = |slot: &ThemeSlotConfig| slot_to_style(slot, &pal);

    Ok(Theme::new(
        resolve(&tc.popup_surface)?,
        resolve(&tc.key_label)?,
        resolve(&tc.border)?,
        resolve(&tc.title)?,
        resolve(&tc.header)?,
        resolve(&tc.header_cell)?,
        resolve(&tc.header_cell_active)?,
        resolve(&tc.selection)?,
        resolve(&tc.column_focus)?,
        resolve(&tc.nav_bar)?,
        resolve(&tc.status_active)?,
        resolve(&tc.status_empty)?,
        resolve(&tc.safe)?,
        resolve(&tc.caution)?,
        resolve(&tc.danger)?,
        resolve(&tc.body_text)?,
        resolve(&tc.hint)?,
    ))
}

fn slot_to_style(slot: &ThemeSlotConfig, pal: &HashMap<String, Color>) -> Result<Style, Error> {
    let lookup = |name: &str| {
        pal.get(name)
            .copied()
            .ok_or_else(|| ThemeResolutionError::UnknownColour(name.to_string()))
    };

    let mut style = Style::default();

    if let Some(name) = &slot.fg {
        style = style.fg(lookup(name)?);
    }
    if let Some(name) = &slot.bg {
        style = style.bg(lookup(name)?);
    }
    if let Some(mods) = &slot.modifiers {
        for m in mods {
            style = style.add_modifier(Modifier::from(*m));
        }
    }

    Ok(style)
}

fn parse_palette(cfg: &HashMap<String, String>) -> Result<HashMap<String, Color>, Error> {
    cfg.iter()
        .map(|(name, hex)| Ok((name.clone(), parse_hex(hex)?)))
        .collect()
}

fn parse_hex(s: &str) -> Result<Color, Error> {
    let hex = s
        .strip_prefix('#')
        .ok_or_else(|| ThemeResolutionError::MissingHexPrefix(s.to_string()))?;

    if hex.len() != 6 {
        return Err(ThemeResolutionError::InvalidHexLength {
            value: s.to_string(),
            len: hex.len(),
        }
        .into());
    }

    let byte = |i| {
        u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| ThemeResolutionError::InvalidHexDigit {
            value: s.to_string(),
            source: e,
        })
    };

    Ok(Color::Rgb(byte(0)?, byte(2)?, byte(4)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use color_eyre::Result;
    use ratatui::style::{Color, Modifier, Style};
    use rstest::fixture;
    use rstest::rstest;

    #[fixture]
    fn frappe_palette() -> HashMap<String, Color> {
        [
            ("text", Color::Rgb(198, 208, 245)),
            ("mantle", Color::Rgb(41, 44, 60)),
            ("base", Color::Rgb(48, 52, 70)),
            ("surface0", Color::Rgb(65, 69, 89)),
            ("mauve", Color::Rgb(202, 158, 230)),
            ("peach", Color::Rgb(239, 159, 118)),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
    }

    fn deserialize_slot(json: &str) -> Result<ThemeSlotConfig> {
        Ok(serde_json::from_str(json)?)
    }

    /// Builds a `ThemeConfig` with all slots set to `{"fg": colour}`.
    /// Compile-time enforced: adding a field to `ThemeConfig` causes a build error here.
    fn full_theme_config(colour: &str) -> ThemeConfig {
        let slot = ThemeSlotConfig {
            fg: Some(colour.to_string()),
            bg: None,
            modifiers: None,
        };

        ThemeConfig {
            popup_surface: slot.clone(),
            key_label: slot.clone(),
            border: slot.clone(),
            title: slot.clone(),
            header: slot.clone(),
            header_cell: slot.clone(),
            header_cell_active: slot.clone(),
            selection: slot.clone(),
            column_focus: slot.clone(),
            nav_bar: slot.clone(),
            status_active: slot.clone(),
            status_empty: slot.clone(),
            safe: slot.clone(),
            caution: slot.clone(),
            danger: slot.clone(),
            body_text: slot.clone(),
            hint: slot,
        }
    }

    fn full_theme_json(colour: &str) -> Result<String> {
        Ok(serde_json::to_string(&full_theme_config(colour))?)
    }

    mod parse_hex {
        use super::*;

        #[test]
        fn valid_rgb() -> Result<()> {
            assert_eq!(parse_hex("#c6d0f5")?, Color::Rgb(198, 208, 245));

            Ok(())
        }

        #[rstest]
        fn invalid_input_is_err(#[values("c6d0f5", "#c6d0", "#gggggg")] input: &str) {
            assert!(parse_hex(input).is_err());
        }
    }

    mod slot_to_style {
        use super::*;

        #[rstest]
        fn fg_only_sets_fg(frappe_palette: HashMap<String, Color>) -> Result<()> {
            let style = slot_to_style(&deserialize_slot(r#"{"fg":"text"}"#)?, &frappe_palette)?;

            assert_eq!(style.fg, Some(Color::Rgb(198, 208, 245)));
            assert_eq!(style.bg, None);
            assert!(style.add_modifier.is_empty());

            Ok(())
        }

        #[rstest]
        fn bg_only_sets_bg(frappe_palette: HashMap<String, Color>) -> Result<()> {
            let style = slot_to_style(&deserialize_slot(r#"{"bg":"mantle"}"#)?, &frappe_palette)?;

            assert_eq!(style.fg, None);
            assert_eq!(style.bg, Some(Color::Rgb(41, 44, 60)));
            assert!(style.add_modifier.is_empty());

            Ok(())
        }

        #[rstest]
        fn modifier_only_leaves_fg_bg_unset(frappe_palette: HashMap<String, Color>) -> Result<()> {
            let style = slot_to_style(
                &deserialize_slot(r#"{"modifiers":["bold"]}"#)?,
                &frappe_palette,
            )?;

            assert_eq!(style.fg, None);
            assert_eq!(style.bg, None);
            assert!(style.add_modifier.contains(Modifier::BOLD));

            Ok(())
        }

        #[rstest]
        fn multiple_modifiers_all_applied(frappe_palette: HashMap<String, Color>) -> Result<()> {
            let style = slot_to_style(
                &deserialize_slot(r#"{"modifiers":["bold","underlined"]}"#)?,
                &frappe_palette,
            )?;

            assert!(style.add_modifier.contains(Modifier::BOLD));
            assert!(style.add_modifier.contains(Modifier::UNDERLINED));

            Ok(())
        }

        #[rstest]
        fn fg_bg_modifier_all_set(frappe_palette: HashMap<String, Color>) -> Result<()> {
            let style = slot_to_style(
                &deserialize_slot(r#"{"fg":"base","bg":"mauve","modifiers":["bold"]}"#)?,
                &frappe_palette,
            )?;

            assert_eq!(style.fg, Some(Color::Rgb(48, 52, 70)));
            assert_eq!(style.bg, Some(Color::Rgb(202, 158, 230)));
            assert!(style.add_modifier.contains(Modifier::BOLD));

            Ok(())
        }

        #[test]
        fn empty_slot_produces_default_style() -> Result<()> {
            let style = slot_to_style(&deserialize_slot("{}")?, &HashMap::new())?;
            assert_eq!(style, Style::default());

            Ok(())
        }

        #[rstest]
        fn unknown_colour_is_err(
            frappe_palette: HashMap<String, Color>,
            #[values(r#"{"fg":"nosuch"}"#, r#"{"bg":"nosuch"}"#)] json: &str,
        ) -> Result<()> {
            assert!(slot_to_style(&deserialize_slot(json)?, &frappe_palette).is_err());

            Ok(())
        }

        // The colour names in #[values] must match the keys in the `frappe_palette` fixture.
        #[rstest]
        fn all_palette_colours_resolve(
            frappe_palette: HashMap<String, Color>,
            #[values("text", "mantle", "base", "surface0", "mauve", "peach")] colour: &str,
            #[values("fg", "bg")] field: &str,
        ) -> Result<()> {
            let json = format!(r#"{{"{field}":"{colour}"}}"#);

            slot_to_style(&deserialize_slot(&json)?, &frappe_palette)?;

            Ok(())
        }

        // Verifies every ModifierName variant deserialises from its camelCase JSON name.
        #[rstest]
        #[case("bold", Modifier::BOLD)]
        #[case("dim", Modifier::DIM)]
        #[case("italic", Modifier::ITALIC)]
        #[case("underlined", Modifier::UNDERLINED)]
        #[case("slowBlink", Modifier::SLOW_BLINK)]
        #[case("rapidBlink", Modifier::RAPID_BLINK)]
        #[case("reversed", Modifier::REVERSED)]
        #[case("hidden", Modifier::HIDDEN)]
        #[case("crossedOut", Modifier::CROSSED_OUT)]
        fn modifier_name_round_trips(#[case] name: &str, #[case] expected: Modifier) -> Result<()> {
            let json = format!(r#"{{"modifiers":["{name}"]}}"#);
            let style = slot_to_style(&deserialize_slot(&json)?, &HashMap::new())?;

            assert!(style.add_modifier.contains(expected));

            Ok(())
        }
    }

    mod roundtrip {
        use super::*;

        fn value_roundtrip<T>(json: &str) -> Result<()>
        where
            T: for<'de> Deserialize<'de> + Serialize,
        {
            let first: T = serde_json::from_str(json)?;
            let serialized = serde_json::to_string(&first)?;
            let second: T = serde_json::from_str(&serialized)?;
            let re_serialized = serde_json::to_string(&second)?;

            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&serialized)?,
                serde_json::from_str::<serde_json::Value>(&re_serialized)?,
            );

            Ok(())
        }

        #[test]
        fn theme_slot_config_empty() -> Result<()> {
            value_roundtrip::<ThemeSlotConfig>("{}")
        }

        #[test]
        fn theme_slot_config_fg_only() -> Result<()> {
            value_roundtrip::<ThemeSlotConfig>(r#"{"fg":"text"}"#)
        }

        #[test]
        fn theme_slot_config_bg_only() -> Result<()> {
            value_roundtrip::<ThemeSlotConfig>(r#"{"bg":"mantle"}"#)
        }

        #[test]
        fn theme_slot_config_all_fields() -> Result<()> {
            value_roundtrip::<ThemeSlotConfig>(
                r#"{"fg":"text","bg":"mantle","modifiers":["bold","italic"]}"#,
            )
        }

        #[test]
        fn theme_slot_config_all_modifier_names() -> Result<()> {
            value_roundtrip::<ThemeSlotConfig>(
                r#"{"modifiers":["bold","dim","italic","underlined","slowBlink","rapidBlink","reversed","hidden","crossedOut"]}"#,
            )
        }

        #[test]
        fn theme_config_all_slots_uniform_fg() -> Result<()> {
            value_roundtrip::<ThemeConfig>(&full_theme_json("text")?)
        }
    }

    mod config_integration {
        use super::*;

        #[fixture]
        fn config_defaults() -> Result<Config> {
            let dir = tempfile::tempdir()?;

            Ok(Config::from_dirs(Some(dir.path()), None)?)
        }

        #[rstest]
        fn theme_resolved_from_embedded_config(config_defaults: Result<Config>) -> Result<()> {
            assert_eq!(
                config_defaults?.active_theme().key_label(),
                // Default is catpuccineFrappe; peach = #ef9f76 = Rgb(239, 159, 118).
                Style::from((Color::Rgb(239, 159, 118), Modifier::BOLD)),
            );

            Ok(())
        }

        #[rstest]
        #[case("catpuccineLatte")]
        #[case("catpuccineFrappe")]
        #[case("catpuccineMacchiato")]
        #[case("catpuccineMocha")]
        fn all_four_catppuccin_themes_resolved(
            config_defaults: Result<Config>,
            #[case] name: &str,
        ) -> Result<()> {
            assert!(
                config_defaults?.themes.contains_key(name),
                "missing theme '{name}'"
            );

            Ok(())
        }

        #[test]
        fn unknown_default_theme_is_rejected() -> Result<()> {
            let dir = tempfile::tempdir()?;

            std::fs::write(
                dir.path().join("config.json5"),
                r#"{ defaultTheme: "doesNotExist" }"#,
            )?;

            let result = Config::from_dirs(Some(dir.path()), None);

            assert!(matches!(
                result,
                Err(crate::config::ConfigError::UnknownTheme(ref s)) if s.contains("doesNotExist")
            ));

            Ok(())
        }

        #[test]
        fn theme_without_palette_is_rejected() -> Result<()> {
            let dir = tempfile::tempdir()?;

            let content = format!(
                r#"{{ "themes": {{ "orphanTheme": {} }} }}"#,
                full_theme_json("text")?
            );
            std::fs::write(dir.path().join("config.json5"), content)?;

            assert!(Config::from_dirs(Some(dir.path()), None).is_err());

            Ok(())
        }
    }

    mod resolve_theme {
        use super::*;

        #[test]
        fn returns_theme_for_each_entry() -> Result<()> {
            let themes: HashMap<String, ThemeConfig> =
                serde_json::from_str(&format!(r#"{{"t": {}}}"#, full_theme_json("text")?))?;
            let palettes: HashMap<String, PaletteConfig> =
                serde_json::from_str(r##"{"t": {"text": "#c6d0f5"}}"##)?;
            let resolved = resolve_theme(&themes, &palettes)?;

            assert_eq!(
                resolved["t"].body_text().fg,
                Some(Color::Rgb(198, 208, 245))
            );

            Ok(())
        }

        #[test]
        fn missing_palette_is_err() -> Result<()> {
            let themes: HashMap<String, ThemeConfig> =
                serde_json::from_str(&format!(r#"{{"t": {}}}"#, full_theme_json("text")?))?;

            assert!(resolve_theme(&themes, &HashMap::new()).is_err());

            Ok(())
        }

        #[test]
        fn unknown_colour_name_is_err() -> Result<()> {
            let themes: HashMap<String, ThemeConfig> =
                serde_json::from_str(&format!(r#"{{"t": {}}}"#, full_theme_json("nosuch")?))?;
            let palettes: HashMap<String, PaletteConfig> =
                serde_json::from_str(r##"{"t": {"text": "#c6d0f5"}}"##)?;

            assert!(resolve_theme(&themes, &palettes).is_err());

            Ok(())
        }
    }
}
