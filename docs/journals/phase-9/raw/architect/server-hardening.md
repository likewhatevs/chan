# Server Hardening

Date: 2026-05-24
Owner: @@Architect
Status: implementation notes for Phase 9

## Lock Poisoning

Phase 9 carries the phase-8 P2 follow-up to turn broader chan-server
`Mutex` / `RwLock` poisoning paths into explicit errors where they sit on HTTP
route paths.

2026-05-24: `POST /api/storage/reset` no longer panics on poisoned
`drive_cell` or `server_config` locks. The reset route maps those cases to a
500 response through its existing `ResetError` path, preserving the normal busy
and chan-drive error mappings.

2026-05-24: the first-party control socket no longer panics when the live
drive-cell lock is poisoned or unavailable. `chan open` style requests now get
a structured control-socket error instead of taking down the server task.

2026-05-24: the tunnel event task no longer unwraps the SPA prefix lock while
processing a Connected event. A poisoned prefix lock now logs a warning and
skips that event instead of unwinding the background task.

2026-05-24: session blob routes no longer run chan-drive session I/O on tokio
worker threads. `GET /api/session`, `PUT /api/session`, `DELETE /api/session`,
and `GET /api/sessions` now execute the synchronous drive calls behind
`spawn_blocking`, matching the file, search, graph, report, and inspector
route shape.

2026-05-24: route-facing indexer snapshots no longer unwrap the shared
drive-cell lock. Health, index status, indexing-state, and index-rebuild
routes now map poisoned or missing drive state to the normal 500 JSON error
shape. The MCP bridge also stops unwrapping the same lock when accepting a
new agent connection; if the drive state is unavailable it refuses that
session and keeps the bridge task alive.

2026-05-24: file create and delete routes no longer run synchronous drive I/O
on tokio worker threads. Target collision checks, directory creation, file
creation, and delete now run through blocking workers.

2026-05-24: verified the Codex MCP startup path against a throwaway chan
server and the current `codex` binary. The isolated `HOME` run is a harness
check only, so it can own a temporary Codex config without touching the real
profile. Product terminals still start in the selected drive or draft cwd
while preserving the user's normal `HOME`, which is what real agent sessions
should see.

Evidence:

- `cargo test -p chan-server routes::storage::tests::err_from_reset_maps_poisoned_locks_to_500`
- `cargo test -p chan-server control_socket::tests`
- `cargo test -p chan-server routes::sessions`
- `cargo test -p chan-server routes::files::write_tests`
- `cargo test -p chan-server routes::search`
- `cargo test -p chan-server routes::health`
- `cargo test -p chan-server state::test_support`
- `cargo test -p chan-server --lib`
- `cargo build -p chan`
- `codex exec` with `chan/list_files` over the current MCP bridge
