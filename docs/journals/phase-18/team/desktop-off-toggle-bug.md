# Desktop OFF-toggle lifecycle race (captured from desktop-bug-report/)

Captured here by @@LaneE (release lane) before deleting the untracked
`desktop-bug-report/` draft dir at @@Alex's request ("don't leave drafts
behind"). This is the source bug for the phase-18 close-out `fix(desktop): ...`.

## Symptom (reported by @@Alex, with screenshots)
Turning a workspace OFF in chan-desktop flips the row's ON/OFF toggle to OFF
in the UI BEFORE the underlying chan-server has actually shut down. A
subsequent click (turning it back ON) then fails with the banner:

    "This workspace is open in another chan process. Quit it and try again."

and leaves the UI in a broken state: the row shows ON but the **Open** button
is greyed out / unavailable (screenshots showed `chan-prod-setup` ON with a
disabled Open + the error banner).

## Root cause (code analysis)
A toggle / lifecycle RACE between the UI and the server-shutdown completion:
- `desktop/src/main.js` toggle handler (`data-act="toggle-on"` change):
  the NATIVE checkbox flips visually the instant it is clicked, and nothing
  disables the control during the in-flight transition, so a second click
  fires while the first transition is still settling.
- `desktop/src-tauri/src/main.rs set_workspace_on(off)` -> `serve::stop` ->
  `host.close_workspace(prefix)` removes the runtime synchronously, but the
  workspace flock can linger briefly: a background indexer / in-flight
  request still holding an `Arc<Workspace>` releases the flock a moment
  later (the same window `unregister_with_retry` exists for).
- A rapid OFF->ON therefore hits `library.open_workspace(root)` while the
  flock is still held -> `WorkspaceAlreadyOpen` / `WorkspaceLocked` -> the
  "open in another chan process" error.
- `list_workspaces` derives BOTH `on` and `url` from the live serve handle,
  and `refresh()` dedupes on the workspace-list JSON, so after a failed
  re-enable the DOM (carrying the user's stray checkbox flip) is not
  reconciled back to the true state -> stuck "ON but no Open".

## Fix (@@LaneE, phase-18 close-out)
Per @@Lead's handoff: make the toggle reflect ACTUAL server state - disable
the control until the transition completes + reconcile the DOM to the true
serve state.
- Frontend (`main.js`): disable the toggle for the whole transition (serve
  start/stop + flock release), then force a re-render (bypass the
  list-JSON dedupe) so the toggle + Open reconcile to the real serve state
  even when the net registry JSON is unchanged.
- Backend (`src-tauri set_workspace_on`): assessed - see the commit.
WKWebView-only surface; smoked by @@LaneE/@@Alex in chan-desktop (Chrome
automation can't drive WKWebView). Committed as `fix(desktop): ...`.
