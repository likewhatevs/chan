# journal-LaneC (File Browser)

Append-only running log for @@LaneC, phase-18 round-1.

## 2026-06-04 - bootstrap

- Self-identified from `$CHAN_TAB_NAME=@@LaneC`.
- Read `team/bootstrap.md` (process) and `team/round-1-plan.md` (work).
- My lane = File Browser. Scope (from round plan, re-verify vs HEAD):
  1. Context-menu regression: tab right-click menu merged with docked
     file-browser menu. Remove "Reload". Below "Expand all directories"
     add (from WORKSPACE ROOT): "New file or Directory", "New Terminal",
     "New Graph". Anchors: FileBrowserSurface ~637-748, FileTree in-row
     menu ~1353-1417, handlers newFileOrDir/terminalFromHere/graphThis
     ~552-657.
  2. Keyboard-shortcut hints in context menu (image-6.png): New Terminal
     cmd+t, New Graph cmd+shift+m, Delete = backspace, Settings cmd+,.
     Read from shortcuts.ts chordFor; empty chord spans at
     FileBrowserSurface ~669-706. Record any missing chord in
     shortcuts.ts (ports to linux/macos/web).
  3. Loading hang: expand dir stalls on "Loading"; console
     "SecurityError: history.replaceState more than 100 times / 10s"
     (image-9.png). Root: expand -> persistLayoutToHash (FileBrowserSurface
     ~250-259) -> persistStateToHash (store ~1597) calls replaceState w/o
     debounce. Debounce/coalesce the hash write.
- Owned files: web/src/components/{FileBrowserSurface,FileTree,
  HamburgerMenu}.svelte, menuClamp.ts, store.svelte.ts (persist region
  ~1569-1598), App.svelte (layout-persist effects ~160-217), shortcuts.ts
  (FB chord additions).
- Shared-file notes: shortcuts.ts (C then E, Lead sequences + single
  resync); store.svelte.ts (B graph region vs C persist, far apart);
  App.svelte (C layout effects vs E rich-prompt, far apart);
  fromHere.ts (D owns, C consumes as-is); tabMenu.svelte.ts (coordinate
  if both add fields).
- STATUS: holding. Waiting for @@Lead task poke before editing code.

## 2026-06-04 - Wave 1 implementation (task-Lead-LaneC-1)

Poked by @@Lead; read task-Lead-LaneC-1.md. Verified anchors vs HEAD,
viewed image-6 (selection menu) + image-9 (replaceState SecurityError).

Item 1 - context-menu regression (FileBrowserSurface.svelte, exclusive):
- Removed the "Reload" entry + the now-dead `reloadTree` handler and the
  unused `refreshTree` import.
- Below "Expand all directories" added 3 workspace-root actions:
  New file or Directory (`fileOps.createFileOrDir("")`), New Terminal
  (`openTerminalInPane(layout.activePaneId, terminalFromHereTarget("",
  true))`), New Graph (`openFsGraphForDirectory("")`). New handlers
  newFileOrDirFromRoot/newTerminalFromRoot/newGraphFromRoot. Imports
  added: FilePlus, Terminal as TerminalIcon (lucide); layout,
  openTerminalInPane (tabs); terminalFromHereTarget (consumed as-is per
  shared-file rule, no signature change).
- The TAB-menu root entries carry NO chord (empty menu-row-chord) - chords
  are item-2's selection-menu scope per the spec. Decision: kept the tab
  menu minimal.

Item 2 - shortcut hints in the selection menu (FileTree.svelte + shortcuts.ts):
- The "from selection" in-row menu (image-6) now shows chords via
  chordFor() on: New Terminal (app.terminal.toggle), New Graph
  (app.graph.toggle), Delete (app.files.delete - NEW), Settings
  (app.settings.toggle). Restructured those 4 buttons to
  menu-row-label + menu-row-chord (the :global classes from
  HamburgerMenu); other rows stay plain spans.
