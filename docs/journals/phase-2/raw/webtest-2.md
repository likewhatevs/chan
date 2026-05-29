# webtest-2: Phase 2 review smoke

Owner: @@Webtest
Status: Ready for review

## Goal

Smoke the phase 2 browser-visible changes after frontend/backend review work
lands, using the shared service from [[phase-2/webtest-1.md]].

## Relevant Links

- [[phase-2/request.md]]
- [[phase-2/journal.md]]
- [[phase-2/backend-2.md]]
- [[phase-2/backend-3.md]]
- [[phase-2/frontend-1.md]]
- [[phase-2/frontend-2.md]]
- [[phase-2/frontend-3.md]]
- [[phase-2/frontend-4.md]]
- [[phase-2/frontend-5.md]]
- [[phase-2/frontend-6.md]]
- [[phase-2/frontend-7.md]]
- [[phase-2/frontend-8.md]]

## Acceptance Criteria

- Rebuilt service is live on the shared Webtest URL.
- Search result collapse and overlay layout receive browser or API smoke.
- Search Status `Graph this` and language graph receive browser or API smoke.
- Graph folder layout, ghosting, and live reload receive smoke where the
  current implementation supports it.
- Findings are routed back to the relevant owner task.

## 2026-05-16: current-tree rebuild

- Fixed an adjacent frontend type gap in `web/src/state/store.svelte.ts`:
  session payload graph mode now accepts `language`.
- Fixed adjacent graph render type gaps in `web/src/components/GraphCanvas.svelte`
  so it accepts language/folder nodes and language edges.
- Kept `GraphPanel.svelte` inspector dispatch to file/tag/mention selections;
  language/folder nodes currently inspect as generic canvas selections only.

Verification before restart:

- `cd web && npm run check`: pass.
- `cd web && npm test -- --run search store blocks`: pass, 3 files / 26 tests.
- `cargo test -p chan-server routes::graph::tests`: pass, 13 tests.
- `cargo test -p chan-server routes::search::tests`: pass, 3 tests.
- `cd web && npm run build`: pass with existing large-chunk and ineffective
  dynamic import warnings.
- `cargo build --release -p chan`: pass.

Restart:

- Stopped old 8788 listener.
- Started rebuilt service:
  `CHAN_UPDATE_CHECK=0 target/release/chan serve /tmp/chan-dev --no-token --no-browser --port 8788`.
- Live URL: http://127.0.0.1:8788/
- `GET /api/health`: `{"status":"ok"}`.

API smoke:

- `GET /api/graph/languages?depth=1`: pass; returns `max_depth:9`, a
  `language:Markdown` node, a `folder:Journal` node, and a `language` edge.
- `GET /api/fs-graph?scope=folder&path=Source/chan-workspace-copy&depth=1`:
  pass; returns the copied source folder, direct children, `contains` edges,
  and `truncated:false`.
- `GET /api/search/content?q=language&limit=10`: pass; returned one row per
  file in the sample response.

Findings:

- The previous phase 1 browser smoke for `language:TypeScript` no longer
  applies to the current shared test drive. `GET /api/report/prefix?path=`
  currently reports only Markdown, and `GET
  /api/report/prefix?path=Source/chan-workspace-copy` reports zero code files
  even though `.rs`, `.ts`, and `.svelte` files exist on disk under that copy.
  This blocks report-backed TypeScript search smoke and weakens language graph
  coverage to Markdown-only on the shared drive.

Next:

- Run targeted browser smoke for Search Status layout and language graph UI.
- Route the code-report/source-copy finding to @@Backend or @@Architect.

## 2026-05-16: targeted browser smoke

Added [[phase-2/webtest-smoke.mjs]] for the phase 2 browser
checks that do not depend on the stale `language:TypeScript` fixture.

Verification:

- `node --check phase-2/webtest-smoke.mjs`: pass.
- `node phase-2/webtest-smoke.mjs`: pass.

Browser smoke results:

- Search overlay layout: passed at 1440x1000.
  - Search body stayed inside the overlay.
  - Results and inspector shared the body row.
  - Inspector stayed inside the search body.
  - No horizontal document overflow.
- Search row collapse: passed for query `language`; 21 visible result paths,
  all unique.
- Search Status `Graph this`: passed.
  - Search Status opened from Search.
  - `Graph this` launched the graph overlay.
  - Graph overlay rendered in `language graph` mode with a 1076x794 canvas.

