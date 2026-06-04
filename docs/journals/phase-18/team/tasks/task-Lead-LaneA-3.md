# task Lead -> LaneA (3): items 1-3 ACCEPTED + item 4 UNBLOCKED (revised, cleaner)

Items 1-3 accepted (own-gate-green, in-lane, fingerprint 50dc82ea..., base
d5f7dd38). The full-tree red (fileTreeSelectionMenu.test.ts) is @@LaneC WIP, not
yours - confirmed. Strong root-causes on all three. Commit is Wave 3 (your
fingerprint detects drift).

## Survey is ALREADY ANSWERED - you wrote this report before my task-2 landed
@@Alex chose **"Paths + existing link-targets (BOTH)"**. See task-Lead-LaneA-2.md
(it crossed with your completion poke). So `[[` keeps /api/link-targets AND
gains workspace-path completion.

## Item 4 design: YOUR recon wins - do it CLIENT-SIDE, NO backend, NO B contention
Your finding (api.list(dir) -> GET /api/files returns real workspace paths) is
the right design and it SUPERSEDES the graph.rs path in task-2:
- Keep `/api/link-targets` UNCHANGED (that's the "existing link-targets" half).
- ADD workspace-path completion CLIENT-SIDE off the existing api.list /
  GET /api/files route, merged into bubbles/wiki.ts (+ triggers.ts as needed).
- This needs NO chan-server route change AND NO chan-workspace link_targets /
  graph.rs change.
=> The graph.rs SEQUENCING behind @@LaneB in task-2 is MOOT. DISREGARD it. You
   are UNBLOCKED to build item 4 NOW, entirely in your owned files, with ZERO
   dependency on B. (I'm telling B it's released too.)
- Stay in lane (bubbles/wiki.ts, triggers.ts). If you discover you actually DO
  need a backend touch after all, STOP and route to me first - but you shouldn't.
- Gate: make web-check + svelte-check + npm run build; browser-smoke the `[[`
  bubble showing BOTH path candidates (as you type a path) and the existing
  filename/heading targets.

## Item 3 Source.svelte parallel: AUTHORIZED (trivial consistency fix)
You flagged Source.svelte:461 has the SAME `scroll-behavior: smooth` on its
.cm-scroller (unassigned in the plan). Apply the IDENTICAL one-line fix there for
consistency - leaving a known-identical stall in source mode during a bug-fix
round is worse. Browser-smoke source-mode scroll to confirm no regression, and
note it as a parallel/consistency fix in your completion. Add Source.svelte to
your pathspec.

## Hand-smoke items I'm tracking for @@Alex (you can't drive these)
- Item 3 definitive "no stall" on a REAL trackpad (Blink synthetic events can't
  reproduce momentum) - Chrome or chan-desktop, @@Alex's hand.
- Source-mode scroll after your parallel fix (same hardware caveat).

## On completion of item 4
Append item-4 status (+ the Source.svelte fix) to a task-LaneA-Lead-2.md with an
updated pathspec/fingerprint, poke me. Journal as you go.
