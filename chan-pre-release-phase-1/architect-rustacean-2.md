# architect-rustacean-2

Pre-seal sync from rustacean. The architect / syseng audit
(`architect-syseng-3.md`, `summary.md`) was written between the
chan-server-85 and chan-43 milestones rustacean delivered with
rustacean-4 and rustacean-5. Two small numbers / one residual need
folding into the seal artifacts.

## State drift to fold in

### Test counts

`summary.md` and `architect-syseng-3.md` both quote
`cargo test -p chan-server` at 78 and `cargo test -p chan` at 39.
Those were the rustacean-3 totals. With rustacean-4 and
rustacean-5 landed (still REVIEW, uncommitted):

```
cargo test -p chan-server                 # 89 passed
cargo test -p chan                        # 46 passed
cargo clippy --all-targets -- -D warnings # clean
cargo fmt --all -- --check                # clean
cargo build --no-default-features         # ok
```

Net +7 / +4 from the two new task files.

### Residual closed

`architect-syseng-3.md` "Residuals at seal" item 1 — "CLI sandbox
tightening" — is closed by `rustacean-5.md`. The behavior was
already present in `graph_scope_nodes` after the architect
verification pass; rustacean-5 added the lock-in tests:

- `graph_scope_file_rejects_escape_target`
- `graph_scope_file_rejects_missing_target`
- `graph_scope_file_rejects_directory_target`
- `graph_scope_folder_rejects_escape_target`

Live probes also confirm exit 1 + clear error messages for all
four cases. Item 1 can drop off the residual list before
`summary.md` is finalized.

### Residuals that stay

- 4: `target_is_inside_drive` lexical fallback — edge case on
  cloud-mounted submounts only; documented in
  `rustacean-2.md`'s residual risks. Defer.
- 5: assistant active-turn live smoke — needs an enabled assistant
  fixture, not rustacean's lane.
- The `chan config` assistant namespace was closed by later backend
  reconciliation: the CLI now covers the Settings-overlay assistant
  keys without exposing every low-level chan-llm tuning field.
The prior CLI fs-graph parity, `truncated: true` UI surfacing, and
`webdev-4` dispatch-table residuals were closed by later backend /
webdev / architect reconciliation.

## Commit ordering

architect-syseng-3.md proposes:

1. chan-core: chan-core-purge-1
2. chan: rustacean-1
3. chan: rustacean-2
4. chan: rustacean-3
5. chan: architect-syseng-2 fix (apply_watch_change + 7 tests)
6. chan: webdev-1..5 (bundle)
7. chan: phase records

This is fine. Rustacean's only addition: rustacean-5's four tests
land naturally with the rustacean-3 commit (same file
`crates/chan/src/main.rs`, same test module) or as its own slot
between (4) and (5). Splitting it out keeps the test-only
character clear; bundling it shrinks the PR count by one.

## What rustacean needs next

Nothing for seal. Idling. Will pick up additional Rust work if the
architect dispatches more before commits land.

rustacean.
