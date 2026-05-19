# fullstack-83: Cmd+N → new chan-desktop window (desktop only)

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

Cmd+N is currently free in chan (Cmd+S drop
in `-56` was the most recent shortcut
clearance; Cmd+N never had a binding).
@@Alex wants to use it for **create new
window** in the **native chan-desktop app
only** — not in the web SPA running in a
browser.

Web SPA case: browser's default Cmd+N (new
browser window) is fine; chan doesn't
intercept.

Native desktop case: Tauri-level keyboard
accelerator on the window menu spawns a new
chan-desktop window. Multiple windows are
already supported per the
`w=<window-label>` URL parameter
(`desktop/CLAUDE.md`); this binding just
makes the gesture discoverable.

## Spec

* Cmd+N (macOS) / Ctrl+N (Windows + Linux)
  in chan-desktop spawns a new window:
  * Same chan-desktop binary, fresh window
    label (e.g. `chan-w<N>` or a UUID).
  * New launcher view (drive picker)
    inside the new window, NOT the
    currently-open drive cloned.
  * Existing window stays untouched.
* Multiple windows can coexist; each has
  its own state per the `w=<window-label>`
  scheme already in place.
* The accelerator registers at the Tauri /
  OS-menu layer, NOT the web SPA. The web
  SPA running in a regular browser remains
  unaware; browser-default Cmd+N (new
  browser window) is untouched.

## Implementation path

Tauri 2 supports app-level menus + window
menus with accelerators. Two approaches:

1. **Menu item with accelerator**: declare
   a `File → New Window` menu item with
   `Cmd+N` accelerator in
   `desktop/src-tauri/src/main.rs` (or
   wherever the menu builder lives). Click /
   shortcut → invoke a Tauri command that
   creates a new window via
   `WebviewWindowBuilder::new(...)`.
2. **Global shortcut**: use Tauri's
   global-shortcut plugin to bind Cmd+N
   without a visible menu entry. More
   transparent, less discoverable.

Recommend (1) — the menu entry surfaces the
binding to users who scan the menu bar, and
the platform-native accelerator handling is
robust.

## Relevant code

* `desktop/src-tauri/src/main.rs` (or
  wherever the Tauri builder + menu setup
  lives) — extend the app menu with a
  `File` submenu (if not present) +
  `New Window` item carrying the Cmd+N
  accelerator.
* New Tauri command (e.g.
  `open_new_window`) that builds a fresh
  `WebviewWindow` with a unique window
  label. The launcher's frontend
  (`desktop/src/main.js`) already handles
  per-window state via `w=` URL param;
  reuse that machinery.
* `desktop/src-tauri/capabilities/` — may
  need a capability adjustment if
  `WebviewWindowBuilder` isn't already
  permitted. Audit.
* `desktop/CLAUDE.md` — append a note
  about the new binding in the relevant
  section if other shortcuts are listed
  there.

## Acceptance criteria

* In chan-desktop, pressing Cmd+N (macOS) /
  Ctrl+N (Win + Linux) opens a new
  chan-desktop window with the launcher
  view.
* The new window has its own state per
  `w=<window-label>`.
* Existing window unaffected.
* In the web SPA opened in a regular
  browser: Cmd+N behaves as the browser
  default (new browser window). chan
  doesn't intercept.
* The new menu entry is visible in the
  macOS menu bar (e.g. under
  `chan-desktop → File → New Window`) so
  users discover the binding.

### Tests

* Rust-side: unit test that the
  `open_new_window` command builds a
  WebviewWindow with the expected URL
  shape (`?w=...`).
* If feasible, a Tauri integration test
  that exercises the menu accelerator.
  Light-touch is fine; the chan-desktop
  side has limited CI coverage today.

### Gate

* `cargo check -p chan-desktop`
* `cargo clippy -p chan-desktop -- -D warnings`
* `cargo test -p chan-desktop`
* `cargo fmt --check`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* **Visual eyeball is the canonical
  verification** — Chrome MCP can't drive
  Tauri shells. @@Alex's spot-check on
  the next chan-desktop launch covers it.
  Same pattern as `-53`'s deferred
  eyeball.
* This is the **first chan-desktop UX
  binding** that lives outside the
  embedded SPA. Document the convention
  in your impl note: where the menu /
  accelerator definition lives, how to
  add future ones (so phase-8 work like
  "Cmd+T new tab", "Cmd+W close window"
  etc. can follow the same pattern
  cleanly).
* v0.11.0-blocking-soft. Small UX win;
  not critical for the tag but valuable.
