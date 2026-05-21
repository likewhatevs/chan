# fullstack-a-36: Tab right-click Reload + Open Inspector (SPA dispatch)

Owner: @@FullStackA
Date: 2026-05-21

## Goal

SPA side of the chan-desktop dev meta-blocker fix. The tab's
right-click context menu has "Reload" + "Open Inspector"
entries. On chan-desktop (Tauri webview) these no-op today
because the entries' click handlers were designed for the
browser-default behaviour. This task makes them work on
chan-desktop via Tauri IPC, while preserving the existing
web-build behaviour.

## Background

Bug entry:
[`../phase-8-bugs.md`](../phase-8-bugs.md) — "Tab right-click
'Reload' + 'Open Inspector' entries no-op on chan-desktop
(macOS)" (filed 2026-05-21).

Paired with [`../fullstack-b/fullstack-b-17.md`](../fullstack-b/fullstack-b-17.md)
(Tauri IPC + accelerator bindings). `-b-17` exposes:

* `__TAURI__.invoke('reload_window')` — calls Tauri 2's
  `WebviewWindow::reload()`.
* `__TAURI__.invoke('open_devtools')` — calls
  `WebviewWindow::open_devtools()`.

This task wires the SPA tab context menu's existing entries
to call those IPC commands when running under chan-desktop;
falls back to `window.location.reload()` (web) or a
no-op-with-toast (web inspector) otherwise.

Severity: **DEV META-BLOCKER**. Without Open Inspector working
on chan-desktop, @@Alex can't DevTools-inspect any of the
other desktop-native UX bugs (file-moved false-positive,
notification spinner stuck, etc.). This task + `-b-17` unblock
the rest of the v0.11.2 wave's investigation work.

## Authorization

**Authorization: yes**, covers:

* `web/src/components/Pane.svelte` (or `TabStrip.svelte` —
  wherever the tab context menu lives; grep for "Reload" or
  "Open Inspector" string).
* `web/src/api/desktop.ts` (or similar runtime-detection
  helper) — chan-desktop runtime detection.
* `web/src/state/*.ts` — any state-mod required for the
  dispatch helper.

@@FullStackA may proceed without further @@Alex confirmation.

## Acceptance criteria

* Right-clicking a tab on chan-desktop → Reload → reloads
  the tab's content via Tauri IPC. For file tabs,
  re-fetches the file from chan-drive. For terminal tabs,
  Reload either no-ops with a status message ("Reload not
  supported for terminal tabs") OR scopes the menu so
  non-file tabs don't show Reload (implementer picks; the
  no-op-with-status is simpler).
* Right-clicking a tab on chan-desktop → Open Inspector →
  opens Tauri's DevTools for the chan-desktop window. Same
  DevTools UX as Chrome (element tree, console, network).
* On web build: Reload calls `window.location.reload()`;
  Open Inspector either hides from the menu OR shows a
  brief toast like "Use the browser's built-in inspector
  (Right-click → Inspect Element)".
* Runtime detection: check `window.__TAURI__` (or
  equivalent) to decide which branch to dispatch.
* Unit test pin: assert the dispatcher routes to the
  expected IPC command on chan-desktop fixture +
  `window.location.reload()` on web fixture.
* Pre-push gate: clean (vitest + svelte-check + npm build).

## How to start

1. Wait for `-b-17` to commit OR coordinate at task-cut so
   the IPC command names + signatures are locked. The IPC
   surface is the load-bearing API contract.
2. Grep the SPA source for the tab context menu definition
   ("Reload" / "Open Inspector" / "Inspect" strings). Find
   the entries' click handlers.
3. Implement / update the runtime-detection helper if not
   already present. Likely a small module that exports a
   boolean `isTauriDesktop` derived from `window.__TAURI__`.
4. Update the click handlers to branch on runtime + invoke
   the appropriate path (IPC vs `window.location.reload()`).
5. Add the vitest pin.
6. Test locally:
   * chan-desktop dev build (after `-b-17` lands): right-click
     a file tab → Reload → see the content reload. Open
     Inspector → see DevTools.
   * web build (against a regular browser tab): same
     menu entries with the web-appropriate behaviour.
7. Append commit-readiness.

## Coordination

* **Pairs with [`-b-17`](../fullstack-b/fullstack-b-17.md)** —
  hard dependency on the Tauri IPC surface. Can scaffold
  the SPA dispatch shape against placeholder
  `__TAURI__.invoke` calls; finalise once `-b-17` commits.
* **Rides v0.11.2 mini-wave** per
  [`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md).
  Priority 1 in the wave's critical path (DEV META-BLOCKER
  unlock).

## Open questions

(populated as you investigate)
