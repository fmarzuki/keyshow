//! Turns raw input events into on-screen bubbles: modifier tracking, display
//! filtering, ×N merging, typing runs, hotkeys and fade timing.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use crate::config::{Config, Mode};
use crate::input::{InputEvent, MouseButton};
use crate::keys::{Key, ModifierStyle, us_shifted};

/// Order in which held modifiers are listed.
const MODIFIER_ORDER: [Key; 5] = [Key::Super, Key::Ctrl, Key::Alt, Key::AltGr, Key::Shift];
/// Longest running text kept in a typing bubble.
const MAX_TYPING_CHARS: usize = 40;
/// A modifier "held" longer than this is assumed stuck (missed release, e.g. after a lock screen).
const STALE_MODIFIER: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, PartialEq)]
pub enum Content {
    Keys(Vec<Key>),
    Text(String),
    Mouse(Vec<Key>, MouseButton),
    Scroll(Vec<Key>),
    Notice(&'static str),
}

#[derive(Clone, Debug)]
pub struct Bubble {
    pub content: Content,
    pub count: u32,
    pub born: Instant,
    pub updated: Instant,
}

impl Bubble {
    /// Display segments. `has_glyphs` decides whether a symbol label can be rendered.
    pub fn parts(
        &self,
        style: ModifierStyle,
        has_glyphs: &mut dyn FnMut(&str) -> bool,
    ) -> Vec<String> {
        let mut label = |k: Key| match (style, k.symbol_label()) {
            (ModifierStyle::Symbols, Some(sym)) if has_glyphs(sym) => sym.to_owned(),
            _ => k.text_label(),
        };
        match &self.content {
            Content::Keys(keys) => keys.iter().map(|k| label(*k)).collect(),
            Content::Text(t) => {
                let n = t.chars().count();
                if n > MAX_TYPING_CHARS {
                    let tail: String = t.chars().skip(n - MAX_TYPING_CHARS).collect();
                    vec![format!("…{tail}")]
                } else {
                    vec![t.clone()]
                }
            }
            Content::Mouse(mods, b) => mods
                .iter()
                .map(|k| label(*k))
                .chain([b.label().to_owned()])
                .collect(),
            Content::Scroll(mods) => mods
                .iter()
                .map(|k| label(*k))
                .chain(["Scroll".to_owned()])
                .collect(),
            Content::Notice(s) => vec![(*s).to_owned()],
        }
    }

    /// Whether to draw the "×N" counter.
    pub fn show_count(&self) -> bool {
        self.count > 1 && !matches!(self.content, Content::Scroll(_) | Content::Text(_))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    TogglePause,
    OpenSettings,
}

#[derive(Default)]
pub struct Engine {
    /// Held modifiers by physical key code.
    held: HashMap<u32, (Key, Instant)>,
    /// Modifiers held when the last modifier went down, if nothing else was pressed since.
    lone: Option<Vec<Key>>,
    caps_lock: bool,
    bubbles: VecDeque<Bubble>,
    pub paused: bool,
}

impl Engine {
    pub fn bubbles(&self) -> impl DoubleEndedIterator<Item = &Bubble> {
        self.bubbles.iter()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.bubbles.is_empty()
    }

    fn mods(&self) -> Vec<Key> {
        MODIFIER_ORDER
            .into_iter()
            .filter(|m| self.held.values().any(|(k, _)| k == m))
            .collect()
    }

