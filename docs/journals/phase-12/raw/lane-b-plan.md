# @@LaneB plan: drive -> workspace rename (phase 12, SCOPE FIRST)

You are @@LaneB, a SCOPING ARCHITECT. Full opening context: `bootstrap.md` +
`phase-12-backlog.md` (the rename section). Phase 1 is SCOPE, not codemod.

## Phase 1 - scope (do this first, no codemod)

Read the actual code/docs before describing anything (CLAUDE.md, design.md,
crates/chan-drive/design.md, the chan-drive crate, the registry, CLI help, the
tunnel proto, install.sh + /dl tooling, native uniffi bindings). Then write
`docs/journals/phase-12/workspace-rename-spec.md` covering every surface in the
backlog's rename section, and SURFACE the big decisions to @@Alex on
`event-lane-b-alex.md` BEFORE any codemod:
- Does the user-facing tunnel domain `drive.chan.app` / `{user}.drive.chan.app`
  rename? (coordinate with release/build carryover + a future release lane).
- Is the crate-name + on-disk registry/config break acceptable pre-release (no
  migration), or is back-compat required?
- How to disambiguate from the EXISTING "team workspace" concept (Drafts/
  metadata) so `drive` -> `workspace` does not clash.
- uniffi native (iOS/Android) impact of the `chan-drive` crate rename.

Also propose the SEQUENCING: when does the codemod land relative to @@LaneA
(graph/FB) + @@LaneC (cosmetics) work on the same files - a quiescent window,
or a last mechanical pass, or crate-by-crate increments. @@Architect picks the
window + announces the freeze.

## Phase 2 - codemod (only after @@Alex ratifies the scope)

Execute per the ratified spec in the @@Architect-chosen window. Each chunk
independently gated + merge-ready ("ready to merge: phase-12-lane-b@<sha>" on
event-lane-b-architect.md). Touching .github/workflows or Cargo.lock = shared
infra (state authorization inline; secret NAMES only).
