//! Edit operations dispatched through the event loop.

use std::fmt;
use std::str::FromStr;

use super::ActionError;
use super::error::ParseError;

/// Edit operations produced by yank, paste, and delete key bindings.
///
/// Mirrors the structure of [`Movement`](crate::action::Movement): each variant
/// carries an optional register character resolved by [`SequenceEngine`] before
/// dispatch. `None` means the unnamed register `"`.
///
/// ## Serialisation
///
/// Config bindings always omit the register — `"Edit(YankRow)"` — the
/// [`SequenceEngine`] injects it at runtime, producing `"Edit(YankRow(a))"` when
/// a `"a` prefix was typed.
///
/// ```
/// use std::str::FromStr;
/// use dps::action::EditOp;
///
/// assert_eq!(EditOp::YankRow(None).to_string(),    "YankRow");
/// assert_eq!(EditOp::YankRow(Some('a')).to_string(), "YankRow(a)");
/// assert_eq!(EditOp::from_str("Paste").unwrap(),    EditOp::Paste(None));
/// assert_eq!(EditOp::from_str("Paste(+)").unwrap(), EditOp::Paste(Some('+')));
/// assert_eq!(EditOp::from_str("CyclePaste").unwrap(), EditOp::CyclePaste);
/// ```
///
/// [`SequenceEngine`]: crate::keymap::SequenceEngine
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOp {
    /// Yank (copy) the focused row to a register.
    YankRow(Option<char>),
    /// Insert a row from a register below the focused row.
    ///
    /// Inserts a new row immediately below the cursor and moves the cursor to
    /// it. A subsequent [`CyclePaste`](EditOp::CyclePaste) replaces that row
    /// with the next entry in the yank ring.
    Paste(Option<char>),
    /// Insert a row from a register above the focused row.
    ///
    /// Inserts a new row at the cursor position (pushing the current row down)
    /// and keeps the cursor on the newly inserted row. A subsequent
    /// [`CyclePaste`](EditOp::CyclePaste) replaces that row with the next
    /// entry in the yank ring.
    PasteAbove(Option<char>),
    /// Replace the most recently pasted row with the next yank-ring entry.
    ///
    /// Only has an effect immediately after [`Paste`](EditOp::Paste) or
    /// [`PasteAbove`](EditOp::PasteAbove). Any intervening action (move,
    /// yank, delete, …) breaks the chain and makes this a no-op. Successive
    /// `CyclePaste` actions walk the ring from newest to oldest, wrapping when
    /// the ring is exhausted.
    CyclePaste,
    /// Delete the focused row, pushing to the delete history stack.
    Delete(Option<char>),
    // TODO: Change(Option<char>) — blocked on Mode::Insert (Insert mode not yet implemented).
    //       Add once Mode::Insert and the EnterInsert/EnterNormal action pair exist.
}

impl EditOp {
    /// Returns a copy of `self` with the register field replaced by `reg`.
    ///
    /// Used by [`SequenceEngine`] to inject the register resolved from a `"x`
    /// prefix into the action just before dispatch.
    ///
    /// ```
    /// use dps::action::EditOp;
    ///
    /// assert_eq!(EditOp::Delete(None).with_register(Some('a')), EditOp::Delete(Some('a')));
    /// assert_eq!(EditOp::Delete(Some('b')).with_register(None),  EditOp::Delete(None));
    /// ```
    ///
    /// [`SequenceEngine`]: crate::keymap::SequenceEngine
    #[must_use]
    pub const fn with_register(self, reg: Option<char>) -> Self {
        match self {
            Self::YankRow(_) => Self::YankRow(reg),
            Self::Paste(_) => Self::Paste(reg),
            Self::PasteAbove(_) => Self::PasteAbove(reg),
            Self::CyclePaste => Self::CyclePaste,
            Self::Delete(_) => Self::Delete(reg),
        }
    }

    /// Returns the register associated with this operation, or `None` for the unnamed register.
    ///
    /// ```
    /// use dps::action::EditOp;
    ///
    /// assert_eq!(EditOp::YankRow(Some('a')).register(), Some('a'));
    /// assert_eq!(EditOp::YankRow(None).register(),      None);
    /// ```
    #[must_use]
    pub const fn register(self) -> Option<char> {
        match self {
            Self::YankRow(r) | Self::Paste(r) | Self::PasteAbove(r) | Self::Delete(r) => r,
            Self::CyclePaste => None,
        }
    }
}

impl fmt::Display for EditOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::YankRow(None) => f.write_str("YankRow"),
            Self::YankRow(Some(r)) => write!(f, "YankRow({r})"),
            Self::Paste(None) => f.write_str("Paste"),
            Self::Paste(Some(r)) => write!(f, "Paste({r})"),
            Self::PasteAbove(None) => f.write_str("PasteAbove"),
            Self::PasteAbove(Some(r)) => write!(f, "PasteAbove({r})"),
            Self::CyclePaste => f.write_str("CyclePaste"),
            Self::Delete(None) => f.write_str("Delete"),
            Self::Delete(Some(r)) => write!(f, "Delete({r})"),
        }
    }
}

