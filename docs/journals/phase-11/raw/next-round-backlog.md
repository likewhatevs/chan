# Phase 11 close: summary + next-round backlog

Round wrapped 2026-05-27. Next round starts fresh from this backlog.

## CONTINUATION close (2026-05-27): what landed + updated carryover

A continuation round ran from this backlog (lanes: @@LaneA graph, @@LaneC
release/CI). main advanced 85e6f15 -> 3ce94f0, ALL LOCAL (not pushed - the first
push fires the new `make ci-linux`/`ci-macos` CI over the whole round). Final
gate green: fmt/clippy/test (31 suites), web svelte-check 0/0 + vitest 1596/0,
web-marketing check.

LANDED:
- Terminal (0691dc9): WebGL renderer recreates on context loss (bounded retry) +
  per-retry [chan] console logging - was a one-way DOM downgrade.
- GI-9 (64225b9): fs-mode directory graphs render the full containment spine
  (scopedNodeIds null in fs-mode); fixed 0/N + dropped sibling subdirs.
- GI-8 (e61b8c4 C1, be05dae C2, a89f171): reveal (graph Show Directory/File,
  editor Reveal, search Show File, window-command) always opens a File Browser
  tab; dead GraphPanel OverlayShell branch removed -> OverlayShell now
  Search+Settings only.
- LaneC release contract (bd979bc, 96c9c17, 3ce94f0): chan upgrade + install.sh
  on /dl/cli metadata (vX.Y.Z + SHA256-from-metadata); root Makefile public
  surface + scripts/pre-push -> make pre-push; /dl metadata generator + verifier
  + site-consumes-/dl/releases.json (GitHub fallback); release CI with the
  release-cut gate (publish only on v* tag or workflow_dispatch publish=true;
  secret NAMES only).

NEXT-ROUND carryover (NEW + still-open; the GI-8/GI-9 sections below are DONE):
- OVERLAY/SCOPE-CONCEPT WIPE (the big graph item): `overlay-scope-wipe-spec.md`
  (W1-W7, design-resolved). Kill availableGraphScopes / "panes form scope";
  scope == filesystem directory (tag rootable depth-1, language rootable,
  file->parent, drop group/global/git_repo); GraphPanel graphState=tab; dock
  owns browserState; retire legacy ?graph=/?files= hash; delete the LOAD-BEARING
  graphOverlay/browserOverlay state (coupling documented in the spec).
- GI-10 (drive-at-bottom layout, GraphCanvas) - not started (detail below).
- Graph loading-state UX - not started (detail below + graph-loading-state-spec).
- GI-11 - confirmed a STALE-INDEX non-bug (both resolvers already normalize ../).
  Optional ../ / ./ link-target regression-locks, else drop.
- LaneC slice 5: Tauri updater UX (Check-for-Updates menu, prompt, signed
  payloads) -> /dl/desktop/latest.json + the Tauri dep bump (Cargo.lock - the one
  cross-lane seam, sequence vs graph work).
- LaneC slice 6: graph manual/site copy - waits on GI-10 + loading-state.

VERIFICATION GAPS for @@Alex in a real build:
- Editor "Reveal in browser" + search "Show File" -> FB tab (LaneA could not
  click live; thin wrappers, low risk).
- Terminal WebGL self-heal: background/foreground, display sleep, monitor switch;
  watch console for [chan] context-loss lines.

## What landed this round (on `main`)

Round-1 bugs (all done): list-input regression (1), desktop drag-removal +
native download (2), binary-size audit / lean release (3), New File
trailing-slash (4), image-paste-at-cursor (5), idle-terminal repaint (6),
Too-Many-Open-Files / autosave-hang via fd-budget pacing (7), desktop
auto-reload/hang (8), stuck reindex pill (9), Cmd+N cursor (10).

Features + structural:
- Partial-load CORE: drive bootstrap spine (unified ignore rules, fs walk
  -> tree + counts), per-directory watcher pub/sub (single recursive OS
  watcher + logical refcounted scope filters), paced index/report jobs.
