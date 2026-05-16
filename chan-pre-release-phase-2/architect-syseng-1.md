# architect-syseng-1: syseng idle and ready for more work

From: @@Syseng. To: @@Architect. Status: DONE.

## TL;DR

[[chan-pre-release-phase-2/syseng-2.md]] is REVIEW. The full
phase-2 syseng-lane workload — backend-1 (tag-gate regression
tests), backend-3 + rustacean-2 (`/api/graph` FS-truth), backend-4
(`/api/graph/languages` rank fan-out), and the /ws-side contract
for frontend-7 — has been reviewed and the hardening probe matrix
against `/tmp/chan-syseng-phase2-fixture` ran clean. All gates
green: chan-server 99 tests, chan-drive 429 tests,
`scripts/pre-push` clean (fmt + clippy + test + no-default-features),
`npm run check` 0/0/3911, vitest 9/111. Detail in syseng-2.md.

Verdicts:

- backend-1 — **Approved**. `is_markdown_file` gate + integration
  test coverage matches the contract syseng-1 proposed.
- backend-3 + rustacean-2 — **Approved**. `indexed_file_exists`
  uses `symlink_metadata`, present_files cache is single-pass,
  watcher race verified live.
- backend-4 — **Approved**. Per-language folder rank is stable,
  case-insensitive filter is symmetric, depth cap correct, report
  fan-out is O(N) over cached `report.files`.
- frontend-7 — **Approved (syseng side)**. /ws self-write
  suppression still holds; closed-overlay path latches the nonce
  without leaking subscriptions; 250ms debounce coalesces bulk
  events. Webtest browser smoke is the remaining gate, owned by
  @@Webtest in [[chan-pre-release-phase-2/webtest-2.md]].

Non-blocking residuals captured in syseng-2.md:

1. Optional empty-drive unit test for `build_language_graph` (one
   liner; live probe already confirms correct behaviour).
2. Browser smoke for the live add/delete/rename graph paths is
   carried in [[chan-pre-release-phase-2/webtest-2.md]].

## Status

DONE. The depth-cap close-out I committed to in this ping landed
as [[chan-pre-release-phase-2/syseng-3.md]] (REVIEW). frontend-9's
depth.ts is approved from a syseng standpoint, the empty-drive
lang-graph test is in (added by @@Backend), and the hardening
matrix re-runs clean against the new fixture with the depth-probe
path in the loop.

@@Architect now has two REVIEW items from @@Syseng:
[[chan-pre-release-phase-2/syseng-2.md]] (the four specialist
approvals from the first wave) and
[[chan-pre-release-phase-2/syseng-3.md]] (depth-cap close-out).
Both ready for ack.

@@Syseng idle. If new syseng-lane work appears, file as
`syseng-N.md` and I'll pick it up.
