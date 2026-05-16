# rustacean-5: chan graph CLI input validation lock-in

Owner: rustacean. Picked up after syseng noted two CLI residuals
during the hardening pass:

1. `chan graph --target ../etc/hosts` returned `1 nodes, 0 edges`
   with exit 0 instead of rejecting the `..`-escape.
2. `chan graph --target notes/no-such-file.md` returned
   `1 nodes, 0 edges` with exit 0 instead of an explicit
   "no such target" error.

Status: REVIEW.

## What was already in place

The architect's verification pass (before this task) had already
added `Drive::stat`-based validation inside `graph_scope_nodes`
(`crates/chan/src/main.rs`). The behavioral fix is therefore
present on `main`; rustacean-5 locks it in with focused tests so a
future refactor cannot silently drop the rejection.

Live probes confirm the existing behavior:

```
$ chan graph /tmp/chan-syseng-fixture --scope file --target ../etc/hosts
Error: stat graph file target `../etc/hosts`
Caused by: path escapes drive root          # exit 1

$ chan graph /tmp/chan-syseng-fixture --scope file --target notes/no-such-file.md
Error: stat graph file target `notes/no-such-file.md`
Caused by: io error: No such file or directory (os error 2)  # exit 1

$ chan graph /tmp/chan-syseng-fixture --scope folder --target ../etc
Error: stat graph folder target `../etc`
Caused by: path escapes drive root          # exit 1

$ chan graph /tmp/chan-syseng-fixture --scope file --target notes
Error: --scope file requires a file; `notes` is a directory  # exit 1
```

## Files changed

- `crates/chan/src/main.rs`
  - Added four tests under the existing `tests` module against
    `graph_scope_nodes` to pin escape / missing / wrong-type
    rejections.
- `crates/chan/Cargo.toml`
  - Added `tempfile` as a `dev-dependencies` entry. The graph
    tests need a real Library + Drive bootstrapped against a
    temp dir.

## Tests added (4)

- `graph_scope_file_rejects_escape_target` — `..`-escape on
  `--scope file`.
- `graph_scope_file_rejects_missing_target` — non-existent file
  on `--scope file`.
- `graph_scope_file_rejects_directory_target` — passing a
  directory to `--scope file`.
- `graph_scope_folder_rejects_escape_target` — `..`-escape on
  `--scope folder`.

## Verification

```
cargo fmt --all -- --check                # clean
cargo clippy --all-targets -- -D warnings # clean
cargo test -p chan                        # 43 passed (39 prior + 4 new)
```

## Residual / out of scope

- syseng residual 3 (`chan graph` does not query the new
  `/api/fs-graph` route for filesystem-graph queries) is a feature
  add, not a bug, and stays open as a follow-up to webdev-1's
  decision on whether to consume `/api/fs-graph` at all. If the
  decision is yes, the CLI can mirror with a `--fs-graph` flag in
  a future phase.