- File Browser: per-instance metadata + watcher subscribe/unsubscribe.
- Graph: gradual load, depth slider, edge coloring; GI-1..7 inspector
  hotfixes (Open/Show-File/Show-Directory/Graph-from-here/depth, false
  broken-links).
- Inspector consistency + layout (actions section, FB/Graph parity).
- Image-drag-across-rows feature.
- FB capabilities: multi-select, clipboard (cmd+C/X/V), multi-move DnD,
  `Drive::copy` + `POST /api/fs/transfer`, Finder-style collision policy.
- Ignore-set consistency: unified WalkFilter applied to index + both
  graphs + report + trash + watcher walks (was leaking node_modules/target
  into the graph -> 60K-131K-node runaway, now fixed).
- GPU embedding defaults to CPU (Metal hang); GPU opt-in.
- macOS CLI->desktop handoff (Option B + C fallback), incl. a launch-crash
  fix (listener was panicking outside the Tauri tokio runtime).
- Indexing benchmark: structural index ~2-2.7s; chan-report ~doubles E2E.

## Follow-up tasks for next round (OWNER: graph/web = Lane A unless noted)

### GI-8: "Show Directory" reloads the graph (should open a File Browser tab)
GI-5 fixed the no-op, but clicking Show Directory on a dir node now RELOADS
THE GRAPH instead of opening/focusing a File Browser tab at that directory.
Same reactivity-reload class as GI-1/GI-2 (the reveal triggers a
currentScope recompute -> reload $effect); apply the untracked /
stable-scope-key reveal pattern to `revealSelectedFsEntry`. Expected: open
a File Browser tab at the directory (like Show File reveals a file).
WEB-only, `web/src/components/GraphPanel.svelte`.

### GI-9: filesystem graph omits most subdirectories at depth
Scope=`agents/`, depth 2 shows only `orchestration/` as a subdir, while the
File Browser shows ~10 siblings (architect/, ci/, desktacean/, desktect/,
desktest/, fullstack-a/, fullstack-b/, systacean/, webtest-a/, webtest-b/)
at the same level that the graph does NOT plot. The status bar read
"27/47 nodes" - ~20 nodes present in data but not rendered, so likely a
FRONTEND render/fetch cap (the backend fs_graph walker DOES follow
`contains` parent->child to depth). Per round-1 every subdir must hang off
its parent via a grey dir->dir containment edge, and increasing depth must
reveal the next layer for ALL branches, not just link-related ones.
Investigate: frontend render cap / kind-filter (graphData / GraphPanel)
vs the fs-graph endpoint node cap. Files: `web/src/state/graphData.svelte.ts`,
`GraphPanel.svelte`, `crates/chan-server/src/routes/fs_graph.rs`.

FRAMING (@@Alex is right): for the filesystem SPINE this should be the
SAME logic as File Browser expand/collapse - round-1 explicitly said "plot
their depth, similarly to expand/collapse of the File Browser, reusing the
same pub/sub mechanism". The bug is a DIVERGENCE: the graph has a separate
fs walk (fs_graph.rs) that anchors on linked/semantic nodes and/or hits a
render cap, dropping containment-only subdirs - while the FB shows them
all. FIX DIRECTION: make the graph's spine expansion reuse the SAME
containment walk the File Browser uses (all ignore-filtered children to
depth), then layer the semantic edges (links/tags/contacts/language) on
top of that complete spine. The semantic overlay + node dedup + spatial
layout + global caps are the only legitimate extra concerns vs the FB
tree; none justify dropping subdirs.

### GI-10: graph layout - drive at the bottom, spine grows upward
@@Alex wants the DRIVE node pinned to the BOTTOM of the graph, pushing
other nodes upward, so the filesystem spine (dirs + their files) grows
bottom-up. Cytoscape layout change: a hierarchical/directed layout rooted
at the drive at the bottom, or a constraint pinning the drive node low +
repelling others upward. WEB-only, `web/src/components/GraphCanvas.svelte`
(+ GraphPanel layout config).

