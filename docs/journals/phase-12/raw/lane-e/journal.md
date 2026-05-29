# Lane E journal: cross-platform keyboard shortcuts

@@LaneE, phase 12. Append-only. Worktree `../chan-lane-e` on branch
`phase-12-lane-e` from main baseline `f72b8a7`. Source code in the worktree;
this journal + channels live in the MAIN checkout per the phase-12 protocol.

## 2026-05-27 - kickoff + audit

Identified, read lane-e-plan.md + bootstrap.md + the spec (addendum-2 request.md
Shortcuts section) + the ratified answers (round-n-review.md Q5-Q9). @@Alex
authorized carrying on autonomously; unblocks go to @@Lead as tasks.

Created worktree `../chan-lane-e` on `phase-12-lane-e` from `f72b8a7`.

Ran the AUDIT (three parallel Explore passes + direct reads of the load-bearing
files). Verdict: the shortcut system is FAR more complete than the spec implies;
@@Alex's "we already have this" instinct is correct. Most of the policy is
already wired. The real work is a small set of gaps + verifications, not
greenfield.

Architecture as found:
- `web/src/state/shortcuts.ts` - the single chord REGISTRY (web + native
  variants, OS label formatting, escapeTerminal flags, the help table).
- `web/src/App.svelte` `onWindowKey` - the WEB keymap (bubble, document-level)
  + `onCtrlDCapture` (capture-phase close) + `handlePaneModeKey` (Hybrid Nav
  chord chain after `Mod+.`) + `runCommand` (the `chan:command` bus that native
  + menus + chords all converge on).
- `desktop/src-tauri/src/serve.rs` `KEY_BRIDGE_JS` - the NATIVE keymap. A
  capture-phase init script that intercepts OS-reserved chords and replays them
  as `chan:command` events (or invokes Tauri IPC for zoom/reload/devtools).
- `desktop/src-tauri/src/main.rs` - native menu accelerators (CmdOrCtrl+,
  settings, CmdOrCtrl+Shift+N new window) + zoom IPC handlers + window
  hide/close lifecycle.
