# @@Architect journal: phase 11

Orchestration log for the phase-11 two-lane dispatch. Append-only.

## 2026-05-26: dispatch set up

Read phase-11 round 1 (bugs + features) and round 2 (phase-10 carryover).
Swept the code to validate lane boundaries and conflict surfaces. Split
the work into two parallel lanes, each run by an architect that spawns its
own webdev + rustacean subagents:

- @@LaneA: drive streaming spine (bootstrap/pre-flight, per-directory
  watcher pub/sub, paced jobs, File Browser, Graph) plus bugs 7 and 9
  (they live in this domain). Plan: `../lane-a-plan.md`.
- @@LaneB: editor bugs (1, 4, 5, 6, 10), image-drag feature, desktop
  (drag removal + download indicator, auto-reload, Linux launch),
  binary-size audit, macOS CLI-to-desktop handoff. Plan:
  `../lane-b-plan.md`.

Decisions ratified by @@Alex:
1. Lane cut as above (FB + Graph + partial-load stay together because
   Graph reuses FB's pub/sub).
2. Partial-load core-first; chunked/resumable transfers + full async
   audit deferred.
3. One git worktree per lane (`phase-11-lane-a`, `phase-11-lane-b`);
   conflicts reconcile at merge.
4. Only the CLI-to-desktop handoff is an @@Alex design gate. Spine
   protocol and binary-size decision are architect-approved.

Coordination: append-only directional channels under
`../coordination/`. Lanes report to me via `event-lane-{a,b}-architect.md`;
I direct via `event-architect-lane-{a,b}.md`; cross-lane via
`event-lane-a-lane-b.md` / `event-lane-b-lane-a.md`; @@Alex escalations
via `event-lane-{a,b}-alex.md`.

Merge cadence: small frequent slices to `main`. @@LaneA owns the
structural shape of the shared files (store.svelte.ts, tabs.svelte.ts,
lib.rs::router(), state.rs) and lands them early; @@LaneB rebases. The
integration seam is @@LaneA's bootstrap/init change vs @@LaneB's desktop
launch re-validation.

Next: @@Alex launches the two lane-architect sessions from the bootstrap
headers. I watch the channels and act on reports.

## 2026-05-26: course corrections + launch

- @@Alex: Linux desktop launch is DEFERRED to a later run on a Linux
  machine; it cannot be validated in this environment. Pulled from
  @@LaneB active scope in `lane-b-plan.md` (item 9 marked deferred,
  context/seam/verification updated). @@LaneB now: editor bugs +
  image-drag + desktop drag-removal/download-indicator + auto-reload +
  binary-size + macOS CLI handoff.
- Briefly launched then stopped both lanes during a back-and-forth over
  who launches the sessions; both were killed in their reading phase
  (no worktrees/branches created, tree clean). @@Alex confirmed I manage
  the lanes myself. Re-launching @@LaneA and @@LaneB as background agents
  off `main` (HEAD 198beb9).

## 2026-05-26: lanes running; merge protocol revised

- Lane A turn 1 done: spine contract written + Slice B (drive bootstrap
  spine) merged to main `3d42b09`. Lane B still running on its branch
  (`phase-11-lane-b` at baseline, no merge yet).
- Two reality checks from the lanes:
  1. No Agent/Task spawn tool inside spawned agents. Lanes load skills
     in-session and do slices directly. APPROVED; deliverables/gates
     unchanged. (Update the bootstrap headers' "spawn subagents" wording
     next round to "load skills in-session".)
  2. No SendMessage tool here either. Continuation = re-spawn a FRESH
     lane agent that recovers state from its journal + channels. The
     self-documenting journal design makes this clean.
- MERGE OWNERSHIP: I (@@Architect) now own all merges to `main`. Lanes
  merge by cd-ing into the main checkout to `git merge`; two doing that
  concurrently races. So lanes stay on their branch, report
  "ready to merge: branch@commit", and I serialize the merges. Communic-
  ated on both event-architect-lane-* channels. No remote push yet.
- D2 (Lane A): approved keeping global `watch` frame + adding scoped `fs`
  frame.
- Re-spawning Lane A to continue Slice A (web tree per-FB reshape, the
  file @@LaneB waits on) then Slice C (scoped pub/sub + hardening test).

## 2026-05-26: Lane B turn 1 merged; inspector task captured

- Merged Lane B editor fixes to `main` `d70f585` (--no-ff): bug 4
  (trailing-slash), bug 5 (image paste), bug 10 (Cmd+N focus, fixed in
  Wysiwyg/Source not App.svelte so no shared-file contention), + tests.
  Bug 1 was already fixed at HEAD. Lane A still running Slice C.
- New @@Alex task: inspector consistency + layout redesign across FB /
  editor / Graph. Captured in `inspector-spec.md` (recovered the prior
  phase-7/phase-10 spec, identified the phase-7 fullstack-a-33 drift).
  OWNERSHIP: Lane A owns the inspector end-to-end; Lane B bug 2 narrows
  to drag-removal + the native download-flow capability that Lane A's
  inspector calls. Queued after Lane A's FB/Graph slices.
- Re-spawning Lane B: rebase onto d70f585, then image-drag + bug 6
  (webdev) and the reshaped bug 2 + bug 8 + binary-size (rustacean).
- Dogfood server: `chan serve --here docs` on :8790 for @@Alex to read
  the coordination docs inside Chan. Survives via nohup+disown; the
  bg-task wrapper SIGHUPs it, so do not launch long-lived servers through
  run_in_background.

## 2026-05-26: Lane A Slices A+C merged; integration gate green

- Merged `phase-11-lane-a` (Slice A `5c97410`, Slice C `ac21cd2`) to main
  `ce41e39`. Two-way merge with d70f585 (Lane B); file sets disjoint, ort
  clean. Re-ran FULL gate on merged main (combined tree never gated
  together): fmt/clippy/no-default-build/all-tests/svelte-check 0-0/vite
  build all green. main is a validated integration point.
- Flagged @@LaneB: api/client.ts is the rebase overlap (Lane A WatchSocket
  vs Lane B bug-2 download-flow); reconcile at its next rebase.
- Re-spawning Lane A for Slice D (paced jobs under fd_budget + bugs 7/9),
  after it rebases its branch onto ce41e39.
- Standing merge order working well: lanes on branches, I serialize merges
  + re-run the gate on merged main. No races.

## 2026-05-26: Lane A Slice D merged (bugs 7+9)

- Merged `phase-11-lane-a` Slice D (`07f0a7c`) to main `1918992`. Linear
  merge (main == base ce41e39), re-gate skipped as redundant. bug 7
  (fd exhaustion / autosave hang) fixed via fd_budget::pace_reindex_worker
  back-pressure, validated 40/40 autosaves at ulimit 256 + 2 terminals +
  rebuild. bug 9 (stuck reindex pill) fixed in the indexer status
  transitions (no state.rs/AppStatusBar change needed).
- Re-spawned Lane A for Slice E (FB /ws fs-frame wiring), then F (Graph)
  and G (progress widgets); inspector feature queued after.
- Lane B still running (image-drag + bug 6 done at 0a8e0ae; on bug 2 /
  bug 8 / binary-size).

## 2026-05-26: Lane B batch merged (bugs 6, 2a, 2b, image-drag)

- Merged `phase-11-lane-b` (b70f4ac image-drag, 0a8e0ae bug 6, 3fec962
  bug 2a drag removal, 66dec92 bug 2b download capability) to main
  `ebcabad`. Two-way merge with Lane A backend; clean. Full gate re-run
  green (332+ Rust tests, svelte-check 0/0, vitest 1508). 
- Cross-lane overlap to watch: FileTree.svelte changed by both Lane B
  (bug 2a, merged) and Lane A (Slice E, in flight). Flagged Lane A to
  reconcile at its Slice-E rebase onto ebcabad.
- Open @@Alex decisions: (1) bug 2a scope - did "remove drag entirely"
  mean all FB drag or just OS<->app? Lane B kept app-internal drag;
  proceeding on that reading. (2) GPU/Metal embed-hang triage (still
  open from prior turn). Both surfaced to Alex.
- Re-spawned Lane B for the rustacean track: bug 8, binary-size audit,
  then the CLI-to-desktop handoff design note (the @@Alex gate).

## 2026-05-26: @@Alex decisions on the two small calls

- bug 2a scope: CONFIRMED correct - remove only OS<->app drag, keep
  app-internal drag. AND @@Alex added a new feature: standard File Browser
  capabilities (multi-select via mouse rubber-band + shift/cmd-click +
  shift+arrows; cmd+C/X/V clipboard; mouse DnD to move one or many).
  Captured in `fb-capabilities-spec.md`, assigned to @@LaneA, queued after
  Slice E. Large; @@LaneA to sub-slice it.
- GPU/Metal embed hang: @@Alex chose to DISABLE the GPU path by default +
  file a follow-up bug. Dispatched a focused one-off agent on branch
  `phase-11-gpu-embed-default` to flip the default (GPU opt-in) and write
  `gpu-embed-followup.md`. I merge when it reports.

## 2026-05-26: E/F/G + GPU merged; watcher analysis; new tasks filed

- Merged Lane A Slices E/F/G to main `7998b4e` (FB watcher wiring via new
  fbWatch.svelte.ts + FileBrowserSurface - avoided the FileTree conflict;
  Graph gradual-load + edge coloring; progress widgets). One FLAKY vitest
  run (3 fails) that passed clean on isolated re-run; flagged Lane A to
  harden timing-sensitive new tests. Then merged the GPU default-CPU fix
  (`phase-11-gpu-embed-default` 044c23f) -> main `1f88ce0`. All gated.
- @@Alex watcher concern (expand-all / graph-from-here -> too many
  watchers -> max fd?): ANSWERED grounded in code. Design already uses
  ONE recursive OS watcher (watch.rs RecursiveMode::Recursive) + LOGICAL
  refcounted scope filters (bus.rs ScopeRegistry); UI actions add zero OS
  watchers. Real residual risks: event-storm volume (git) + Linux inotify
  watch-count from the recursive root. Analysis + hardening task in
  `watcher-scalability.md` - HELD pending @@Alex alignment.
- Filed new @@Alex tasks (`new-file-and-draft-spec.md`): item 1 source-
  mode-no-md-input-rules -> @@LaneB; items 2 (New File open-after-create)
  + 3 (Save-from-draft dialog parity, dir-only) -> @@LaneA.
- Re-spawning Lane A for the inspector feature (its planned next).

## 2026-05-26: Lane B bug8+binary-size merged; CLI note posted; new @@Alex tasks

- Merged Lane B (bug 8 desktop auto-reload watcher-scoping + bootstrap
  retry; binary-size audit = CI already lean 28MB, Makefile re-pointed off
  embed-model) to main `250d2f6`. store.svelte.ts auto-merged with Lane
  A's Slice E (different regions). Full gate green (29 Rust suites,
  vitest 1537, svelte-check 0/0). Round-1 bug list now essentially clear.
