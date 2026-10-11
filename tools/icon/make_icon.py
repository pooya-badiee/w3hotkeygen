"""Draws the WC3 Grid Hotkeys icon: a gold "W3" on a key cap in a riveted metal frame.
Original artwork (no Blizzard assets). Needs Pillow.
Usage: python tools/icon/make_icon.py crates/app/assets"""
import sys
import math
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter

N = 1024  # draw big, downscale for antialiasing
GOLD = (255, 212, 40)
GOLD_DARK = (150, 100, 10)


def vgrad(size, top, bottom):
    w, h = size
    img = Image.new("RGBA", size)
    d = ImageDraw.Draw(img)
    for y in range(h):
        t = y / max(h - 1, 1)
        d.line([(0, y), (w, y)], fill=tuple(int(a + (b - a) * t) for a, b in zip(top, bottom)) + (255,))
    return img


def rounded_mask(size, box, r):
    m = Image.new("L", size, 0)
    ImageDraw.Draw(m).rounded_rectangle(box, r, fill=255)
    return m


def paste_grad(img, box, r, top, bottom):
    img.alpha_composite(Image.composite(vgrad(img.size, top, bottom), Image.new("RGBA", img.size), rounded_mask(img.size, box, r)))


img = Image.new("RGBA", (N, N), (0, 0, 0, 0))

# Metal frame + dark blue plate.
paste_grad(img, (24, 24, N - 24, N - 24), 190, (150, 156, 166), (52, 56, 64))
paste_grad(img, (70, 70, N - 70, N - 70), 150, (28, 40, 72), (8, 12, 24))
d = ImageDraw.Draw(img)
d.rounded_rectangle((70, 70, N - 70, N - 70), 150, outline=GOLD_DARK + (255,), width=10)
for x, y in [(118, 118), (N - 118, 118), (118, N - 118), (N - 118, N - 118)]:  # rivets
    d.ellipse((x - 20, y - 20, x + 20, y + 20), fill=(190, 194, 200, 255), outline=(40, 42, 48, 255), width=5)

# Key cap: shadow, side, top face.
cap = (232, 210, N - 232, N - 278)
shadow = Image.new("RGBA", (N, N), (0, 0, 0, 0))
ImageDraw.Draw(shadow).rounded_rectangle((cap[0], cap[1] + 70, cap[2], cap[3] + 90), 90, fill=(0, 0, 0, 170))
img.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(26)))
paste_grad(img, (cap[0], cap[1] + 40, cap[2], cap[3] + 64), 90, (40, 42, 50), (16, 17, 22))  # side
paste_grad(img, cap, 90, (78, 82, 94), (38, 40, 48))  # face
d = ImageDraw.Draw(img)
d.rounded_rectangle(cap, 90, outline=(120, 124, 136, 255), width=6)

# Gold "W3": dark outline, gold body, light highlight.
cy = (cap[1] + cap[3]) / 2
h = 250
wx, ww = N / 2 - 100, 230  # W center, width
w_pts = [(wx - ww / 2, cy - h / 2), (wx - ww / 4, cy + h / 2), (wx, cy - h / 10), (wx + ww / 4, cy + h / 2), (wx + ww / 2, cy - h / 2)]


def arc(cx, cy, rx, ry, a0, a1, n=24):
    return [(cx + rx * math.cos(math.radians(a0 + (a1 - a0) * i / n)), cy + ry * math.sin(math.radians(a0 + (a1 - a0) * i / n))) for i in range(n + 1)]


tx, r = N / 2 + 140, h / 4  # 3 center, bowl radius
three = [arc(tx, cy - r, r * 1.05, r, -165, 90), arc(tx, cy + r, r * 1.15, r, -90, 165)]
for width, color, dy in [(80, (60, 34, 0), 0), (64, GOLD_DARK, 0), (50, GOLD, -5)]:
    for pts in [w_pts] + three:
        d.line([(x, y + dy) for x, y in pts], fill=color + (255,), width=width, joint="curve")

out = Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)
big = img.resize((256, 256), Image.LANCZOS)
big.save(out / "icon.png", optimize=True)
big.save(out / "icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
