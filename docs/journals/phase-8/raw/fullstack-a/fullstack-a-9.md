# fullstack-a-9: Hybrid NAV `[` / `]` resize keys — fix inversion / pick a convention

Owner: @@FullStackA
Date: 2026-05-19

## Goal

@@Alex reports that `[` and `]` in Hybrid NAV (for resizing the
focused divider) feel inverted "often but not always". Either:

* The binding really is mis-mapped on one axis (horizontal vs
  vertical split), OR
* The convention is ambiguous and the user's mental model
  doesn't match the implementation.

Diagnose, then pick a single consistent convention and document
it in `PaneModeHelp`.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md): "Hybrid NAV
`[` and `]` resize keys feel inverted".

Phase-7 reference: `fullstack-16` introduced the Cmd+K
transactional pane mode + WASD/arrows/resize. The `[` / `]`
bindings landed in that work.

## Convention to land

Pick one and document:

* `[` → grow left or up (i.e. move the divider toward the
  right/bottom child, shrinking it; the focused side gains
  space if it's the left/top side).
* `]` → grow right or down (mirror).

Equivalently: `[` and `]` always shift the divider in the
direction the bracket "points" (`[` opens left → divider goes
left; `]` opens right → divider goes right). Whichever child
has the focus gains/loses space accordingly.

If the existing implementation matches the above on one axis
but inverts on the other, fix the inverted axis. If the
implementation is uniform but doesn't match this convention,
swap globally (and confirm with @@Alex if the convention
proposed here doesn't match their mental model).

## Acceptance criteria

* `[` and `]` in Hybrid NAV behave consistently across
  horizontal AND vertical splits.
* Convention documented in `PaneModeHelp.svelte` with a
  one-line "[ shrinks left/up boundary; ] shrinks right/down
  boundary" entry (or whichever wording matches the
  implementation).
* No regression on the WASD/arrow resize bindings from
  `fullstack-16`.

## How to start

* `web/src/state/shortcuts.ts` for the chord declarations.
* `web/src/App.svelte` (`handlePaneModeKey`) or
  `web/src/state/paneMode.svelte.ts` for the dispatch.
* `web/src/state/pane.svelte.ts` for the actual resize
  primitives (likely two functions, one per axis).
* Verify with both horizontal and vertical splits in the same
  walkthrough.

## 2026-05-19 — implementation note

Root cause: `paneModeResize` in `web/src/state/tabs.svelte.ts`
flipped the sign of `ratio` delta based on `containsLeaf(...,
split.a, activePaneId)`. That meant `[` / `]` shifted the
divider in a direction relative to the focused pane's side of
the split — which read as "inverted" when focus was on the
right / bottom child but matched the bracket direction when
focus was on the left / top child. Hence @@Alex's "inverted
often but not always."

Fix:

Dropped the `inA` branch entirely. The function now applies
`+amount` for `positive=true` (`]` / `=`) and `-amount` for
`positive=false` (`[` / `-`), independent of where the
active pane sits in the split tree. Since `ratio` is A's
(left / top) share, `+amount` always moves the divider to
the right / bottom and `-amount` always to the left / top.
That matches the bracket-direction convention from the
task spec.

Call sites in `App.svelte` (`case "["` / `"]"` / `"-"` /
`"="`) already pass the right boolean for the new semantics
— no caller change needed:

| Key | axis    | positive | Divider moves |
|-----|---------|----------|---------------|
| `[` | row     | false    | left          |
| `]` | row     | true     | right         |
| `-` | column  | false    | up            |
| `=` | column  | true     | down          |

Renamed the boolean from `grow` to `positive` for clarity and
amended the docstring with the convention.

`PaneModeHelp.svelte`'s Resize section copy updated to
`Move divider left / right` (row) and `Move divider up / down`
(column) — the prior "Shrink / grow horizontally" wording was
the ambiguous-language source.

The two existing `paneModeResize` tests in
`state/tabs.test.ts` (lines 565 + 948) exercise the
draft/commit/cancel path and don't assert specific ratio
outcomes after the resize itself — both still pass under the
new semantics (vitest 474/474 green).

Files touched:

* `web/src/state/tabs.svelte.ts` — drop the `inA` branch,
  rename `grow` → `positive`, update docstring.
* `web/src/components/PaneModeHelp.svelte` — cheatsheet
  copy + comment.

Pre-push gate (SPA portion): vitest 474/474 green;
`npm run check` 0 errors / 0 warnings.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Root cause is clean: the focus-side-relative sign flip in
`paneModeResize`'s `inA` branch was the "inverted often but
not always" pattern Alex flagged. Dropping the branch and
moving to a fixed-direction convention (`positive=true` →
ratio grows → divider moves right/down regardless of focused
child) is the right move. The `grow → positive` rename is
load-bearing in the docstring since "grow" is ambiguous in a
two-axis split; "positive" matches the bracket arithmetic.

Help-overlay copy update ("Move divider left/right" /
"Move divider up/down") removes the prior "Shrink/grow
horizontally" ambiguity that compounded the bug-feeling.
Resize tests in `tabs.test.ts` exercise the draft/commit
machinery rather than direction outcomes, so they ride
through without amendment. Gate green.

**Commit clearance**: approved. Suggested subject:

```
Hybrid NAV: [ / ] / - / = move divider in fixed directions (fullstack-a-9)
```

Push waits for Round-1 close.
