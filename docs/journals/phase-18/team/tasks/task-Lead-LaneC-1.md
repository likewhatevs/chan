# task Lead -> LaneC (1): File Browser

You are @@LaneC - File Browser lane. Round-1, Wave 1. START NOW.

## Read first (context lives here, not in this poke)
- Process: docs/journals/phase-18/team/bootstrap.md
- Plan + your lane section + gate/quality bar + shared-file table:
  docs/journals/phase-18/team/round-1-plan.md  (section "@@LaneC - File Browser")
- Verbatim spec: docs/journals/phase-18/round-1/draft.md  ("### File Browser")
- Images: round-1/image-6.png (shortcut hints), image-9.png (loading-hang console).
- Re-verify line anchors against HEAD; they drift.

## Wave 1 scope (3 items)
1. Context-menu regression: the tab right-click menu got merged with the docked
   file-browser menu. Remove "Reload". Below "Expand all directories" add (all
   acting from the WORKSPACE ROOT): "New file or Directory", "New Terminal",
   "New Graph". Reuse existing handlers newFileOrDir/terminalFromHere/graphThis.
2. Show keyboard-shortcut hints in the context menu (image-6): New Terminal
   cmd+t, New Graph cmd+shift+m, Delete = backspace, Settings cmd+, . Read from
   the central store (shortcuts.ts chordFor). Record any MISSING chord in
   shortcuts.ts so it ports to linux/macos/web.
3. Loading hang: expanding a dir stalls on "Loading"; console shows
   "SecurityError: history.replaceState more than 100 times / 10s" (image-9).
   Root cause: expand -> persistLayoutToHash -> persistStateToHash calls
   history.replaceState with NO debounce. Debounce / coalesce the hash write.

## Owned files (edit ONLY these)
web/src/components/{FileBrowserSurface.svelte,FileTree.svelte,HamburgerMenu.svelte},
web/src/components/menuClamp.ts, web/src/state/store.svelte.ts (persist region
~1569-1598 ONLY), web/src/App.svelte (layout-persist effects ~160-217 ONLY),
web/src/state/shortcuts.ts (FB chord additions).

## Shared-file rules (plan "Shared-file contention")
- shortcuts.ts: you AND @@LaneE both append to SHORTCUTS. @@Lead sequences
  C THEN E. Append your FB chords; do NOT run the resync script yourself -
  @@Lead runs `node web/scripts/shortcuts-table.mjs` ONCE after BOTH land.
- store.svelte.ts: you = persist region; @@LaneB = graph region (far apart,
  interleave-safe). @@Lead commits the merged file.
- fromHere.ts: @@LaneD OWNS seed-format changes; you CONSUME terminalFromHere
  as-is. If you need a signature change, route through @@Lead.
- App.svelte: you = layout effects; @@LaneE = rich-prompt handler (far apart).

## Gate before any "done" report
make web-check + svelte-check + npm run build. Browser-smoke the menu +
loading-hang fix (history.replaceState debounce is a runtime behavior).

## On completion
Cut task-LaneC-Lead-1.md (own-gate-green + pathspec sha + per-item status +
note exactly which SHORTCUTS entries you appended so I sequence E + resync),
poke me. Journal: journal-LaneC.md. Flag ANY shared-file touch BEFORE landing.
