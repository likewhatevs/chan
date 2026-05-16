# webtest-1: Phase 1 web smoke and dev server ownership

Owner: webtest. Depends on: webdev-1, webdev-2, webdev-3. Unblocks:
final commit readiness.

## Goal

Own the web test server and run the end-to-end smoke pass for Phase 1 UI
changes.

## Responsibilities

- Start and maintain the Vite dev server for Alex/manual testing.
- Consolidate duplicate server-start requests from other agents.
- Record URL, backend URL/token assumptions, and restarts here.
- Smoke Graph this, Search dashboard, search navigation, assistant
  scrolling, bubble width, and thinking badge behavior.

## Acceptance criteria

1. Dev server URL is recorded.
2. Each UI feature has a short pass/fail note with browser/viewport.
3. Any crash/reload is logged with the triggering action.
4. Any blocker is filed back to architect as a new task.

## Verification

- `npm run check`
- `npm test -- --run`
- Browser smoke at desktop and narrow viewport.

## Done means

Update this file with smoke results, known gaps, and mark `webtest-1`
REVIEW in `journal.md`.

## 2026-05-16 10:39 Europe/London: server status

- Live URL: http://127.0.0.1:8788/
- PID: 2750
- Drive: `/tmp/chan-dev`
- Command: `CHAN_UPDATE_CHECK=0 target/release/chan serve /tmp/chan-dev --no-token --no-browser --port 8788`
- Port 8787 is already occupied by an existing host-shell `chan serve .`, so
  webtest is using 8788.
- Smoke: `GET /` returns 200 OK; `GET /api/health` returns `{"status":"ok"}`.
- Static check: `npm run check` passed with 0 errors / 0 warnings.
- Unit tests: `npm test -- --run` passed, 6 files / 94 tests.
- Startup needs access to `~/.chan` registry/state, so it runs outside the
  workspace sandbox.
- Detached `nohup` was reaped by the command runner; current service is held by
  the managed tool session.

## 2026-05-16 11:16 Europe/London: refreshed current tree

- Rebuilt frontend bundle with current webdev changes.
- Rebuilt `target/release/chan` from the current workspace.
- Restarted webtest service on the same URL: http://127.0.0.1:8788/
- Current PID: 17864

Verification:

- `cd web && npm run check`: pass, 0 errors / 0 warnings.
- `cd web && npm test -- --run`: pass, 6 files / 94 tests.
- `cd web && npm run build`: pass; Vite reports the existing large-chunk
  warnings.
- `cargo build --release -p chan`: pass.
- `GET /`: 200 OK.
- `GET /api/health`: `{"status":"ok"}`.
- `GET /api/fs-graph?scope=folder&path=&depth=1`: 200 OK with root node,
  direct children, `contains` edges, and `truncated:false`.
- `GET /api/report/prefix?path=`: 200 OK with whole-drive totals and
  per-language rollup.

Additional work performed:

- Closed the webdev-2 `language:<name>` search gap in
  `web/src/components/SearchPanel.svelte`.
- Marked `webdev-2` REVIEW in `journal.md`.

Remaining:

- Full browser/viewport smoke for graph context menu, Search Status overlay,
  assistant scrolling, and `language:<name>` search still needs an interactive
  browser pass. This file stays out of REVIEW until that pass is recorded.

## 2026-05-16 12:31 Europe/London: browser smoke

- Live URL: http://127.0.0.1:8788/
- Current managed session: restarted after rebuild; current service remains
  attached to the tool session.
- Added reproducible CDP smoke runner:
  `node chan-pre-release-phase-1/webtest-smoke.mjs`.

Verification:

- `cd web && npm run check`: pass, 0 errors / 0 warnings.
- `cd web && npm test -- --run`: pass, 6 files / 94 tests.
- `cd web && npm run build`: pass; Vite reports existing large-chunk and
  ineffective dynamic import warnings.
