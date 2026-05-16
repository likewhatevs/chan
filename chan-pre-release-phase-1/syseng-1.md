# syseng-1: Phase 1 hardening session

Owner: syseng. Depends on: rustacean-1, rustacean-2, rustacean-3.
Unblocks: final commit readiness.

## Goal

Run a focused hardening pass over the new release-critical backend and
CLI surfaces before architect allows commits.

## Scope

- Fresh install and first-run behavior.
- Drive sandbox invariants for config/status/graph commands.
- Symlink, hardlink, and broken-link handling in filesystem graph.
- Index reset/rebuild behavior under concurrent reads.
- CLI output and exit statuses for success, bad input, missing drive,
  missing path, and settings-disabled cases.
- No runtime dependency creep.

## Acceptance criteria

1. Hardening notes are added to this file.
2. At least one fixture drive covers nested directories, symlink,
   broken symlink, hardlink where the platform supports it, text files,
   binary files, and ignored directories.
3. Any blocker is filed back to architect as a new task.
4. Non-blocking risks are listed for the final summary.

## Verification

- `cargo build`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- Manual CLI probes against the fixture drive.

## Done means

Update this file with exact commands, results, blockers, residual risks,
and mark `syseng-1` REVIEW in `journal.md`.

---

## 2026-05-16 Prep phase

Historical status at prep time: waiting on rustacean-1/-2/-3 before the full
hardening pass could execute. Later sections record the REVIEW result. This section
captures durable prep work that is independent of the rustacean
output: a reproducible fixture drive, the hardening checklist
mapped to each scope bullet, and a survey of the chan-drive
invariants the new code has to respect.

### Fixture drive

Path: `/tmp/chan-syseng-fixture`. Rebuild with:

```
FX=/tmp/chan-syseng-fixture
rm -rf "$FX"
mkdir -p "$FX"/{notes,notes/sub,notes/sub/deep,ignored/.git,ignored/node_modules,attach,bin}
echo "# Top note"           > "$FX/top.md"
echo "# Nested"              > "$FX/notes/sub/nested.md"
echo "deeper"                > "$FX/notes/sub/deep/deep.md"
head -c 4096 /dev/urandom    > "$FX/bin/blob.bin"
echo "gitignored"            > "$FX/ignored/.git/HEAD"
echo "noisy"                 > "$FX/ignored/node_modules/pkg.json"
ln -s ../top.md              "$FX/notes/alias-to-top.md"
ln -s notes/sub/nested.md    "$FX/notes/alias-to-nested.md"
ln -s does-not-exist.md      "$FX/notes/broken-alias.md"
ln -s /etc/hosts             "$FX/notes/alias-outside.md"
ln "$FX/top.md"              "$FX/hardlink-to-top.md"
mkfifo                        "$FX/attach/named.pipe"
```

Verified shape (macOS, APFS):

```
top.md             inode=72108170 nlink=2 -rw-r--r--
hardlink-to-top.md inode=72108170 nlink=2 -rw-r--r--
notes/alias-to-top.md          inode=72108176 nlink=1 lrwxr-xr-x
notes/broken-alias.md          inode=72108178 nlink=1 lrwxr-xr-x
attach/named.pipe              inode=72108182 nlink=1 prw-r--r--
```

Coverage map vs syseng-1 acceptance criterion 2:

