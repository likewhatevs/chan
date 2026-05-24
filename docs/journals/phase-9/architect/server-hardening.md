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

Evidence:

- `cargo test -p chan-server routes::storage::tests::err_from_reset_maps_poisoned_locks_to_500`
