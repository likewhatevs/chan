# fullstack-70: preserve back-side state across splitPane

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Why

`webtest-a-12`'s ad-hoc with @@Alex caught a
real defect: when splitting a pane that's
showing its back side, the new pane lands on
the front. User loses orientation — they were
looking at back content, they split, and
half the layout silently flips to front.

Walker's repro (pre-fix):

```
preSplit:  pane A {sb:1, ht:"l"}
postSplit: a {bt:[...], ht:"l", sb:1}    ← original kept
           b {t:[], f:1}                  ← new: front, no override
```

## What's already in the working tree

@@WebtestA's session wrote a candidate fix +
two unit tests directly into the working tree.
They explicitly didn't commit (webtest lanes
don't commit code) and routed the work to me;
I'm routing to you. Files modified:

* `web/src/state/tabs.svelte.ts` — proposed
  patch in `splitPane()`.
* `web/src/state/tabs.test.ts` — two new tests
  under `describe("splitPane side preservation")`.

The patch (the walker's proposed shape — adapt
if you prefer a different idiom):

```ts
const newPane: LeafNode = {
  kind: "leaf",
  id: id("pane"),
  tabs: [],
  activeTabId: null,
  ...(original.showingBack
    ? {
        showingBack: true,
        back: { tabs: [], activeTabId: null },
      }
    : {}),
};
```

Behavior with the patch:
* Source on front → new pane on front
  (unchanged from today).
* Source on back → new pane on back too,
  with an empty `back` slot. Per-pane theme
  overrides stay per-pane (no inheritance).

## Acceptance criteria

* `splitPane()` propagates `showingBack` from
  the source pane to the new pane.
* If source is on back, new pane is initialized
  with `showingBack: true` AND an empty `back`
  slot (`tabs: []`, `activeTabId: null`).
* Theme overrides (`pane.theme` / `back.theme`)
  do NOT inherit. Per-pane independence stays.
* `webtest-a-12`'s two new tests pass:
  * "splitting from the front side leaves the
    new pane on the front" (guard against
    drift).
  * "splitting from the back side puts the new
    pane on its back too" (pins the new
    behavior).
* No regression on existing front-side split
  behaviour.

### Pre-existing blocker to be aware of

@@WebtestA's session flagged a pre-existing
`npm run check` error in working tree:
`App.svelte:759 "Cannot find name
'dispatchPaneModeAction'"` — UNRELATED to the
walker's diff (verified by stashing only the
walker's two files and re-running check, which
passed). Likely Lane A WIP from one of the
recent landings — stashed or in-flight elsewhere.

Resolve this first before running the gate:
* If your in-flight WIP introduced the
  reference, finish or revert it.
* If it's an actual main-branch break, that's
  a separate fire; surface it.

### Gate

* `npm run check`
* `npm run test`
* `npm run build`
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`

## Notes

* Small fix, low-risk. Patch shape is
  prescribed by the walker; you can refine the
  idiom but the behavior is locked.
* v0.11.0-blocking-soft — splits across the
  Hybrid back/front axis are part of the
  marquee Hybrid flip surface. Disorienting
  silently flipping to front is a UX bug worth
  fixing before tag.
* Re-walk: light. Lane A's 8801 has the layout
  preserved from `-12`; @@Alex's ad-hoc
  scenario can be re-run quickly to confirm.
* Queue position: Lane A's first new item
  since the queue cleared. Standing topic-level
  commit clearance.

## 2026-05-19 18:23 BST — @@FullStackA implementation note

Picking up the walker's patch from the working
tree as-is. The shape matches what `splitPane`
needs:

* If `original.showingBack` → spread
  `{ showingBack: true, back: { tabs: [],
  activeTabId: null } }` into the new LeafNode.
* Otherwise → no extra fields (default
  front-side render).

Theme overrides intentionally not copied —
matches the per-pane independence rule. Empty
back materialised on demand so the visible side
renders cleanly without poking at `front` slot.

The walker's two tests pass cleanly:

* `splitting from the front side leaves the new
  pane on the front` (guard against drift).
* `splitting from the back side puts the new
  pane on its back too` (pins the new
  behaviour + verifies the empty back slot has
  no theme inherited and the original pane's
  hybrid state stays intact).

No App.svelte `dispatchPaneModeAction` blocker
in the current working tree — `npm run check`
returns 0 errors / 0 warnings without any
additional touches. Likely the pre-existing
issue the walker flagged was cleaned up by Lane
A's earlier landings.

Gate green:

* `npm run test -- tabs` (87 passed),
* `npm run test` (393 passed total — Lane B's
  recent ships swelled the suite),
* `npm run check` (0 errors / 0 warnings),
* `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'`
  (green).

Visual eyeball worth doing: split a back-side
pane, verify the new pane lands on the back
side with an empty back slot. Per-Hybrid theme
override on the original stays per-pane (no
inheritance).

Proposed commit message:

> Preserve back-side state across splitPane (fullstack-70)
>
> Splitting a pane that's showing its back side now
> propagates `showingBack: true` to the new pane and
> materialises an empty back slot so the user
> doesn't lose orientation. Theme overrides stay
> per-pane (no inheritance). Front-side splits
> unchanged. Two new tests pin both directions of
> the side-preservation rule. Patch authored by
> @@WebtestA's webtest-a-12 walker; routed through
> Lane A.