- CLI-to-desktop handoff DESIGN NOTE posted to event-lane-b-alex.md
  (recommends Option B = desktop opens a drive window, with Option C =
  own-server fallback; UDS discovery via mcp_bridge pattern). AWAITING
  @@Alex ratification - this is the gate. Surfaced to @@Alex.
- @@Alex agreed with the watcher analysis -> RELEASED watcher hardening to
  Lane A, and ADDED an e2e indexing benchmark (shallow repo copy, with vs
  without chan-report, bge disabled).
- New @@Alex task: graph dead-ends / loading-state (graph-loading-state-
  spec.md) -> Lane A. Investigate ghost nodes + show parent-dir loading
  state instead of inaccurate dead-ends.
- Re-spawned Lane B for item 1 (source-mode list rule) while the handoff
  awaits ratification; Lane A continues the inspector.

## 2026-05-26: CLI handoff ratified; item 1 merged

- @@Alex RATIFIED Option B + C fallback for the CLI-to-desktop handoff.
  Released the implementation to Lane B (UDS discovery + handshake +
  open-window request + --standalone/headless-skip/version-skew/tunnel-
  skip in slice 1; C fallback for no-desktop). The one design gate is now
  cleared.
- Lane B item 1 (source-mode list rule) merged `f088e83` (test-only; was
  already correctly gated). Re-spawned Lane B to implement the handoff.
