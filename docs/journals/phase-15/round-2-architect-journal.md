# Round-2 architect journal (@@LaneA = @@Architect)

## Kickoff (2026-05-31)

### Role shift (from @@Host)
@@LaneA -> @@Architect; worker handles shifted up: @@LaneB = round-1 Lane-A
domain (Dashboard/flip), @@LaneC = round-1 Lane-B (search), @@LaneD = round-1
Lane-C (terminal/cs). Verified the 4 live tabs in group `chan-dev` via
`cs term list`. The shift keeps each worker on the domain it already knows;
notably @@LaneD continues its own round-1 terminal code (incomplete shift+Enter
fix, `cs term`, tab groups).

### @@Host product/scope calls
- **`chan open` (desktop OS-association path): action in round-2.** Folded into
  @@LaneD's desktop wave (DESKTOP-OPEN), adjacent to `chan shell`.
- **Survey bubbles (2.3): defer to round-3.** v0.21.0 ships poke protocol 2.2
  (agent<->agent) only. The bubble event-pump/reply rebuild is the round-3
  headline. @@LaneD leaves `BubbleOverlay`/`TeamWorkState` hooks untouched.

### Architect-side calls (made, not escalated)
- **Theme split** maps domain->tab (see coordination.md lane map). @@LaneB gets
  all of part-1 (its own drops) + the two frontend bugs; @@LaneC the unified
  indexing bug + cs search; @@LaneD the terminal/cs/desktop/Team-Work load.
- **`graphFromHere` fix scope:** directory case only (matches the bug report);
  file case + breadcrumb `rescopeFromHere` unchanged. Told @@LaneB not to widen.
- **`cs search` ownership -> @@LaneC** (search domain owns result semantics +
  markdown), appended onto @@LaneD's main.rs/control_socket.rs as a disjoint arm
  at CK-RENAME, rather than splitting cs-search plumbing across lanes ad hoc.
- **Load balance:** @@LaneD is heaviest; sequenced into 3 waves, told to spawn
  subagents, and Team Work (wave-3) is explicitly round-3-backlog-able if
  untested at close (do not rush into the release).
- **Output-flag convention** standardized for cs search + cs terminal list:
  markdown default, `--json` compact, `--json --pretty` indented. No
  `--pretty-json`.

### Sequencing
Wave plan + 5 checkpoints recorded in coordination.md. The load-bearing one:
**CK-SUBMIT first** - @@LaneD's shift+Enter fix gates poke auto-delivery for the
whole team, so it leads wave-1.

### Doc layout
Round-2 task files: `round-2-lane-{b,c,d}.md`. Round-1 `lane-*-tasks.md` kept as
history (no clobber). coordination.md lane map + region split + checkpoints
filled. No `-a` task file (architect, not a worker).

## Open / pending
- Dispatched wave-1 to all three lanes (poke). Awaiting confirmations.
- I own, post-CK-RENAME: updating the poke/bootstrap doc references from
  `cs term` to `cs terminal`.
- DONE: shared baseline drive `/tmp/chan-test-r2` (shallow clone, .git stripped,
  915 md) + server `/tmp/r2srv` @ http://127.0.0.1:8820/ (no-token, log
  /tmp/r2-server.log, pid 49740). Usage + per-lane smoke discipline recorded in
  coordination.md "Test server (round-2)". @@Host approved the shallow-clone seed.
- Release gate at close additionally builds the gateway workspace.
