# v0.59.0-rc1: rolling release journal

Working journal for the v0.59.0 cycle. Unlike the per-release notes above, this is a rolling doc: appended to as each work stream from `dev/v0.59.0/request.md` lands on its branch, and reconciled into a final `release-0.59.0.md` at cut time. As of this entry the `devserver-cmd`, `graph-tuning`, and `index-dashboard` streams are merged onto `main`; the editor, chan-desktop, and UX streams are still in flight, and the `v0.59.0-rc1` tag waits for all of them. Each section stands alone so the release summary can be assembled from these entries.

## Work streams (from `dev/v0.59.0/request.md`)

- [x] **`chan devserver` command**: reshape `--service` into explicit action verbs (branch `devserver-cmd`)
- [x] Graph: focus-on-select grey-out with first-order edge focus, deeper fs-graph, live force tuner (branch `graph-tuning`)
  - [ ] Carryover, tracked in `dev/v0.59.0/graph-remaining-items.md`: auto-select root on open, restore the "data being indexed, hang tight..." empty-state message, `@@mention` "Graph from here" missing edges
- [x] Index & dashboard: clickable indexing notification opening a paused Dashboard Indexing slide, per-path indexing pulse, no reload on tab switch (branch `index-dashboard`)
- [ ] Editor: directory-link click to file browser, list continuation glyphs, enumerated-list indent, `mermaid-to-excalidraw`
- [ ] Chan desktop: second-monitor hide/show window shrink, window-title glyphs
- [ ] UX: friendlier `cs open` from standalone, unblock `cs download`/`upload` in workspaces

---

## `chan devserver` command: explicit action verbs

**Branch:** `devserver-cmd` (worktree `../chan-devserver-cmd`, off `origin/main`). Not merged. **Status:** complete, gated green, empirically verified end-to-end on all reachable backends.

### The request (verbatim intent)

From `dev/v0.59.0/request.md`, "The `chan devserver` command". The starting behavior of `--service` auto-picks a backend (none / chan on Windows / systemd on Linux / launchd on macOS) and does one overloaded thing: create-or-update the service, restart if flags (port/bind) changed, then monitor `/healthz` to stay blocking (so it can front a tunnel). systemd additionally sets user linger, uses the fdstore to preserve PTYs across restarts, enables on boot, and `--stop` should stop and disable.

What the maintainer wanted:

- `chan devserver --service=none`: the default, `--bind`/`--port`, run in foreground.
- All other modes support `--start` (background), `--stop`, `--status`, `--restart`.
- The default "start-or-restart-if-flags-changed, then attach/block" becomes `--join`.
- If unix-domain sockets are not supported yet, add `--bind={path}` to switch to AF_UNIX and ignore/reject `--port`. The point: "not listen on a port and still make it work" (open to suggestions).

### Decisions (agreed with the maintainer up front)

1. **Defer Unix-domain sockets.** Reason surfaced during exploration: axum 0.7.9's `serve` is hardcoded to `TcpListener` (no generic `Listener` until axum 0.8), and `reqwest` cannot probe `/api/health` over a unix socket, so `--bind=/path.sock` needs a new hyper-util accept loop plus a unix-aware watchdog. Punted to a follow-up; `--bind` stays `Option<IpAddr>`, `--port` stays.
2. **Bare `--service=systemd`/`--service=launchd` requires an explicit verb** (error otherwise).
3. **Only `--join` blocks.** `--start`/`--restart`/`--stop`/`--status` return immediately (a behavior change for `--restart`, which used to attach).

### The deliverable