- Lane A still on the inspector.

## 2026-05-26: doc-commit timing confirmed

- @@Alex confirmed: keep the phase-11 coordination docs (plans, journals,
  channels, specs) UNTRACKED/dirty during the round (they're the live
  bus); commit the whole `docs/journals/phase-11/` tree to main as one
  `docs(phase-11): ...` commit at ROUND CLOSE. Not as-we-go. No periodic
  snapshots for now. >>> ROUND-CLOSE TODO: commit the tree. <<<

## 2026-05-26: inspector merged

- Lane A inspector (I1-I4) merged to main `cc17a37`: consistent body
  across FB/editor/Graph, actions-section-under-filename layout, retired
  DirectoryInfoBody (FB/Graph folder parity), Graph-from-here on file+
  folder nodes, Open any editable (read-only incl.), Download via Lane B's
  progress capability. Web gate green (vitest 1541). Re-spawned Lane A for
  new-file items 2/3 (reuses the I4 editable-open). Queue after: FB caps,
  graph dead-ends, watcher hardening + benchmark. Lane B on the handoff.

## 2026-05-26: handoff merged; graph/inspector hotfix filed

- Merged Lane B CLI-to-desktop handoff (0f3d4ea) to main: handoff.rs
  shared module + cmd_serve client + desktop listener (Option B + C
  fallback). Clean merge (additive), full Rust gate green (29 suites,
  clippy incl chan-desktop). Remaining verification: packaged Tauri
  window-spawn smoke (Lane B verified the listener via probes + 9 cases;
  the real packaged window-spawn is unproven - best verified by @@Alex in
  a fresh desktop build, or a later Lane B build-smoke).
