# task LaneE -> Lead (2): caller-free confirm + osChord divergence DONE

Both follow-ups done, gates green. This is the last shortcuts step on my
side -> you're clear to run the single resync.

## FLAG 1 caller-free CONFIRM (+ one real catch)

Grepped the WHOLE repo (all *.rs/*.js/*.ts/*.svelte/*.toml/*.json).
`compute_workspace_preflight` has NO remaining caller. Specifically:
- main.js: gone (renderLocal registers via add_workspace, no scan).
- main.rs: command + PreflightReport + 6 helpers + 4 unit tests +
  generate_handler! registration all removed.
- NO cloud / remote / tunnel / outbound flow ever invoked it (the only
  caller was the local renderLocal path).
- Only remaining string match is MY negative-assertion test in serve.rs
  (`!MAIN_RS.contains("fn compute_workspace_preflight(")`) - asserts it's gone.

CATCH (the grep earned its keep): two STALE references in TRACKED files
the cargo gate did NOT flag (Tauri perms are build/runtime-validated, not
cargo-checked - the "gate-blind wire" class):
- `desktop/src-tauri/permissions/app.toml`: the `main-window` set listed
  `allow-compute-workspace-preflight` AND defined that permission
  (`commands.allow = ["compute_workspace_preflight"]`). Removed both. A
  defined permission for a removed command is exactly the "no pre-flight in
  chan-desktop" debt @@Alex wants gone; left in, it'd have ridden your
  commit.
- `desktop/src-tauri/gen/schemas/acl-manifests.json`: GITIGNORED build
  artifact; rebuilt `cargo build -p chan-desktop` -> regenerated clean (0
  refs). Not committed.
Re-ran `cargo test -p chan-desktop`: 74 + 7 pass (incl. the contract test).

## CAVEAT osChord cross-platform fix DONE

shortcuts.ts: applied the reload `osChord` pattern to terminal copy/paste.
- Added `TERMINAL_COPY_ID` / `TERMINAL_PASTE_ID` consts + two lines in
  `osChord`: on `os !== "mac"`, copy -> `Mod+Shift+C`, paste -> `Mod+Shift+V`
  (render to Ctrl+Shift+C/V). macOS keeps the stored Cmd+C/V. KEPT the
  reload line verbatim so cmdRWindowReload.test.ts's source-pin stays green;
  generalized the now-plural doc comment.
- Result per OS: mac Cmd+C / Cmd+V; Linux / Windows Ctrl+Shift+C /
  Ctrl+Shift+V (display + help table now match the handler, which already
  split this way). The displayed-hint bug is gone.
- Added `terminalCopyPasteChords.test.ts` (mirrors cmdRWindowReload's
  pattern): pins the registry entries + notes + the osChord divergence lines
  + the TerminalTab per-OS detection + the menu chordFor hints.
- Did NOT run shortcuts-table.mjs (yours, once).

## Gate (re-run on the live shared tree)
- make web-check: green - svelte-check 0 errors (1 pre-existing a11y
  WARNING in RichPrompt.svelte, not mine), 1692 vitest pass (+ my new file),
  build ok.
- cargo build -p chan-desktop: ok (regenerated gen/ clean).
- cargo test -p chan-desktop: 74 + 7 pass.

## Pathspec delta (this round; base HEAD d5f7dd38; NOT committed)
NEW since task-LaneE-Lead-1:
```
desktop/src-tauri/permissions/app.toml          (removed dead permission)
web/src/components/terminalCopyPasteChords.test.ts   (new test)
```
SHARED (you merge + resync): `web/src/state/shortcuts.ts` now has C's
app.files.delete + my terminal.copy/paste entries + the "Terminal" union
member + the osChord divergence for terminal.copy/paste.
NOT committed: desktop/src-tauri/gen/** (gitignored build artifacts).

## You're clear for the single resync
osChord is in + web-check green. Run `node web/scripts/shortcuts-table.mjs`
ONCE now (C's File chords + my Terminal chords all present) to sync
crates/chan/src/main.rs SERVE_LONG_ABOUT. That's the last shortcuts step.
