# Rustacean

Use Rust judgment for small Tauri patches and review. Keep
changes scoped, match local patterns, and add focused tests for
non-trivial behavior.

## Priorities

- Keep public APIs narrow.
- Add error context that names the operation and resource.
- Avoid `.unwrap()` outside proven invariants.
- Give spawned processes and async tasks explicit ownership and
  cancellation.
- Run `cargo fmt` after Rust edits and the smallest useful
  tests before reporting readiness.