- @@Alex live-testing the inspector on docs/ found a hotfix batch
  (graph-inspector-bugs.md), filed URGENT to Lane A (immediate next after
  new-file items 2/3, before FB-caps):
  - GI-1 inspector Open reloads graph instead of opening editor.
  - GI-2 inspector Show File reloads graph instead of FB reveal+select.
  - GI-3 FALSE broken-link: docs/journals/phase-2/frontend-3.md EXISTS but
    graph labels it "does not exist". Relative-link existence-check base
    mismatch. Real bug (confirms @@Alex's distrust of the ghost nodes).
  - GI-4 directory nodes slightly bigger for clickability.
- Will rebuild + restart @@Alex's :8791 test server after merging the fix.

## 2026-05-26: re-balanced to two parallel lanes

- Lane A was the long pole (web graph/FB/inspector cluster, can't split
  without thrash); Lane B drained after the handoff. Re-activated Lane B
  on the SEPARABLE backend/verification work to parallelize:
  - Lane B: watcher feed ignore-filter + git-storm resilience check,
    end-to-end indexing benchmark (with/without chan-report, no bge),
    handoff packaged smoke. (watcher-scalability.md ownership -> Lane B.)
  - Lane A: new-file 2/3 -> graph/inspector hotfix (GI-1..4) -> FB caps ->
    graph-loading UX (+ GI-3 link resolution).
- Contention boundary: chan-drive index/link modules (Lane A GI-3 +
  graph-loading completeness signal vs Lane B watcher/index/benchmark) -
  both told to declare touches on the cross-lane channel.

## 2026-05-26: HALT - ignore rules not applied to index/graph

- @@Alex hit a graph plotting node_modules/target (60K-131K nodes).
  Stopped both lanes; killed the runaway server (PID 33064 =
  `chan serve <repo-root> --port 8799`, served by the Lane A hotfix agent
  as its graph test drive - a test-discipline violation; should have been
  a small /tmp drive). Only the safe docs server (8791) remains.
- No work lost: Lane A branch == main (committed nothing this turn).
  Lane B branch has `c9a9aae` (watcher-feed WalkFilter ignore-filter) -
  partial, kept on branch.
- DIAGNOSIS: the unified ignore set (WalkFilter, built from registry
  `index_excluded_dirs` in library.rs) IS plumbed + applied to bootstrap
  (File Browser) + Library reindex + now the watcher feed (c9a9aae). But
  it FAILED for the repo-root drive -> graph/index plotted node_modules/
  target. Candidate causes: (a) default `index_excluded_dirs` may not
  include node_modules/target/venv out of the box; (b) an index/graph
  walk site bypasses the filter (drive.rs uses unfiltered
  `fs_ops::walk_drive` at ~1226/1320; graph build's file source). Round-1
  required ONE consistent ignore set across chan-desktop + chan serve;
  this is the gap.
- FIX (the real task, pending @@Alex go): make the unified ignore set
  default-sane (node_modules, target, venv, .git, ...) and applied
  CONSISTENTLY across bootstrap + index/reindex + graph build + watcher.
  Fold in c9a9aae. Backend -> @@LaneB likely. Also tighten lane
  test-server discipline: never serve the repo root / a node_modules-
  bearing tree; use small seeded /tmp drives.
- HOLDING both lanes per @@Alex's stop; awaiting direction.

## 2026-05-26: GI hotfix ready (held); awaiting ignore fix

- Lane A GI hotfix done: `phase-11-lane-a@d35b852` (web 7299625 =
  GI-1/GI-2 + GI-4; backend graph.rs d35b852 = GI-3). GI-1/2 root cause
  was REACTIVE OVER-TRACKING (reload $effect re-fired on currentScope
  recompute that Open/Show File trigger), fixed by stable
  scopeId|depth|mode key + untracked load - NOT a mis-bound handler.
  GI-3 = wiki-link ancestor-walk resolution (real partial-prefix links
  resolve, broken stay flagged). Gate green.
- HOLDING Lane A's merge: it touched graph.rs (GI-3), and Lane B's ignore
  fix (still running) may also touch graph.rs. Per @@Alex's order, merge
  Lane B's ignore fix FIRST, then Lane A on top (reconcile graph.rs if
  both touched), then ONE :8791 rebuild for @@Alex re-verify.
- Lane A flagged 3 chan-drive indexer debounce-test FLAKES (pass single-
  threaded; @@LaneB's area) - have @@LaneB harden them. Same flake class
  seen earlier; re-run single-threaded when gating merged main.
- Lane A idle until I merge GI hotfix, then re-spawn for FB capabilities.

## 2026-05-26: both fixes merged; server rebuilt; flaky tests tracked

- Merged Lane B ignore fix (e7b7824 + watcher b43ddeb) -> main 5e288ca,
  then Lane A GI hotfix (d35b852) -> main 4a7ab0f. graph.rs auto-merged
  clean (GI-3 + ignore filter in different functions). Full gate green
  modulo flakes.
- FLAKY TESTS (pass isolated, fail under full parallel run; CI risk before
  push): 3 Rust indexer debounce (debounce_coalesces..., writes_to_disk...,
  writes_to_drafts...) -> Lane B to harden; 3 web (EmptyPaneCarousel,
  Pane, TerminalTab) -> Lane A to harden. Both queued.
- Rebuilt + restarted @@Alex's docs server on :8792 (was 8791) with both
  fixes for live re-verify of GI-1..4.
- Re-spawned: Lane A -> FB capabilities (+ harden 3 web flakes);
  Lane B -> harden 3 indexer flakes -> indexing benchmark -> handoff smoke.

## 2026-05-27: Lane B batch merged; 2 more flaky tests found

- Merged Lane B batch -> main `aae96d1`: 3-debounce-test de-flake (34e3e23),
  indexing benchmark (3f2aa57; structural ~2-2.7s, chan-report ~doubles
  E2E), and the handoff LAUNCH-CRASH fix (fba85d8 - the handoff listener
  panicked outside the Tauri tokio runtime, aborting every desktop launch;
  main's desktop was broken-on-launch since the handoff merge). Smoke
  earned its keep. fmt/clippy/build green.
- Full PARALLEL cargo test surfaced 2 MORE flaky tests in chan-server
  indexer.rs (boot-walk: indexer_spawn_walks_drafts_on_boot...,
  webtest_a_repro_drafts_via_write_text_then_boot_walk). Confirmed FLAKE
  (pass isolated --exact, fail under full parallel) - pre-existing, not
  Lane B's code. Re-tasked Lane B (idle) to de-flake these 2. Last CI
  flake blocker before round-close push.
- No server rebuild (all backend/test/desktop; docs server unaffected).
  Handoff window-paint is a visual gap for @@Alex in a real desktop build.
- Lane A still running FB-capabilities.

## 2026-05-27: FB-capabilities merged

- Merged Lane A FB-caps -> main `b458ef6`: multi-select (FB1), clipboard
  C/X/V + multi-drag-move (FB2/3), Drive::copy + /api/fs/transfer (FB4,
  copy didn't exist before), Finder-style " copy" non-overwrite collision
  policy. Also the 3 web flake fixes (root cause was 30s import-contention
  timeouts at per-test await import(), fixed via static top-level imports).
  Gate: web vitest 1582/0, svelte-check 0/0; Rust only the 2 known
  chan-server boot-walk flakes (Lane B fixing). 
- Re-spawned Lane A for GI-5/6/7 (dir Show Directory / Graph-from-here /
  depth slider). Holding @@Alex server rebuild until GI-5/6/7 lands ->
  rebuild FB-caps + dir-fixes together; give @@Alex a SAFE scratch drive
  for FB-caps mutation testing (NOT docs/ - the coordination bus lives
  there and copy/move must not touch it).
- Lane B still de-flaking the 2 chan-server boot-walk tests.

## 2026-05-27: all flakes fixed; main green under parallel

- Lane B de-flaked the 2 chan-server boot-walk tests AND found+fixed the
  same flake class in 4 terminal.rs real-PTY tests (serialize behind a
  process-wide tokio Mutex + 30s early-returning polling budgets). It
  ended mid-verification UNCOMMITTED; I verified 3x full-parallel green in
  its worktree, committed it on its branch myself (096e9ce, fmt/clippy
  clean, attributed), merged -> main `b81636e`.
- main now FULLY GREEN under full parallel cargo test (0 fail) + web
  vitest 1582/0. All flaky tests across the round are fixed. CI-ready for
  round-close push.
- Lane A on GI-5/6/7 (dir-inspector + depth). Holding @@Alex :8792 rebuild
  until GI-5/6/7 lands (FB-caps + dir-fixes together) + will give a safe
  scratch drive for FB-caps mutation testing.
- Lane B DRAINED (de-flake + benchmark + smoke all done). Parked; Linux
  desktop + manual/site copy remain deferred (manual copy waits for the
  graph work to settle).

## 2026-05-27: GI-5/6/7 merged; FS-test flakiness is SYSTEMIC

- Merged Lane A GI-5/6/7 (8906d07) -> main `dc7dbfb` (web-only):
  Show Directory opens FB at the dir (enter:true); Graph-from-here on a
  dir re-roots to the dir itself (was misapplying the file parent rule ->
  blank); depth slider probes the dir's real reachable depth (dir:journals
  = 3). Web gate green (vitest 1593).
- CORRECTION to my earlier "main fully green": NOT reliable. Full parallel
  cargo test flakes intermittently - run 1 had 4 failures, run 2 had 2
  DIFFERENT ones, including tests Lane B already de-flaked. The whole class
  of FSEvents/watcher/debounce/PTY timing tests flakes under parallel CPU
  load; per-test serialization is not converging. Re-tasked Lane B with a
  SYSTEMIC fix (injectable clock preferred / single shared serial gate /
  dedicated single-threaded test target), verify >=10 parallel runs.
  Product code is fine; purely test infra. THIS is the round-close CI
  blocker now.
- Rebuilt @@Alex server on :8793 (was 8792) with FB-caps + GI-5/6/7 for
  combined re-verify (graph dir-inspector + depth + FB-caps). Warn @@Alex:
  FB-caps copy/move mutates files - avoid journals/phase-11/coordination
  (the live bus); offer a dedicated scratch drive if preferred.

## Candidate bugs / future follow-ups
- GPU/Metal embed hang: being defaulted-off now; proper fix (timeout + CPU
  fallback or correct Metal command-buffer usage) deferred to the
  follow-up note `gpu-embed-followup.md`.

## Open watch items
- Integration seam: @@LaneA bootstrap/init merge -> @@LaneB desktop
  re-validation (Linux).
- App.svelte two-sided merge point (Cmd+N from B, overlay/status from A).
- CLI-handoff design note pending @@Alex ratification before @@LaneB
  implements.