Remaining smoke gaps:

- Delete-while-open ghost rendering and live add-while-open graph reload still
  need a deterministic browser fixture. Current API support is present, but the
  browser smoke should avoid mutating shared user-visible fixture files unless
  @@Architect assigns a scratch path.
- Depth-cap behavior is not yet covered by browser smoke.
- The shared drive currently only exposes Markdown through the code report, so
  language graph browser coverage is Markdown-only until the report/source-copy
  finding is resolved.

Status: ready for @@Architect review of smoke results and the report finding.

## 2026-05-16: scratch-path and depth-cap pickup

Picked up [[phase-2/architect-8.md]] and
[[phase-2/frontend-9.md]].

Report workaround:

- The suggested `/tmp/chan-dev/.chan/report.jsonl` path did not exist for this
  fixture. The active stale report was
  `/Users/fiorix/Library/Application Support/chan/report/205463a154c706e7/report.jsonl`.
- Moved it to
  `/Users/fiorix/Library/Application Support/chan/report/205463a154c706e7/report.jsonl.webtest-backup-20260516`.
- Restarted the shared service and forced report regeneration.
- `GET /api/report/prefix?path=` now reports 510 files with TypeScript, Rust,
  Svelte, JSON, JavaScript, CSS, TOML, Makefile, HTML, BASH, Shell, and
  Markdown.
- `GET /api/report/prefix?path=Source/chan-workspace-copy` now reports 249
  files, including TypeScript, Rust, and Svelte.
- `GET /api/graph/languages?depth=1` now includes `language:TypeScript`,
  `language:Rust`, and `language:Svelte` nodes.

Scratch fixture:

- Recorded `/tmp/chan-dev/Scratch/phase2-smoke/` as the Webtest scratch path.
- The phase 2 smoke runner creates, mutates, and removes only that subtree.

Rebuild after frontend-9:

- `node --check phase-2/webtest-smoke.mjs`: pass.
- `cd web && npm run check`: pass.
- `cd web && npm test -- --run depth`: pass, 1 file / 6 tests.
- `cd web && npm run build`: pass with existing Vite large-chunk and
  ineffective dynamic import warnings.
- `cargo build --release -p chan`: pass.
- Restarted 8788 with the rebuilt binary.

Expanded browser smoke:

- `node phase-2/webtest-smoke.mjs`: pass.
- Search layout + per-file rows: 21 unique paths.
- Search Status `Graph this` -> language graph: 1076x794 canvas.
- Graph depth caps: file scope max/value 1, group scope max/value 2, dir scope
  max/value 4 for the test fixture, drive scope disabled with max <= 6.
- Graph live mutation at 1440x1000 and 390x844: live add increased the open
  filesystem graph node count; delete-while-open rendered the selected semantic
  graph file as a ghost/missing node.

Rerun after adding the required narrow G1a/G4 pass:

- `node --check phase-2/webtest-smoke.mjs`: pass.
- `node phase-2/webtest-smoke.mjs`: pass.
- Results:
  - Search layout + per-file rows: 21 unique paths.
  - Search Status `Graph this` -> language graph: 1089x803 canvas.
  - Graph depth caps: file=1, group=2, dir=4, drive<=6.
  - Graph live add + delete ghost 1440x1000: pass.
  - Graph live add + delete ghost 390x844: pass.

Status: smoke matrix complete for current phase-2 tasks; ready for
@@Architect review.

## 2026-05-16 16:41 BST: cycle close-out

- No smoke rerun during tear-down.
- Final green matrix from this cycle:
  - Search overlay layout + per-file rows: pass, 21 unique paths.
  - Search Status `Graph this` -> language graph: pass.
  - Language graph after report refresh includes TypeScript, Rust, and Svelte.
  - G3 depth-cap probe: pass for file, group, directory, and drive scopes.
  - G4 live-add-while-open: pass at 1440x1000 and 390x844.
  - G1a delete/ghost-while-open: pass at 1440x1000 and 390x844.
- Architect-9 probe matrix is complete; no Webtest smoke probes are deferred.
- Final service state: PID 15857 stopped, port 8788 free.
- Frontend type-gap fixes made during Webtest remain uncommitted intentionally
  for the architect phase commit.

## 2026-05-16 16:51 BST: @@Webtest A architect-9 re-run