- **Model.** `--service=none` (default) is plain foreground on `--bind`/`--port`, no supervision. `--service=chan` is the self-managed foreground daemon (pidfile + flock). `--service=systemd`/`launchd` are detached background services. The per-OS auto-pick (`ServiceKind::Auto`) is removed; there is no implicit backend.
- **Verbs (systemd/launchd):** `--start` (write/enable/start, then return), `--stop` (stop and disable, so it does not come back on boot/login), `--restart` (rewrite unit for the current binary/addr, bounce, return; fdstore-preserves live PTYs unless `--force`), `--status`, `--join` (ensure running, start if down or attach if up, then block on the health watchdog; SIGINT detaches and the service keeps running). `--join` is the old default behavior, now explicit.
- **Verbs (chan):** bare `--service=chan` runs the foreground daemon; `--stop`/`--restart`/`--status` act on the pidfile; `--join` attaches to a running daemon (errors if none); `--start` is rejected (chan has no detached background; it is a foreground backend).
- **Validity matrix** is a pure `plan_devserver(service, action) -> Result<DevPlan, String>` plus `selected_devserver_action(...)`, both unit-tested, so the async dispatcher stays thin and every invalid `(service, action)` pair errors with a precise, actionable message.
- **Backend re-slicing (no behavior invented):** the systemd/launchd helpers were split from the overloaded functions. `join_*` is the attach + watchdog path, new `start_*` does the same setup without the watchdog and returns, `restart_*` lost its trailing watchdog, `stop_*` gained `disable` (systemctl disable / launchctl disable).
- **`CHAN_HOME` propagation fix** (bug discovered while setting up the isolated launchd test, see "What didn't"): the generated unit carries `Environment="CHAN_HOME=…"` and the plist an `EnvironmentVariables`/`CHAN_HOME` entry, but only when `CHAN_HOME` is set, so production behavior is unchanged.
- **Callers/docs/examples updated:** launcher connect-script samples got `--join` on the systemd/launchd examples (`demo.ts`, `mock.ts`, `NewWorkspaceDialog.svelte`); `design.md`, `crates/chan/design.md`, `docs/contributing/linux-and-macos.md`, the chan-server and desktop comments, and two user-facing error strings dropped the stale `--systemd`/`--launchd` flags; `CHANGELOG.md` gained Changed + Fixed entries.

**Touched files (11):** `crates/chan/src/lib.rs` (bulk), `crates/chan/src/devserver_daemon.rs`, `crates/chan-server/src/devserver.rs`, `desktop/src-tauri/src/devserver.rs`, `design.md`, `crates/chan/design.md`, `docs/contributing/linux-and-macos.md`, `CHANGELOG.md`, and the three launcher files.

### The tests

Static gate (macOS, all green): `cargo fmt --check`; `RUSTFLAGS="-D warnings" cargo clippy -p chan --all-targets`; `cargo test -p chan --lib` (100, including new `plan_devserver` validity matrix, `selected_devserver_action`, action-group parse, and `CHAN_HOME`-propagation tests for both the systemd unit and the launchd plist, with and without `CHAN_HOME`); `cargo test -p chan --test devserver_resilience` (9 foreground SIGINT/SIGTERM/SIGKILL, flock release, tenant PTY reap, `chan close` sync, all unchanged, confirming the default foreground path is untouched); `cargo build -p chan --no-default-features`; `make web-check` (svelte-check + vitest + build for both SPAs); plus `chan devserver --help` and every error path by hand.

Runtime end-to-end (empirically verified, not just gated):

- **systemd** (lima VM, real `systemctl --user`, aarch64 Ubuntu): bare `--service=systemd` errors; `--start` returns, active + enabled, `/api/health` 200; `--status`; `--restart` returns, still active; `--join` attaches + blocks, SIGINT detaches and the unit survives; `--stop` leaves it inactive and disabled. Re-run with `CHAN_HOME` set: the unit carried `Environment="CHAN_HOME=…"`, systemd accepted it, config isolated to the override dir.
- **chan daemon** (lima VM, flock + pidfile): `--start` rejected; empty-state `--status`/`--join` handled; bare run brings up the foreground daemon with `daemon.json`/`daemon.lock` and health 200; `--join` attach/detach; `--restart` takeover (old pid dies, new pid serves on the preserved port); `--stop` clears the pidfile and the process exits.
- **launchd** (macOS, real `gui/$uid` domain), isolated via `CHAN_HOME` pointed at a throwaway dir: bare errors; all verbs walked; `--start` returns; plist carried `CHAN_HOME`; `--restart` returns; `--join` attach/detach; `--stop` deregistered and disabled; all config/token/log landed in the override and `~/.chan/devserver` was never created; plist + agent removed on cleanup.

### What worked

