//! Register-name type — validated at construction.
//!
//! [`RegisterName`] identifies a single Vim-style register slot. Construct
//! one from a character via [`TryFrom<char>`]; the unnamed register is the
//! constant [`RegisterName::Unnamed`].
//!
//! ```
//! use dps::registers::RegisterName;
//!
//! assert_eq!(RegisterName::try_from('"'), Ok(RegisterName::Unnamed));
//! assert_eq!(RegisterName::try_from('_'), Ok(RegisterName::BlackHole));
//! assert_eq!(RegisterName::try_from('+'), Ok(RegisterName::Clipboard));
//! assert_eq!(RegisterName::try_from('*'), Ok(RegisterName::Selection));
//! assert_eq!(RegisterName::try_from('0'), Ok(RegisterName::Yank));
//! assert!(matches!(RegisterName::try_from('1'), Ok(RegisterName::Numbered(_))));
//! assert!(RegisterName::try_from('a').is_ok());
//! assert!(RegisterName::try_from('😀').is_err());
//! assert_eq!(char::from(RegisterName::Unnamed), '"');
//! ```

use std::fmt;

use super::RegisterError;
use super::error::{InvalidRegisterIndex, InvalidRegisterLetter};

/// Private-field wrapper that prevents `Named(RegLetter(c))` from being written
/// outside this module. Obtain one through [`TryFrom<char>`] on [`RegisterName`].
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegLetter(char);

/// Validated numeric index for a numbered delete-history register (`1`–`9`).
///
/// The inner value is always in `1..=9`; construct via [`TryFrom<u8>`].
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegIndex(pub(crate) u8);

impl TryFrom<u8> for RegIndex {
    type Error = RegisterError;

    /// Converts a numeric index `1`–`9` into a [`RegIndex`], returning
    /// [`InvalidRegisterIndex`] for any value outside that range.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::RegIndex;
    ///
    /// assert!(RegIndex::try_from(1u8).is_ok());
    /// assert!(RegIndex::try_from(9u8).is_ok());
    /// assert!(matches!(RegIndex::try_from(0u8), Err(_)));
    /// assert!(matches!(RegIndex::try_from(10u8), Err(_)));
    /// ```
    fn try_from(n: u8) -> Result<Self, Self::Error> {
        match n {
            1..=9 => Ok(Self(n)),
            _ => Err(InvalidRegisterIndex(n).into()),
        }
    }
}

/// Identifies a single Vim-style register slot.
///
/// | Variant     | Character(s) | Behaviour                                        |
/// |-------------|--------------|--------------------------------------------------|
/// | `Unnamed`   | `"`          | Always receives the most recent yank or delete   |
/// | `BlackHole` | `_`          | Black hole — writes discarded, reads always `None` |
/// | `Clipboard` | `+`          | OS clipboard (standard)                          |
/// | `Selection` | `*`          | OS clipboard (primary/selection)                 |
/// | `Yank`      | `0`          | Yank register — head of the yank ring            |
/// | `Numbered`  | `1`–`9`      | Delete history stack                             |
/// | `Named`     | `a`–`z`      | Named user registers (persist until overwritten) |
/// | `Named`     | `A`–`Z`      | Append to lowercase partner                      |
///
/// Direct construction of `Named` is impossible outside this module; use
/// [`TryFrom<char>`] instead.
///
/// ```
/// use dps::registers::RegisterName;
///
/// assert_eq!(char::from(RegisterName::try_from('a').unwrap()), 'a');
/// assert_eq!(char::from(RegisterName::try_from('1').unwrap()), '1');
/// assert_eq!(char::from(RegisterName::Unnamed), '"');
/// assert_eq!(char::from(RegisterName::BlackHole), '_');
/// assert_eq!(char::from(RegisterName::Clipboard), '+');
/// assert_eq!(char::from(RegisterName::Selection), '*');
/// assert_eq!(char::from(RegisterName::Yank), '0');
/// assert!(RegisterName::try_from('?').is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegisterName {
    /// The unnamed register `"` — the implicit default.
    Unnamed,
    /// The black-hole register `_` — writes discarded, reads always `None`.
    BlackHole,
    /// The OS clipboard register `+`.
    Clipboard,
    /// The OS primary-selection register `*`.
    Selection,
    /// The yank register `0` — head of the internal yank ring.
    Yank,
    /// A numbered delete-history register `1`–`9`.
    ///
    /// Constructed only through [`TryFrom<char>`]; invariant: inner value ∈ 1..=9.
    Numbered(RegIndex),
    /// A validated named register (`a`–`z`, `A`–`Z`).
    Named(RegLetter),
}

