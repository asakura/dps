//! UI action types dispatched through the event loop.
//!
//! Domain types — each owns a payload enum and serialises as `Variant(payload)`:
//!
//! - [`Movement`] — cursor / scroll navigation (`Up`, `Down`, `GotoBottom`, …).
//! - [`EditOp`] — yank / paste / delete operations (`YankRow`, `Paste`, …).
//! - [`TabMotion`] — tab-bar navigation (`Next`, `Prev`, `GoTo(n)`).
//! - [`PromptOp`] — modal prompt responses (`Confirm`, `Cancel`).
//! - [`UiOp`] — display-layer controls (`Help`).
//!
//! [`Action`] is the unified envelope dispatched through the event loop.
//! Infrastructure variants (`Tick`, `Render`, `Resize`, …) are handled directly
//! by `App::run`; domain variants are forwarded to every component's
//! [`Component::update`](crate::components::Component::update).
//! Serialises as a flat string for key-binding configuration files:
//! `"Quit"`, `"Move(Down)"`, `"Resize(80,24)"`, `"Error(oops)"`, …
//!
//! ```
//! use std::str::FromStr;
//! use dps::action::{Action, Movement};
//!
//! assert_eq!(Action::Quit.to_string(), "Quit");
//! assert_eq!(
//!     Action::from_str("Move(Down)").unwrap(),
//!     Action::Move(Movement::Down),
//! );
//! ```

mod edit;
mod error;
mod movement;
mod prompt;
mod tab;
mod ui;

pub use self::edit::EditOp;
pub use self::error::Error as ActionError;
use self::error::ParseError;
pub use self::movement::Movement;
pub use self::prompt::PromptOp;
pub use self::tab::TabMotion;
pub use self::ui::UiOp;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use strum::VariantNames;

use std::{fmt, str::FromStr};

/// The unified message type flowing through the application event loop.
///
/// `Action` is produced by the tick/render timers, OS signals, key bindings,
/// and component logic. `App::run` handles infrastructure variants directly
/// (`Tick`, `Render`, `Resize`, `Quit`, `Suspend`, `Resume`, `ClearScreen`) and
/// forwards every other action to each component's
/// [`Component::update`](crate::components::Component::update).
///
/// ## Serialisation
///
/// [`Display`](std::fmt::Display) and [`FromStr`] encode every variant as a
/// flat string, used by the key-binding configuration layer:
///
/// ```text
/// Tick             →  "Tick"
/// Resize(80, 24)   →  "Resize(80,24)"
/// Move(Down)       →  "Move(Down)"
/// Error("oops")    →  "Error(oops)"
/// ```
///
/// # Examples
///
/// Pattern-matching on a received action:
///
/// ```
/// use dps::action::{Action, Movement};
///
/// fn is_terminal(action: &Action) -> bool {
///     matches!(action, Action::Quit | Action::Error(_))
/// }
///
/// assert!(is_terminal(&Action::Quit));
/// assert!(is_terminal(&Action::Error("disk full".to_owned())));
/// assert!(!is_terminal(&Action::Move(Movement::Down)));
/// ```
///
/// Parsing from a key-binding configuration string:
///
/// ```
/// use std::str::FromStr;
/// use dps::action::{Action, Movement};
///
/// assert_eq!(
///     Action::from_str("Move(Down)").unwrap(),
///     Action::Move(Movement::Down),
/// );
/// assert_eq!(
///     Action::from_str("Resize(80,24)").unwrap(),
///     Action::Resize(80, 24),
/// );
/// assert_eq!(
///     Action::from_str("Error(out of gas)").unwrap(),
///     Action::Error("out of gas".to_owned()),
/// );
/// ```
#[non_exhaustive]
#[derive(Default, Debug, Clone, PartialEq, Eq, VariantNames)]
pub enum Action {
    // Infrastructure
    // Handled directly by App::run; never forwarded to components.
    //
    /// Fired at the configured `tick_rate`.
    ///
    /// Drives periodic state updates such as polling background tasks or
    /// advancing progress indicators. Components respond to this in
    /// [`Component::update`](crate::components::Component::update).
    Tick,
    /// Requests a terminal frame draw.
    ///
    /// Fired at the configured `frame_rate`. `App::run`
    /// calls [`Tui::draw`](crate::tui::Tui) when this arrives and a render
    /// has been flagged as necessary.
    Render,
    /// Save terminal state and suspend the process with `SIGTSTP`.
    ///
    /// On Unix, `App::run` exits the alternate screen before raising the
    /// signal so the shell prompt appears cleanly. The process is resumed
    /// when `SIGCONT` is received — see [`Resume`](Action::Resume). Not
    /// meaningful on non-Unix platforms.
    Suspend,
    /// Restore terminal state after the process is resumed from suspension.
    ///
    /// Produced when `SIGCONT` wakes the process following a
    /// [`Suspend`](Action::Suspend). `App::run` calls
    /// [`Tui::resume`](crate::tui::Tui::resume) to re-enter the alternate
    /// screen and restart the event loop.
    Resume,
    /// Exit the event loop.
    ///
    /// Produced by key bindings (default `q` / Esc), `SIGTERM`, and
    /// `SIGINT`. `App::run` breaks on this variant; terminal cleanup is
    /// handled by [`Tui`](crate::tui::Tui)'s `Drop` implementation.
    Quit,
    /// The terminal was resized to `(columns, rows)`.
    ///
    /// Produced by [`Tui`](crate::tui::Tui)'s event loop when the terminal
    /// emulator sends a resize notification. Forces an immediate re-layout
    /// and render on the next [`Render`](Action::Render) tick.
    Resize(u16, u16),
    /// Clear the entire terminal screen before the next render.
    ///
    /// Useful after running a subprocess or when stray output has corrupted
    /// the display.
    ClearScreen,

