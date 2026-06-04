# task LaneF -> Lead (1): Wave 1 repo/docs consolidation DONE

Wave 1 (consolidate, NO deletions) is complete and self-gated. Holding
for your Wave 3 go before ANY deletion. Summary below; detail is on disk.

## 1. Phases consolidated -> docs/phases/ (NEW)

Fanned out one subagent per phase (1-16) plus did the strays myself. All
16 + pub-site-release written to `docs/phases/phase-N.md`, every file in
the same 6-section template (Roadmap / Rounds and waves / Team and
coordination / What shipped, tried, undone / Retrospective / Notes):

- phase-1..14: reshaped from the already-distilled `phase-N/README.md`,
  preserving every fact, the Tags line, terminology-drift notes, and the
  retrospective lessons.
- phase-15 (341 lines): synthesized from the raw 4-round bus (v0.20.0 ->
  v0.23.0).
- phase-16 (327 lines): synthesized from the raw bus (v0.24.0) AND the
  carryover `docs/journals/round-16/` (the chan-desktop launcher redesign
  that landed after the v0.24.0 cut) folded in as the closing round.
- pub-site-release.md: the standalone 2026-06-01 branding/marketing round
  (NOT a numbered phase). Captured so it survives the Wave 3 journals
  deletion. See "Decisions for you" below re: placement.
- docs/phases/README.md: NEW front-door index (replaces the
  to-be-deleted docs/journals/README.md), links every phase + the
  playbook + the agents roster.

Verified across docs/phases/: 0 em dashes, 0 markdown links into
docs/journals/** (raw material pointed to via git-history prose only),
agent refs go to ../agents/README.md only, screenshots described in prose
(text-only). The shared subagent brief is at
`docs/journals/phase-18/team/F-consolidation-spec.md`.

NOT touched: docs/journals/phase-17 and docs/journals/phase-18 (they fold
in at close, Wave 3; phase-18 is the live bus).

## 2. docs/agents distilled

- NEW `docs/agents/playbook.md` (169 lines): the cross-phase operational
  lessons playbook (coordination model + discipline, git/commit in a
  shared worktree, the gate + quality bar, verification/smoke discipline,
  wire/rename/cross-crate discipline, pre-release norms, working inside
  chan). Each lesson cites the phase that taught it. 0 em dashes.
- `docs/agents/README.md` (edited, non-destructive): added a playbook
  pointer; repointed the `@@{name}` link-target note from
  docs/journals/phase-*/ to docs/phases/.

### Keep / cut plan (deletion is Wave 3; nothing deleted yet)

KEEP (the minimal referenced set, under docs/agents/):
- README.md, playbook.md
- Distinct role cards referenced across the phases: architect.md,
  fullstack-a.md, fullstack-b.md, systacean.md, webtest-a.md,
  webtest-b.md (phase-7 active roster) + desktect.md, desktacean.md,
  desktest.md, ci.md (phase-8 desktop + CI lanes).
- orchestration/ (README + atomic-writes + mcp-discovery +
  spawn-protocol): the chan integration contracts / the blueprint for the
  planned watcher return. Recommend KEEP (useful reference for new
  agents); confirm.

CUT in Wave 3 (git rm):
- 8 historical redirect-only cards: backend.md, frontend.md, webdev.md,
  fullstack.md, syseng.md, rustacean.md, backsystacean.md, webtest.md.
  They only redirect old handles to successors; the README "Historical
  handles" map already does that, and no docs/phases/*.md deep-links
  them.
- bootstrap.md (21KB): a stale generated team-bootstrap snapshot. The
  live bootstrap is per-team and tool-generated (cs terminal team
  new --script, e.g. this team's). Its durable essence (working rules,
  standing permissions) is folded into playbook.md.

### Wave 3 scrub list (kept files that cite to-be-deleted paths)

- desktect.md: links docs/journals/phase-8/README.md (x3) + bootstrap.md
  -> repoint to ../phases/phase-8.md and drop the bootstrap.md link.
- README.md "Historical handles" table: links the 8 cut cards -> convert
  to a link-free map.
- README.md "## Skills" + the kept cards' "## Skills" sections: depend on
  the skills/ decision below.
- Plus the task's named source files: chan-workspace/embeddings.rs,
  chan-server/routes/graph.rs, pages.yml (I will grep + scrub stale
  docs/journals|docs/agents path mentions in Wave 3).

## Decisions for you (route to @@Alex if needed)

1. Per-agent skills/ subdirs (12 files, e.g. architect/skills/architect.md):
   COPIES of the shared dotfiles skill library, kept for repo
   self-containment. Not load-bearing for "learn from past phases." CUT
   (canonical source is dotfiles) and scrub the "## Skills" links from the
   kept cards, OR keep for self-containment? I lean cut, but it reverses a
   prior decision, so flagging.
2. pub-site-release: own doc under docs/phases/ (current), or fold into an
   adjacent phase, or drop? It is not a numbered phase but ships real,
   deletable work.
3. Confirm orchestration/ stays.

## Gate

No code touched, so no build gate applies to this lane. Format-checked
the docs by hand (em dash / journal-link / line-width sweeps above).

Pathspec for my Wave-1 output (untracked unless noted):
  docs/phases/ (NEW, 18 files)
  docs/agents/playbook.md (NEW)
  docs/agents/README.md (M)
  docs/journals/phase-18/team/F-consolidation-spec.md (working brief)

Holding for your Wave 3 go before any rm.
