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

## 2026-05-22 — ready for review

Four-file change. SPA-only.

### What landed

`web/src/state/shortcuts.ts`: registry entry for
`app.window.reload` (web + native: `Mod+R`, group:
App). Lets `chordLabel("app.window.reload")` render
the platform-aware annotation on the menu entry.

`web/src/App.svelte`:

* Imports `reloadWindow` from `./api/desktop`.
* New keymap branch in `onWindowKey` —
  `Cmd+R / Ctrl+R` (matches existing `meta` test
  pattern, with `!e.altKey && !e.shiftKey &&
  !e.ctrlKey` filters so chord doesn't fire on
  Cmd+Shift+R / Cmd+Alt+R etc.). Calls
  `void reloadWindow()` + `e.preventDefault()`.

`web/src/components/Pane.svelte`:

* Reload menu entry restructured from a bare
  `<span>Reload</span>` to the standard
  `menu-row-label` + `menu-row-chord` two-span
  shape that other annotated rows use, with
  `chordLabel("app.window.reload")` rendering the
  chord. Comment block documents the dual entry
  point + chan-desktop's
  `serve.rs:1140` defense-in-depth.

`web/src/components/cmdRWindowReload.test.ts` (new):
5 raw-source pins covering the registry entry,
reloadWindow import, keymap handler shape, menu
annotation render, and the dual-entry-point
comment.

### Acceptance

1. **Cmd+R reloads the window**: SPA handler
   dispatches `reloadWindow()` ✓ (mechanism via
   tests; UI walk by @@WebtestA for empirical
   confirmation).
2. **Menu annotation visible**: Reload entry
   renders `chordLabel("app.window.reload")` →
   "⌘R" on macOS, "Ctrl+R" elsewhere ✓.
3. **No browser-default fallthrough on web**:
   `e.preventDefault()` suppresses the browser
   reload chord ✓.
4. **No IPC regression**: chan-desktop's
   serve.rs:1140 Tauri binding stays unchanged ✓
   (not touched in this commit).

### Gate

* vitest **814 / 814** (+5 net from `-a-72`'s
  809).
* svelte-check 0 errors / 0 warnings across
  4009 files.
* npm build clean.
* Rust gate not re-run (no Rust touched; task body
  explicitly preserves the serve.rs binding).

### Decisions

* **`!e.ctrlKey` AND `!e.altKey` AND `!e.shiftKey`
  filters** — strict modifier match. Cmd+Shift+R is
  the browser's "hard reload" chord; we don't want
  to intercept it (let the browser do its thing).
* **`void reloadWindow()`** — `reloadWindow()`
  returns a Promise that on chan-desktop awaits
  the IPC; on web it calls `window.location.reload()`
  synchronously (the function doesn't return).
  `void` suppresses the
  unhandled-promise-rejection lint without
  blocking the keymap dispatch.
* **Same registry entry across web + native** —
  `Mod+R` resolves to ⌘R on Mac (web + native)
  and Ctrl+R on Linux/Windows. No platform
  divergence.
* **Reload entries on other surfaces deferred** —
  task body's out-of-scope. `-b-26` shipped per-
  tab "Reload from Disk" on editor + "Restart" on
  terminal; those have different semantics. Could
  bundle the annotation as a polish follow-up if
  @@Alex wants consistency.

### Suggested commit subject

```
Cmd+R global chord → window reload; annotate pane Reload entry (fullstack-a-73)
```

Single commit. Registry + keymap + menu + test
tightly coupled.

### Files for `git add` (per-path discipline)

* `web/src/state/shortcuts.ts`
* `web/src/App.svelte`
* `web/src/components/Pane.svelte`
* `web/src/components/cmdRWindowReload.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-73.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
