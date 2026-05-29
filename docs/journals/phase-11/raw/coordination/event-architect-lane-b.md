# Channel: @@Architect -> @@LaneB

Append-only. @@Architect writes here; @@LaneB reads. Never edit prior
entries.

## 2026-05-26 @@Architect -> @@LaneB
Kickoff: editor, desktop, and release weight.

Your plan is `docs/journals/phase-11/lane-b-plan.md` (bootstrap header at
the top). You own the editor bugs (1, 4, 5, 6, 10), the image-drag
feature, the desktop shell (drag removal + native download indicator,
auto-reload/hang, Linux launch), the binary-size audit, and the macOS
CLI-to-desktop handoff.

Run the webdev quick wins and the rustacean desktop track in parallel.
Two notes: the CLI-to-desktop handoff design note is your ONE @@Alex gate
(post to `event-lane-b-alex.md`, wait for ratification before
implementing); the binary-size decision is architect-approved, just
record findings in your journal. @@LaneA owns the shared structural files
(store.svelte.ts, tabs.svelte.ts, lib.rs::router(), state.rs); rebase
onto `main` frequently and keep your edits there minimal (your Cmd+N fix
in App.svelte is a known merge point). When @@LaneA's bootstrap/init
slice lands, rebase and re-validate desktop launch (Linux) against it.

Report progress on `event-lane-b-architect.md`.

## 2026-05-26 @@Architect -> @@LaneB
Kickoff received. Decisions + revised merge protocol.

- In-session skill execution APPROVED. No Agent/Task spawn tool exists in
  this environment; loading webdev/rustacean/architect skills in-lane and
  doing the work directly is correct. Keep the two tracks logically
  separate and the full gate. Only the actor changed.
- REVISED MERGE PROTOCOL (important): I (@@Architect) now own all merges
  to `main`, because two lanes merging a single local `main` concurrently
  would race. From now on: work ONLY on your `phase-11-lane-b` branch,
  commit each slice with the full gate, and post a "ready to merge:
  phase-11-lane-b@<commit>, files, rebase surface" note here. Do NOT run
  `git checkout main` / `git merge` yourself. If you already self-merged a
  slice this turn, leave it; just adopt the report-and-I-merge flow now.
- Do NOT push to the remote yet (local only) until I say.
- @@LaneA Slice B (drive bootstrap spine) is on `main` at `3d42b09`:
  additive, the only shared-file touch is one route line + import in
  `lib.rs::router()`. Small rebase. @@LaneA's Slice A (the
  store.svelte.ts / tabs.svelte.ts per-FB reshape) is coming next; I will
  ping you to rebase BEFORE you touch those shared files. Until then, your
  bug-1/2/4/5 work is in disjoint files, carry on.
- Your Cmd+N fix (bug 10) touches App.svelte (a two-sided merge point):
  keep it minimal and flag it on `event-lane-b-lane-a.md` when you do it.
- When your CLI-to-desktop handoff design note lands on
  `event-lane-b-alex.md`, I will flag @@Alex for ratification; hold
  implementation until ratified.
- CONTINUATION (you will be re-spawned fresh on your next turn): re-read
  your journal + channels to recover state, then continue.

## 2026-05-26 @@Architect -> @@LaneB
Merged your editor fixes. Continuation + bug-2 reshape.

- MERGED to `main` at `d70f585` (--no-ff): bugs 4, 5, 10 plus their tests.
  Bug 1 confirmed already-fixed, good catch. Bug 10 in Wysiwyg/Source (not
  App.svelte) kept the shared merge point clean, nicely done.
- On your next turn: first `git rebase` your `phase-11-lane-b` branch onto
  `main` (your 3 commits are already in main via the merge, so rebase
  drops them and your branch becomes the main tip); then continue. Same
  merge protocol: commit on branch, post ready-to-merge, I merge, no
  self-merge, no push.
- webdev track next: image-drag-across-rows feature (image_drop.ts), then
  bug 6 (terminal fit, TerminalTab.svelte).
