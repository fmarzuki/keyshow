//! Paints the bubbles into the transparent overlay window.

use std::sync::Arc;
use std::time::Instant;

use eframe::egui::{
    self, Align2, Color32, FontId, Galley, Pos2, Rect, Shadow, Stroke, StrokeKind, Ui, Vec2, pos2,
    vec2,
};

use crate::config::{Config, KeyStyle, Theme};
use crate::display::{Bubble, Engine};
use crate::keys::ModifierStyle;

/// Fade-in / slide-in time for a new bubble.
const APPEAR_SECS: f32 = 0.12;

struct Metrics {
    pad: Vec2,
    gap: f32,
    cap_gap: f32,
    depth: f32,
}

impl Metrics {
    fn new(cfg: &Config) -> Self {
        let f = cfg.font_size;
        Metrics {
            pad: vec2(f * 0.55, f * 0.32),
            gap: f * 0.35,
            cap_gap: f * 0.22,
            depth: (f * 0.1).max(2.0),
        }
    }
}

/// Room needed around bubbles for shadows.
fn margin(cfg: &Config) -> f32 {
    (cfg.font_size * 0.5).max(16.0)
}

/// Height of one bubble row, in points.
fn row_height(cfg: &Config) -> f32 {
    let m = Metrics::new(cfg);
    cfg.font_size * 1.3 + 2.0 * m.pad.y + m.depth
}

/// Height of the overlay window so `max_bubbles` rows fit.
pub fn band_height(cfg: &Config) -> f32 {
    let n = cfg.max_bubbles.max(1) as f32;
    n * row_height(cfg) + (n - 1.0) * Metrics::new(cfg).gap + 2.0 * margin(cfg)
}

fn color(c: [u8; 4], alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (f32::from(c[3]) * alpha).round() as u8)
}

struct Laid {
    caps: Vec<Arc<Galley>>,
    count: Option<Arc<Galley>>,
    size: Vec2,
    alpha: f32,
    appear: f32,
}

fn lay_out(ui: &Ui, b: &Bubble, cfg: &Config, theme: &Theme, now: Instant) -> Laid {
    let m = Metrics::new(cfg);
    let font = FontId::new(cfg.font_size, crate::fonts::overlay_family());
    let painter = ui.painter();
    let parts = b.parts(cfg.modifier_style, &mut crate::fonts::has_glyphs);

    let text_color = color(theme.text, 1.0);
    let caps: Vec<Arc<Galley>> = match cfg.key_style {
        KeyStyle::Keycaps => parts
            .into_iter()
            .map(|p| painter.layout_no_wrap(p, font.clone(), text_color))
            .collect(),
        KeyStyle::Pill => {
            let sep = match cfg.modifier_style {
                ModifierStyle::Text => " + ",
                ModifierStyle::Symbols => " ",
            };
            vec![painter.layout_no_wrap(parts.join(sep), font.clone(), text_color)]
        }
    };
    let count = b.show_count().then(|| {
        painter.layout_no_wrap(
            format!("×{}", b.count),
            FontId::new(cfg.font_size * 0.7, crate::fonts::overlay_family()),
            color(theme.accent, 1.0),
        )
    });

    let row = row_height(cfg);
    let mut width = match cfg.key_style {
        KeyStyle::Pill => caps[0].size().x + 2.0 * m.pad.x,
        KeyStyle::Keycaps => {
            caps.iter()
                .map(|g| g.size().x.max(cfg.font_size * 0.6) + 2.0 * m.pad.x)
                .sum::<f32>()
                + m.cap_gap * (caps.len().saturating_sub(1)) as f32
        }
    };
    if let Some(c) = &count {
        width += c.size().x + m.cap_gap;
    }

    let age_born = now.duration_since(b.born).as_secs_f32();
    Laid {
        caps,
        count,
        size: vec2(width, row),
        alpha: Engine::alpha(b, cfg, now) * cfg.opacity.clamp(0.05, 1.0),
        appear: (age_born / APPEAR_SECS).clamp(0.0, 1.0),
    }
}

