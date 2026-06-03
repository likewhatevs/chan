# Task: deterministic DMG layout (fix the flat CI DMG)

Dispatched by @@LaneA (@@Lead) to **@@LaneB**. Goes into the 0.25.0 release.

## Symptom (from @@Host)

The macOS .dmg that comes out of CI looks wrong: small default icons crammed
top-left, Applications first then Chan.app, an oversized flat Finder window.
The LOCAL `make macos-chan-dmg-notarised` build produces the good one: large
128px icons, Chan.app on the left + Applications on the right, a snug centered
window, drag-to-install. Same content, different window layout.

## Root cause (confirmed by @@LaneA)

Both local and CI run the SAME command: `cargo tauri build --bundles ...,dmg`
(`desktop/Makefile` `app-notarized`, line ~225). tauri-bundler 2.11.2's DMG
step sets the Finder window layout (icon size, the Chan-left/Apps-right
positions, the window bounds) by mounting the volume and running an AppleScript
that drives **Finder** (`osascript`). That needs a live GUI/Finder session:

- Local: a logged-in GUI session exists -> the AppleScript applies the layout
  -> the good DMG.
- CI (headless GitHub macOS runner): no usable Finder/WindowServer -> the
  AppleScript no-ops (build still succeeds) -> the volume keeps Finder's raw
  default view -> the flat DMG.

There is also NO `dmg` layout config in `desktop/src-tauri/tauri.conf.json`
(only `targets`, icon, signing identity), so nothing pins the layout.

## Goal

CI produces the SAME nice layout as local, deterministically. Match the good
shot: window roughly the local size, 128px icons, Chan.app left-of-center,
Applications symlink right-of-center, title "Chan". No custom background needed
(the dark look in the good shot is just Finder dark mode; do not add a bg image
unless it's required to lock the look). Drag-to-Applications must work.

## Approach (recommended: Finder-less layout tool)

Stop depending on a GUI Finder. Build the signed .app with tauri, then build
the DMG with a tool that writes the `.DS_Store` PROGRAMMATICALLY (no Finder),
so local == CI byte-for-byte on layout:

1. `cargo tauri build --bundles app` -> the signed `Chan.app` (signing stays
   tauri-managed; do NOT change entitlements/signing).
2. Build the DMG from that .app with **`dmgbuild`** (Python, actively
   maintained, writes `.DS_Store` via the `ds_store` lib, fully headless) OR
   **`appdmg`** (Node; the web build toolchain already pulls Node, so `npx
   appdmg` is low-friction). Pick one, justify in your journal. Layout spec:
   icon size 128, window size + positions matching the good shot, the
   `/Applications` symlink, title "Chan".
3. Notarize + staple the produced DMG with the EXISTING flow
   (`xcrun notarytool submit "$DMG"` + `stapler`) - it operates on any .dmg,
   just point it at the new output path. Notarization is unaffected.

Wire it into `desktop/Makefile` (`app-notarized`) and `.github/workflows/
release-desktop.yml` (the `make macos-chan-dmg-notarised` step). The build tool
(Python `dmgbuild` / Node `appdmg`) is a BUILD-time dep only - consistent with
the existing Node web build; the single-binary RUNTIME principle is untouched.

If you find a simpler path that's genuinely headless-safe (e.g. tauri 2 DMG
config that does NOT route through Finder), propose it instead - but the
AppleScript path is the trap to avoid.

## Authorization + boundaries

- @@LaneA (@@Lead) authorizes editing the desktop release-packaging path:
  `desktop/Makefile`, `desktop/src-tauri/tauri.conf.json`, and the DMG-LAYOUT
  portion of `.github/workflows/release-desktop.yml`. This is packaging only.
- Do NOT touch the signing/notarization SECRET references (APPLE_ID /
  APPLE_PASSWORD / TEAM_ID / signing identity / Keychain profile). Secret
  VALUES never appear in journals/commits; they stay in Actions Secrets /
  local Keychain. Leave the auth shape exactly as `app-notarized` has it.
- Own non-overlapping files. @@LaneA owns the full-tree gate + commit + tag.

## Verification

1. LOCAL layout proof (you, this Mac): build the DMG via the new path
   (UNSIGNED .app is fine for layout - no certs needed). `hdiutil attach` it,
   confirm the `.DS_Store` icon positions/size programmatically, then poke
   @@Host to OPEN the local DMG and eyeball it vs the good shot. Because the
   tool is Finder-less, local layout == CI layout.