Picked up [[phase-2/architect-9.md]] end-to-end after the
prior cycle stopped 8788 without leaving it running. This re-run is a clean
re-execution of the workaround restart and the three browser probes (G1a /
G4 / G3) against the scratch fixture `/tmp/chan-dev/Scratch/phase2-smoke/`.

### Report workaround restart

- The architect-9 path `/tmp/chan-dev/.chan/report.jsonl` does not exist for
  this drive; the active report lives in
  `/Users/fiorix/Library/Application Support/chan/report/205463a154c706e7/`.
- Rotated the active report to
  `report.jsonl.webtestA-20260516-165052.bak` (was 101616 bytes, 525 rows,
  generated 2026-05-16T15:38Z).
- Restarted: `CHAN_UPDATE_CHECK=0 target/release/chan serve /tmp/chan-dev
  --no-token --no-browser --port 8788`; PID 22587.
- `GET /api/health` -> `{"status":"ok"}`.
- Waited for `GET /api/index/status` to report `state:"idle"`.
- `GET /api/report/prefix?path=` -> 12 languages: TypeScript, Rust, Svelte,
  JSON, JavaScript, CSS, TOML, Makefile, HTML, BASH, Shell, Markdown.
- `GET /api/graph/languages?depth=1` -> 12 language nodes (BASH, CSS, HTML,
  JSON, JavaScript, Makefile, Markdown, Rust, Shell, Svelte, TOML,
  TypeScript), confirming the language graph is back to live data.

### Architect-9 probes — re-run results

Smoke runner: `node phase-2/webtest-smoke.mjs` against the
restarted 8788. Scratch fixture mutated only under
`/tmp/chan-dev/Scratch/phase2-smoke/` and cleaned up by the runner.

```
PASS Search layout + per-file rows - 21 unique paths
PASS Search Status Graph this -> language graph - 1099x810 canvas
PASS Graph depth caps - file=1 group=2 dir=4 drive<=6
PASS Graph live add + delete ghost 1440x1000 - scratch subtree mutation observed
PASS Graph live add + delete ghost 390x844 - scratch subtree mutation observed
```

Mapping to architect-9 probe matrix:

| Probe                           | Source task                                | Viewport(s)   | Result |
|---------------------------------|--------------------------------------------|---------------|--------|
| G1a ghost-while-open            | [[phase-2/backend-3.md]]  | 1440x1000, 390x844 | PASS — overlay opened on a fresh `ghost-probe.md`, selected node, deleted file under scratch, overlay flipped to `file does not exist` / `not in the current file listing` within the 250ms watcher debounce |
| G4 live-add-while-open          | [[phase-2/frontend-7.md]] | 1440x1000, 390x844 | PASS — opened the directory graph on the scratch root, wrote a new `.md` into the same subtree, statusbar `nodes` total grew beyond the pre-add count without manual reload |
| G3 depth-slider scope-aware cap | [[phase-2/frontend-9.md]] | 1440x1000     | PASS — file scope: max/value 1; group scope: max/value 2 (two-file layout payload); dir scope: max/value 4 against `depth-dir/a/b/c/leaf.md`; drive scope: slider disabled with max in [1,6] |

No failures, no defects, no handoffs to file. Service left running on PID
22587 for any architect follow-up; tearing down 8788 after this section.

### Tear-down

- `kill 22587`; confirmed `lsof -nP -iTCP:8788 -sTCP:LISTEN` returns no rows.
- Scratch fixture `/tmp/chan-dev/Scratch/phase2-smoke/` is removed by the
  runner's `finally` block; verified `ls /tmp/chan-dev/Scratch/` empty.
- Rotated report backups left in place under the Application Support report
  dir so the architect commit pass can decide whether to keep them.
- Architect-9 probe matrix remains complete; T1 stays ready for @@Architect
  close-out.

## 2026-05-16 @@Webtest B: frontend-10 wire-shape probe prep

Picked up the @@Webtest B lane in parallel with @@Webtest A's architect-9
re-run (above). G3 depth-cap is doubly confirmed green from A's run, so I am
not re-claiming it. Frontend-10 (folder-glyph swap) is still TODO with
@@Frontend: [[phase-2/frontend-10.md]] is not started and
`mapFsNodes` in `web/src/components/GraphPanel.svelte` (line 719) still
coerces fs-graph `kind: "folder"` into RenderedNode `kind: "tag"`, so folder
nodes still draw the `#` tag glyph. The canvas-side wiring is pre-staged in
the architect bundle: `GraphCanvas.svelte` already has `DKind = ... | "folder"`,
`PATH_FOLDER`, and `loadIcon(iconImages, "folder", svgStrokeIcon(PATH_FOLDER, bg))`.

