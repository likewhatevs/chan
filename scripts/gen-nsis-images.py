#!/usr/bin/env python3
"""Generate the NSIS installer images (header + sidebar) from the Chan app icon.

The Windows installer chrome otherwise shows NSIS's own default header/sidebar
branding. Tauri's NSIS bundler takes a `headerImage` (150x57) and a
`sidebarImage` (164x314); both must be 24-bit BMP3 (no alpha). This bakes them
from the SAME squircle Chan logo the macOS app uses (`icons/icon.png`), so the
installer carries the product brand and the source never drifts from the app
icon.

Layout follows the NSIS MUI conventions:
  - header (150x57): the page title text is drawn over the LEFT of the strip, so
    the logo sits on the RIGHT with a white background behind the title.
  - sidebar (164x314): the welcome/finish text is on the RIGHT of the page, so
    the bitmap is a self-contained left column -- logo centered on white.

Usage:
  python3 scripts/gen-nsis-images.py            # write the BMPs into icons/
  python3 scripts/gen-nsis-images.py --preview  # also write PNG previews to /tmp
"""
import argparse
import os

from PIL import Image

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICONS = os.path.join(REPO, "desktop/src-tauri/icons")
SRC = os.path.join(ICONS, "icon.png")  # the squircle Chan logo (same art as the .icns/.ico)

# White matches the NSIS MUI page background so the squircle blends in.
BG = (255, 255, 255)

HEADER = (150, 57)
SIDEBAR = (164, 314)


def load_logo() -> Image.Image:
    logo = Image.open(SRC).convert("RGBA")
    return logo


def fit(logo: Image.Image, box: int) -> Image.Image:
    """Square logo scaled so its longest side is `box`, high-quality."""
    return logo.resize((box, box), Image.LANCZOS)


def flatten(canvas: Image.Image) -> Image.Image:
    """RGBA canvas -> RGB on the white background, for 24-bit BMP."""
    bg = Image.new("RGB", canvas.size, BG)
    bg.paste(canvas, (0, 0), canvas)
    return bg


def build_header(logo: Image.Image) -> Image.Image:
    w, h = HEADER
    canvas = Image.new("RGBA", (w, h), BG + (255,))
    # Logo fills the height with a small margin, right-aligned (title text rides
    # the left of the strip).
    side = h - 6
    icon = fit(logo, side)
    x = w - side - 3
    y = (h - side) // 2
    canvas.paste(icon, (x, y), icon)
    return flatten(canvas)


def build_sidebar(logo: Image.Image) -> Image.Image:
    w, h = SIDEBAR
    canvas = Image.new("RGBA", (w, h), BG + (255,))
    # Centered horizontally, sat in the upper-middle so it reads with the
    # welcome heading to its right.
    side = 124
    icon = fit(logo, side)
    x = (w - side) // 2
    y = (h - side) // 2 - 24
    canvas.paste(icon, (x, y), icon)
    return flatten(canvas)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--preview", action="store_true", help="also write PNG previews to /tmp")
    args = ap.parse_args()

    logo = load_logo()
    header = build_header(logo)
    sidebar = build_sidebar(logo)

    header_path = os.path.join(ICONS, "installer-header.bmp")
    sidebar_path = os.path.join(ICONS, "installer-sidebar.bmp")
    # BMP3 / 24-bit (PIL writes BMP3 for RGB images), which NSIS requires.
    header.save(header_path, "BMP")
    sidebar.save(sidebar_path, "BMP")
    print(f"wrote {header_path} ({header.size[0]}x{header.size[1]}, {header.mode})")
    print(f"wrote {sidebar_path} ({sidebar.size[0]}x{sidebar.size[1]}, {sidebar.mode})")

    if args.preview:
        header.save("/tmp/installer-header.png")
        sidebar.save("/tmp/installer-sidebar.png")
        print("previews: /tmp/installer-header.png /tmp/installer-sidebar.png")


if __name__ == "__main__":
    main()
