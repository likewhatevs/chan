# @@Alex -> @@LaneB

Append-only. @@Alex writes here. Most recent entry at the bottom.

## 2026-05-28 @@Alex -> @@LaneB (cut by @@LaneA on @@Alex's direction)
New task: retire the empty-pane right-click menu + close the hamburger gap

New DISCRETE task queued: `docs/journals/phase-13/lane-b-empty-pane-menu.md`.

Summary: remove the empty pane's RIGHT-CLICK context menu, leaving the
pane hamburger (⋮) as the single menu. But the hamburger does NOT
currently carry Dashboard or Search (they live only in the right-click
menu's `emptyPaneExtraActions` + the single-pane welcome grid, and in
multi-pane the empty pane has no welcome grid). So Part 1 adds Dashboard
+ Search to the hamburger FIRST; Part 2 then removes the empty-pane
right-click path. Full spec, file/line anchors, tests, acceptance, and
gate are in the task file.

Scoping notes:
- All touched files are yours (`Pane.svelte`, `EmptyPaneWelcome.svelte`,
  `Pane.test.ts`); no Lane A overlap.
- Per `feedback_no_midtask_interrupts` + `feedback_inflight_task_amendments`:
  this is a NEW task, NOT an amendment to your started
  `lane-b-round-1-closing-2.md`. Pick it up at a coherent point AFTER
  your in-flight `Pane.svelte` closing-2 work lands - don't interleave
  half-states, and don't TaskStop to take it now.
- @@Alex: please confirm whether this is a closing-2 tail item or a
  round-2 carryover (`roadmap-round-2.md`).
