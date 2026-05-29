# fullstack-a-11: Closing the last tab on Hybrid back must not auto-flip to front

Owner: @@FullStackA
Date: 2026-05-19

## Goal

Sibling fix to `fullstack-a-5`. That task fixed
"closing the last tab in a Hybrid pane removes the pane" by
dropping the `collapseEmptyPane` call. This task fixes the
related "closing the last tab on the back side of a Hybrid flips
the Hybrid back to the front" — closing should never change the
Hybrid's front/back orientation, only the user's explicit
flip chord should.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md): "Closing
the last tab on the back of a Hybrid auto-flips back to front".

Phase-7 references:
* `fullstack-15` / `fullstack-16` — Hybrid pane substrate +
  Cmd+K transactional mode.
* `fullstack-70` — back-side state preserved across splitPane;
  back/front are independent state.
* `fullstack-a-5` — empty Hybrid pane preservation (sibling
  fix; reference for which closing-state-machine to amend).

## Rule

Closing the last tab on a Hybrid pane side (front OR back)
keeps that side visible as the empty landing. The Hybrid's
flip state (which side is currently rendered) does NOT change
as a side effect of any close action. Flipping is only ever
the explicit chord (Cmd+. Tab or whatever the binding lands as
in `fullstack-a-7`).

## Acceptance criteria

* Hybrid with terminal on front + tabs on back → close last
  back tab → back stays visible, renders empty-pane landing,
  no flip.
* Mirror case: tabs on front + terminal on back → close last
  front tab → front stays visible, renders empty-pane
  landing, no flip.
* Explicit flip chord still works as expected (no regression
  on the user-driven flip path).

## How to start

* Find the close-tab transition that touches Hybrid flip
  state. Likely in `web/src/state/tabs.svelte.ts::closeTabAsync`
  (where `fullstack-a-5` already removed the
  `collapseEmptyPane` call) — there may be a sibling line
  that flips the Hybrid orientation when the current side
  becomes empty.
* Drop the auto-flip; leave only the explicit chord as the
  flip trigger.
* Pin with a regression test in `tabs.test.ts` mirroring the
  shape of `fullstack-a-5`'s new last-tab-stay test.

## 2026-05-19 — implementation note

Investigated the close-tab path and found no code that flips
`showingBack` as a side effect. `closeTabAsync` mutates
`p.tabs` only (which IS the back side's tabs when
`showingBack === true`, because `flipHybrid()` swaps the slots
on every flip). `showingBack` is only ever written by
`flipHybrid()` itself or by the URL-hash restore path.

What @@Alex perceived as "auto-flip" before `fullstack-a-5`
was actually `closeTabAsync`'s `collapseEmptyPane(p.id)` call:
the empty back-side pane collapsed into its sibling, and the
sibling's content (the front pane's tabs) took over the
visible space. That looked like a flip but was structurally a
collapse. With `fullstack-a-5` dropping `collapseEmptyPane`,
the Hybrid pane survives empty and `showingBack` stays true —
no flip happens.

So this task is now a regression-pin: a new test in
`state/tabs.test.ts` (`tab close confirmation > closing the
last tab on the back side keeps showingBack=true`) seeds a
Hybrid, flips to back, opens a tab, closes it, and asserts:

* `live.showingBack === true` after the close.
* `live.tabs.length === 0` (empty back-side).
* `live.activeTabId === null`.
* `live.back?.tabs.map(t => t.id) === ["front"]` (front tabs
  parked on the back slot are untouched).

If `fullstack-a-5`'s `collapseEmptyPane` drop ever regresses
or someone adds new flip side-effects to a close path, this
test catches it.

Files touched:

* `web/src/state/tabs.test.ts` — new last-back-tab-stay test.

Pre-push gate (SPA portion): vitest 102/102 in `tabs.test.ts`
(`fullstack-a-11`'s new test plus the prior 101) green; full
suite gate runs at the end.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

The investigation is the load-bearing piece here: Alex's
"auto-flip" was structurally a `collapseEmptyPane` artifact,
not a real flip-state mutation. `fullstack-a-5` already
killed that source. Turning -11 into a regression pin is the
right call — the test seeds the exact shape (Hybrid + flip
to back + close last back tab) and asserts the four
invariants that catch any future close-path side effect on
`showingBack`, `tabs`, `activeTabId`, or the parked
`back.tabs`.

A single test landing under `tab close confirmation` is the
right granularity; no production-code change needed. Gate
green (475/475 including the new test).

**Commit clearance**: approved. Suggested subject:

```
Pin: closing the last back-side tab on Hybrid keeps showingBack=true (fullstack-a-11)
```

Push waits for Round-1 close.