    pub fn handle(&mut self, ev: &InputEvent, cfg: &Config, now: Instant) -> Action {
        match ev {
            InputEvent::Key {
                key,
                code,
                pressed,
                repeat,
                text,
            } => self.handle_key(*key, *code, *pressed, *repeat, text.as_deref(), cfg, now),
            InputEvent::Mouse(button) => {
                self.lone = None;
                if cfg.show_mouse && !self.paused {
                    self.push(Content::Mouse(self.mods(), *button), cfg, now);
                }
                Action::None
            }
            InputEvent::Scroll => {
                if cfg.show_mouse && !self.paused {
                    self.push(Content::Scroll(self.mods()), cfg, now);
                }
                Action::None
            }
            InputEvent::Status(_) => Action::None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_key(
        &mut self,
        key: Key,
        code: u32,
        pressed: bool,
        repeat: bool,
        text: Option<&str>,
        cfg: &Config,
        now: Instant,
    ) -> Action {
        if key.is_modifier() {
            if !pressed {
                self.held.remove(&code);
                if let Some(lone) = self.lone.take()
                    && cfg.show_lone_modifiers
                    && !self.paused
                {
                    self.push(Content::Keys(lone), cfg, now);
                }
            } else if !repeat {
                self.held.insert(code, (key, now));
                self.lone = Some(self.mods());
            }
            return Action::None;
        }
        if !pressed {
            return Action::None;
        }
        self.lone = None;
        self.held
            .retain(|_, (_, since)| now.duration_since(*since) < STALE_MODIFIER);
        if key == Key::CapsLock && !repeat {
            self.caps_lock = !self.caps_lock;
        }

        let mods = self.mods();
        if !repeat && mods == [Key::Ctrl, Key::Alt, Key::Shift] {
            match key {
                Key::Char('k') => {
                    self.toggle_pause(cfg, now);
                    return Action::TogglePause;
                }
                Key::Char('s') => return Action::OpenSettings,
                _ => {}
            }
        }
        if self.paused {
            return Action::None;
        }

        let command = mods
            .iter()
            .any(|m| matches!(m, Key::Ctrl | Key::Alt | Key::Super));
        let typing_mode = cfg.mode == Mode::AllKeys;
        let combo = |mut mods: Vec<Key>| {
            mods.push(key);
            Content::Keys(mods)
        };

        if command {
            self.push(combo(mods), cfg, now);
        } else if key.is_function() {
            if cfg.show_function_keys {
                self.push(combo(mods), cfg, now);
            }
        } else if typing_mode && key == Key::Backspace && self.pop_typed_char(cfg, now) {
            // Edited the running text bubble.
        } else if key.is_special() {
            if cfg.show_special_keys {
                self.push(combo(mods), cfg, now);
            }
        } else if typing_mode {
            let typed = match (text, key) {
                (Some(t), _) => Some(t.to_owned()),
                (None, Key::Space) => Some(" ".to_owned()),
                (None, Key::Char(c)) if !mods.contains(&Key::AltGr) => {
                    let shift = mods.contains(&Key::Shift);
                    let upper = if c.is_ascii_alphabetic() {
                        shift != self.caps_lock
                    } else {
                        shift
                    };
                    Some(if upper { us_shifted(c) } else { c }.to_string())
                }
                _ => None,
            };
            match typed {
                Some(t) => self.push_text(&t, cfg, now),
                None => self.push(combo(mods), cfg, now),
            }
        }
        Action::None
    }

    pub fn toggle_pause(&mut self, cfg: &Config, now: Instant) {
        self.paused = !self.paused;
        let msg = if self.paused {
            "Keyshow paused"
        } else {
            "Keyshow resumed"
        };
        self.push(Content::Notice(msg), cfg, now);
    }

    fn fresh(&self, b: &Bubble, cfg: &Config, now: Instant) -> bool {
        now.duration_since(b.updated) < Duration::from_millis(cfg.duration_ms)
    }

    fn push(&mut self, content: Content, cfg: &Config, now: Instant) {
        if let Some(last) = self.bubbles.back()
            && last.content == content
            && self.fresh(last, cfg, now)
        {
            let last = self.bubbles.back_mut().unwrap();
            last.count += 1;
            last.updated = now;
            return;
        }
        self.bubbles.push_back(Bubble {
            content,
            count: 1,
            born: now,
            updated: now,
        });
        while self.bubbles.len() > cfg.max_bubbles.max(1) {
            self.bubbles.pop_front();
        }
    }

    fn push_text(&mut self, t: &str, cfg: &Config, now: Instant) {
        let fresh = self.bubbles.back().is_some_and(|b| self.fresh(b, cfg, now));
        if fresh
            && let Some(Bubble {
                content: Content::Text(s),
                updated,
                ..
            }) = self.bubbles.back_mut()
        {
            s.push_str(t);
            *updated = now;
        } else {
            self.push(Content::Text(t.to_owned()), cfg, now);
        }
    }

    /// Backspace inside a fresh typing bubble removes the last character.
    fn pop_typed_char(&mut self, cfg: &Config, now: Instant) -> bool {
        let fresh = self.bubbles.back().is_some_and(|b| self.fresh(b, cfg, now));
        match self.bubbles.back_mut() {
            Some(Bubble {
                content: Content::Text(s),
                updated,
                ..
            }) if fresh && !s.is_empty() => {
                s.pop();
                *updated = now;
                true
            }
            _ => false,
        }
    }

    /// Drops bubbles that have fully faded out.
    pub fn prune(&mut self, cfg: &Config, now: Instant) {
        let life = Duration::from_millis(cfg.duration_ms + cfg.fade_ms);
        self.bubbles
            .retain(|b| now.duration_since(b.updated) < life);
        while self.bubbles.len() > cfg.max_bubbles.max(1) {
            self.bubbles.pop_front();
        }
    }

    /// Opacity of a bubble in 0..=1, fading out after `duration_ms`.
    pub fn alpha(b: &Bubble, cfg: &Config, now: Instant) -> f32 {
        let age = now.duration_since(b.updated).as_secs_f32() * 1000.0;
        let duration = cfg.duration_ms as f32;
        if age <= duration {
            1.0
        } else if cfg.fade_ms == 0 {
            0.0
        } else {
            (1.0 - (age - duration) / cfg.fade_ms as f32).clamp(0.0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(k: Key, code: u32, pressed: bool) -> InputEvent {
        InputEvent::Key {
            key: k,
            code,
            pressed,
            repeat: false,
            text: None,
        }
    }

    /// Press and release `k`.
    fn tap(e: &mut Engine, cfg: &Config, now: Instant, k: Key) -> Action {
        let a = e.handle(&key(k, 1000, true), cfg, now);
        e.handle(&key(k, 1000, false), cfg, now);
        a
    }

    fn texts(e: &Engine) -> Vec<String> {
        e.bubbles()
            .map(|b| {
                let s = b.parts(ModifierStyle::Text, &mut |_| true).join(" + ");
                if b.show_count() {
                    format!("{s} ×{}", b.count)
                } else {
                    s
                }
            })
            .collect()
    }

    const LCTRL: u32 = 29;
    const LSHIFT: u32 = 42;
    const LALT: u32 = 56;

    #[test]
    fn combo_is_shown_and_plain_typing_is_not_in_combos_mode() {
        let cfg = Config::default();
        let mut e = Engine::default();
        let now = Instant::now();
        tap(&mut e, &cfg, now, Key::Char('a'));
        e.handle(&key(Key::Ctrl, LCTRL, true), &cfg, now);
        e.handle(&key(Key::Shift, LSHIFT, true), &cfg, now);
        tap(&mut e, &cfg, now, Key::Char('t'));
        assert_eq!(texts(&e), ["Ctrl + Shift + T"]);
    }

    #[test]
    fn shift_plus_letter_is_typing_not_a_combo() {
        let cfg = Config::default();
        let mut e = Engine::default();
        let now = Instant::now();
        e.handle(&key(Key::Shift, LSHIFT, true), &cfg, now);
        tap(&mut e, &cfg, now, Key::Char('a'));
        assert!(e.is_empty());
    }

    #[test]
    fn repeated_combo_merges_into_counter() {
        let cfg = Config::default();
        let mut e = Engine::default();
        let now = Instant::now();
        e.handle(&key(Key::Ctrl, LCTRL, true), &cfg, now);
        for i in 0..3 {
            tap(
                &mut e,
                &cfg,
                now + Duration::from_millis(100 * i),
                Key::Char('z'),
            );
        }
        assert_eq!(texts(&e), ["Ctrl + Z ×3"]);
    }

    #[test]
    fn combo_after_expiry_starts_new_bubble() {
        let cfg = Config::default();
        let mut e = Engine::default();
        let now = Instant::now();
        e.handle(&key(Key::Ctrl, LCTRL, true), &cfg, now);
        tap(&mut e, &cfg, now, Key::Char('c'));
        tap(
            &mut e,
            &cfg,
            now + Duration::from_millis(cfg.duration_ms + 1),
            Key::Char('c'),
        );
        assert_eq!(texts(&e), ["Ctrl + C", "Ctrl + C"]);
    }

    #[test]
    fn all_keys_mode_merges_typing_and_handles_shift_and_backspace() {
        let cfg = Config {
            mode: Mode::AllKeys,
            ..Config::default()
        };
        let mut e = Engine::default();
        let now = Instant::now();
        e.handle(&key(Key::Shift, LSHIFT, true), &cfg, now);
        tap(&mut e, &cfg, now, Key::Char('h'));
        e.handle(&key(Key::Shift, LSHIFT, false), &cfg, now);
        for c in ['i', 'x'] {
            tap(&mut e, &cfg, now, Key::Char(c));
        }
        tap(&mut e, &cfg, now, Key::Backspace);
        tap(&mut e, &cfg, now, Key::Space);
        e.handle(&key(Key::Shift, LSHIFT, true), &cfg, now);
        tap(&mut e, &cfg, now, Key::Char('1'));
        assert_eq!(texts(&e), ["Hi !"]);
    }

    #[test]
    fn backend_text_wins_over_us_table() {
        let cfg = Config {
            mode: Mode::AllKeys,
            ..Config::default()
        };
        let mut e = Engine::default();
        let now = Instant::now();
        e.handle(
            &InputEvent::Key {
                key: Key::Char('2'),
                code: 3,
                pressed: true,
                repeat: false,
                text: Some("é".into()),
            },
            &cfg,
            now,
        );
        assert_eq!(texts(&e), ["é"]);
    }

    #[test]
    fn special_and_function_keys_respect_filters() {
        let mut cfg = Config::default();
        let mut e = Engine::default();
        let now = Instant::now();
        tap(&mut e, &cfg, now, Key::Enter);
        tap(&mut e, &cfg, now, Key::F(5));
        assert_eq!(texts(&e), ["Enter", "F5"]);

        cfg.show_special_keys = false;
        cfg.show_function_keys = false;
        let mut e = Engine::default();
        tap(&mut e, &cfg, now, Key::Enter);
        tap(&mut e, &cfg, now, Key::F(5));
        assert!(e.is_empty());
    }

    #[test]
    fn hotkeys_toggle_pause_and_open_settings() {
        let cfg = Config::default();
        let mut e = Engine::default();
        let now = Instant::now();
        for (k, c) in [(Key::Ctrl, LCTRL), (Key::Alt, LALT), (Key::Shift, LSHIFT)] {
            e.handle(&key(k, c, true), &cfg, now);
        }
        assert_eq!(tap(&mut e, &cfg, now, Key::Char('s')), Action::OpenSettings);
        assert_eq!(tap(&mut e, &cfg, now, Key::Char('k')), Action::TogglePause);
        assert!(e.paused);
        assert_eq!(texts(&e), ["Keyshow paused"]);

        // While paused, combos are hidden.
        e.handle(&key(Key::Alt, LALT, false), &cfg, now);
        e.handle(&key(Key::Shift, LSHIFT, false), &cfg, now);
        tap(&mut e, &cfg, now, Key::Char('c'));
        assert_eq!(texts(&e), ["Keyshow paused"]);
    }

    #[test]
    fn lone_modifier_only_when_enabled() {
        let now = Instant::now();
        let mut cfg = Config::default();
        let mut e = Engine::default();
        tap(&mut e, &cfg, now, Key::Shift);
        assert!(e.is_empty());

        cfg.show_lone_modifiers = true;
        tap(&mut e, &cfg, now, Key::Shift);
        assert_eq!(texts(&e), ["Shift"]);

        // A modifier used in a combo is not "lone".
        let mut e = Engine::default();
        e.handle(&key(Key::Ctrl, LCTRL, true), &cfg, now);
        tap(&mut e, &cfg, now, Key::Char('v'));
        e.handle(&key(Key::Ctrl, LCTRL, false), &cfg, now);
        assert_eq!(texts(&e), ["Ctrl + V"]);
    }

    #[test]
    fn mouse_clicks_with_modifiers_and_double_click() {
        let cfg = Config {
            show_mouse: true,
            ..Config::default()
        };
        let mut e = Engine::default();
        let now = Instant::now();
        e.handle(&key(Key::Ctrl, LCTRL, true), &cfg, now);
        e.handle(&InputEvent::Mouse(MouseButton::Left), &cfg, now);
        e.handle(&InputEvent::Mouse(MouseButton::Left), &cfg, now);
        assert_eq!(texts(&e), ["Ctrl + Left Click ×2"]);
    }

    #[test]
    fn max_bubbles_and_fade() {
        let cfg = Config {
            max_bubbles: 2,
            ..Config::default()
        };
        let mut e = Engine::default();
        let now = Instant::now();
        for k in [Key::Enter, Key::Tab, Key::Esc] {
            tap(&mut e, &cfg, now, k);
        }
        assert_eq!(texts(&e), ["Tab", "Esc"]);

        let b = e.bubbles().next().unwrap().clone();
        let mid_fade = now + Duration::from_millis(cfg.duration_ms + cfg.fade_ms / 2);
        assert_eq!(Engine::alpha(&b, &cfg, now), 1.0);
        assert!((Engine::alpha(&b, &cfg, mid_fade) - 0.5).abs() < 0.01);

        e.prune(
            &cfg,
            now + Duration::from_millis(cfg.duration_ms + cfg.fade_ms + 1),
        );
        assert!(e.is_empty());
    }

    #[test]
    fn symbols_fall_back_to_text_when_glyph_missing() {
        let b = Bubble {
            content: Content::Keys(vec![Key::Ctrl, Key::Shift, Key::Char('p')]),
            count: 1,
            born: Instant::now(),
            updated: Instant::now(),
        };
        let parts = b.parts(ModifierStyle::Symbols, &mut |s| s != "⌃");
        assert_eq!(parts, ["Ctrl", "⇧", "P"]);
    }
}
