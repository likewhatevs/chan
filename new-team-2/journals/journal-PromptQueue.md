# journal — @@PromptQueue (new-team-2)

Append-only. Owner: @@PromptQueue.

## 2026-06-12 — task received

- Poked by @@Conductor: task-Conductor-PromptQueue-2.md (item 2, Rich
  Prompt queue visibility). Design doc read in full
  (designs/item-2-prompt-queue-visibility.md).
- Sequencing acknowledged: server half FIRST (terminal_sessions.rs +
  routes/terminal.rs), milestone poke with sha to @@Conductor (gates
  @@CtxPass wave 4). Web half second; Pane.svelte badge WAITS for
  @@Editor's restructure — will not touch Pane.svelte until poked.
- chan-server is three-lane hot: signature change + all call sites in
  one burst, `cargo check -p chan-server` green before pausing.
- Now: verifying design line numbers against main @ e0ec0d3c, then the
  server burst.

## 2026-06-12 — server burst starting (multi-file Rust burst announcement)

- Verified design anchors against HEAD e0ec0d3c: write_queue @982,
  try_drain_one @1258, enqueue_write @1306, queue tests @1963-2050;
  routes ClientFrame @108, ServerFrame @150, session frame @564,
  Prompt arm @653, rx arm @690. All match the design.
- Confirmed contract surfaces: SessionEvent matched ONLY in
  routes/terminal.rs; Session::enqueue_write callers are
  enqueue_write_matching (CLI, contract frozen) + the Prompt arm (being
  replaced); team identity pokes use write_input_matching (untouched).
- BURST (chan-server, two files, one signature cluster):
  terminal_sessions.rs (QueuedWrite, SessionEvent variants,
  enqueue_prompt, queue_depth, try_drain_one emission, tests) +
  routes/terminal.rs (Prompt.id, 3 ServerFrames, session.queue_depth,
  Prompt arm, rx arm, tests). `cargo check -p chan-server` green
  before pausing.

## 2026-06-12 — server half LANDED: ca40ea6b

- Commit ca40ea6b (pathspec-atomic: terminal_sessions.rs +
  routes/terminal.rs only; staged-stat clean before, show-stat
  verified after). chan-server compile window closed — @@CtxPass
  wave 4 unblocked.
- Scoped own-gate green AFTER the final edit (post-fmt re-run):
  `cargo fmt --check` 0, `RUSTFLAGS="-D warnings" cargo clippy
  --all-targets` 0, `RUSTFLAGS="-D warnings" cargo test -p
  chan-server` 424 passed / 0 failed. New tests:
  enqueue_prompt_is_all_or_nothing_at_cap,
  queue_depth_counts_messages_not_writes,
  drain_emits_delivered_on_last_write_only,
  enqueue_broadcasts_queue_depth_on_both_paths,
  server_frame_queue_wire_shapes, prompt decode pin extended with id.
- Regression: control_socket.rs untouched; enqueue_write raw-length
  return + EnqueueOutcome + WRITE_QUEUE_CAP=100 byte-for-byte; the 4
  pre-existing queue tests pass unmodified.
- Bonus per design: the at-cap gemini CR silent-drop is fixed
  (all-or-nothing enqueue_prompt).
- ACK task-Conductor-PromptQueue-8 (B1 wave-1 cross-review of
  7c6a36af): queued BEHIND item-2 per its priority order. I'll take it
  at my next natural break — likely while web half waits on the
  Pane.svelte gate, or right after the web half lands.
- Next: item-2 web half (tabs store → TerminalTab frames → RichPrompt
  state machine → vitest; Pane.svelte badge still WAITING on
  @@Editor).

## 2026-06-12 — web half landed (86d50a25) + B1 review batch done

- Web half commit 86d50a25 (6 files, pathspec-atomic; first attempt
  failed cleanly on the untracked new test file — staged it, re-ran,
  show-stat verified). Gate green: svelte-check 0 errors, `make
  web-check` vitest 176 files / 1743 tests, production build OK.
  Pane.svelte untouched (waiting on @@Editor per sequencing).
- ACK task-Conductor-PromptQueue-12 mid-burst; finished the RichPrompt
  edits to coherence first, then took the review batch at the natural
  break, per the tasks' own priority order.
- B1 cross-reviews (tasks 8 + 12): BOTH CLEAN — full field-by-field
  mapping in tasks/task-PromptQueue-Conductor-18.md. Key refutations:
  wave-1 edge_set seed point unmoved; wave-2 cancel-Arc identity
  preserved end-to-end (Indexer struct included).
- Remaining on item 2: browser smoke of the RichPrompt state machine +
  the manual recipe on a throwaway standalone server; Pane badge on
  @@Conductor's clear; then ONE completion poke.

## 2026-06-12 — item 2 complete (badge held); manual recipe ALL PASS on the wire

- @@Conductor accepted both B1 reviews; web-half review routed to
  @@TeamFlow, server half to @@CtxPass (task-13); badge still HELD.
