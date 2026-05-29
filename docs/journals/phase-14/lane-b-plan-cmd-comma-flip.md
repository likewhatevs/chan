# Lane B plan - Cmd+, queued-shortcut "panes flip" desync

Phase 14, round 2 correctness item (live bug). Owner: **Lane B**
(frontend). Cross-referenced from `lane-b-request.md`. This is an audit
+ cleanup of the shortcut-dispatch / pane-focus path, not a redesign.

## Symptom

The "panes flip" bug. Cmd+, normally flips the focused element (per the
phase-13 Dashboard remap: Cmd+, toggles the focused Editor / Terminal /
Graph / File Browser / Dashboard). When Cmd+, is pressed while an
overlay or modal dialog holds focus - e.g. the Search overlay, or the
File Browser's "New file or directory" dialog - the command gets
QUEUED. Once queued, pane state loses sync: it acts up across panes, and
changing focus flips the PREVIOUS pane instead of the current one.

## Suspected code path (to pin during the audit)

- `web/src/state/shortcuts.ts` - the chord registry; Cmd+, maps to a
  `chan:command` id. (Confirmed: this is the single source of truth for
  chords; dispatch is App.svelte's `onWindowKey` on web, and
  chan-desktop's `KEY_BRIDGE_JS` replays the same `chan:command`.)
- `web/src/App.svelte` `onWindowKey` - the dispatch, and the mechanism
  that defers / queues a command when an overlay or dialog currently
  owns focus (find the queue/defer; what it captures, when it replays).
- `web/src/components/Pane.svelte` + the pane-focus state (active pane /
  last-focused pane) - what the replayed command targets.
- The focus-restore that runs when the Search overlay / "New file or
  directory" dialog closes - the likely point where the queued command
  replays against the pane focused WHEN IT WAS QUEUED rather than the
  pane focused WHEN IT RUNS.

## Root-cause hypothesis

A command queued while an overlay/dialog is open captures the
then-focused pane and, on replay (overlay close / focus restore),
targets that stale pane instead of the pane that is actually focused
when the command runs. The flip lands on the previous pane and pane
focus desyncs from there.

## Fix direction (audit + cleanup)

- A command queued during an overlay/dialog must resolve its target
  pane AT REPLAY TIME against the currently-focused pane, never the
  pane captured at queue time; or it should be dropped if no longer
  meaningful. Make the queue carry intent (the command id), not a bound
  pane reference.
- Ensure overlay/dialog close restores focus deterministically to a
  single, well-defined pane before any queued command replays, so the
  replay has one unambiguous target.
- Audit the whole shortcut-while-overlay-open path for correctness:
  exactly one focused pane at all times; no command both runs live AND
  replays; Cmd+, is idempotent w.r.t. the focused element.

## Regression test (vitest)

Reproduce and lock it: open an overlay/dialog (Search, "New file or
directory"), fire Cmd+, while it is open, close it, and assert the
flip/focus targets the CURRENTLY focused pane (not the previous one),
and that pane focus stays in sync after several queued/!queued cycles.
Build on `web/src/components/paneFocusClickRestore.test.ts` and the
shortcut/pane tests if present.

## Verification

- The vitest regression reproduces the desync BEFORE the fix and passes
  after; `cd web && npm test` green.
- Manual (run the app): on the Search overlay, press Cmd+,; on the File
  Browser "New file or directory" dialog, press Cmd+,; in both cases,
  after the overlay/dialog closes, focus + flip act on the correct pane
  and pane focus stays in sync across repeated cycles.
