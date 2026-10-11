//! Picks a bold system UI font for the overlay, plus a fallback with key symbols
//! (⌃ ⇧ ⌥ ⏎ ⌫). egui's built-in fonts stay as the last fallback.

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use skrifa::MetadataProvider;

/// Key-symbol characters that the loaded overlay fonts can draw.
// egui's own `has_glyphs` reports false for anything owned by its replacement-glyph
// face, which is usually the very font that has these symbols, so we check charmaps.
static SYMBOLS: OnceLock<HashSet<char>> = OnceLock::new();

#[cfg(windows)]
fn candidates() -> (Vec<String>, Vec<String>) {
    let dir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into()) + r"\Fonts\";
    (
        vec![dir.clone() + "segoeuib.ttf", dir.clone() + "arialbd.ttf"],
        vec![dir.clone() + "seguisym.ttf", dir + "seguiemj.ttf"],
    )
}

#[cfg(not(windows))]
fn candidates() -> (Vec<String>, Vec<String>) {
    let dejavu = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf",
    ];
    let main = [
        "/usr/share/fonts/truetype/ubuntu/Ubuntu-B.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/google-noto/NotoSans-Bold.ttf",
    ];
    let own = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    (own(&[&main[..], &dejavu[..]].concat()), own(&dejavu))
}

fn first_readable(paths: &[String]) -> Option<Vec<u8>> {
    paths.iter().find_map(|p| std::fs::read(p).ok())
}

/// Whether the overlay font can draw every character of `s`.
pub fn has_glyphs(s: &str) -> bool {
    let symbols = SYMBOLS.get();
    s.chars()
        .all(|c| c.is_ascii() || symbols.is_some_and(|set| set.contains(&c)))
}

/// Font family used by the overlay bubbles.
pub fn overlay_family() -> FontFamily {
    FontFamily::Name("keyshow".into())
}

pub fn install(ctx: &egui::Context) {
    let (main, symbols) = candidates();
    let mut fonts = FontDefinitions::default();
    let defaults = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut overlay = Vec::new();
    let mut supported = HashSet::new();
    for (key, list) in [("keyshow-main", main), ("keyshow-symbols", symbols)] {
        let Some(bytes) = first_readable(&list) else {
            continue;
        };
        if let Ok(face) = skrifa::FontRef::new(&bytes) {
            let charmap = face.charmap();
            supported.extend(
                crate::keys::SYMBOL_CHARS
                    .chars()
                    .filter(|c| charmap.map(*c).is_some()),
            );
        }
        fonts
            .font_data
            .insert(key.to_owned(), Arc::new(FontData::from_owned(bytes)));
        overlay.push(key.to_owned());
    }
    // The Settings window keeps egui's font but can still render key symbols.
    if overlay.iter().any(|k| k == "keyshow-symbols") {
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .push("keyshow-symbols".into());
    }
    let _ = SYMBOLS.set(supported);
    overlay.extend(defaults);
    fonts.families.insert(overlay_family(), overlay);
    ctx.set_fonts(fonts);
}
