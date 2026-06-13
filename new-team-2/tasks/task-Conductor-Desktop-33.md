# task-Conductor-Desktop-33 — round-close instrumented WKWebView walk (joint with @@Editor)

From: @@Conductor. To: @@Desktop. Cut: 2026-06-13.

## Go

Your pre-staged base (b82a0a27, binary sha 58b6d195, instrumentation
stripped, fresh web/dist) is the walk build. Run the INSTRUMENTABLE
portion of the round-close WKWebView checklist now — this is
independent of the open B5 survey (a veto only touches cap counting
+ menu text; if it lands, only the B5 hand-smoke line moves).

## Checklist source

designs/round-1-report-data.md § 5 (TeamFlow's draft, incl. their
appends) PLUS task-PromptQueue-Conductor-28's updated item-2 list
(supersedes the older item-2 section; includes the tab-strip pill
counts + flipped-pane pill non-mirroring) PLUS the B5 30-second
check (b5 note, last section) IF cheap to instrument — else it
stays hand-smoke.

## Division of labor (joint task; @@Editor is getting the mirror task)

- YOU own: build, harness (your item-6 driver pattern), isolated
  $HOME, provenance check (verify the served binary == 58b6d195
  before any assertion), teardown.
- @@EDITOR owns: the item-1/4 assertion specs (what exactly to read
  back: document.activeElement chain, scrollTop, decoration counts,
  session-restore caret landing) and item-2 SPA state assertions
  (per the design's state machine) — they offered to drive; you two
  coordinate directly for this walk (peer-to-peer is fine here;
  report back through me as usual).
- Anything not automatable in the harness: mark [hand-smoke] with a
  one-line reason; it lands on @@Alex's list. Don't force it — an
  honest split beats a flaky assertion.

## Report

One completion task from YOU (Editor co-signs in their own report or
inline): per-item PASS/FAIL/HAND-SMOKE table, console/runtime-error
sweep result (the state_unsafe_mutation watch), binary provenance
line, teardown confirmation. Findings = tasks via me, as ever.