- `web/src/components/FindBar.svelte` + `web/src/editor/{base,find}.ts` - the
  custom find-in-document (NOT CodeMirror's search extension); ESC closes,
  Enter/Shift+Enter advance, scrollIntoView on query-edit + index change only.

Full gap table: `audit.md` (this dir). Posted the summary + open decisions on
`event-lane-e-architect.md` for @@Lead's review before any large change.
Declared web/src + serve.rs touches on the b-e channel (vs @@LaneB codemod) and
TerminalTab.svelte touches on the c-e channel (vs @@LaneC terminal recovery).

Holding for @@Lead's review of the audit + the open decisions before slice i.

## 2026-05-27 - GO from @@Lead; slices i/iii/iv implemented

@@Lead reviewed the audit: APPROVED, @@Alex ruled every open point, GO on all.
Rebased phase-12-lane-e onto 2140925 (chan-drive->chan-workspace crate rename
merged; touched serve.rs). Rulings applied:

- Web pane nav -> Alt+[/] (desktop keeps Cmd+[/]). DONE: App.svelte handler +
  registry web entry + SERVE_LONG_ABOUT + paneModeKeymap.test.ts.
- cmd+s search: wired web (onWindowKey, preventDefault per Q5) + desktop
  (KEY_BRIDGE KeyS -> app.search.toggle, command already existed) + registry
  entry. DONE.
- Splits cmd+/ (right) cmd+\ (bottom), desktop-native only: KEY_BRIDGE
  Slash/Backslash -> app.pane.splitRight/Down; runCommand -> splitActive("row"/
  "column"); native-only registry entries. DONE.
- Close-cascade tail (Q6): closeActiveEmptyPane now, on the LAST empty pane +
  desktop, invokes the new request_close_window IPC (main.rs) which shows the
  launcher then closes the drive window. Added allow-request-close-window to the
  drive-window capability set. Web stays a no-op. DONE.
- Linux ctrl+w (slice iii): @@Alex ruled NO ctrl+w-for-close on Linux. One-line
  fix - gated KEY_BRIDGE KeyW close to metaKey (Cmd, macOS) only, so Linux Ctrl+W
  reaches xterm readline. ctrl+d was already context-aware. Did NOT touch
  TerminalTab.svelte. Seam with @@LaneC dissolved (noted on c-e). DONE.
- Infographics (slice iv): @@Alex ruled BOTH direct cmd+i AND Hybrid Nav `i`.
  Added KEY_BRIDGE KeyI, web onWindowKey cmd+i, handlePaneModeKey `i` case,
  registry entry, SERVE_LONG_ABOUT row. app.infographics.open command pre-existed.
  DONE.

Open: cmd+. f -> cmd+. s rename (ruling #2) COLLIDES with `s` = WASD swap-down.
Flagged on event-lane-e-architect.md with options; left cmd+. f as-is pending
@@Lead/@@Alex. The top-level cmd+s is wired regardless.

FLAGGED: chunk-1 left two Tauri permission names stale (list_drives /
remove_drive) after renaming the commands to list_workspaces / remove_workspace -
runtime IPC denial in the desktop launcher. Raised on event-lane-e-lane-b.md +
CC @@Lead (not my domain to fix).

Gate: web vitest 1601 pass, svelte-check 0 errors, build OK. Rust gate
(clippy/test/build --no-default-features) running. Slice ii (find triad) is
verify-only - browser walkthrough next.

## 2026-05-27 - merged + slice ii closed + cleanup

Full gate green (one test fix: fullstack-42's stale negative assertion
`!KEY_BRIDGE_JS.contains("app.search.toggle")` flipped to the keeps-list since
Cmd+S search is now a first-class bridge chord). Committed fc8310c; reported
ready-to-merge. @@Lead re-gated + MERGED to main as 4cb5ca8 (Merge phase-12-lane-e:
cross-platform keyboard shortcuts (addendum-2 i/iii/iv)).

Slice ii (find triad Q9): VERIFY-ONLY, no find code touched. Rests on the audit's
code trace - FindBar closes on ESC (onKeydown), scrollIntoView fires only on
query-edit (debounced) + next/prev index-change, never idle -> matches Q9. Tried a
browser walkthrough on a scoped test server (/tmp/chan-lane-e-find :4790); @@Alex
declined the Chrome navigate permission. Empirical cmd+f/g/shift+g confirmation
belongs on chan-desktop anyway (web cmd+f is browser-owned), which is @@Alex's
surface. Marking slice ii done on code-analysis; flagged for a desktop spot-check.
Tore down the test server (scoped pkill to my drive), unregistered + rm'd the
drive, closed the browser tab. Clean.

@@LaneC confirmed on c-e they are NOT touching handleTerminalKeyEvent / focus
tracking - and my metaKey-gate approach meant I never needed a terminal-focus
signal, so the c-e seam closed with zero coupling.

PENDING (not my code to land):
- cmd+. f -> cmd+. s ruling (#2): collides with `s` = WASD swap-down. Left cmd+. f
  as-is; top-level cmd+s shipped. Awaiting @@Lead/@@Alex. If ruled, tiny follow-up.
- Two chunk-1 rename regressions I found + flagged to @@LaneB (CC @@Lead), both
  runtime-only (pass cargo test): (1) app.toml grants list_drives/remove_drive but
  commands are list_workspaces/remove_workspace -> launcher IPC denied at runtime;
  (2) handoff variant open_workspace vs deserializer open_drive -> chan open ->
  desktop handoff broken. Offered to take the small fixes since I'm in app.toml.

Lane E work for this round is essentially complete pending the cmd+. f ruling.

## 2026-05-27 - cmd+. f RESOLVED (option a); rebased; lane done

@@Lead relayed @@Alex's ruling on the open item: option (a) - KEEP cmd+. f, keep
WASD swap-down on `s` (@@Alex had missed the WASD collision; swap is load-bearing).
fc8310c ALREADY does this, so NO code change. The cmd+. f -> cmd+. s half of
round-2 ruling #2 is WITHDRAWN. Standing constraint going forward: WASD (any case)
owns swap-tile in Hybrid Nav - don't rebind `s`.

@@Lead also confirmed the two chunk-1 rename artifacts (stale Tauri perm names +
open_workspace/open_drive handoff variant) are routed to @@LaneB as a chunk-1b
fixup - I do NOT take the app.toml 2-liner; @@LaneB owns rename completeness.

Rebased phase-12-lane-e onto current main 4cb5ca8 (my fc8310c absorbed as the
merge; picked up A3 / C-follow-up / D-RPM). Branch == main, no unique commits.

Lane E this round: COMPLETE. All slices merged (4cb5ca8); slice ii find-triad is
verify-only and rests on code analysis (desktop spot-check is @@Alex's surface);
the one open ruling is resolved with no code change. Nothing queued on my side.

## 2026-05-27 - FREEZE LIFTED add-on: Cmd+R pane menu (already done)

@@Lead routed a new nit (post chunk-2, main bce6bd3): wire Cmd+R to the pane
right-click 'Reload' + show the accelerator label + document. Rebased onto
bce6bd3. Investigated - it's "we already have this" (fullstack-a-73). No
functional change:
- Pane 'Reload' = WINDOW reload: doReloadPane() (Pane.svelte:469) -> reloadWindow().
- Cmd+R global: App.svelte:791 (meta+KeyR -> reloadWindow) + serve.rs KEY_BRIDGE:613
  (Cmd+R -> reload_window IPC). Same action as the menu item.
- Label already shown: Pane.svelte:1256 chordLabel("app.window.reload") -> "Cmd+R".
- Documented: SERVE_LONG_ABOUT:83 "Reload window  Cmd+R".
Reported to @@Lead with the determination (pane Reload = window reload) + evidence.
Offered optional follow-ups: (a) fix stale "serve.rs:1140" comment pointer in
Pane.svelte:1251 (needs a cmdRWindowReload.test.ts regex update too); (b) scope a
distinct PANE-scoped reload if that's what @@Alex actually wants (new feature);
(c) empirical screenshot (needs browser/desktop). Awaiting steer; no merge.
