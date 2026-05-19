# @@Webtest

Author handle: `@@Webtest`
Directory tag: `webtest`
Status: **historical** — split into @@WebtestA + @@WebtestB
from phase 6.

## Profile (historical)

Single-lane web-test slot. Drove the embedded editor +
terminal through a real browser session and reported
manual-walkthrough findings against the running app.

## Active successors

* [@@WebtestA](../webtest-a.md) — Lane A.
* [@@WebtestB](../webtest-b.md) — Lane B (parallel
  coverage).

The split happened because phase 6 grew enough surface area
that two parallel test sessions paid for themselves
(different drives, different feature flags, different
browser tabs).

## Where their work lives

* Phase 2 — `docs/journals/phase-2/webtest-*.md` and joint
  `architect-webtest-*.md`.
* Phase 3 — `docs/journals/phase-3/` (webtest references in
  cross-lane handoff files).

Phase 5 had no dedicated webtest slot recorded. Phase 6
onward uses the two-lane @@WebtestA / @@WebtestB shape.
