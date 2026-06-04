# task LaneC -> Lead (1): File Browser - Wave 1 complete

@@LaneC reporting Wave-1 (3 items) DONE + own-gate GREEN. NOT committed
(shared-file coupling; see below). Runtime smoke deferred to Wave 2.

## Own-gate (frontend) - GREEN
- `npm run check` (svelte-check): 0 errors. 1 warning, pre-existing,
  @@LaneE's RichPrompt.svelte a11y (not mine).
- `npm test` (vitest): 169 files / 1679 tests PASS.
- `npm run build`: OK (only pre-existing INEFFECTIVE_DYNAMIC_IMPORT +
  chunk-size warnings).

## Per-item status
1. Context-menu regression (DONE) - FileBrowserSurface.svelte (exclusive):
   removed "Reload" + dead `reloadTree`/`refreshTree`. Below "Expand all
   directories" added 3 WORKSPACE-ROOT entries: New file or Directory,
   New Terminal, New Graph (reuse fileOps.createFileOrDir("") /
   openTerminalInPane(layout.activePaneId, terminalFromHereTarget("",true))
   / openFsGraphForDirectory("")). Tab-menu entries carry NO chord (chords
   are item-2's selection-menu scope per spec).
2. Shortcut hints (DONE) - FileTree.svelte selection menu (image-6) now
   shows chords via chordFor(): New Terminal, New Graph, Delete, Settings.
   See SHORTCUTS note below for the new entry.
3. Loading hang (DONE, code) - store.svelte.ts persist region:
   - DEDUP: persistStateToHash skips replaceState when computed URL ==
     current href (kills the FB-expand churn storm: the effect re-runs on
     every fbTreeInstances registry churn recomputing the SAME hash).
   - DEBOUNCE: new `schedulePersistStateToHash()` (150ms trailing), used by
     the FB expansion effect (FileBrowserSurface) + App's 2 layout effects.
   - persistStateToHash stays SYNCHRONOUS (cancels pending timer) so the
     pagehide flush (App.svelte:1078, untouched, outside my region) + the
     store.test seam keep their immediate write.
   - schedulePersistStateToHash is a pure timer setter (no $state) -> no
     reactive-graph change, no Svelte-5 reactivity hazard.

## SHORTCUTS append (for your E-sequencing + resync)
- Appended EXACTLY ONE entry to web/src/state/shortcuts.ts SHORTCUTS:
  `{ id: "app.files.delete", label: "Delete file or directory",
     web: "Backspace", native: "Backspace", group: "File" }`
  (escapeTerminal intentionally UNSET/false; bare Backspace must reach the
  shell, and chordFromEvent ignores modifierless keys so it can't escape.)
  Backspace/Delete was already wired in FileTree.onTreeKeydown; this just
  records it centrally. Introduces the (previously unused) "File" group ->
  a new section in the rendered table / main.rs help after resync.
- DO NOT resync yet (per task): sequence @@LaneE's shortcuts.ts append,
  THEN run `node web/scripts/shortcuts-table.mjs` ONCE to resync main.rs.
  I did NOT run it.

## Shared-file flags (BEFORE landing) + commit guidance
- store.svelte.ts: my persist hunk (~1562-1647) coexists with @@LaneB's
  graphReloadSignal hunk (~1952). Far apart, interleave-safe per the plan.
  You commit the merged file.
- App.svelte: my 2 layout-effect call-site swaps + 1 import (region
  ~40-216). @@LaneE's rich-prompt region (~659) is untouched/not present
  yet. Far apart.
- shortcuts.ts: only my app.files.delete append present (E not yet).
- I did NOT commit anything: FileBrowserSurface/App import
  schedulePersistStateToHash from store, so a partial commit wouldn't
  compile, and committing the shared files would pre-empt your merge role.

## Verifiable shas (working-tree diff content, point-in-time)
- Full changeset (9 files, INCLUDES @@LaneB's store graph hunk):
  `git diff -- <9 files> | git hash-object --stdin` = 6aac8719b229089bf6a9f06956c3c0107990620a
- Clean (8 files, EXCLUDES store.svelte.ts) - my exclusive changeset:
  0ccda12b740d4ff90b2134549d719596b3374e2e
- Files: web/src/components/{FileBrowserSurface,FileTree}.svelte,
  web/src/state/{store.svelte.ts,shortcuts.ts}, web/src/App.svelte, and
  tests fileBrowserRightClickRevamp / fileTreeSelectionMenu /
  fileBrowserUnifiedDialog / perTabInspectorWidth.

## Tests updated (source-pins guarding my changed components)
- perTabInspectorWidth.test.ts: persistLayoutToHash -> schedulePersistStateToHash.
- fileBrowserRightClickRevamp.test.ts: Reload pins replaced with
  root-spawn order + "Reload removed" + import-after-root-spawn.
- fileTreeSelectionMenu.test.ts: New Terminal/New Graph/Delete spans now
  menu-row-label; ADDED a chord-hint (chordFor wiring) test.
- fileBrowserUnifiedDialog.test.ts: Settings span -> menu-row-label.

## Pending / asks
- RUNTIME smoke (menu render, chord alignment in the .ctx menu, no
  replaceState SecurityError on expand) DEFERRED to the Wave-2 convergence
  server per the round plan. Ready to drive the FB-area smoke when it's up
  (and once @@Alex answers the open question on which client to smoke).
- No decisions needed from @@Alex on my items.
