#!/usr/bin/env python3
"""Generate a macOS .icns for CrystalBall using only the Python standard library."""

from __future__ import annotations

import math
import os
import struct
import sys
import zlib
from pathlib import Path


ICNS_SCALES = (
    ("icp4", 16),
    ("icp5", 32),
    ("icp6", 64),
    ("ic07", 128),
    ("ic08", 256),
    ("ic09", 512),
    ("ic10", 1024),
)


def clamp(value: float, low: int = 0, high: int = 255) -> int:
    return max(low, min(high, int(round(value))))


def mix(a: tuple[int, int, int, int], b: tuple[int, int, int, int], t: float) -> tuple[int, int, int, int]:
    return tuple(clamp(a[i] + (b[i] - a[i]) * t) for i in range(4))


def rounded_rect_alpha(x: float, y: float, size: int, radius: float) -> float:
    px = abs(x - 0.5)
    py = abs(y - 0.5)
    half = 0.5
    inner = half - radius
    dx = max(px - inner, 0.0)
    dy = max(py - inner, 0.0)
    distance = math.hypot(dx, dy)
    return max(0.0, min(1.0, (radius - distance) * size))


def circle_alpha(x: float, y: float, cx: float, cy: float, radius: float, size: int) -> float:
    distance = math.hypot(x - cx, y - cy)
    return max(0.0, min(1.0, (radius - distance) * size))


def line_alpha(x: float, y: float, x1: float, y1: float, x2: float, y2: float, width: float, size: int) -> float:
    vx = x2 - x1
    vy = y2 - y1
    wx = x - x1
    wy = y - y1
    length_sq = vx * vx + vy * vy
    if length_sq == 0:
        distance = math.hypot(x - x1, y - y1)
    else:
        t = max(0.0, min(1.0, (wx * vx + wy * vy) / length_sq))
        distance = math.hypot(x - (x1 + t * vx), y - (y1 + t * vy))
    return max(0.0, min(1.0, (width - distance) * size))


def shade_pixel(x: float, y: float, size: int) -> tuple[int, int, int, int]:
    bg_alpha = rounded_rect_alpha(x, y, size, 0.22)
    color = (8, 8, 9, clamp(255 * bg_alpha))

    base_top = y > 0.72 and abs(x - 0.5) < (0.25 + (y - 0.72) * 0.75)
    if base_top:
        shade = 22 + int(54 * (1.0 - abs(x - 0.5) * 2.0))
        base = (shade, shade, shade + 4, 255)
        color = mix(color, base, bg_alpha)

    globe = circle_alpha(x, y, 0.5, 0.42, 0.285, size)
    if globe > 0:
        light = max(0.0, 1.0 - math.hypot(x - 0.38, y - 0.29) / 0.45)
        edge = math.hypot(x - 0.5, y - 0.42) / 0.285
        gray = clamp(148 + 92 * light + 40 * max(0.0, edge - 0.72))
        glass = (gray, gray, clamp(gray + 4), 228)
        color = mix(color, glass, globe)

    rim = abs(math.hypot(x - 0.5, y - 0.42) - 0.285)
    if rim < 0.012:
        color = mix(color, (252, 252, 253, 255), max(0.0, 1.0 - rim / 0.012))

    highlight = line_alpha(x, y, 0.36, 0.29, 0.58, 0.19, 0.028, size)
    if highlight > 0:
        color = mix(color, (255, 255, 255, 255), highlight * 0.88)

    shadow = line_alpha(x, y, 0.35, 0.56, 0.64, 0.59, 0.018, size)
    if shadow > 0:
        color = mix(color, (24, 24, 27, 255), shadow * 0.35)

    return color


def png_bytes(width: int, height: int, pixels: bytes) -> bytes:
    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    raw = bytearray()
    stride = width * 4
    for row in range(height):
        raw.append(0)
        raw.extend(pixels[row * stride : (row + 1) * stride])
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


def render(size: int) -> bytes:
    samples = 3 if size <= 128 else 2
    pixels = bytearray()
    for py in range(size):
        for px in range(size):
            acc = [0.0, 0.0, 0.0, 0.0]
            for sy in range(samples):
                for sx in range(samples):
                    x = (px + (sx + 0.5) / samples) / size
                    y = (py + (sy + 0.5) / samples) / size
                    r, g, b, a = shade_pixel(x, y, size)
                    acc[0] += r
                    acc[1] += g
                    acc[2] += b
                    acc[3] += a
            scale = samples * samples
            pixels.extend(clamp(channel / scale) for channel in acc)
    return png_bytes(size, size, bytes(pixels))


def icns_bytes() -> bytes:
    elements = bytearray()
    for icon_type, size in ICNS_SCALES:
        png = render(size)
        elements.extend(icon_type.encode("ascii"))
        elements.extend(struct.pack(">I", len(png) + 8))
        elements.extend(png)

    return b"icns" + struct.pack(">I", len(elements) + 8) + bytes(elements)


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: generate_macos_icon.py <output.icns>", file=sys.stderr)
        return 2

    output_path = Path(sys.argv[1])
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_bytes(icns_bytes())

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