- The reshape was mostly a re-slice of already-verified building blocks, so behavior parity held: the foreground resilience suite passed untouched, and `--join` reproduces the old default exactly.
- All four backends (none, chan, systemd, launchd) verified against real supervisors, including the two behavior changes that mattered most: `--start`/`--restart` return, and `--stop` disables.
- `CHAN_HOME` isolation genuinely works for supervised services, proven by the launchd run leaving the real `~/.chan` completely untouched.

### What did not work / issues found

- **`CHAN_HOME` split-brain bug (found + fixed).** Setting `CHAN_HOME` on the supervisor alone was insufficient and actually broken: launchd/systemd spawn the service with a fresh environment, so the service used the real `~/.chan` while the supervisor read the isolated config, the token handshake would time out, and `--start` would fail. Fixed by baking `CHAN_HOME` into the unit/plist. This is why isolating the launchd test required a code change rather than just an env var.
- **Unix-domain sockets deferred**, not delivered. The `--bind={path}` ask is unmet this round (axum 0.7.9 is `TcpListener`-only and reqwest cannot probe a unix socket). Needs a hyper-util accept loop plus a unix-aware watchdog; tracked as a follow-up.
- **`chan --service=chan --restart` blocks** (it re-serves in the foreground). Inherent to a foreground backend; the "returns" contract only applies to systemd/launchd. Documented.
- **launchd is not CI-reachable** (needs a macOS GUI login domain), so it can only be verified locally, which was done here. systemd is likewise not in CI (no user manager); the lima VM is the exercise path.
- **VM full build snag (worked around):** `cargo build -p chan` in the aarch64 lima VM fails in candle's `gemm-f16` (inline asm needs the `fullfp16` CPU feature). Not our code; sidestepped for testing with `--no-default-features` (drops candle; BM25 search and the whole devserver remain). Flagging in case the aarch64-linux release build hits the same.
- **Pre-existing VM state surprise:** an old `chan-devserver.service` (from earlier manual testing, on port 9800, pointing at `~/.local/bin/chan`) was already active and masked the first clean `--start`; `--start` correctly reported "already running" and returned. Cleared it (which also exercised `--stop`=stop+disable) before the clean run.

### Follow-ups

- Unix-domain-socket `--bind=/path.sock` (deferred). Likely an axum 0.8 upgrade or a scoped hyper-util accept loop for the unix path, plus a unix-socket health probe for the supervised watchdog.
- Consider whether the aarch64-linux release build needs a `gemm`/`fullfp16` target-feature or `--no-default-features` accommodation (separate from this work).

---

## Session notes: devserver-cmd (process retrospective)

Honest lowlights from the agent (me) this session, worth recording so the pattern does not repeat:

- **Hard-wrapped Markdown.** I first wrote this journal wrapped at ~80 columns. House style for `.md` is free-flowing prose (one paragraph or bullet per line; only tables stay near ~80 cols). Rewrote unwrapped, and captured the rule in memory.
- **Introduced em dashes.** My first-pass comments, docs, and this journal used the em-dash character, against the no-em-dash house rule. Fixed my own additions and, at the maintainer's direction, a follow-up commit purges the pre-existing em dashes in the touched files.
- **Scope discipline held elsewhere:** deferred unix sockets up front rather than half-building them, and kept `~/.chan` untouched while testing launchd (via `CHAN_HOME` isolation, which surfaced the propagation bug).

---

## Graph: live force tuner, focus-on-select, deeper fs-graph

Branch `graph-tuning` (merged). Covers the focus and depth parts of the `## Graph` request; the remaining Graph asks are carryover (see the checklist and `dev/v0.59.0/graph-remaining-items.md`).

### What landed

Graph physics as a shared, tunable module. `web/packages/workspace-app/src/graph/force.ts` holds the `GraphForce` type and `DEFAULT_FORCE`, the single source of truth for the d3-force physics. `GraphCanvas` takes an optional `force` prop defaulting to `DEFAULT_FORCE`; every production caller omits it and gets the default. The tuned values: charge -90, link distance 125/128, link strength 1.12, collide 8, center 0.05, hierarchy 90/0.45, parent-X 0.18.

