# v0.59.0-rc1: rolling release journal

Working journal for the v0.59.0 cycle. Unlike the per-release notes above, this is a rolling doc: appended to as each work stream from `dev/v0.59.0/request.md` lands on its branch, and reconciled into a final `release-0.59.0.md` at cut time. As of this entry the `devserver-cmd`, `graph-tuning`, `index-dashboard`, `semantic-optout-gate`, `editor-fixes`, `mermaid-ux`, and `cs-copy-paste` streams are merged onto `main`; only chan-desktop is still in flight, and the `v0.59.0-rc1` tag waits for it. Each section stands alone so the release summary can be assembled from these entries.

## Work streams (from `dev/v0.59.0/request.md`)

- [x] **`chan devserver` command**: reshape `--service` into explicit action verbs (branch `devserver-cmd`)
- [x] Graph: focus-on-select grey-out with first-order edge focus, deeper fs-graph, live force tuner (branch `graph-tuning`)
  - [x] Carryover (branch `graph-remaining-items`): auto-select root on open, "data being indexed, hang tight..." placeholder, `@@mention` "Graph from here" mention-lens edges, plus full-containment-spine spotlight on select
- [x] Index & dashboard: clickable indexing notification opening a paused Dashboard Indexing slide, per-path indexing pulse, no reload on tab switch (branch `index-dashboard`)
- [x] Editor bugs: directory links open the file browser, list continuation hang-indent, enumerated-list indent, plus smart list-row paste (branch `editor-fixes`; setext bold-flash dropped by decision)
  - [x] Feature: `mermaid-to-excalidraw` renderer via a shared diagram widget, lazy-loaded (branch `mermaid-ux`)
- [ ] Chan desktop: second-monitor hide/show window shrink, window-title glyphs
- [x] UX: friendlier `cs open` guidance, coherent standalone-terminal command gating, `cs download`/`upload` confirmed working from both standalone and workspace (branch `mermaid-ux`)
- [x] Semantic indexing opt-out (maintainer-added, outside `request.md`): with semantic search off never embed, disabling wipes vectors, enabling rebuilds (branch `semantic-optout-gate`)
- [x] `cs copy` / `cs paste` clipboard bridge (maintainer-added, outside `request.md`): copy/paste between the terminal and the system clipboard (branch `cs-copy-paste`)

---

## `chan devserver` command: explicit action verbs

**Branch:** `devserver-cmd` (worktree `../chan-devserver-cmd`, off `origin/main`). Merged. **Status:** complete, gated green, empirically verified end-to-end on all reachable backends.

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

Dashboard tab keep-alive (fixes reload/re-layout on tab switch). `DashboardTab` moves into the keep-alive each-loop in `Pane.svelte`, mirroring graph/file/terminal tabs: it stays mounted and hides via the `visibility: hidden; pointer-events: none` contract (never `display:none`) with an `active` gate. The Indexing carousel's `GraphCanvas` force layout and 3s poll survive tab switches; the `active` gate also pauses the carousel, stops the indexer poll, and pauses the `GraphCanvas` render loop (`paused={!active}`, mirroring `GraphPanel`) while hidden, so a backgrounded dashboard does no work and no canvas paint. Reload is an explicit user action (Cmd+R or the right-click Reload row).

### Validation

`cargo test -p chan-server` (new embedding-sweep-with-current-file test plus updated `EmbedProgress`/`set_idle` tests); workspace-app `npm run check` + full vitest green (new `paneDashboardTabKeepAlive.test.ts` and updated `dashboardTabAndCarousel.test.ts`); full `make pre-push` gate. Seeded a local standalone server: watched the pill build, clicked to the paused Indexing slide, confirmed `/api/indexing/state` reported one indexing directory at a time, and confirmed the graph did not reload on tab switch. Desktop (WKWebView) not separately verified, so it is on the rc validation list.

### Open items

The Indexing graph polls every 3s, so the pulse advances in 3s steps. Between embed batch flushes `current_file` can briefly be `None`, so a large workspace with long flush intervals can show a brief broad-pulse blip (by design). The right-click Reload row still does a full window reload; a lighter graph-only refresh could come later.

---

## Editor: mermaid-to-excalidraw renderer + shared diagram widget

### What landed

A second diagram renderer, triggered by a fenced `mermaid-to-excalidraw` block. Built on `@excalidraw/mermaid-to-excalidraw` + `@excalidraw/excalidraw` (both MIT): `parseMermaidToExcalidraw` -> `convertToExcalidrawElements` -> `exportToSvg`, all headless (no React editor mounted), returning an SVG string exactly like the mermaid path.

