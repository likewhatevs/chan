# Phase-16 round-1 plan (shared)

Architect-owned orchestration doc. Lanes re-orient from here + their lane
file. Full task catalog + rationale: the approved plan (task IDs F/G/C/P/
S/TW/I/B/D/DT are defined there and echoed in each lane file). Source asks:
`round-1-requirements.md`. Desktop redesign spec + screenshots:
`desktop-redesign-draft/`.

## Round-1 scope

Ship D1 (the tunnel/gateway content reframe) as an independent track; land
the lead tooling (C2 then C3, plus C1) so the @@Lead process becomes self-
hosting; and start the highest-value parallel slices of the other lanes.
B1 (Node-20 CI bump) is folded in because it is date-bound (2026-06-16).
I1 and I2 (Mac/large-workspace bound) are deferred to a later round.

## Lane map (round-1)

```
Lane | owner   | round-1 tasks                              | starts
-----+---------+--------------------------------------------+--------
A    | @@LaneA | C2 (scrollback) -> C3 (pane+resize) -> S1; | now
     |         | C1 (team load fix) alongside               |
B    | @@LaneB | G1 (dir spine on lang/tag/mention lenses)  | now
C    | @@LaneC | P1 (cs-symlink check, non-blocking);       | now
     |         | begin P2/DT1/DT2 design (coupled)          |
D    | @@LaneD | F1,F2,F3,F6,TW3 small wins; F4 design;     | now
     |         | TW1 (waits on A's C1 semantics)            |
E    | @@LaneE | D1 (Track-0, ships first) + B1; D2,D3;     | now
     |         | TW2 after cs-terminal patterns settle      |
```

## Waves

- **Track-0 (independent, ships first):** @@LaneE D1 + D2 + D3. No code
  overlap with the lanes; lands and unblocks @@Host's designer.
- **Wave-1 (lead tooling + small wins, parallel):** A builds C2 then C3
  (+resize) then S1, C1 alongside; B does G1; C does P1; D lands
  F1/F2/F3/F6/TW3; E does B1.
- **Wave-2 (after wave-1 merges):** F4 (context-menu overhaul, design
  first), P2 + DT1 + DT2 (the coupled settings move), G2, TW1 (needs C1),
  TW2.

## Cross-lane coupling (coordinate via pokes; do NOT cross file ownership)

- **DT1 <-> P2** (both @@LaneC): DT1 removes the desktop per-row settings
  gear; P2 moves those settings into the SPA. One owner, sequence P2 before
  or with DT1 so settings never vanish.
- **TW1 <-> C1** (@@LaneD needs @@LaneA): the Team Work load dialog (TW1)
  mirrors `cs terminal team load` (C1) path+spawn semantics. @@LaneD waits
  for @@LaneA's C1 contract (posted to `event-lane-a.md`) before finalizing
  TW1.
- **F6 <-> D1** (@@LaneD touches @@LaneE's area): F6 swaps the web-marketing
  theme toggle to the sun/moon icon; D1 rewrites web-marketing copy. Split
  ownership: D1 = copy/content, F6 = the icon component. Coordinate which
  files each edits before touching `web-marketing/`.
- **control_socket / wire / terminal_sessions** are @@LaneA's exclusively
  this round (C2/C3/S1 add a bidirectional channel). No other lane edits
  them.

## Gates + merges

- A slice is mergeable when its lane has run `make pre-push` green, committed
  with pathspec, and posted the commit sha to its `event-lane-<x>.md`.
- @@Lead reviews the diff, merges (sequencing to avoid collisions), and
  re-gates the merged tree green before opening the next dependent slice.
- Cut a release when a coherent slice is green (version bump + tag on
  @@Host's go). D1 may ship on its own.

## Deferred

- I1 (magic file-type detect + pending-index state) and I2 (Metal GPU hang)
  need a Mac + large workspace; slot when available.
