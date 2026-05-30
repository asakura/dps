//! UI action types dispatched through the event loop.
//!
//! Two enums are provided:
//!
//! - [`Movement`] — directional navigation commands (Up, Down, scroll, page, go-to-top/bottom).
//!   Implements [`Display`](std::fmt::Display), [`FromStr`], [`Serialize`], and [`Deserialize`]
//!   so that key-binding configuration can reference movements by name
//!   (`"Down"`, `"GotoBottom"`, …).
//!
//! - [`Action`] — the unified message type flowing through the application event loop.
//!   Produced by timers, OS signals, key bindings, and component logic; consumed by
//!   `App::run`. Serialises as a flat string — simple variants
//!   by name (`"Quit"`, `"Tick"`, …), data variants with parenthesised payload
//!   (`"Move(Down)"`, `"Resize(80,24)"`, `"Error(oops)"`) — for use in key-binding
//!   configuration files.

mod edit;
mod error;
mod movement;

pub use edit::EditOp;
pub use error::Error as ActionError;
pub use movement::Movement;

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use strum::VariantNames;

use error::ParseError;

/// The unified message type flowing through the application event loop.
///
/// `Action` is produced by the tick/render timers, OS signals, key bindings,
/// and component logic. `App::run` handles infrastructure variants directly
/// (`Tick`, `Render`, `Resize`, `Quit`, `Suspend`, `Resume`, `ClearScreen`) and
/// forwards every action to each component's
/// [`ComponentNew::update`](crate::components::ComponentNew::update).
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
#[derive(Default, Debug, Clone, PartialEq, Eq, VariantNames)]
pub enum Action {
    /// Fired at the configured `tick_rate`.
    ///
    /// Drives periodic state updates such as polling background tasks or
    /// advancing progress indicators. Components respond to this in
    /// [`ComponentNew::update`](crate::components::ComponentNew::update).
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

    /// A directional or positional navigation command.
    ///
    /// Forwarded to every component's
    /// [`ComponentNew::update`](crate::components::ComponentNew::update),
    /// which applies the movement to its table selection or scroll offset.
    Move(Movement),
    /// An edit operation: yank, paste, paste-above, or delete.
    ///
    /// Forwarded to every component's
    /// [`ComponentNew::update`](crate::components::ComponentNew::update),
    /// which reads or writes the [`RegisterStore`](crate::registers::RegisterStore)
    /// and mutates its table state accordingly.
    Edit(EditOp),
    /// Confirm or activate the currently highlighted row.
    ///
    /// Typically mapped to Enter. Forwarded to every component's
    /// [`ComponentNew::update`](crate::components::ComponentNew::update),
    /// which records the selection and may produce a follow-up action.
    Select,
    /// Affirmative answer to a confirmation prompt.
    ///
    /// Produced in [`Mode::Confirm`](crate::keymap::Mode) when the user
    /// presses `y` or Enter. The active component decides what to do next
    /// (e.g. proceed with a destructive operation).
    Confirm,
    /// Negative answer to a confirmation prompt — dismiss without acting.
    ///
    /// Produced in [`Mode::Confirm`](crate::keymap::Mode) when the user
    /// presses `n`, Esc, or `q`. Returns control to the caller without
    /// performing the guarded operation.
    Cancel,
    /// Toggle the which-key / help overlay.
    ///
    /// Wired to `?` by default. Forwarded to every component's
    /// [`ComponentNew::update`](crate::components::ComponentNew::update).
    Help,

    /// A key was consumed but produced no state change.
    ///
    /// The default variant. Components return this from
    /// [`ComponentNew::update`](crate::components::ComponentNew::update)
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
    /// use dps::action::{Action, EditOp, Movement};
    ///
    /// assert!(Action::Move(Movement::Down).accepts_count());
    /// assert!(Action::Edit(EditOp::Delete(None)).accepts_count());
    /// assert!(!Action::Edit(EditOp::YankRow(None)).accepts_count());
    /// assert!(!Action::Quit.accepts_count());
    /// assert!(!Action::Help.accepts_count());
    /// assert!(!Action::Confirm.accepts_count());
    /// ```
    #[must_use]
    pub const fn accepts_count(&self) -> bool {
        matches!(
            self,
            Self::Move(_) | Self::Edit(EditOp::Delete(_) | EditOp::Paste(_) | EditOp::PasteAbove(_))
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
            Self::Select => f.write_str("Select"),
            Self::Help => f.write_str("Help"),
            Self::Confirm => f.write_str("Confirm"),
            Self::Cancel => f.write_str("Cancel"),
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
            "Help" => Ok(Self::Help),
            "Confirm" => Ok(Self::Confirm),
            "Cancel" => Ok(Self::Cancel),
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
        #[case(Action::Help, "Help")]
        #[case(Action::Confirm, "Confirm")]
        #[case(Action::Cancel, "Cancel")]
        #[case(Action::None, "None")]
        #[case(Action::Select, "Select")]
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
        #[case("Help", Action::Help)]
        #[case("Confirm", Action::Confirm)]
        #[case("Cancel", Action::Cancel)]
        #[case("None", Action::None)]
        #[case("Select", Action::Select)]
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
        #[case(Action::Help)]
        #[case(Action::Confirm)]
        #[case(Action::Cancel)]
        #[case(Action::None)]
        #[case(Action::Select)]
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
        #[case(Action::Confirm)]
        #[case(Action::Cancel)]
        #[case(Action::Help)]
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
    }
}