The mermaid widget was generalized rather than copied: `widgets/mermaid.ts` became `widgets/diagram.ts`, a renderer-agnostic block-replace widget parameterized by `{ lang, label, render, isDark, onView }`, with its own per-instance face/error caches so the two renderers never collide on a shared source key. `mermaidDecorations` and `excalidrawDecorations` are thin wrappers; `mermaid_render.ts` and a new `excalidraw_render.ts` supply the render functions over a shared `diagram_render.ts` (the `DiagramResult` type + `parseErrorPos`). The widget CSS moved from `cm-md-mermaid-*` to shared `cm-md-diagram-*`. Both libraries are dynamic-imported, so excalidraw + its React runtime code-split out of the eager editor bundle (confirmed in the vite chunk output: excalidraw lands in a lazy `prod-*.js`, not the entry).

The click-to-zoom overlay (`state/diagramZoom.ts`, removed in `e0026410`) was reintroduced for BOTH renderers per the maintainer decision, on a hover "View" button. It always renders LIGHT on a light panel (a dark-editor diagram re-renders light for the overlay), which is the black-on-black fix from the original `04b0413e`.

### Validation

`npm run check` (0 errors / 0 warnings) + full vitest (2121 pass, including new `widgets/diagram.test.ts`, `excalidraw_render.test.ts`, and the restored `state/diagramZoom.test.ts`) + production build. Browser-verified on a standalone server in a dark editor: the mermaid flowchart still renders (no regression from the refactor), the excalidraw flowchart and sequence render with the embedded hand-drawn Excalifont (dark mode reads correctly, no black-on-black), a bad excalidraw block shows the actionable "Excalidraw error - line N" face, and the View -> zoom overlay opens on a light panel with working +/-/Reset/pan for BOTH renderers, dismissed cleanly with Escape.

Found and fixed a real zoom bug inherited from the restored overlay: mermaid's SVG carries `width="100%"` and no height, so `width:auto` collapsed it to 0x0 inside the shrink-to-fit panel (the diagram vanished; this matches the empty / buggy-box behavior the maintainer hit before, and is the likely reason the overlay was originally removed). `diagramZoom.ts` now derives an intrinsic pixel width from the SVG's viewBox; excalidraw's export already carries pixel dimensions so it is unaffected. Pinned with two `diagramZoom.test.ts` cases.

One benign console notice remains from excalidraw's font subsetter ("Failed to use workers for subsetting, falling back to the main thread"): it falls back to the main thread and the font still inlines (the Excalifont renders), so it is cosmetic.

### Open items

- The fence token: the request wrote `mermaid-to-excallidraw` (doubled l) but the upstream library is `mermaid-to-excalidraw`. Shipped with the upstream spelling as the default, isolated in one constant `EXCALIDRAW_LANG` in `widgets/diagram.ts` for a one-line swap; the maintainer survey to confirm the exact token is still open at journal time.
- Light-editor inline render not separately screenshotted (strictly the easier case: default palette on a light surface, and the overlay is always light regardless of editor theme); dark mode (the risky case) is fully verified.
- Desktop (WKWebView) not separately verified, so it is on the rc validation list.

## UX: friendly `cs open` + coherent standalone-terminal command gating

### What landed

`cs open PATH` from a standalone terminal (which has no workspace to open a path into) now prints friendly guidance to run `chan open PATH` to load it as a workspace window, instead of the generic "needs a workspace" refusal. The standalone-vs-workspace gating, previously scattered across `handle_request` match arms and conflated with workspace resolution in `workspace_from_cell`, is now a single pure decision `terminal_tenant_refusal(&ControlRequest, ControlTenant) -> Option<String>` consulted once at the top of `handle_request`. It refuses only the workspace-content commands on a terminal tenant (`cs open` -> the chan-open guidance; `cs graph` / `search` -> the generic refusal; `cs terminal new --path` -> the path message) and lets window-routing, session/pane ops, and the cwd-scoped `cs upload` / `download` through.

`cs upload` / `download` from a standalone terminal already worked (server-side tenant routing landed earlier in `c7deaab7`); this stream verified that against HEAD and did NOT re-add any restriction. Also fixed two stale comments that listed `dashboard` as a workspace-gated command (it is not gated) and removed the em dash from the `TERMINAL_ONLY_NEEDS_WORKSPACE` string (house style).

### Validation