- `cargo build --release -p chan`: pass.
- `GET /api/health`: `{"status":"ok"}`.
- `node chan-pre-release-phase-1/webtest-smoke.mjs`: pass.

Browser smoke results:

- Desktop 1440x1000: `language:TypeScript` search returns 25 report-backed
  file rows with language/SLOC metadata; repeated ArrowDown keeps the active
  row visible.
- Desktop 1440x1000: Search Status opens from Search and renders Index plus
  Code Report/SLOC fields.
- Desktop 1440x1000: File Browser row context menu exposes `Graph this` and
  opens the Graph overlay.
- Narrow 390x844: `language:TypeScript` search returns 25 report-backed file
  rows and keeps the active row visible after repeated ArrowDown.
- Narrow 390x844: File Browser `Graph this` opens the Graph overlay.
- Assistant overlay active-turn smoke is skipped in this fixture because
  `/api/drive` reports `preferences.assistant.effective_enabled:false`.
  Scrolling, bubble width under real transcript content, and live thinking
  badge behavior still need a drive with an enabled assistant backend.

Issue found and fixed during smoke:

- `language:<name>` initially scanned only the lazy-loaded root tree entries.
  `SearchPanel.svelte` now hydrates lazy folder listings before scanning
  per-file report rows.

Status: REVIEW, with the assistant active-turn gap recorded above.

## 2026-05-16 12:31 Europe/London: static/HTTP smoke refresh

The webtest service is still reachable at http://127.0.0.1:8788/.

Current-tree verification:

- `cd web && npm run check`: pass, 0 errors / 0 warnings.
- `cd web && npm test -- --run`: pass, 6 files / 94 tests.
- `cd web && npm run build`: pass. Vite reports the existing large-chunk
  warnings and ineffective dynamic import warnings for CodeMirror language
  chunks.

HTTP smoke against the running webtest server:

- `GET /`: 200 OK.
- `GET /api/health`: `{"status":"ok"}`.
- `GET /api/index/status`: 200 OK, idle, 174 docs / 174 vectors.
- `GET /api/fs-graph?scope=folder&path=&depth=1`: 200 OK, direct root
  children and `contains` edges, `truncated:false`.
- `GET /api/report/prefix?path=`: 200 OK, whole-drive totals and
  per-language SLOC rollup.
- `GET /api/search/files?q=&limit=5`: 200 OK, file rows returned.

This is a non-interactive refresh of the already recorded CDP browser smoke
above. `webtest-1` remains REVIEW. The only browser gap left is assistant
active-turn behavior under a drive with an enabled assistant backend.

## 2026-05-16 12:41 Europe/London: filesystem graph smoke refresh

Rebuilt the current tree after `webdev-5`, restarted the webtest service on
http://127.0.0.1:8788/, and reran the tightened CDP browser smoke.

Verification:

- `cd web && npm run check`: pass, 0 errors / 0 warnings.
- `cd web && npm test -- --run`: pass, 6 files / 94 tests.
- `node --check chan-pre-release-phase-1/webtest-smoke.mjs`: pass.
- `cd web && npm run build`: pass; same existing Vite large-chunk /
  ineffective dynamic import warnings.
- `cargo build --release -p chan`: pass.
- `GET /api/health`: `{"status":"ok"}` after restart.
- `node chan-pre-release-phase-1/webtest-smoke.mjs`: pass.

CDP browser results:

- Desktop 1440x1000: `language:TypeScript` search, Search Status dashboard,
  and File Browser `Graph this` passed.
- Desktop 1440x1000: `Graph this` now specifically verifies filesystem graph
  mode/status rather than only checking that some graph overlay opened.
- Narrow 390x844: `language:TypeScript` search and filesystem `Graph this`
  passed.
- Assistant overlay remains skipped because `/tmp/chan-dev` has
  `preferences.assistant.effective_enabled:false`.

Status: REVIEW.