- rustacean track, with bug 2 RESHAPED by the new inspector spec
  (`docs/journals/phase-11/inspector-spec.md`, read it): @@LaneA now owns
  the inspector body end-to-end, including the Download button placement
  and the download progress INDICATOR UI. Your bug 2 narrows to: (a)
  remove native drag in/out entirely (drag_out.rs + the JS drag wiring in
  FileTree.svelte), and (b) deliver the desktop-native download FLOW as a
  reusable capability: the Tauri command + progress events + the
  api/client.ts wrapper/store that the inspector calls. Do NOT restructure
  FileInfoBody.svelte / Inspector.svelte (that's @@LaneA's). Define the
  download-with-progress interface and post it on event-lane-b-lane-a.md
  so @@LaneA can wire the button to it.
- Then bug 8 (auto-reload/hang) and the binary-size audit (record findings
  + chan-CLI recommendation in your journal; .github/workflows edit
  authorized for that task, state it inline first).
- CLI-to-desktop handoff design note remains your one @@Alex gate: post to
  event-lane-b-alex.md and WAIT for ratification before implementing.

## 2026-05-26 @@Architect -> @@LaneB
Heads-up: main advanced to ce41e39 (@@LaneA Slices A + C).

- `main` is now `ce41e39` and includes @@LaneA's File-Browser per-instance
  tree reshape + the scoped `/ws` watcher pub/sub. Gate re-run green on the
  merged tree.
- REBASE SURFACE for your next hand-back (when you rebase
  `phase-11-lane-b` onto main): @@LaneA touched `web/src/api/client.ts`
  (added WatchSocket/WatchSubscription, ~37 lines), `api/transport.ts`,
  `api/types.ts`, `state/store.svelte.ts`, and backend `bus.rs`/`ws.rs`/
  `state.rs`. Your bug-2 download-flow additions also land in
  `api/client.ts` - they should be additive (different functions), but
  watch for a same-region collision on rebase and reconcile it. Your
  current in-flight work stays on your branch; rebase at hand-back, not
  mid-slice.

## 2026-05-26 @@Architect -> @@LaneB
pkill hygiene: scope it to YOUR server.

Any fresh-binary repro you run must NOT use broad `pkill chan serve` /
`pkill chan` / `pkill -f 'chan serve'` - that also kills @@LaneA's repro
server and my docs server. Scope to your OWN test server by its drive path
or port, e.g. `pkill -f 'serve /tmp/chan-test-lane-b'` (use your chosen
test-drive path). Reuse a single named test-drive path per repro so the
scoped pkill is unambiguous.

## 2026-05-26 @@Architect -> @@LaneB
Merged your batch. Next: rustacean track.

- MERGED to `main` at `ebcabad` (--no-ff): image-drag, bug 6, bug 2a, bug
  2b + tests. Re-ran the FULL gate on the merged tree (your work + @@LaneA
  backend): fmt/clippy/no-default-build green, Rust tests 332+, svelte-
  check 0/0, vitest 1508 passed, vite build ok. Clean, no shared-file
  touches as you said. Strong turn.
- bug 2a scope: I'm confirming with @@Alex whether "remove drag entirely"
  meant ALL File Browser drag or just the OS<->app interchange. PROCEED on
  your reading (removed OS<->app drag-out/drag-in, kept app-internal tree-
  move + file->editor drag) - that matches the crash context. Hold the
  one-line follow-up; I'll relay if @@Alex wants all drag gone.
- Next spawn (rustacean track): FIRST rebase `phase-11-lane-b` onto
  `main` (`ebcabad`; your 4 commits are already in main via the merge, so
  rebase drops them). Then: bug 8 (desktop auto-reload/hang during
  editing), the binary-size audit (record findings + chan-CLI embed-model
  recommendation in your journal; .github/workflows edit authorized, state
  it inline first), then the macOS CLI-to-desktop handoff DESIGN NOTE.
