//! Application colour theme. Assumes 24-bit ("true colour") terminal support.
//!
//! Palette data from <https://github.com/catppuccin/palette> (palette.json v1.8.0).
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

/// Full Catppuccin palette with all 26 named colour slots.
///
/// Field names follow the canonical Catppuccin naming exactly, so callers can
/// express intent directly (e.g. `THEME.red` for danger, `THEME.green` for safe).
///
/// The first 14 fields (`rosewater` → `lavender`) are accent colours (`"accent": true`
/// in palette.json). The remaining 12 (`text` → `crust`) are non-accent neutrals.
#[derive(Debug)]
pub struct Theme {
    // ── Accents (accent: true) ───────────────────────────────────────────────
    /// Catppuccin rosewater accent colour.
    pub rosewater: Color,
    /// Catppuccin flamingo accent colour.
    pub flamingo: Color,
    /// Catppuccin pink accent colour.
    pub pink: Color,
    /// Catppuccin mauve accent colour.
    pub mauve: Color,
    /// Catppuccin red accent colour.
    pub red: Color,
    /// Catppuccin maroon accent colour.
    pub maroon: Color,
    /// Catppuccin peach accent colour.
    pub peach: Color,
    /// Catppuccin yellow accent colour.
    pub yellow: Color,
    /// Catppuccin green accent colour.
    pub green: Color,
    /// Catppuccin teal accent colour.
    pub teal: Color,
    /// Catppuccin sky accent colour.
    pub sky: Color,
    /// Catppuccin sapphire accent colour.
    pub sapphire: Color,
    /// Catppuccin blue accent colour.
    pub blue: Color,
    /// Catppuccin lavender accent colour.
    pub lavender: Color,
    // ── Text ────────────────────────────────────────────────────────────────
    /// Primary text colour.
    pub text: Color,
    /// Secondary text colour (slightly dimmed).
    pub subtext1: Color,
    /// Tertiary text colour (more dimmed).
    pub subtext0: Color,
    // ── Overlay ─────────────────────────────────────────────────────────────
    /// Overlay colour level 2 (brightest).
    pub overlay2: Color,
    /// Overlay colour level 1.
    pub overlay1: Color,
    /// Overlay colour level 0 (darkest).
    pub overlay0: Color,
    // ── Surface ─────────────────────────────────────────────────────────────
    /// Surface colour level 2 (lightest surface).
    pub surface2: Color,
    /// Surface colour level 1.
    pub surface1: Color,
    /// Surface colour level 0 (darkest surface).
    pub surface0: Color,
    // ── Base ────────────────────────────────────────────────────────────────
    /// Base background colour.
    pub base: Color,
    /// Darker background layer (beneath base).
    pub mantle: Color,
    /// Darkest background layer (beneath mantle).
    pub crust: Color,
}

impl Theme {
    /// Catppuccin Latte — light flavour.
    #[must_use]
    pub const fn latte() -> Self {
        Self {
            rosewater: Color::Rgb(220, 138, 120), // #dc8a78
            flamingo: Color::Rgb(221, 120, 120),  // #dd7878
            pink: Color::Rgb(234, 118, 203),      // #ea76cb
            mauve: Color::Rgb(136, 57, 239),      // #8839ef
            red: Color::Rgb(210, 15, 57),         // #d20f39
            maroon: Color::Rgb(230, 69, 83),      // #e64553
            peach: Color::Rgb(254, 100, 11),      // #fe640b
            yellow: Color::Rgb(223, 142, 29),     // #df8e1d
            green: Color::Rgb(64, 160, 43),       // #40a02b
            teal: Color::Rgb(23, 146, 153),       // #179299
            sky: Color::Rgb(4, 165, 229),         // #04a5e5
            sapphire: Color::Rgb(32, 159, 181),   // #209fb5
            blue: Color::Rgb(30, 102, 245),       // #1e66f5
            lavender: Color::Rgb(114, 135, 253),  // #7287fd
            text: Color::Rgb(76, 79, 105),        // #4c4f69
            subtext1: Color::Rgb(92, 95, 119),    // #5c5f77
            subtext0: Color::Rgb(108, 111, 133),  // #6c6f85
            overlay2: Color::Rgb(124, 127, 147),  // #7c7f93
            overlay1: Color::Rgb(140, 143, 161),  // #8c8fa1
            overlay0: Color::Rgb(156, 160, 176),  // #9ca0b0
            surface2: Color::Rgb(172, 176, 190),  // #acb0be
            surface1: Color::Rgb(188, 192, 204),  // #bcc0cc
            surface0: Color::Rgb(204, 208, 218),  // #ccd0da
            base: Color::Rgb(239, 241, 245),      // #eff1f5
            mantle: Color::Rgb(230, 233, 239),    // #e6e9ef
            crust: Color::Rgb(220, 224, 232),     // #dce0e8
        }
    }