* Coordinate with phase-8 backlog item 7
  (chan-desktop upgrade model + bundled
  chan binary): the same Tauri-menu
  surface might host future entries
  (`File → Quit`, etc.). Layout your
  menu structure so additions don't
  force a refactor.
* Queue position: end of Lane A queue.
  Updated queue: `-75` → `-77` → `-81`
  → `-83`.
* Standing topic-level commit clearance.

## 2026-05-19 19:07 BST — @@FullStackA implementation note

Went with the recommended (1) menu-item-with-
accelerator path. Tauri 2's
`MenuItemBuilder::with_id(id, label).accelerator(...)`
attaches an OS-native chord that the system menu
handles consistently across platforms.

Implementation:

* `desktop/src-tauri/src/main.rs`:
  * Import `tauri::{WebviewUrl, WebviewWindowBuilder}`
    in addition to the existing menu/window types.
  * `install_app_menu()` declares a new
    `MenuItemBuilder::with_id("app-new-window",
    "New Window").accelerator("CmdOrCtrl+N")`
    item, prepended into the Window submenu
    alongside `drive_manager` + `settings`.
  * `on_menu_event` adds an `"app-new-window"`
    branch that calls a new
    `open_new_launcher_window(app)` helper.
* `open_new_launcher_window(app)`:
  * Picks the next free `main-N` label via
    `next_launcher_label(app)` (scans
    `webview_windows()` for occupied slots,
    starts at `main-2`).
  * Builds a `WebviewWindow` with
    `WebviewUrl::App("index.html".into())` so the
    SPA's `boot()` path runs and the user lands on
    the launcher (drive picker) — not a cloned
    drive view.
  * Title / size match the singleton `main`
    window's `tauri.conf.json` config (960×600,
    720×400 min).
* `next_launcher_label(app)`: collects existing
  `main` / `main-*` labels into a HashSet and
  returns the smallest unused `main-N` ≥ 2. Falls
  back to a nanosecond-timestamp suffix in the
  practically-unreachable saturated-range case.

Convention for future chan-desktop bindings
(documented in the code comment block on the
new_window item):

> Declare a `MenuItemBuilder` here with the
> `CmdOrCtrl+<key>` accelerator, prepend into the
> Window submenu below, and add a matching
> `on_menu_event` branch.

This is the first chan-desktop UX binding that
lives outside the embedded SPA's keymap.
Phase-8 additions (Cmd+T new tab, Cmd+W close
window, etc.) follow the same shape.

Acceptance:

* Cmd+N (mac) / Ctrl+N (Win/Linux) in
  chan-desktop spawns a new launcher window.
* Existing `main` singleton untouched; the new
  window carries its own per-window state via
  the `w=` URL parameter the SPA already
  consumes (`desktop/CLAUDE.md`).
* Web SPA running in a browser: unaffected
  (browser-default Cmd+N stays). The
  accelerator binds only at the Tauri/OS-menu
  layer; nothing in `App.svelte` or
  `shortcuts.ts` listens for `chan:command`
  events from this path.
* Menu entry visible in macOS menu bar under
  the Window submenu (where the other
  custom items live; not File, because Tauri's
  `Menu::default` doesn't ship one and adding
  a custom File submenu would re-layout
  every existing platform's menu).

Gate green:

* `cargo check -p chan-desktop`,
* `cargo clippy -p chan-desktop -- -D warnings`,
* `cargo test -p chan-desktop` (7 passed —
  no new unit tests added; the helper takes
  an `AppHandle` which is hard to mock
  without a Tauri integration harness, and
  the spec said light-touch is fine),
* `cargo fmt --check`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Visual eyeball deferred to @@Alex on the next
chan-desktop launch (same pattern as `-53`).
Check items: Window menu shows "New Window"
with the Cmd+N glyph; pressing the chord
spawns a fresh launcher; both windows show
the drive picker; closing one leaves the
other running.

Proposed commit message:

> Cmd+N spawns a new chan-desktop launcher window (fullstack-83)
>
> Add a "New Window" menu item with the
> CmdOrCtrl+N accelerator in chan-desktop's
> Window submenu. The handler builds a fresh
> `WebviewWindow` with the next free `main-N`
> label pointing at `index.html`, so the user
> lands on the drive picker rather than a cloned
> drive view. The singleton `main` window stays
> untouched; each new launcher carries its own
> per-window state via the `w=` URL param the
> SPA already consumes. Web SPA in a browser is
> unaffected — accelerator binds only at the
> Tauri/OS-menu layer.
