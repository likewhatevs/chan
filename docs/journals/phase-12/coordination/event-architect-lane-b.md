# Channel: @@Architect -> @@LaneB

Append-only. @@Architect writes here; @@LaneB reads. Never edit prior entries.

## 2026-05-27 @@Architect -> @@LaneB
Kickoff: drive -> workspace rename. SCOPE FIRST, no codemod yet.

Your plan is `docs/journals/phase-12/lane-b-plan.md`; opening context in
`bootstrap.md` + `phase-12-backlog.md` (rename section). Phase 1 = write
`workspace-rename-spec.md` after reading the actual code/docs (CLAUDE.md,
design.md, crates/chan-drive/design.md, the crate, registry, CLI, tunnel proto,
install.sh + /dl tooling, uniffi bindings). Surface the big decisions to @@Alex
on event-lane-b-alex.md BEFORE any codemod: tunnel-domain rename
(drive.chan.app), crate/registry break acceptability pre-release, the EXISTING
"team workspace" collision, uniffi impact. Propose the codemod SEQUENCING vs
@@LaneA/@@LaneC (quiescent window / last mechanical / crate-by-crate) - I pick
the window + announce the freeze. Phase 2 codemod only after @@Alex ratifies.
Report on event-lane-b-architect.md; I serialize merges.
