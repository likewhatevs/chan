# webtest-a-6 — -a-52 walkthrough (G9 depth slider forward-only + G10 drop link filter)

Owner: @@WebtestA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Walk `fullstack-a-52` (`4cf496c`) — the G9 + G10 minimum
cut. Scope is tight: depth slider semantics + link-filter
chip removal. `-a-49` + `-a-50` + `-a-51` already
validated 4/4 HOLD in your proactive walk (`a63c8cb`).

## Background

`-a-52` commit `4cf496c`: `Graph depth slider forward-only
+ drop link filter (fullstack-a-52 — G9 + G10 minimum cut)`.

Two pieces (per `fullstack-a-52.md` + the @@FullStackA
commit-readiness poke):

### G9 — Depth slider forward-only

Today's behavior (pre-`-a-52`): depth slider supposedly
reveals "forward" nodes as depth increases (per the
bug-list entry @@Alex filed). Reality: slider may
include reverse-branch traversal too.

`-a-52` fix: BFS direction limited to forward-only.
Reverse branch removed; comment documents the
direction.

### G10 — Drop link filter

Today's behavior: filter chips include a "link" chip
that's effectively redundant (the `contains` edges
that drive the filesystem hierarchy are NOT the same
as user-visible "link" edges; the chip's semantic was
ambiguous).

`-a-52` fix: `FilterKind` union drops `"link"`.
`edgeVisibleByChip("link")` short-circuits to `true`.
Two chip iteration sites updated. `FILTER_COLORS`
literal: `link` key dropped. Filesystem-mode label
dispatch: dead `kind === "link" ? "contains"` branches
removed at both label ladders.

## Coverage slice (lane A)

Tight scope. Single chan + test-server boot; walk the
graph + check the depth slider + check the filter chip
set; capture a single verdict.

## Acceptance criteria

### G9 — Depth slider semantics

1. **Slider at depth = 1**: open graph (Cmd+Shift+M);
   confirm the visible node set is the current root +
   one hop of forward edges. Take screenshot.
2. **Slider at depth = 3**: drag slider higher; confirm
   the visible node set EXPANDS to include 2 + 3 hops
   forward from root. Take screenshot.
3. **Slider at depth = 1 again**: drag back; confirm
   the visible node set shrinks back to root + 1 hop.
   (Empirical no-reverse-traversal: should look the
   same as check #1.)
4. **Forward-only direction documented**: read the
   relevant code section (`GraphPanel.svelte` around
   the depth-filter logic); confirm the comment
   documents forward-direction semantic.

### G10 — Link filter chip removed

5. **No "link" chip in the filter row**: open graph;
   look at the filter-chip row. Confirm NO chip labeled
   "link" (or equivalent visible representation).
6. **Remaining chips function**: verify the OTHER
   filter chips (tag / mention / language / img /
   folder / etc.) still toggle correctly. Drag-toggle
   them; observe the visible node set updates.
7. **Filesystem-mode labels unaffected**: switch graph
   to filesystem mode; confirm edge labels render
   correctly (the `kind === "link" ? "contains"` dead
   branch removal shouldn't break anything since
   "link" never reached that ladder in filesystem
   mode).

### Side observation re-verification (optional)

The proactive `a63c8cb` walk surfaced a "click
hit-radius too tight" observation on the graph canvas.
NOT in `-a-52`'s scope; if you have time at the end,
re-walk that empirically (click near nodes WITHOUT
zoom; note hit/miss rate). Otherwise the bug-list
entry stands as filed.

### Walkthrough audit trail

Append a fresh dated heading to
[`webtest-a-1.md`](webtest-a-1.md):
`## 2026-05-22 — fullstack-a-52 walkthrough (G9 depth
slider forward-only + G10 drop link filter)`. Capture:

* The 7 acceptance check verdicts (HOLD / FAIL /
  PARTIAL).
* Screenshots at key states (slider at 1 / 3 / 1
  again; filter row with no link chip).
* Any side observations.
* Tear-down evidence.

## How to start

1. `git status` confirm clean; `git log --oneline -5`
   confirms `4cf496c` is in HEAD.
2. Rebuild chan: `cd web && npm run build && cd ..`
   then `cargo build -p chan` (web/dist may be stale
   relative to `-a-52`).
3. Spin up test server; chan-source seed drive.
4. Open graph; walk G9 checks (1-4).
5. Walk G10 checks (5-7).
6. Append verdict; fire poke to
   `event-webtest-a-architect.md`.
7. Tear down per the standing rule.

## Coordination

* @@WebtestA lane (reactive).
* Standing terminal + Chrome MCP perm covers the walk.
* Light walk; ~30 min of empirical work.

## Numbering

Highest committed `webtest-a-N` is `-5` (verdict +
close-out + the proactive `a63c8cb` walk). This is
`-6`.

## Out of scope

* `-a-55` follow-up (already 3/3 HOLD in your
  proactive `-a-55` walk per `1eabe95`).
* `-a-49`/`-a-50`/`-a-51` (4/4 HOLD in your proactive
  `a63c8cb` walk).
* Graph hit-radius polish (side observation; filed in
  bug list; not in `-a-52`'s scope).
* Future graph-overhaul slices (G5 markdown overlay,
  etc.) — those land in future walks.
