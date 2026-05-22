# fullstack-b-26 — Tab right-click "Reload" + "Open Inspector" no-op on chan-desktop (macOS)

Owner: @@FullStackB
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Wire the tab right-click "Reload" + "Open Inspector"
context menu entries through Tauri IPC so they
actually do something on chan-desktop / macOS. Today
they're no-op (clicking either does nothing).

## Reference

[`../phase-8-bugs.md`](../phase-8-bugs.md) "Tab
right-click 'Reload' + 'Open Inspector' entries
no-op on chan-desktop (macOS)" — full bug body
with root cause hypothesis.

## Today's behaviour

Right-clicking a tab in chan-desktop / Tauri webview
on macOS surfaces a context menu with "Reload" +
"Open Inspector" entries. Clicking either does
nothing. Both entries work (or have a browser-default
analogue) in the web build.

## Root cause hypothesis

The SPA's tab context menu defines "Reload" + "Open
Inspector" entries unconditionally (designed against
the web build's browser-default surface), but on
chan-desktop the entries don't have a Tauri IPC
equivalent wired through. Two paths likely missing:

1. **Reload**: should call `tauri::WebviewWindow::reload()`
   or equivalent on the current tab's webview. Could
   be plumbed via a `tab_reload` IPC command.
2. **Open Inspector**: should call
   `tauri::WebviewWindow::open_devtools()` (gated on
   the `devtools` feature flag in `tauri.conf.json`).
   IPC command `open_inspector`.

## Fix shape

Two pieces:

### chan-desktop IPC handlers

Add two new `#[tauri::command]` handlers in
`desktop/src-tauri/src/main.rs` (or wherever the
existing IPC surface lives):

```rust
#[tauri::command]
fn tab_reload(window: tauri::Window) -> Result<(), String> {
    window.webview().reload().map_err(|e| e.to_string())
}

#[tauri::command]
fn open_inspector(window: tauri::Window) -> Result<(), String> {
    #[cfg(feature = "devtools")]
    window.open_devtools();
    Ok(())
}
```

Register in `generate_handler!` alongside the existing
commands.

### SPA wiring

Update the tab right-click menu handler in
`web/src/components/Pane.svelte` (or wherever
tab context menus are defined) to:
* In chan-desktop: invoke `tab_reload` / `open_inspector`
  via the `tauri.invoke` bridge.
* In web build: fall back to today's browser default
  (or hide the entries entirely if they don't apply).

Detection: `import.meta.env.TAURI` is true in
chan-desktop builds; gate the IPC call accordingly.

## Acceptance

1. **Reload works on chan-desktop**: right-click tab
   → "Reload" → tab's webview reloads cleanly.
2. **Open Inspector works on chan-desktop**: right-click
   tab → "Open Inspector" → DevTools opens for that
   webview (gated on debug build / devtools feature).
3. **Web build behavior preserved**: right-click tab
   in web mode still surfaces the entries (or hides
   them if architecturally cleaner) without regression.
4. **No new IPC permission warnings**: Tauri capabilities
   updated if needed.

### Tests

* Rust-side: structural test that `tab_reload` +
  `open_inspector` are registered in `generate_handler!`.
* SPA-side: vitest pin on the right-click menu's
  click handler invoking the correct IPC command in
  chan-desktop mode.

### Gate

* `cargo fmt --check`, `cargo clippy --all-targets --
  -D warnings`, `cargo test -p chan-desktop` green.
* `npm test`, `npm run check`, `npm run build` green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`
  green.

## Coordination

* @@FullStackB lane (chan-desktop runtime + SPA glue).
* Standing chan-desktop runtime perm covers throwaway-
  drive verification.
* Atomic-audit-commit discipline.

## Authorization

**Yes** for:
* `desktop/src-tauri/src/*.rs` (new IPC handlers).
* `desktop/src-tauri/capabilities/*.toml` if Tauri
  capabilities need updating (likely required for
  new IPC commands).
* `web/src/components/Pane.svelte` (or wherever tab
  context menus live).
* Test files.
* Task tail + outbound.

## Numbering

Highest committed `fullstack-b-N` is `-25` (orphan-
detect heuristic + reclaim dialog). This is `-26`.

## Out of scope

* Re-architecting the tab context menu beyond Reload +
  Open Inspector.
* Adding new context menu entries.
* Per-tab DevTools auto-attach.
* Browser-default behavior changes in web mode.
