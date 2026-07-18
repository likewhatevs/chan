# v0.57.0: systemd fdstore restarts and devserver close sync

Cut from `main` after `v0.56.4`. This release lands the already-tested `chan close` devserver state fix and the rebased Linux systemd fdstore PTY restart work.

## Theme

Make devserver lifecycle state match reality. A systemd-managed devserver restart should preserve live terminal PTYs when the supervisor supports fdstore, and a control-socket `chan close` should update the launcher feed immediately instead of waiting for stale devserver state to age out.

## What landed

### Systemd fdstore PTY preservation

- `chan devserver --service=systemd --restart` now prepares a preservation handoff before restarting the user unit.
- The running devserver snapshots live terminal PTY masters, stores them in systemd fdstore with deterministic `chan.pty.*` names and `FDPOLL=0`, writes a nonce/TTL restart manifest under the chan config dir, and marks only those exact sessions for shutdown preservation.
- The replacement devserver imports inherited named FDs, validates the manifest version, age, library id, tenant prefix, window/session metadata, and fd set, then restores matching PTYs into the rebuilt session registry.
- Failed or partial restore paths are cleanup paths: consumed fdstore entries are removed, orphan children are signalled, and standalone terminal rows are reaped only when the skipped session can be tied back safely.
- The generated systemd user unit now includes `Type=notify`, `NotifyAccess=main`, `FileDescriptorStoreMax=512`, and `KillMode=process`, and the devserver sends `READY=1` only after startup restore and serving readiness.
- `chan-systemd` is the systemd boundary for notify, fdstore add/remove, inherited named FDs, and fdstore cleanup. The Linux implementation lives in its own module, while unsupported platforms get no-op readiness helpers and a stable prepare route that returns an explicit unsupported response.
- The fdstore handoff waits for a systemd notify barrier after fd upload and manifest write. If the barrier fails or times out, chan removes any uploaded fds, deletes the manifest, and aborts the preserving restart instead of assuming systemd owns the descriptors.
- On systems that provide `LISTEN_PIDFDID`, inherited fd adoption now verifies the pidfd inode against the current process before accepting descriptors; activation environment variables are cleared whether adoption succeeds or fails.

### Restart CLI behavior

- `--restart` contacts the running devserver's authenticated management API at `/api/devserver/systemd-fdstore/prepare` before invoking `systemctl --user restart`.
- If fdstore preparation fails, the restart aborts by default and tells the user to rerun with `--force` for the old destructive behavior.
- The CLI prints preserved/skipped counts and up to eight skipped reasons; startup restore logs the matching restored/skipped summary to stderr/journal.
- The final hardening pass moved the restart implementation out of the monolithic devserver module into `crates/chan-server/src/devserver/fdstore.rs`, keeping the Linux-only flow explicit without leaking Linux-only imports into non-Linux builds.

### Devserver close and launcher state

- `chan close <path>` against a devserver-served workspace now makes the devserver management list report the real host state immediately: off, stopped, and tokenless.
- `chan close --remove <path>` now removes the row from the management list immediately instead of surfacing a stale in-memory workspace record after the host has removed the library row.
- Reopening a closed devserver workspace uses the host's real mount state, so a stale map record no longer turns the on-toggle into a no-op.
- Desktop refreshes a connected devserver workspace cache immediately after toggle and forget actions.
- The launcher disables new workspace windows unless the workspace status is actually running, so stopped/closing/removing rows cannot mint queued workspace windows.

### Devserver connection polish

- A stored write-only token can authenticate a script-backed devserver connection after the script opens the transport, so tunnel scripts can be pure transport setup.
- Editing a devserver with a full URL containing an empty `?token=` clears the stored token.
- Disconnecting a devserver reaps its transient control row from launcher state.

## Validation

The fdstore branch was validated before this release branch with:

- `cargo fmt --check`
- `git diff --check`
- `cargo test -p chan-systemd`
- `cargo test -p chan-library terminal_sessions`
- `cargo test -p chan-library fdstore_skip_cleanup`
- `cargo test -p chan-server --no-default-features devserver`
- `cargo test -p chan --no-default-features devserver`
- `cargo build -p chan --no-default-features`
- Disposable user-systemd smoke tests for successful PTY restore and missing-manifest cleanup.

The `chan close` fix added focused devserver and launcher coverage for close/off state, close-remove row removal, remount after stale state, running-only window minting, token clearing, and transient control-row cleanup.

Final fdstore hardening added focused coverage for `LISTEN_PIDFDID` matching/rejection, activation-env cleanup on pid mismatch, the no-notify-socket barrier no-op, and the generated systemd unit content.

## Release

- GA bumps all release pins to `0.57.0`, updates the changelog and this release report, then pushes commits without the `v0.57.0` tag.
- The release workflow is run with `publish=false` before tag push so macOS and Windows build coverage can catch cross-platform breakage from the Linux fdstore changes. The first dry run caught non-Linux cfg fallout; after the Windows fix, the second dry run completed green on Linux, macOS, Windows, gateway, and Docker before the final fdstore hardening follow-up landed.
- The final head must run the same `publish=false` release workflow again before tag push.
- After the dry run is reviewed, the release tag is `v0.57.0`.
