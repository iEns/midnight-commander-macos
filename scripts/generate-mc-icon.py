#!/usr/bin/env python3
"""Generate Midnight Commander style app icons (classic blue dual-pane look)."""

from __future__ import annotations

import math
import struct
import zlib
from pathlib import Path


def _chunk(tag: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + tag
        + data
        + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    )


def write_png(path: Path, width: int, height: int, rgba_rows: list[bytes]) -> None:
    raw = b"".join(b"\x00" + row for row in rgba_rows)
    compressed = zlib.compress(raw, 9)
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n" + _chunk(b"IHDR", ihdr) + _chunk(b"IDAT", compressed) + _chunk(b"IEND", b"")
    path.write_bytes(png)


def blend(fg: tuple[int, int, int], bg: tuple[int, int, int], alpha: float) -> tuple[int, int, int]:
    return tuple(int(bg[i] * (1 - alpha) + fg[i] * alpha) for i in range(3))


def render_icon(size: int) -> list[bytes]:
    # Classic mc palette
    bg = (0, 0, 128)          # navy frame
    left = (85, 85, 255)      # bright blue left pane
    right = (0, 0, 170)       # darker right pane
    status = (255, 255, 85)   # yellow status bar
    text = (255, 255, 255)
    border = (170, 170, 255)

    margin = max(1, size // 10)
    bar_h = max(2, size // 7)
    split_x = size // 2
    top = margin
    bottom = size - margin - bar_h
    rows: list[bytes] = []

    for y in range(size):
        row = bytearray(size * 4)
        for x in range(size):
            color = bg
            if top <= y < bottom and margin <= x < size - margin:
                pane = left if x < split_x else right
                color = pane
                if x in (margin, split_x - 1, split_x, size - margin - 1) or y in (top, bottom - 1):
                    color = border
            elif bottom <= y < size - margin and margin <= x < size - margin:
                color = status
                if x in (margin, size - margin - 1):
                    color = border

            if size >= 48 and y == top + max(2, size // 12) and margin + 2 <= x < split_x - 3:
                color = text
            if size >= 48 and y == top + max(2, size // 12) and split_x + 2 <= x < size - margin - 3:
                color = blend((220, 220, 220), right, 0.8)

            idx = x * 4
            row[idx : idx + 4] = bytes([color[0], color[1], color[2], 255])
        rows.append(bytes(row))

    if size >= 64:
        # Simple dash hint in the status bar (byte index is within the row only)
        cx = size // 2 - max(2, size // 16)
        cy = bottom + max(1, bar_h // 3)
        row = bytearray(rows[cy])
        for dx in range(max(4, size // 10)):
            px = cx + dx
            if margin + 1 <= px < size - margin - 1:
                idx = px * 4
                row[idx : idx + 4] = bytes([0, 0, 0, 255])
        rows[cy] = bytes(row)

    return rows


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    icon_dir = root / "src-tauri" / "icons"
    iconset = icon_dir / "AppIcon.iconset"
    icon_dir.mkdir(parents=True, exist_ok=True)
    if iconset.exists():
        import shutil

        shutil.rmtree(iconset)
    iconset.mkdir()

    size_map = {
        16: ["icon_16x16.png"],
        32: ["icon_32x32.png", "icon_16x16@2x.png"],
        64: ["icon_32x32@2x.png"],
        128: ["icon_128x128.png"],
        256: ["icon_256x256.png", "icon_128x128@2x.png"],
        512: ["icon_512x512.png", "icon_256x256@2x.png"],
        1024: ["icon_512x512@2x.png"],
    }

    for size, names in size_map.items():
        rows = render_icon(size)
        for name in names:
            write_png(iconset / name, size, size, rows)

    for name, src in [
        ("32x32.png", "icon_32x32.png"),
        ("128x128.png", "icon_128x128.png"),
        ("128x128@2x.png", "icon_256x256.png"),
    ]:
        (icon_dir / name).write_bytes((iconset / src).read_bytes())

    import subprocess

    icns = icon_dir / "icon.icns"
    subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(icns)], check=True)
    import shutil

    shutil.rmtree(iconset)
    print(f"Generated icons in {icon_dir}")


if __name__ == "__main__":
    main()