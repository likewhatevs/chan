# webtest-1: Phase 2 web test service ownership

Owner: @@Webtest
Status: IN PROGRESS

## Goal

Own the running web test service for [[chan-pre-release-phase-2/request.md]]
and provide a stable URL for Alex and the phase assistants.

## Relevant Links

- [[chan-pre-release-phase-2/request.md]]
- [[chan-pre-release-phase-1/summary.md]]
- [[chan-pre-release-phase-1/webtest-1.md]]
- [[chan-pre-release-phase-1/webtest-smoke.mjs]]

## Acceptance Criteria

- Live server URL, drive, and backend assumptions are recorded.
- Restarts, crashes, and reloads are logged here.
- Browser smoke failures are routed to the relevant owner task.
- Duplicate server requests are consolidated through this file.

## Test Expectations

- Keep `http://127.0.0.1:8788/` available unless another port is recorded here.
- Run the existing CDP smoke after major frontend/backend changes.
- Add or update smoke coverage when phase 2 work introduces new user-visible
  graph, search, editor, or overlay behavior.

## 2026-05-16 15:47 BST: startup baseline

- Live URL: http://127.0.0.1:8788/
- Drive: `/private/tmp/chan-dev`
- Existing listener: `chan` on TCP 127.0.0.1:8788.
- Backend health: `GET /api/health` returns `{"status":"ok"}`.
- Drive API: `GET /api/drive` returns `name:"chan-dev"` and
  `preferences.assistant.effective_enabled:true`.
- Index status: idle, 174 docs / 174 vectors,
  model `BAAI/bge-small-en-v1.5`.
- Code report: `GET /api/report/prefix?path=` returns whole-drive totals and
  per-language rollups, including TypeScript, Rust, Svelte, and Markdown.
- Static smoke script syntax: `node --check
  chan-pre-release-phase-1/webtest-smoke.mjs` passed.

Next step: run the existing CDP browser smoke against the live service.

## 2026-05-16 15:50 BST: baseline browser smoke

Initial full smoke command:

- `node chan-pre-release-phase-1/webtest-smoke.mjs`

Result:

- Search language + arrow scroll passed at 1440x1000.
- Search Status overlay passed at 1440x1000.
- File Browser `Graph this` passed at 1440x1000.
- Assistant smoke reached the pending state, then timed out waiting for
  `assistant smoke ok`.

Assessment:

- The shared 8788 service has assistant enabled through normal drive prefs, so
  the phase 1 assistant check is not deterministic here. It expects the fake
  Codex fixture output used by the isolated assistant smoke server in
  [[chan-pre-release-phase-1/webtest-1.md]].
- No server health regression observed after the failed assistant check.

Focused shared-service smoke:

- Command: `env CHAN_WEBTEST_ONLY=search,search-status,graph node
  chan-pre-release-phase-1/webtest-smoke.mjs`
- First sandboxed attempts could not reliably launch headless Chrome; reran
  with approval outside the process sandbox.
- Result: passed.

Browser smoke results:

- Desktop 1440x1000: `language:TypeScript` search returned 25 rows and active
  row navigation stayed visible.
- Desktop 1440x1000: Search Status opened from Search and rendered Code
  Report/SLOC fields.
- Desktop 1440x1000: File Browser `Graph this` opened the filesystem graph.
- Narrow 390x844: `language:TypeScript` search returned 25 rows and active
  row navigation stayed visible.
- Narrow 390x844: File Browser `Graph this` opened the filesystem graph.

Status: 8788 is live and ready for phase 2 shared web testing. Assistant
browser smoke should use an isolated fake-Codex fixture before being treated as
release evidence.

## 2026-05-16: test drive source seed

- Copied the current workspace source tree into
  `/tmp/chan-dev/Source/chan-workspace-copy`.
- This is a real directory copy inside the test drive, not a symlink, bind
  mount, or mapped external drive.
- Excluded large/generated/local directories: `.git`, `target`,
  `web/node_modules`, `web/dist`, `.claude`, `.svelte-kit`, `.vite`,
  `node_modules`, `dist`, and `.DS_Store`.
- Resulting copy size: 67 MB; `web/` subtree is 2.4 MB.
- Verified excluded paths are absent and `GET /api/health` still returns
  `{"status":"ok"}`.

## 2026-05-16: rebuilt shared service for review smoke

- Rebuilt the current phase 2 tree and restarted the shared service on
  http://127.0.0.1:8788/.
- Current listener PID from `lsof`: 5601.
- Command:
  `CHAN_UPDATE_CHECK=0 target/release/chan serve /tmp/chan-dev --no-token --no-browser --port 8788`.
- Health after restart: `GET /api/health` returns `{"status":"ok"}`.
- Review smoke results are recorded in [[chan-pre-release-phase-2/webtest-2.md]].

## 2026-05-16: report refresh and frontend-9 rebuild

- Applied the [[chan-pre-release-phase-2/architect-8.md]] report workaround by
  backing up the active stale report JSONL under
  `/Users/fiorix/Library/Application Support/chan/report/205463a154c706e7/`.
- Rebuilt frontend + release binary after [[chan-pre-release-phase-2/frontend-9.md]].
- Restarted the shared service on http://127.0.0.1:8788/.
- Current listener PID from `lsof`: 15857.
- Health after restart: `GET /api/health` returns `{"status":"ok"}`.
- Scratch path for destructive Webtest browser probes:
  `/tmp/chan-dev/Scratch/phase2-smoke/`.
- Expanded smoke results are recorded in [[chan-pre-release-phase-2/webtest-2.md]].

## 2026-05-16 16:41 BST: cycle close-out

- Final 8788 listener before tear-down: PID 15857.
- Stopped PID 15857 with SIGTERM.
- Confirmed `lsof -nP -iTCP:8788 -sTCP:LISTEN` returns no rows.
- Final green smoke state is recorded in [[chan-pre-release-phase-2/webtest-2.md]].
- `web/src/state/store.svelte.ts`, `web/src/components/GraphCanvas.svelte`,
  and `web/src/components/GraphPanel.svelte` type-gap fixes remain uncommitted
  intentionally for the architect phase commit with [[chan-pre-release-phase-2/frontend-8.md]].
- The scratch fixture directory `/tmp/chan-dev/Scratch/phase2-smoke/` was left
  in place for the next cycle.
