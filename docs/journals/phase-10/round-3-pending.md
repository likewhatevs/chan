# Phase 10 Round 3 Pending Work

Date: 2026-05-25.

> Migrated 2026-05-26: the remaining open items in this index moved to
> `docs/journals/phase-11/phase-11-round-2.md`. Track A item 4 (Tauri icon +
> desktop docs/config audit) closed in phase 10. Of the rest: Linux desktop
> launch and the macOS CLI-to-desktop handoff carry forward; Linux native
> drag-out is superseded (drag in/out removed in phase 11); release
> verification is no longer blocked now that the repo is public and the
> release process is ironed out; the manual/site streaming-copy update is
> deferred behind the phase-11 partial-load rework. See that file for the
> decisions. The text below is the historical round-3 snapshot.

Snapshot code baseline: `454bfa2` (`fix(desktop): install web deps before
build`).

This document captures the items still pending in the repo state at the
snapshot code baseline above. It is a Round 3 planning index for Track A and
Track B only. Track C is complete per
`docs/journals/phase-10/roadmap-track-c.md` and the Track C closeout in
`track-c-next-agent-handoff.md`.

Desktop implementation is intentionally not started by this docs pass.
Desktop-owned pending items are listed only so Round 3 has a complete queue.

## Source Docs

- `docs/journals/phase-10/roadmap-track-a.md`
- `docs/journals/phase-10/track-a-round-2-handoff.md`
- `docs/journals/phase-10/track-a-handoff-from-track-b-logo-and-docs.md`
- `docs/journals/phase-10/roadmap-track-b.md`
- `docs/journals/phase-10/roadmap-track-c.md`
- `docs/journals/phase-10/track-c-next-agent-handoff.md`
- `docs/journals/phase-10/rich-prompt-watcher-audit.md`

## Round 3 Track Docs

- Track A:
  `docs/journals/phase-10/track-a-round-3-handoff.md`
- Track B:
  `docs/journals/phase-10/track-b-round-3-handoff.md`

## Ad-Hoc Sessions Landed Before Round 3

These items are already present in the snapshot code baseline. They are not
opened as Round 3 pending work unless a regression is found.

- Terminal xterm.js/session routing: `3ce1db0` restored xterm.js as the
  terminal renderer after the ghostty-web experiment, removed ghostty-web, and
  isolated PTY session routing so split panes, tab moves, and terminal
  reconnects cannot reuse another terminal's buffer. It also kept the two
  specific fixes found during the audit: trailing resize fit for xterm.js and
  Cmd+. handling through the shortcut/chord registry.
- Draft management: `05c5cee` updated draft preflight/list handling so team
  workspace metadata under `Drafts/` is skipped instead of being treated as a
  broken draft that must contain `draft.md`.
- Desktop new-clone build: `454bfa2` updated `desktop/Makefile` so the desktop
  build path installs `web` dependencies before building the embedded web
  bundle and release helper binary. This is recorded only as landed work.

## Track A Pending

Track A owns backend, desktop shell, CLI handoff, MCP, API, release, and
server-side Rich Prompt follow-ups.

Pending items:

- CLI-to-desktop handoff design.
- Linux desktop launch validation.
- Linux native drag-out validation, after Linux desktop launch is viable.
- Release validation.
- Tauri app icon regeneration, from the Track B to Track A handoff.
- Desktop docs and config audit, from the Track B to Track A handoff.
- Rich Prompt watcher reattach decision and implementation, if selected.
- Rich Prompt pre-flight backend-dispatch decision, if selected.
- `AgentEventEcho` replay or loss-acceptance decision, if selected.

See `track-a-round-3-handoff.md` for details and sequencing.

## Track B Pending

Track B owns public site, generated manual, install surface, release link
validation, and site/manual updates.

Pending items:

- Run full release verification after the next `chan-v*` tag includes the
  manual bundle and the repository is public.
- Update `docs/manual/` and generated public manual copy for current
  streaming, relationship loading, graph streaming, and inspector transfer
  behavior.
- Run `cd web-marketing && npm run check` after those manual/site edits.

See `track-b-round-3-handoff.md` for details and gating.

## Track C Status

No Round 3 Track C queue is opened from this snapshot.

Track C closeout says these are complete:

- Rich Prompt browser validation.
- Spawn agents clipboard and pre-flight validation.
- Rapid editor autosave and search-index convergence.
- Streaming inspector/report/backlink/graph UI.
- Shared inspector Upload/Download.
- Draft explicit Save-to-drive.
- Terminal ANSI and scroll-heavy pane switching.
- Graph filesystem spine.
- Matrix and plain screen-lock follow-ups.

## Suggested Order

1. Track A: choose either CLI handoff design or Linux launch validation.
2. Track A: run release validation only when the release cut is intended.
3. Track B: update manual/site copy once the owner confirms the current
   streaming and inspector-transfer behavior is final enough for public docs.
4. Track B: run full release verification only after the next public
   `chan-v*` tag exists.
5. Separately select whether the Rich Prompt watcher audit follow-ups enter
   Track A Round 3 or remain backlog.
