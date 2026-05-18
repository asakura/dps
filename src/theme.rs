//! Application colour theme. Assumes 24-bit ("true colour") terminal support.
//!
//! Colour values sourced from <https://github.com/catppuccin/palette> (palette.json v1.8.0).
//!
//! # Styling guide
//!
//! Every colour used in the UI is listed below. When adding new widgets, pick
//! from this table first — only introduce a new slot if none of the existing
//! ones fits the role.
//!
//! ## Surfaces (backgrounds & borders)
//!
//! Surfaces define visual depth. Darker = deeper in the hierarchy.
//!
//! | Slot | Attribute | Widget | Role |
//! |------|-----------|--------|------|
//! | `surface0` | fg (border) | table `Block` | border / frame colour; subtle, not loud |
//! | `surface0` | bg | status bar | lifts the bar off the terminal background |
//! | `lavender` | fg (title) | table `Block` | block title text colour |
//! | `base` | fg (text on selection) | selected row | text colour when `mauve` is the bg |
//!
//! `mantle` and `crust` are available for "sunken" elements (e.g. an input
//! field that sits below the main surface) but are not currently used. The
//! application background is left unset so the terminal's own colour shows
//! through — set `base` on the root widget if an explicit background is needed.
//!
//! ## Text hierarchy
//!
//! Use these slots in descending prominence. Never skip levels.
//!
//! | Slot | Attribute | Widget | Role |
//! |------|-----------|--------|------|
//! | `text` | fg + BOLD | status bar (active) | primary text; selected-gas summary |
//! | `subtext0` | fg | help line (bottom row) | keyboard shortcuts and hints |
//! | `overlay0` | fg | status bar (empty state) | dimmed secondary / placeholder text |
//!
//! `subtext1` is the next step down from `text` — use it for secondary
//! metadata alongside a primary value (e.g. a unit label next to a number).
//! `overlay1`/`overlay2` sit between `overlay0` and `surface2` and suit
//! deeply-dimmed decorative elements. None of these are currently used.
//!
//! ## Accents
//!
//! ### Selection
//!
//! The same **`mauve` bg + `base` fg** rule applies to every "active" element
//! (the Catppuccin Selection Rule): a soft accent background with the deepest
//! base as foreground for maximum legibility without harshness.
//!
//! | Slot | Attribute | Widget | Role |
//! |------|-----------|--------|------|
//! | `mauve` | bg + BOLD | active tab | active tab indicator in the tab bar |
//! | `mauve` | bg + BOLD | selected table row | active row highlight |
//! | `base` | fg | active tab + selected row | text on any mauve background |
//!
//! `sapphire` is a recommended alternative if `mauve` feels too purple for a
//! particular context.
//!
//! ### Navigation chrome
//!
//! | Slot | Attribute | Widget | Role |
//! |------|-----------|--------|------|
//! | `surface0` | bg | tab bar | background shared with inactive tabs and status bar |
//! | `subtext0` | fg | inactive tabs | muted tab labels; same slot as the help line |
//! | `blue` | fg | table header row | column labels; info-level chrome |
//! | `lavender` | fg + BOLD | focused column cells | softer accent than `mauve`; same hue family |
//!
//! ### Data-cell safety levels
//!
//! Both tables colour computed values by dive-safety level. Thresholds differ
//! per table but the colour mapping is shared:
//!
//! | Slot | Level | MOD condition | ppO₂ condition |
//! |------|-------|---------------|----------------|
//! | `green` | safe | depth ≥ 20 m | ppO₂ ∈ \[0.18, 1.4) |
//! | `yellow` | caution | depth ∈ \[10, 20) m | ppO₂ ∈ \[1.4, 1.6) |
//! | `red` | danger | depth < 10 m | ppO₂ < 0.18 or ≥ 1.6 |
//!
//! **Unassigned neutrals:** `surface1` (between `surface0` and `surface2`;
//! natural fit for a focused-but-not-active panel border or a secondary
//! status row), `surface2`, `subtext1`, `overlay1`, `overlay2`, `mantle`,
//! `crust`.
//!
//! **Unassigned accents:** `rosewater`, `flamingo`, `pink`, `maroon`, `peach`,
//! `teal`, `sky`, `sapphire`. Candidates: `peach` for secondary highlights,
//! `maroon` for soft-error states distinct from hard `red`, `sapphire` as an
//! alternative selection colour to `mauve`.