impl RegisterName {
    /// Returns the next higher-numbered delete-history slot, or `None` if this
    /// is already `'9'` or not a [`Numbered`](Self::Numbered) variant.
    ///
    /// Used by [`RegisterStore::push_delete`](crate::registers::RegisterStore::push_delete)
    /// to shift the delete-history stack without raw byte arithmetic.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::RegisterName;
    ///
    /// let r1 = RegisterName::try_from('1').unwrap();
    /// let r8 = RegisterName::try_from('8').unwrap();
    /// let r9 = RegisterName::try_from('9').unwrap();
    ///
    /// assert_eq!(r1.next_numbered(), Some(RegisterName::try_from('2').unwrap()));
    /// assert_eq!(r8.next_numbered(), Some(r9));
    /// assert_eq!(r9.next_numbered(), None);
    /// assert_eq!(RegisterName::Unnamed.next_numbered(), None);
    /// ```
    #[must_use]
    pub const fn next_numbered(self) -> Option<Self> {
        match self {
            Self::Numbered(RegIndex(n)) if n < 9 => Some(Self::Numbered(RegIndex(n + 1))),
            _ => None,
        }
    }
}

/// Formats the register name as its identifying character.
///
/// # Examples
///
/// ```
/// use dps::registers::RegisterName;
///
/// assert_eq!(format!("{}", RegisterName::Unnamed), "\"");
/// assert_eq!(format!("{}", RegisterName::try_from('a').unwrap()), "a");
/// ```
impl fmt::Display for RegisterName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", char::from(*self))
    }
}

impl TryFrom<char> for RegisterName {
    type Error = RegisterError;

    /// Converts a `char` into a [`RegisterName`], returning
    /// [`InvalidRegisterLetter`] if it is not a valid register character.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::RegisterName;
    ///
    /// assert_eq!(RegisterName::try_from('"'), Ok(RegisterName::Unnamed));
    /// assert_eq!(RegisterName::try_from('_'), Ok(RegisterName::BlackHole));
    /// assert_eq!(RegisterName::try_from('+'), Ok(RegisterName::Clipboard));
    /// assert_eq!(RegisterName::try_from('*'), Ok(RegisterName::Selection));
    /// assert_eq!(RegisterName::try_from('0'), Ok(RegisterName::Yank));
    /// assert!(RegisterName::try_from('a').is_ok());
    /// assert!(matches!(RegisterName::try_from('?'), Err(_)));
    /// ```
    fn try_from(c: char) -> Result<Self, Self::Error> {
        match c {
            '"' => Ok(Self::Unnamed),
            '_' => Ok(Self::BlackHole),
            '+' => Ok(Self::Clipboard),
            '*' => Ok(Self::Selection),
            '0' => Ok(Self::Yank),
            '1'..='9' => Ok(Self::Numbered(RegIndex(c as u8 - b'0'))),
            'a'..='z' | 'A'..='Z' => Ok(Self::Named(RegLetter(c))),
            _ => Err(InvalidRegisterLetter(c).into()),
        }
    }
}

impl From<RegisterName> for char {
    /// Converts a [`RegisterName`] into its identifying character.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::RegisterName;
    ///
    /// assert_eq!(char::from(RegisterName::Unnamed),      '"');
    /// assert_eq!(char::from(RegisterName::BlackHole),    '_');
    /// assert_eq!(char::from(RegisterName::Clipboard),    '+');
    /// assert_eq!(char::from(RegisterName::Selection),    '*');
    /// assert_eq!(char::from(RegisterName::Yank),         '0');
    /// assert_eq!(char::from(RegisterName::try_from('3').unwrap()), '3');
    /// assert_eq!(char::from(RegisterName::try_from('a').unwrap()), 'a');
    /// ```
    fn from(r: RegisterName) -> Self {
        match r {
            RegisterName::Unnamed => '"',
            RegisterName::BlackHole => '_',
            RegisterName::Clipboard => '+',
            RegisterName::Selection => '*',
            RegisterName::Yank => '0',
            RegisterName::Numbered(RegIndex(n)) => Self::from(b'0' + n),
            RegisterName::Named(rc) => rc.0,
        }
    }
}