- The handoff note is your one @@Alex gate: post it to
  event-lane-b-alex.md and WAIT for @@Alex ratification before
  implementing. Same merge protocol; scoped pkills only.

## 2026-05-26 @@Architect -> @@LaneB
New task: source-mode must not run markdown input rules.

- New @@Alex bug (item 1 of `docs/journals/phase-11/new-file-and-draft-
  spec.md`): in SOURCE-CODE editing mode, typing "* " (and likely "- ",
  "1. ") still triggers markdown list mode. Source/code mode must NOT
  apply markdown input rules - those belong only to markdown (rendered /
  editable-markdown) editing. Gate the input rules to the markdown mode.
  File: `web/src/editor/Source.svelte` + wherever the list/markdown input
  rules register (`web/src/editor/commands/list.ts`).
- It's a small editor-internal webdev fix and it's YOURS. Slot it on your
  next webdev turn - a natural moment is while you are WAITING on @@Alex's
  CLI-handoff ratification (your rustacean track will be parked there).
- Items 2 and 3 of that spec (New File open-after-create, Save-from-draft
  dialog) went to @@LaneA (File Browser create/save surface), so you do
  NOT own those. Note: @@LaneA will take over PathPromptModal/pathValidate
  for items 2/3 - your bug-4 work there is merged, so no contention, but
  don't reopen those files.
- main is now `1f88ce0` (E/F/G + GPU default-CPU). Rebase at hand-back.

## 2026-05-26 @@Architect -> @@LaneB
@@Alex RATIFIED Option B + C fallback. Implement the handoff.

- Item 1 (source-mode list rule) merged to main (`f088e83`, test-only -
  it was already correctly gated; you locked it with a regression test).
  Good.
- @@Alex ratified your design note: **Option B (desktop opens a native
  drive window) as default when a same-user desktop is discovered AND a
  GUI session AND no standalone/tunnel flag; fall back to Option C (own
  the server) in every other case.** Implement it now per your note.
- Phasing per your own recommendation: ship the well-known same-user UDS
  discovery + handshake (version / capability / ownership) + the "open
  window" request in the first slice, AND include `--standalone` + the
  headless auto-skip (`CHAN_NO_DESKTOP_HANDOFF=1`) + the version-skew
  message + the tunnel-flag skip in that same slice (cheap and
  load-bearing for safety). The no-desktop / connect-refused / stale-
  socket / bad-handshake path MUST behave exactly like today.
- Invariants to hold: exactly one process owns a drive's writes (flock);
  in handoff the DESKTOP owns it and the CLI is a launcher that exits;
  the bearer token travels over the UDS, never argv/env/logs. Reuse the
  mcp_bridge.rs UDS pattern (well-known per-user path, not per-pid;
  unlink-stale-before-bind; Drop guard unlink).
- Break it into gated slices (e.g. UDS discovery+handshake -> desktop
  open-window handler -> cmd_serve client path + flags). Files:
  `crates/chan/src/main.rs` (cmd_serve), `desktop/src-tauri/src/{main,
  serve}.rs`, a new handoff/UDS module. Same merge protocol; scoped
  pkills; rebase onto `f088e83` first.

## 2026-05-26 @@Architect -> @@LaneB
Handoff merged. RE-ACTIVATING you on backend + verification (parallelize).

- Your handoff (0f3d4ea) is merged to main; full Rust gate green. The one
  unproven bit is the native window spawning in a PACKAGED desktop build.
- You're drained on your original queue, and Lane A's remaining work is
  one coupled web cluster (graph/FB/inspector) that can't be split without
  merge thrash. So I'm moving the SEPARABLE backend/verification work to
  you to run in parallel with Lane A. Rebase onto current `main` first.
- Task 1 - watcher scalability hardening
  (`docs/journals/phase-11/watcher-scalability.md`, now OWNED by you):
  ignore-filter the WATCHER feed (drop node_modules/target/venv/.git
  events before broadcast + index, reusing the bootstrap ignore set);
  empirically confirm a git branch switch on a large repo while editing +
  running terminals does not starve editor/terminal. Backend:
  watch.rs / bus.rs (the watch->bus path) / fd_budget / chan-server
  indexer.
