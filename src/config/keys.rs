//! Vim-style key sequence parsing and serialization.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MediaKeyCode, ModifierKeyCode};

/// Parses a Vim-style key sequence into a [`Vec<KeyEvent>`].
///
/// Each `<…>` group is a single key spec; any character outside `<…>` is a
/// literal key press with no modifiers. Inside `<…>`, modifier prefixes
/// `C-` (Ctrl), `M-`/`A-` (Alt), and `S-` (Shift) may be stacked before
/// the key name. Key names are case-insensitive.
///
/// # Errors
///
/// Returns an `Err` string if the input is empty, contains an unclosed `<`,
/// or references an unrecognised key name.
///
/// # Examples
///
/// ```
/// use dps::config::keys::parse_key_sequence;
/// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
///
/// // plain character
/// let seq = parse_key_sequence("j").unwrap();
/// assert_eq!(seq, [KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)]);
///
/// // named special key
/// let seq = parse_key_sequence("<Esc>").unwrap();
/// assert_eq!(seq, [KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)]);
///
/// // modifier combo
/// let seq = parse_key_sequence("<C-d>").unwrap();
/// assert_eq!(seq, [KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)]);
///
/// // two-key chord
/// let seq = parse_key_sequence("gg").unwrap();
/// assert_eq!(seq, [
///     KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
///     KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
/// ]);
///
/// // mixed chord: <C-w> then j
/// let seq = parse_key_sequence("<C-w>j").unwrap();
/// assert_eq!(seq, [
///     KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
///     KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
/// ]);
///
/// // errors
/// assert!(parse_key_sequence("").is_err());
/// assert!(parse_key_sequence("<C-d").is_err());
/// assert!(parse_key_sequence("<nope>").is_err());
/// ```
pub fn parse_key_sequence(raw: &str) -> Result<Vec<KeyEvent>, String> {
    if raw.is_empty() {
        return Err("Empty key sequence".to_owned());
    }

    let mut keys = Vec::new();
    let mut chars = raw.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut spec = String::new();
            let mut closed = false;

            for inner in chars.by_ref() {
                if inner == '>' {
                    closed = true;
                    break;
                }
                spec.push(inner);
            }

            if !closed {
                return Err(format!("Unclosed `<` in `{raw}`"));
            }

            keys.push(parse_key_event(&spec)?);
        } else {
            keys.push(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
    }

    Ok(keys)
}

