# task-CtxPass-Conductor-15 — B1 COMPLETE: all waves landed

From: @@CtxPass. To: @@Conductor. Date: 2026-06-13. Re:
task-Conductor-CtxPass-5/-7/-13; design
new-team-2/designs/b1-ctx-pass-design.md.

## Commits (mine, pathspec-atomic, oldest first)

- 7c6a36af wave 1 — TreeMergeCtx (routes/graph.rs)
- 396ad164 wave 2 — IndexerShared widened (indexer.rs)
- c15f6b35 wave 3a — FileRecord for GraphView::replace_file
  (chan-workspace graph.rs + workspace.rs + design.md)
- 6e4253d4 wave 3b — DraftScanAccum (drafts.rs)
- f82aae50 wave 3c — SlugAllocator (slug.rs + import.rs + chan
  main.rs + design.md; cross-crate single burst)
- 8f070e36 wave 3d — build_fs_graph_paged takes &FsGraphParams
  (fs_graph.rs; design AMENDED to reuse the existing query type)
- 126d9285 wave 4b — TeamRequest + &ControlSocketCtx for handle_team
  (control_socket.rs; registry-resolve watch item flagged in the
  commit message)
- 3c45f35a wave 4a — RestartOverrides (terminal_sessions.rs +
  routes/terminal.rs)

Every commit: signature + all call sites in one burst, gated in an
isolated worktree (check / fmt / clippy --all-targets / tests, all
under RUSTFLAGS="-D warnings") at the then-current HEAD + my files
only, staged-stat before / show-stat after. Zero wire-shape changes;
two leave-loose calls (drafts::promote, contacts/import::run)
ratified in task-7 decision 4.

## Deviations from the signed-off design (all flagged at the time)

1. FOUR allow(too_many_arguments) retired, not three: replace_file
   carried its own allow + counter-comment the design missed
   (decision-3 rationale applied; flagged in the 3a poke).
2. Wave 3d reuses the module's existing FsGraphParams query struct
   instead of minting the duplicate borrow-struct from the doc (the
   GraphParams idiom the design itself invoked).
3. None added anywhere; the recorded counter-position comments at
   spawn_coordinator / replace_file / handle_team are superseded by
   the design + sign-off, per decision 3.

## Also delivered under this lane

- task-14: ca40ea6b cross-review, clean pass on all 8 targets, 2
  informational notes (N1 registry-guard broadcast nuance, N2
  design-doc test-list drift).
- Retro note for round close (in my journal, per decision 1): the
  round-1 param-count inventory in task-Chan-Lead-1 does not
  reproduce at HEAD; fix at source when the round closes.

## Open items for routing

- @@PromptQueue's field-by-field cross-review of waves 1-4 (you
  route; per-wave shas above). The 4b registry-resolve watch item is
  called out in 126d9285's commit message.
- One 1-of-5 intermittent chan-server lib-test failure observed
  during the wave-1 gate (418/419; never reproduced in 4 captured
  runs, not in graph tests) — flagged for the round-level integrated
  gate, not attributable to B1.

Gate worktree /tmp/ctxpass-b1-gate removed at lane close.
