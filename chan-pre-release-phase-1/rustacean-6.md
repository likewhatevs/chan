# rustacean-6: Close fs-graph mid-path symlink escape

Owner: rustacean. Surfaced by rustacean's own pre-seal audit
after architect-syseng-3 closed its formal review.

Status: REVIEW.

## What was broken

`GET /api/fs-graph` and the `chan graph --scope file|folder` CLI
both call into `build_fs_graph`, which resolved the request path
through `chan_drive::fs_ops::resolve_safe`. `resolve_safe` is a
LEXICAL join: it rejects `..` traversal but it does not look at
the filesystem. If the drive contained an in-drive symlink whose
target landed outside the drive root, a request that used the
symlink as a mid-path component leaked outside contents.

Live repro (pre-fix, against a fixture drive with
`escape-link -> /etc`):

```
$ curl /api/fs-graph?scope=file&path=escape-link/hosts
{"nodes": [..., {"id": "escape-link/hosts", "kind": "file",
                 "size": 213, "mtime": 1778236090}, ...], ...}

$ curl /api/fs-graph?scope=folder&path=escape-link/ssl
{"nodes": [..., {"id": "escape-link/ssl/cert.pem", "kind": "file",
                 "size": 333483, ...}, ...]}
```

The wire response only reported drive-relative ids and file
metadata (no content). chan's threat model is loopback-only single-
user single-machine, so this is information disclosure rather than
content disclosure — but it's still a deviation from the documented
"all I/O sandboxed under drive root" contract, and syseng's
`design-snapshot.md` explicitly called out mid-path symlink
resolution as something the walker had to handle.

## Fix

Added `ensure_parent_inside_drive` in
`crates/chan-server/src/routes/fs_graph.rs` and called it from
`build_fs_graph` after the lexical `resolve_safe` step:

- For root-scope requests (`rel == ""`) or top-level leaves
  (`parent == drive_root`), skip the check (no parent traversal).
- Otherwise, canonicalize the drive root and the parent of the
  joined absolute path. If the parent's canonical form does not
  start with the drive root's canonical form, reject with
  `BAD_REQUEST` and the message
  `path escapes drive root via mid-path symlink: <rel>`.
- If parent canonicalization fails (parent dir does not exist),
  fall through — the caller's `symlink_metadata` will surface the
  standard `NOT_FOUND` for the leaf.
- If the drive root itself does not canonicalize, return
  `INTERNAL_SERVER_ERROR` rather than silently accepting.

Critically, the LEAF is still allowed to be a symlink. The walker
classifies symlink leaves via `readlink` without following them,
so an in-drive symlink to an outside file (e.g.
`alias-outside.md -> /etc/hosts`) still surfaces correctly as an
outside-drive ghost node. That's the documented behavior; the fix
only closes the case where the symlink appears MID-path.

## Tests

Added in `crates/chan-server/src/routes/fs_graph.rs::tests`:

- `build_fs_graph_rejects_mid_path_symlink_escape` (unix). Builds
  `<root>/escape-link -> /etc` and asserts both
  `scope=folder&path=escape-link/ssl` and
  `scope=file&path=escape-link/hosts` return
  `BAD_REQUEST` with the escape message.
- `build_fs_graph_allows_in_drive_symlink_leaf_to_outside` (unix).
  Builds `<root>/alias-outside.md -> /etc/hosts`, requests
  `scope=file&path=alias-outside.md`, and asserts the response
  contains the symlink node + an `outside:alias-outside.md` ghost
  node. The mid-path guard MUST NOT regress this case.

Total fs_graph module tests: 18 (was 15).

## Live verification

```
$ curl /api/fs-graph?scope=file&path=escape-link/hosts
{"error":"path escapes drive root via mid-path symlink: escape-link/hosts"}

$ curl /api/fs-graph?scope=folder&path=escape-link/ssl
{"error":"path escapes drive root via mid-path symlink: escape-link/ssl"}

$ curl /api/fs-graph?scope=folder&path=&depth=1
# Drive root listing still shows escape-link classified as a symlink
# pointing at /etc, with the outside-drive ghost target. No regression
# on the documented in-drive-symlink case.
```

## Verification gate

```
cargo fmt --all -- --check                # clean
cargo clippy --all-targets -- -D warnings # clean
cargo test -p chan-server                 # 92 passed (89 prior + 3 new)
cargo test -p chan                        # 46 passed
cargo build --no-default-features         # ok
```

## Residual

None on this surface. The `truncated: true` flag, the cloud-mounted
`canonicalize` fallback, and the chan-server's lexical-fallback
branch (only hit when `canonicalize` on the drive root itself fails)
are all already documented as residuals on `rustacean-2.md`. None of
them open the same escape — they only affect classification, not
sandbox enforcement.

## Files changed

- `crates/chan-server/src/routes/fs_graph.rs`
  - Added `ensure_parent_inside_drive` helper.
  - Called it from `build_fs_graph` before `symlink_metadata`.
  - Added 2 new tests for the escape rejection + leaf allowance.