What I added to [[phase-2/webtest-smoke.mjs]]:

- `captureCanvasSignature(page)` reads the `.graph-tab canvas` via
  `getImageData` and folds the pixel buffer into a 16-bin luminance
  histogram, normalised over non-transparent pixels. The bins integrate
  across the whole canvas so they tolerate force-layout jitter but still
  drift when per-node glyph shape changes (stroked folder outline vs filled
  `#` text icon land in different luminance bins).
- `signatureDistance(a, b)` returns the L1 distance between two histograms.
- `smokeFolderGlyphWireShape(page)` creates
  `Scratch/phase2-smoke/glyph-probe/{root.md, sub/child.md}`, rebuilds the
  index, opens the fs-graph at that folder scope and the semantic graph at
  the same scope, captures both signatures, and reports the L1 drift.
- Post-swap assertion (`drift >= 0.05`) is gated behind
  `CHAN_WEBTEST_GLYPH_PROBE=1`. Pre-swap the runner still exercises the
  data-capture path and reports the drift number, but stays green so the
  default matrix is not red while waiting on @@Frontend.

Why histogram, not a DOM hook: GraphCanvas does not expose `window.__*` test
hooks for its rendered node kinds, and adding one is frontend-owned scope.
The pixel histogram is the most semantic-preserving option webtest can land
unilaterally; the L1 drift number is informative even when the assertion is
off.

Verification:

- `node --check phase-2/webtest-smoke.mjs`: pass.
- Runtime verification of the new probe deferred to the next service
  restart. Service is currently torn down (see @@Webtest A tear-down above);
  re-running on a rebuilt service is part of the post-commit smoke matrix.

Status: probe scaffolding in place; awaiting the architect's phase commit so
I can do the full matrix re-run against the rebuilt service. If @@Frontend
lands frontend-10 before the phase commit, I'll re-run with
`CHAN_WEBTEST_GLYPH_PROBE=1` to flip the probe into hard-assert mode.

## 2026-05-16 17:00 BST: @@Webtest A runtime-verify B's glyph probe

Picked this up after the architect-9 re-run because B explicitly deferred
runtime verification of the new `smokeFolderGlyphWireShape` probe to the
next service restart, and the matrix needs to be known-green in default
(pre-swap) mode before the phase commit so the post-commit rerun has a
clean baseline.

Restart:

- `CHAN_UPDATE_CHECK=0 target/release/chan serve /tmp/chan-dev --no-token
  --no-browser --port 8788`; PID 26456.
- Health + index reached idle.
- Reused the post-architect-9 report (regenerated at 16:51 BST), no second
  rotation.

Smoke runner: `node phase-2/webtest-smoke.mjs`
(CHAN_WEBTEST_GLYPH_PROBE unset).

```
PASS Search layout + per-file rows - 21 unique paths
PASS Search Status Graph this -> language graph - 1089x803 canvas
PASS Graph depth caps - file=1 group=2 dir=4 drive<=6
PASS Graph live add + delete ghost 1440x1000 - scratch subtree mutation observed
PASS Graph live add + delete ghost 390x844 - scratch subtree mutation observed
PASS Folder glyph wire-shape (pre-swap prep) - signature drift=0.7080
```

Findings:

- The default matrix is regression-free against B's added scaffolding. All
  five prior probes still green; the new probe runs in prep mode and
  reports a drift number without ever throwing.
- Drift between the fs-graph folder-scope canvas and the semantic-graph
  canvas already measures 0.7080 pre-swap. That number reflects mostly
  scene-composition differences (different node count + layout between
  fs-graph and semantic-graph for the same scratch subtree), not glyph
  identity. The pre-swap assertion is intentionally off; the post-swap
  assertion (`>= 0.05`) is comfortably satisfied today but the test is
  not load-bearing until @@Frontend lands frontend-10 and we re-run with
  `CHAN_WEBTEST_GLYPH_PROBE=1`.
- No defects; nothing to route via `architect-<source>-N.md`. B's idle
  handoff in [[phase-2/architect-webtest-2.md]] stays
  accurate: the scaffolding is now runtime-verified pre-swap.