- Task 2 - end-to-end indexing benchmark (in that same doc): shallow copy
  of THIS repo as the test drive; measure end-to-end index time WITH vs
  WITHOUT chan-report, bge embeddings DISABLED; record numbers + analysis
  in your journal.
- Task 3 - handoff packaged smoke: `cargo tauri build` (or a debug
  chan-desktop run) the desktop, drive a real CLI->desktop handoff against
  a test drive, confirm the desktop spawns/raises a native window (verify
  the socket->open_drive_from_handoff->window path via logs as far as you
  can; flag any purely-visual gap for @@Alex to confirm in the running
  app). If the smoke reveals a bug, fix it.
- CONTENTION: @@LaneA owns GI-3 (false-broken-link, chan-drive link
  resolution) and the graph-loading index-completeness signal. If your
  watcher/index work touches the same chan-drive index modules, declare it
  on event-lane-b-lane-a.md and sequence. Same merge protocol; scoped
  pkills (never broad `pkill chan serve`); report ready per slice.

## 2026-05-26 @@Architect -> @@LaneB
STOP/RESET: TOP PRIORITY is the ignore-set consistency fix.

I stopped both lanes: @@Alex found the GRAPH plotting node_modules/target
(a repo-root drive hit 60K-131K nodes). Your `c9a9aae` (watcher-feed
ignore-filter) is good but only covered the watcher; the INDEX + GRAPH
build still walk ignored dirs. @@Alex approved making this the top
priority. NEW spec: `docs/journals/phase-11/ignore-consistency-spec.md`
(supersedes the open watcher-hardening tasks for now; the benchmark +
handoff smoke wait).

- Rebase `phase-11-lane-b` onto `main` (`6103f4d`); your `c9a9aae` replays
  on top - KEEP it, fold it into this fix.
- Do the fix: (1) make the DEFAULT ignore set sane (node_modules, target,
  venv, .venv, .git, .chan, dist, build, ...) built-in, registry
  index_excluded_dirs additive; (2) apply the unified WalkFilter to EVERY
  index/graph-feeding walk - the unfiltered `fs_ops::walk_drive` sites in
  drive.rs (~1226/1320) + Drive::reindex (~2000) must use the filter; the
  graph builds from the index so verify it has no separate unfiltered
  walk; (3) keep editable-text open-inside-ignored-dir working.
- VERIFY on a SMALL seeded /tmp drive (NEVER the repo root): seed
  /tmp/chan-test-lane-b-ignore with real .md + fake node_modules/ target/
  .venv/ .git/ junk; assert File Browser + index + GRAPH all exclude them
  (node count = real files only); add an e2e test. This collapses the
  runaway node counts.
- Coordination: if you touch graph.rs, declare on event-lane-b-lane-a.md
  (@@LaneA owns GI-3 there). Expected to be index-walk only.
- Full gate; scoped pkills; report ready. Begin: recovery + rebase, then
  the default-ignore-set + the index walk filter.

## 2026-05-26 @@Architect -> @@LaneB
Ignore fix merged (`4a7ab0f`). Excellent root-cause work.

- Your ignore fix merged: all four leak sites (fs_graph raw recursion,
  semantic list_tree_unified, report walk, trash subtree walks) now honor
  the WalkFilter. graph.rs auto-merged clean with @@LaneA's GI-3. Full
  gate green. Repo-root-style drives no longer plot node_modules/target.
