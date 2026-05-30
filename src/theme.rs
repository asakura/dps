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
/// `Theme::new` in `crate::config::theme`.
///
/// All colours are accessed through semantic style methods (`danger()`,
/// `safe()`, `selection()`, …).
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    // Which-key popup
    popup_surface: Style,
    key_label: Style,

    // Table chrome
    border: Style,
    title: Style,
    header: Style,
    header_cell: Style,
    header_cell_active: Style,
    selection: Style,
    column_focus: Style,

    // Tab bar
    nav_bar: Style,

    // Status bar
    status_active: Style,
    status_empty: Style,

    // Safety levels
    safe: Style,
    caution: Style,
    danger: Style,

    // Text
    body_text: Style,
    hint: Style,
}

impl Theme {
    // Which-key popup
    /// Popup surface: text-on-mantle background.
    #[must_use]
    pub const fn popup_surface(self) -> Style {
        self.popup_surface
    }

    /// Key label: bold peach.
    #[must_use]
    pub const fn key_label(self) -> Style {
        self.key_label
    }

    // Table chrome
    /// Table border.
    #[must_use]
    pub const fn border(self) -> Style {
        self.border
    }

    /// Block title above a table.
    #[must_use]
    pub const fn title(self) -> Style {
        self.title
    }

    /// Header row text.
    #[must_use]
    pub const fn header(self) -> Style {
        self.header
    }

    /// Non-highlighted header cell.
    #[must_use]
    pub const fn header_cell(self) -> Style {
        self.header_cell
    }

    /// Highlighted (selected column) header cell.
    #[must_use]
    pub const fn header_cell_active(self) -> Style {
        self.header_cell_active
    }

    /// Selected row / active element (Catppuccin Selection Rule: mauve bg + base fg).
    #[must_use]
    pub const fn selection(self) -> Style {
        self.selection
    }

    /// Focused column highlight.
    #[must_use]
    pub const fn column_focus(self) -> Style {
        self.column_focus
    }

    // Tab bar
    /// Inactive tab bar background.
    #[must_use]
    pub const fn nav_bar(self) -> Style {
        self.nav_bar
    }

    // Status bar
    /// Status bar with an active selection.
    #[must_use]
    pub const fn status_active(self) -> Style {
        self.status_active
    }

    /// Status bar empty / placeholder state.
    #[must_use]
    pub const fn status_empty(self) -> Style {
        self.status_empty
    }

    // Safety levels
    /// Safe dive condition (green).
    #[must_use]
    pub const fn safe(self) -> Style {
        self.safe
    }

    /// Caution dive condition (yellow).
    #[must_use]
    pub const fn caution(self) -> Style {
        self.caution
    }

    /// Danger dive condition (red).
    #[must_use]
    pub const fn danger(self) -> Style {
        self.danger
    }

    // Text
    /// Body text: plain text fg.
    #[must_use]
    pub const fn body_text(self) -> Style {
        self.body_text
    }

    /// Hint / help line text.
    #[must_use]
    pub const fn hint(self) -> Style {
        self.hint
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
        popup_surface: Style,
        key_label: Style,
        border: Style,
        title: Style,
        header: Style,
        header_cell: Style,
        header_cell_active: Style,
        selection: Style,
        column_focus: Style,
        nav_bar: Style,
        status_active: Style,
        status_empty: Style,
        safe: Style,
        caution: Style,
        danger: Style,
        body_text: Style,
        hint: Style,
    ) -> Self {
        Self {
            popup_surface,
            key_label,
            border,
            title,
            header,
            header_cell,
            header_cell_active,
            selection,
            column_focus,
            nav_bar,
            status_active,
            status_empty,
            safe,
            caution,
            danger,
            body_text,
            hint,
        }
    }
}