use ratatui::style::{Color, Modifier, Style};

/// Application colour theme.
///
/// Each semantic slot stores the resolved [`Color`]s directly, so that config-
/// driven themes (where slot → palette-colour-name mappings are read from the
/// config file) and the hard-coded Catppuccin constructors share the same type.
///
/// Fields are private; use [`Theme::default`] for the built-in Frappé theme,
/// or obtain the active config-driven theme via
/// [`crate::config::Config::active_theme`]. Config code constructs themes via
/// [`Theme::new`] in `crate::config::theme`.
///
/// All colours are accessed through semantic style methods (`danger()`,
/// `safe()`, `selection()`, …).
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    // Which-key popup
    popup_surface_fg: Color,
    popup_surface_bg: Color,
    key_label_fg: Color,
    // Table chrome
    border_fg: Color,
    title_fg: Color,
    header_fg: Color,
    selection_fg: Color,
    selection_bg: Color,
    column_focus_fg: Color,
    // Tab bar
    nav_bar_fg: Color,
    nav_bar_bg: Color,
    // Status bar
    status_active_fg: Color,
    status_active_bg: Color,
    status_empty_fg: Color,
    status_empty_bg: Color,
    // Safety levels
    safe_fg: Color,
    caution_fg: Color,
    danger_fg: Color,
    // Text
    body_text_fg: Color,
    hint_fg: Color,
}

impl Theme {
    // Which-key popup
    /// Popup surface: text-on-mantle background.
    #[must_use]
    pub fn popup_surface(&self) -> Style {
        Style::from((self.popup_surface_fg, self.popup_surface_bg))
    }

    /// Key label: bold peach.
    #[must_use]
    pub fn key_label(&self) -> Style {
        Style::from((self.key_label_fg, Modifier::BOLD))
    }

    // Table chrome
    /// Table border.
    #[must_use]
    pub fn border(&self) -> Style {
        Style::from(self.border_fg)
    }

    /// Block title above a table.
    #[must_use]
    pub fn title(&self) -> Style {
        Style::from(self.title_fg)
    }

    /// Header row text.
    #[must_use]
    pub fn header(&self) -> Style {
        Style::from(self.header_fg)
    }

    /// Non-highlighted header cell.
    #[must_use]
    pub fn header_cell() -> Style {
        Style::from(Modifier::BOLD)
    }

    /// Highlighted (selected column) header cell.
    #[must_use]
    pub fn header_cell_active() -> Style {
        Style::from(Modifier::BOLD | Modifier::UNDERLINED)
    }

    /// Selected row / active element (Catppuccin Selection Rule: mauve bg + base fg).
    #[must_use]
    pub fn selection(&self) -> Style {
        Style::from((self.selection_fg, self.selection_bg, Modifier::BOLD))
    }

    /// Focused column highlight.
    #[must_use]
    pub fn column_focus(&self) -> Style {
        Style::from((self.column_focus_fg, Modifier::BOLD))
    }

    // Tab bar
    /// Inactive tab bar background.
    #[must_use]
    pub fn nav_bar(&self) -> Style {
        Style::from((self.nav_bar_fg, self.nav_bar_bg))
    }

    // Status bar
    /// Status bar with an active selection.
    #[must_use]
    pub fn status_active(&self) -> Style {
        Style::from((self.status_active_fg, self.status_active_bg, Modifier::BOLD))
    }

    /// Status bar empty / placeholder state.
    #[must_use]
    pub fn status_empty(&self) -> Style {
        Style::from((self.status_empty_fg, self.status_empty_bg))
    }