    // User commands
    // Forwarded to every component's update method.
    //
    /// A directional or positional navigation command.
    ///
    /// Forwarded to every component's
    /// [`Component::update`](crate::components::Component::update),
    /// which applies the movement to its table selection or scroll offset.
    Move(Movement),
    /// An edit operation: yank, paste, paste-above, or delete.
    ///
    /// Forwarded to every component's
    /// [`Component::update`](crate::components::Component::update),
    /// which reads or writes the [`RegisterStore`](crate::registers::RegisterStore)
    /// and mutates its table state accordingly.
    Edit(EditOp),
    /// Switch the active tab.
    ///
    /// Consumed by the tab-pane component; forwarded to every
    /// [`Component::update`](crate::components::Component::update) so
    /// components other than the tab pane can react if needed.
    /// [`Next`](TabMotion::Next) and [`Prev`](TabMotion::Prev) accept a count
    /// (e.g. `3gt` cycles three tabs forward).
    /// [`GoTo`](TabMotion::GoTo) does not — the count *is* the destination.
    Tab(TabMotion),
    /// Confirm or activate the currently highlighted row.
    ///
    /// Typically mapped to Enter. Forwarded to every component's
    /// [`Component::update`](crate::components::Component::update),
    /// which records the selection and may produce a follow-up action.
    Select,

    // Modal responses
    // Produced only in Mode::Confirm; answer an in-flight confirmation dialog.
    //
    /// Response to a modal confirmation prompt.
    ///
    /// Produced in [`Mode::Confirm`](crate::keymap::Mode) when the user
    /// answers the active dialog. The active component decides what to do
    /// next based on the [`PromptOp`] variant.
    Prompt(PromptOp),

    // UI controls
    // Overlay panels, toggles, and display-layer controls.
    //
    /// A UI-layer control operation.
    ///
    /// Forwarded to every component's
    /// [`Component::update`](crate::components::Component::update).
    /// Currently carries [`UiOp::Help`] to toggle the which-key overlay;
    /// panel toggles and other display controls will be added here.
    Ui(UiOp),

    // Sentinels
    //
    /// A key was consumed but produced no state change.
    ///
    /// The default variant. Components return this from
    /// [`Component::update`](crate::components::Component::update)
    /// for any action they recognise but handle silently, signalling to the
    /// caller that no further processing is needed.
    #[default]
    None,
    /// Carry an error message to the event loop.
    ///
    /// Components or background tasks that encounter recoverable errors
    /// surface them through this variant rather than panicking. The event
    /// loop logs or displays the message and continues running.
    Error(String),
}