- Manual recipe: Chrome origin permission DENIED 3× for the throwaway
  server (localhost:8923 + 127.0.0.1) — stopped retrying per protocol.
  Pivoted to a Node WebSocket walker driving the REAL wire against a
  throwaway standalone server (fresh binary, bundle grep-verified):
  ALL 18 CHECKS PASSED — busy-hold, ack position, raw-vs-message
  divergence (cs stdout 3/4/5 byte-for-byte), mid-queue reattach
  re-sync, delivered-before-queue ordering, untagged-poke silence,
  execution order, 946ms idle fast path, 99/100 all-or-nothing
  straddle reject, cap stdout. Evidence archived in
  new-team-2/evidence/item-2/ (walker + transcripts, including the
  earlier run where 6 FAILs were walker fd-capture bugs, kept for
  honesty). First walker run's lesson: cs prints responses on stderr.
- Teardown: scoped pkill of the renamed /tmp/pqsrv only, workspace
  unregistered + deleted, my Chrome tab closed (@@Editor's untouched).
- Completion task cut: tasks/task-PromptQueue-Conductor-23.md (shas
  ca40ea6b + 86d50a25, gates, evidence, Chrome-permission blocker
  routed with recommendation to fold into @@Desktop's WKWebView pass).
- Holding for: badge clearance (Pane.svelte), B1 wave-3 reviews as
  they land, any reviewer findings on my two commits.

## 2026-06-13 — badge landed, wave-3 batch clean, task-21 gap-diffed

- Pokes processed (4 arrived mid-batch): task-21 smoke flags, badge
  release (dadd5e64), item-2 acceptance + Chrome-blocker decision (a),
  CtxPass 8/8 clean on ca40ea6b with N1/N2 notes. Sequencing note: the
  task-20 batch was already finished when the order poke crossed.
- Badge: 7c976a68 (Pane.svelte queue-pill + flipped-strip mirror fix +
  deferred wiring pin). Web gate green post-edit (svelte-check 0,
  vitest 177/1749, build). Item 2 now FULLY landed.
- N1 comment: b82a0a27 (registry-guard broadcast nuance pinned at
  enqueue_write). fmt/clippy/scoped tests green.
- B1 wave-3 (task-20): ALL FIVE CLEAN — 3a..3e field-by-field, all
  five specific flags refuted/verified; corroborating test run
  543+424+62 green. One observation routed: design § 3d
  "build_fs_graph forwards" sentence was never true (it's an
  independent whole-scope walk). One nano-nit: 3c commit prose says
  14 tests, it's 13 fns/17 call sites.
- Task-21: gap-diff in the report — flags 3/4/5 protocol+store halves
  already covered by the wire walker + vitest; flags 1/2/6 + visual
  halves folded into an UPDATED WKWebView checklist (supersedes
  task-23's).
- Report: tasks/task-PromptQueue-Conductor-28.md. Review queue empty;
  holding for wave-4a/4b routing.

## 2026-06-13 — B1 final batch (4a+4b): both clean; B1 review queue closed

- task-26 read after the badge had already landed (ordering satisfied
  by crossing, noted in the report).
- 4b watch item resolved with the mechanism, not just the claim: no
  await between the old preamble resolve and the new in-body resolve;
  OnceLock has no unset path, so the new read is equivalent-or-fresher
  in every interleaving. 7 None-sites → fresh empty cells (no timing
  dependence); load test sets its own cell and asserts it.
- 4a verified incl. target 6 first-hand (my own commits contain zero
  restart hunks); tri-state apply is untouched code outside all hunks;
  shorthand bindings make the Option<String> trio untransposable.
- Corroboration: the wave-3 test run (543+424+62) already contained
  both commits; scoped 37/37 rerun sat on 4a.
- Report: tasks/task-PromptQueue-Conductor-29.md. My B1 seat: 7/7
  commits clean. Holding.

## 2026-06-13 — task-28 ACCEPTED (crossing noted)

- @@Conductor accepted badge + N1 comment + wave-3 all-clean +
  task-21 gap-diff; flipped-pill catch noted; the § 3d design-doc
  correction was applied with attribution.
- His pointer to task-26 crossed my completion: 4a+4b were already
  reviewed clean (task-PromptQueue-Conductor-29, completion poke in
  his queue). No re-poke — it's queued; double-poking stacks.
- Status: lane fully landed, B1 seat 7/7 clean, review queue empty.
  Holding for round-close routing (WKWebView checklist is in
  task-28 §4 when @@Desktop's build is ready).

## 2026-06-13 — lane COMPLETE (Conductor's closing ack)

- 4a+4b batch accepted; B1 fully review-covered, zero findings across
  the 9 commits. Lane declared COMPLETE; WKWebView walk routed
  elsewhere (checklist handed off in task-28 §4). Holding for round
  close.
- Final lane ledger: item 2 end-to-end — ca40ea6b (server),
  86d50a25 (web), 7c976a68 (badge), b82a0a27 (N1 doc) — all
  cross-reviewed clean; wire recipe 18/18 with evidence archived in
  new-team-2/evidence/item-2/; reviews delivered: B1 waves 1, 2,
  3a-3e, 4a, 4b, all clean, 2 doc observations routed (both applied
  or acknowledged).
- Round-retro seeds (for the round report, per round-1 practice):
  HIGHLIGHT: the wire-level walker turned a blocked Chrome smoke into
  stronger protocol evidence + caught my own walker's fd bug honestly.
  LOWLIGHT: 6 walker FAILs shipped to the first evidence log because
  I asserted on stdout without checking which fd cs writes to —
  probe fd routing before scripting against CLI output. Also 3 pokes
  crossed mid-flight this round; the append-only bus absorbed every
  crossing cleanly (system worked as designed).