    // Safety levels
    /// Safe dive condition (green).
    #[must_use]
    pub fn safe(&self) -> Style {
        Style::from(self.safe_fg)
    }

    /// Caution dive condition (yellow).
    #[must_use]
    pub fn caution(&self) -> Style {
        Style::from(self.caution_fg)
    }

    /// Danger dive condition (red).
    #[must_use]
    pub fn danger(&self) -> Style {
        Style::from(self.danger_fg)
    }

    // Text
    /// Body text: plain text fg.
    #[must_use]
    pub fn body_text(&self) -> Style {
        Style::from(self.body_text_fg)
    }

    /// Hint / help line text.
    #[must_use]
    pub fn hint(&self) -> Style {
        Style::from(self.hint_fg)
    }

    /// Constructs a theme from pre-resolved colours.
    ///
    /// This is the single construction path for config-driven themes; call sites
    /// live in [`crate::config::theme`]. Hard-coded themes use [`Theme::default`].
    #[expect(
        clippy::too_many_arguments,
        reason = "one argument per semantic colour slot"
    )]
    #[must_use]
    pub(crate) const fn new(
        popup_surface_fg: Color,
        popup_surface_bg: Color,
        key_label_fg: Color,
        border_fg: Color,
        title_fg: Color,
        header_fg: Color,
        selection_fg: Color,
        selection_bg: Color,
        column_focus_fg: Color,
        nav_bar_fg: Color,
        nav_bar_bg: Color,
        status_active_fg: Color,
        status_active_bg: Color,
        status_empty_fg: Color,
        status_empty_bg: Color,
        safe_fg: Color,
        caution_fg: Color,
        danger_fg: Color,
        body_text_fg: Color,
        hint_fg: Color,
    ) -> Self {
        Self {
            popup_surface_fg,
            popup_surface_bg,
            key_label_fg,
            border_fg,
            title_fg,
            header_fg,
            selection_fg,
            selection_bg,
            column_focus_fg,
            nav_bar_fg,
            nav_bar_bg,
            status_active_fg,
            status_active_bg,
            status_empty_fg,
            status_empty_bg,
            safe_fg,
            caution_fg,
            danger_fg,
            body_text_fg,
            hint_fg,
        }
    }
}

impl Default for Theme {
    /// Catppuccin Frappé — cool dark flavour.
    fn default() -> Self {
        Self {
            popup_surface_fg: Color::Rgb(198, 208, 245), // text
            popup_surface_bg: Color::Rgb(41, 44, 60),    // mantle
            key_label_fg: Color::Rgb(239, 159, 118),     // peach
            border_fg: Color::Rgb(65, 69, 89),           // surface0
            title_fg: Color::Rgb(186, 187, 241),         // lavender
            header_fg: Color::Rgb(140, 170, 238),        // blue
            selection_fg: Color::Rgb(48, 52, 70),        // base
            selection_bg: Color::Rgb(202, 158, 230),     // mauve
            column_focus_fg: Color::Rgb(186, 187, 241),  // lavender
            nav_bar_fg: Color::Rgb(165, 173, 206),       // subtext0
            nav_bar_bg: Color::Rgb(65, 69, 89),          // surface0
            status_active_fg: Color::Rgb(198, 208, 245), // text
            status_active_bg: Color::Rgb(65, 69, 89),    // surface0
            status_empty_fg: Color::Rgb(115, 121, 148),  // overlay0
            status_empty_bg: Color::Rgb(65, 69, 89),     // surface0
            safe_fg: Color::Rgb(166, 209, 137),          // green
            caution_fg: Color::Rgb(229, 200, 144),       // yellow
            danger_fg: Color::Rgb(231, 130, 132),        // red
            body_text_fg: Color::Rgb(198, 208, 245),     // text
            hint_fg: Color::Rgb(165, 173, 206),          // subtext0
        }
    }
}