impl From<RegLetter> for char {
    /// Converts a [`RegLetter`] into the underlying character.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::registers::RegisterName;
    ///
    /// if let RegisterName::Named(rc) = RegisterName::try_from('z').unwrap() {
    ///     assert_eq!(char::from(rc), 'z');
    /// }
    /// ```
    fn from(rc: RegLetter) -> Self {
        rc.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registers::RegisterError;
    use rstest::rstest;

    mod display {
        use super::*;

        #[rstest]
        fn unnamed_displays_as_double_quote() {
            assert_eq!(RegisterName::Unnamed.to_string(), "\"");
        }

        #[rstest]
        fn black_hole_displays_as_underscore() {
            assert_eq!(RegisterName::BlackHole.to_string(), "_");
        }

        #[rstest]
        fn clipboard_displays_as_plus() {
            assert_eq!(RegisterName::Clipboard.to_string(), "+");
        }

        #[rstest]
        fn selection_displays_as_star() {
            assert_eq!(RegisterName::Selection.to_string(), "*");
        }

        #[rstest]
        fn yank_displays_as_zero() {
            assert_eq!(RegisterName::Yank.to_string(), "0");
        }

        #[rstest]
        fn named_displays_as_char() -> Result<(), RegisterError> {
            let r = RegisterName::try_from('a')?;

            assert_eq!(r.to_string(), "a");

            Ok(())
        }
    }

    mod try_from_char {
        use super::*;

        #[rstest]
        fn unnamed_char_gives_unnamed() {
            assert_eq!(RegisterName::try_from('"'), Ok(RegisterName::Unnamed));
        }

        #[rstest]
        fn underscore_gives_black_hole() {
            assert_eq!(RegisterName::try_from('_'), Ok(RegisterName::BlackHole));
        }

        #[rstest]
        fn plus_gives_clipboard() {
            assert_eq!(RegisterName::try_from('+'), Ok(RegisterName::Clipboard));
        }

        #[rstest]
        fn star_gives_selection() {
            assert_eq!(RegisterName::try_from('*'), Ok(RegisterName::Selection));
        }

        #[rstest]
        fn zero_gives_yank() {
            assert_eq!(RegisterName::try_from('0'), Ok(RegisterName::Yank));
        }

        #[rstest]
        #[case('a')]
        fn valid_named_chars_succeed(#[case] c: char) -> Result<(), RegisterError> {
            RegisterName::try_from(c)?;

            Ok(())
        }

        #[rstest]
        #[case('1', 1u8)]
        #[case('9', 9u8)]
        fn digit_gives_numbered(#[case] c: char, #[case] n: u8) {
            assert_eq!(
                RegisterName::try_from(c),
                Ok(RegisterName::Numbered(RegIndex(n)))
            );
        }

        #[rstest]
        #[case('?')]
        #[case('😀')]
        fn invalid_char_returns_err_with_original(#[case] c: char) {
            assert!(RegisterName::try_from(c).is_err());
        }
    }

    mod from_register_name_for_char {
        use super::*;

        #[rstest]
        fn unnamed_gives_double_quote() {
            assert_eq!(char::from(RegisterName::Unnamed), '"');
        }

        #[rstest]
        fn black_hole_gives_underscore() {
            assert_eq!(char::from(RegisterName::BlackHole), '_');
        }

        #[rstest]
        fn clipboard_gives_plus() {
            assert_eq!(char::from(RegisterName::Clipboard), '+');
        }

        #[rstest]
        fn selection_gives_star() {
            assert_eq!(char::from(RegisterName::Selection), '*');
        }

        #[rstest]
        fn yank_gives_zero() {
            assert_eq!(char::from(RegisterName::Yank), '0');
        }

        #[rstest]
        #[case('a')]
        fn named_roundtrips(#[case] c: char) -> Result<(), RegisterError> {
            let r = RegisterName::try_from(c)?;

            assert_eq!(char::from(r), c);

            Ok(())
        }

        #[rstest]
        #[case(1u8, '1')]
        #[case(9u8, '9')]
        fn numbered_roundtrips(#[case] n: u8, #[case] expected: char) {
            assert_eq!(char::from(RegisterName::Numbered(RegIndex(n))), expected);
        }
    }

    mod next_numbered {
        use super::*;

        #[rstest]
        fn advances_within_range() -> Result<(), RegisterError> {
            let r = RegisterName::try_from('1')?;

            assert_eq!(r.next_numbered(), Some(RegisterName::try_from('2')?));

            Ok(())
        }

        #[rstest]
        fn eight_advances_to_nine() -> Result<(), RegisterError> {
            let r = RegisterName::try_from('8')?;

            assert_eq!(r.next_numbered(), Some(RegisterName::try_from('9')?));

            Ok(())
        }

        #[rstest]
        fn nine_returns_none() -> Result<(), RegisterError> {
            let r = RegisterName::try_from('9')?;

            assert!(r.next_numbered().is_none());

            Ok(())
        }

        #[rstest]
        fn non_numbered_returns_none() {
            assert!(RegisterName::Unnamed.next_numbered().is_none());
            assert!(RegisterName::Yank.next_numbered().is_none());
        }
    }
}