    /// Catppuccin Frappé — cool dark flavour.
    #[must_use]
    pub const fn frappe() -> Self {
        Self {
            rosewater: Color::Rgb(242, 213, 207), // #f2d5cf
            flamingo: Color::Rgb(238, 190, 190),  // #eebebe
            pink: Color::Rgb(244, 184, 228),      // #f4b8e4
            mauve: Color::Rgb(202, 158, 230),     // #ca9ee6
            red: Color::Rgb(231, 130, 132),       // #e78284
            maroon: Color::Rgb(234, 153, 156),    // #ea999c
            peach: Color::Rgb(239, 159, 118),     // #ef9f76
            yellow: Color::Rgb(229, 200, 144),    // #e5c890
            green: Color::Rgb(166, 209, 137),     // #a6d189
            teal: Color::Rgb(129, 200, 190),      // #81c8be
            sky: Color::Rgb(153, 209, 219),       // #99d1db
            sapphire: Color::Rgb(133, 193, 220),  // #85c1dc
            blue: Color::Rgb(140, 170, 238),      // #8caaee
            lavender: Color::Rgb(186, 187, 241),  // #babbf1
            text: Color::Rgb(198, 208, 245),      // #c6d0f5
            subtext1: Color::Rgb(181, 191, 226),  // #b5bfe2
            subtext0: Color::Rgb(165, 173, 206),  // #a5adce
            overlay2: Color::Rgb(148, 156, 187),  // #949cbb
            overlay1: Color::Rgb(131, 139, 167),  // #838ba7
            overlay0: Color::Rgb(115, 121, 148),  // #737994
            surface2: Color::Rgb(98, 104, 128),   // #626880
            surface1: Color::Rgb(81, 87, 109),    // #51576d
            surface0: Color::Rgb(65, 69, 89),     // #414559
            base: Color::Rgb(48, 52, 70),         // #303446
            mantle: Color::Rgb(41, 44, 60),       // #292c3c
            crust: Color::Rgb(35, 38, 52),        // #232634
        }
    }

    /// Catppuccin Macchiato — medium dark flavour.
    #[must_use]
    pub const fn macchiato() -> Self {
        Self {
            rosewater: Color::Rgb(244, 219, 214), // #f4dbd6
            flamingo: Color::Rgb(240, 198, 198),  // #f0c6c6
            pink: Color::Rgb(245, 189, 230),      // #f5bde6
            mauve: Color::Rgb(198, 160, 246),     // #c6a0f6
            red: Color::Rgb(237, 135, 150),       // #ed8796
            maroon: Color::Rgb(238, 153, 160),    // #ee99a0
            peach: Color::Rgb(245, 169, 127),     // #f5a97f
            yellow: Color::Rgb(238, 212, 159),    // #eed49f
            green: Color::Rgb(166, 218, 149),     // #a6da95
            teal: Color::Rgb(139, 213, 202),      // #8bd5ca
            sky: Color::Rgb(145, 215, 227),       // #91d7e3
            sapphire: Color::Rgb(125, 196, 228),  // #7dc4e4
            blue: Color::Rgb(138, 173, 244),      // #8aadf4
            lavender: Color::Rgb(183, 189, 248),  // #b7bdf8
            text: Color::Rgb(202, 211, 245),      // #cad3f5
            subtext1: Color::Rgb(184, 192, 224),  // #b8c0e0
            subtext0: Color::Rgb(165, 173, 203),  // #a5adcb
            overlay2: Color::Rgb(147, 154, 183),  // #939ab7
            overlay1: Color::Rgb(128, 135, 162),  // #8087a2
            overlay0: Color::Rgb(110, 115, 141),  // #6e738d
            surface2: Color::Rgb(91, 96, 120),    // #5b6078
            surface1: Color::Rgb(73, 77, 100),    // #494d64
            surface0: Color::Rgb(54, 58, 79),     // #363a4f
            base: Color::Rgb(36, 39, 58),         // #24273a
            mantle: Color::Rgb(30, 32, 48),       // #1e2030
            crust: Color::Rgb(24, 25, 38),        // #181926
        }
    }

