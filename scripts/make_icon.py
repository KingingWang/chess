#!/usr/bin/env python3
"""Render the Xiangqi app icon (1024x1024) in the 「玄玉」 theme.

Design: an ink-black field with a brass hairline frame, and a single cream
Xiangqi piece in the centre bearing the cinnabar glyph 「將」 — exactly the
red-side piece the player moves in game. Rendered at 4x and downsampled for
crisp edges. Colours mirror `crates/chess-app/src/ui_theme.rs` and
`board_theme.rs` (Classic palette).

Usage: python3 scripts/make_icon.py [out.png]   # default: macos/AppIcon-1024.png
"""

import sys
from PIL import Image, ImageDraw, ImageFilter, ImageFont

SS = 4                      # supersampling factor
SIZE = 1024                 # final size
S = SIZE * SS

# Theme colours (srgb floats -> 0-255)
def C(r, g, b):
    return (round(r * 255), round(g * 255), round(b * 255))

INK = C(0.070, 0.064, 0.055)            # app background
INK_GLOW = C(0.115, 0.100, 0.082)       # subtle centre glow
GOLD = C(0.792, 0.639, 0.373)           # brass frame
DISC_FACE = C(0.965, 0.925, 0.827)      # piece face
DISC_EDGE = C(0.30, 0.21, 0.10)         # piece rim
RED_INK = C(0.678, 0.141, 0.141)        # red glyph

FONT = "assets/fonts/src/NotoSerifCJKsc-Bold.otf"


def radial_background() -> Image.Image:
    """Ink field with a soft warm glow behind the piece."""
    img = Image.new("RGB", (S, S), INK)
    glow = Image.new("L", (S, S), 0)
    d = ImageDraw.Draw(glow)
    # Concentric ellipses, brightest in the middle, feathered afterwards.
    steps = 48
    for i in range(steps, 0, -1):
        r = S * 0.62 * i / steps
        v = round(90 * (1 - i / steps) ** 1.6)
        d.ellipse([S / 2 - r, S * 0.46 - r, S / 2 + r, S * 0.46 + r], fill=v)
    glow = glow.filter(ImageFilter.GaussianBlur(S * 0.06))
    warm = Image.new("RGB", (S, S), INK_GLOW)
    return Image.composite(warm, img, glow)


def main() -> None:
    out = sys.argv[1] if len(sys.argv) > 1 else "macos/AppIcon-1024.png"

    img = radial_background().convert("RGBA")
    d = ImageDraw.Draw(img)

    # Faint brass ring around the piece — a circle survives the macOS
    # squircle mask, a square frame would get its corners clipped.
    cx, cy = S / 2, S * 0.48
    R = S * 0.33                       # piece radius
    d.ellipse([cx - R * 1.12, cy - R * 1.12, cx + R * 1.12, cy + R * 1.12],
              outline=GOLD + (110,), width=SS * 3)

    # Soft contact shadow under the piece.
    shadow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).ellipse(
        [cx - R * 0.96, cy - R * 0.88 + S * 0.018,
         cx + R * 0.96, cy + R * 1.02 + S * 0.018],
        fill=(0, 0, 0, 150),
    )
    img.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(S * 0.02)))

    # Piece face with a slightly domed radial highlight.
    face = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    fd = ImageDraw.Draw(face)
    fd.ellipse([cx - R, cy - R, cx + R, cy + R], fill=DISC_FACE + (255,))
    dome = Image.new("L", (S, S), 0)
    dd = ImageDraw.Draw(dome)
    for i in range(40, 0, -1):
        r = R * 0.92 * i / 40
        v = round(46 * (1 - i / 40))
        dd.ellipse([cx - r, cy - r * 1.06 - R * 0.06,
                    cx + r, cy + r * 1.06 - R * 0.06], fill=v)
    dome = dome.filter(ImageFilter.GaussianBlur(S * 0.012))
    face.paste(Image.new("RGBA", (S, S), (255, 252, 240, 0)), (0, 0), dome)
    img.alpha_composite(face)

    # Rims: dark outer edge, then the engraved double ring seen on real pieces.
    d = ImageDraw.Draw(img)
    d.ellipse([cx - R, cy - R, cx + R, cy + R],
              outline=DISC_EDGE + (255,), width=SS * 10)
    d.ellipse([cx - R * 0.90, cy - R * 0.90, cx + R * 0.90, cy + R * 0.90],
              outline=DISC_EDGE + (200,), width=SS * 5)
    d.ellipse([cx - R * 0.80, cy - R * 0.80, cx + R * 0.80, cy + R * 0.80],
              outline=RED_INK + (170,), width=SS * 4)

    # The glyph 「將」 in Noto Serif CJK Bold.
    font = ImageFont.truetype(FONT, round(R * 1.06))
    glyph = "將"
    box = d.textbbox((0, 0), glyph, font=font)
    w, h = box[2] - box[0], box[3] - box[1]
    d.text((cx - w / 2 - box[0], cy - h / 2 - box[1]), glyph,
           font=font, fill=RED_INK + (255,))

    img = img.resize((SIZE, SIZE), Image.LANCZOS).convert("RGB")
    img.save(out, "PNG")
    print(f"wrote {out} ({SIZE}x{SIZE})")


if __name__ == "__main__":
    main()
