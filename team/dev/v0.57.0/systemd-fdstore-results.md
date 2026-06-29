# systemd fdstore PTY Restart Results

Branch: `systemd-fdstore-pty-restart`
Date: 2026-06-29
Target dir: `/var/tmp/chan-target-systemd-fdstore` after `/tmp` ran out of space during an earlier test link.

## Built

Implemented the Linux-only systemd fdstore restart path for `chan devserver --service=systemd --restart`:

- Added `chan-systemd`, an explicit Linux unsafe boundary for `sd_notify`/fdstore, inherited named fd adoption, fdstore remove, `READY=1`, and `FDPOLL=0` PTY storage.
- Generated systemd user units now include `Type=notify`, `NotifyAccess=main`, `FileDescriptorStoreMax=512`, and `KillMode=process`.
- CLI restart now asks the running devserver to prepare fdstore preservation before `systemctl --user restart`; failure is refused unless `--force` asks for a destructive restart.
- The devserver prepare endpoint stores live PTY masters, writes a nonce/TTL manifest, marks only the exact prepared session ids for shutdown preservation, and clears those marks if the lease expires without restart.
- Startup imports systemd named FDs, validates manifest version/TTL/library id, validates tenant prefix and persisted window layout, restores matching sessions, removes fdstore entries, and deletes the manifest.
- Invalid startup paths remove inherited fdstore entries. FD names include the child pid, so if the manifest is missing/invalid at startup, the replacement devserver can signal those child pids before removing the stored FDs.

The existing live `chan-devserver.service` was not touched for e2e because it was already running this shell/Codex session. The live tests used disposable transient user units via `systemd-run --user`, with temporary `HOME`, `CHAN_HOME`, and `XDG_RUNTIME_DIR`.

## Verification

Passed:

- `cargo fmt --check`
- `git diff --check`
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan-systemd`
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan-library terminal_sessions` (`75 passed`)
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan-server --no-default-features devserver` (`57 passed`)
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan --no-default-features devserver` (`15` CLI unit tests plus `5` devserver resilience tests passed)
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo build -p chan --no-default-features`

Default-feature check:

- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan-server devserver` still fails before compiling chan code in transitive `gemm-f16`/`gemm-common` inline AArch64 assembly: `instruction requires: fullfp16`.

## End To End

Happy-path transient systemd e2e passed:

- Started a disposable notify/fdstore user unit with the branch binary.
- Created a real terminal PTY session under the shared terminal tenant.
- Called `POST /api/devserver/systemd-fdstore/prepare`.
- Observed `preserved: 1`, `skipped: []`.
- Observed `NFileDescriptorStore=1` before restart.
- Restarted the transient unit with `systemctl --user restart`.
- Verified the same terminal `session_id` and `window_id` were present after restart.
- Verified the restart manifest was removed.
- Observed `NFileDescriptorStore=0` after restore.

Fault injection passed:

- Lease expiry without restart: after prepare, waited for the fdstore TTL; fdstore count returned to `0`, the manifest was removed, preservation marks were cleared, and a later service stop reaped the child process.
- Manifest deleted before restart: after prepare, deleted the manifest and restarted; startup removed inherited fdstore entries, signalled the child pid encoded in the FD name, did not restore the session, and left `NFileDescriptorStore=0`.

## Residual Notes

The e2e coverage was executed as disposable live-system scripts, not yet committed as a gated automated test. The implementation paths are Linux-gated; non-Linux returns a clear unsupported response for fdstore prepare.


## Rebase Verification

Rebased `systemd-fdstore-pty-restart` onto `origin/main` at `90b3881e` (`0.56.2`) on 2026-06-29.

Passed after rebase:

- `cargo fmt --check`
- `git diff --check`
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan-systemd`
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan-library terminal_sessions` (`75 passed`)
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan-server --no-default-features devserver` (`57 passed`)
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan --no-default-features devserver` (`15` CLI unit tests plus `5` devserver resilience tests passed)
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo build -p chan --no-default-features`
- Disposable user-systemd fdstore smoke: `preserved=1`, `NFileDescriptorStore` went `1 -> 0`, and the same terminal session id was restored after restart.


## Defensive Hardening Update

Added after the initial rebase verification:

- Structured skipped-session restore metadata: tenant prefix, session id, window id, child pid, and reason.
- Startup cleanup for every skipped manifest session: signal the recorded child pid, remove the fdstore entry, and remove the corresponding window only when it is a non-control standalone terminal row.
- Metadata-loss cleanup: if systemd inherits chan fdstore FDs but the restart manifest is missing or unreadable, startup removes all non-control standalone terminal rows because exact session-to-window mapping is gone. Workspace windows remain untouched.
- Orphan inherited FDs with no manifest entry are signalled by pid encoded in the fd name and removed from systemd's fdstore.
- CLI prepare now prints skipped prepare reasons, capped at eight lines, before restarting the service.
- `signal_fdstore_child`, fdstore child-name cleanup, and the new fdstore restore helpers are all `#[cfg(target_os = "linux")]`.