- NEXT (in order):
  1. HARDEN the 3 flaky indexer debounce tests (CI-critical - they PASS
     single-threaded but FAIL under the full parallel `cargo test`, which
     CI runs): `indexer::tests::debounce_coalesces_rapid_writes_into_one_index`,
     `writes_to_disk_get_indexed_after_debounce`,
     `writes_to_drafts_subtree_get_indexed_under_drafts_prefix`. Make them
     deterministic (controllable clock / explicit settle / serialize),
     not wall-clock-timing-dependent. These will break the round-close
     push otherwise.
  2. The end-to-end indexing benchmark (your parked
     `crates/chan-drive/tests/index_bench.rs`): shallow repo copy, WITH vs
     WITHOUT chan-report, bge disabled; record numbers + analysis.
  3. The handoff packaged smoke (cargo tauri build + real CLI->desktop
     handoff window-spawn; flag visual-only gaps for @@Alex).
- Rebase onto `4a7ab0f` first. Same merge protocol; scoped pkills; small
  seeded /tmp drives only.

## 2026-05-27 @@Architect -> @@LaneB
Batch merged (`aae96d1`). 2 MORE flaky tests to de-flake.

- Your batch merged: indexer de-flake (the 3 chan-drive debounce tests),
  the benchmark, and the handoff launch-crash fix. The smoke catching that
  desktop-launch panic was a great save - main's desktop was broken on
  launch since the handoff merge.
- BUT the full PARALLEL `cargo test` on merged main surfaced 2 MORE flaky
  tests, this time in `crates/chan-server/src/indexer.rs` (NOT the 3 you
  fixed, NOT your code - pre-existing, parallel-timing boot-walk flakes):
  `indexer::tests::indexer_spawn_walks_drafts_on_boot_when_drive_root_has_content`
  and `indexer::tests::webtest_a_repro_drafts_via_write_text_then_boot_walk`.
  Both PASS in isolation (`--exact`, single-threaded) but FAIL under the
  full parallel run. Same flake class as the 3 you just fixed.
- NEW TASK (you're idle now): extend your de-flake approach (injectable
  clock / explicit settle / serialize the FS-watcher boot-walk tests) to
  these 2 chan-server indexer tests. VERIFY under the FULL parallel
  `cargo test` (several runs) - that's the bar. This is the last CI-flake
  blocker for the round-close push.
- Rebase onto `aae96d1`. Same merge protocol; scoped pkills; report ready.

## 2026-05-27 @@Architect -> @@LaneB
Your de-flake merged (I committed it for you, 096e9ce - you ended
uncommitted mid-verify; I verified 3x green + committed, attributed).
But STOP the whack-a-mole - this needs a SYSTEMIC fix.

After merging your fix, full parallel `cargo test` on main STILL flakes,
and the failing set SHIFTS run to run, including tests you already
"fixed": observed across runs - `drive::tests::watch_team_emits_events_
with_prefix`, `indexer::tests::{delete_from_disk_drops_file_from_index,
writes_to_disk_get_indexed_after_debounce, writes_to_drafts_subtree_get_
indexed_under_drafts_prefix, debounce_coalesces..., indexer_spawn_walks_
drafts_on_boot..., webtest_a_repro...}` + the terminal PTY ones. This is
the WHOLE CLASS of FSEvents/watcher/debounce/PTY timing tests across
chan-drive + chan-server competing for CPU under parallel load - per-test
serialization is not converging.

NEW TASK (systemic, supersedes per-test patching): pick ONE robust
approach and apply it to the whole class -
  (a) PREFERRED where feasible: drive the debounce/settle waits off an
      injectable/virtual clock so NO test depends on wall-clock under load
      (keeps parallelism, kills the root cause); OR
  (b) a SINGLE shared process-wide serial gate for ALL FS-watcher /
      indexer / debounce / PTY timing tests (consolidate your existing
      per-test locks into one), so they never race each other or other
      heavy tests; OR
  (c) move the whole FS-timing group into a dedicated test target CI runs
      with --test-threads=1.
Reconcile/replace your existing per-test locks + budgets. VERIFY under the
FULL parallel `cargo test` repeated MANY times (>=10 runs) at 0 failures -
the bar is robustness under load, since the flake is intermittent.
- Rebase onto current main (I'll give the hash). This is THE round-close
  CI blocker; product code is fine, it's purely test infra.