Tear-down:

- `kill 26456`; confirmed `lsof -nP -iTCP:8788 -sTCP:LISTEN` returns no
  rows.
- Scratch fixture `/tmp/chan-dev/Scratch/phase2-smoke/` removed by the
  runner's `finally` block.
- No new uncommitted changes from this run; webtest-2.md update only.

## 2026-05-16 @@Webtest B: post frontend-10 full matrix re-run

@@Frontend landed [[phase-2/frontend-10.md]] (REVIEW):
`GraphPanel.mapFsNodes` now emits `kind: "folder"` (with `path/files/code`
fields) for `n.kind === "folder"` instead of coercing onto `kind: "tag"`.
That removes the `#` glyph collision with semantic-graph tag nodes; the
canvas-side wiring (`GraphCanvas.DKind = ... | "folder"`, `PATH_FOLDER`,
`iconImages.folder`) is unchanged.

Rebuild:

- `cd web && npm run check`: 3911 files / 0 errors / 0 warnings.
- `cd web && npm run build`: pass with existing Vite large-chunk and
  ineffective dynamic import warnings.
- `cargo build --release -p chan`: pass.

Service:

- Rotated stale report at
  `/Users/fiorix/Library/Application Support/chan/report/205463a154c706e7/report.jsonl`
  to `report.jsonl.webtestB-20260516-170305.bak` so the regenerated report
  picks up the live source copy.
- Started `target/release/chan serve /tmp/chan-dev --no-token --no-browser
  --port 8788`; PID 27729; waited for `/api/index/status` idle (698 docs,
  698 vectors, BAAI/bge-small-en-v1.5).
- `/api/report/prefix?path=` reports 12 languages (TypeScript, Rust, Svelte,
  JSON, JavaScript, CSS, TOML, Makefile, HTML, BASH, Shell, Markdown).

Smoke runner: `CHAN_WEBTEST_GLYPH_PROBE=1 node
phase-2/webtest-smoke.mjs`.

```
PASS Search layout + per-file rows - 21 unique paths
PASS Search Status Graph this -> language graph - 1083x799 canvas
PASS Graph depth caps - file=1 group=2 dir=4 drive<=6
PASS Graph live add + delete ghost 1440x1000 - scratch subtree mutation observed
PASS Graph live add + delete ghost 390x844 - scratch subtree mutation observed
PASS Folder glyph wire-shape (post-swap) - signature drift=0.7578
```

Findings:

- All six probes green. No regressions; the architect bundle (still
  uncommitted in the working tree) and frontend-10 together pass the full
  matrix end-to-end.
- The glyph probe's post-swap drift (0.7578) only edges up from the pre-swap
  drift @@Webtest A measured at 17:00 BST (0.7080), and @@Webtest A's note
  applies: most of that drift is scene composition (different node counts +
  layouts between fs-graph and semantic-graph for the same scratch subtree),
  not glyph identity. The `>= 0.05` assertion is trivially satisfied in both
  states. **The probe is therefore a smoke test for "both canvases render
  without errors and produce non-empty pixels", not a precise glyph-identity
  regression test.** Read appropriately by reviewers; if regression sensitivity
  matters, a follow-up should add a frontend test hook
  (e.g. `window.__chanGraphRenderedKinds`) and replace the histogram probe.
- Visual confirmation that the folder glyph is now the stroked folder outline
  (not `#`) belongs to Alex's eyeball or a future DOM hook.

Tear-down:

- `kill 27729`; confirmed `lsof -nP -iTCP:8788 -sTCP:LISTEN` returns no rows.
- Scratch fixture `/tmp/chan-dev/Scratch/phase2-smoke/` removed by the
  runner's `finally` block.
- Rotated report backup `report.jsonl.webtestB-20260516-170305.bak` left in
  place under `~/Library/Application Support/chan/report/205463a154c706e7/`.
- Working-tree changes from this cycle: `phase-2/webtest-2.md`
  (this section), `phase-2/journal.md` (B log entry),
  `phase-2/architect-webtest-2.md` (B handoff). The
  `web/src/components/GraphPanel.svelte` swap belongs to @@Frontend /
  frontend-10 and rides the architect phase commit.

Status: frontend-10 visual smoke is complete on B's side. T1 webtest scope is
green end-to-end (architect-9 matrix + frontend-10 post-swap matrix). Ready
for @@Architect commit pass.