impl Action {
    /// Returns `true` if the dispatch layer should repeat this action when the
    /// user types a count prefix (e.g. `5j` → `Move(Down)` × 5).
    ///
    /// Repetition is only meaningful when each firing has a distinct
    /// side-effect. [`Movement`] variants advance the cursor each time.
    /// [`EditOp::Delete`] removes the focused row so the cursor naturally
    /// lands on the next row. [`EditOp::Paste`] and [`EditOp::PasteAbove`]
    /// insert a new row each time, so `3p` inserts three copies.
    ///
    /// [`EditOp::YankRow`] does **not** accept a count: yanking does not move
    /// the cursor, so repeating it N times merely overwrites the same register
    /// slot with the same value N times — a no-op beyond the first. See the
    /// `registers` module documentation for the planned `YankRows` action that
    /// will handle count > 1 with proper multi-value semantics.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::action::{Action, EditOp, Movement, PromptOp, TabMotion, UiOp};
    ///
    /// assert!(Action::Move(Movement::Down).accepts_count());
    /// assert!(Action::Edit(EditOp::Delete(None)).accepts_count());
    /// assert!(!Action::Edit(EditOp::YankRow(None)).accepts_count());
    /// assert!(!Action::Quit.accepts_count());
    /// assert!(!Action::Ui(UiOp::Help).accepts_count());
    /// assert!(!Action::Prompt(PromptOp::Confirm).accepts_count());
    /// ```
    #[must_use]
    pub const fn accepts_count(&self) -> bool {
        matches!(
            self,
            Self::Move(_)
                | Self::Edit(EditOp::Delete(_) | EditOp::Paste(_) | EditOp::PasteAbove(_))
                | Self::Tab(TabMotion::Next | TabMotion::Prev)
        )
    }
}

/// Serialises `Action` as a flat string.
///
/// Simple (unit) variants render as their name; data variants use
/// parenthesised payload — the same format accepted by [`FromStr`]:
///
/// ```
/// use dps::action::{Action, Movement};
///
/// assert_eq!(Action::Quit.to_string(), "Quit");
/// assert_eq!(Action::Resize(80, 24).to_string(), "Resize(80,24)");
/// assert_eq!(Action::Move(Movement::Down).to_string(), "Move(Down)");
/// assert_eq!(Action::Error("oops".to_owned()).to_string(), "Error(oops)");
/// ```
impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tick => f.write_str("Tick"),
            Self::Render => f.write_str("Render"),
            Self::Suspend => f.write_str("Suspend"),
            Self::Resume => f.write_str("Resume"),
            Self::Quit => f.write_str("Quit"),
            Self::Resize(w, h) => write!(f, "Resize({w},{h})"),
            Self::ClearScreen => f.write_str("ClearScreen"),
            Self::Move(mv) => write!(f, "Move({mv})"),
            Self::Edit(op) => write!(f, "Edit({op})"),
            Self::Tab(motion) => write!(f, "Tab({motion})"),
            Self::Select => f.write_str("Select"),
            Self::Prompt(op) => write!(f, "Prompt({op})"),
            Self::Ui(op) => write!(f, "Ui({op})"),
            Self::None => f.write_str("None"),
            Self::Error(msg) => write!(f, "Error({msg})"),
        }
    }
}

/// Parses an `Action` from its flat-string representation.
///
/// The format mirrors [`Display`](std::fmt::Display): unit variants by name,
/// data variants as `Variant(payload)`. `Resize` expects two comma-separated
/// `u16` values; `Error` and `Move` accept any non-empty payload.
///
/// # Errors
///
/// Returns [`ActionError`] if the string does not match any known variant name
/// or if a `Resize` payload cannot be parsed as two `u16` values.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use dps::action::{Action, Movement};
///
/// // unit variants
/// assert_eq!(Action::from_str("Quit").unwrap(),   Action::Quit);
/// assert_eq!(Action::from_str("Tick").unwrap(),   Action::Tick);
///
/// // data variants
/// assert_eq!(
///     Action::from_str("Move(Up)").unwrap(),
///     Action::Move(Movement::Up),
/// );
/// assert_eq!(
///     Action::from_str("Resize(120,40)").unwrap(),
///     Action::Resize(120, 40),
/// );
/// assert_eq!(
///     Action::from_str("Error(disk full)").unwrap(),
///     Action::Error("disk full".to_owned()),
/// );
///
/// // invalid input
/// assert!(Action::from_str("Unknown").is_err());
/// assert!(Action::from_str("Resize(abc,40)").is_err());
/// ```
impl FromStr for Action {
    type Err = ActionError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if let Some(inner) = s.strip_prefix("Resize(").and_then(|s| s.strip_suffix(")")) {
            let mut parts = inner.splitn(2, ',');
            let w = parts.next().and_then(|v| v.trim().parse::<u16>().ok());
            let h = parts.next().and_then(|v| v.trim().parse::<u16>().ok());

            return match (w, h) {
                (Some(w), Some(h)) => Ok(Self::Resize(w, h)),
                _ => Err(ParseError::VariantNotFound.into()),
            };
        }

