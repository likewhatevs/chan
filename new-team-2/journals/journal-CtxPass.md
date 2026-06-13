# journal-CtxPass — round 1 (new-team-2)

Append-only.

## 2026-06-12 — B1 design doc cut (no code edited)

- Read task-Conductor-CtxPass-5, backlog-ctx-pass.md, round-1
  inventory (task-Chan-Lead-1), precedent commit 01d0cba6.
- Read every in-scope definition + orchestrator at HEAD e0ec0d3c:
  build_graph_view + merge_* (routes/graph.rs), Indexer::spawn /
  spawn_coordinator / spawn_watcher_loop / set_idle / reconcile_idle
  (indexer.rs), build_fs_graph_paged, create_followup_file,
  GraphView::replace_file, drafts scan_entries/promote, contacts
  slug_for/import::run, TerminalRegistry::restart, handle_team +
  ControlSocketCtx.
- Qualified call-site sweeps via `rg --text --no-ignore` (one sweep
  re-run after it caught the embedded desktop binary; excluded
  binaries/target). Round-1 param counts do not reproduce — verified
  table in the design doc. Caught the replace_file name collision
  (GraphView vs VectorStore — only GraphView's is in scope).
- Wrote new-team-2/designs/b1-ctx-pass-design.md: TreeMergeCtx
  (wave 1), IndexerShared widening (wave 2), FileRecord /
  DraftScanAccum / SlugAllocator / FsGraphParams / FollowupSpec + two
  leave-loose calls (wave 3), RestartOverrides (wave 4a, gated),
  TeamRequest + ControlSocketCtx reuse (wave 4b, gated; ControlRequest
  wire shape frozen).
- Cut task-CtxPass-Conductor-6 (sign-off ask + 6 flagged decisions);
  poked @@Conductor. Holding — no code before sign-off.

## 2026-06-12 — sign-off received (task-Conductor-CtxPass-7); wave 1 burst

- All six flagged decisions ruled: counts supersede inventory,
  collision exclusion confirmed, allow-retirements approved as lead
  call, promote/import::run stay LOOSE (do not group), wave-3 as five
  per-family commits, doc-sync riders coupled.
- RETRO NOTE (per decision 1, for round-close): the round-1 inventory
  in new-team-1/tasks/task-Chan-Lead-1.md "Remaining 6+-param
  inventory" records param counts 1-4 higher than HEAD e0ec0d3c
  reproduces (merge_* "11/9/9/8" vs actual max 7; restart "8" vs
  6+self; handle_team "11" vs 9; scan_entries "9" vs 8; replace_file
  "10" vs 9+self). Fix at source when the round closes.
- Wave-4b watch item from @@Conductor: registry resolution moving
  inside handle_team (ctx.terminal_registry.get()) must be called out
  to @@PromptQueue explicitly as the design's only observable-order
  change.
- BURST (wave 1, starting now): routes/graph.rs — TreeMergeCtx struct
  + 4 helpers become methods (push_contains_edge,
  ensure_directory_path, merge_tree_file_node, merge_tree_entry);
  merge_directory_node + contains_edge_key stay free fns;
  merge_unified_tree_layer constructs the ctx at today's edge_set
  build point. Single file, zero test edits expected.
- Wave 1 LANDED: 7c6a36af (routes/graph.rs only, 106+/109-).
  Main-tree cargo check was red from @@PromptQueue's in-flight item-2
  WIP in terminal_sessions.rs (NOT my burst; verified via git status +
  error locations), so the gate ran in an isolated worktree at HEAD
  c9fbb909 + my file only: check green, fmt clean (one wrap fixed,
  gate re-run after), clippy --all-targets + test green under
  RUSTFLAGS="-D warnings". One 1-of-5 intermittent test failure
  (418/419) never reproduced in 4 captured runs and is not in graph
  tests — environment flake, flagged here for the round gate.
- @@Conductor poke received mid-gate: wave 4b RELEASED (item-5 Part B
  = 86a0dce9, team_config.rs only), binding order unchanged (4b after
  waves 1-3), 4a still GATED; handle_team fields to re-verify at HEAD
  when I get there. Acked in the wave-1 sha poke.
- BURST (wave 2, starting now): indexer.rs only — IndexerShared widens
  with cancel + search_aggression; spawn_coordinator drops the allow +
  counter-comment and takes (workspace, shared, rebuild_rx,
  progress_sink); spawn_watcher_loop drops its loose cancel/aggression
  params; set_idle/reconcile_idle take (workspace, &shared); 3 tests
  construct IndexerShared. Indexer::spawn pub signature untouched.
- Wave 2 LANDED: 396ad164 (indexer.rs only, 65+/70-). Gate in the
  isolated worktree (HEAD-equivalent 7c6a36af + file): check/fmt/
  clippy/test all green under -D warnings, 419/419. The retired
  spawn_coordinator allow+comment noted in the commit message per
  decision 3. Proceeding to wave 3 (five per-family commits).
- BURST (wave 3a, starting now): chan-workspace graph.rs FileRecord
  for GraphView::replace_file + workspace.rs prod call +
  design.md:1041 sync. DESIGN-DOC CORRECTION found mid-read:
  replace_file carries its OWN allow(too_many_arguments) + counter-
  comment (graph.rs:722-724, "would churn ~20 call sites for a style
  win") that the design doc missed — decision-3 logic covers it
  (designed pass supersedes churn objections), retiring it too; tally
  becomes 4 allows retired, not 3. Flagged to @@Conductor in the
  wave-3a poke.
- Wave 3a LANDED: c15f6b35 (chan-workspace graph.rs + workspace.rs +
  design.md sync). 19 call sites to FileRecord named fields; clippy
  caught one scripted-rewrite shorthand nit (rel: rel), fixed, full
  gate re-run after: fmt/clippy/test 543-pass green under -D
  warnings; chan-server/chan/chan-shell dependents check green.
  Third allow+counter-comment retired (see burst note above).
- BURSTS (waves 3b-3e, sequential, announced together): 3b drafts.rs
  DraftScanAccum (allow retired; single file); 3c contacts SlugAllocator
  (chan-workspace slug.rs+import.rs + chan main.rs in ONE burst -
  dependent crates - + design.md:1187 sync); 3d fs_graph.rs
  FsGraphParams (single file); 3e survey.rs FollowupSpec (single
  file). Each gated + committed per-family before the next starts.
- Wave 3b LANDED: 6e4253d4 (drafts.rs only, 30+/50-). Gate green
  first run: fmt/clippy/full-test 543-pass under -D warnings.
- Wave 3c LANDED: f82aae50 (slug.rs + import.rs + chan main.rs +
  design.md, one cross-crate burst). Gate green: fmt/clippy both
  crates, chan-workspace 543 + chan 62 tests under -D warnings.
- Wave 3d LANDED: 8f070e36 (fs_graph.rs only). DESIGN AMENDMENT: the
  module already had an FsGraphParams query type carrying exactly the
  designed fields; reused it (&FsGraphParams) instead of minting the
  duplicate borrow-struct from the doc - same GraphParams idiom the
  design invoked. Gate green: fmt/clippy/test 419 under -D warnings.
- Wave 3e LANDED: e249de55 (survey.rs only). WAVE 3 COMPLETE:
  c15f6b35 / 6e4253d4 / f82aae50 / 8f070e36 / e249de55, each
  per-family pathspec-atomic, each gate green under -D warnings.
- Wave 4b now unblocked (Conductor's gate-release poke + waves 1-3
  done; 4a still GATED on item-2 server half). Per the release poke:
  re-verify handle_team fields at HEAD first (item-5 Part B =
  86a0dce9, team_config.rs only).
- Re-verified handle_team at HEAD e249de55 per the gate-release poke:
  signature still the 9 params from the design (86a0dce9 touched
  team_config.rs only); TerminalTeam variant fields unchanged.
- BURST (wave 4b, starting now): control_socket.rs only - TeamRequest
  (the TerminalTeam variant's fields, built at the dispatch site;
  wire enum untouched) + handle_team(req, &ctx) on the existing
  ControlSocketCtx; registry resolved INSIDE handle_team via
  ctx.terminal_registry.get() (the design's only observable-order
  change - flagged for adversarial review); allow + counter-comment
  retired; 8 tests onto test_ctx.
- Wave 4b LANDED: 126d9285 (control_socket.rs only, 113+/104-). Gate
  green at f198df7b: fmt/clippy/test 424-pass incl. all 7 handle_team
  tests under -D warnings. Registry-resolution watch item called out
  in the commit message for @@PromptQueue's adversarial review.
- task-Conductor-CtxPass-13 received: wave 4a RELEASED (item-2 server
  half = ca40ea6b) + cross-review of ca40ea6b routed to me, 8
  targets, recommended before 4a. Doing cross-review now (one
  at-HEAD read serves both deliverables).
- Cross-review ca40ea6b COMPLETE: clean pass on all 8 targets, 2
  informational notes (N1 QueueDepth broadcast under the registry
  guard via enqueue_write_matching - sync send, no cycle, suggest a
  comment; N2 the "update 4 existing queue tests" design line needed
  no edits in practice). task-CtxPass-Conductor-14.md cut.
- Confirmed ca40ea6b leaves restart/restart_matching untouched - the
  signed-off RestartOverrides design applies unchanged.
- BURST (wave 4a, starting now): terminal_sessions.rs +
  routes/terminal.rs - RestartOverrides struct, restart(&self, id,
  overrides); restart_matching passes RestartOverrides::default();
  route builds the literal. Both files just landed @@PromptQueue work
  so I re-read the exact call-site text at HEAD before editing.
- Wave 4a LANDED: 3c45f35a (terminal_sessions.rs + routes/terminal.rs,
  40+/25-). Gate green at HEAD-equivalent 126d9285: fmt/clippy/test
  424-pass under -D warnings. B1 COMPLETE — all 8 commits in
  task-CtxPass-Conductor-15.md.
- @@Conductor RATIFIED the 3a deviation mid-close (4 allows retired,
  0 added; decision-3 rationale extended to replace_file's), and will
  route wave-3 cross-reviews as one batched task with the deviation
  flag included. Acked in the completion poke.
- Gate worktree /tmp/ctxpass-b1-gate removed (git worktree remove).

## 2026-06-13 — queued-poke reconciliation (post-B1-complete)

- Two @@Conductor pokes arrived behind my lane state (both cut before
  4a/4b/15 landed; reconciled against HEAD, nothing to redo):
  (1) 3d amendment RATIFIED + review flag; (2) ca40ea6b review
  ACCEPTED 8/8 (N1 -> follow-ups as one-line-comment candidate, N2
  recorded).
- 3d review flag VERIFIED at HEAD 3c45f35a:
  (a) serde-default equivalence is structurally guaranteed —
  FsGraphParams has no Default impl, so every internal/test literal
  must spell all 5 fields (all 6 construction sites verified
  explicit); serde defaults (scope=Directory, depth=1) live only on
  the untouched Query<FsGraphParams> wire path.
  (b) build_fs_graph wrapper: byte-identical across 8f070e36 (diff
  hit was hunk-header context only), all 10 loose-signature callers
  (route non-paged branch, 8 tests, graph.rs:1091) untouched.
- B1 ACCEPTED IN FULL by @@Conductor: 8 commits verified at landing,
  2 ratified deviations, 4 allows retired / 0 added, zero wire
  changes. 4a+4b review batch routed (not to me); the 418/419 flake
  is now the round gate-#3 watch item; the round-1 inventory fix is a
  retro item. LANE CLOSED — holding for round close.
