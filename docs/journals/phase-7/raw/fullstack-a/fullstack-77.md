# fullstack-77: move kill-pane binding to Cmd+K Backspace

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

@@Alex flagged: change the kill-pane keybinding
to `Cmd+K + Backspace`. Backspace = delete is
the intuitive shape for "kill this pane".

## Current state — audit

@@Alex's framing was "cmd+k k → cmd+k
backspace". My read of the current keymap
(per `fullstack-39` + `webtest-a-8` item 2)
is that kill-pane lives on `Q`, not `k`. The
implementer should audit `paneModeKeymap` /
`App.svelte:handlePaneModeKey` and:

* If `Q` is the current binding → remove `Q`
  / `q` cases for kill-pane, add `Backspace`.
* If `k` is the current binding (architect
  mis-remembered) → remove `k` / `K` cases,
  add `Backspace`.
* If both are bound → remove both, add
  `Backspace` only.

Confirm via the source-of-truth in the
implementation note.

## Spec

* New binding: `Cmd+K + Backspace` →
  kill focused pane.
* `Backspace` is a single `KeyboardEvent.key`
  value (`"Backspace"`), case-insensitive
  by nature.
* Behaviour unchanged: closes the focused
  pane + exits Hybrid NAV (same semantics as
  the current kill binding).
* The freed letter key (`Q`, `k`, or
  whatever the current binding is) becomes
  unbound in Hybrid NAV. Drop the case
  entry; don't repurpose without a separate
  ask from @@Alex.

## Relevant code

* `web/src/state/paneModeKeymap.ts` (or the
  keymap module wherever it lives) — entry
  for kill-pane. Update the binding.
* `web/src/App.svelte:handlePaneModeKey()` —
  dispatch switch. Remove the old case, add
  `case "Backspace"`.
* `web/src/components/PaneModeHelp.svelte` —
  cheatsheet:
  * Remove the old row (Q or k, whichever
    was there).
  * Add a new row under whichever section
    fits (probably the `Close` group):
    `[⌫]  Close pane` or similar.
  * Coordinate with `-63`'s clickable
    buttons: the new Backspace cap is
    clickable and dispatches the same
    action.
* `paneModeKeymap.test.ts` — update assertions
  that referenced the old binding.

## Acceptance criteria

* `Cmd+K + Backspace` closes the focused
  pane and exits Hybrid NAV.
* The previous binding (`Q` per my audit,
  or `k` per @@Alex's framing — implementer
  confirms) no longer fires kill-pane.
* PaneModeHelp shows `Backspace` (or `⌫`
  glyph) in the cheatsheet under the Close
  group.
* Clickable help button for `Backspace`
  works (parity with the keystroke per
  `-63`).
* No regression on other Hybrid NAV
  bindings: arrows / WASD / 1-4 / Tab / p /
  h / `<` / `>` / f (post-`-74`).

### Tests

* Vitest: `Cmd+K Backspace` dispatches the
  kill-pane action.
* Vitest: the old binding (whichever it
  was) no longer dispatches anything.
* Update any test that asserted the old
  cheatsheet row.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Coordinate with `-72` (spawn keys →
  draft/commit). Kill-pane is currently
  immediate-commit; that stays unchanged.
  Backspace doesn't enter the draft model.
* Coordinate with `-74` (Search → f).
  Both lands cleanly in parallel; no
  collision.
* If the audit reveals the current binding
  was actually `k`, flag it in the impl
  note so the architect journal +
  memory get corrected.
* v0.11.0-blocking-soft. UX polish; keymap
  intuition.
* Queue position: end of Lane A queue.
  Updated queue: `-70` (shipping) → `-72`
  → `-73` → `-74` → `-75` → `-76` → `-77`.
* Standing topic-level commit clearance.

## 2026-05-19 18:50 BST — @@FullStackA implementation note

Audit result: **@@Alex's framing was right** —
kill-pane was on `k` / `K`, not `Q`. The `Q` /
`q` keys are NOT bound in the current pane-mode
dispatch (App.svelte:548-559 had only `x/X` for
close-all and `k/K` for kill). Architect journal
+ memory should be updated to reflect this if
relevant; flagging here for the audit trail.

Implementation:

* `App.svelte:handlePaneModeKey`:
  * Removed the `case "k": case "K":` block.
  * Added `case "Backspace":` with the same
    commit + scheduleSessionSave +
    `closePane(layout.activePaneId)` body.
  * Comment updated to document the move
    + the rationale.
* `PaneModeHelp.svelte`: Close group row
  flips from `{ label: "k", key: "k" }` to
  `{ label: "⌫", key: "Backspace" }`. Glyph
  reads as "delete" so the cap matches the
  action.
* `paneModeKeymap.test.ts`: new describe block
  `Cmd+K Backspace kill-pane (fullstack-77)`
  asserts the new dispatch shape AND that the
  old `k / K → closePane` block is gone.

Acceptance:

* Cmd+K Backspace closes the focused pane and
  exits Pane Mode (commit + scheduleSessionSave
  + closePane, same body as the old `k`
  handler).
* `k` / `K` no longer fire anything in Pane
  Mode (unbound, not repurposed).
* PaneModeHelp shows ⌫ for kill-pane under
  Close.
* No regression: arrows / WASD / 1-3 staging /
  4 / Tab / p / h / `<` / `>` / `f` / `x`
  unchanged.

Gate green:

* `npm run test -- paneModeKeymap` (14 passed),
* `npm run test` (404 passed),
* `npm run check` (0 errors / 0 warnings),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Proposed commit message:

> Move kill-pane binding from `k` to Cmd+K Backspace (fullstack-77)
>
> Backspace = delete reads as the intuitive shape
> for "delete this pane". Old `k` / `K` letter
> binding dropped — unbound, not repurposed —
> waiting on a future ask if it gets a new home.
> PaneModeHelp shows ⌫ under Close. Confirmed via
> audit: kill-pane was on `k`, not `Q` as the
> architect journal had it. Behaviour unchanged
> (commit + scheduleSessionSave + closePane).
