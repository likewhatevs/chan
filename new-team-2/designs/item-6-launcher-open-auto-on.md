# Item 6 — launcher Open: always enabled, auto-turn-on, failure dialog

Lane: @@Desktop. Desktop-only; **no Rust changes needed**. Line
numbers from main @ 3ebee587. Note: the launcher is plain JS
(`desktop/src/main.js`), not the web/ SPA.

## Today

- Open is disabled until the workspace is on:
  `renderOpenSplit()` main.js ~899-918 —
  `const openDisabled = hasUrl ? '' : 'disabled'` (~900), button ~908.
  `hasUrl` (~814) comes from `list_workspaces`
  (desktop/src-tauri/src/main.rs ~333-405): url only populated while a
  serve handle exists.
- Turn-on failure is effectively silent: toggle handler (~976-996)
  invokes `set_workspace_on` (main.rs ~533-546) → `serve::start`
  (serve.rs ~65-94) → `embedded.open_workspace` (embedded.rs ~62-96,
  8×150ms retry on transient locks) → `map_open_error` (~158-173)
  already returns a user-friendly string — notably
  "This workspace is open in another chan process. Quit it and try
  again." for WorkspaceLocked/WorkspaceAlreadyOpen. The JS catch
  (~993) shows it in a 5-second auto-dismiss `.error-banner`
  (~1088-1096) while `refresh(true)` (~995) reconciles the pill back
  to off — the user mostly just sees the pill flip back.
- Open handler (~998-1006) invokes `open_local_workspace` (main.rs
  ~908-923), which errors "workspace {key} is not running" when off
  (~920) and otherwise spawns the workspace window.

## Changes (all in desktop/src/main.js + styles.css)

### A. Open always enabled + auto-turn-on
- ~900: drop the disabled gating (`const openDisabled = ''` or remove
  the attribute entirely).
- Launch handler (~998-1006): when the workspace is off (`!hasUrl` /
  row state), first `await invoke('set_workspace_on', { path, on: true })`;
  on success `await refresh(true)` then
  `invoke('open_local_workspace', { path })`. On turn-on failure: show
  the failure dialog (below) and stop — do not attempt open. Guard
  against double-clicks while the turn-on is in flight (disable the
  button for the duration, restore after).
- Keep the split-menu behavior of renderOpenSplit intact for the
  other entries.

### B. Turn-on failure dialog (replaces the banner for turn-on)
- New `showTurnOnFailureDialog(reason)` modeled on the existing modal
  pattern `showMissingDefaultWorkspaceDialog` (~227-297): reuse
  `.preflight-overlay` + `.preflight-dialog` (styles.css ~506-557),
  `role="dialog"` `aria-modal="true"`, title "Cannot turn on
  workspace", body = the error string from Rust verbatim (it is
  already user-friendly for the lock case and includes the cause
  chain otherwise), OK button (focused), Escape + backdrop-click
  close. Remove the keydown listener on close (the existing examples
  leak it — don't copy that bug; use { once: true } or explicit
  removal).
- Use it in BOTH paths that can turn on: the pill toggle handler
  (~976-996) and the new launch-handler turn-on. Keep `showError` for
  other errors (e.g. open_local_workspace failing while running).
- The `refresh(true)` reconciliation stays — pill back to off is
  CORRECT, the dialog explains why.

## Tests

No launcher test harness exists (desktop JS is untested today); this
stays manual + review. If cheap, a pure-function extraction (e.g.
deriving button state from row data) with a small node test is
welcome but not required.

## Verification

1. Happy path: workspace off → click Open → pill turns on, window
   opens. Open while already on → unchanged behavior.
2. Failure path: hold the flock from a shell
   (`chan serve <that-workspace-path>` — NOT --standalone is fine,
   any process holding it) → click Open (and separately, the pill) →
   dialog appears with the in-use reason; dismiss via OK, Escape,
   backdrop; pill consistent (off) after.
3. Toggle off path unchanged; remote-workspace rows unaffected.
4. Run as a real desktop build (`make desktop-dev` or the bundle) —
   this is a WKWebView/launcher surface; include in @@Alex's final
   smoke checklist.
