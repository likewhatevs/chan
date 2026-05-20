# systacean-8: chan index ergonomics polish (status lock + auto-register + flag asymmetry)

Owner: @@Systacean
Date: 2026-05-20

## Goal

Polish three ergonomic gaps in the `chan index` subcommand
surface that `systacean-7` shipped. None are bugs that
break behaviour; all three cost script-writer cycles and
risk surprising the user. Fix all three in one pass.

## Background

@@WebtestB ran a proactive CLI walk on `systacean-7`
(commit `6bf44cd`) and surfaced three issues. Verbatim
from the lane-B audit at the tail of
[`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
"2026-05-20 — systacean-7 proactive CLI walk":

### 1. Drive lock blocks read-only `status` (most impactful)

```
$ chan index status --path /tmp/chan-test-phase8-wb
Error: drive is locked by another process
```

Triggered when lane-B's `chan serve` is running against
the same drive. `status` is read-only — the natural use
case ("is semantic enabled?" against the drive the user
has open) is blocked. Either acquire a shared lock or
skip the lock for the read-only status path.

### 2. `status` auto-registers on a non-existent path

```
$ chan index status --path /tmp/nonexistent
Error: registering /tmp/nonexistent
```

A read-only query has a registration side-effect, and the
error message leaks the implementation detail. `status`
should refuse cleanly without registering: "not a chan
drive at <path>" (or similar — phrasing your call).

### 3. Argument-shape asymmetry inside `chan index`

`rebuild` takes a positional `<PATH>`; the other four
subcommands (`download-model`, `enable-semantic`,
`disable-semantic`, `status`) take `--path <PATH>` as a
flag. Scripts that want to operate uniformly across all
five have to special-case `rebuild`.

Suggested fix per @@WebtestB: accept `--path` as a
synonym on `rebuild` so a wrapper can treat all five
uniformly. The existing positional shape stays as the
backwards-compat alias.

## Authorization

**Authorization: yes**, this task covers edits to
`crates/chan/src/main.rs` (CLI clap definitions),
`crates/chan-drive/src/drive.rs` (lock acquisition for
status), and any chan-server route that mirrors the CLI
shape. @@Systacean may proceed without further in-chat
confirmation from @@Alex.

## Acceptance criteria

* `chan index status --path <live-served-drive>` succeeds
  while `chan serve` is running on the same drive.
  Returns the SemanticState JSON / human-readable shape
  unchanged.
* `chan index status --path /tmp/nonexistent` errors
  cleanly without registering. Error message names the
  user-visible problem ("not a chan drive at <path>") and
  doesn't leak "registering" implementation detail.
* `chan index rebuild --path <PATH>` works identically to
  `chan index rebuild <PATH>` (positional). The positional
  shape stays supported for backwards-compat.
* The same `--path` flag works on all five subcommands
  (`rebuild`, `download-model`, `enable-semantic`,
  `disable-semantic`, `status`). Script-writer can pass
  `--path <X>` uniformly.
* If the API endpoint
  `GET /api/index/semantic/state` has the same lock-block
  symptom against a served drive, fix that too — same
  read-only-lock or skip-lock-for-read pattern.
* Pre-push gate: fmt + clippy `-D warnings` + workspace
  test + svelte-check + npm build.
* New unit test pinning the status-on-live-served-drive
  path (some form of "Drive::open_for_read" against a
  drive someone else holds the write lock on works).

## How to start

1. Locate the lock acquisition in `crates/chan-drive/src/drive.rs`.
   The current shape probably acquires an exclusive lock.
   Add a read-only / shared variant + thread it through
   the status-side call paths in `crates/chan/src/main.rs`
   + `crates/chan-server/src/routes/index.rs`.
2. Locate the "registering" code path that fires on
   `status --path <nonexistent>`. Likely the drive-registry
   lookup falls through to a registration call when the
   path isn't already registered. For `status`, the lookup
   should NOT auto-register; missing-from-registry → "not
   a chan drive at <path>" with a non-zero exit.
3. Extend the clap definition for `rebuild` to accept
   `--path` as a flag in addition to the positional. Both
   resolve to the same internal arg. Update help text to
   note the alias.
4. Visual / functional sanity on lane-B (the test fixture
   that surfaced these issues is still up):
   `chan index status --path /tmp/chan-test-phase8-wb`
   while lane-B's serve runs.

## Coordination

* @@WebtestB verifies on lane-B drive once landed.
* No backend / Rust work in the chan-server lane unless
  the API endpoint has the same lock issue (in which
  case fix in the same commit).
* Coordinate with @@FullStackA on `fullstack-a-21`: if
  the Settings UI's polling-pattern relies on
  `/api/index/semantic/state` succeeding against the
  live-served drive, this fix unblocks an otherwise-
  silent failure mode there. @@FullStackA's -21 logic
  already handles the failure (toast on download/enable
  error), but a successful read is the happy path the
  polling depends on.

## 2026-05-20 — implementation + commit

### Changes

* `crates/chan/src/main.rs`:
  * `IndexAction::Rebuild` accepts `path: Option<PathBuf>`
    (positional, backwards-compat) AND
    `#[arg(long = "path")] path_flag: Option<PathBuf>`
    (uniform with the other four subcommands). Either
    form resolves to the same internal arg; both empty
    → clean "rebuild requires a drive path" error.
  * `cmd_index_status` rewritten lock-free:
    * Look up the registered `DrivePaths` via
      `Library::drive_paths_for(&root)`; bail with
      `not_a_chan_drive_hint(&root)` when the path
      isn't in the registry. No `Drive::open`, no
      writer lock acquired.
    * Load `IndexConfig` directly with
      `chan_drive::index::config::load(&paths.index)`.
    * Same JSON / text output as before; canonical
      drive path comes from `Library::list_drives()`
      look-up + the `same_path` helper.
  * `cmd_index_set_semantic` drops `ensure_drive_named`
    so an unregistered `--path` argument refuses
    cleanly instead of auto-registering. Still uses
    `Drive::open` (writer lock) — `enable` / `disable`
    must mutate the drive's `IndexConfig`, so the lock
    is appropriate. Users hitting the lock-blocked path
    against a live-served drive should use the
    `/api/index/semantic/{enable,disable}` endpoint
    instead.
  * New helper `not_a_chan_drive_hint(&Path) -> String`
    rendered with "run `chan add <path>` first" so the
    next-step is obvious.
* `crates/chan-drive/src/index/config.rs`: new unit test
  `load_works_while_drive_lock_is_held` pins the
  invariant: `config::load` reads cleanly even when
  another holder (simulating `chan serve`) has acquired
  the per-drive writer flock. Pre-fix the CLI took the
  writer lock + got `DriveLocked`; post-fix it doesn't
  touch the lock at all.

### API endpoint check

`GET /api/index/semantic/state` is unaffected by the
lock-block symptom: chan-server holds the writer lock
on its served drive (single process), and the endpoint
queries the in-process `Arc<Drive>` directly. No new
lock acquired, no contention with itself. The CLI was
blocked because it tried to open a SEPARATE `Drive`
handle against the same drive, which `chan serve`
already held. The API path doesn't have that shape, so
no endpoint change.

### Gate

* `cargo fmt --check` — clean.
* `cargo clippy --all-targets -- -D warnings` (default,
  embed-model, no-default-features) — all clean.
* `cargo test --all` — green. New
  `load_works_while_drive_lock_is_held` test passes;
  existing tests unaffected.

### Status

Committed as `693b161`:

```
chan index ergonomics: lock-free status + no auto-register + --path on rebuild (systacean-8)
```

2 files (`crates/chan-drive/src/index/config.rs`,
`crates/chan/src/main.rs`), +98 / -14. Pre-commit
`git diff --staged --stat` audit clean.

@@WebtestB to re-verify on lane-B against
`/tmp/chan-test-phase8-wb` (running chan serve) once
the rebuilt binary is in place.

## 2026-05-20 — @@Architect: approved + cleared (already committed)

Reviewer: @@Architect.

Three issues, three clean fixes, two files. Each fix
matches the root cause:

* **Lock-free status path**: rewriting `cmd_index_status`
  to look up `DrivePaths` via `Library::drive_paths_for`
  + load `IndexConfig` directly is the right shape — no
  `Drive::open`, no writer lock, no contention with a
  live-served drive. The new
  `load_works_while_drive_lock_is_held` test pins the
  invariant explicitly.
* **No-auto-register**: dropping `ensure_drive_named`
  from `cmd_index_status` + emitting
  `not_a_chan_drive_hint(&root)` with "run `chan add
  <path>` first" gives the user a clear next step
  without the leaky "registering" abstraction.
  Sensible call to leave `cmd_index_set_semantic` on
  the writer-lock path (enable/disable must mutate
  IndexConfig, so the lock IS appropriate; live-served
  case routes through the API endpoint instead).
* **`--path` synonym on `rebuild`**: keeping the
  positional shape AND adding `--path` for uniform
  scripting is exactly what @@WebtestB's audit asked
  for. Empty-both case emits a clean "rebuild requires
  a drive path" error.

The API-endpoint check is good engineering hygiene —
verifying that `GET /api/index/semantic/state` is
already lock-free (in-process `Arc<Drive>` query, not
a separate `Drive::open`) means the CLI was the only
broken surface. No endpoint change needed.

Pre-push gate green across all three feature paths.
Pre-commit audit clean per the systacean-4 lesson.

**Cleared (already committed)**: `693b161`. Push waits
until end of Round 2.

@@WebtestB picks up the re-verification on lane-B
against the same fixture they used to find the bugs.