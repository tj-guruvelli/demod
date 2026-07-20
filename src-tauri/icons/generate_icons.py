"""
Demod icon generator.

Renders the SAME 5-bar waveform mark the app shows in its sidebar
(index.html `.app-icon.pulse-glow`) into every Tauri icon file.

Two defects this file exists to prevent, both of which shipped once:

1. TWO-TONE. The first version drew a dark midnight-purple bar body inside a
   neon outline. At taskbar size that does not read as body-plus-outline, it
   reads as a gradient -- rejected by TJ 2026-07-20 ("the wave needs to be a
   solid color, not a gradient color"). The mark is now ONE flat colour with
   no outline. Do not reintroduce a second hue inside the glyph.

2. DRIFT FROM THE APP. The first version invented its own symmetric bar
   heights, so the desktop icon and the in-app mark were different shapes in
   different colours. The bar geometry below is copied verbatim from the
   sidebar SVG's 24x24 viewBox and is the single source of truth. If the SVG
   in index.html changes, change BARS to match and re-run.

Run: python generate_icons.py
Overwrites every icon file in this directory in place, at its original size.
"""

import os

from PIL import Image, ImageDraw

# ---------------------------------------------------------------------------
# Design constants (no magic numbers below this block)
# ---------------------------------------------------------------------------

# Single flat colour, matching --accent / --logo-color in src/style.css.
# One colour only -- see defect 1 in the module docstring.
MARK_COLOR = (242, 61, 245, 255)  # #f23df5 - primary neon magenta
BACKGROUND_COLOR = (0, 0, 0, 0)   # transparent canvas

# The mark's own coordinate system, copied from the sidebar SVG in index.html
# (`viewBox="0 0 24 24"`). Keep these in lockstep with that markup.
VIEWBOX = 24.0

# (x, y, width, height, corner_radius) per <rect class="app-icon-bar">.
# Center bar tallest, tapering outward, symmetric about the middle bar.
# Width 3, gap 2 => span 23, centred in the 24 viewBox at x=0.5.
BARS = [
    (0.5,  9.0,  3.0,  6.0,  1.4),
    (5.5,  5.0,  3.0,  14.0, 1.4),
    (10.5, 1.5,  3.0,  21.0, 1.4),
    (15.5, 5.0,  3.0,  14.0, 1.4),
    (20.5, 9.0,  3.0,  6.0,  1.4),
]

# Empty margin around the mark, as a fraction of canvas size per side. The SVG
# is edge-to-edge in its viewBox; icons need breathing room so the glyph is not
# flush against the tile edge.
CANVAS_PADDING_FRACTION = 0.10

# Base resolution the mark is drawn at before high-quality downscaling.
BASE_RESOLUTION = 1024

# ---------------------------------------------------------------------------
# Drawing
# ---------------------------------------------------------------------------


def draw_waveform_mark(size: int) -> Image.Image:
    """Draw the Demod waveform mark on a transparent square canvas of `size` px."""
    img = Image.new("RGBA", (size, size), BACKGROUND_COLOR)
    draw = ImageDraw.Draw(img)

    padding = size * CANVAS_PADDING_FRACTION
    scale = (size - 2 * padding) / VIEWBOX

    for x, y, w, h, radius in BARS:
        x0 = padding + x * scale
        y0 = padding + y * scale
        x1 = padding + (x + w) * scale
        y1 = padding + (y + h) * scale
        draw.rounded_rectangle(
            [x0, y0, x1, y1],
            radius=radius * scale,
            fill=MARK_COLOR,
        )

    return img


def render_at(size: int) -> Image.Image:
    """Render the mark at BASE_RESOLUTION then downscale to `size` with LANCZOS."""
    base = draw_waveform_mark(BASE_RESOLUTION)
    if size == BASE_RESOLUTION:
        return base
    return base.resize((size, size), Image.LANCZOS)


# ---------------------------------------------------------------------------
# Output targets: (filename, width, height) - must match existing files exactly
# ---------------------------------------------------------------------------

ICON_DIR = os.path.dirname(os.path.abspath(__file__))

SQUARE_TARGETS = [
    ("128x128.png", 128, 128),
    ("128x128@2x.png", 256, 256),
    ("32x32.png", 32, 32),
    ("Square107x107Logo.png", 107, 107),
    ("Square142x142Logo.png", 142, 142),
    ("Square150x150Logo.png", 150, 150),
    ("Square284x284Logo.png", 284, 284),
    ("Square30x30Logo.png", 30, 30),
    ("Square310x310Logo.png", 310, 310),
    ("Square44x44Logo.png", 44, 44),
    ("Square71x71Logo.png", 71, 71),
    ("Square89x89Logo.png", 89, 89),
    ("StoreLogo.png", 50, 50),
    ("icon.png", 512, 512),
]

ICO_SIZES = [16, 32, 48, 256]
ICNS_SIZE = 512  # icon.icns is a single 512x512 RGBA PNG-in-ICNS


def generate_square_pngs():
    for filename, w, h in SQUARE_TARGETS:
        assert w == h, f"unexpected non-square target: {filename}"
        img = render_at(w)
        img.save(os.path.join(ICON_DIR, filename))
        print(f"wrote {filename} ({w}x{h})")


def generate_ico():
    path = os.path.join(ICON_DIR, "icon.ico")
    render_at(256).save(path, format="ICO", sizes=[(s, s) for s in ICO_SIZES])
    print(f"wrote icon.ico with sizes {ICO_SIZES}")


def generate_icns():
    path = os.path.join(ICON_DIR, "icon.icns")
    render_at(ICNS_SIZE).save(path, format="ICNS")
    print(f"wrote icon.icns ({ICNS_SIZE}x{ICNS_SIZE})")


def main():
    generate_square_pngs()
    generate_ico()
    generate_icns()


if __name__ == "__main__":
    main()
