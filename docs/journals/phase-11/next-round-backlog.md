# Phase 11 close: summary + next-round backlog

Round wrapped 2026-05-27. Next round starts fresh from this backlog.

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

### GI-10: graph layout - drive at the bottom, spine grows upward
@@Alex wants the DRIVE node pinned to the BOTTOM of the graph, pushing
other nodes upward, so the filesystem spine (dirs + their files) grows
bottom-up. Cytoscape layout change: a hierarchical/directed layout rooted
at the drive at the bottom, or a constraint pinning the drive node low +
repelling others upward. WEB-only, `web/src/components/GraphCanvas.svelte`
(+ GraphPanel layout config).

### Graph dead-ends / loading-state UX (graph-loading-state-spec.md)
Was held this round. Show a parent-dir loading/pulsing state while a scope
is still loading (mirror the File Browser expand spinner) instead of
rendering an incomplete graph as fact; genuinely-broken links shown
distinctly once a scope's index is complete. May need a per-scope
index-completeness signal from the backend (coordinate ownership).

### Systemic FS-test de-flake (OWNER: Lane B / test infra)
Status at wrap: IN PROGRESS on `phase-11-lane-b` (a shared `test_gate.rs`
serial-gate approach across chan-drive + chan-server). The FSEvents /
watcher / debounce / PTY timing tests flake under full parallel
`cargo test`; per-test serialization did not converge. If merged at close,
done; else carry forward. The bar: full default-parallel `cargo test`
deterministic over >=10 runs. This is the round-close-push CI gate.

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
