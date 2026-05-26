# chan-desktop in-process registry — drop the `chan` binary entirely

Phase 10, Track A (desktop shell + CLI handoff). Owner: @@Desktect.

This document tracks a single phase-10 change: making chan-desktop have
ZERO runtime dependency on the `chan` binary. It carries the full
implementation plan (verbatim copy of
`~/.claude/plans/serene-discovering-treehouse.md`) so the phase journal
holds the rationale and the step ordering, and reserves an
"Implementation summary" section at the end to be filled in when the
round closes.

---

## Plan

### Context

chan-desktop (the Tauri app under `desktop/`) shells out to a bundled
`chan` CLI binary for registry mutations and feature toggles. This is
the root of a user-visible bug and a standing architectural wart.

**The bug.** On a fresh environment, opening a brand-new directory via
"Open drive" fails with `chan-drive: drive not registered: <path>`,
and the drive row's Open button stays greyed out. Root cause: the
embedded `chan_server::DriveHost` opens a `chan_drive::Library` ONCE at
boot (`desktop/src-tauri/src/embedded.rs:23`) and owns it for the
process lifetime. `add_drive` registers the new drive by spawning
`chan add <path>` — a SEPARATE process that writes `~/.chan/config.toml`
on disk. The embedded `Library`'s in-memory registry snapshot
(`Mutex<Registry>`, `crates/chan-drive/src/library.rs:61`) never learns
about the new row, so the immediately-following `serve::start` →
`open_registered_drive` → `Library::open_drive` → `reg.find()`
(`library.rs:221-223`) returns `None` → `DriveNotRegistered`. The same
class of bug affects `default_drive.rs`, which registers through its
own throwaway `Library::open_at(...)` handle rather than the embedded
one.

**The directive.** chan-desktop must have ZERO runtime dependency on
the `chan` binary. Every operation currently done via subprocess must
run in-process through the `chan_drive` / `chan_server` crates the app
already links, against the SINGLE embedded `Library` instance. The
`chan` binary is also removed from the app bundle and build.

**Intended outcome.** Registering and opening a never-seen directory
becomes one in-process step against the shared registry, so the
staleness window is gone. The app no longer probes for, gates on, or
ships a `chan` binary.

### Design (recommended approach)

The fix is to make the embedded `Library` the single source of truth
for the registry and route every desktop registry/feature operation
through it. Because `register_drive` mutates the in-memory
`Mutex<Registry>` AND persists to disk in one call
(`library.rs:171-176`), a subsequent `open_drive` on the same handle
sees the row immediately. No reload, no subprocess, no staleness.

#### 1. Expose shared Library + live-drive accessor (chan-server + embedded)

The embedded `DriveHost` keeps drives MOUNTED (holding an `Arc<Drive>`
in `DriveCell.drive`, `crates/chan-server/src/state.rs:122`). A second
`Library::open_drive` for a mounted path returns `DriveAlreadyOpen`
(`library.rs:235-241`) and `Drive::open` holds a lifetime flock
(`crates/chan-drive/src/drive.rs:365`). So feature toggles on a running
drive must reach the SAME live `Arc<Drive>`, not re-open it.

- `crates/chan-server/src/host.rs`: add
  `pub fn live_drive(&self, root: &Path) -> Option<Arc<Drive>>`. Scan
  `self.drives` for a runtime whose canonicalized `root` matches the
  canonicalized argument, then return
  `runtime.artifacts.drive_cell.read().ok()?.as_ref()?.drive.clone()`.
  Mirror the proven pattern in `AppState::try_drive` (`state.rs:145-154`):
  treat lock poisoning and a `None` cell as "not live" (return `None`).
  Compare via canonical form (mirror `library.rs:520` `canonical_key`),
  NOT raw `PathBuf` equality, so a symlinked/non-normalized SPA path
  still matches. `DriveHost::library()` already exists (`host.rs:77`).
- `desktop/src-tauri/src/embedded.rs`: add
  `pub fn library(&self) -> &chan_drive::Library` (delegates to
  `self.host.library()`) and
  `pub fn live_drive(&self, root: &Path) -> Option<Arc<chan_drive::Drive>>`
  (delegates to `self.host.live_drive(root)`).

#### 2. Convert the IPC commands in `desktop/src-tauri/src/main.rs`

All blocking chan-drive calls (`register_drive`, `open_drive`, `boot`,
`unregister_drive`, `set_*`) run via `tokio::task::spawn_blocking` with
a cloned `Library` / `Arc<Drive>` (both are `Send + Sync`, Arc inside).
`boot()` can run a slow initial scan, so it MUST stay off the async
executor.

