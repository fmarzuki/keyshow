#!/usr/bin/env python3
"""Render the Keyshow icon (same drawing as src/icon.rs) into packaging/windows/keyshow.ico.

Usage: python3 packaging/icons.py
"""
import math
import pathlib
import struct
import zlib

SIZES = [16, 24, 32, 48, 64, 128, 256]
STROKES = [((0.36, 0.28), (0.36, 0.66)), ((0.36, 0.50), (0.64, 0.28)), ((0.45, 0.43), (0.65, 0.66))]


def rounded_rect(u, v, mn, mx, r):
    cx, cy = (mn[0] + mx[0]) / 2, (mn[1] + mx[1]) / 2
    hx, hy = (mx[0] - mn[0]) / 2 - r, (mx[1] - mn[1]) / 2 - r
    dx, dy = abs(u - cx) - hx, abs(v - cy) - hy
    return math.hypot(max(dx, 0), max(dy, 0)) + min(max(dx, dy), 0) - r


def segment(u, v, a, b):
    px, py, bx, by = u - a[0], v - a[1], b[0] - a[0], b[1] - a[1]
    t = max(0.0, min(1.0, (px * bx + py * by) / (bx * bx + by * by)))
    return math.hypot(px - bx * t, py - by * t)


def render(size):
    out = bytearray()
    aa = 1 / size
    cover = lambda d: max(0.0, min(1.0, 0.5 - d / aa))
    for y in range(size):
        for x in range(size):
            u, v = (x + 0.5) / size, (y + 0.5) / size
            base = cover(rounded_rect(u, v, (0.06, 0.08), (0.94, 0.94), 0.16))
            face = cover(rounded_rect(u, v, (0.10, 0.08), (0.90, 0.84), 0.13))
            glyph = max(cover(segment(u, v, a, b) - 0.055) for a, b in STROKES)
            c = [0.0, 0.0, 0.0, 0.0]
            for rgb, a in (((0.09, 0.09, 0.11), base), ((0.20, 0.21, 0.25), face), ((0.98, 0.80, 0.08), glyph * face)):
                for i in range(3):
                    c[i] = rgb[i] * a + c[i] * (1 - a)
                c[3] = a + c[3] * (1 - a)
            # Stored premultiplied above; un-premultiply for PNG.
            alpha = c[3]
            rgb = [ch / alpha if alpha > 0 else 0 for ch in c[:3]]
            out += bytes(round(min(1, ch) * 255) for ch in rgb + [alpha])
    return bytes(out)


def png(size, rgba):
    rows = b"".join(b"\0" + rgba[y * size * 4:(y + 1) * size * 4] for y in range(size))
    chunk = lambda t, d: struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d))
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")


def ico(images):
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    entries, data = b"", b""
    for size, blob in images:
        dim = 0 if size >= 256 else size
        entries += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(blob), offset + len(data))
        data += blob
    return header + entries + data


if __name__ == "__main__":
    out = pathlib.Path(__file__).parent / "windows" / "keyshow.ico"
    out.parent.mkdir(exist_ok=True)
    out.write_bytes(ico([(s, png(s, render(s))) for s in SIZES]))
    print(f"wrote {out}")
