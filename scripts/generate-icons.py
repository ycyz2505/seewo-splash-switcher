import math
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"
OUT.mkdir(parents=True, exist_ok=True)


def lerp(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))


def rounded_rect(x, y, w, h, r):
    cx, cy = w / 2.0, h / 2.0
    qx = abs(x - cx) - (w / 2.0 - r)
    qy = abs(y - cy) - (h / 2.0 - r)
    dx = max(qx, 0.0)
    dy = max(qy, 0.0)
    return math.hypot(dx, dy) + min(max(qx, qy), 0.0) - r


def color_at(x, y, size):
    t = max(0.0, min(1.0, (x / size + y / size) / 2.0))
    return lerp((110, 168, 255), (139, 124, 255), t)


def alpha_at(x, y, size):
    # rounded square
    if rounded_rect(x, y, size, size, size * 0.22) > 0:
        return 0
    cx, cy = size / 2.0, size / 2.0
    dx, dy = x - cx, y - cy
    dist = math.hypot(dx, dy)
    angle = math.degrees(math.atan2(dy, dx))
    # white arc from -55deg to 235deg
    if size * 0.20 <= dist <= size * 0.30 and -55 <= angle <= 235:
        return 0.95
    # dot at the end of the arc
    dot_angle = math.radians(235)
    dot_x = cx + math.cos(dot_angle) * size * 0.25
    dot_y = cy + math.sin(dot_angle) * size * 0.25
    if math.hypot(x - dot_x, y - dot_y) <= size * 0.045:
        return 0.95
    return 0


def render(size):
    rows = []
    samples = 4
    for y in range(size):
        row = bytearray()
        for x in range(size):
            a_sum = 0.0
            r_sum = g_sum = b_sum = 0.0
            for sy in range(samples):
                for sx in range(samples):
                    px = x + (sx + 0.5) / samples
                    py = y + (sy + 0.5) / samples
                    a = alpha_at(px, py, size)
                    r, g, b = color_at(px, py, size)
                    a_sum += a
                    r_sum += r * a
                    g_sum += g * a
                    b_sum += b * a
            n = samples * samples
            alpha = int(round(255 * a_sum / n))
            if alpha == 0:
                row += bytes((0, 0, 0, 0))
            else:
                row += bytes(
                    (
                        int(round(r_sum / a_sum)),
                        int(round(g_sum / a_sum)),
                        int(round(b_sum / a_sum)),
                        alpha,
                    )
                )
        rows.append(bytes(row))
    return rows


def write_png(path, size):
    rows = render(size)

    def chunk(kind, data):
        return (
            struct.pack(">I", len(data))
            + kind
            + data
            + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
        )

    raw = b"".join(b"\x00" + row for row in rows)
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 9))
    png += chunk(b"IEND", b"")
    path.write_bytes(png)


def write_ico(path, png_path, size):
    png = png_path.read_bytes()
    header = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack("<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32, len(png), 22)
    path.write_bytes(header + entry + png)


write_png(OUT / "icon.png", 512)
write_png(OUT / "128x128@2x.png", 256)
write_png(OUT / "128x128.png", 128)
write_png(OUT / "32x32.png", 32)
write_ico(OUT / "icon.ico", OUT / "128x128@2x.png", 256)
print("icons written to", OUT)