graph-tuner playground (replaces the removed graph-demo). It mounts the real `GraphCanvas`, not a re-implementation, so what you tune is what the live graph does: live sliders for all ten force params, a Copy FORCE button that emits a literal to paste into `force.ts`, plus theme, root-anchor, regenerate, and a depth slider matching the Graph tab's workspace-scope depth. A data-source toggle switches between a synthetic generator and a real `/api/graph` snapshot of this repo's own source, captured to `src/graph-tuner/sampleGraph.json`.

Focus-on-select in `GraphCanvas`. Clicking a node spotlights its first-degree neighbourhood: the selection and its neighbours stay full-strength with labels, incident edges light up, and everything else greys out.

Bottom anchor as the default. `GraphCanvas` `focalAnchor` defaults to `bottom`, so the main Graph tab and the Dashboard slide grow the workspace spine upward from the root.

Deeper fs-graph. `FS_GRAPH_DEPTH_MAX` (frontend) and `MAX_DEPTH` in the chan-server `fs_graph` route both move to 10, so the workspace depth slider reaches the full depth of a deeper source-style workspace; a single request stays bounded by `MAX_NODES`.

Removed the dead sphere-tuner and d3-compare cytoscape-era playgrounds.

### Validation

svelte-check 0/0/0; workspace-app vitest green; chan-server `fs_graph` tests green; `cargo fmt --check` clean. Browser-verified in the tuner against real data (depth slider, focus-on-select spotlight, bottom anchor). The main Graph tab inside chan-desktop was not verified on this branch (checks ran against the web SPA on a local `chan open` server), so it is on the rc validation list.

### Open items

The `sampleGraph.json` fixture is 381 KB (a real-data sample, heavy for the tree): keep, slim to about 307 KB by deriving `contains` edges from paths, or drop. This branch also adds a root `AGENTS.md` so Codex reads the `.agents/` standards.

---

## Index and dashboard: per-path pulse, clickable notification, keep-alive

Branch `index-dashboard` (merged). Covers the whole `## Index and dashboard` request.

### What landed

Clickable indexing notification to a paused Dashboard Indexing slide. The top-right indexing status pill (`AppStatusBar.svelte`) is a button; clicking it opens a Dashboard tab focused on the Indexing (Search) carousel slide with auto-rotation off, so a user watching the index build lands on the live graph and it does not rotate away. A shared `openIndexingDashboard()` helper (plus `DASHBOARD_SEARCH_SLIDE` and an `OpenDashboardOptions` overrides type) in `tabs.svelte.ts`; the server `cs dashboard` handler reuses the same `openDashboardInActivePane({ slide, autoRotate })` path.

Per-path indexing pulse (fixes the "all nodes flash orange together" report). The root cause was backend: during the background embedding sweep the indexer reaches `Idle { embedding: Some(..) }` with no per-file label, so `build_indexing_state` marked every directory with indexable files as `Indexing` at once. `EmbedProgress` now carries `file: Option<String>`, populated from the live `IndexFile` label; `current_index_file` surfaces it during the embed sweep, and the sweep-broadening condition is narrowed so that whenever a real file label is known only that one directory pulses `Indexing` while the rest resolve to `Indexed`/`Pending`. The broad pulse stays only as a fallback for the gaps with no per-file signal.

Dashboard tab keep-alive (fixes reload/re-layout on tab switch). `DashboardTab` moves into the keep-alive each-loop in `Pane.svelte`, mirroring graph/file/terminal tabs: it stays mounted and hides via the `visibility: hidden; pointer-events: none` contract (never `display:none`) with an `active` gate. The Indexing carousel's `GraphCanvas` force layout and 3s poll survive tab switches; the `active` gate also pauses the carousel and stops the indexer poll while hidden. Reload is an explicit user action (Cmd+R or the right-click Reload row).

### Validation

`cargo test -p chan-server` (new embedding-sweep-with-current-file test plus updated `EmbedProgress`/`set_idle` tests); workspace-app `npm run check` + full vitest green (new `paneDashboardTabKeepAlive.test.ts` and updated `dashboardTabAndCarousel.test.ts`); full `make pre-push` gate. Seeded a local standalone server: watched the pill build, clicked to the paused Indexing slide, confirmed `/api/indexing/state` reported one indexing directory at a time, and confirmed the graph did not reload on tab switch. Desktop (WKWebView) not separately verified, so it is on the rc validation list.

### Open items

