# fullstack-a-73 — Cmd+R global shortcut bound to pane right-click "Reload" + annotate the menu entry

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Bind Cmd+R as a SPA-level global shortcut that fires
the pane right-click menu's "Reload" entry. Plus
display "Cmd+R" as the shortcut annotation next to
the menu entry.

**Semantic (@@Alex 2026-05-22 clarification)**: this
is a WINDOW-LEVEL reload of the entire app — like a
browser Cmd+R reload. Calls the existing
`reloadWindow()` helper from `api/desktop.ts` which:
* in chan-desktop → `reload_window` IPC reloads the
  webview.
* in web → `window.location.reload()` reloads the
  browser tab.

NOT a per-tab reload (those exist separately as
"Reload from Disk" on editor tabs + "Restart" on
terminals).

## Reference

@@Alex 2026-05-22: "I would like to add one more
global shortcut, for the pane's right-click menu, the
'reload' for Cmd+R"

## Today's state (audit)

* `web/src/components/Pane.svelte:455` — pane right-
  click menu's Reload entry calls `reloadWindow()`.
* `web/src/api/desktop.ts:47-58` — `reloadWindow()`
  invokes `reload_window` IPC on chan-desktop OR
  falls back to `window.location.reload()` in web.
* `desktop/src-tauri/src/serve.rs:1140` — chan-desktop
  Tauri-side JS bridge already binds `KeyR` →
  `reload_window` IPC.
* **Gap**: no SPA-level Cmd+R keymap binding. Chord
  fires via chan-desktop accelerator OR browser
  default depending on build. The menu entry has no
  visible "Cmd+R" annotation.

## Fix shape

### 1. SPA keymap binding

Add a Cmd+R (Ctrl+R on non-Mac) handler in
`web/src/App.svelte`'s global keymap that calls
`reloadWindow()`. Detection mirrors the existing
pattern (Cmd+T, Cmd+P, etc.):

```ts
// somewhere in the App.svelte global keymap dispatch
if (e.metaKey && !e.altKey && !e.shiftKey && !e.ctrlKey && e.code === "KeyR") {
  e.preventDefault();
  reloadWindow();
  return;
}
```

In chan-desktop the existing serve.rs:1140 path still
covers Tauri-menu fire; the SPA handler also runs but
the IPC is idempotent (window reload). In web, the
SPA handler suppresses the browser-default reload AND
calls `reloadWindow()` which falls through to
`window.location.reload()` anyway — same effect, but
now consistently SPA-controlled.

### 2. Menu entry annotation

Update the pane right-click menu's "Reload" entry to
display "Cmd+R" as the shortcut annotation. Mirror
the existing pattern (Cmd+T / Cmd+P annotations on
other entries).

## Acceptance

1. **Cmd+R reloads the window**: in chan-desktop AND
   web builds, Cmd+R triggers the window reload.
2. **Menu annotation visible**: pane right-click menu
   shows "Cmd+R" next to the Reload entry.
3. **No browser-default fallthrough in web**:
   preventDefault prevents the browser's own reload
   chord from firing twice.
4. **No regression on the IPC path**: chan-desktop's
   existing serve.rs Tauri-side binding still works.

### Tests

Vitest pin on the chord handler + the menu entry
annotation render.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.
* Trivial cross-lane heads-up: chan-desktop's
  serve.rs:1140 binding remains unchanged; no
  Tauri-side edits needed.

## Authorization

Yes for `web/src/App.svelte` + `web/src/components/Pane.svelte`
+ test files + task tail + outbound.

## Numbering

This is `-a-73`.

## Out of scope

* Adding Cmd+R-equivalent to OTHER right-click menus
  (terminal / editor / FB / graph). The Reload entries
  in `FileEditorTab.svelte` + `TerminalTab.svelte`
  from `-b-26` could optionally get the same
  annotation; implementer's call whether to bundle.
* Changing the underlying reload semantic (window
  reload, not per-tab reload).
* Removing chan-desktop's serve.rs:1140 binding (keep
  the redundancy for chan-desktop hardening).
