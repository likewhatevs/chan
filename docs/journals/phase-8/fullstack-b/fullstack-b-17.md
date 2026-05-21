# fullstack-b-17: Tab right-click Reload + Open Inspector (Tauri IPC)

Owner: @@FullStackB
Date: 2026-05-21

## Goal

Tauri side of the chan-desktop dev meta-blocker fix. Expose
two Tauri IPC commands the SPA tab context menu invokes for
its Reload + Open Inspector entries; wire `Cmd+R` /
`Cmd+Opt+I` accelerators for keyboard parity. Verify
`tauri.conf.json` `app.devTools` is enabled at runtime.

## Background

Bug entry:
[`../phase-8-bugs.md`](../phase-8-bugs.md) — "Tab right-click
'Reload' + 'Open Inspector' entries no-op on chan-desktop
(macOS)" (filed 2026-05-21).

Paired with [`../fullstack-a/fullstack-a-36.md`](../fullstack-a/fullstack-a-36.md)
(SPA dispatch + runtime detection). `-a-36` calls the IPC
commands this task exposes.

Severity: **DEV META-BLOCKER** — without Open Inspector
working on chan-desktop, @@Alex (and webtest lanes with
chan-desktop runtime permission per `ada8478`) can't
DevTools-inspect any of the other desktop-native UX bugs
in the v0.11.2 wave (file-moved false-positive, notification
spinner stuck, etc.). This task + `-a-36` unblock the rest of
the wave's investigation work.

## Authorization

**Authorization: yes**, covers:

* `desktop/src-tauri/src/main.rs` (or wherever Tauri IPC
  handlers are registered) — new `reload_window` +
  `open_devtools` commands.
* `desktop/src-tauri/tauri.conf.json` — confirm
  `app.devTools` is enabled (or the equivalent Tauri 2 key
  gating devtools in release builds; flip to true if gated
  away).
* `desktop/src-tauri/src/key_bridge.rs` (or wherever
  `KEY_BRIDGE_JS` lives — established by `-a-32`'s chord
  migration) — wire `Cmd+R` + `Cmd+Opt+I` accelerators to
  the new IPC commands.

@@FullStackB may proceed without further @@Alex confirmation.

## Acceptance criteria

* New Tauri IPC command `reload_window` that calls
  `WebviewWindow::reload()` (or eval'd
  `location.reload()` as fallback if Tauri 2's API differs).
* New Tauri IPC command `open_devtools` that calls
  `WebviewWindow::open_devtools()`.
* `tauri.conf.json` `app.devTools` (or equivalent) set to
  `true` for both debug + release builds (or document why
  release should hide devtools — but @@Alex's framing
  implies dev/release parity for the inspector affordance).
* Both commands are reachable via SPA `__TAURI__.invoke(...)`
  calls.
* Tauri-side accelerators:
  * `Cmd+R` → invokes `reload_window`.
  * `Cmd+Opt+I` (macOS) / `Ctrl+Shift+I` (Linux / Windows
    if @@Alex adds those platforms) → invokes
    `open_devtools`.
* Unit / integration test in
  `desktop/src-tauri/src/main.rs` (or adjacent) pinning the
  IPC command registration + the accelerator binding.
* Pre-push gate: clean (chan-desktop cargo test + fmt +
  clippy `-D warnings`).

## How to start

1. Read Tauri 2's WebviewWindow API docs for `reload` +
   `open_devtools`. Confirm the methods exist on Tauri 2.x
   (the chan-desktop tauri-bundler version per
   `fullstack-b-15`'s investigation).
2. Audit `tauri.conf.json` — find `app.devTools` (or
   whichever Tauri 2 key controls it). If false / absent,
   flip to true.
3. Register the two IPC commands in `main.rs`. Each is a
   thin wrapper that grabs the WebviewWindow via
   `app.webview_windows()` or equivalent + calls the API.
4. Wire the accelerator bindings via `KEY_BRIDGE_JS`
   following `-a-32`'s pattern.
5. Add the test pin.
6. Test locally:
   * `make app` build → launch → `Cmd+R` reloads, `Cmd+Opt+I`
     opens DevTools.
   * Right-click a tab in the SPA (after `-a-36` lands) →
     Reload / Open Inspector → see the same behaviour.
7. Append commit-readiness.

## Coordination

* **Pairs with [`-a-36`](../fullstack-a/fullstack-a-36.md)** —
  SPA dispatch consumes the IPC commands exposed here. Lock
  the IPC command names + signatures early so @@FullStackA
  can scaffold against them.
* **Rides v0.11.2 mini-wave** per
  [`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md).
  Priority 1 in the wave's critical path (DEV META-BLOCKER
  unlock).

## Open questions

(populated as you investigate)
