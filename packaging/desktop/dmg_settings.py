# dmgbuild layout for the Chan.app installer DMG.
#
# Read by `dmgbuild` (see packaging/desktop/build-dmg.sh). dmgbuild writes the Finder
# .DS_Store layout PROGRAMMATICALLY (pure Python, via the ds_store lib, then
# hdiutil for the image) with NO Finder / AppleScript. tauri-bundler's own DMG
# step drives Finder over osascript, which no-ops on a headless CI runner and
# yields a flat default-layout DMG; this settings file pins the layout so a
# headless CI build is byte-for-byte identical to a local one.
#
# The .app path arrives as `-D app=<abs path>`; the volume name is dmgbuild's
# positional arg ("Chan"). No background image (the dark look in the good shot
# is just Finder dark mode); only the icon view, sizes, and positions are
# pinned. ASCII only, no em dashes (repo writing rules).

import os.path

# Injected by dmgbuild from `-D app=...`; default keeps the file runnable
# standalone for a quick layout check.
application = defines.get("app", "Chan.app")  # noqa: F821 (dmgbuild global)
appname = os.path.basename(application)

# Compressed image; the standard distributable DMG format.
format = "UDZO"

# Volume contents: the app plus a drag-to-install Applications symlink.
files = [application]
symlinks = {"Applications": "/Applications"}

# Icon view, chromeless window (no toolbar/sidebar/status bar) so the drag
# target is the whole snug window, matching the good local shot.
default_view = "icon-view"
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
show_icon_preview = False

# 600x400 window; (x, y) screen origin is cosmetic (where it opens), the
# (w, h) is what the layout is pinned against.
window_rect = ((400, 200), (600, 400))

# 128px icons (the good shot); Chan.app left-of-center, Applications symlink
# right-of-center, both vertically centered. Centers are symmetric about the
# 300px window midline.
icon_size = 128
text_size = 13
icon_locations = {
    appname: (150, 200),
    "Applications": (450, 200),
}
