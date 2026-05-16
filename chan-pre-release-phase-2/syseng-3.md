# syseng-3: Post-syseng-2 close-out — frontend-9 depth cap + lang-graph residual

Owner: @@Syseng. Status: REVIEW.

Pulled from [[chan-pre-release-phase-2/architect-syseng-1.md]] —
the "I'll re-run the hardening matrix once webtest-2 closes and
frontend-9 (depth cap) lands" item. [[chan-pre-release-phase-2/frontend-9.md]]
landed in REVIEW after syseng-2 closed, and the depth cap reuses
`/api/fs-graph` (the route I hardened in phase 1) for the drive /
global scope probe. Three things in scope:

1. Review the per-scope depth-cap derivation in
   `web/src/graph/depth.ts` against drive-shape semantics
   (symlink boundary, hardlink dedup behaviour, truncation
   propagation).
2. Land the optional empty-drive unit test for
   `build_language_graph` flagged as non-blocking residual in
   syseng-2.
3. Re-run the live probe matrix against the rebuilt fixture with
   the depth-cap path in the loop (`/api/fs-graph?scope=folder&path=&depth=6`).

## Acceptance criteria

1. depth.ts dir-scope max derivation does not count past a
   symlink-only sub-tree (because fs-graph itself doesn't traverse
   symlink targets), and folder counts match what
   `/api/fs-graph?scope=folder&path=<dir>&depth=6` returns for the
   same directory.
2. depth.ts drive-scope max derivation correctly reads `truncated:
   true` from the route and clamps to MAX_DEPTH=6.
3. `build_language_graph(&[], 0, None)` has a focused Rust unit test
   that asserts `{max_depth:0, nodes:[], edges:[]}`.
4. Hardening matrix re-run: every probe in syseng-2 re-passes, plus
   one new probe that hits `/api/fs-graph?scope=folder&path=&depth=6`
   on the fixture and matches the depth.ts driveScopeMax helper's
   expected output.
5. All cargo + web gates green; no regression from syseng-2's pass.

## Verification gate

```
cargo test -p chan-server build_language_graph
cd web && npm test -- --run graph/depth
cd web && npm run check
target/debug/chan serve <fixture> --no-token --no-browser --port 18801
curl 'http://127.0.0.1:18801/api/fs-graph?scope=folder&path=&depth=6'
```

## Status

REVIEW pending @@Architect ack. All three acceptance items closed:

1. frontend-9 depth.ts — approved (symlink boundary, hardlink
   neutrality, truncation semantics, bootstrap window, path-not-
   under-root all hold).
2. Empty-drive lang-graph test — already landed by @@Backend
   (`language_graph_empty_drive_returns_empty_payload`); verified.
3. Hardening matrix re-run — drive-scope `/api/fs-graph?path=&depth=6`
   probe behaves as depth.ts expects: deepest visible path is
   exactly at depth 6 on a fixture with on-disk depth 8, slider
   max correctly clamps to 6.

No follow-up. No new blockers.

## Review outputs

### frontend-9 depth.ts review

Read `web/src/graph/depth.ts` and `web/src/graph/depth.test.ts`.
Cross-checked against `crates/chan-server/src/routes/fs_graph.rs`.

Findings:

- **Symlink boundary holds.** `fs_graph::walk_dir` only descends
  into `child_meta.is_dir() && !child_meta.file_type().is_symlink()`.
  Symlinks at any depth surface as `kind: "symlink"` nodes but their
  targets are not traversed. depth.ts's `maxDepthFromPaths` counts
  segments of node paths verbatim, so a symlink to a deeper dir
  contributes only its own location, not the target's. The cap
  cannot leak depth from a symlink's target.
- **Hardlink dedup is irrelevant for the slider.** Two hardlinked
  paths contribute the same `(dev, ino)` to the walker's visited
  set, but the walker keys at the *directory* level only (and macOS
  rejects hardlinked dirs, so this guard is mostly belt-and-braces).
  For file hardlinks, both paths surface as separate file nodes;
  depth.ts counts whichever is deepest. No double-count failure
  mode because the deepest hardlink path is at most as deep as any
  single one.
- **Truncation semantics.** `truncated` is set when the node-count
  limit (`MAX_NODES`) is exceeded with depth still remaining, or
  not at all from depth exhaustion alone. So a drive deeper than
  `MAX_DEPTH=6` returns `truncated: false` with paths capped at
  depth 6. depth.ts then derives slider max from the visible paths
  (which max out at 6), giving slider max=6. That's the correct
  bound for the BFS the canvas can render against the route's
  capabilities. The `truncated: true` path (slider→fsMax) only
  triggers for node-count truncation, which still clamps to 6
  defensively. No misleading UX.
- **Path-not-under-root edge case.** `relativeDepth` returns 0
  when a path doesn't start with the prefix; `Math.max(max, 0)`
  keeps `max` at the running maximum (init 1). depth=1 baseline
  preserved.
- **Bootstrap window.** Drive/global scopes return `hardMax=10`
  when `fsGraph` is null (probe not loaded yet). Cap narrows to
  the real value once the probe lands. Single-shot probe is
  triggered in `GraphPanel.svelte:222` with `depth=FS_GRAPH_DEPTH_MAX=6`,
  matching the route's hard cap.
- **Test coverage.** `depth.test.ts` covers file/group/dir
  (content + fs-graph)/drive/global/tag/git_repo, plus truncated.
  Symlink boundary isn't explicitly tested but is enforced
  upstream by the fs-graph walker; adding a fake symlink node to
  the depth test would only re-prove that the walker's output is
  what depth.ts consumes.

Approved. No follow-up.

### Empty-drive lang-graph test

@@Backend landed this already as
`routes::graph::tests::language_graph_empty_drive_returns_empty_payload`
(graph.rs:1012-1019). Verified: asserts `max_depth=0`, empty nodes,
empty edges for `build_language_graph(&[], 0, None)`.

```
cargo test -p chan-server routes::graph::tests::language_graph_empty_drive_returns_empty_payload
test result: ok. 1 passed
```

Closed.

### Hardening matrix re-run

Fresh fixture at `/tmp/chan-syseng-phase3-fixture-1778945028`,
chan serve on port 18801, indexed_docs settled at 6 (5 .md +
1 .txt + 1 .c never indexed by design + 1 FIFO never indexed +
2 in-drive symlinks never indexed). Probes:

| Probe | Result |
|---|---|
| `GET /api/index/status` | `idle docs=6 vectors=6` |
| `GET /api/fs-graph?scope=folder&path=&depth=6` (drive depth probe) | `truncated:false`; 23 nodes; deepest folder `deep1/deep2/deep3/deep4/deep5/deep6` (depth 6); `deep7/buried.md` (depth 8) absent — clamped by route's MAX_DEPTH. depth.ts-equivalent max = 6 = `clampDepth(6, fsMax=6) = 6`. Slider would cap at 6. |
| `GET /api/graph` baseline | 5 file nodes (no missing); tag `#phase2` from `has-tag.md`; broken-link ghosts as expected |
| Symlink + special handling | `notes/alias-to-top.md` and `notes/broken-alias.md` as `symlink`; `notes/does-not-exist.md` as `ghost broken:true`; `attach/named.pipe` as `ghost` |
| `GET /api/graph/languages?depth=0` on multi-language fixture | language nodes for C, Markdown, Plain Text; folder ranks correct |

```
cargo test -p chan-server routes::graph::tests::language_graph_empty_drive_returns_empty_payload
                                          1 passed
cd web && npm test -- --run graph/depth   6 passed
```

No regression from syseng-2's pass.
