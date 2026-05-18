# architect-rustacean-3

Pre-seal escape closed. Rustacean ran one more pass over the
fs-graph surface and found a real mid-path symlink escape that
slipped through both rustacean-2's tests and syseng-1's live
hardening pass (the syseng fixture didn't include an in-drive
symlink whose target was an outside DIRECTORY, only an outside
FILE).

## Bug

`GET /api/fs-graph?scope=file&path=alias/hosts` and
`scope=folder&path=alias/ssl` (where `alias` is an in-drive
symlink to `/etc/`) returned drive-relative ids carrying
`/etc/*` file metadata. No content, but absolute paths and stat
info crossed the sandbox.

Live repro lives in `rustacean-6.md`. Post-fix repro confirmed
both rejected with `BAD_REQUEST` and message
`path escapes drive root via mid-path symlink: <rel>`.

## Fix

`crates/chan-server/src/routes/fs_graph.rs`:
- New `ensure_parent_inside_drive` helper canonicalizes the parent
  of the joined request path and verifies it stays under the
  drive's canonical root.
- Called from `build_fs_graph` after the lexical `resolve_safe`
  step, before `symlink_metadata` ever touches the leaf.
- Leaf is still allowed to be a symlink — that path is documented
  and the walker classifies via `readlink`.

## Tests

- `build_fs_graph_rejects_mid_path_symlink_escape` (unix):
  asserts the BAD_REQUEST + message for both
  `scope=file&path=escape-link/hosts` and
  `scope=folder&path=escape-link/ssl`.
- `build_fs_graph_allows_in_drive_symlink_leaf_to_outside` (unix):
  asserts an in-drive `alias-outside.md -> /etc/hosts` still
  classifies as outside-drive ghost (no regression on the
  documented case).

## Gate

```
cargo fmt --all -- --check                # clean
cargo clippy --all-targets -- -D warnings # clean
cargo test -p chan-server                 # 92 passed
cargo test -p chan                        # 46 passed
cargo build --no-default-features         # ok
```

## Where this slots into the commit ordering

The fix lives in the same `crates/chan-server/src/routes/fs_graph.rs`
file as rustacean-2's original `/api/fs-graph` route. Two clean
options:

1. **Bundle into the rustacean-2 commit.** Keeps the route's
   sandbox story self-contained in one diff.
2. **Separate commit between rustacean-2 and the architect-
   syseng-2 fix.** Makes the regression discoverable in
   `git log`, similar to architect-syseng-2's recommended slot.

Rustacean's preference: option 2 — the escape is a security-
shaped fix that benefits from its own commit message even though
it sits next to existing code.

## Recommendation

Phase 1 is still sealable; this just adds one more line to the
seal checklist. Suggested updated commit ordering:

1. chan-core: chan-core-purge-1
2. chan: rustacean-1
3. chan: rustacean-2 (`/api/fs-graph` route)
4. chan: rustacean-6 (mid-path symlink escape close)  ← NEW
5. chan: rustacean-3 (CLI parity)
6. chan: architect-syseng-2 fix
7. chan: webdev bundle
8. chan: phase records

rustacean.
