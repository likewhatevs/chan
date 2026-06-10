# Phases

The development history of chan, one consolidated report per phase. Each
`phase-N.md` is the phase's front door: its roadmap (the asks), how the
rounds and waves were structured, who worked it and how they coordinated,
what shipped, and a retrospective with the lessons worth carrying forward.

These reports are the project's second brain. The raw per-author journals
and coordination buses that backed them were distilled into these files
and removed from the working tree; the raw material is preserved in git
history under the former `docs/journals/phase-N/` trees.

New agents should also read [`.agents/playbook.md`](../../.agents/playbook.md)
(the operational lessons distilled across every phase) and the contact
cards in [`.agents/roster/README.md`](../../.agents/roster/README.md).

## Index

- [Phase 1](phase-1.md) - first public release prep: filesystem graph,
  search status, CLI parity, hardening.
- [Phase 2](phase-2.md) - graph, editor, and search hardening, plus
  language elevated into the graph.
- [Phase 3](phase-3.md) - UI polish and the Assistant-to-Agent rename,
  URL state, and editor fixes.
- [Phase 4](phase-4.md) - sparse bug-bounty placeholder; effectively a
  numbering skip from 3 to 5 (see the report).
- [Phase 5](phase-5.md) - the MCP-only refactor, persistent terminals,
  and VCS-aware indexing.
- [Phase 6](phase-6.md) - the filesystem made the primary graph layer,
  plus the folder-to-directory terminology change.
- [Phase 7](phase-7.md) - project hygiene, Hybrid panes, and the
  agent-orchestration substrate (v0.10.1, v0.11.0).
- [Phase 8](phase-8.md) - bug sweep, the signed-DMG pipeline, and
  public-flip preparation.
- [Phase 9](phase-9.md) - the desktop-native vision, drive metadata
  isolation, and the Rich Prompt revamp (v0.14.0).
- [Phase 10](phase-10.md) - the desktop embedded-server merge and the
  public site and manual.
- [Phase 11](phase-11.md) - the drive streaming spine, editor and graph
  fixes, and the release contract.
- [Phase 12](phase-12.md) - the drive-to-workspace rename and the graph
  and File Browser carryover.
- [Phase 13](phase-13.md) - the Graph and Dashboard rework, then the Team
  Work revamp (v0.17.0, v0.18.0).
- [Phase 14](phase-14.md) - Gateway monorepo migration into a nested
  workspace, then a frontend review and pristine cleanup, plus paced
  graph hot paths and the new-workspace pre-flight relocation.
- [Phase 15](phase-15.md) - Dashboard carousel, the cs CLI, Team Work
  plus the survey rebuild, and the indexing hardening thread
  (v0.20.0 through v0.23.0).
- [Phase 16](phase-16.md) - cs lead-tooling, a long host-driven feature
  and polish stream, and the chan-desktop launcher redesign (v0.24.0).
- [Phase 17](phase-17.md) - host bug sweep, survey v2, and the desktop
  connecting screen (v0.25.0).
- [Phase 18](phase-18.md) - hybrid-surface bug sweep (editor lists, graph,
  File Browser, inspector pills, terminal), the repo/docs fold, and the
  v0.26.0 release wave.
- [Phase 19](phase-19.md) - Linux/WebKitGTK desktop parity, the in-tree
  Drafts model, the graph @@mention lens plus contact nodes and offline
  reconcile, and the .agents/ docs fold (v0.26.1 through v0.28.1).
- [Phase 20](phase-20.md) - chan-desktop refinements (plain-text update
  prompt, unified About) and standalone terminal windows, run as an
  orchestrator + subagents round.
- [Phase 21](phase-21.md) - terminal cross-window awareness: a cross-window
  broadcast roster (menu + indicators + toggle gate), per-tenant Terminal-N
  naming, group-wide cross-window Select All, and tenant-wide name
  uniqueness (v0.29.0, with phase-20).
- [pub-site-release](pub-site-release.md) - a standalone branding and
  positioning re-steer plus marketing site refresh (not a numbered
  phase).

## Conventions

Agent references in prose use the `@@{name}` form and resolve to the
contact cards under [`.agents/roster/`](../../.agents/roster/). The
coordination scheme evolved over the project; each report records the
scheme that phase ran on, and the cross-phase summary is in
[`.agents/playbook.md`](../../.agents/playbook.md). Reports are text only;
a load-bearing screenshot is described in prose rather than embedded.