        if let Some(inner) = s.strip_prefix("Move(").and_then(|s| s.strip_suffix(")")) {
            return Movement::from_str(inner).map(Self::Move);
        }

        if let Some(inner) = s.strip_prefix("Edit(").and_then(|s| s.strip_suffix(")")) {
            return EditOp::from_str(inner).map(Self::Edit);
        }

        if let Some(inner) = s.strip_prefix("Tab(").and_then(|s| s.strip_suffix(")")) {
            return TabMotion::from_str(inner).map(Self::Tab);
        }

        if let Some(inner) = s.strip_prefix("Prompt(").and_then(|s| s.strip_suffix(")")) {
            return PromptOp::from_str(inner).map(Self::Prompt);
        }

        if let Some(inner) = s.strip_prefix("Ui(").and_then(|s| s.strip_suffix(")")) {
            return UiOp::from_str(inner).map(Self::Ui);
        }

        if let Some(inner) = s.strip_prefix("Error(").and_then(|s| s.strip_suffix(")")) {
            return Ok(Self::Error(inner.to_owned()));
        }

        match s {
            "Tick" => Ok(Self::Tick),
            "Render" => Ok(Self::Render),
            "Suspend" => Ok(Self::Suspend),
            "Resume" => Ok(Self::Resume),
            "Quit" => Ok(Self::Quit),
            "ClearScreen" => Ok(Self::ClearScreen),
            "Select" => Ok(Self::Select),
            "None" => Ok(Self::None),
            _ => Err(ParseError::VariantNotFound.into()),
        }
    }
}

impl Serialize for Action {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;