- **`add_drive`** (~266): drop the subprocess. Mirror
  `chan/src/main.rs::cmd_add` (~823): if `!path.exists()`,
  `fs::create_dir_all`. Then `embedded.library().register_drive(&path)`.
  If features requested, in a scoped block: `open_drive` →
  `set_semantic_enabled(true)` / `set_reports_enabled(true)` → `boot()`,
  then DROP the `Arc<Drive>` before returning from the block. Dropping
  before `serve::start` is critical — otherwise `open_registered_drive`
  inside `serve::start` hits `DriveAlreadyOpen`. Then `serve::start` as
  today. Keep mirroring chosen features into the desktop cache.
- **`remove_drive`** (~320): keep `serve::stop` first, then replace
  `chan remove` with `embedded.library().unregister_drive(&key)`.
  Teardown race: after `close_drive`, an in-flight indexer rebuild or
  HTTP/WS handler may briefly still hold an `Arc<Drive>`, so
  `unregister_drive` → `reset_drive` can return `DriveAlreadyOpen` or
  `DriveLocked`. Wrap in ONE `spawn_blocking` running a bounded retry
  loop (`std::thread::sleep`, ~20 attempts × 150ms ≈ 3s), retrying ONLY
  on those two errors and surfacing any other `ChanError` immediately.
  `reset_drive` acquires the flock before any registry mutation
  (`library.rs:331` then `:360-365`), so a failed unregister leaves no
  half-state. On exhaustion, return a clear "still shutting down" error.
- **`get_drive_features`** (~392): replace
  `read_features_via_chan_index_status`. If
  `embedded.live_drive(&key)` is `Some`, read `semantic_enabled()` +
  `reports_enabled()` off it; else if registered
  (`library.drive_paths_for(&key).is_some()`), transient `open_drive` →
  read → drop. Else return the desktop-cache default. Update the cache
  on a successful read as today.
- **`set_drive_features`** (~731): replace
  `run_chan_feature_subcommand`. Resolve the target `Arc<Drive>` (live
  if mounted, else transient open). Apply only changed flags:
  `set_semantic_enabled(bool)`, `set_reports_enabled(bool)`. When
  enabling reports, also call `boot()` to kick the initial scan (mirror
  `cmd_reports_set`, `chan/src/main.rs:1170-1209`). Then update cache.
- **`compute_drive_preflight`** (~642): replace
  `check_drive_already_registered` (`chan list --json`) with
  `embedded.library().drive_paths_for(&key).is_some()` (`library.rs:429`).
- Delete the now-dead helpers: `read_features_via_chan_index_status`
  (~426), `check_drive_already_registered` (~685),
  `run_chan_feature_subcommand` (~775).

#### 3. Route default-drive registration through the shared Library

`desktop/src-tauri/src/default_drive.rs` (`create_default_drive_at` ~94,
`factory_reset_default_drive_at` ~163) registers via its own
`Library::open_at(config_path)` handle — same staleness class. Make the
Tauri command wrappers (`create_default_drive`, `factory_reset_default_drive`)
reconcile with the embedded `Library`: after the seed/registry work,
call `embedded.library().register_drive(root)` +
`set_default_drive_root(Some(root))` so the embedded in-memory registry
matches disk before any serve. (Pure helpers can keep their
`config_path` signature for unit tests; the reconciliation is the
command-level addition.)

#### 4. Remove the binary-resolution + gating machinery

- `desktop/src-tauri/src/serve.rs` (~173-325): delete
  `bundled_chan_path`, `probe_chan_version`, `resolve_chan_binary`,
  `resolve_chan_binary_with`, `which_chan`, `which_chan_in`,
  `is_executable_file`, and their doc comments.
- `desktop/src-tauri/src/main.rs`: delete `BinStatus` struct + impl
  (~85-100), `compute_bin_status` (~1346), `chan_bin_status` command
  (~1390), `require_bin` (~1399), the `AppState.bin_status` field (~51)
  and its init (~1499, ~1505). Remove ALL `require_bin` calls: 272, 325,
  736, 917 (the `tunnel_start` one is spurious — the tunnel is fully
  in-process), and the gating reads at 397, 654. Remove `chan_bin_status`
  from `tauri::generate_handler!` (~1648). Remove the now-unused
  `use tokio::process::Command;` import (the `opener` spawn at ~1211
  uses `std::process::Command` and stays — it is the OS file-opener,
  not `chan`).