- shortcuts.ts: APPENDED ONE entry `app.files.delete` (label "Delete file
  or directory", web+native "Backspace", group "File", escapeTerminal
  unset/false - bare Backspace must stay shell-Backspace; chordFromEvent
  ignores modifierless keys so it can't escape anyway). Backspace/Delete
  was already wired in FileTree.onTreeKeydown; this just records it
  centrally so it ports + the hint reads from the store.
  @@Lead: do NOT resync yet - sequence E's shortcuts.ts append, then run
  `node web/scripts/shortcuts-table.mjs` ONCE.

Item 3 - loading hang (store.svelte.ts persist region + FileBrowserSurface
+ App.svelte layout effects):
- Root cause confirmed: FB expansion effect re-runs on every
  `fbTreeInstances.byId` reassignment churn during expand and recomputes
  the SAME hash; the synchronous persistStateToHash fired
  history.replaceState each time -> WebKit >100/10s SecurityError -> hang.
- Fix: (a) DEDUP in persistStateToHash - skip replaceState when the
  computed URL == window.location.href (collapses the churn storm to 0
  writes); (b) DEBOUNCE - new `schedulePersistStateToHash()` (150ms
  trailing) used by the FB expansion effect + App's two layout effects;
  (c) persistStateToHash stays SYNCHRONOUS (cancels the pending timer) so
  the pagehide/beforeunload flush (App.svelte:1078, OUTSIDE my region,
  unchanged) + the store.test seam keep their immediate write.
- Note: schedulePersistStateToHash is a pure timer setter (no $state
  read/write), so it does NOT alter the reactive graph - no Svelte-5
  reactivity hazard introduced.

Tests updated (source-pins guarding my changed components):
- perTabInspectorWidth.test.ts: persistLayoutToHash -> schedulePersistStateToHash.
- fileBrowserRightClickRevamp.test.ts: Reload pins -> root-spawn order +
  "Reload removed" + import-after-root-spawn band.
- fileTreeSelectionMenu.test.ts: New Terminal/New Graph/Delete spans now
  menu-row-label; ADDED a chord-hint test (chordFor wiring).
- fileBrowserUnifiedDialog.test.ts: Settings span now menu-row-label.

Own-gate GREEN: svelte-check 0 errors (1 pre-existing RichPrompt a11y
warning, @@LaneE's file); vitest 169 files / 1679 tests pass; npm run
build OK.

Shared-file state at report time:
- store.svelte.ts: my persist hunk (~1562-1647) + @@LaneB's
  graphReloadSignal hunk (~1952) coexist (far apart, interleave-safe per
  plan). NOT committed - @@Lead merges/commits.
- App.svelte: only my changes present (E's rich-prompt region untouched).
- shortcuts.ts: only my app.files.delete append present (E not yet).
- Did NOT commit (FileBrowserSurface/App import schedulePersistStateToHash
  from store; a partial commit wouldn't compile, and committing the shared
  files would pre-empt @@Lead's merge role). Reported diff-content shas
  instead.

Runtime smoke (menu render + chord alignment in .ctx + loading-hang):
DEFERRED to the Wave-2 Lead-coordinated convergence server per the round
plan (smokes are Wave 2; client-choice question pending to @@Alex). Ready
to drive the FB-area smoke when the shared server is up.

STATUS: Wave 1 code complete + own-gate green. Cut completion task to
@@Lead; awaiting convergence/smoke sequencing.

## 2026-06-04 - Wave 1 ACCEPTED (task-Lead-LaneC-2)

@@Lead accepted all 3 items. Notes:
- @@Lead owns the store/App/shortcuts MERGES (Wave 3 / convergence). My
  fingerprints recorded for drift-check: clean 0ccda12b (8 files, excl
  store), full 6aac8719 (9 files incl B's store hunk).
- @@Lead confirmed my full-tree 1679-PASS resolved the transient reds A
  (fileTreeSelectionMenu) + D (FileBrowserSurface refreshTree /
  fileBrowserRightClickRevamp) saw while I was churning those files.
- shortcuts.ts: my single app.files.delete append landed; @@Lead released
  @@LaneE to append its terminal copy/paste chords AFTER mine, then runs
  the resync ONCE + commits main.rs. DIRECTIVE: do NOT touch shortcuts.ts
  again. Acknowledged - hands off shortcuts.ts.
- Wave-2 FB smoke (menu render, .ctx chord alignment, no replaceState
  SecurityError on expand): @@Lead builds the clean persistent server at
  convergence (after A item 4 + B + E land) + resolves the smoke-client
  question with @@Alex, then pokes me. "Nothing pending."

STATUS: STANDBY for the Wave-2 FB smoke poke. No code changes; not touching
shared files; not spinning a server.

## 2026-06-04 - round close / STAND DOWN

@@Lead: round work DONE + committed (9fcf0187); CLEARED to stand down;
@@LaneE owns the release from here.

Verified my work is in committed history (read-only grep against HEAD,
which has since advanced to 688955c5 with later lane work layered on top):
- store.svelte.ts: schedulePersistStateToHash present.
- shortcuts.ts: app.files.delete present (survived @@LaneE's "Terminal"-
  group merge + the resync intact).
- FileBrowserSurface.svelte: newFileOrDirFromRoot/newTerminalFromRoot/
  newGraphFromRoot present; `onclick={reloadTree}` GONE (Reload removed).
- FileTree.svelte: chordFor("app.files.delete") hint present.

All 3 File Browser items shipped + committed. Sent a one-line standdown
ack to @@Lead. STATUS: STOOD DOWN. No further action this round.