/// Parses an `EditOp` from its flat-string representation.
///
/// The format mirrors [`Display`](std::fmt::Display): variant name optionally
/// followed by a single register character in parentheses.
///
/// # Errors
///
/// Returns [`ActionError`] if the string does not match any known variant name.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use dps::action::EditOp;
///
/// assert_eq!(EditOp::from_str("YankRow").unwrap(),    EditOp::YankRow(None));
/// assert_eq!(EditOp::from_str("YankRow(a)").unwrap(), EditOp::YankRow(Some('a')));
/// assert_eq!(EditOp::from_str("Delete(_)").unwrap(),  EditOp::Delete(Some('_')));
/// assert!(EditOp::from_str("Unknown").is_err());
/// ```
impl FromStr for EditOp {
    type Err = ActionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, reg) = split_name_reg(s);
        match name {
            "YankRow" => Ok(Self::YankRow(reg)),
            "Paste" => Ok(Self::Paste(reg)),
            "PasteAbove" => Ok(Self::PasteAbove(reg)),
            "CyclePaste" => Ok(Self::CyclePaste),
            "Delete" => Ok(Self::Delete(reg)),
            _ => Err(ParseError::VariantNotFound.into()),
        }
    }
}

/// Splits `"Name"` → `("Name", None)` and `"Name(x)"` → `("Name", Some('x'))`.
///
/// The register char is the single character between the final `(` and closing `)`.
/// Returns `None` for the register if the inner text is not exactly one character.
fn split_name_reg(s: &str) -> (&str, Option<char>) {
    if let Some(without_close) = s.strip_suffix(')')
        && let Some(open) = without_close.rfind('(')
    {
        let inner = &without_close[open + 1..];
        let name = &without_close[..open];
        let reg = if inner.chars().count() == 1 {
            inner.chars().next()
        } else {
            None
        };
        return (name, reg);
    }
    (s, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    mod display {
        use super::*;

        #[rstest]
        #[case(EditOp::YankRow(None), "YankRow")]
        #[case(EditOp::YankRow(Some('a')), "YankRow(a)")]
        #[case(EditOp::Paste(None), "Paste")]
        #[case(EditOp::Paste(Some('+')), "Paste(+)")]
        #[case(EditOp::PasteAbove(None), "PasteAbove")]
        #[case(EditOp::PasteAbove(Some('*')), "PasteAbove(*)")]
        #[case(EditOp::CyclePaste, "CyclePaste")]
        #[case(EditOp::Delete(None), "Delete")]
        #[case(EditOp::Delete(Some('_')), "Delete(_)")]
        fn formats_correctly(#[case] op: EditOp, #[case] expected: &str) {
            assert_eq!(op.to_string(), expected);
        }
    }

    mod from_str {
        use super::*;

        #[rstest]
        #[case("YankRow", EditOp::YankRow(None))]
        #[case("YankRow(a)", EditOp::YankRow(Some('a')))]
        #[case("Paste", EditOp::Paste(None))]
        #[case("Paste(+)", EditOp::Paste(Some('+')))]
        #[case("PasteAbove", EditOp::PasteAbove(None))]
        #[case("CyclePaste", EditOp::CyclePaste)]
        #[case("Delete", EditOp::Delete(None))]
        #[case("Delete(_)", EditOp::Delete(Some('_')))]
        fn parses_correctly(
            #[case] input: &str,
            #[case] expected: EditOp,
        ) -> Result<(), ActionError> {
            assert_eq!(EditOp::from_str(input)?, expected);
            Ok(())
        }

        #[rstest]
        #[case("Unknown")]
        #[case("")]
        #[case("yankrow")]
        fn unknown_variants_return_err(#[case] input: &str) {
            assert!(EditOp::from_str(input).is_err());
        }
    }

    mod roundtrip {
        use super::*;

        #[rstest]
        #[case(EditOp::YankRow(None))]
        #[case(EditOp::YankRow(Some('a')))]
        #[case(EditOp::Paste(None))]
        #[case(EditOp::Paste(Some('+')))]
        #[case(EditOp::PasteAbove(None))]
        #[case(EditOp::PasteAbove(Some('*')))]
        #[case(EditOp::CyclePaste)]
        #[case(EditOp::Delete(None))]
        #[case(EditOp::Delete(Some('_')))]
        fn display_then_from_str_is_identity(#[case] op: EditOp) -> Result<(), ActionError> {
            assert_eq!(EditOp::from_str(&op.to_string())?, op);
            Ok(())
        }
    }

    mod with_register {
        use super::*;

        #[rstest]
        fn injects_register_into_yank_row() {
            assert_eq!(
                EditOp::YankRow(None).with_register(Some('a')),
                EditOp::YankRow(Some('a'))
            );
        }

        #[rstest]
        fn clears_register_from_delete() {
            assert_eq!(
                EditOp::Delete(Some('b')).with_register(None),
                EditOp::Delete(None)
            );
        }

        #[rstest]
        fn preserves_variant_on_paste() {
            let op = EditOp::Paste(None);
            assert_eq!(op.with_register(Some('+')), EditOp::Paste(Some('+')));
        }

        #[rstest]
        fn preserves_variant_on_paste_above() {
            let op = EditOp::PasteAbove(None);
            assert_eq!(op.with_register(Some('+')), EditOp::PasteAbove(Some('+')));
        }

        #[rstest]
        fn cycle_paste_ignores_register() {
            assert_eq!(
                EditOp::CyclePaste.with_register(Some('a')),
                EditOp::CyclePaste
            );
            assert_eq!(EditOp::CyclePaste.with_register(None), EditOp::CyclePaste);
        }
    }

    mod register_accessor {
        use super::*;

        #[rstest]
        #[case(EditOp::YankRow(Some('a')), Some('a'))]
        #[case(EditOp::YankRow(None), None)]
        #[case(EditOp::Delete(Some('_')), Some('_'))]
        #[case(EditOp::CyclePaste, None)]
        fn returns_inner_char(#[case] op: EditOp, #[case] expected: Option<char>) {
            assert_eq!(op.register(), expected);
        }
    }
}
