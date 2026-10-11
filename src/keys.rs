//! Platform-neutral key model and display labels.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Ctrl,
    Shift,
    Alt,
    AltGr,
    Super,
    /// A character key, identified by its unshifted base character (lowercase).
    Char(char),
    Space,
    F(u8),
    Enter,
    Esc,
    Tab,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    CapsLock,
    NumLock,
    ScrollLock,
    PrintScreen,
    Pause,
    Menu,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModifierStyle {
    #[default]
    Text,
    Symbols,
}

/// Every non-ASCII character used by [`Key::symbol_label`].
pub const SYMBOL_CHARS: &str = "⌃⇧⌥ʳ❖␣⏎⎋⇥⌫⌦⇱⇲⇞⇟⇪";

impl Key {
    pub fn is_modifier(self) -> bool {
        matches!(
            self,
            Key::Ctrl | Key::Shift | Key::Alt | Key::AltGr | Key::Super
        )
    }

    pub fn is_function(self) -> bool {
        matches!(self, Key::F(_))
    }

    /// Navigation / editing keys that are worth showing on their own.
    pub fn is_special(self) -> bool {
        !self.is_modifier() && !self.is_function() && !matches!(self, Key::Char(_) | Key::Space)
    }

    /// Text label, used when symbols are off or a glyph is missing from the font.
    pub fn text_label(self) -> String {
        let s = match self {
            Key::Ctrl => "Ctrl",
            Key::Shift => "Shift",
            Key::Alt => "Alt",
            Key::AltGr => "AltGr",
            Key::Super => {
                if cfg!(windows) {
                    "Win"
                } else {
                    "Super"
                }
            }
            Key::Char(c) => return c.to_uppercase().collect(),
            Key::Space => "Space",
            Key::F(n) => return format!("F{n}"),
            Key::Enter => "Enter",
            Key::Esc => "Esc",
            Key::Tab => "Tab",
            Key::Backspace => "Backspace",
            Key::Delete => "Del",
            Key::Insert => "Ins",
            Key::Home => "Home",
            Key::End => "End",
            Key::PageUp => "PgUp",
            Key::PageDown => "PgDn",
            Key::Up => "↑",
            Key::Down => "↓",
            Key::Left => "←",
            Key::Right => "→",
            Key::CapsLock => "Caps Lock",
            Key::NumLock => "Num Lock",
            Key::ScrollLock => "Scroll Lock",
            Key::PrintScreen => "PrtSc",
            Key::Pause => "Pause",
            Key::Menu => "Menu",
        };
        s.to_owned()
    }

    /// Compact symbol label, if this key has one.
    pub fn symbol_label(self) -> Option<&'static str> {
        Some(match self {
            Key::Ctrl => "⌃",
            Key::Shift => "⇧",
            Key::Alt => "⌥",
            Key::AltGr => "⌥ʳ",
            Key::Super => "❖",
            Key::Space => "␣",
            Key::Enter => "⏎",
            Key::Esc => "⎋",
            Key::Tab => "⇥",
            Key::Backspace => "⌫",
            Key::Delete => "⌦",
            Key::Home => "⇱",
            Key::End => "⇲",
            Key::PageUp => "⇞",
            Key::PageDown => "⇟",
            Key::CapsLock => "⇪",
            _ => return None,
        })
    }
}

/// Character produced by a base character with Shift held, on a US layout.
/// Used where the platform backend cannot tell us the real typed text.
pub fn us_shifted(c: char) -> char {
    match c {
        '1' => '!',
        '2' => '@',
        '3' => '#',
        '4' => '$',
        '5' => '%',
        '6' => '^',
        '7' => '&',
        '8' => '*',
        '9' => '(',
        '0' => ')',
        '-' => '_',
        '=' => '+',
        '[' => '{',
        ']' => '}',
        '\\' => '|',
        ';' => ':',
        '\'' => '"',
        ',' => '<',
        '.' => '>',
        '/' => '?',
        '`' => '~',
        c => c.to_ascii_uppercase(),
    }
}
