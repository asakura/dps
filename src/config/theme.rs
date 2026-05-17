//! Theme and palette configuration types, and the [`resolve_theme`] resolver.

use std::collections::HashMap;

use color_eyre::eyre::eyre;
use ratatui::style::Color;
use serde::Deserialize;

use crate::theme::Theme;

/// Foreground (and optional background) colour name for one semantic theme slot.
#[derive(Clone, Debug, Deserialize)]
pub(super) struct ThemeSlotConfig {
    fg: String,
    bg: Option<String>,
}

/// One named theme entry: maps every semantic slot to palette colour names.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ThemeConfig {
    popup_surface: ThemeSlotConfig,
    key_label: ThemeSlotConfig,
    border: ThemeSlotConfig,
    title: ThemeSlotConfig,
    header: ThemeSlotConfig,
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
) -> color_eyre::Result<HashMap<String, Theme>> {
    themes
        .iter()
        .map(|(name, tc)| {
            let pc = palettes
                .get(name)
                .ok_or_else(|| eyre!("theme '{name}' has no matching palette"))?;
            Ok((name.clone(), resolve_single(tc, pc)?))
        })
        .collect()
}

fn resolve_single(tc: &ThemeConfig, pc: &PaletteConfig) -> color_eyre::Result<Theme> {
    let pal = parse_palette(pc)?;

    let fg = |slot: &ThemeSlotConfig| -> color_eyre::Result<Color> {
        pal.get(slot.fg.as_str())
            .copied()
            .ok_or_else(|| eyre!("unknown palette colour '{}'", slot.fg))
    };
    let bg = |slot: &ThemeSlotConfig| -> color_eyre::Result<Color> {
        let name = slot
            .bg
            .as_deref()
            .ok_or_else(|| eyre!("slot '{}' has no bg colour", slot.fg))?;
        pal.get(name)
            .copied()
            .ok_or_else(|| eyre!("unknown palette colour '{name}'"))
    };

    Ok(Theme::new(
        fg(&tc.popup_surface)?,
        bg(&tc.popup_surface)?,
        fg(&tc.key_label)?,
        fg(&tc.border)?,
        fg(&tc.title)?,
        fg(&tc.header)?,
        fg(&tc.selection)?,
        bg(&tc.selection)?,
        fg(&tc.column_focus)?,
        fg(&tc.nav_bar)?,
        bg(&tc.nav_bar)?,
        fg(&tc.status_active)?,
        bg(&tc.status_active)?,
        fg(&tc.status_empty)?,
        bg(&tc.status_empty)?,
        fg(&tc.safe)?,
        fg(&tc.caution)?,
        fg(&tc.danger)?,
        fg(&tc.body_text)?,
        fg(&tc.hint)?,
    ))
}

fn parse_palette(cfg: &HashMap<String, String>) -> color_eyre::Result<HashMap<String, Color>> {
    cfg.iter()
        .map(|(name, hex)| Ok((name.clone(), parse_hex(hex)?)))
        .collect()
}

fn parse_hex(s: &str) -> color_eyre::Result<Color> {
    let hex = s
        .strip_prefix('#')
        .ok_or_else(|| eyre!("expected '#' prefix in colour '{s}'"))?;

    if hex.len() != 6 {
        return Err(eyre!("expected 6 hex digits in '{s}', got {}", hex.len()));
    }

    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| eyre!("bad hex in '{s}': {e}"))?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| eyre!("bad hex in '{s}': {e}"))?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| eyre!("bad hex in '{s}': {e}"))?;

    Ok(Color::Rgb(r, g, b))
}