        Self::from_str(&s).map_err(|_| de::Error::unknown_variant(&s, Self::VARIANTS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::registers::RegisterName;

    use rstest::rstest;

    use std::str::FromStr;

    mod display {
        use super::*;

        #[rstest]
        #[case(Action::Tick, "Tick")]
        #[case(Action::Render, "Render")]
        #[case(Action::Suspend, "Suspend")]
        #[case(Action::Resume, "Resume")]
        #[case(Action::Quit, "Quit")]
        #[case(Action::ClearScreen, "ClearScreen")]
        #[case(Action::Select, "Select")]
        #[case(Action::None, "None")]
        #[case(Action::Edit(EditOp::YankRow(None)), "Edit(YankRow)")]
        #[case(Action::Edit(EditOp::YankRow(RegisterName::try_from('a').ok())), "Edit(YankRow(a))")]
        #[case(Action::Edit(EditOp::Paste(None)), "Edit(Paste)")]
        #[case(Action::Edit(EditOp::PasteAbove(None)), "Edit(PasteAbove)")]
        #[case(Action::Edit(EditOp::Delete(RegisterName::try_from('_').ok())), "Edit(Delete(_))")]
        fn simple_variants_display(#[case] action: Action, #[case] expected: &str) {
            assert_eq!(action.to_string(), expected);
        }

        #[rstest]
        fn resize_displays_as_parens_pair() {
            assert_eq!(Action::Resize(80, 24).to_string(), "Resize(80,24)");
        }

        #[rstest]
        fn error_displays_with_message() {
            assert_eq!(
                Action::Error("something went wrong".to_owned()).to_string(),
                "Error(something went wrong)"
            );
        }

        #[rstest]
        #[case(")", "Error())")]
        #[case("(", "Error(()")]
        #[case("(nested)", "Error((nested))")]
        fn error_with_parens_in_message_displays(#[case] msg: &str, #[case] expected: &str) {
            assert_eq!(Action::Error(msg.to_owned()).to_string(), expected);
        }

        #[rstest]
        #[case(Movement::Down, "Move(Down)")]
        #[case(Movement::GotoTop, "Move(GotoTop)")]
        #[case(Movement::ScrollUp, "Move(ScrollUp)")]
        fn move_wraps_movement_in_parens(#[case] mv: Movement, #[case] expected: &str) {
            assert_eq!(Action::Move(mv).to_string(), expected);
        }

        #[rstest]
        #[case(TabMotion::Next, "Tab(Next)")]
        #[case(TabMotion::Prev, "Tab(Prev)")]
        #[case(TabMotion::GoTo(3), "Tab(GoTo(3))")]
        fn tab_wraps_motion_in_parens(#[case] motion: TabMotion, #[case] expected: &str) {
            assert_eq!(Action::Tab(motion).to_string(), expected);
        }

        #[rstest]
        #[case(PromptOp::Confirm, "Prompt(Confirm)")]
        #[case(PromptOp::Cancel, "Prompt(Cancel)")]
        fn prompt_wraps_op_in_parens(#[case] op: PromptOp, #[case] expected: &str) {
            assert_eq!(Action::Prompt(op).to_string(), expected);
        }

        #[rstest]
        fn ui_wraps_op_in_parens() {
            assert_eq!(Action::Ui(UiOp::Help).to_string(), "Ui(Help)");
        }
    }

    mod from_str {
        use super::*;

        #[rstest]
        #[case("Tick", Action::Tick)]
        #[case("Render", Action::Render)]
        #[case("Suspend", Action::Suspend)]
        #[case("Resume", Action::Resume)]
        #[case("Quit", Action::Quit)]
        #[case("ClearScreen", Action::ClearScreen)]
        #[case("Select", Action::Select)]
        #[case("None", Action::None)]
        #[case("Move(Up)", Action::Move(Movement::Up))]
        #[case("Move(Down)", Action::Move(Movement::Down))]
        #[case("Move(GotoBottom)", Action::Move(Movement::GotoBottom))]
        #[case("Edit(YankRow)", Action::Edit(EditOp::YankRow(None)))]
        #[case("Edit(YankRow(a))", Action::Edit(EditOp::YankRow(RegisterName::try_from('a').ok())))]
        #[case("Edit(Paste)", Action::Edit(EditOp::Paste(None)))]
        #[case("Edit(Paste(+))", Action::Edit(EditOp::Paste(RegisterName::try_from('+').ok())))]
        #[case("Edit(PasteAbove)", Action::Edit(EditOp::PasteAbove(None)))]
        #[case("Edit(PasteAbove(+))", Action::Edit(EditOp::PasteAbove(RegisterName::try_from('+').ok())))]
        #[case("Edit(Delete)", Action::Edit(EditOp::Delete(None)))]
        #[case("Edit(Delete(_))", Action::Edit(EditOp::Delete(RegisterName::try_from('_').ok())))]
        fn known_variants_parse(
            #[case] input: &str,
            #[case] expected: Action,
        ) -> Result<(), ActionError> {
            assert_eq!(Action::from_str(input)?, expected);

            Ok(())
        }

        #[rstest]
        fn resize_parses_width_and_height() -> Result<(), ActionError> {
            assert_eq!(Action::from_str("Resize(80,24)")?, Action::Resize(80, 24));

            Ok(())
        }

        #[rstest]
        fn error_parses_message() -> Result<(), ActionError> {
            assert_eq!(
                Action::from_str("Error(oops)")?,
                Action::Error("oops".to_owned())
            );

            Ok(())
        }

        #[rstest]
        #[case("Error())", Action::Error(")".to_owned()))]
        #[case("Error(()", Action::Error("(".to_owned()))]
        #[case("Error((nested))", Action::Error("(nested)".to_owned()))]
        fn error_with_parens_in_payload_parses(
            #[case] input: &str,
            #[case] expected: Action,
        ) -> Result<(), ActionError> {
            assert_eq!(Action::from_str(input)?, expected);

            Ok(())
        }

        #[rstest]
        #[case("Tab(Next)", Action::Tab(TabMotion::Next))]
        #[case("Tab(Prev)", Action::Tab(TabMotion::Prev))]
        #[case("Tab(GoTo(3))", Action::Tab(TabMotion::GoTo(3)))]
        fn tab_variants_parse(
            #[case] input: &str,
            #[case] expected: Action,
        ) -> Result<(), ActionError> {
            assert_eq!(Action::from_str(input)?, expected);

            Ok(())
        }

        #[rstest]
        #[case("Prompt(Confirm)", Action::Prompt(PromptOp::Confirm))]
        #[case("Prompt(Cancel)", Action::Prompt(PromptOp::Cancel))]
        fn prompt_variants_parse(
            #[case] input: &str,
            #[case] expected: Action,
        ) -> Result<(), ActionError> {
            assert_eq!(Action::from_str(input)?, expected);

            Ok(())
        }

        #[rstest]
        fn ui_help_parses() -> Result<(), ActionError> {
            assert_eq!(Action::from_str("Ui(Help)")?, Action::Ui(UiOp::Help));

            Ok(())
        }

        #[rstest]
        #[case("Down")]
        #[case("Unknown")]
        #[case("Resize(notanumber,24)")]
        #[case("Resize(80)")]
        fn invalid_input_returns_err(#[case] input: &str) {
            assert!(Action::from_str(input).is_err());
        }
    }

    mod serde_roundtrip {
        use super::*;

        fn roundtrip(action: &Action) -> Result<Action, serde_json::Error> {
            let json = serde_json::to_string(action)?;

            serde_json::from_str(&json)
        }

        #[rstest]
        #[case(Action::Tick)]
        #[case(Action::Render)]
        #[case(Action::Suspend)]
        #[case(Action::Resume)]
        #[case(Action::Quit)]
        #[case(Action::ClearScreen)]
        #[case(Action::Select)]
        #[case(Action::None)]
        fn simple_actions_roundtrip(#[case] action: Action) -> Result<(), serde_json::Error> {
            assert_eq!(roundtrip(&action)?, action);

            Ok(())
        }

        #[rstest]
        fn resize_roundtrips() -> Result<(), serde_json::Error> {
            assert_eq!(roundtrip(&Action::Resize(80, 24))?, Action::Resize(80, 24));

            Ok(())
        }

        #[rstest]
        fn error_roundtrips() -> Result<(), serde_json::Error> {
            let action = Action::Error("oops".to_owned());
            assert_eq!(roundtrip(&action)?, action);

            Ok(())
        }

        #[rstest]
        #[case(")")]
        #[case("(")]
        #[case("(nested)")]
        fn error_with_parens_in_message_roundtrips(
            #[case] msg: &str,
        ) -> Result<(), serde_json::Error> {
            let action = Action::Error(msg.to_owned());
            assert_eq!(roundtrip(&action)?, action);

            Ok(())
        }

        #[rstest]
        #[case(Movement::Up)]
        #[case(Movement::Down)]
        #[case(Movement::Left)]
        #[case(Movement::Right)]
        #[case(Movement::LineUp)]
        #[case(Movement::LineDown)]
        #[case(Movement::ScrollUp)]
        #[case(Movement::ScrollDown)]
        #[case(Movement::PageUp)]
        #[case(Movement::PageDown)]
        #[case(Movement::GotoTop)]
        #[case(Movement::GotoBottom)]
        fn movement_roundtrips(#[case] mv: Movement) -> Result<(), serde_json::Error> {
            let action = Action::Move(mv);
            assert_eq!(roundtrip(&action)?, action);

            Ok(())
        }

        #[rstest]
        fn movement_serializes_as_move_parens_string() -> Result<(), serde_json::Error> {
            assert_eq!(
                serde_json::to_string(&Action::Move(Movement::Down))?,
                "\"Move(Down)\""
            );

            Ok(())
        }

        #[rstest]
        #[case(Action::Edit(EditOp::YankRow(None)))]
        #[case(Action::Edit(EditOp::YankRow(RegisterName::try_from('a').ok())))]
        #[case(Action::Edit(EditOp::Paste(None)))]
        #[case(Action::Edit(EditOp::Paste(RegisterName::try_from('+').ok())))]
        #[case(Action::Edit(EditOp::PasteAbove(None)))]
        #[case(Action::Edit(EditOp::Delete(None)))]
        #[case(Action::Edit(EditOp::Delete(RegisterName::try_from('_').ok())))]
        fn edit_variants_roundtrip(#[case] action: Action) -> Result<(), serde_json::Error> {
            assert_eq!(roundtrip(&action)?, action);

            Ok(())
        }

        #[rstest]
        fn edit_serializes_as_edit_parens_string() -> Result<(), serde_json::Error> {
            assert_eq!(
                serde_json::to_string(&Action::Edit(EditOp::YankRow(None)))?,
                "\"Edit(YankRow)\""
            );
            assert_eq!(
                serde_json::to_string(&Action::Edit(EditOp::YankRow(
                    RegisterName::try_from('a').ok()
                )))?,
                "\"Edit(YankRow(a))\""
            );

            Ok(())
        }

        #[rstest]
        #[case(Action::Tab(TabMotion::Next))]
        #[case(Action::Tab(TabMotion::Prev))]
        #[case(Action::Tab(TabMotion::GoTo(3)))]
        fn tab_variants_roundtrip(#[case] action: Action) -> Result<(), serde_json::Error> {
            assert_eq!(roundtrip(&action)?, action);

            Ok(())
        }

        #[rstest]
        #[case(Action::Prompt(PromptOp::Confirm))]
        #[case(Action::Prompt(PromptOp::Cancel))]
        fn prompt_variants_roundtrip(#[case] action: Action) -> Result<(), serde_json::Error> {
            assert_eq!(roundtrip(&action)?, action);

            Ok(())
        }

        #[rstest]
        fn ui_variant_roundtrips() -> Result<(), serde_json::Error> {
            assert_eq!(roundtrip(&Action::Ui(UiOp::Help))?, Action::Ui(UiOp::Help));

            Ok(())
        }

        #[rstest]
        fn unknown_variant_returns_error() {
            assert!(serde_json::from_str::<Action>("\"NotAnAction\"").is_err());
        }
    }

    mod accepts_count {
        use super::*;

        #[rstest]
        #[case(Movement::Down)]
        #[case(Movement::Up)]
        #[case(Movement::Left)]
        #[case(Movement::Right)]
        #[case(Movement::LineUp)]
        #[case(Movement::LineDown)]
        #[case(Movement::ScrollUp)]
        #[case(Movement::ScrollDown)]
        #[case(Movement::PageUp)]
        #[case(Movement::PageDown)]
        #[case(Movement::GotoTop)]
        #[case(Movement::GotoBottom)]
        fn move_variants_accept_count(#[case] mv: Movement) {
            assert!(Action::Move(mv).accepts_count());
        }

        #[rstest]
        #[case(Action::Edit(EditOp::Paste(None)))]
        #[case(Action::Edit(EditOp::PasteAbove(None)))]
        #[case(Action::Edit(EditOp::Delete(None)))]
        #[case(Action::Edit(EditOp::Delete(RegisterName::try_from('_').ok())))]
        fn edit_variants_accept_count(#[case] action: Action) {
            assert!(action.accepts_count());
        }

        #[rstest]
        #[case(Action::Quit)]
        #[case(Action::Suspend)]
        #[case(Action::Resume)]
        #[case(Action::Tick)]
        #[case(Action::Render)]
        #[case(Action::ClearScreen)]
        #[case(Action::Select)]
        #[case(Action::None)]
        #[case(Action::Edit(EditOp::YankRow(None)))]
        #[case(Action::Edit(EditOp::YankRow(RegisterName::try_from('a').ok())))]
        #[case(Action::Edit(EditOp::CyclePaste))]
        fn non_movement_actions_reject_count(#[case] action: Action) {
            assert!(!action.accepts_count());
        }

        #[rstest]
        fn resize_rejects_count() {
            assert!(!Action::Resize(80, 24).accepts_count());
        }

        #[rstest]
        fn error_rejects_count() {
            assert!(!Action::Error("oops".to_owned()).accepts_count());
        }

        #[rstest]
        #[case(TabMotion::Next)]
        #[case(TabMotion::Prev)]
        fn tab_next_prev_accept_count(#[case] motion: TabMotion) {
            assert!(Action::Tab(motion).accepts_count());
        }

        #[rstest]
        fn tab_goto_rejects_count() {
            assert!(!Action::Tab(TabMotion::GoTo(3)).accepts_count());
        }

        #[rstest]
        #[case(PromptOp::Confirm)]
        #[case(PromptOp::Cancel)]
        fn prompt_variants_reject_count(#[case] op: PromptOp) {
            assert!(!Action::Prompt(op).accepts_count());
        }

        #[rstest]
        fn ui_help_rejects_count() {
            assert!(!Action::Ui(UiOp::Help).accepts_count());
        }
    }
}
