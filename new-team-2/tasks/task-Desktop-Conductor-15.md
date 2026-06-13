# task-Desktop-Conductor-15 — item 6 + B3 complete

From: @@Desktop. To: @@Conductor. Re: task-Conductor-Desktop-4.
Date: 2026-06-12.

## Landed

- **Item 6** — launcher Open always enabled + auto-turn-on + failure
  dialog: commit `3d4f564b` (desktop/src/main.js only, 101+/11-).
  - `renderOpenSplit`: launch button unconditionally enabled; "Open
    in Browser" + caret keep their `hasUrl` gating; Forget untouched.
  - Launch handler: off → `set_workspace_on` (button disabled for the
    whole transition — double-click guard) → `refresh(true)` → open;
    turn-on failure → dialog + stop, no open attempt.
  - New `showTurnOnFailureDialog` on the `.preflight-*` scaffold
    (zero styles.css changes needed), OK focused, Escape + backdrop
    close, keydown listener explicitly removed in `close()` — the
    leak the design warned about is not copied.
  - Pill toggle handler routes turn-ON failures to the dialog;
    turn-off failures keep the banner; `refresh(true)` reconciliation
    unchanged.
- **B3** — drag-pasteboard negative pins: commit `54b65a60`
  (desktop/src-tauri/src/serve.rs only, +14). Pins BOTH remaining
  broad surfaces: `default.json` capability AND the `main-window` app
  permission set (full belt symmetry with the existing
  workspace.json / workspace-window pins).

## Gate (scoped own-gate, isolated)

Main-tree gating was impossible: peer WIP in
`crates/chan-server` (terminal_sessions.rs mid-signature-change)
does not compile, and chan-desktop depends on it by path. Per the
isolated-gate model I gated a detached worktree at HEAD `e0ec0d3c`
+ exactly my two files (sha256-verified identical to the committed
tree):

- `cargo fmt --all -- --check` — green.
- `RUSTFLAGS="-D warnings" cargo clippy --all-targets -- -D warnings`
  — green.
- `RUSTFLAGS="-D warnings" cargo test --all-targets` — green (86
  tests; `drag_pasteboard_read_is_scoped_to_locally_served_windows`
  also run by name: 1 passed).
- `node --check` on main.js — green. Gate ran AFTER the last edit.

## Verification walk (design list, real WKWebView + real Rust)

No AX/System-Events automation available, so I built the walk INTO
the launcher: dev `chan-desktop` binary from the isolated worktree
with a throwaway driver script (worktree-only instrumentation,
reverted after; never committed) that performs the design's
verification list in-page and reports each assertion to a local
listener. Isolated `$HOME` (`/private/tmp/chan-item6-home`) so
Alex's real registry/config were never touched; real flock conflict
via a second `chan serve --standalone` (worktree-built binary,
renamed copy, own port 48899; lock-proven by a third serve refusing
with "workspace is locked by another process").

**Result: 36/36 checks PASS** (full report:
`/tmp/chan-item6-report.jsonl`, volatile). Mapping to the design:

1. Happy path: Open while off → pill on, serve URL populated
   (`http://127.0.0.1:54547/workspace-…?t=…`), window spawned, no
   dialog/banner. Open while on → unchanged, no dialog/banner.
2. Failure path (flock genuinely held): Open AND pill both raise the
   dialog with the verbatim Rust reason "This workspace is open in
   another chan process. Quit it and try again."; dismissed via OK,
   Escape, and backdrop; pill consistent (off) after each; stray
   Escape after close is inert (no stacked listeners); double-click
   during in-flight turn-on yields exactly one dialog.
3. Toggle-off unchanged (no dialog, url cleared, Open STAYS enabled).
4. Remote rows: seeded outbound row — no pill, Open enabled
   (unchanged: hasUrl is always true for remote rows), "Open in
   Browser" enabled, "Forget URL" intact.

Screenshots: not cheap — display was asleep (`screencapture` cannot
image the display; Screen Recording perm also absent). Skipped.
Literal pixel/hit-testing remains on @@Alex's final smoke checklist
exactly as the design already routes it.

**Reusable lesson for anyone automating the launcher/terminal in a
real WKWebView:** with the display asleep the WebContent process is
suspended ~10s after launch (timers AND fetch callbacks freeze;
walks silently stall). Fix that worked: `backgroundThrottling:
"disabled"` on the window in tauri.conf.json (tauri 2.11 supports
it; instrumentation-only here) — possibly worth a permanent dev-flag
discussion some round.

## Standing lane duty — build base ready

- Isolated build worktree: `/tmp/chan-desktop-gate` (detached
  `e0ec0d3c`, now carrying exactly the two committed diffs; all
  instrumentation reverted) + warm `CARGO_TARGET_DIR=
  /tmp/chan-desktop-gate-target` + built `web/dist`. Dev builds of
  chan-desktop now take ~5-8s incremental; `chan` CLI also built.
- NOTE for the team: until the chan-server signature-change WIP
  lands, MAIN-TREE desktop builds will not compile. WKWebView
  verification builds should be requested through me and will come
  from the isolated worktree (re-synced to HEAD + the requesting
  lane's files). Fresh-binary provenance check before any re-walk,
  as specced.

## B5/B6/B4 context notes (recovered; ACK REQUESTED before starting)

- **B5 — buried-window memory visibility** (phase-22.md:122-124).
  Buried windows keep webviews alive deliberately (warm state) but
  it's memory the user can't see, and `MAX_WINDOWS_PER_WORKSPACE`
  (10) counts buried windows. Open: should the cap exclude buried
  windows; should the UI surface buried-window count/cost. Proposed
  scope if acked: decision note + small fix (cap semantics) +
  Window-menu count affordance; no webview-offloading refactor.
- **B6 — GTK set_menu in-place mutation check** (phase-22.md:114-115).
  `rebuild_window_menu` mutates the Window submenu in place
  (remove + append); muda main-thread constraint documented for
  macOS, UNVERIFIED on Linux/GTK; full `set_menu` rebuild fallback
  documented but not wired. Proposed scope if acked: empirical check
  via sdme (lima aarch64) — bury/unbury cycles watching for menu
  corruption; wire the fallback only if mutation misbehaves; record
  the finding either way.
- **B4 — Linux drop path-print** (phase-23.md:96-98, 225;
  task-ChanDesktop-Chan-1.md:72-78). Already a DOCUMENTED no-op:
  WebKitGTK has no persistent drag pasteboard; `read_dropped_paths`
  returns `[]` off-macOS by design; the takeover guard protects all
  platforms. Proposed scope if acked: short investigation note on
  whether any GTK/XDND route could recover paths post-drop (likely
  "no, by DOM/WebKit design"), then close as documented-no-op —
  no code.

Priority understood: build requests outrank B5/B6/B4.

## Review pairing

Launcher JS ready for @@Editor's adversarial review: commit
`3d4f564b`, design new-team-2/designs/item-6-launcher-open-auto-on.md.
Suggested foci: the in-flight guard around `refresh(true)`
re-renders (stale-element hazards), dialog listener lifecycle, and
the `browserDisabled` gating split.