`cargo fmt --check` + `cargo clippy -p chan-server --all-targets` under `RUSTFLAGS=-D warnings` + `cargo test -p chan-server` (495 pass). New tests: a platform-neutral `tenant_gate_tests` module table-driving `terminal_tenant_refusal` across every command/tenant pair, plus a `handle_request`-level test that `cs open` on a terminal tenant returns the `chan open` guidance.

### Open items

- `cs terminal team` keeps its own lazy in-handler workspace refusal (unchanged): coherent, but not folded into the pure decision, to avoid destabilizing the team path.

---

## Retrospective (mermaid-ux branch)

### What was asked

The `dev/v0.59.0/task-mermaid-ux.md` brief: the Editor *Feature* (a mermaid-to-excalidraw diagram renderer that follows and abstracts the existing mermaid renderer, with minimal integration points, clean APIs, documentation, lazy-loading, a license-compatible embedded bundle, and the existing renderer's lifecycle) plus the whole `UX` section (a friendlier `cs open` from a standalone terminal, and unblocking `cs download` / `upload` from standalone terminals through a clean gating refactor with unit-testable command-context gating). The four Editor bugs were explicitly out of scope (a different branch).

### What shipped

Both streams, on branch `mermaid-ux` off `origin/main`, in one commit (`d6712ac`), full `make pre-push` green and browser-verified. Stream A: the excalidraw renderer, the mermaid widget generalized into a shared `widgets/diagram.ts`, and the click-to-zoom overlay reintroduced for both renderers (a maintainer decision, since the overlay had been removed from the tree). Stream B: the friendly `cs open` guidance and a single pure `terminal_tenant_refusal` gate; `cs upload` / `download` were already working from standalone terminals, so that half was verified rather than re-implemented. Detail is in the two sections above. The fence token shipped as the upstream spelling `mermaid-to-excalidraw`, isolated in one constant `EXCALIDRAW_LANG` for a one-line change.

### Highlights (what went well)

- Abstract, do not copy: generalizing the intricate ~470-line mermaid widget into one parameterized `diagram.ts` means both renderers share the entire CM6 implementation (flip, reverse-flip ghost, atomic ranges, vertical step-into, error accents, per-instance caches); the new renderer is a thin wrapper plus a render module. This is the "abstract where necessary, minimal integration points" the brief asked for, not a parallel stack.
- Caught the maintainer's exact prior bug. The empty / collapsed mermaid zoom reproduced in the browser, root-caused (mermaid's SVG is `width="100%"` with no height, so it collapses to 0x0 in the overlay's shrink-to-fit panel), fixed by deriving an intrinsic width from the viewBox, and pinned with tests. This is very likely why the zoom was removed originally, so the reintroduction closes that loop rather than reopening it.
- Reconciled the brief against HEAD before building. `cs upload` / `download` already worked from standalone terminals, so Stream B did not re-add a restriction and spent its effort on the real gap (friendly `cs open` plus the pure gate).
- Visual validation earned its keep. Beyond the mermaid-zoom collapse, it surfaced that excalidraw embeds subgraph flowcharts as an image (graceful, not an error), which sets honest expectations for the renderer.
- Lazy-load discipline held. Excalidraw and its React runtime code-split out of the eager editor bundle, confirmed in the vite chunk output and pinned by a test that forbids a static import.

### Lowlights (what was missed, could be better)

- Survey hygiene. I fired two separate surveys (fence token, zoom scope) instead of consolidating into one; the token survey then timed out unanswered and I proceeded on the default. One batched survey would have been cleaner and less intrusive.
- The subgraph image-fallback slipped the first validation round. The initial synthetic doc used a simple flowchart plus two clean diagrams, so the fact that flowcharts with subgraphs fall back to an embedded image only surfaced during the real-docs showcase pass. A subgraph case belonged in the first round.
- The excalidraw font subsetter logs a benign "failed to use workers for subsetting, falling back to the main thread" warning. I left it as cosmetic instead of checking whether the worker asset can be bundled to silence it.
- Coverage gaps left open: the light-editor inline render is inferred rather than screenshotted (only the harder dark case is captured), and desktop / WKWebView is unverified. Both are on the rc list.
- What the brief itself missed: it described the existing renderer's lifecycle as including a working "view/zoom overlay" and warned about a black-on-black overlay, but that overlay had already been removed. The spec assumed a lifecycle that was not in the tree, which needed a maintainer decision to resolve. Briefs that reference existing behavior are worth reconciling against HEAD before they go out.
- Dependency weight. Excalidraw pulls React and roughly 339 packages into `node_modules`. It is lazy-loaded and out of the eager bundle, but it is a large addition for a Svelte app and deserves a conscious eye at release time.

---

## Integration notes (release editor)

Merged onto `main` in order: `devserver-cmd`, `graph-tuning`, `index-dashboard`, `semantic-optout-gate`, then `editor-fixes`, each as a `--no-ff` merge. Every merge shared exactly one add/add conflict, on this journal, and every code file merged clean. `semantic-optout-gate` was cut from the reconciled `main` (strictly ahead, no code conflict); `editor-fixes` was cut from the original `main` and auto-merged over the other streams, its `tabs.svelte.ts` and `workspace.rs` edits sitting in functions disjoint from the index-dashboard and semantic-optout changes. A later `index-dashboard` follow-up (`7a026ba4`, pause the indexing `GraphCanvas` render loop while hidden) was merged on top; its code auto-merged clean and its journal delta folded into the Index section above. Then `mermaid-ux` was merged: cut from an earlier `main`, it auto-merged over the later streams, and its only cross-stream file, `Wysiwyg.svelte` (also touched by `editor-fixes`), merged coherently (the list-decoration changes and the diagram-widget decorations sit in separate regions of the extensions list); the CHANGELOG add/add on the `### Added` bullets was resolved by keeping both. `mermaid-ux` adds a heavy frontend dependency (`@excalidraw/excalidraw`, which pulls React plus roughly 339 packages, lazy-loaded out of the eager editor bundle); it was full-gate green on its branch, and the rc1-cut full gate is the authoritative build check for the merged tree. Finally `cs-copy-paste` (maintainer-added, outside `request.md`) merged with no conflicts at all; its overlaps `crates/chan-server/src/lib.rs` (also touched by mermaid-ux) and `state/store.svelte.ts` (also touched by index-dashboard) auto-merged in disjoint regions, it was full-gate green on its branch, and the editor added its missing CHANGELOG entry. Finally `graph-remaining-items` (the graph carryover, frontend-only) merged as a 3-way merge that conflicted only on this journal and the CHANGELOG (both reconciled by union); `tabs.svelte.ts` auto-merged coherently across the index-dashboard, editor-fixes, and graph edits, and it was svelte-check + vitest green and browser-verified on its branch. This file is the reconciliation of the per-branch journals into one, unwrapped and free of em dashes.

Quality pass on the merged tree: removed five newly-introduced em dashes and reworded newly-added change-history ("archaeology") comments to present-tense in the index-dashboard test files (`paneDashboardTabKeepAlive.test.ts`, `dashboardTabAndCarousel.test.ts`) and the style comment in `DashboardTab.svelte`. `devserver-cmd`, `graph-tuning`, `semantic-optout-gate`, `editor-fixes`, and `mermaid-ux` introduced none (the semantic `vectors_epoch` "old epoch" comment is present-tense domain language, not archaeology, and the one em-dash occurrence in the mermaid-ux diff is a test asserting the `cs open` guidance string carries no em dash). Remaining rc validation and the Graph carryover are tracked in `dev/v0.59.0/plan.md` and `dev/v0.59.0/graph-remaining-items.md`.

---

## Semantic indexing: honor the opt-out (no silent embedding)

**Branch:** `semantic-optout-gate` (worktree `../chan-semantic-optout`, off `main`). Merged. **Status:** complete, gated green (fmt, clippy, `cargo test`, both feature sets build), hardened across three adversarial review rounds. Not yet exercised in a live browser: the `cargo test` environment has no embedder loaded, so the runtime enable/download/rebuild path is on the rc validation list. Maintainer-requested stream outside `dev/v0.59.0/request.md`.

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

- CHANGELOG entry: added at merge.
- Live browser validation: enable via Settings, watch the whole-tree embed, toggle off and confirm the embeddings dir is binned and search stays bm25, toggle on and confirm the rebuild.
- Optional: cache the `dashboard.toml` read (parsed once per reindex and once per per-file save today). A minor hot-path cost, not a correctness issue.

---

## Editor bugs: list hang-indent, smart list paste, directory links

Branch `editor-fixes` (merged). Covers the Editor bug fixes from `request.md`; the `mermaid-to-excalidraw` feature is deferred to the mermaid/ux stream, and the setext-heading bold flash (bug 2) was dropped by maintainer decision (a non-issue; setext headings stay on). The list work is presentation-only: the markdown document is never rewritten, so everything round-trips.

### What landed

List hang-indent (bugs 3 and 4). Wrapped continuation lines hang under the item text across hyphen, asterisk, plus, ordered-period, ordered-paren, and task lists at every depth. Source whitespace around a marker is hidden render-only so text starts at a fixed marker column; a static CSS rule pads by that column and pulls the first line back with `text-indent`. Indentation follows the item's syntactic depth (one marker column per level), so ordered lists step the same width as bullets and nested markers sit under the parent's text. Task checkboxes share the marker column and stay clickable.

Smart list paste. Pasting a copied list row into an existing list item (typically the empty one Enter just created) flows the content into that bullet instead of inserting a second marker. The rich-HTML path already dedented via `dedentListPaste`; the same dedent now runs on the plain-text path (a chan-to-chan copy uses `navigator.clipboard.writeText`). It fires only when the caret is on a list line and the pasted text starts with a marker; every other paste defers to CodeMirror.

Directory links (bug 1). `resolve_link` detects a directory target after its file-candidate probe and returns it with a new additive `is_dir` wire flag (serde default, no `NodeKind` variant, no route change). The link renders as a valid directory pill instead of a broken strikethrough, and the click opens the file browser at that folder via `openBrowserInActivePane` instead of handing a directory to the text editor. File links and genuinely missing links keep their behavior.

### Validation

`cargo fmt --check`, `cargo clippy` with `RUSTFLAGS="-D warnings"`, `cargo check --workspace`, `cargo test -p chan-workspace` (the `resolve_link` suite is 15 tests, two new for the directory and non-directory cases); web `npm run check` (svelte-check 0), vitest 2090 (new `openLinkTarget` directory-routing test), build. Browser-verified against a workspace seeded with every list type at long wrap widths: zero text-column delta per depth; click-to-edit, undo, Enter continuation, checkbox toggle, Tab and Shift-Tab all correct; copy-paste round-trips preserved (including rows with an inline image); the directory link opened the file browser while file and missing links stayed as before. Verified on the pre-merge branch; the merge with `main` auto-merged clean, with the `tabs.svelte.ts` and `workspace.rs` edits in functions disjoint from the other streams.

### Still open

Progressive outdent: Enter on an empty nested item exits the list in one press rather than outdenting a level at a time (optional keymap tweak, not done). The `mermaid-to-excalidraw` feature is deferred to the mermaid/ux stream.

---

## `cs copy` / `cs paste`: clipboard bridge for the terminal

**Branch:** `cs-copy-paste` (rebased on `origin/main`, in the primary worktree). Committed as a `feat` plus this `docs` commit; merged. **Status:** feature complete plus a full two-sided adversarial audit whose findings are all fixed; gated green (fmt, clippy, `cargo test`, `--no-default-features`, svelte-check, full vitest, web build, `cargo check -p chan-desktop`). Not yet exercised in a live browser or chan-desktop: this host has no browser/Chromium libs and the desktop image path is WKWebView specific, so the paste-of-an-image and decode-bomb paths are on the rc validation list. Maintainer-requested stream outside `dev/v0.59.0/request.md`.

### What was asked

Two requests, back to back. First: add `cs copy` and `cs paste` so an agent or human in the embedded terminal can bridge stdin/stdout to the user's clipboard on both the web UI and chan-desktop. The motivating cases are macOS shaped: select an image and `cs paste > file.png`, or `cs copy < file.png` then Cmd+V into Gmail in a real browser. Plain text and images should work out of the box; on the follow-up question the maintainer chose text plus images plus HTML (rich text), and image-first when the clipboard holds both an image and text. Second: spin up a rustacean and a webdev reviewer to adversarially attack each other's boundary (server vs client), find new bugs, prove them, and report an analysis; then, on the maintainer's word, fix everything found.

### What shipped

- The bridge rides the existing `cs pane` style window round-trip, reusing the generic `WindowBus` and the `POST /api/window/reply` route unchanged (its payload was already an opaque `serde_json::Value`), so no new reply route was needed. `cs copy` and `cs paste` both block for a real success/failure instead of firing and forgetting, because a browser clipboard write or read can fail on permission or a missing user gesture.
- Wire and CLI (`crates/chan-shell`): `ControlRequest::ClipboardCopy` / `ClipboardPaste` plus a `PastePrefer` enum in `wire.rs`; `cs copy [--mime|--html]` and `cs paste [--text|--html|--image]` in `cli.rs`, with the copy path base64-encoding stdin and the paste path base64-decoding the reply to raw stdout (so a redirect yields the real asset) and printing the emitted MIME to stderr.
- Server (`crates/chan-server/src/control_socket.rs`): `WindowCommand::ClipboardWrite` / `ClipboardRead`, a `clipboard_round_trip` built on the same bus as pane, and content sniffing that reuses `chan_workspace::fs_ops` (`looks_like_text` plus a new `sniff_image_mime` magic-byte helper next to `classify`, and a light HTML signature sniff), so detection lives server-side and chan-shell stays lean.
- Frontend (`web/packages/workspace-app`): a new `api/clipboard.ts` bridge (base64, PNG normalization via canvas, an HTML two-representation ClipboardItem with a DOMParser plain-text fallback), the `clipboard_write` / `clipboard_read` dispatch and responders in `store.svelte.ts`, and image/HTML clipboard helpers in `api/desktop.ts` that branch browser `navigator.clipboard` vs the desktop IPC exactly like the existing text helpers.
- Desktop (`desktop/src-tauri`): native `read/write_clipboard_image` and `read/write_clipboard_html` IPC over `arboard` (enabling its `image-data` feature, adding the `image` crate for PNG to RGBA), because WKWebView's async clipboard write is gesture gated, which is why text already went native.
- Then, after the audit, all seven findings fixed (below).

### The adversarial audit (second request)

Two read-only reviewers ran in parallel, each owning one side and told to attack the other's trust assumptions, calibrate severity to the real threat model (loopback, single-user, bearer-token gated, so the adversaries are a semi-trusted same-user client and an in-origin script), and refute each finding against the code before reporting. They converged independently on the same top bug, which is strong signal it was real rather than a model artifact. Seven findings survived, all since fixed:

- **F1 (blocker, functional):** `cs paste` of any image larger than about 1.5 MB (a normal screenshot or photo) failed. `POST /api/window/reply` had no body-limit override, so axum's default 2 MB applied; the base64 image reply exceeded it and 413'd, the SPA swallowed the failure as if it were a stale-id 404, the window-bus oneshot never fired, and the CLI hung the full 30 s then printed a misleading "no reply from the window". Fixed by raising the body limit on both reply-route registrations and narrowing the swallow to warn on a non-404. Both reviewers found this one.
- **F2 (robustness/DoS):** no size cap anywhere on the copy pipeline, so `cs copy < /dev/zero` OOMed the CLI and a multi-GB payload OOMed the server and fanned out to every connected tab. Fixed with a shared `MAX_CLIPBOARD_BYTES` enforced at the CLI stdin read, a bounded control-socket `read_line`, and the server copy handler.
- **F3 (robustness/DoS):** image decompression bombs (a tiny file declaring huge dimensions) OOMed the desktop process on decode and the SPA tab on the canvas normalization. Fixed with `image::Limits` on the desktop decode and a pixel-dimension guard plus a read-size guard on the web side.
- **F4 (defense in depth, preconditioned on an in-origin script, pre-existing for `cs pane`):** predictable `win-N` request-ids, window_commands broadcast to every `/ws` subscriber, and a token-gated but not origin-bound reply route meant an in-origin script could race a forged reply into a pending `cs paste > file` and inject bytes. Hardened by moving the window_command filter server-side in the `/ws` pump (so other-window and untagged sockets stop receiving another window's ids and payloads) and by minting unguessable random request-ids across the window, survey, and handover buses. Honest residual below.
- **F5 / F6 / F7 (low):** the web paste labeled the first image type as `image/png` regardless of the real type; a forced `--mime text/plain` on binary was silently lossy; `cs paste` writes raw bytes including control sequences to stdout. Fixed the label, added a non-UTF-8 rejection for a forced text mime, and documented the raw-bytes behavior in the help.

### The tests

- Rust: wire-tag round-trips for the new requests and `PastePrefer` strings; `sniff_image_mime`, `detect_clipboard_mime`, and `looks_like_html`; CLI parse tests for the copy/paste flags; an integration test that `POST /api/window/reply` accepts a body over 2 MB (the F1 regression guard, driving the real router); over-cap and non-UTF-8 rejections in the copy handler; a prefix-coupling test pinning the serialized `window_command` shape the `/ws` scan depends on; and the `window_command_target` extraction. Gate: chan-shell 81, chan-workspace 573, chan-server 497, all green, plus clippy and `--no-default-features`.
- Web (vitest): the base64 round-trip, the write representations, the prefer read order, the over-cap read guard, and the actual-type image label. Full suite 206 files / 2116 tests green, plus svelte-check and the production build.
- Desktop: `cargo check -p chan-desktop` green with the `arboard` image-data feature and the `image` crate.

### Highlights (what went well)

- The feature reused existing machinery instead of inventing it: the whole round-trip is the pane-query mechanism, the `WindowBus` and the reply route were already generic over an opaque payload, and the content detectors were already in `fs_ops`. The only genuinely new plumbing was the desktop `arboard` image/HTML IPC and the small web bridge, which itself reused the existing browser-vs-desktop branch shape.
- The two-reviewer audit paid for itself immediately: the rustacean and the webdev, run without seeing each other's output, both landed on F1 first. That independent convergence is the reason I trusted the finding enough to call the feature a shipped-broken headline case rather than a maybe.
- Severity was calibrated honestly to the loopback single-user model rather than inflated: F4 is presented as preconditioned on an in-origin foothold and pre-existing for `cs pane`, not as a remote exploit, and the residual that the hardening does not fully close is written down rather than hidden.

### Lowlights (what I missed, what we missed, slowdowns, bugs I caused)

- The big one: I declared the feature complete and reported it gate-green with no known bug, and it shipped a blocker. F1 broke the headline use case (paste of a normal photo), and my green gate never caught it, because there is no automated end-to-end clipboard test and this host has no browser to run one by hand. The gate proved the code compiles and the units pass; it proved nothing about the live round-trip, and I over-trusted its green. Two independent reviewers found in one pass what my own "done" report had missed.
- The bugs the audit surfaced were mine, from the first pass. I added a byte-carrying reply path (`cs paste` to a redirected file) on top of a request-id scheme and a broadcast that were fine for printing pane-layout JSON but not for injecting bytes, and I never revisited that trust boundary when the stakes changed (F4). I added a stdin-to-clipboard path with no size cap (F2) and handed attacker-influenced image bytes to a decoder and a canvas with no limits (F3). None of these are exotic; they are the first questions an adversarial reviewer asks, and I did not ask them of my own code until prompted to.
- What the codebase missed, not just me: the reply-hijack surface (predictable ids, broadcast window_commands, an origin-agnostic reply route) predates this feature and is shared by `cs pane` and `cs terminal survey`. It sat latent; clipboard only raised its impact from "forge some JSON the CLI prints" to "write attacker bytes to a user-chosen file". The full fix (per-window capability tokens) is bigger than this branch and is left as a tracked follow-up.
- Slowdowns: chan-server recompiles took about a minute per server-side touch, so the loop of a small edit plus a scoped test was the dominant cost; the F1 body-limit test needed the full 60-line `route_test_app` harness, which is copy-pasted per test module in this crate rather than shared, so I duplicated it a third time; one backgrounded `cargo build --no-default-features` hit the 2-minute foreground timeout and had to be re-run detached; and the 32 MB over-cap test allocates real memory, a slow (about 1 s) but honest test.
- I did not write a CHANGELOG entry (consistent with the other in-flight streams here, which note it as pending merge, but still a gap), and nothing is committed yet, so this work is not on a branch tip CI or the maintainer can pull.

### Residual (accepted)

- F4 is hardened, not closed: the single-user model has no per-window authentication, so a script already executing in the SPA origin (an XSS foothold, which already implies broad file and terminal compromise) can still connect `/ws?w=<victim>` to impersonate the target and race a reply. The two changes raise the bar from "any in-origin subscriber passively hijacks" to "must actively impersonate the specific window and win the race". Full closure needs per-window capability tokens, a separate design change.
- The web decode-bomb guard is a canvas-and-re-encode guard: `createImageBitmap` may allocate the decoded bitmap before the dimension check runs, a residual hard to avoid from JS; the byte cap bounds the source file and the desktop path is fully bounded by `image::Limits`.
- Desktop HTML read uses `arboard`'s HTML getter and is only reachable via an explicit `cs paste --html`; the default image-first order never selects it.

### Follow-ups

- Live verification once a server and a real browser or chan-desktop are available: paste of a multi-MB photo completes fast instead of the old 30 s hang (F1), copy-then-paste of an image round-trips, `cs copy --html` keeps formatting into Gmail, and the decode-bomb inputs fail cleanly on both surfaces (F3). This is the item that would have caught F1 before it shipped.
- Per-window capability tokens to close F4 across `cs pane`, `cs terminal survey`, and clipboard.
- A shared route-test harness so the `route_test_app` builder stops being copy-pasted per module.
- CHANGELOG entry, and commit the branch (feature and audit fixes, split or squashed per the maintainer's preference).

---

## Graph remaining: root auto-select, indexing placeholder, mention-lens edges

Branch `graph-remaining-items` (off `main` after `graph-tuning` merged). Covers the three open `## Graph` asks that `graph-tuning` did not: auto-select the root on open, the "data being indexed" placeholder, and the `@@mention` "Graph from here" missing-edges bug. Frontend-only; the semantic graph server route is unchanged.

### What landed

Focus the workspace root on open (Item A). `openGraphInPane` seeds `pendingSelectId` to the workspace-root node id `""` (the server's `directory_node_id("")`) for a semantic workspace open, so `load()` resolves it through the same path the lenses use: the root node is selected, focus-on-select spotlights the root and its first-degree neighbourhood, and the inspector opens on it. This holds for the main-window Graph shortcut and every other non-lens open, and matches the lens opens (file / directory / `@@mention` / `#tag` / contact / language), which already pass their own focal node. Filesystem and language modes carry no root focus. A manual click still re-selects, and a lens's own `pendingSelectId` is preserved.

Indexing placeholder (Item B). `GraphPanel`'s `emptyStateMessage` markdown-mode fallback reads "data being indexed, hang tight...". The filesystem / index-building / language sibling branches are unchanged, as only the markdown-mode string was called out.

Mention / tag / contact lens edges (Item C). The bug is entirely client-side: `build_graph_view` (`/api/graph`) returns the complete workspace-wide semantic graph regardless of scope (verified with a throwaway `build_graph_view` integration test over the nested fixture and with `chan workspace graph --scope all`), so no edge is dropped on the server. The mention / tag / contact lenses BFS out from the seed to the documents that reference it and stop there; a surfaced document's other `@@mention` / `#tag` / language edges point one hop past the frontier, and the both-endpoints `visibleEdges` filter then culls them. New pure helper `web/packages/workspace-app/src/graph/lensClosure.ts` (`pullMetaNeighbours`) closes over those incident meta-nodes, bounded so only meta-nodes join and the neighbourhood never fans out through unrelated documents; the three BFS arms call it before the directory-spine pull. Directory scope is unaffected (it already admits every meta-node); its remaining gaps are cross-subtree file-to-file links, which is the tree-and-expand model, not this bug (see Open items).

Focus-on-select containment spine (follow-up enhancement). Selecting a directory-tree node (any file / directory / contact / symlink / media, i.e. any node with a `contains` parent) lights and labels its whole parent chain up to the workspace root, not just the immediate parent, so the path home reads at a glance. Pure helper `web/packages/workspace-app/src/graph/containmentSpine.ts` walks the `contains` parent chain (cycle-guarded); `GraphCanvas` builds a child -> parent map in `rebuildAdjacency` and joins the spine to the lit-node, relit-edge, and label sets on selection. Keying on the `contains` parent means tag / mention / language meta-nodes, which have no containment edge, get no spine with no explicit kind check.

### Validation

svelte-check 0/0/0; full workspace-app vitest green including new `graph/lensClosure.test.ts` (before/after: the `@@Alice` lens keeps 3 mention edges without the closure, 5 with it), `graphLensMetaClosure.test.ts` (arm wiring), and a `tabs.test.ts` case asserting a semantic workspace open seeds the root node id `""` as its pending selection (lens opens keep their own focal node; non-semantic modes and non-workspace scopes get none); the existing lens / spine / depth `?raw` tests stay green (the inline closure call fits their wildcard spans). No Rust touched. Browser-verified on a local `chan open --standalone` server over a nested `@@mention` + `#tag` fixture: the workspace Graph shortcut lands with the root `graph-smoke/` selected, its first-degree neighbourhood lit, and the inspector open on the workspace node; the `@@Alice` mention lens (opened via a `mention:@@Alice` graph link) lands with `@@Alice` selected, the inspector open, a single `mention=@@Alice` tab title, and every co-mentioning document plus its other handles and tags rendered. Desktop (WKWebView) not separately verified; graph scoping is pure JS and Blink-faithful, so it is on the rc validation list.

### Open items

Directory "Graph from here" still scopes to the expanded subtree, so a link from an in-subtree file to a collapsed nested or sibling-subtree file is not drawn until the user expands that directory or raises the depth slider. That is the directory lens's tree-and-expand model, distinct from the mention-lens bug fixed here; whether a directory lens should also pull in its 1-hop out-of-subtree link neighbours is a UX call left for the maintainer. Item B's markdown placeholder now overlaps semantically with the index-building branch ("graph temporarily unavailable while indexing the workspace"): both imply indexing, and the markdown fallback says so even when the index is idle. Only the markdown-mode wording was requested, so the index-building branch is left as is.