/// Converts a [`KeyEvent`] to its Vim-style string representation.
///
/// Plain printable characters without modifiers are returned bare.
/// Special keys and any key with a modifier are wrapped in `<…>`.
/// Modifier order: `C-` (Ctrl) → `M-` (Alt) → `S-` (Shift).
///
/// This is the inverse of [`parse_key_sequence`] for single-key sequences.
///
/// # Examples
///
/// ```
/// use dps::config::keys::{key_event_to_string, parse_key_sequence};
/// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
///
/// // plain character — no brackets
/// assert_eq!(
///     key_event_to_string(&KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
///     "j"
/// );
///
/// // special key — bracketed
/// assert_eq!(
///     key_event_to_string(&KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
///     "<Esc>"
/// );
///
/// // Ctrl modifier
/// assert_eq!(
///     key_event_to_string(&KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)),
///     "<C-d>"
/// );
///
/// // F key
/// assert_eq!(
///     key_event_to_string(&KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE)),
///     "<F5>"
/// );
///
/// // BackTab always carries the S- prefix
/// assert_eq!(
///     key_event_to_string(&KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE)),
///     "<S-Tab>"
/// );
///
/// // round-trip for a modifier combo
/// let event = &parse_key_sequence("<C-w>").unwrap()[0];
/// assert_eq!(key_event_to_string(event), "<C-w>");
/// ```
pub fn key_event_to_string(key_event: &KeyEvent) -> String {
    let (key_str, is_special): (String, bool) = match key_event.code {
        KeyCode::Backspace => ("BS".into(), true),
        KeyCode::Enter => ("CR".into(), true),
        KeyCode::Left => ("Left".into(), true),
        KeyCode::Right => ("Right".into(), true),
        KeyCode::Up => ("Up".into(), true),
        KeyCode::Down => ("Down".into(), true),
        KeyCode::Home => ("Home".into(), true),
        KeyCode::End => ("End".into(), true),
        KeyCode::PageUp => ("PageUp".into(), true),
        KeyCode::PageDown => ("PageDown".into(), true),
        KeyCode::Tab => ("Tab".into(), true),
        KeyCode::BackTab => ("S-Tab".into(), true),
        KeyCode::Delete => ("Del".into(), true),
        KeyCode::Insert => ("Ins".into(), true),
        KeyCode::Esc => ("Esc".into(), true),
        KeyCode::Null => ("NUL".into(), true),
        KeyCode::CapsLock => ("CapsLock".into(), true),
        KeyCode::ScrollLock => ("ScrollLock".into(), true),
        KeyCode::NumLock => ("NumLock".into(), true),
        KeyCode::PrintScreen => ("PrintScreen".into(), true),
        KeyCode::Pause => ("Pause".into(), true),
        KeyCode::Menu => ("Menu".into(), true),
        KeyCode::KeypadBegin => ("KeypadBegin".into(), true),
        KeyCode::Char(' ') => ("Space".into(), true),
        KeyCode::Char('<') => ("lt".into(), true),
        KeyCode::Char('|') => ("Bar".into(), true),
        KeyCode::Char(c) => {
            let c = if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                c.to_ascii_uppercase()
            } else {
                c
            };
            (c.to_string(), false)
        }
        KeyCode::F(n) => (format!("F{n}"), true),
        KeyCode::Media(m) => (
            match m {
                MediaKeyCode::Play => "Play",
                MediaKeyCode::Pause => "MediaPause",
                MediaKeyCode::PlayPause => "PlayPause",
                MediaKeyCode::Reverse => "Reverse",
                MediaKeyCode::Stop => "Stop",
                MediaKeyCode::FastForward => "FastForward",
                MediaKeyCode::Rewind => "Rewind",
                MediaKeyCode::TrackNext => "TrackNext",
                MediaKeyCode::TrackPrevious => "TrackPrev",
                MediaKeyCode::Record => "Record",
                MediaKeyCode::LowerVolume => "LowerVolume",
                MediaKeyCode::RaiseVolume => "RaiseVolume",
                MediaKeyCode::MuteVolume => "MuteVolume",
            }
            .into(),
            true,
        ),
        KeyCode::Modifier(m) => (
            match m {
                ModifierKeyCode::LeftShift => "LeftShift",
                ModifierKeyCode::RightShift => "RightShift",
                ModifierKeyCode::LeftControl => "LeftCtrl",
                ModifierKeyCode::RightControl => "RightCtrl",
                ModifierKeyCode::LeftAlt => "LeftAlt",
                ModifierKeyCode::RightAlt => "RightAlt",
                ModifierKeyCode::LeftSuper => "LeftSuper",
                ModifierKeyCode::RightSuper => "RightSuper",
                ModifierKeyCode::LeftHyper => "LeftHyper",
                ModifierKeyCode::RightHyper => "RightHyper",
                ModifierKeyCode::LeftMeta => "LeftMeta",
                ModifierKeyCode::RightMeta => "RightMeta",
                ModifierKeyCode::IsoLevel3Shift => "IsoLevel3Shift",
                ModifierKeyCode::IsoLevel5Shift => "IsoLevel5Shift",
            }
            .into(),
            true,
        ),
    };

    let mut mods = Vec::with_capacity(3);
    if key_event.modifiers.intersects(KeyModifiers::CONTROL) {
        mods.push("C");
    }
    if key_event.modifiers.intersects(KeyModifiers::ALT) {
        mods.push("M");
    }
    if key_event.modifiers.intersects(KeyModifiers::SHIFT) {
        mods.push("S");
    }

    if mods.is_empty() && !is_special {
        key_str
    } else if mods.is_empty() {
        format!("<{key_str}>")
    } else {
        format!("<{}-{key_str}>", mods.join("-"))
    }
}

/// Parses the content of a `<…>` spec (without the angle brackets) into a
/// [`KeyEvent`]. The input is lowercased before modifier extraction so that
/// `<C-A>` and `<c-a>` are equivalent.
fn parse_key_event(raw: &str) -> Result<KeyEvent, String> {
    let raw_lower = raw.to_ascii_lowercase();
    let (remaining, modifiers) = extract_modifiers(&raw_lower);
    parse_key_code_with_modifiers(remaining, modifiers)
}

/// Strips leading Vim modifier prefixes from `raw` and returns the
/// remaining key name together with the accumulated [`KeyModifiers`].
/// Recognises `c-` (Ctrl), `m-`/`a-` (Alt), and `s-` (Shift).
fn extract_modifiers(raw: &str) -> (&str, KeyModifiers) {
    let mut modifiers = KeyModifiers::empty();
    let mut current = raw;

    loop {
        match current {
            rest if rest.starts_with("c-") => {
                modifiers.insert(KeyModifiers::CONTROL);
                current = &rest[2..];
            }
            rest if rest.starts_with("m-") || rest.starts_with("a-") => {
                modifiers.insert(KeyModifiers::ALT);
                current = &rest[2..];
            }
            rest if rest.starts_with("s-") => {
                modifiers.insert(KeyModifiers::SHIFT);
                current = &rest[2..];
            }
            _ => break,
        }
    }

    (current, modifiers)
}

