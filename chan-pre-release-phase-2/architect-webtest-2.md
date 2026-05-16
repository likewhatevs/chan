# architect-webtest-2: @@Webtest B idle handoff

Owner: @@Webtest B. Status: DONE (item 2 of fourth-sweep commit
plan closed; B is idle again). To: @@Architect.

## Update — 2026-05-16: frontend-10 runtime-verify complete

@@Frontend landed [[chan-pre-release-phase-2/frontend-10.md]]
(REVIEW) while B was idle. B picked the trigger up immediately:

* Rebuilt: `npm run check` (3911/0/0), `npm run build`,
  `cargo build --release -p chan`.
* Rotated stale `report.jsonl` to
  `report.jsonl.webtestB-20260516-170305.bak` and restarted 8788.
* Ran `CHAN_WEBTEST_GLYPH_PROBE=1 node
  chan-pre-release-phase-2/webtest-smoke.mjs` → all six probes
  green (drift 0.7578 post-swap vs 0.7080 pre-swap from A).
* Tore down 8788; scratch fixture cleaned by the runner.
* Recorded results in [[chan-pre-release-phase-2/webtest-2.md]]
  "@@Webtest B post frontend-10 full matrix re-run" and journal log.

Caveat (also recorded in webtest-2.md): the histogram-based glyph
probe's hard assertion is trivially satisfied by scene composition,
not glyph identity. It's an end-to-end render check, not a precise
regression test. If glyph-identity sensitivity matters going forward,
a follow-up should add a frontend `window.__chanGraphRenderedKinds`
test hook and replace the histogram probe.

Item 2 of the architect's fourth-sweep commit plan is closed. The
only remaining gates before the commit pass are:

1. @@Rustacean implements rustacean-4 option A (routed via
   [[chan-pre-release-phase-2/architect-rustacean-3.md]]).
2. Architect runs `scripts/pre-push` on the assembled tree.

B remains idle for any further dispatch.

## Context

@@Webtest B was spun up in parallel with @@Webtest A. A took the
architect-9 re-run end-to-end (all five smoke probes green, 8788
torn down clean); details under
[[chan-pre-release-phase-2/webtest-2.md]] "@@Webtest A architect-9
re-run". B did the frontend-10 wire-shape probe prep so the visual
smoke is ready the moment @@Frontend flips the folder routing in
`mapFsNodes`.

This file is the B-side idle handoff so @@Architect can dispatch
B again without re-scanning the journal.

## What B finished

* Coordination log entry in
  [[chan-pre-release-phase-2/journal.md]] under `## Log` dated
  2026-05-16 (@@Webtest B).
* New probe scaffolding in
  [[chan-pre-release-phase-2/webtest-smoke.mjs]]:
  `captureCanvasSignature`, `signatureDistance`, and
  `smokeFolderGlyphWireShape`. Wired into `main()` after the
  existing probes. Gated post-swap assertion behind
  `CHAN_WEBTEST_GLYPH_PROBE=1` so the default matrix stays green
  pre-swap.
* Section in [[chan-pre-release-phase-2/webtest-2.md]]
  "@@Webtest B frontend-10 wire-shape probe prep" describing the
  probe shape, the rationale (no DOM hook in GraphCanvas, so pixel
  histogram is the most semantic-preserving option), and the gating
  flag.

## What is blocking B

Two non-blocking signals could pull B back in:

1. **Architect phase commit lands.** B then re-runs
   `node chan-pre-release-phase-2/webtest-smoke.mjs` against the
   rebuilt service to confirm no regression. The new
   `smokeFolderGlyphWireShape` probe runs in prep mode (reports
   drift, never fails) so the matrix can stay deterministic.
2. **@@Frontend lands frontend-10
   ([[chan-pre-release-phase-2/frontend-10.md]]).** B then re-runs
   the matrix with `CHAN_WEBTEST_GLYPH_PROBE=1` to flip the
   wire-shape probe into hard-assert mode (drift >= 0.05). If both
   land together, B does a single re-run with the flag set.

Neither signal is owed to B; B is idle until @@Architect or
@@Frontend says go.

## What B explicitly did not do

* Did not re-run the architect-9 probe matrix. @@Webtest A already
  did that this cycle (green); duplicating would have burned the
  scratch fixture twice.
* Did not start the shared 8788 service. @@Webtest A's tear-down
  intentionally left port 8788 free so the next probe cycle picks
  the fixture. Restart belongs to whichever lane runs next.
* Did not commit. Per the brief, commits are coordinated through
  @@Architect; the new smoke-runner edit + the webtest-2.md +
  journal.md + architect-webtest-2.md changes are unstaged.
* Did not modify any frontend/backend code. Backend lane stays
  quiet per the role brief.

## Files B touched (uncommitted)

```
chan-pre-release-phase-2/journal.md
chan-pre-release-phase-2/webtest-2.md
chan-pre-release-phase-2/webtest-smoke.mjs
chan-pre-release-phase-2/architect-webtest-2.md
```

## Done means

* @@Architect either dispatches B back (commit landed / frontend-10
  landed) or absorbs the probe-prep into the phase summary.

Status: IDLE.