### GI-11: false broken-links from `../` parent-relative markdown links
GI-3 (Lane A `d35b852`) fixed wiki-link ancestor-walk resolution but NOT
markdown links with `../` parent-relative paths. @@Alex hit it in the
inspector: a node `journals/phase-8/phase-7/next-phase-backlog.md` shown as
"file does not exist (broken-link target)". CONFIRMED FALSE - the real
target `docs/journals/phase-7/next-phase-backlog.md` EXISTS, and
`docs/journals/phase-8/{request,process}.md` link it as
`[...](../phase-7/next-phase-backlog.md)`. The resolver joins the relative
path onto the SOURCE dir (`journals/phase-8/`) WITHOUT canonicalizing the
`../`, yielding the malformed `journals/phase-8/phase-7/...`. Fix: normalize
`../` and `./` segments in link-target resolution relative to the source
doc's directory (clamped to the drive root). Same area as GI-3:
`crates/chan-server/src/routes/graph.rs` link resolution (and confirm the
inspector existence-check uses the normalized path). Add tests for `../`,
`./`, and multi-`../` cases.

### Graph dead-ends / loading-state UX (graph-loading-state-spec.md)
Was held this round. Show a parent-dir loading/pulsing state while a scope
is still loading (mirror the File Browser expand spinner) instead of
rendering an incomplete graph as fact; genuinely-broken links shown
distinctly once a scope's index is complete. May need a per-scope
index-completeness signal from the backend (coordinate ownership).

### Systemic FS-test de-flake (OWNER: Lane B / test infra) - MERGED
MERGED at close: `88e196f` -> main `88ea5c3`. One CROSS-PROCESS OS
file-lock gate (`crates/chan-drive/src/test_gate.rs`) serializes the whole
FSEvents/indexer/debounce/PTY test class across BOTH crates' separate test
binaries (per-binary in-process mutexes can't serialize across binaries -
that's why the earlier attempts didn't converge). fmt/clippy/build green;
gate mechanism proven (815ms cross-process contention experiment).
OUTSTANDING: the full 10x parallel sweep was UNMEASURABLE locally because
macOS FSEvents is WEDGED machine-wide on this box (standalone notify probe
delivers 0 events -> the 4 real-watcher tests fail deterministically,
independent of this gate). Next round: confirm the 10x parallel sweep is
green either on CI (Linux/inotify, no FSEvents) or locally after fseventsd
recovers (`sudo killall fseventsd`; launchd restarts it).

### macOS FSEvents wedged on this machine (transient, environmental)
A standalone `notify` probe delivers ZERO events; any chan watcher / live
reload / file-change detection is dead until fseventsd is restarted
(`sudo killall fseventsd`). Likely induced by the heavy watcher/serve churn
this round (incl. the repo-root 60K-node indexing run). NOT a code bug -
flagged so local manual testing accounts for it.

### Deferred (unchanged)
- Manual/site streaming copy (docs/manual + web-marketing) - was deferred
  behind the partial-load rework, now mostly unblocked; write against the
  final graph behavior once GI-8/9/10 + loading-state settle.
- Linux desktop launch - run on a Linux machine (lima + sdme), still open.
- macOS handoff WINDOW-PAINT visual check - @@Alex to confirm in a real
  desktop build (`chan serve <drive>` while chan-desktop runs -> a native
  window should appear); the socket->open path is verified via logs.
- GPU embedding proper fix (gpu-embed-followup.md) - timeout + CPU fallback
  or correct Metal command-buffer usage.
- Linux inotify-watch-count follow-up (with Linux desktop).

## Reference docs
graph-inspector-bugs.md (GI-1..7), inspector-spec.md, fb-capabilities-spec.md,
graph-loading-state-spec.md, watcher-scalability.md, ignore-consistency-spec.md,
new-file-and-draft-spec.md, gpu-embed-followup.md, the lane plans + journals.
