# Phase 12 bootstrap (opening)

Opened 2026-05-27 by @@Architect at the close of phase 11. main baseline
`5f25cc1`. Carryover + scope: `phase-12-backlog.md`. This file is the opening
contract: roster, lane definitions, boundaries, the codemod-sequencing
strategy, and the kickoff protocol. @@Alex launches the lane sessions; they
recover from their plan + this bootstrap + the bus; @@Architect serializes all
merges to main.

## Roster

| Handle      | Role                                                        |
|-------------|-------------------------------------------------------------|
| @@Architect | Orchestrator. Plan, dispatch, serialize merges, re-gate.    |
| @@LaneA     | Graph + File Browser carryover. MAY spawn 2-3 subagents.    |
| @@LaneB     | Scoping architect: drive -> workspace terminology/codemod.  |
| @@LaneC     | @@Alex ad-hoc: frontend/cosmetics, keyboard shortcuts.      |
| @@Alex      | Human owner. Drives @@LaneC; rules on scope/infra decisions. |

@@Alex will add a couple more lanes later with new requests (a release/build
lane is the obvious home for the release carryover in the backlog). Leave room:
new channels follow the same `event-<from>-<to>.md` convention.

## Lane A: graph + File Browser carryover

Scope: the graph/FB items in `phase-12-backlog.md` - the overlay/scope-concept
wipe (`../phase-11/overlay-scope-wipe-spec.md` W1-W7, the big one), GI-10
(drive-at-bottom layout), graph loading-state UX, optional GI-11 locks, FB
browserOverlay cleanup (C3 of the wipe).

Surfaces: `web/src/components/{GraphPanel,GraphCanvas,FileBrowserSurface,
FileTree}.svelte`, `web/src/state/{graphData,store,tabs}.svelte.ts`,
`web/src/App.svelte`, `crates/chan-server/src/routes/{fs_graph,graph}.rs`.

SUBAGENTS: @@Alex authorized @@LaneA to spawn its own 2-3 subagents if useful
(e.g. split the W1-W7 wipe). If the spawn tool is unavailable in-session (it was
in phase 11), fall back to in-session skill loading + sub-slices - deliverables
and the per-slice gate are unchanged either way. @@LaneA owns its own internal
coordination; it reports merge-ready slices to @@Architect.

## Lane B: drive -> workspace rename (SCOPE FIRST, then codemod)

@@LaneB is a SCOPING ARCHITECT. Phase 1 = scope, NOT codemod. Produce
`docs/journals/phase-12/workspace-rename-spec.md` covering every surface in the
backlog's rename section (crate, types/API, docs, the "team workspace" name
collision, user-facing CLI/registry/config/errors, the `drive.chan.app` tunnel
domain, uniffi native bindings, back-compat, and the SEQUENCING proposal).
Surface the big decisions to @@Alex via `event-lane-b-alex.md` BEFORE any
codemod - especially: does the user-facing tunnel domain rename, and is the
crate-name/registry break acceptable pre-release. The codemod itself is phase 2,
gated on @@Alex's scope ratification + an @@Architect sequencing window.

## Lane C: @@Alex ad-hoc frontend / cosmetics / keyboard shortcuts

@@Alex-driven. Standing theme: frontend cosmetics + keyboard shortcuts, incl.
the differences between web / Linux desktop / macOS native shortcuts (Cmd vs
Ctrl, native menu accelerators vs DOM handlers, the desktop key-bridge in
`desktop/src-tauri/src/serve.rs`, the chord registry + `web/src/terminal/
keymap.ts`). Requests arrive ad-hoc on `event-architect-lane-c.md` /
`event-alex-lane-c.md`; @@LaneC reports merge-ready slices like the others.

## THE phase-12 coordination problem: heavy frontend overlap

All three lanes touch `web/src`. @@LaneB's codemod touches ~everything,
INCLUDING the files @@LaneA and @@LaneC are actively editing. Strategy:

1. @@LaneB SCOPES first - no codemod while it scopes. @@LaneA + @@LaneC do their
   feature/cosmetic work in parallel during the scoping phase.
2. The codemod lands in a SEQUENCED WINDOW once the scope is ratified: either at
   a quiescent checkpoint (@@LaneA/@@LaneC merged + paused) or as the LAST,
   mechanical pass - @@LaneB proposes which in its scope doc; @@Architect picks
   the window and announces a brief freeze on the affected files.
3. A mid-flight codemod would force massive rebases on @@LaneA/@@LaneC - avoid
   it. If the codemod must be incremental, @@LaneB lands it crate-by-crate /
   area-by-area with the cross-lane channel announcing each so the others
   rebase in small steps.
4. @@LaneA's scope-wipe renames the "scope" concept (graph), @@LaneB renames
   "drive" -> "workspace" - orthogonal terms but the same shared frontend files
   (GraphPanel/store). Declare touches on the cross-lane channel.

@@Architect serializes every merge + re-gates the combined tree, and owns the
codemod window decision.

## Merge + gate protocol (unchanged from phase 11)

- Each lane on its own branch `phase-12-lane-{a,b,c}` in a worktree
  (`../chan-lane-{a,b,c}`); source code only in the worktree, channels +
  journals edited in the MAIN checkout by absolute path.
- Lanes do NOT merge to main; they report "ready to merge: phase-12-lane-X@<sha>"
  on `event-lane-X-architect.md`. @@Architect serializes + re-gates.
- Full gate before any ready-to-merge: `cargo fmt --check`; `cargo clippy
  --all-targets -- -D warnings`; `cargo test`; `cargo build
  --no-default-features`; and in `web/`: `npm run check` + `npm run build`
  (and `web-marketing/` check when touched). FSEvents is recovered on this box.
- Test servers from a SMALL /tmp scratch drive on a scoped port; never serve the
  repo root or docs/; scoped pkills only.

## Coordination bus

Append-only directional channels in `docs/journals/phase-12/coordination/`
(edit by absolute path in the MAIN checkout). READ `event-architect-lane-X.md`
each turn + before any merge-ready report; WRITE progress/blockers/merge-ready
to `event-lane-X-architect.md`; escalate human-decision blockers on
`event-lane-X-alex.md`; cross-lane on `event-lane-X-lane-Y.md` (created on first
use). Keep `docs/journals/phase-12/lane-X/journal.md` self-documenting +
append-only. The bus + journals ARE the record - do not rely on @@Alex relaying
chat.

## Docs commit timing

Phase-12 plans/journals/channels stay UNTRACKED/dirty as the live bus during the
round; @@Architect commits the tree to main as `docs(phase-12): ...` at phase
close (this opening scaffold is committed once as `docs(phase-12): open`).