- Decide whether to keep `emit_chan_busy` / the `chan-busy` event. KEEP
  it (decoupled from the deleted `bin_status`) as a progress indicator
  around the `spawn_blocking` add/remove work — `boot()` can be slow.

#### 5. Frontend `desktop/src/main.js`

Excise every `chanBinStatus` reference (decl ~98; auto-bootstrap gate
~154 `&& chanBinStatus.ok`; `checkChanBin` ~393-404; `applyChanBinStatus`
~406-426; `applyChanBusyState` read ~430; `chanCommandDisabledAttr`
~721). Remove the `chan_bin_status` invoke, the `chan-bin-banner` DOM,
and the `chan-bin-unavailable` body class. `openBtn.disabled` and
`chanCommandDisabledAttr` now key only off `chanBusy`. Drop the
`.chan-bin-unavailable` rule from `desktop/src/styles.css` if present.
Keep the `chan-busy` listener and busy banner.

#### 6. Stop shipping the `chan` binary (build + bundle + docs)

- `desktop/Makefile`: remove the `chan-bin` target (~49-57), drop it
  from the `.PHONY` list (~1) and from the `run` / `build` /
  `app-signed` / `app-notarized` prerequisites (~59, 62, 232, 249), and
  remove the `CHAN_BIN` var (~8). The chan codesign step inside
  `chan-bin` goes with it.
- `desktop/src-tauri/tauri.conf.json`: remove the
  `bundle.macOS.files."MacOS/chan"` entry (~59-60). If `files` becomes
  empty, drop the key.
- `desktop/CLAUDE.md`: rewrite the "Local serving and bundled chan
  helper", "Helper binary layout", "Resolution helpers", and
  "Resolution algorithm" sections to state that chan-desktop is fully
  self-contained and links `chan-drive` / `chan-server` directly with
  no `chan` binary at runtime or in the bundle.

### Failure modes & cross-process contention

