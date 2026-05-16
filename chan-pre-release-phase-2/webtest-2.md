# webtest-2: Phase 2 review smoke

Owner: @@Webtest
Status: Ready for review

## Goal

Smoke the phase 2 browser-visible changes after frontend/backend review work
lands, using the shared service from [[chan-pre-release-phase-2/webtest-1.md]].

## Relevant Links

- [[chan-pre-release-phase-2/request.md]]
- [[chan-pre-release-phase-2/journal.md]]
- [[chan-pre-release-phase-2/backend-2.md]]
- [[chan-pre-release-phase-2/backend-3.md]]
- [[chan-pre-release-phase-2/frontend-1.md]]
- [[chan-pre-release-phase-2/frontend-2.md]]
- [[chan-pre-release-phase-2/frontend-3.md]]
- [[chan-pre-release-phase-2/frontend-4.md]]
- [[chan-pre-release-phase-2/frontend-5.md]]
- [[chan-pre-release-phase-2/frontend-6.md]]
- [[chan-pre-release-phase-2/frontend-7.md]]
- [[chan-pre-release-phase-2/frontend-8.md]]

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

Added [[chan-pre-release-phase-2/webtest-smoke.mjs]] for the phase 2 browser
checks that do not depend on the stale `language:TypeScript` fixture.

Verification:

- `node --check chan-pre-release-phase-2/webtest-smoke.mjs`: pass.
- `node chan-pre-release-phase-2/webtest-smoke.mjs`: pass.

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

Picked up [[chan-pre-release-phase-2/architect-8.md]] and
[[chan-pre-release-phase-2/frontend-9.md]].

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

- `node --check chan-pre-release-phase-2/webtest-smoke.mjs`: pass.
- `cd web && npm run check`: pass.
- `cd web && npm test -- --run depth`: pass, 1 file / 6 tests.
- `cd web && npm run build`: pass with existing Vite large-chunk and
  ineffective dynamic import warnings.
- `cargo build --release -p chan`: pass.
- Restarted 8788 with the rebuilt binary.

Expanded browser smoke:

- `node chan-pre-release-phase-2/webtest-smoke.mjs`: pass.
- Search layout + per-file rows: 21 unique paths.
- Search Status `Graph this` -> language graph: 1076x794 canvas.
- Graph depth caps: file scope max/value 1, group scope max/value 2, dir scope
  max/value 4 for the test fixture, drive scope disabled with max <= 6.
- Graph live mutation at 1440x1000 and 390x844: live add increased the open
  filesystem graph node count; delete-while-open rendered the selected semantic
  graph file as a ghost/missing node.

Rerun after adding the required narrow G1a/G4 pass:

- `node --check chan-pre-release-phase-2/webtest-smoke.mjs`: pass.
- `node chan-pre-release-phase-2/webtest-smoke.mjs`: pass.
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