/// Maps a lowercase key name to a [`KeyCode`] and constructs the final
/// [`KeyEvent`]. For single-character keys, applies ASCII uppercasing when
/// [`KeyModifiers::SHIFT`] is present.
fn parse_key_code_with_modifiers(
    raw: &str,
    mut modifiers: KeyModifiers,
) -> Result<KeyEvent, String> {
    let c = match raw {
        "esc" | "escape" => KeyCode::Esc,
        "cr" | "enter" | "return" => KeyCode::Enter,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "bs" | "backspace" => KeyCode::Backspace,
        "del" | "delete" => KeyCode::Delete,
        "ins" | "insert" => KeyCode::Insert,
        "tab" => KeyCode::Tab,
        "backtab" => {
            modifiers.insert(KeyModifiers::SHIFT);
            KeyCode::BackTab
        }
        "space" => KeyCode::Char(' '),
        "lt" => KeyCode::Char('<'),
        "bar" => KeyCode::Char('|'),
        "nul" | "null" => KeyCode::Null,
        "capslock" => KeyCode::CapsLock,
        "scrolllock" => KeyCode::ScrollLock,
        "numlock" => KeyCode::NumLock,
        "printscreen" | "print" => KeyCode::PrintScreen,
        "pause" => KeyCode::Pause,
        "menu" => KeyCode::Menu,
        "keypadbegin" => KeyCode::KeypadBegin,
        "play" => KeyCode::Media(MediaKeyCode::Play),
        "mediapause" => KeyCode::Media(MediaKeyCode::Pause),
        "playpause" => KeyCode::Media(MediaKeyCode::PlayPause),
        "reverse" => KeyCode::Media(MediaKeyCode::Reverse),
        "stop" => KeyCode::Media(MediaKeyCode::Stop),
        "fastforward" => KeyCode::Media(MediaKeyCode::FastForward),
        "rewind" => KeyCode::Media(MediaKeyCode::Rewind),
        "tracknext" => KeyCode::Media(MediaKeyCode::TrackNext),
        "trackprev" => KeyCode::Media(MediaKeyCode::TrackPrevious),
        "record" => KeyCode::Media(MediaKeyCode::Record),
        "lowervolume" => KeyCode::Media(MediaKeyCode::LowerVolume),
        "raisevolume" => KeyCode::Media(MediaKeyCode::RaiseVolume),
        "mutevolume" => KeyCode::Media(MediaKeyCode::MuteVolume),
        "leftshift" => KeyCode::Modifier(ModifierKeyCode::LeftShift),
        "rightshift" => KeyCode::Modifier(ModifierKeyCode::RightShift),
        "leftctrl" | "leftcontrol" => KeyCode::Modifier(ModifierKeyCode::LeftControl),
        "rightctrl" | "rightcontrol" => KeyCode::Modifier(ModifierKeyCode::RightControl),
        "leftalt" => KeyCode::Modifier(ModifierKeyCode::LeftAlt),
        "rightalt" => KeyCode::Modifier(ModifierKeyCode::RightAlt),
        "leftsuper" => KeyCode::Modifier(ModifierKeyCode::LeftSuper),
        "rightsuper" => KeyCode::Modifier(ModifierKeyCode::RightSuper),
        "lefthyper" => KeyCode::Modifier(ModifierKeyCode::LeftHyper),
        "righthyper" => KeyCode::Modifier(ModifierKeyCode::RightHyper),
        "leftmeta" => KeyCode::Modifier(ModifierKeyCode::LeftMeta),
        "rightmeta" => KeyCode::Modifier(ModifierKeyCode::RightMeta),
        "isolevel3shift" => KeyCode::Modifier(ModifierKeyCode::IsoLevel3Shift),
        "isolevel5shift" => KeyCode::Modifier(ModifierKeyCode::IsoLevel5Shift),
        k if k.starts_with('f') && k[1..].parse::<u8>().is_ok_and(|n| n >= 1) => {
            KeyCode::F(k[1..].parse().unwrap())
        }
        c if c.chars().count() == 1 => {
            let mut c = c.chars().next().unwrap();
            if modifiers.contains(KeyModifiers::SHIFT) {
                c = c.to_ascii_uppercase();
            }
            KeyCode::Char(c)
        }
        _ => return Err(format!("Unable to parse `{raw}`")),
    };
    Ok(KeyEvent::new(c, modifiers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn simple_chars_and_named_keys() {
        assert_eq!(
            parse_key_event("a").unwrap(),
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty())
        );
        assert_eq!(
            parse_key_event("enter").unwrap(),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty())
        );
        assert_eq!(
            parse_key_event("esc").unwrap(),
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty())
        );
    }

    #[test]
    fn single_modifier() {
        // Vim short-form prefixes: c- (Ctrl), a-/m- (Alt), s- (Shift).
        assert_eq!(
            parse_key_event("c-a").unwrap(),
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL)
        );
        assert_eq!(
            parse_key_event("a-enter").unwrap(),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT)
        );
        assert_eq!(
            parse_key_event("m-x").unwrap(),
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT)
        );
        assert_eq!(
            parse_key_event("s-esc").unwrap(),
            KeyEvent::new(KeyCode::Esc, KeyModifiers::SHIFT)
        );
    }

    #[test]
    fn stacked_modifiers() {
        assert_eq!(
            parse_key_event("c-a-a").unwrap(),
            KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )
        );
        assert_eq!(
            parse_key_event("c-s-enter").unwrap(),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL | KeyModifiers::SHIFT)
        );
    }

    #[test]
    fn key_event_to_string_formats() {
        assert_eq!(
            key_event_to_string(&KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL | KeyModifiers::ALT
            )),
            "<C-M-a>"
        );
        assert_eq!(
            key_event_to_string(&KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
            "j"
        );
        assert_eq!(
            key_event_to_string(&KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            "<Esc>"
        );
    }

    #[test]
    fn case_insensitive_modifier_prefix() {
        // parse_key_event lowercases input, so C-A and c-a are equivalent.
        assert_eq!(
            parse_key_event("C-A").unwrap(),
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL)
        );
        assert_eq!(
            parse_key_event("A-Enter").unwrap(),
            KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT)
        );
    }

    #[test]
    fn backtab_inserts_shift() {
        assert_eq!(
            parse_key_event("backtab").unwrap(),
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)
        );
    }

    #[test]
    fn f_keys() {
        assert_eq!(
            parse_key_event("f1").unwrap(),
            KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE)
        );
        assert_eq!(
            parse_key_event("f12").unwrap(),
            KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE)
        );
    }

    #[test]
    fn invalid_keys_return_error() {
        assert!(parse_key_event("invalid-key").is_err());
        // "ctrl-" is not a recognised prefix; the whole string is treated as a key name.
        assert!(parse_key_event("ctrl-invalid-key").is_err());
    }

    #[test]
    fn parse_key_sequence_errors() {
        assert!(parse_key_sequence("").is_err());
        assert!(parse_key_sequence("<C-d").is_err());
        assert!(parse_key_sequence("<nope>").is_err());
    }

    #[test]
    fn parse_key_sequence_round_trip() {
        let seq = parse_key_sequence("<C-w>j").unwrap();
        assert_eq!(seq.len(), 2);
        assert_eq!(key_event_to_string(&seq[0]), "<C-w>");
        assert_eq!(key_event_to_string(&seq[1]), "j");
    }

    #[test]
    fn shift_prefix_uppercases_char() {
        assert_eq!(
            parse_key_event("s-g").unwrap(),
            KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT)
        );
    }

    #[test]
    fn f0_is_rejected() {
        assert!(parse_key_event("f0").is_err());
    }

    #[test]
    fn two_consecutive_angle_bracket_specs() {
        let seq = parse_key_sequence("<C-w><C-j>").unwrap();
        assert_eq!(seq.len(), 2);
        assert_eq!(
            seq[0],
            KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL)
        );
        assert_eq!(
            seq[1],
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL)
        );
    }

    #[test]
    fn space_key_parses_and_round_trips() {
        let seq = parse_key_sequence("<Space>").unwrap();
        assert_eq!(seq, [KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)]);
        assert_eq!(key_event_to_string(&seq[0]), "<Space>");
    }

    #[test]
    fn shift_char_normalizes_to_uppercase_and_round_trips() {
        // Lowercase input with SHIFT is normalized to uppercase in the string form.
        let lower = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::SHIFT);
        assert_eq!(key_event_to_string(&lower), "<S-G>");

        // The canonical uppercase form round-trips cleanly.
        let upper = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
        let s = key_event_to_string(&upper);
        assert_eq!(s, "<S-G>");
        assert_eq!(&parse_key_sequence(&s).unwrap()[0], &upper);
    }

    #[test]
    fn special_keys_format_correctly() {
        assert_eq!(
            key_event_to_string(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            "<CR>"
        );
        assert_eq!(
            key_event_to_string(&KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
            "<BS>"
        );
        assert_eq!(
            key_event_to_string(&KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
            "<Tab>"
        );
        assert_eq!(
            key_event_to_string(&KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)),
            "<Del>"
        );
    }

    #[test]
    fn bare_and_bracketed_mixed_sequence() {
        let seq = parse_key_sequence("j<Down>").unwrap();
        assert_eq!(seq.len(), 2);
        assert_eq!(
            seq[0],
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)
        );
        assert_eq!(seq[1], KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
}