The Indexing graph polls every 3s, so the pulse advances in 3s steps. Between embed batch flushes `current_file` can briefly be `None`, so a large workspace with long flush intervals can show a brief broad-pulse blip (by design). The right-click Reload row still does a full window reload; a lighter graph-only refresh could come later.

---

## Integration notes (release editor)

Merged onto `main` in order: `devserver-cmd`, `graph-tuning`, `index-dashboard`, each as a `--no-ff` merge. The only conflict across all three was this journal, an add/add, confirmed up front with `git merge-tree`; every code file merged clean. This file is the reconciliation of the three per-branch journals into one, unwrapped and free of em dashes.

Quality pass on the merged tree: removed five newly-introduced em dashes and reworded newly-added change-history ("archaeology") comments to present-tense in the index-dashboard test files (`paneDashboardTabKeepAlive.test.ts`, `dashboardTabAndCarousel.test.ts`) and the style comment in `DashboardTab.svelte`. `devserver-cmd` and `graph-tuning` introduced none. Remaining rc validation and the Graph carryover are tracked in `dev/v0.59.0/plan.md` and `dev/v0.59.0/graph-remaining-items.md`.

---

## Semantic indexing: honor the opt-out (no silent embedding)

**Branch:** `semantic-optout-gate` (worktree `../chan-semantic-optout`, off `main`). Not merged, not pushed. **Status:** complete, gated green (fmt, clippy, `cargo test`, both feature sets build), hardened across three adversarial review rounds. Not yet exercised in a live browser: the `cargo test` environment has no embedder loaded, so the runtime enable/download/rebuild path is on the rc validation list. Maintainer-requested stream outside `dev/v0.59.0/request.md`.

### What was asked

Verify a suspicion, then fix it: does chan start embedding (semantic indexing) whenever a cached BGE model is on disk, even when the user has chosen not to use semantic search, on the premise that "if the user turns it on it is instantly available"? If so, remove that behavior. The user's choice must be the only input to the enable/disable decision; a cached model on disk is not a reason to index. Concretely: with semantic search off, never compute embeddings; turning it off after it was on must bin the indices and wipe them; turning it back on rebuilds from scratch, the same as a reindex. Then a second pass to harden the on/off state machine adversarially (syseng against rustacean) across the chaotic cases: on then off, on then `rm -rf` index, off then on quickly.

### What the investigation found (the suspicion was correct)

`semantic_enabled` (per-workspace, `dashboard.toml`, default false) gated only the query path (bm25 vs hybrid). The indexer never read it: `BuildOptions::include_vectors` defaulted true, `Index::index_one` hard-coded embedding on, and disabling only flipped the flag without wiping. Because the BGE model is bundled and seeded on boot, `model_present` is effectively always true, so every cold boot, full reindex, and per-file save embedded regardless of the user's choice. That is exactly the reported behavior.

### What shipped

- Gate embedding on the opt-in at both write seams: `reindex_with_aggression` and the per-file `index_file_inner` set `include_vectors` from `semantic_enabled()` (fail-safe to false on a config read error). With the flag off, `build_all` and per-file saves write BM25 only, with no embedder load and no shards.
- Destructive disable: `set_semantic_enabled(false)` bins the vector store via a new `Index::clear_vectors` (factored out of `set_model`), mirroring the existing destructive `set_reports_enabled`. BM25 keyword search is untouched, so search keeps working with semantic off.
- Rebuild from scratch on enable: the `/api/index/semantic/enable` endpoint fires `Indexer::request_rebuild()` after persisting the flag, and the reindex now embeds because the gate reads true.
- Cap bypass on explicit opt-in (maintainer decision): a new `BuildOptions::ignore_embed_cap`, set whenever semantic is enabled, so an opted-in workspace embeds its whole tree instead of falling back to BM25-only above the 2000-file `EMBED_FILE_CAP`. On this repo (about 4k files) enabling now populates vectors across the tree rather than only for files later edited.
- Concurrency hardening (the second pass): a `vectors_epoch` generation counter on `Index`, bumped by `clear_vectors` and snapshotted by each build and per-file save before it reads the opt-in flag, so a disable that races an in-flight embed drops the vectors and skips the stamp instead of leaving orphan shards plus a stale `vectors_model`. Supporting changes: clear the on-disk stamp last (so a failed wipe is catchable rather than trusted), and make the embeddings-dir wipe tolerate a missing dir.
- Frontend: none. The Settings toggle already calls the enable/disable endpoints; the behavior change is entirely server-side.
- Touched files (3): `crates/chan-workspace/src/index/facade.rs`, `crates/chan-workspace/src/workspace.rs`, `crates/chan-server/src/routes/index.rs`. No new dependencies, no schema change. CHANGELOG entry is pending merge.