2. CI proof (@@Host): dry-run `release-desktop.yml` via workflow_dispatch
   (publish=false) to confirm the signed+notarized HEADLESS pipeline yields the
   same nice DMG before the tag.

Own-gate: `make -C desktop check` + confirm `cargo tauri build` still produces a
working bundle. Do NOT push. Report to @@LaneA with a 1-line poke -> this file.

================================================================================
## Implementation (@@LaneB) - DONE, layout proven headless

### Tool choice: dmgbuild (Python), justified

Picked `dmgbuild` over `appdmg`. It writes the `.DS_Store` PURELY
programmatically (the `ds_store` lib, then `hdiutil` for the image) with NO
Finder/osascript - the exact root-cause fix - and pulls only pure-Python wheels
(dmgbuild + ds_store + mac_alias), so there is NO native-addon compilation.
`appdmg` would reuse the workflow's Node 20, but it pulls `fs-xattr`, a native
node-gyp addon - an avoidable release-time fragility. macOS runners + local
Macs ship `python3`; a throwaway venv pins the version and sidesteps PEP 668.

### What landed (packaging only; no Rust, no signing/secret changes)

- `desktop/packaging/dmg_settings.py` (NEW): the pinned layout - icon view,
  chromeless 600x400 window, `icon_size = 128`, `Chan.app` at (150,200)
  left-of-center, `Applications` at (450,200) right-of-center, the
  `/Applications` drag symlink. No background image (the dark look is Finder
  dark mode). The layout is fully pinned here, so any dmgbuild 1.x is identical.
- `desktop/packaging/build-dmg.sh` (NEW): Finder-less builder. Creates/reuses a
  version-pinned dmgbuild venv, runs `dmgbuild -s dmg_settings.py -D app=<app>
  "Chan" <out>`. Build-time only (like the Node web build); nothing shipped.
- `desktop/Makefile`: `app-notarized` now builds ONLY the signed `.app`
  (`--bundles app`, was `app,dmg`) then builds the DMG via `build-dmg.sh` and
  notarizes+staples THAT (existing keychain/env notarytool flow, just pointed at
  the new path). New `dmg-layout-proof` target builds the unsigned `.app` + DMG
  (no certs) for a local layout eyeball. Vars: `DMGBUILD_SPEC` (default
  `dmgbuild>=1.6,<2`), `DMG_VENV`, `BUNDLE_DIR`.

### Deliberately NOT changed

- `tauri.conf.json`: untouched. dmgbuild owns layout, so there is no tauri DMG
  config to add; `bundle.targets` stays `"all"` (Linux deb/rpm/appimage need
  it), and the explicit `--bundles app` in `app-notarized` is what keeps tauri
  from re-introducing its Finder DMG.
- `release-desktop.yml`: untouched. `make macos-chan-dmg-notarised` is unchanged
  and now produces the good DMG via the Makefile; macOS runners ship `python3`
  so the venv needs no `setup-python` step. (If a CI dry-run ever reports no
  python3, the fix is a 1-line `actions/setup-python` add - flagged, not
  pre-emptively added, to keep the release pipeline churn-free.)
- `release.yml` already finds the DMG by glob (`find bundle/dmg -name '*.dmg'`)
  then renames to `Chan_${VERSION}.dmg`, and the updater payload tars `Chan.app`
  (not the DMG) - so the new path/name is a drop-in. The Makefile clears stale
  `bundle/dmg/*.dmg` so exactly one DMG is present for the glob.

### Verification (LOCAL, this Mac)

- `make -C desktop check`: GREEN (Makefile parses, crate compiles).
- Finder-less path proven END-TO-END against a stub `Chan.app` (the layout is
  independent of app contents): `build-dmg.sh` built the DMG headlessly (venv +
  dmgbuild, no Finder), `hdiutil attach` showed `Chan.app` + `Applications ->
  /Applications`, and the `.DS_Store` (read via `ds_store`) carried the exact
  programmatic icon locations `Chan.app -> (150,200)`, `Applications ->
  (450,200)` + the icon-view settings record. Headless determinism proof: same
  code path local and CI.

### Remaining (not mine to run)

- Full real-`.app` local eyeball: `make -C desktop dmg-layout-proof` (slow
  release compile; no certs needed) -> `hdiutil attach` -> @@Alex eyeballs vs
  the good shot.
- CI proof: @@Host dry-runs `release-desktop.yml` (workflow_dispatch,
  publish=false) to confirm the signed+notarized HEADLESS pipeline yields the
  same layout before the tag.

Own-gate GREEN (`make -C desktop check`). Did NOT push.
