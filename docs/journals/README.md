# Journals

Per-phase development logs for chan, kept as the project's second brain:
how we got here and what we tried, did, undid, and roughly how long it
took. Each phase directory has a `README.md` report as its front door,
backed by the raw working material.

## Layout

Each `phase-N/` directory holds:

- `README.md` - the synthesized report (the front door): the initial
  asks, the team and how they coordinated, an estimated duration,
  highlights and lowlights, constructive feedback, and what shipped,
  was tried, and was undone.
- `raw/` - the original working material as provenance: the per-author
  journals and task files, the request and roadmap files, and the
  coordination logs.

Images have been removed across all journals and replaced with short
text descriptions in brackets, so the logs read without binary
attachments.

## Phases

- [Phase 1](phase-1/README.md) - closed. First public-release prep:
  filesystem graph, search status, CLI parity, and hardening.
- [Phase 2](phase-2/README.md) - closed. Graph, editor, and search
  hardening, plus language elevated into the graph.
- [Phase 3](phase-3/README.md) - closed. UI polish and the
  Assistant-to-Agent rename, URL state, and editor fixes.
- [Phase 4](phase-4/README.md) - sparse. A bug-bounty placeholder that
  never ran; effectively a numbering skip (see the report).
- [Phase 5](phase-5/README.md) - closed. The MCP-only refactor,
  persistent terminals, and VCS-aware indexing.
- [Phase 6](phase-6/README.md) - closed. The filesystem made the primary
  graph layer, plus the folder-to-directory terminology change.
- [Phase 7](phase-7/README.md) - closed. Project hygiene, Hybrid panes,
  and the agent-orchestration substrate (v0.10.1, v0.11.0).
- [Phase 8](phase-8/README.md) - closed. Bug sweep, the signed-DMG
  pipeline, and public-flip preparation.
- [Phase 9](phase-9/README.md) - closed. The desktop-native vision,
  drive metadata isolation, and the Rich Prompt revamp (v0.14.0).
- [Phase 10](phase-10/README.md) - closed. The desktop embedded-server
  merge and the public site and manual.
- [Phase 11](phase-11/README.md) - closed. The drive streaming spine,
  editor and graph fixes, and the release contract.
- [Phase 12](phase-12/README.md) - closed. The drive-to-workspace rename
  and the graph and File Browser carryover.
- [Phase 13](phase-13/README.md) - closed. The Graph and Dashboard
  rework, then the Team Work revamp (v0.17.0, v0.18.0).
- [Phase 14](phase-14/) - in progress. Gateway monorepo migration, then a
  frontend review and pristine cleanup. The report lands when the phase
  closes.

## Conventions

Agent references in prose use the `@@{name}` form; contact cards live
under [`../agents/`](../agents/), with legacy handles mapped to their
current successors in [`../agents/README.md`](../agents/README.md).

The coordination process evolved over the project, and each phase report
records the scheme that phase ran on. Phases 1 through 6 used flat task
files at the phase root plus a single shared journal. Phase 7 introduced
the model the later phases refined: one directory per author, append-only
dated journals, typed event-channel files, and architect-orchestrated
dispatch. From phase 11 onward, lanes worked in per-lane git worktrees
for code while the coordination documents stayed in one shared,
append-only bus.

Phase 4 is a real but empty directory, not a missing one: it was a
bug-bounty placeholder that accumulated no work, so the released phase
sequence effectively skips from 3 to 5. Its report explains this.