Failure behavior baseline:

| Failure | Behavior | Error surface |
| --- | --- | --- |
| Prepare called outside systemd / no `NOTIFY_SOCKET` | `409`, no restart unless CLI uses `--force` | CLI stderr and HTTP body |
| fdstore send fails during prepare | Remove any FDs already stored for this prepare, return `409` | CLI stderr and HTTP body |
| Manifest write fails after fd upload | Remove uploaded FDs, return `409` | CLI stderr and HTTP body |
| Prepare skips a session with no window id | Do not store that PTY; old process shuts it down normally on restart | CLI stderr now prints skipped reason |
| Startup inherits chan FDs but manifest is missing/unreadable | Signal child pids from fd names, remove fdstore entries, remove all non-control standalone terminal rows | devserver stderr/journal |
| Startup manifest version/TTL invalid | Signal child pids from fd names, remove fdstore entries, remove exact terminal rows when metadata names them | devserver stderr/journal |
| Manifest session names an FD not inherited from systemd | Skip restore, signal metadata child pid, remove exact standalone terminal row if safe | devserver stderr/journal |
| Extra inherited chan FD has no manifest entry | Signal child pid from fd name, remove fdstore entry | devserver stderr/journal |
| Manifest library id differs | Skip all imports, signal metadata child pids, remove exact standalone terminal rows if safe, remove fdstore entries | devserver stderr/journal |
| Window id missing or no longer persisted | Skip restore, signal metadata child pid, remove exact standalone terminal row if safe | devserver stderr/journal |
| Tenant prefix missing/unmounted | Skip restore, signal metadata child pid, remove exact standalone terminal row if safe | devserver stderr/journal |
| Registry import cap/race/spawn error | Skip restore, close the newly imported session on race, signal metadata child pid, remove exact standalone terminal row if safe | devserver stderr/journal |
| Workspace pane/tab metadata missing | Current server validates the owning window id, not an opaque client layout blob. A failed pane PTY restore cannot delete a workspace window; only standalone terminal rows are auto-reaped. | documented limitation |

## Platform Gating

Validated by source scan and Linux builds:

- `chan-systemd` has Linux-only public API items and Linux-only `rustix` dependency wiring under `[target.'cfg(target_os = "linux")'.dependencies]`.
- All `chan_systemd::` calls in `chan-server` are under `#[cfg(target_os = "linux")]` functions or blocks.
- `FdStoreSession*` library types and host restore/cleanup APIs are `#[cfg(target_os = "linux")]`.
- The management route `/api/devserver/systemd-fdstore/prepare` exists on all platforms; non-Linux returns `400 "systemd fdstore restart is Linux-only"`, so clients do not depend on a missing route.
- `chan devserver --service=systemd` and `--restart --service=systemd` remain runtime-gated: on non-Linux they bail with a Linux-only error instead of calling systemd helpers.

Cross-target note: this toolchain currently has only `aarch64-unknown-linux-gnu` installed (`rustup target list --installed`), so I did not run macOS or Windows `cargo check` without changing the developer toolchain.

Additional verification after hardening:

- `cargo fmt --check`
- `git diff --check`
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan-library terminal_sessions` (`75 passed`)
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan-library fdstore_skip_cleanup` (`1 passed`)
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan-server --no-default-features devserver` (`57 passed`)
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan --no-default-features devserver` (`15` CLI unit tests plus `5` devserver resilience tests passed)
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo test -p chan-systemd`
- `CARGO_TARGET_DIR=/var/tmp/chan-target-systemd-fdstore cargo build -p chan --no-default-features`

Additional disposable user-systemd smoke after hardening:

- Happy restart: `preserved=1`, `skipped=[]`, `NFileDescriptorStore` went `1 -> 0`, and session `5d175845f0eb0f0654a8f33e881fe53c` was restored under the same window id.
- Manifest-deleted fault: `preserved=1`, `NFileDescriptorStore` went `1 -> 0`, and `terminal_rows_after=0`, proving the missing-manifest path drains fdstore and does not leave a standalone terminal window dangling.
