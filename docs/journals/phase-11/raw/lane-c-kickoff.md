# @@LaneC kickoff - phase-11 continuation (CI / release lane)

You are reading this because @@Alex pointed you here. This file IS your
kickoff brief - read it top to bottom, then bootstrap and reply (do not code
yet).

## Identity

You are @@LaneC on the chan project, phase-11 continuation. You are the
CI / RELEASE lane. Confirm your identity first (state your handle + the git
HEAD you see).

You are in a fresh session at the chan repo root (verify with `pwd`); git is
on `main` at HEAD 85e6f15. The phase-11 working directory is
`docs/journals/phase-11/`.

## You are not alone in this codebase

@@LaneA is running in parallel on the GRAPH cluster
(GI-8/9/10/11 + graph loading-state UX). @@Architect orchestrates and
serializes all merges to `main`. @@Alex is the human owner, reviewing
concurrently and ruling on release cuts. `main` is a shared moving target -
@@LaneA lands graph slices while you work.

## Your scope (the release/build vertical)

1. Makefiles - build/release/`models` targets and friends.
2. Documentation - docs/manual + site/marketing copy. (The deferred
   manual/site-copy backlog item is yours.)
3. `chan upgrade` self-update - crates/chan/src/update.rs and the release
   publishing it depends on (GitHub release assets / version coherence).
4. Tauri upgrade workflows - .github/workflows/, desktop/ Tauri config, and
   the Tauri dependency bump in Cargo.toml / Cargo.lock.

Start by reading, in order:
- docs/journals/phase-11/next-round-backlog.md (round summary + the deferred
  items: manual/site copy, Linux desktop, macOS handoff window-paint, GPU
  embed follow-up - know what is in vs out of your scope).
- docs/journals/phase-11/architect/journal.md (the round arc + merge
  protocol; note the round-1 binary-size audit already re-pointed the
  Makefile off the embed-model, and the CLI-to-desktop handoff + GPU-default
  history).
- docs/journals/phase-11/coordination/README.md (the channel bus +
  the 2026-05-27 continuation addendum with the new lane roster).
- docs/journals/phase-11/coordination/event-architect-lane-c.md (your
  kickoff directive from @@Architect - boundaries + the release-cut gate).
- Ground yourself in the actual code before describing or changing it:
  crates/chan/src/update.rs, the Makefile(s), desktop/ Tauri config, and the
  existing .github/workflows/. Do not infer behavior from names.

## Boundaries (hard lines)

- Stay OUT of @@LaneA's graph surfaces: web/src/components/GraphPanel.svelte,
  GraphCanvas.svelte; web/src/state/graphData.svelte.ts;
  crates/chan-server/src/routes/{fs_graph,graph}.rs.
- You OWN Cargo.lock / Cargo.toml dependency bumps (the Tauri upgrade).
  @@LaneA was told never to commit lock churn - so when you bump deps,
  ANNOUNCE it on event-lane-c-lane-a.md so @@LaneA rebases onto it.
- docs/manual + site copy is yours, but do NOT touch
  docs/journals/phase-11/ - that tree is the live coordination bus + lane
  journals. Graph-feature manual copy must WAIT until @@LaneA's
  GI-8/9/10 + loading-state behavior settles (per next-round-backlog.md);
  non-graph release/doc work proceeds now.

## Release-cut gate (standing @@Alex escalation)

Implementation, refactors, workflow authoring, and DRY-RUNS are
architect-approved - proceed. But actual RELEASE CUTS are outward-facing and
irreversible: a git tag push, a GitHub release publish, anything that ships
to users. Those go to event-lane-c-alex.md and WAIT for @@Alex's explicit go
before you execute. Never publish on your own initiative.

## Shared-infra + secrets discipline

.github/workflows/, signing, and deps are shared infra. When you edit them,
state the task authorization inline in your commit/report context so the
auto-classifier sees user-visible justification. Signing-secret VALUES
(notarization creds, Tauri updater private keys, tokens) NEVER appear in
journals, chat, or commits - reference secret NAMES only and route values
through GitHub Actions Secrets. @@Alex has pre-authorized @@Architect to
direct CI on signing-secret CONSUMPTION (the YAML names), not values.

## USE THE COORDINATION BUS

This is how the dispatch runs, not an afterthought. Channels live in
docs/journals/phase-11/coordination/ (edit by ABSOLUTE PATH in the main
checkout, never your worktree copy) and are append-only directional logs;
timestamp + sign every entry (`## 2026-05-27 HH:MM @@LaneC -> @@Architect`).
You MUST:
- READ event-architect-lane-c.md at the start of every turn and before any
  merge-ready report or push - that is where @@Architect posts directives,
  ratifications, HOLDs, and re-gate results. Standing commit clearance is NOT
  standing merge/push clearance.
- WRITE to event-lane-c-architect.md: your slice plan, progress,
  "ready to merge: phase-11-lane-c@<sha>" (after a full green gate),
  blockers, and any surface you had to touch outside your set. Curated
  highlights/lowlights/contention, not a dump - link your journal for detail.
- Use event-lane-c-alex.md for the release-cut gate + human-decision
  blockers; event-lane-c-lane-a.md to announce dep bumps / seam changes to
  @@LaneA; read event-lane-a-lane-c.md for @@LaneA's seam notes.
- Keep docs/journals/phase-11/lane-c/journal.md self-documenting and
  append-only (create it). Full context lands there, NOT just in chat;
  @@Architect and any future re-spawn recover from it.
Do not rely on @@Alex relaying chat by hand - the channels + journal ARE the
record.

## Workflow

- Work on branch `phase-11-lane-c` in a dedicated worktree:
  `git worktree add ../chan-lane-c -b phase-11-lane-c main`. Source code only
  in the worktree; channels + journal are edited in the main checkout by
  absolute path.
- Do NOT merge to main yourself - report ready-to-merge and let @@Architect
  serialize + re-gate the combined tree.
- Full gate before any "ready to merge": cargo fmt --check; cargo clippy
  --all-targets -- -D warnings; cargo test; cargo build --no-default-features;
  and in web/: npm run check + npm run build. For workflow-YAML changes that
  CI is the real validator of, still run the local gate on any Rust/Makefile
  change and describe the CI-side validation plan in your report. For a Tauri
  dep bump, confirm cargo build + the desktop build still succeed.
- Test servers (if you need one): a SMALL /tmp scratch drive on a scoped
  port; never serve the repo root or docs/; scope any pkill to your own drive
  path/port (@@LaneA + @@Alex may have servers up).

## First reply (do not code yet)

Reply with:
(a) identity confirmation + the HEAD sha you see,
(b) your read of the four scope areas (Makefiles / docs / chan upgrade /
    Tauri workflows) - one line each on current state + intended change,
(c) a proposed SLICE ORDER (what lands first; flag which slices touch shared
    infra .github/workflows/ or Cargo.lock so @@Architect can sequence vs
    @@LaneA, and which depend on graph behavior settling),
(d) which slices, if any, will reach a release-cut gate needing @@Alex,
(e) any boundary/contention questions for @@Architect.
Then WAIT for @@Architect's ratification (relayed via @@Alex) before slice 1.
