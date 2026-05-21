# fullstack-b-19: chan-desktop browser-style zoom (Cmd + / - / 0)

Owner: @@FullStackB
Date: 2026-05-21

## Goal

Add browser-style zoom chords to chan-desktop's Tauri webview.
Standard Cmd++ / Cmd+- / Cmd+0 (zoom in / out / reset to
100 %) wired to `WebviewWindow::set_zoom`. Persist the zoom
level per-window in `WindowConfig` (composes with `-b-1`
LRU window-config restore).

## Background

Bug entry:
[`../phase-8-bugs.md`](../phase-8-bugs.md) — "chan-desktop
missing browser-style zoom (Cmd + / - / 0)" (filed
2026-05-20).

Today: standard zoom chords no-op in chan-desktop. Same
chords work natively in a Chrome / Safari tab against the
chan SPA. The `core:webview:allow-set-webview-zoom`
capability is already granted (enabled during `-b-7` for
the opener IPC plumbing) — the underlying API is reachable;
just need explicit accelerator bindings.

## Authorization

**Authorization: yes**, covers:

* `desktop/src-tauri/src/main.rs` (or wherever Tauri
  accelerators are registered, alongside `-a-32`'s
  `KEY_BRIDGE_JS` work + `-b-17`'s reload + devtools
  accelerators).
* `desktop/src-tauri/src/serve.rs` (the
  `WindowConfig` struct from `-b-1`) — add a
  `zoom_level: f64` field with default 1.0.
* `desktop/src-tauri/tauri.conf.json` if any capability
  grant needs widening.

@@FullStackB may proceed without further @@Alex confirmation.

## Acceptance criteria

* **Cmd+=** (and **Cmd++** if the keyboard layout
  distinguishes): zoom in by ~10 % step. Floor / ceiling
  reasonable (Tauri webview accepts ~0.25 - 5.0 typically).
* **Cmd+-**: zoom out by ~10 % step. Floor at the lower
  bound (e.g., 0.25 or 0.5).
* **Cmd+0**: reset to 100 % (zoom level 1.0).
* Each zoom action calls
  `WebviewWindow::set_zoom(new_level)`.
* Per-window persistence: `WindowConfig.zoom_level: f64`
  field. Saved on every zoom event. Restored on next launch
  via `-b-1`'s LRU restore path (the per-window-key keyed
  config picks the right zoom level up automatically).
* Cross-platform: Cmd on macOS, Ctrl on Linux / Windows.
  Bind both via Tauri's accelerator macros following
  `-a-32`'s pattern + `-b-17`'s additions.
* Test pin in `desktop/src-tauri/src/`: assert the
  `WindowConfig` struct gains the `zoom_level` field with
  serde-default 1.0 (backward-compat with existing config
  files); assert the accelerator-binding registry includes
  the three chord names.
* Pre-push gate: clean (chan-desktop cargo test + fmt +
  clippy `-D warnings`).

## How to start

1. Read Tauri 2's `WebviewWindow::set_zoom` API. Confirm
   signature + acceptable range.
2. Locate `WindowConfig` in `desktop/src-tauri/src/serve.rs`
   (per `-b-1`'s landing). Add `zoom_level: f64` with
   `#[serde(default = "default_zoom")] fn default_zoom() ->
   f64 { 1.0 }`. Backward-compat with existing
   `~/Library/Application Support/Chan Desktop/config.json`
   entries (missing field reads as 1.0).
3. Implement the three IPC handlers:
   * `zoom_in` — get current via `get_zoom()` (or track via
     `WindowConfig`), increment, clamp, set, persist.
   * `zoom_out` — same shape, decrement, clamp.
   * `zoom_reset` — set to 1.0, persist.
4. Wire accelerators via the `KEY_BRIDGE_JS` pattern from
   `-a-32` / `-b-17`.
5. Update `-b-1`'s LRU restore path to apply
   `WindowConfig.zoom_level` after the window mounts +
   the SPA finishes loading.
6. Write the unit tests.
7. Test locally:
   * Launch chan-desktop, press Cmd++ a few times → visible
     zoom in. Cmd+- back out. Cmd+0 reset.
   * Close window, reopen → zoom level persists.
8. Append commit-readiness.

## Coordination

* **Composes with [`-b-1`](../fullstack-b/fullstack-b-1.md)** —
  `WindowConfig` struct gets a new field; `-b-1`'s LRU
  restore path picks it up automatically. Backward compat
  via serde-default = 1.0.
* **Composes with `-b-17`** — both touch chan-desktop's
  accelerator bindings + IPC command registry. Coordinate
  on the registration pattern + Test fixtures.
* **Rides v0.11.2 mini-wave** per
  [`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md).
  Parallelisable.

## Open questions

(populated as you investigate)