### The tests

- Four new deterministic tests, none needing an embedder (the disabled path never embeds, so "zero vectors, populated BM25" is model-independent; the disable-wipe test uses a stand-in stale shard exactly like the existing model-switch test): `reindex_disabled_writes_no_vectors_but_indexes_bm25`, `per_file_index_disabled_writes_no_vectors`, `disabling_semantic_bins_the_vector_store` (all in `workspace.rs`), and `clear_vectors_is_idempotent_and_tolerates_missing_dir` (in `facade.rs`).
- Gate: 571 `chan-workspace` lib tests and 490 `chan-server` tests green, plus the workspace integration suites; `cargo clippy --all-targets` clean; `cargo fmt --check` clean; full-workspace `cargo build` and `cargo build --no-default-features` both green.
- The race hardening is validated by adversarial code review, not a concurrency test: a deterministic test would need an embedder plus a hook to flip the flag mid-build, beyond the scope agreed for this pass.

### Highlights (what went well)

- The fix reused existing seams instead of inventing machinery: one bool (`include_vectors`) already gated all embedding, the wipe already lived inside `set_model`, the destructive-on-disable shape already existed in `set_reports_enabled`, and the reindex trigger already existed as `Indexer::request_rebuild`. The behavior change is small and idiomatic.
- The adversarial second pass did its job: syseng and rustacean, run independently without seeing each other's output first, converged on the same defect in each round. That agreement is strong signal the findings were real rather than model artifacts.
- Fail-safe defaults throughout: a config read error yields BM25-only, never an accidental embed.

### Lowlights (what needed a nudge, bugs, slowdowns)

- The first-pass gate carried a real HIGH bug that only the hardening pass caught. Disabling while an enable-triggered whole-tree embed was in flight resurrected the vectors it had just wiped and wrote a `vectors_model` stamp; because a disabled reindex skips vector cleanup and `Index::open` only wipes on a model mismatch, the orphan vectors and the lying stamp persisted across restarts. This is precisely the "turn on then off quickly" case. Fixed with the epoch counter.
- The epoch fix itself had a residual that round-2 verification caught: the epoch was sampled after the opt-in flag was read, leaving a TOCTOU window (the file walk) where a disable could still slip vectors through and persist them. Fixed by sampling the epoch before the flag read and threading it into the build and per-file paths. Two iterations before the state machine was actually closed.
- House-style slips, the same two the previous session flagged: first-pass code comments used em dashes (fixed in my additions), and the planning doc hard-wrapped prose (kept to the plan file, outside the tree). The no-em-dash and no-archaeology rules were applied to the committed comments.
- One subagent verification run returned corrupted, off-topic output with zero tool calls and had to be re-run. Cost a round-trip; caught only because the result did not reference the code.
- No live browser verification this session: the sandbox test environment has no embedder, so the download/enable/rebuild path and the bm25-to-hybrid upgrade were not exercised end to end. On the rc validation list.

### Residual (accepted)

If the disable-time `remove_dir_all` itself fails with a genuine filesystem I/O error (not the common already-absent case, which is tolerated), a few shards can linger while disabled. It is logged, not hidden, and `Index::open` does not auto-reclaim it because the model is unchanged. This sits outside the three chaotic cases scoped for the pass and is the only path outside the guarantee. A crash-safe wipe marker (like `rebuild.inprogress`) would close it if we decide it is worth the machinery.

### Follow-ups

- CHANGELOG entry on merge.
- Live browser validation: enable via Settings, watch the whole-tree embed, toggle off and confirm the embeddings dir is binned and search stays bm25, toggle on and confirm the rebuild.
- Optional: cache the `dashboard.toml` read (parsed once per reindex and once per per-file save today). A minor hot-path cost, not a correctness issue.
