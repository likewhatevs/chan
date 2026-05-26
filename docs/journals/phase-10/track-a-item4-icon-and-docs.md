# Phase 10 Track A Item 4 - Icon Regeneration + Desktop Docs Audit

Agent: @@IconDocs. Date: 2026-05-26. Baseline: `23fa3aa` (v0.15.4).

Item 4 was two disjoint desktop-facing subtasks handed from Track B:
regenerate the Tauri app icon, and audit stale desktop docs/config.

## Subtask A: Tauri app icon

Goal: Cmd+Tab and the Dock show a dark `#101112` ground with the orange
enso `#ef8f58`, colors taken from the dark site tokens
(`web-marketing/src/styles.css` `[data-mode="dark"]`: `--bg #101112`,
`--accent #ef8f58`).

The enso shape comes from `web-marketing/chan-mark.png` (1024x1024 RGBA,
shape in the alpha channel). The master was built by filling `#ef8f58`
through that alpha over the dark ground, then regenerated with
`cargo tauri icon`.

Two corrections during the work:

1. Centering. The source enso art is not centered: a least-squares
   circle fit puts the ring center about 78px left and 9px up of the
   1024 canvas center, with faint brush spray trailing upper-right.
   Bounding-box centering looks left-shifted (the eye reads the solid
   ring, not the faint spray). The enso is centered on its fitted
   circle instead.
2. macOS shape. A full-bleed square let macOS apply its own squircle +
   safe-area inset, which mangled placement. The master now bakes the
   macOS recipe: transparent margins + a dark `#101112` rounded rect in
   the safe area (824x824 tile in 1024, ~100px margin, ~185 radius),
   enso centered at ~60% of the tile. This matches measured macOS system
   icons (Safari / Notes / Maps / Music all render an 80% solid rounded
   rect, mine is 80.5%).

Files changed: the 17 tracked icons under `desktop/src-tauri/icons/`
(32/64/128/128@2x, the Square*Logo / StoreLogo Windows set, icon.icns,
icon.ico, icon.png). The iOS/Android sets that `cargo tauri icon` also
emits were removed; they were never tracked and the desktop shell does
not consume them.

### Verification on the built .app (not just source PNGs)

- `make build` in `desktop/` succeeded (`BUILD_EXIT=0`), producing
  `target/release/bundle/macos/Chan.app` and
  `Chan_0.15.4_aarch64.dmg`.
- Rendered the bundled icon
  (`sips -s format png Chan.app/Contents/Resources/Chan.icns`): centered
  orange enso on the dark squircle, transparent corners (alpha 0, so the
  squircle is baked in and macOS shows it as-is rather than re-masking).
- DMG / Finder / app menus: confirmed showing the new centered icon.
- Cmd+Tab: the switcher initially showed an off-center icon. Traced to a
  stale launch-time icon, not a file defect: the running `Chan.app`'s
  on-disk `Chan.icns` is byte-identical (sha `3c736e03`) to the centered
  build, but the app switcher renders the icon the running process
  loaded at launch (from the earlier off-center build). Resolution is a
  clean relaunch (and `killall Dock` to refresh icon services); the
  bundled / on-disk icon itself is verified correct.

## Subtask B: desktop docs/config audit (fresh-state only)

### Changed

- `desktop/README.md` Download section: was "(and, later, Linux and
  Windows builds)". Now macOS plus Linux (`.deb` / `.AppImage`) are
  published; Windows is not published yet. Grounded in
  `release-desktop.yml` (matrix builds macOS + Linux only) and
  `web-marketing` install copy.
- `desktop/design.md`, reconciled against `desktop/CLAUDE.md` (the
  canonical current-state doc) and the desktop source:
  - Section 5 ("The chan binary" -> "Self-contained runtime"): removed
    the bundled-`chan`-binary architecture (no `Contents/Resources/bin/chan`,
    no `externalBin`). The app links chan-drive + chan-server directly
    and runs registry mutations / feature toggles in-process against one
    shared `chan_drive::Library`. Confirmed in source: no
    `Command::new("chan")` registry calls, and `serve.rs` has tests
    asserting registry commands route through the embedded Library and
    that the `chan_bin_status` machinery is gone.
  - Section 8 (Self-upgrade): replaced the `chan.app/dl/latest/VERSION`
    fetch + "extract the upgrade module into chan-core" plan with the
    current `tauri-plugin-updater` reality (minisign-signed bundles,
    pubkey in `tauri.conf.json`, manifest endpoint owned by
    chan-prod-setup). The plugin is wired in `main.rs` with `updater:*`
    capabilities.
  - Sections 6 and 7 (Windows MSI): Windows desktop is deferred, not
    killed. Kept as forward-looking design, marked clearly not-yet-built
    (no Windows artifact published, Authenticode lane not open) rather
    than a current distribution channel. Section 7 distribution rewritten
    to the fresh-state CI shape (macOS DMG + Linux deb/AppImage on GitHub
    Releases, linked from chan.app/install).
  - Coherence fixes for the same contradictions where they recur:
    Section 1 (purpose + non-goal), Section 3.2 / 3.5 (open / close drive
    now register / unregister in-process, not via a `chan` subprocess),
    Section 4 (registry mutations in-process). Section 6 bundle name
    corrected `ChanDesktop.app` -> `Chan.app`.

### Intentionally kept (no change)

- `desktop/src-tauri/tauri.conf.json` `plugins.updater.endpoints`:
  `https://chan.app/dl/desktop/{{target}}/{{current_version}}/latest.json`.
  This is the desktop updater manifest, owned by chan-prod-setup and
  documented in `desktop/CLAUDE.md` ("Manifest endpoint"). It is NOT the
  removed CLI `/dl/latest` first-install contract, so it stays.

### Flagged to @@Architect

- `desktop/design.md` section 10.2 said chan-desktop embeds
  `chan-tunnel-server` "from chan-core". Phase 5 collapsed chan-core into
  this workspace. @@Architect authorized this as a one-line factual fix
  in the same item-4 pass: now "embeds the `chan-tunnel-server` workspace
  crate".
- `desktop/CLAUDE.md` "Release package version metadata" still uses
  `Chan_0.14.0` as the worked example; current version is 0.15.4.
  Outside the three named files. @@Architect deferred this to the next
  version-bump chore, not this commit.

## Verification commands

- Icons: `cargo tauri icon /tmp/chan-icon-master.png`;
  `make build` (`BUILD_EXIT=0`); `sips -s format png ...Chan.icns`.
- Docs: `desktop/*.md` edits only, no `docs/manual/` or `web-marketing/`
  copy touched, so no `npm run check` gate. No Rust touched, so no
  `cargo fmt --check`.

## Close-out (2026-05-26)

@@Architect approved item 4 (see `track-a-item4-review.md`
"Re-review: APPROVED"). Close-out state:

- design.md §10.2 chan-core reference fixed in this same pass per
  @@Architect authorization (one-line factual correction).
- Commit scope (atomic, path-scoped): `desktop/README.md`,
  `desktop/design.md`, the 17 icons under `desktop/src-tauri/icons/`, and
  this journal note. `phase-11*` and `attachments/` were left unstaged.
  Not pushed.
- Known-state, not a defect: the live Cmd+Tab switcher re-confirm is
  pending Alex's relaunch + `killall Dock`. The on-disk / bundled
  `Chan.icns` is byte-verified correct (sha `3c736e03`); the off-center
  switcher render was a stale launch-time icon held by the
  already-running app, not a file defect.