/// Draws all live bubbles. Returns true while an animation is running.
pub fn paint(ui: &Ui, engine: &Engine, cfg: &Config, now: Instant) -> bool {
    let theme = cfg.theme();
    let m = Metrics::new(cfg);
    let area = ui.max_rect().shrink(margin(cfg));
    let (h_align, v_align) = cfg.anchor.align();

    // Newest bubble sits closest to the anchored edge.
    let mut laid: Vec<Laid> = engine
        .bubbles()
        .map(|b| lay_out(ui, b, cfg, &theme, now))
        .collect();
    if v_align == 0 {
        laid.reverse();
    }
    let total_h: f32 =
        laid.iter().map(|l| l.size.y).sum::<f32>() + m.gap * laid.len().saturating_sub(1) as f32;
    let mut y = match v_align {
        0 => area.top(),
        1 => area.center().y - total_h / 2.0,
        _ => area.bottom() - total_h,
    };

    let mut animating = false;
    for l in &laid {
        let x = match h_align {
            0 => area.left(),
            1 => area.center().x - l.size.x / 2.0,
            _ => area.right() - l.size.x,
        };
        // Slide in from the anchored edge.
        let slide = (1.0 - l.appear) * cfg.font_size * 0.4 * if v_align == 0 { -1.0 } else { 1.0 };
        let rect = Rect::from_min_size(pos2(x, y + slide), l.size);
        let alpha = l.alpha * l.appear;
        animating |= l.alpha < 1.0 || l.appear < 1.0;
        draw_bubble(ui, l, rect, cfg, &theme, &m, alpha);
        y += l.size.y + m.gap;
    }
    animating
}

fn draw_bubble(
    ui: &Ui,
    l: &Laid,
    rect: Rect,
    cfg: &Config,
    theme: &Theme,
    m: &Metrics,
    alpha: f32,
) {
    let painter = ui.painter();
    let radius = theme.corner_radius;
    let stroke = Stroke::new(theme.border_width, color(theme.border, alpha));
    let shadow = |r: Rect| {
        if theme.shadow {
            let s = Shadow {
                offset: [0, 4],
                blur: 18,
                spread: 0,
                color: Color32::from_black_alpha((90.0 * alpha) as u8),
            };
            painter.add(s.as_shape(r, radius));
        }
    };
    let text_pos =
        |g: &Galley, r: Rect| Align2::CENTER_CENTER.anchor_size(r.center(), g.size()).min;
    let tint = |g: &Arc<Galley>, r: Rect| {
        painter.galley_with_override_text_color(
            text_pos(g, r),
            g.clone(),
            color(theme.text, alpha),
        );
    };

    match cfg.key_style {
        KeyStyle::Pill => {
            let body = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.max.y - m.depth));
            shadow(body);
            painter.rect(
                body,
                radius,
                color(theme.background, alpha),
                stroke,
                StrokeKind::Inside,
            );
            let text_rect = match &l.count {
                Some(c) => Rect::from_min_max(
                    body.min,
                    pos2(body.max.x - c.size().x - m.cap_gap, body.max.y),
                ),
                None => body,
            };
            tint(&l.caps[0], text_rect);
            if let Some(c) = &l.count {
                let pos = pos2(
                    text_rect.max.x - m.pad.x * 0.5,
                    body.center().y - c.size().y / 2.0,
                );
                fade_galley(painter, pos, c, theme, alpha);
            }
        }
        KeyStyle::Keycaps => {
            let mut x = rect.left();
            for g in &l.caps {
                let w = g.size().x.max(cfg.font_size * 0.6) + 2.0 * m.pad.x;
                let cap =
                    Rect::from_min_size(pos2(x, rect.top()), vec2(w, rect.height() - m.depth));
                shadow(cap);
                // Key "depth": a darker copy offset downwards.
                let base = cap.translate(vec2(0.0, m.depth));
                painter.rect_filled(base, radius, darken(color(theme.background, alpha)));
                painter.rect(
                    cap,
                    radius,
                    color(theme.background, alpha),
                    stroke,
                    StrokeKind::Inside,
                );
                tint(g, cap);
                x += w + m.cap_gap;
            }
            if let Some(c) = &l.count {
                let pos = pos2(x, rect.top() + (rect.height() - m.depth - c.size().y) / 2.0);
                fade_galley(painter, pos, c, theme, alpha);
            }
        }
    }
}

fn fade_galley(painter: &egui::Painter, pos: Pos2, g: &Arc<Galley>, theme: &Theme, alpha: f32) {
    painter.galley_with_override_text_color(pos, g.clone(), color(theme.accent, alpha));
}

fn darken(c: Color32) -> Color32 {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    let d = |v: u8| (f32::from(v) * 0.55) as u8;
    Color32::from_rgba_unmultiplied(d(r), d(g), d(b), a)
}