impl Default for Theme {
    /// Catppuccin Frappé — cool dark flavour.
    fn default() -> Self {
        // Catppuccin Frappé Palette Colors
        let text = Color::Rgb(198, 208, 245);
        let mantle = Color::Rgb(41, 44, 60);
        let base = Color::Rgb(48, 52, 70);
        let surface0 = Color::Rgb(65, 69, 89);
        let overlay0 = Color::Rgb(115, 121, 148);
        let subtext0 = Color::Rgb(165, 173, 206);
        let lavender = Color::Rgb(186, 187, 241);
        let mauve = Color::Rgb(202, 158, 230);
        let blue = Color::Rgb(140, 170, 238);
        let peach = Color::Rgb(239, 159, 118);
        let green = Color::Rgb(166, 209, 137);
        let yellow = Color::Rgb(229, 200, 144);
        let red = Color::Rgb(231, 130, 132);

        Self {
            popup_surface: Style::from((text, mantle)),
            key_label: Style::from((peach, Modifier::BOLD)),
            border: Style::from(surface0),
            title: Style::from(lavender),
            header: Style::from(blue),
            header_cell: Style::from(Modifier::BOLD),
            header_cell_active: Style::from(Modifier::BOLD | Modifier::UNDERLINED),
            selection: Style::from((base, mauve, Modifier::BOLD)),
            column_focus: Style::from((lavender, Modifier::BOLD)),
            nav_bar: Style::from((subtext0, surface0)),
            status_active: Style::from((text, surface0, Modifier::BOLD)),
            status_empty: Style::from((overlay0, surface0)),
            safe: Style::from(green),
            caution: Style::from(yellow),
            danger: Style::from(red),
            body_text: Style::from(text),
            hint: Style::from(subtext0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ratatui::style::{Color, Modifier};

    #[test]
    fn header_cell_has_bold_modifier() {
        assert!(
            Theme::default()
                .header_cell()
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn header_cell_active_has_bold_and_underlined() {
        let style = Theme::default().header_cell_active();

        assert!(style.add_modifier.contains(Modifier::BOLD));
        assert!(style.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn header_cell_active_differs_from_plain_header_cell() {
        // BOLD | UNDERLINED combined must differ from BOLD alone
        assert_ne!(
            Theme::default().header_cell_active().add_modifier,
            Theme::default().header_cell().add_modifier
        );
    }

    mod style_accessors {
        use super::*;

        // Frappé palette constants mirrored from Default; keep in sync if colours change.
        const TEXT: Color = Color::Rgb(198, 208, 245);
        const MANTLE: Color = Color::Rgb(41, 44, 60);
        const BASE: Color = Color::Rgb(48, 52, 70);
        const SURFACE0: Color = Color::Rgb(65, 69, 89);
        const OVERLAY0: Color = Color::Rgb(115, 121, 148);
        const SUBTEXT0: Color = Color::Rgb(165, 173, 206);
        const LAVENDER: Color = Color::Rgb(186, 187, 241);
        const MAUVE: Color = Color::Rgb(202, 158, 230);
        const BLUE: Color = Color::Rgb(140, 170, 238);
        const PEACH: Color = Color::Rgb(239, 159, 118);
        const GREEN: Color = Color::Rgb(166, 209, 137);
        const YELLOW: Color = Color::Rgb(229, 200, 144);
        const RED: Color = Color::Rgb(231, 130, 132);

        #[test]
        fn popup_surface_fg_is_text_bg_is_mantle() {
            let s = Theme::default().popup_surface();
            assert_eq!(s.fg, Some(TEXT));
            assert_eq!(s.bg, Some(MANTLE));
        }

        #[test]
        fn key_label_fg_is_peach_and_bold() {
            let s = Theme::default().key_label();
            assert_eq!(s.fg, Some(PEACH));
            assert!(s.add_modifier.contains(Modifier::BOLD));
        }

        #[test]
        fn border_fg_is_surface0() {
            assert_eq!(Theme::default().border().fg, Some(SURFACE0));
        }

        #[test]
        fn title_fg_is_lavender() {
            assert_eq!(Theme::default().title().fg, Some(LAVENDER));
        }

        #[test]
        fn header_fg_is_blue() {
            assert_eq!(Theme::default().header().fg, Some(BLUE));
        }

        #[test]
        fn selection_fg_is_base_bg_is_mauve_and_bold() {
            let s = Theme::default().selection();
            assert_eq!(s.fg, Some(BASE));
            assert_eq!(s.bg, Some(MAUVE));
            assert!(s.add_modifier.contains(Modifier::BOLD));
        }

        #[test]
        fn column_focus_fg_is_lavender_and_bold() {
            let s = Theme::default().column_focus();
            assert_eq!(s.fg, Some(LAVENDER));
            assert!(s.add_modifier.contains(Modifier::BOLD));
        }

        #[test]
        fn nav_bar_fg_is_subtext0_bg_is_surface0() {
            let s = Theme::default().nav_bar();
            assert_eq!(s.fg, Some(SUBTEXT0));
            assert_eq!(s.bg, Some(SURFACE0));
        }

        #[test]
        fn status_active_fg_is_text_bg_is_surface0_and_bold() {
            let s = Theme::default().status_active();
            assert_eq!(s.fg, Some(TEXT));
            assert_eq!(s.bg, Some(SURFACE0));
            assert!(s.add_modifier.contains(Modifier::BOLD));
        }

        #[test]
        fn status_empty_fg_is_overlay0_bg_is_surface0() {
            let s = Theme::default().status_empty();
            assert_eq!(s.fg, Some(OVERLAY0));
            assert_eq!(s.bg, Some(SURFACE0));
        }

        #[test]
        fn safe_fg_is_green() {
            assert_eq!(Theme::default().safe().fg, Some(GREEN));
        }

        #[test]
        fn caution_fg_is_yellow() {
            assert_eq!(Theme::default().caution().fg, Some(YELLOW));
        }

        #[test]
        fn danger_fg_is_red() {
            assert_eq!(Theme::default().danger().fg, Some(RED));
        }

        #[test]
        fn body_text_fg_is_text() {
            assert_eq!(Theme::default().body_text().fg, Some(TEXT));
        }

        #[test]
        fn hint_fg_is_subtext0() {
            assert_eq!(Theme::default().hint().fg, Some(SUBTEXT0));
        }
    }
}