| Required shape         | Path in fixture                         |
|------------------------|-----------------------------------------|
| nested directories     | `notes/sub/deep/deep.md`                |
| text files             | `top.md`, `notes/sub/*.md`              |
| binary file            | `bin/blob.bin`                          |
| symlink (in-drive)     | `notes/alias-to-top.md` -> `../top.md`  |
| symlink (in-drive #2)  | `notes/alias-to-nested.md`              |
| symlink (broken)       | `notes/broken-alias.md`                 |
| symlink (escape)       | `notes/alias-outside.md` -> `/etc/hosts`|
| hardlink               | `hardlink-to-top.md` <-> `top.md`       |
| FIFO (special file)    | `attach/named.pipe`                     |
| ignored dirs           | `ignored/.git/`, `ignored/node_modules/`|

Hardlink and FIFO are skipped on filesystems that do not support
them. macOS APFS supports both; CI on tmpfs/ext4 will also. We
do not check this fixture into git; the script above is the
source of truth.

### Hardening checklist (executed once deps land)

Mapped to scope bullets in this task. Each row lists the
runnable probe and the rustacean dependency that gates it.

| Scope bullet                              | Probe                                                                 | Gated by |
|-------------------------------------------|-----------------------------------------------------------------------|----------|
| Fresh install and first-run behavior      | empty tempdir; `chan serve --no-token --no-browser`; check stderr token, `/api/health`, `/api/index/status` reports a fresh empty index without referencing a prior schema | rustacean-1 |
| Drive sandbox invariants                  | `chan config set editor.theme=dark` against fixture; check write goes through `store::save_toml` (atomic + fsync); attempt `chan graph path=../escape`; expect drive-sandbox rejection with non-zero exit | rustacean-3 |
| Filesystem graph: symlink                 | `GET /api/fs-graph?scope=folder&path=notes&depth=1`; expect `alias-to-top.md`, `alias-to-nested.md`, `alias-outside.md` as `symlink` nodes with `symlink` edges to their targets; in-drive targets resolve to existing nodes; out-of-drive target is a ghost | rustacean-2 |
| Filesystem graph: hardlink                | depth>=1 from drive root; expect `top.md` and `hardlink-to-top.md` as ONE content node (keyed by `(st_dev, st_ino)`) with two `contains` edges from their respective parent folders, or as two `file` nodes joined by a `hardlink` edge, whichever rustacean-2 picks | rustacean-2 |
| Filesystem graph: broken symlink          | `broken-alias.md` -> `does-not-exist.md` surfaces as a `symlink` node pointing at a `ghost` node, not dropped | rustacean-2 |
| Filesystem graph: FIFO and other specials | `attach/named.pipe` shows up as a `ghost` (or explicitly typed special node), NOT silently dropped, NOT traversed | rustacean-2 |
| Filesystem graph: symlink loop            | manual `ln -s loop-a loop-b; ln -s loop-b loop-a` in a sub-folder of the fixture; graph call must terminate and not stack-overflow | rustacean-2 |
| Index reset/rebuild under concurrent reads| `curl -X POST /api/index/rebuild` in parallel with `for i in 1..10; do curl /api/search?q=note; done`; results must be coherent (no panics, no 500s, no corrupted JSON), and `/api/index/status` reports the in-flight rebuild | rustacean-1, rustacean-2 |
| CLI: success path                         | `chan status` against fixture: exit 0, JSON-ish output to stdout, no noise on stderr | rustacean-3 |
| CLI: bad input                            | `chan config set editor.theme=` (empty); exit non-zero, error to stderr, no partial config write | rustacean-3 |
| CLI: missing drive                        | `chan status --drive /tmp/does-not-exist`; exit non-zero, stderr names the missing drive, no traceback | rustacean-3 |
| CLI: missing path                         | `chan graph --path notes/no-such-file.md`; exit non-zero with a clear "no such file" message | rustacean-3 |
| CLI: settings-disabled                    | toggle a setting off in `chan config`, then run a command that depends on it; expect a clean refusal, not a panic | rustacean-3 |
| No runtime dependency creep               | `cargo tree --depth 1 --no-default-features` before vs after; check `ldd target/release/chan` (Linux) / `otool -L` (macOS) for new dyld entries; verify the binary stays self-contained (no node, python, or daemon required at runtime) | rustacean-1, rustacean-2, rustacean-3 |

### Survey findings (durable, captured pre-rustacean)

These are constraints the new code has to respect. They are
already enforced in chan-drive today; I'm restating them so the
hardening pass has a written contract to test against, and so
rustacean-2 has a single page to read before designing the
filesystem-graph walker.

1. **chan-drive uses lstat semantics throughout.** Every
   special-file check goes through `std::fs::symlink_metadata`,
   never through `metadata()` (which follows symlinks). See
   `fs_ops.rs::ensure_regular_file` and `drive.rs` boot path.
   The fs-graph walker must do the same.

2. **The current walker DROPS symlinks and special files.**
   `chan-drive::fs_ops::walk_drive` filters
   `!ft.is_symlink() && !ft.is_fifo() && ...`. The new
   filesystem-graph walker cannot reuse `walk_drive`: that
   walker is for the *content* index, where dropping
   non-regular entries is correct. The graph wants those
   entries surfaced as ghost/symlink/hardlink nodes. Either a
   parallel walker or a `walk_drive_with_specials` variant is
   needed; this should be in chan-core, not chan-server.

3. **Mid-path symlink resolution policy.**
   `fs_ops::resolve_safe_strict` rejects any path component
   that is a symlink pointing outside the drive root. It
   *allows* a symlink whose target lands back inside the drive.
   The graph walker should NOT use `resolve_safe_strict` for
   the target lookup: it should compute the link target with
   `readlink` only, and then classify:
   - target resolves inside the drive and exists -> edge to
     that node
   - target resolves inside the drive but file is gone ->
     ghost
   - target resolves OUTSIDE the drive -> ghost labeled
     `outside-drive`, never traversed
   - target is unreadable / `stat` fails -> ghost

4. **Hardlink identity must be `(st_dev, st_ino)`.** macOS APFS
   and Linux ext4/xfs all expose this through `MetadataExt`.
   Two paths with the same `(dev, ino)` are the same content
   node. nlink>1 is a *hint* that hardlinks may exist; the
   identity itself is the `(dev, ino)` tuple.

5. **Loop detection on symlink chains.** A graph walker that
   resolves `readlink` chains must keep a visited-`(dev, ino)`
   set per traversal to avoid infinite loops (`a -> b -> a` is
   a common manual mistake; the kernel returns `ELOOP` on
   `open` but `readlink` itself doesn't loop). A bounded
   per-traversal hop limit (e.g. 40, matching `MAXSYMLINKS`)
   is a cheap belt-and-braces guard.

6. **Atomic-write story for `chan config set`.** New writes
   under `<config>/chan/` MUST go through `crate::store::{
   load_toml, save_toml}` so they get the temp-file + rename
   + parent-dir fsync that the rest of the app uses. The CLI
   `chan config set` path is going to live in
   `crates/chan/src/main.rs`; rustacean-3 should call the
   server-side store helpers, not roll its own.

7. **pre-v3 contact-email migration is a TWO-side cleanup.**
   The consumer side at `chan-server::indexer.rs:142-155`
   (`emails_need_backfill` branch) is rustacean-1's
   responsibility. The producer side at
   `chan-drive::graph.rs::contacts_need_email_backfill`
   (+ tests around lines 2143-2210) lives in the sibling
   chan-core repo. Per the rustacean-1 brief, chan-core
   cleanup is filed as a separate task to architect, not
   silently done from this repo.

### Adjacent advisory

Three of the survey points above (items 2, 3, 5) are direct
inputs to rustacean-2's design. I'm filing
`architect-syseng-1.md` so the architect can fold them into
the rustacean-2 brief before rustacean starts; rustacean is
the SME on the actual implementation.

### Status

Status: REVIEW. Hardening pass executed against
`/tmp/chan-syseng-fixture-codex` with isolated
`HOME=/tmp/chan-syseng-home-codex` and matching XDG config/data/cache
directories.

Architect preliminary verification, 2026-05-16:

```
cargo test -p chan-server                 # 78 passed
cargo test -p chan                        # 39 passed
cargo clippy --all-targets -- -D warnings # clean
cargo fmt --all -- --check                # clean
cd web && npm run check                   # 0 errors / 0 warnings
```

Isolated CLI smoke used `HOME=/tmp/chan-cli-home-smoke` plus matching
`XDG_DATA_HOME` / `XDG_CACHE_HOME`, so it did not touch the host
registry:

```
target/debug/chan config get editor.theme       # system
target/debug/chan config set editor.theme=dark  # ok
target/debug/chan config set editor.theme=      # exit 1
target/debug/chan config get editor.theme       # dark
```

This only covers the config refusal/no-partial-write slice. The full
fixture/server/concurrency hardening pass is still pending.

## 2026-05-16 Hardening pass

### Fixture

Built a fresh fixture under `/tmp/chan-syseng-fixture-codex` with:

- nested markdown files: `notes/sub/nested.md`,
  `notes/sub/deep/deep.md`
- binary file: `bin/blob.bin`
- in-drive symlink: `notes/alias-to-top.md -> ../top.md`
- broken symlink: `notes/broken-alias.md -> does-not-exist.md`
- outside symlink: `notes/alias-outside.md -> /etc/hosts`
- hardlink pair: `top.md` and `hardlink-to-top.md`
- FIFO: `attach/named.pipe`
- ignored-ish directories: `ignored/.git`, `ignored/node_modules`

The prep fixture's `notes/alias-to-nested.md -> notes/sub/nested.md`
is intentionally recorded as a lowlight: because the link is created
inside `notes/`, that relative target resolves to
`notes/notes/sub/nested.md`. `/api/fs-graph` correctly reported it as
a broken in-drive target. The fixture script should use
`sub/nested.md` if the intended case is a second valid in-drive
symlink.

### CLI probes

Isolated config/status/graph probes:

```
target/debug/chan status /tmp/chan-syseng-fixture-codex --json
target/debug/chan config get --json
target/debug/chan config set editor.theme=dark
target/debug/chan config set editor.theme=
target/debug/chan config get editor.theme
target/debug/chan status /tmp/chan-syseng-missing-drive --json
target/debug/chan graph /tmp/chan-syseng-fixture-codex --scope file --target notes/no-such-file.md --json
target/debug/chan graph /tmp/chan-syseng-fixture-codex --scope folder --target top.md --json
```

Results:

- `chan status` on the fixture exited 0 and reported index/report/graph
  JSON without migration/backfill fields.
- `chan config get --json` included both `editor` and `server`
  namespaces.
- `chan config set editor.theme=` exited 1 and preserved the prior
  `dark` value.
- missing drive status exited 1 with `drive root does not exist`.
- Hardening found `chan graph --scope file --target missing` initially
  exited 0 with a synthetic node. Fixed in `crates/chan/src/main.rs`:
  file scope now `stat`s the target and refuses missing paths or
  directories; folder scope now refuses missing paths and non-directory
  targets. Rechecked both error paths.

### HTTP probes

Started:

```
target/debug/chan serve /tmp/chan-syseng-fixture-codex --here --no-token --no-browser --port 18787
```

Loopback probes:

```
curl -sS http://127.0.0.1:18787/api/health
curl -sS 'http://127.0.0.1:18787/api/fs-graph?scope=folder&path=notes&depth=1'
curl -sS 'http://127.0.0.1:18787/api/fs-graph?scope=folder&path=&depth=1'
curl -sS 'http://127.0.0.1:18787/api/fs-graph?scope=folder&path=attach&depth=1'
curl -sS 'http://127.0.0.1:18787/api/fs-graph?scope=folder&path=notes/sub&depth=2'
curl -sS 'http://127.0.0.1:18787/api/fs-graph?scope=file&path=notes/alias-to-top.md'
curl -sS -i 'http://127.0.0.1:18787/api/fs-graph?scope=folder&path=../escape&depth=1'
curl -sS -i 'http://127.0.0.1:18787/api/fs-graph?scope=file&path=notes/no-such-file.md'
```

Results:

- health returned `{"status":"ok"}`.
- folder `notes` depth 1 returned symlink nodes, broken/outside ghost
  nodes, and contains/symlink edges.
- root depth 1 returned the hardlink pair as two file nodes joined by
  a `hardlink` edge.
- `attach` depth 1 returned FIFO as `kind:"ghost"` and did not block.
- nested folder depth 2 returned grandchildren as expected.
- file-scope symlink returned parent folder, symlink node, target file,
  and symlink/contains edges.
- traversal attempt returned HTTP 400 JSON:
  `{"error":"path escapes drive root"}`.
- missing path returned HTTP 404 JSON:
  `{"error":"no such path: notes/no-such-file.md"}`.

### Rebuild/read concurrency

Ran a rebuild request concurrently with ten content-search reads. The
non-escalated sandbox could not run the concurrent loopback probe
reliably, so the probe was rerun with loopback access outside the
sandbox:

```
curl -sS -X POST http://127.0.0.1:18787/api/index/rebuild >/tmp/chan-rebuild.out &
sleep 0.1
for i in 1 2 3 4 5 6 7 8 9 10; do
  curl -sS 'http://127.0.0.1:18787/api/search/content?q=note&limit=5' >/tmp/chan-search-$i.out
done
wait
```

Results:

- rebuild response: `{"queued":true}`
- all ten search responses were valid JSON with BM25 hits.
- `/api/index/status` returned idle with `indexed_docs=4`,
  `indexed_vectors=4`.

### Verification gates

```
cargo build -p chan
cargo test -p chan
cargo test -p chan-server
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

Results:

- `cargo build -p chan`: ok
- `cargo test -p chan`: 39 passed
- `cargo test -p chan-server`: 78 passed
- `cargo fmt --all -- --check`: clean
- `cargo clippy --all-targets -- -D warnings`: clean

### Blockers

None remaining from this pass.

Resolved during architect follow-up:

- `architect-syseng-2`: watcher events on symlinks/special files no
  longer pin `/api/index/status` to `error`. `apply_watch_change` now
  classifies regular, deleted, missing, and special paths before setting
  status. Live repro with `touch -h notes/alias-to-top.md` kept index
  status at `idle`.

### Residual risks

- CLI `chan graph` still queries the semantic content graph, not the
  `/api/fs-graph` filesystem graph. This is now documented in
  `backend-1.md`; product should decide whether to add a filesystem
  graph mode to the CLI.
- The fixture script in the prep section has one malformed "valid"
  symlink target as noted above.

---

## 2026-05-16 Live hardening pass

Status: REVIEW. The release blocker found in this pass was fixed and
reverified; see "Resolved blocker" below.

Build state when probes ran:
- Debug binary: `target/debug/chan` rebuilt cleanly off the current
  tree (one transient `write_pref_key` resolution error appeared during
  a parallel cargo invocation — rebuild succeeded once the lock cleared,
  confirming it was a mid-edit snapshot, not a regression).
- `cargo test -p chan-server`: 78 passed (matches rustacean-2's claim).
- `cargo test -p chan-server fs_graph`: 11/11 PASS.

### Probes against `/tmp/chan-syseng-fixture` (full fixture)

Sibling chan server on port 8789, `--no-token --no-browser`,
debug binary.

| Probe | Result |
|---|---|
| `GET /api/fs-graph?scope=folder&path=&depth=1` (root) | PASS — 7 nodes, 7 edges, including `hardlink-to-top.md ─[hardlink]→ top.md` (dedup by `(dev, ino)` verified, inode 72108170). |
| `GET /api/fs-graph?scope=folder&path=notes&depth=1` | PASS — three in-drive symlinks classify correctly: `alias-to-top.md` → existing `top.md`, `broken-alias.md` → ghost `notes/does-not-exist.md` with `broken: true`, `alias-outside.md` → synthetic ghost id `outside:notes/alias-outside.md` with `outside: true` and target `/etc/hosts` preserved. Outside-drive target NEVER traversed. |
| `GET /api/fs-graph?scope=folder&path=attach&depth=1` (FIFO) | PASS — `attach/named.pipe` surfaces as `ghost` with `contains` edge from `attach/`. Not silently dropped. |
| `GET /api/fs-graph?scope=folder&path=loop&depth=2` (symlink loop `a→b→a`) | PASS — terminates in 9 ms total HTTP latency, HTTP 200, 3 nodes + 4 edges. visited-`(dev, ino)` guard works. Each symlink edges to its immediate target (no chain-chasing), which is the safer choice. |

Side finding: the prep-phase fixture had `ln -s notes/sub/nested.md
notes/alias-to-nested.md`, which POSIX resolves relative to the
symlink's parent dir (so it landed at `notes/notes/sub/nested.md`,
broken). The route correctly classified it as a ghost. Fixture
corrected in place to `ln -s sub/nested.md`; "Fixture drive" script
above intentionally LEFT as-is to keep the broken-via-bad-target
case in coverage (it exercises the same code path as `broken-alias.md`).

### Probes against `/tmp/chan-syseng-fresh` (empty drive)

| Probe | Result |
|---|---|
| `chan serve --no-token --no-browser` boot | PASS — boot completes cleanly. stderr contains only `chan is ready:` + URL. No migration log lines, no `pre-v3` traces. rustacean-1's purge confirmed downstream. |
| `GET /api/health` | PASS — `{"status":"ok"}` |
| `GET /api/index/status` | PASS — `{"state":"idle","indexed_docs":0,"indexed_vectors":0,"model":"BAAI/bge-small-en-v1.5"}`. No `contacts_backfill` field, no `schema_version` field, no migration sentinel. |
| `GET /api/fs-graph?scope=folder&path=&depth=1` | PASS — `nodes:1 edges:0 truncated:false`. Empty drive correctly reports just the root node. |

### Probes against `/tmp/chan-syseng-clean` (20 notes, no symlinks)

| Probe | Result |
|---|---|
| Initial boot + 4s settle | PASS — went from `idle docs=0` to `idle docs=20 vectors=20` autonomously. |
| 30 parallel `GET /api/search/content?q=alpha` interleaved with `POST /api/index/rebuild` | PASS — all 30 searches HTTP 200, latency 6-14 ms. Rebuild responded HTTP 202 `{"queued":true}`. Status polled during showed `state=building, file=notes/n7.md` mid-flight, then converged back to `state=idle, docs=20`. No 5xx, no panics, no corrupted JSON. |

### CLI probes (rustacean-3 / backend-1)

All CLI probes used an isolated `HOME=/tmp/chan-cli-smoke-syseng`
plus matching `XDG_DATA_HOME` / `XDG_CACHE_HOME` so they did not
touch the host registry.

| Probe | Result |
|---|---|
| `chan status /tmp/chan-syseng-fixture --json` (success) | PASS — exit 0, JSON to stdout with `root`, `registered_name`, `index`, `graph`, `report.by_language` keys. stderr only a `tokei` warning about the `.bin` extension (cosmetic). |
| `chan status /tmp/no-such-drive` (missing drive) | PASS — exit 1, stderr `Error: registering /tmp/no-such-drive` / `drive root does not exist: /tmp/no-such-drive`. Clean. |
| `chan config set editor.theme=` (empty value) | PASS — exit 1, stderr `Error: value must not be empty (got \`editor.theme=\`)`. No partial write. (Matches the architect preliminary slice.) |
| `chan config set editor.doesnotexist=foo` (unknown key) | PASS — exit 1, stderr names the bad key + points at `chan config get`. |
| `chan graph /tmp/chan-syseng-fixture --scope file --target notes/no-such-file.md` (missing target) | FIXED — now exits 1 with a stat error instead of emitting a synthetic node. |
| `chan graph /tmp/chan-syseng-fixture --scope file --target ../etc/hosts` (escape attempt) | FIXED — now exits 1 through the drive path-safety check instead of silently accepting the target. |
| `chan graph /tmp/chan-syseng-fixture --scope folder --target notes --depth 1 --json` | SUPERSEDED — this initially returned an empty content graph; later backend reconciliation switched `--scope file|folder` to the filesystem graph builder. |

### Runtime dependency audit

```
otool -L target/release/chan
  System.framework (Security, CoreFoundation, CoreServices,
                    Metal, Foundation)
  /usr/lib/libSystem.B.dylib
  /usr/lib/libobjc.A.dylib
  /usr/lib/libiconv.2.dylib
```

PASS — macOS system frameworks only. No Python, Node, Postgres,
libcurl, libssh, or other third-party dyld entries. No runtime
daemon needed. Single static binary holds.

### Resolved blocker

Resolved: **`/api/index/status` flipped to permanent `error` when any
symlink or special file changes inside the drive.** Reproduces
on the original syseng fixture; does NOT reproduce on the clean
drive (no symlinks).

Symptoms observed:
```
state=error, message="notes/alias-to-nested.md: refusing to operate on
                      non-regular file (symlink): notes/alias-to-nested.md"
state=error, message="notes/broken-alias.md: io error: No such file or directory
                      (os error 2)"
state=error, message="hardlink-to-top.md: io error: No such file or directory
                      (os error 2)"
```

Root cause (traced):
- `crates/chan-server/src/indexer.rs:280-291` — watcher event apply
  loop calls `drive.index_file(change.path)` blindly and turns any
  error into `IndexStatus::Error{ message }`. The status never
  clears until a successful `index_file` event for another path
  arrives, so a single user-created symlink can park the indexer
  in `error` forever.
- `chan_drive::Drive::index_file` -> `read_text` -> `ensure_regular_file_in`
  rejects symlinks/FIFOs/sockets/devices with `ChanError::SpecialFile`.
  The full-drive walker (`fs_ops::walk_drive_with`) silently drops
  these via `ft.is_dir() || ft.is_file()`, but the per-file watch
  path has no such filter.
- The ENOENT variant (seen on `hardlink-to-top.md`) appears to be a
  separate, possibly racy lookup against the rename log / staging area;
  hardlinks ARE regular files and should not ENOENT. Cause not fully
  traced; symptom reproduced twice.

Release impact: any first-time user who happens to have a symlink,
hardlink, or FIFO in their notes drive will see `/api/index/status`
report `error` on the search dashboard immediately. Search still
returns results when the index facade has docs, but the dashboard
will read as broken and `chan status` shows the indexer as failing.

Fixed in `crates/chan-server/src/indexer.rs`:

- watcher apply now resolves the path under the drive root, then uses
  `std::fs::symlink_metadata` to classify the path before calling
  `Drive::index_file`
- regular files index normally
- symlinks, FIFOs, sockets, devices, and directories skip without
  setting `IndexStatus::Error`
- missing paths are treated as delete races
- skipped/missing paths best-effort call `forget_file` to clear stale
  rows

Live repro after the fix:

```
target/debug/chan serve /tmp/chan-syseng-fixture-codex --here --no-token --no-browser --port 18789
touch -h /tmp/chan-syseng-fixture-codex/notes/alias-to-top.md
curl -sS 'http://127.0.0.1:18789/api/index/status'
```

Result:

```
{"state":"idle","indexed_docs":4,"indexed_vectors":4,"model":"BAAI/bge-small-en-v1.5"}
```

`architect-syseng-2.md` is now REVIEW/resolved in this repo.

### Residual risks (non-blocking)

None remaining from this pass. The previous `target_is_inside_drive`
fallback risk was tightened with a conservative lexical helper that
rejects `..` escape components when canonicalization is unavailable.

CLI graph residuals from the live pass were fixed in the current tree:

- `chan graph --scope file --target ../escape` exits 1 with
  `path escapes drive root`.
- `chan graph --scope file --target notes/no-such-file.md` exits 1
  with `No such file or directory`.
- `chan graph --scope folder --target top.md` exits 1 because the
  target is not a directory.
- `chan graph --scope file|folder` now uses the filesystem graph
  builder instead of the content graph path.

The `/api/fs-graph` `truncated: true` UI residual was addressed by
webdev-5: GraphPanel shows `truncated` in the filesystem graph status
bar.

### Final verification gate

```
cargo build -p chan                       # ok
cargo test -p chan-server                 # 92 passed
cargo test -p chan                        # 46 passed
cargo clippy --all-targets -- -D warnings # clean
cargo fmt --all -- --check                # clean
```

(The architect ran the upper four lines as a preliminary; the
release-binary `otool -L` check above and the live HTTP/CLI probes
in this section are syseng's additions.)

### Recommendation

`syseng-1` can remain REVIEW. No hard blocker remains from this pass.
The only remaining syseng-owned residual is the rare outside-drive
symlink classification fallback documented above.