    /// Catppuccin Mocha — darkest flavour.
    #[must_use]
    pub const fn mocha() -> Self {
        Self {
            rosewater: Color::Rgb(245, 224, 220), // #f5e0dc
            flamingo: Color::Rgb(242, 205, 205),  // #f2cdcd
            pink: Color::Rgb(245, 194, 231),      // #f5c2e7
            mauve: Color::Rgb(203, 166, 247),     // #cba6f7
            red: Color::Rgb(243, 139, 168),       // #f38ba8
            maroon: Color::Rgb(235, 160, 172),    // #eba0ac
            peach: Color::Rgb(250, 179, 135),     // #fab387
            yellow: Color::Rgb(249, 226, 175),    // #f9e2af
            green: Color::Rgb(166, 227, 161),     // #a6e3a1
            teal: Color::Rgb(148, 226, 213),      // #94e2d5
            sky: Color::Rgb(137, 220, 235),       // #89dceb
            sapphire: Color::Rgb(116, 199, 236),  // #74c7ec
            blue: Color::Rgb(137, 180, 250),      // #89b4fa
            lavender: Color::Rgb(180, 190, 254),  // #b4befe
            text: Color::Rgb(205, 214, 244),      // #cdd6f4
            subtext1: Color::Rgb(186, 194, 222),  // #bac2de
            subtext0: Color::Rgb(166, 173, 200),  // #a6adc8
            overlay2: Color::Rgb(147, 153, 178),  // #9399b2
            overlay1: Color::Rgb(127, 132, 156),  // #7f849c
            overlay0: Color::Rgb(108, 112, 134),  // #6c7086
            surface2: Color::Rgb(88, 91, 112),    // #585b70
            surface1: Color::Rgb(69, 71, 90),     // #45475a
            surface0: Color::Rgb(49, 50, 68),     // #313244
            base: Color::Rgb(30, 30, 46),         // #1e1e2e
            mantle: Color::Rgb(24, 24, 37),       // #181825
            crust: Color::Rgb(17, 17, 27),        // #11111b
        }
    }

    // ── Which-key popup ──────────────────────────────────────────────────────
    /// Popup surface: text-on-mantle background.
    #[must_use]
    pub fn popup_surface(&self) -> Style {
        Style::from((self.text, self.mantle))
    }
    /// Key label: bold peach.
    #[must_use]
    pub fn key_label(&self) -> Style {
        Style::from((self.peach, Modifier::BOLD))
    }

    // ── Table chrome ─────────────────────────────────────────────────────────
    /// Table border.
    #[must_use]
    pub fn border(&self) -> Style {
        Style::from(self.surface0)
    }
    /// Block title above a table.
    #[must_use]
    pub fn title(&self) -> Style {
        Style::from(self.lavender)
    }
    /// Header row text.
    #[must_use]
    pub fn header(&self) -> Style {
        Style::from(self.blue)
    }
    /// Non-highlighted header cell.
    #[must_use]
    pub fn header_cell(&self) -> Style {
        Style::from(Modifier::BOLD)
    }
    /// Highlighted (selected column) header cell.
    #[must_use]
    pub fn header_cell_active(&self) -> Style {
        Style::from(Modifier::BOLD | Modifier::UNDERLINED)
    }
    /// Selected row / active element (Catppuccin Selection Rule: mauve bg + base fg).
    #[must_use]
    pub fn selection(&self) -> Style {
        Style::from((self.base, self.mauve, Modifier::BOLD))
    }
    /// Focused column highlight.
    #[must_use]
    pub fn column_focus(&self) -> Style {
        Style::from((self.lavender, Modifier::BOLD))
    }

    // ── Tab bar ───────────────────────────────────────────────────────────────
    /// Inactive tab bar background.
    #[must_use]
    pub fn nav_bar(&self) -> Style {
        Style::from((self.subtext0, self.surface0))
    }

    // ── Status bar ────────────────────────────────────────────────────────────
    /// Status bar with an active selection.
    #[must_use]
    pub fn status_active(&self) -> Style {
        Style::from((self.text, self.surface0, Modifier::BOLD))
    }
    /// Status bar empty / placeholder state.
    #[must_use]
    pub fn status_empty(&self) -> Style {
        Style::from((self.overlay0, self.surface0))
    }

    // ── Text ──────────────────────────────────────────────────────────────────
    /// Body text: plain text fg.
    #[must_use]
    pub fn body_text(&self) -> Style {
        Style::from(self.text)
    }
    /// Hint / help line text.
    #[must_use]
    pub fn hint(&self) -> Style {
        Style::from(self.subtext0)
    }

    /// The 14 accent colours in canonical order (rosewater → lavender).
    ///
    /// ```
    /// use dps::theme::THEME;
    /// let accents = THEME.accents();
    /// assert_eq!(accents.len(), 14);
    /// assert_eq!(accents[0], THEME.rosewater);
    /// assert_eq!(accents[13], THEME.lavender);
    /// ```
    #[must_use]
    pub fn accents(&self) -> [Color; 14] {
        [
            self.rosewater,
            self.flamingo,
            self.pink,
            self.mauve,
            self.red,
            self.maroon,
            self.peach,
            self.yellow,
            self.green,
            self.teal,
            self.sky,
            self.sapphire,
            self.blue,
            self.lavender,
        ]
    }
}

/// The active theme — shared across all rendering code.
pub static THEME: Theme = Theme::frappe();
