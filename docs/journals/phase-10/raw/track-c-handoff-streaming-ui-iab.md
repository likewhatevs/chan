# Track C handoff: streaming inspector and graph UI

Date: 2026-05-25
Owner: Track C / IAB agent
Source: Track A API streaming pass

## Goal

Use the new NDJSON relationship streams in the browser UI and smoke them with
the in-app browser against a repo-sized drive.

## API contracts

- `GET /api/report/file?path=<rel>&stream=1`
  - events: `meta`, `report` or `missing`, `done`
  - late failures: `error`
- `GET /api/backlinks/<rel>?stream=1`
  - events: `meta`, zero or more `edge`, `done`
  - late failures: `error`
- `GET /api/graph?scope=drive|directory|file&path=<rel>&depth=<n>&stream=1`
  - events: `meta`, batched `nodes`, batched `edges`, `done`
  - late failures: `error`
  - `nodes` batches are upserts keyed by node id. Later batches can refine
    fields such as report buckets after slower report work completes.
  - `edges` batches are additions. Consumers should dedupe by source, target,
    kind, and rank when replaying a stream after reload.

All three stream routes run blocking report and graph work off the Tokio
runtime. Closing the HTTP response drops the mpsc receiver; the blocking worker
stops on the next attempted send.

## UI tasks

- Update `web/src/api/client.ts` with typed NDJSON readers for report-file,
  backlinks, and graph streams.
- Update `FileInfoBody.svelte`:
  - report section uses the streaming report route;
  - backlinks section appends `edge` events as they arrive;
  - reference counts keep showing partial state instead of a 10 second hard
    timeout;
  - Reload cancels the old reader and starts a fresh stream.
- Update `graphData.svelte.ts` and `GraphPanel.svelte`:
  - apply `nodes` as id-keyed upserts;
  - append and dedupe `edges`;
  - expose loading counts so Graph Panel can draw partial results before
    `done`;
  - keep invalidation semantics from watcher events.
- Keep the old JSON client path only if it falls out naturally. This codebase is
  pre-release, so do not spend time preserving obsolete caller behavior.

## IAB smoke

1. Build and serve the current repo as a drive with `--no-token --no-browser`.
2. Open `CHANGELOG.md` in the editor.
3. Confirm editor content appears before the full file stream completes and
   editing is disabled until full load.
4. Open the inspector for `CHANGELOG.md`.
5. Confirm report, references, and backlinks show loading or partial state
   without a 10 second timeout.
6. Open Graph from the same file.
7. Confirm nodes and edges appear before the graph stream reaches `done`.
8. Trigger Reload in the inspector and graph UI, then confirm a fresh stream
   starts and the partial state resets cleanly.

## API smoke already covered by Track A

- Unit coverage asserts NDJSON order for report, backlinks, and graph streams.
- API-level curl smoke should hit the three `?stream=1` routes and verify:
  - status 200;
  - `Content-Type: application/x-ndjson`;
  - first event is `meta`;
  - final event is `done` on a healthy fixture.
