# Phase 11 File Browser capabilities (new task)

New feature from @@Alex (2026-05-26): the File Browser should behave like
a normal desktop file browser. These are standard, expected behaviours;
the set is large but coherent. OWNER: @@LaneA (File Browser is its
domain). Queue after Slice E (the per-instance watcher wiring) so it
builds on the per-instance selection/state model; sequence relative to
F/G/inspector at @@LaneA's discretion, but it is FB-centric so doing it
adjacent to E (while in FB code) is natural.

This is the app-internal complement to bug 2a: bug 2a removed only the
OS<->app drag (drag-out to Finder, external drag-in upload). App-internal
drag and these selection/clipboard behaviours STAY and are extended here.

## Requirements

1. Multi-select within a File Browser instance:
   - Mouse: shift+click (range), cmd/ctrl+click (toggle one), and a
     click-drag rubber-band over the tree to select a contiguous run.
   - Keyboard: arrows move the cursor; shift+arrows extend the selection
     range; standard select-all (cmd/ctrl+A) within the focused instance.
   - Selection is PER INSTANCE (consistent with the Slice A per-FB-instance
     metadata); selecting in one File Browser must not affect another.
2. Clipboard:
   - cmd/ctrl+C copies the selection, cmd/ctrl+X cuts it, cmd/ctrl+V
     pastes into the target directory (the focused dir or the selected
     dir). Copy duplicates; cut moves (removes from source on successful
     paste). Cross-instance paste is allowed (same drive).
3. Drag and drop (mouse, app-internal) to MOVE one or many:
   - Dragging the current selection (one or many entries) onto a target
     directory moves them there. This extends the kept internal tree-move
     drag to operate on a multi-selection.

All operations apply to the whole multi-selection ("one or many").

## Backend (chan-drive) needs
- Multi-entry MOVE and COPY through `chan_drive::Drive` (atomic, sandboxed,
  special-file refusal honoured). Move likely reuses the rename path;
  COPY of files and directory subtrees may be new - check `drive.rs` /
  `fs_ops.rs` for existing copy support before adding. Name-collision
  policy on paste (e.g. "copy of", or refuse) is a sub-decision; pick the
  least-surprising default and note it.
- Operations route through the same watcher/broadcast path so all
  subscribed File Browser instances and the Graph update live (reuse the
  Slice C scoped pub/sub).

## Frontend (web) needs
- Per-instance selection model + clipboard state (copy vs cut, the held
  paths) in the per-FB-instance store from Slice A.
- Keyboard handling (arrows / shift+arrows / cmd+A / cmd+C/X/V) scoped to
  the focused File Browser instance, not the global shortcut registry
  unless that is where focus-scoped FB chords already live.
- Rubber-band selection + multi-drag in `FileTree.svelte` (which @@LaneB
  just touched for bug 2a drag removal - reconcile against that).

## Notes
- Large feature; likely its own multi-slice effort for @@LaneA. Break it
  down in the lane journal (e.g. selection model -> clipboard -> DnD move
  -> backend copy/move) and land in small gated slices per the merge
  protocol.
- Key files: `web/src/components/{FileTree,FileBrowserSurface}.svelte`,
  the per-instance store in `web/src/state/store.svelte.ts`,
  `crates/chan-drive/src/{drive,fs_ops}.rs`, and the move/copy routes in
  `crates/chan-server/src/routes/files.rs`.
