//! The app icon (a keycap with a "K"), rasterised at runtime so no image files are needed.

use eframe::egui::IconData;

/// RGBA pixels of a `size`×`size` icon.
pub fn rgba(size: u32) -> Vec<u8> {
    let s = size as f32;
    let mut px = Vec::with_capacity((size * size * 4) as usize);
    // The "K", as line segments in unit coordinates.
    let strokes = [
        ((0.36, 0.28), (0.36, 0.66)),
        ((0.36, 0.50), (0.64, 0.28)),
        ((0.45, 0.43), (0.65, 0.66)),
    ];
    for y in 0..size {
        for x in 0..size {
            let (u, v) = ((x as f32 + 0.5) / s, (y as f32 + 0.5) / s);
            let aa = 1.0 / s;
            let cover = |d: f32| (0.5 - d / aa).clamp(0.0, 1.0);

            // Base (key side) and top face of the keycap.
            let base = cover(rounded_rect(u, v, (0.06, 0.08), (0.94, 0.94), 0.16));
            let face = cover(rounded_rect(u, v, (0.10, 0.08), (0.90, 0.84), 0.13));
            let glyph = strokes
                .iter()
                .map(|(a, b)| cover(segment(u, v, *a, *b) - 0.055))
                .fold(0.0f32, f32::max);

            let mut c = [0.0f32; 4];
            blend(&mut c, [0.09, 0.09, 0.11], base);
            blend(&mut c, [0.20, 0.21, 0.25], face);
            blend(&mut c, [0.98, 0.80, 0.08], glyph * face);
            // `blend` accumulates premultiplied color; icons want straight alpha.
            let a = c[3];
            let straight = |v: f32| if a > 0.0 { (v / a).min(1.0) } else { 0.0 };
            px.extend(
                [straight(c[0]), straight(c[1]), straight(c[2]), a]
                    .iter()
                    .map(|v| (v * 255.0).round() as u8),
            );
        }
    }
    px
}

pub fn icon_data(size: u32) -> IconData {
    IconData {
        rgba: rgba(size),
        width: size,
        height: size,
    }
}

fn blend(dst: &mut [f32; 4], rgb: [f32; 3], a: f32) {
    for i in 0..3 {
        dst[i] = rgb[i] * a + dst[i] * (1.0 - a);
    }
    dst[3] = a + dst[3] * (1.0 - a);
}

/// Signed distance to a rounded rectangle.
fn rounded_rect(u: f32, v: f32, min: (f32, f32), max: (f32, f32), r: f32) -> f32 {
    let (cx, cy) = ((min.0 + max.0) / 2.0, (min.1 + max.1) / 2.0);
    let (hx, hy) = ((max.0 - min.0) / 2.0 - r, (max.1 - min.1) / 2.0 - r);
    let (dx, dy) = ((u - cx).abs() - hx, (v - cy).abs() - hy);
    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    outside + dx.max(dy).min(0.0) - r
}

/// Distance to a line segment.
fn segment(u: f32, v: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (px, py, bx, by) = (u - a.0, v - a.1, b.0 - a.0, b.1 - a.1);
    let t = ((px * bx + py * by) / (bx * bx + by * by)).clamp(0.0, 1.0);
    ((px - bx * t).powi(2) + (py - by * t).powi(2)).sqrt()
}
