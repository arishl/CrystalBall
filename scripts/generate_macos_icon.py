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


def triangle_contains(
    x: float,
    y: float,
    a: tuple[float, float],
    b: tuple[float, float],
    c: tuple[float, float],
) -> bool:
    def sign(p1: tuple[float, float], p2: tuple[float, float], p3: tuple[float, float]) -> float:
        return (p1[0] - p3[0]) * (p2[1] - p3[1]) - (p2[0] - p3[0]) * (p1[1] - p3[1])

    p = (x, y)
    d1 = sign(p, a, b)
    d2 = sign(p, b, c)
    d3 = sign(p, c, a)
    has_neg = d1 < 0 or d2 < 0 or d3 < 0
    has_pos = d1 > 0 or d2 > 0 or d3 > 0
    return not (has_neg and has_pos)


def diamond_alpha(x: float, y: float, cx: float, cy: float, rx: float, ry: float, size: int) -> float:
    distance = abs(x - cx) / rx + abs(y - cy) / ry
    return max(0.0, min(1.0, (1.0 - distance) * size * 0.18))


def shade_pixel(x: float, y: float, size: int) -> tuple[int, int, int, int]:
    bg_alpha = rounded_rect_alpha(x, y, size, 0.22)
    color = (36, 16, 60, clamp(255 * bg_alpha))

    for center in (0.22, 0.38, 0.54, 0.70):
        band = max(0.0, min(1.0, (0.012 - abs(y - center)) * size))
        if band > 0:
            color = mix(color, (22, 7, 35, color[3]), band * 0.45)

    border = bg_alpha if x < 0.035 or x > 0.965 or y < 0.035 or y > 0.965 else 0.0
    if border > 0:
        color = mix(color, (246, 214, 109, 255), border * 0.88)

    fx = (x - 0.5) / 0.82 + 0.5
    fy = (y - 0.5) / 0.82 + 0.5

    for star in (
        (0.215, 0.225, 0.085, 0.085, (255, 227, 122, 255)),
        (0.785, 0.225, 0.065, 0.065, (255, 209, 90, 255)),
    ):
        sparkle = diamond_alpha(fx, fy, star[0], star[1], star[2], star[3], size)
        if sparkle > 0:
            color = mix(color, star[4], sparkle)

    globe = circle_alpha(fx, fy, 0.5, 0.42, 0.285, size)
    if globe > 0:
        color = mix(color, (142, 75, 197, 235), globe)

    rim = abs(math.hypot(fx - 0.5, fy - 0.42) - 0.285)
    if rim < 0.024:
        color = mix(color, (255, 233, 163, 255), max(0.0, 1.0 - rim / 0.024))

    highlight = circle_alpha(fx, fy, 0.40, 0.31, 0.055, size)
    if highlight > 0:
        color = mix(color, (255, 246, 207, 255), highlight * 0.92)

    if 0.66 <= fy <= 0.84:
        top_width = 0.34
        bottom_width = 0.48
        t = max(0.0, min(1.0, (fy - 0.66) / 0.18))
        half_width = top_width / 2.0 + (bottom_width - top_width) * t / 2.0
        base = max(0.0, min(1.0, (half_width - abs(fx - 0.5)) * size))
        if base > 0:
            color = mix(color, (57, 32, 78, 255), base)

    base_top = max(0.0, min(1.0, (0.014 - abs(fy - 0.66)) * size))
    if base_top > 0 and abs(fx - 0.5) < 0.18:
        color = mix(color, (246, 214, 109, 255), base_top)

    groove = max(0.0, min(1.0, (0.010 - abs(fy - 0.79)) * size))
    if groove > 0 and abs(fx - 0.5) < 0.30:
        color = mix(color, (184, 128, 56, 255), groove)

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
