//! User settings, persisted as JSON in the platform config dir.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::keys::ModifierStyle;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    /// Only shortcuts (Ctrl/Alt/Super + key) plus the enabled special/function keys.
    #[default]
    CombosOnly,
    /// Everything, with plain typing merged into one running text bubble.
    AllKeys,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyStyle {
    /// One pill per shortcut: "Ctrl + Shift + T".
    #[default]
    Pill,
    /// Each key drawn as its own keycap.
    Keycaps,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    Center,
    MiddleRight,
    BottomLeft,
    #[default]
    BottomCenter,
    BottomRight,
}

impl Anchor {
    pub const ALL: [Anchor; 9] = [
        Anchor::TopLeft,
        Anchor::TopCenter,
        Anchor::TopRight,
        Anchor::MiddleLeft,
        Anchor::Center,
        Anchor::MiddleRight,
        Anchor::BottomLeft,
        Anchor::BottomCenter,
        Anchor::BottomRight,
    ];

    /// Horizontal and vertical alignment: 0 = start, 1 = middle, 2 = end.
    pub fn align(self) -> (u8, u8) {
        let i = Self::ALL.iter().position(|a| *a == self).unwrap_or(7) as u8;
        (i % 3, i / 3)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeName {
    #[default]
    Dark,
    Light,
    Neon,
    Minimal,
    Solarized,
    Custom,
}

impl ThemeName {
    pub const ALL: [ThemeName; 6] = [
        ThemeName::Dark,
        ThemeName::Light,
        ThemeName::Neon,
        ThemeName::Minimal,
        ThemeName::Solarized,
        ThemeName::Custom,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ThemeName::Dark => "Dark",
            ThemeName::Light => "Light",
            ThemeName::Neon => "Neon",
            ThemeName::Minimal => "Minimal",
            ThemeName::Solarized => "Solarized",
            ThemeName::Custom => "Custom",
        }
    }
}

/// Colors are sRGBA, unpremultiplied.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub text: [u8; 4],
    pub background: [u8; 4],
    pub border: [u8; 4],
    /// Color of the "×N" repeat counter.
    pub accent: [u8; 4],
    pub border_width: f32,
    pub corner_radius: f32,
    pub shadow: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::builtin(ThemeName::Dark)
    }
}

impl Theme {
    pub fn builtin(name: ThemeName) -> Theme {
        let t = |text, background, border, accent, border_width, corner_radius, shadow| Theme {
            text,
            background,
            border,
            accent,
            border_width,
            corner_radius,
            shadow,
        };
        match name {
            ThemeName::Dark | ThemeName::Custom => t(
                [255, 255, 255, 255],
                [24, 24, 27, 225],
                [255, 255, 255, 40],
                [250, 204, 21, 255],
                1.0,
                12.0,
                true,
            ),
            ThemeName::Light => t(
                [24, 24, 27, 255],
                [250, 250, 250, 235],
                [0, 0, 0, 45],
                [37, 99, 235, 255],
                1.0,
                12.0,
                true,
            ),
            ThemeName::Neon => t(
                [94, 234, 212, 255],
                [10, 10, 20, 225],
                [236, 72, 153, 255],
                [236, 72, 153, 255],
                2.0,
                10.0,
                true,
            ),
            ThemeName::Minimal => t(
                [255, 255, 255, 255],
                [0, 0, 0, 150],
                [0, 0, 0, 0],
                [200, 200, 200, 255],
                0.0,
                6.0,
                false,
            ),
            ThemeName::Solarized => t(
                [238, 232, 213, 255],
                [0, 43, 54, 230],
                [88, 110, 117, 255],
                [181, 137, 0, 255],
                1.0,
                10.0,
                true,
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub mode: Mode,
    pub show_function_keys: bool,
    pub show_special_keys: bool,
    /// Show a modifier that was tapped on its own (e.g. a lone Shift).
    pub show_lone_modifiers: bool,
    pub show_mouse: bool,

    pub theme: ThemeName,
    pub custom_theme: Theme,
    pub key_style: KeyStyle,
    pub modifier_style: ModifierStyle,
    pub font_size: f32,
    /// Overall opacity multiplier, 0.1..=1.
    pub opacity: f32,

    pub duration_ms: u64,
    pub fade_ms: u64,
    pub max_bubbles: usize,

    pub anchor: Anchor,
    /// Distance from the anchored screen edges, in points.
    pub offset_x: f32,
    pub offset_y: f32,
    /// Index into the monitor list (0 = first).
    pub monitor: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            mode: Mode::CombosOnly,
            show_function_keys: true,
            show_special_keys: true,
            show_lone_modifiers: false,
            show_mouse: false,
            theme: ThemeName::Dark,
            custom_theme: Theme::default(),
            key_style: KeyStyle::Pill,
            modifier_style: ModifierStyle::Text,
            font_size: 32.0,
            opacity: 1.0,
            duration_ms: 1500,
            fade_ms: 400,
            max_bubbles: 3,
            anchor: Anchor::BottomCenter,
            offset_x: 40.0,
            offset_y: 80.0,
            monitor: 0,
        }
    }
}

impl Config {
    pub fn theme(&self) -> Theme {
        match self.theme {
            ThemeName::Custom => self.custom_theme.clone(),
            name => Theme::builtin(name),
        }
    }

    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("keyshow").join("config.json"))
    }

    /// Loads the config. Returns `(config, existed)`; a missing or broken file yields defaults.
    pub fn load() -> (Config, bool) {
        let Some(path) = Self::path() else {
            return (Config::default(), false);
        };
        match std::fs::read_to_string(&path) {
            Ok(s) => match serde_json::from_str(&s) {
                Ok(cfg) => (cfg, true),
                Err(e) => {
                    eprintln!("keyshow: ignoring invalid {}: {e}", path.display());
                    (Config::default(), true)
                }
            },
            Err(_) => (Config::default(), false),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip() {
        let mut cfg = Config {
            mode: Mode::AllKeys,
            theme: ThemeName::Custom,
            anchor: Anchor::TopRight,
            ..Config::default()
        };
        cfg.custom_theme.text = [1, 2, 3, 4];
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let cfg: Config = serde_json::from_str(r#"{"font_size": 50.0}"#).unwrap();
        assert_eq!(cfg.font_size, 50.0);
        assert_eq!(cfg.mode, Mode::CombosOnly);
    }

    #[test]
    fn anchor_alignment() {
        assert_eq!(Anchor::TopLeft.align(), (0, 0));
        assert_eq!(Anchor::BottomCenter.align(), (1, 2));
        assert_eq!(Anchor::MiddleRight.align(), (2, 1));
    }
}
