//! Vim-style key sequence parsing and serialization.

// TODO: make real error types instead of using String

use color_eyre::Result;
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
/// ```no_run
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
    let mut chars = raw.chars();

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
/// ```no_run
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
#[must_use]
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
        k if k.starts_with('f') && k[1..].parse::<u8>().is_ok_and(|n| n >= 1) => KeyCode::F(
            k[1..]
                .parse()
                .unwrap_or_else(|_| unreachable!("guard ensures parse succeeds")),
        ),
        c if c.chars().count() == 1 => {
            let mut c = c
                .chars()
                .next()
                .unwrap_or_else(|| unreachable!("guard ensures exactly one char"));
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
    use rstest::rstest;

    mod parse_key_event {
        use super::*;

        #[rstest]
        #[case("a", KeyCode::Char('a'))]
        #[case("enter", KeyCode::Enter)]
        #[case("esc", KeyCode::Esc)]
        fn simple_chars_and_named_keys(
            #[case] key: &str,
            #[case] code: KeyCode,
        ) -> Result<(), String> {
            assert_eq!(
                parse_key_event(key)?,
                KeyEvent::new(code, KeyModifiers::empty())
            );

            Ok(())
        }

        #[rstest]
        #[case("c-a", KeyCode::Char('a'), KeyModifiers::CONTROL)]
        #[case("a-enter", KeyCode::Enter, KeyModifiers::ALT)]
        #[case("m-x", KeyCode::Char('x'), KeyModifiers::ALT)]
        #[case("s-esc", KeyCode::Esc, KeyModifiers::SHIFT)]
        fn single_modifier(
            #[case] key: &str,
            #[case] code: KeyCode,
            #[case] modifier: KeyModifiers,
        ) -> Result<(), String> {
            assert_eq!(parse_key_event(key)?, KeyEvent::new(code, modifier));

            Ok(())
        }

        #[rstest]
        #[case("C-A", KeyCode::Char('a'), KeyModifiers::CONTROL)]
        #[case("A-Enter", KeyCode::Enter, KeyModifiers::ALT)]
        fn case_insensitive(
            #[case] key: &str,
            #[case] code: KeyCode,
            #[case] modifier: KeyModifiers,
        ) -> Result<(), String> {
            assert_eq!(parse_key_event(key)?, KeyEvent::new(code, modifier));

            Ok(())
        }

        #[rstest]
        #[case("f1", 1u8)]
        #[case("f12", 12u8)]
        fn f_keys(#[case] key: &str, #[case] n: u8) -> Result<(), String> {
            assert_eq!(
                parse_key_event(key)?,
                KeyEvent::new(KeyCode::F(n), KeyModifiers::NONE)
            );

            Ok(())
        }

        #[rstest]
        #[case("invalid-key")]
        #[case("ctrl-invalid-key")]
        fn invalid_keys_return_error(#[case] input: &str) {
            assert!(parse_key_event(input).is_err());
        }

        #[rstest]
        #[case("c-a-a", KeyCode::Char('a'), KeyModifiers::CONTROL | KeyModifiers::ALT)]
        #[case("c-s-enter", KeyCode::Enter, KeyModifiers::CONTROL | KeyModifiers::SHIFT)]
        fn stacked_modifiers(
            #[case] key: &str,
            #[case] code: KeyCode,
            #[case] modifiers: KeyModifiers,
        ) -> Result<(), String> {
            assert_eq!(parse_key_event(key)?, KeyEvent::new(code, modifiers));

            Ok(())
        }

        #[test]
        fn backtab_inserts_shift() -> Result<(), String> {
            assert_eq!(
                parse_key_event("backtab")?,
                KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)
            );

            Ok(())
        }

        #[test]
        fn shift_prefix_uppercases_char() -> Result<(), String> {
            assert_eq!(
                parse_key_event("s-g")?,
                KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT)
            );

            Ok(())
        }

        #[test]
        fn f0_is_rejected() {
            assert!(parse_key_event("f0").is_err());
        }
    }

    mod key_to_string {
        use super::*;

        #[rstest]
        #[case(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL | KeyModifiers::ALT), "<C-M-a>")]
        #[case(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE), "j")]
        #[case(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), "<Esc>")]
        #[case(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), "<CR>")]
        #[case(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE), "<BS>")]
        #[case(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE), "<Tab>")]
        #[case(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE), "<Del>")]
        fn formats_correctly(#[case] event: KeyEvent, #[case] expected: &str) {
            assert_eq!(key_event_to_string(&event), expected);
        }

        #[test]
        fn shift_char_normalizes_to_uppercase() {
            let lower = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::SHIFT);
            assert_eq!(key_event_to_string(&lower), "<S-G>");

            let upper = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
            assert_eq!(key_event_to_string(&upper), "<S-G>");
        }
    }

    mod key_sequence {
        use super::*;

        #[rstest]
        #[case("")]
        #[case("<C-d")]
        #[case("<nope>")]
        fn errors(#[case] input: &str) {
            assert!(parse_key_sequence(input).is_err());
        }

        #[test]
        fn round_trip() -> Result<(), String> {
            let seq = parse_key_sequence("<C-w>j")?;

            assert_eq!(seq.len(), 2);
            assert_eq!(key_event_to_string(&seq[0]), "<C-w>");
            assert_eq!(key_event_to_string(&seq[1]), "j");

            Ok(())
        }

        #[test]
        fn two_consecutive_angle_bracket_specs() -> Result<(), String> {
            let seq = parse_key_sequence("<C-w><C-j>")?;

            assert_eq!(seq.len(), 2);
            assert_eq!(
                seq[0],
                KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL)
            );
            assert_eq!(
                seq[1],
                KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL)
            );

            Ok(())
        }

        #[test]
        fn space_key_parses_and_round_trips() -> Result<(), String> {
            let seq = parse_key_sequence("<Space>")?;

            assert_eq!(seq, [KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)]);
            assert_eq!(key_event_to_string(&seq[0]), "<Space>");

            Ok(())
        }

        #[test]
        fn bare_and_bracketed_mixed_sequence() -> Result<(), String> {
            let seq = parse_key_sequence("j<Down>")?;

            assert_eq!(seq.len(), 2);
            assert_eq!(
                seq[0],
                KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)
            );
            assert_eq!(seq[1], KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

            Ok(())
        }

        #[test]
        fn shift_char_round_trips() -> Result<(), String> {
            let upper = KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT);
            let s = key_event_to_string(&upper);

            assert_eq!(s, "<S-G>");
            assert_eq!(&parse_key_sequence(&s)?[0], &upper);

            Ok(())
        }
    }

    mod rare_key_names {
        use super::*;
        use crossterm::event::{MediaKeyCode, ModifierKeyCode};

        // Every entry corresponds to a match arm in parse_key_code_with_modifiers.
        // Arms that accept two spellings (e.g. "bs"/"backspace") list both so that
        // a deleted arm fails on its own dedicated test case.
        #[rstest]
        #[case::bs("bs", KeyCode::Backspace)]
        #[case::backspace("backspace", KeyCode::Backspace)]
        #[case::del("del", KeyCode::Delete)]
        #[case::delete("delete", KeyCode::Delete)]
        #[case::ins("ins", KeyCode::Insert)]
        #[case::insert("insert", KeyCode::Insert)]
        #[case::lt("lt", KeyCode::Char('<'))]
        #[case::bar("bar", KeyCode::Char('|'))]
        #[case::nul("nul", KeyCode::Null)]
        #[case::null("null", KeyCode::Null)]
        #[case::capslock("capslock", KeyCode::CapsLock)]
        #[case::scrolllock("scrolllock", KeyCode::ScrollLock)]
        #[case::numlock("numlock", KeyCode::NumLock)]
        #[case::printscreen("printscreen", KeyCode::PrintScreen)]
        #[case::print("print", KeyCode::PrintScreen)]
        #[case::pause("pause", KeyCode::Pause)]
        #[case::menu("menu", KeyCode::Menu)]
        #[case::keypadbegin("keypadbegin", KeyCode::KeypadBegin)]
        #[case::play("play", KeyCode::Media(MediaKeyCode::Play))]
        #[case::mediapause("mediapause", KeyCode::Media(MediaKeyCode::Pause))]
        #[case::playpause("playpause", KeyCode::Media(MediaKeyCode::PlayPause))]
        #[case::reverse("reverse", KeyCode::Media(MediaKeyCode::Reverse))]
        #[case::stop("stop", KeyCode::Media(MediaKeyCode::Stop))]
        #[case::fastforward("fastforward", KeyCode::Media(MediaKeyCode::FastForward))]
        #[case::rewind("rewind", KeyCode::Media(MediaKeyCode::Rewind))]
        #[case::tracknext("tracknext", KeyCode::Media(MediaKeyCode::TrackNext))]
        #[case::trackprev("trackprev", KeyCode::Media(MediaKeyCode::TrackPrevious))]
        #[case::record("record", KeyCode::Media(MediaKeyCode::Record))]
        #[case::lowervolume("lowervolume", KeyCode::Media(MediaKeyCode::LowerVolume))]
        #[case::raisevolume("raisevolume", KeyCode::Media(MediaKeyCode::RaiseVolume))]
        #[case::mutevolume("mutevolume", KeyCode::Media(MediaKeyCode::MuteVolume))]
        #[case::leftshift("leftshift", KeyCode::Modifier(ModifierKeyCode::LeftShift))]
        #[case::rightshift("rightshift", KeyCode::Modifier(ModifierKeyCode::RightShift))]
        #[case::leftctrl("leftctrl", KeyCode::Modifier(ModifierKeyCode::LeftControl))]
        #[case::leftcontrol("leftcontrol", KeyCode::Modifier(ModifierKeyCode::LeftControl))]
        #[case::rightctrl("rightctrl", KeyCode::Modifier(ModifierKeyCode::RightControl))]
        #[case::rightcontrol("rightcontrol", KeyCode::Modifier(ModifierKeyCode::RightControl))]
        #[case::leftalt("leftalt", KeyCode::Modifier(ModifierKeyCode::LeftAlt))]
        #[case::rightalt("rightalt", KeyCode::Modifier(ModifierKeyCode::RightAlt))]
        #[case::leftsuper("leftsuper", KeyCode::Modifier(ModifierKeyCode::LeftSuper))]
        #[case::rightsuper("rightsuper", KeyCode::Modifier(ModifierKeyCode::RightSuper))]
        #[case::lefthyper("lefthyper", KeyCode::Modifier(ModifierKeyCode::LeftHyper))]
        #[case::righthyper("righthyper", KeyCode::Modifier(ModifierKeyCode::RightHyper))]
        #[case::leftmeta("leftmeta", KeyCode::Modifier(ModifierKeyCode::LeftMeta))]
        #[case::rightmeta("rightmeta", KeyCode::Modifier(ModifierKeyCode::RightMeta))]
        #[case::isolevel3shift(
            "isolevel3shift",
            KeyCode::Modifier(ModifierKeyCode::IsoLevel3Shift)
        )]
        #[case::isolevel5shift(
            "isolevel5shift",
            KeyCode::Modifier(ModifierKeyCode::IsoLevel5Shift)
        )]
        fn named_special_key_parses_to_expected_code(
            #[case] name: &str,
            #[case] expected_code: KeyCode,
        ) -> Result<(), String> {
            let key = format!("<{name}>");
            let events = parse_key_sequence(&key)?;

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].code, expected_code);

            Ok(())
        }
    }
}