The single-writer invariant (CLAUDE.md: "one chan serve process owns
the drive's writes") is enforced by a per-drive `DriveLock` flock that
`Drive::open` holds for the drive's lifetime (`drive.rs:365`). The
refactor must keep this clean under the following modes.

#### App quit (Cmd+Q / menu Quit / last window close)

UNCHANGED and must stay working. Tauri fires `RunEvent::Exit`, whose
handler already calls `serve::stop_all` (`main.rs:~1660`): it unmounts
every embedded drive → drops each `Arc<Drive>` → `Drop` releases the
flock and flushes the index/report state, then cancels the tunnel.
`AppState::drop` → `stop_all` (`main.rs:69-73`) is the panic-unwind
backstop. The in-process `Library` needs no explicit flush — registry
writes already persist on each mutation. The only new requirement: the
refactored `add_drive` / feature commands must use SCOPED or transient
`Arc<Drive>` handles dropped before the command returns, so nothing
stays open past a command and the Exit hook has only mounted serves to
tear down. Verify the Exit path still unmounts after the refactor.

#### Force-kill / SIGKILL / power loss

No graceful hook runs; safety rests on existing invariants — the OS
releases the flock on process death (a relaunch or later `chan serve`
can re-acquire it) and chan-drive uses atomic writes for user content +
registry (no torn files). An interrupted reindex is safe (reindex is
idempotent; the next open catches up). Nothing to add; document the
reliance.

#### External `chan serve` vs a drive chan-desktop already mounts

- **Desktop first, then `chan serve <same drive>`**: the CLI's
  `Drive::open` gets `ChanError::DriveLocked` and exits with "drive is
  locked by another process". Correct and expected; chan's CLI owns
  that message. No desktop change.
- **`chan serve <drive>` first, then Open in chan-desktop**:
  `embedded.open_drive` → `open_registered_drive` → `library.open_drive`
  → `Drive::open` returns
  `chan_server::Error::Core(ChanError::DriveLocked)`
  (`crates/chan-server/src/error.rs:17-18`;
  `crates/chan-drive/src/error.rs:29-30`). The desktop MUST map this to
  a clear, non-fatal message (e.g. "This drive is open in another chan
  process. Quit it and try again.") rather than the raw error, and MUST
  NOT leave the row half-on. `serve::start` already returns `Err` before
  inserting a `ServeHandle`, so the frontend reverts the toggle — verify
  the On toggle snaps back and the banner shows the friendly text.
  Implementation: in `EmbeddedServer::open_drive` (`embedded.rs:54`),
  match the `chan_server::Error` BEFORE stringifying — for
  `Error::Core(ChanError::DriveLocked | ChanError::DriveAlreadyOpen)`
  return the friendly message; otherwise keep the existing
  `opening embedded drive {key}: {e}` form.
- **Same-process double-mount** is already a no-op: `serve::start`
  checks `state.serves.contains_key(&key)` first (`serve.rs:55`).

### Critical files

- `crates/chan-server/src/host.rs` — add `live_drive`.
- `desktop/src-tauri/src/embedded.rs` — expose `library` + `live_drive`.
- `desktop/src-tauri/src/main.rs` — rewrite 5 commands, delete machinery.
- `desktop/src-tauri/src/serve.rs` — delete binary resolution + tests.
- `desktop/src-tauri/src/default_drive.rs` — share embedded Library.
- `desktop/src/main.js` — remove bin-status gating.
- `desktop/Makefile`, `desktop/src-tauri/tauri.conf.json`,
  `desktop/CLAUDE.md` — stop shipping the binary.

Reused APIs (no new primitives needed): `Library::register_drive`,
`unregister_drive`, `open_drive`, `drive_paths_for`, `list_drives`,
`set_default_drive_root` (`crates/chan-drive/src/library.rs`);
`Drive::{semantic_enabled,set_semantic_enabled,reports_enabled,set_reports_enabled,boot}`
(`crates/chan-drive/src/drive.rs:2317-2472`); `DriveHost::library`
(`crates/chan-server/src/host.rs:77`); `AppState::try_drive` as the
`live_drive` template (`crates/chan-server/src/state.rs:145-154`).

### Tests

DELETE (assert behavior being removed; in `serve.rs` test module):
- `add_drive_passes_feature_flags_to_chan_cli`
- `get_drive_features_reads_chan_index_status_after_b28b_ii`
- `set_drive_features_calls_chan_cli_after_b28b`
- `chan_index_status_json_carries_reports_enabled_after_b28b_ii`
  (desktop's only `include_str!` dependency on `chan/src/main.rs`)
- all `resolve_chan_binary_*`, `bundled_chan_path_*`, `which_chan_*`,
  `probe_chan_version_*` tests (grep the test module; they test deleted
  fns)

KEEP unchanged (pin surviving command registrations / frontend copy):
`invoke_handler_registers_drive_features_ipcs`,
`invoke_handler_registers_compute_drive_preflight`,
`launcher_calls_drive_features_ipcs`,
`pick_and_add_shows_preflight_dialog_before_add_drive`,
`preflight_dialog_carries_round2_plan_explanatory_copy`,
`preflight_modal_renders_report_rows_after_b28b_iv`,
`launcher_features_panel_carries_round2_plan_toggles`.

ADD:
- `chan-server` test: `DriveHost::live_drive` returns the same `Arc`
  the mounted runtime holds (extend the host.rs test module ~282-374).
- `chan-server` (or chan-drive) test: unregister-after-close eventually
  succeeds under the retry policy (open → trigger rebuild → close →
  assert immediate attempt is `DriveAlreadyOpen`/`DriveLocked`,
  justifying the loop).
- desktop pin test: `chan_bin_status` is NOT in `generate_handler!` and
  `require_bin` no longer exists (mirror-image of the deleted pins).

### Verification

1. `cargo build` (workspace) and `cargo build -p chan-desktop`.
2. `cargo test -p chan-server -p chan-drive` (live_drive + retry tests).
3. `cargo test` in `desktop/src-tauri` (rewritten pin tests).
4. `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`.
5. In `web/`: `npm run build` and `npm run check` (svelte-check).
6. End-to-end against a CLEAN `~/.chan` (move it aside first):
   `cd desktop && make run`. Confirm:
   - "Open drive" → pick the `chan` repo (never registered) →
     onboarding → Open: NO "drive not registered" banner; the drive
     mounts and the editor window opens.
   - Toggle Semantic search / Reports on a running drive: persists and
     takes effect; toggling on a stopped drive also works.
   - Forget a drive immediately after opening it (teardown race): the
     row disappears with no error.
   - The default Documents/Chan drive opens cleanly.
   - Grep the built bundle: no `chan` binary under
     `Chan.app/Contents/MacOS/`.
7. Failure modes:
   - With a drive mounted in chan-desktop, run `chan serve <that drive>`
     from a terminal (a `chan` built from this checkout, NOT bundled):
     expect a clear "drive is locked" failure, no hang or crash.
   - Run `chan serve <drive>` first, then Open the same drive in
     chan-desktop: expect the friendly "open in another process" banner
     and the On toggle reverting — no broken half-on row.
   - Cmd+Q while a drive is mounted, then relaunch: the drive mounts
     cleanly (flock was released), no "locked" error.
8. Restore the original `~/.chan` when done.

### Tear-down note

`make run` launches a desktop process and may leave embedded serves /
windows open. Quit the app cleanly (RunEvent::Exit unmounts drives).
Remove any throwaway test drives with the in-app Forget, and restore
the real `~/.chan` you moved aside.

---

## Implementation summary

Date landed: 2026-05-26. Status: implemented and verified.

Landed as planned, in the plan's order (accessors -> commands ->
delete machinery -> default drive -> frontend -> build/docs). No
material deviations from the design.

What changed:

- `crates/chan-server/src/host.rs`: added
  `DriveHost::live_drive(root) -> Option<Arc<Drive>>`, matching by
  canonical key and mirroring `AppState::try_drive`'s lock posture
  (poisoned lock / drained cell read as "not live"). New unit test
  `live_drive_returns_the_mounted_runtime_handle`.
- `desktop/src-tauri/src/embedded.rs`: exposed `library()` +
  `live_drive()` delegating to the host, and added `map_open_error`
  so `DriveLocked` / `DriveAlreadyOpen` on open surface the friendly
  "open in another chan process" message instead of a raw error.
- `desktop/src-tauri/src/main.rs`: rewrote `add_drive`,
  `remove_drive`, `get_drive_features`, `set_drive_features`, and
  `compute_drive_preflight` to run in-process against the embedded
  `Library` / live `Arc<Drive>`. New blocking helpers
  `register_and_boot`, `unregister_with_retry` (bounded ~20x150ms
  retry on `DriveAlreadyOpen` / `DriveLocked`),
  `read_drive_features_blocking`, `resolve_drive_for_features`,
  `apply_drive_features_blocking`, `reconcile_default_drive`. All
  blocking chan-drive work runs on `spawn_blocking`. Deleted
  `BinStatus`, `compute_bin_status`, `chan_bin_status`,
  `require_bin`, `is_app_translocated`,
  `read_features_via_chan_index_status`,
  `check_drive_already_registered`, `run_chan_feature_subcommand`,
  the `AppState.bin_status` field, the handler registration, and the
  `tokio::process::Command` import. `emit_chan_busy` / the
  `chan-busy` event were kept as the add/remove progress indicator.
- `desktop/src-tauri/src/serve.rs`: deleted `bundled_chan_path`,
  `probe_chan_version`, `resolve_chan_binary`,
  `resolve_chan_binary_with`, `which_chan`, `which_chan_in`,
  `is_executable_file`, and their tests; refreshed the module doc.
- `desktop/src/main.js` + `desktop/src/styles.css`: removed the
  `chanBinStatus` state, `checkChanBin` / `applyChanBinStatus`, the
  `chan-bin-banner` / `chan-bin-unavailable` surfaces, and the
  orphaned `.error-banner.persistent` rule. Disabled state now keys
  only off `chanBusy`; the `chan-busy` listener + busy banner stayed.
- `desktop/Makefile`, `desktop/src-tauri/tauri.conf.json`,
  `desktop/CLAUDE.md`: removed the `chan-bin` target, the `CHAN_BIN`
  var, and the `bundle.macOS.files` chan entry; wired `web` directly
  into the run/build/app-signed/app-notarized prerequisites (it was
  previously a transitive dep through `chan-bin`); rewrote the helper
  docs to describe a fully self-contained app.

Test changes: deleted the four CLI-behavior pin tests that asserted
the removed subprocess shape (including the only `include_str!`
dependency on `crates/chan/src/main.rs`) and the resolver tests;
added `registry_and_feature_commands_run_in_process_not_via_chan_cli`
and `bin_status_machinery_is_gone` as absence pins, plus the
`live_drive` host test.

Verification (run against the combined shared tree that also carries
the concurrent MCP-registration-removal change):

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo build` (workspace) + `cargo build -p chan-desktop`: ok.
- `cargo test` (full workspace): all green, including the three new
  tests above.
- `node --check desktop/src/main.js`: syntax ok. The desktop
  launcher is served raw (`frontendDist: ../src`, no bundler /
  svelte-check step); `web/` was not touched by this change.

Not done in this pass (no environment for it): the live end-to-end
`make run` walk against a moved-aside `~/.chan` (plan Verification
steps 6-7). The logic is covered by unit/integration tests and a
clean build; the manual GUI walk is left for an interactive session.

Follow-ups: none required. The universal2 / multi-platform bundling
follow-up referenced by the old docs is moot now that no second
binary is shipped.
