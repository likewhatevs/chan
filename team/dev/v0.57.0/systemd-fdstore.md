# systemd fdstore PTY Restart Preservation

## Summary

Implement Linux-only PTY preservation for `chan devserver --service=systemd --restart` using systemd's file descriptor store (`FDSTORE=1`, `FDNAME=...`, `FDPOLL=0`). The path is opt-in and only entered through the restart command we initiate. Normal SIGTERM, stop, terminal close, workspace off/forget, and crash paths keep current behavior.

References: https://systemd.io/FILE_DESCRIPTOR_STORE/ and `systemd.service(5)` fdstore settings.

## Key Changes

- Change the generated user unit to support notifications and fdstore:
  - `Type=notify`
  - `NotifyAccess=main`
  - `FileDescriptorStoreMax=<derived cap>`
  - keep `Restart=on-failure`
  - emit `READY=1` only after listener/token/restore readiness.
- Add a systemd restart-prep control path in the running devserver:
  - callable only by local authenticated `chan devserver --service=systemd --restart`
  - rejects non-systemd, non-Linux, stale unit, wrong address, active restart, or missing fdstore support
  - snapshots all live terminal PTY sessions across shared terminal tenant and mounted workspaces
  - excludes control terminals unless they are ordinary devserver terminal sessions; preserves no terminal/workspace teardown path.
- Add durable restart manifest under `$CHAN_HOME/devserver/fdstore-restart.json`:
  - restart nonce, service/unit identity, chan version/protocol, created deadline
  - expected tenant prefixes, workspace roots, window ids, session ids, pane/tab ids, tab/group, size, cwd, generation policy, layout hash
  - fd names and fd count
  - written atomically, mode `0600`.
- Upload PTY master FDs to systemd with deterministic names:
  - `chan.pty.<nonce>.<tenant>.<session>`
  - `FDPOLL=0` to prevent systemd dropping PTY masters on hangup-like readiness events during the handoff
  - bounded lease: if SIGTERM/restart does not arrive shortly after prepare, old process removes the stored FDs and deletes the manifest.
- Replace the current Linux terminal implementation with a small importable PTY backend:
  - keep `portable-pty` for macOS/Windows
  - on Linux, own PTY master FDs directly so sessions can be spawned from `openpty` or imported from fdstore
  - provide the same reader/writer/resize/kill/wait behavior currently wrapped around `portable-pty`.
- Startup restore path:
  - read `$LISTEN_FDS`/`$LISTEN_FDNAMES` before normal terminal creation
  - validate manifest nonce, version, unit identity, fd names, fd count, layout hash, workspace restore set, and still-mounted session metadata
  - reconstruct sessions from imported PTY masters only after the corresponding tenant/layout exists
  - close every unmatched imported FD immediately and log a precise reason
  - if validation fails, start the devserver cleanly without preserved PTYs; do not keep orphan fdstore imports alive.
- CLI restart flow:
  - `--restart` contacts the running service, prepares fdstore, then runs `systemctl --user restart chan-devserver.service`
  - if live PTYs exist and preserve-prep fails, abort unless `--force` is supplied
  - `--force` keeps current destructive restart semantics and prints that PTYs will not be preserved.

## Rustacean/Syseng Challenge Outcomes

- Rustacean constraint: encode restore state as typed `RestartManifest`, `StoredPty`, and `ImportedSession` objects; no stringly fd matching outside one parser.
- Syseng constraint: fdstore is supervisor state, not app state; every stored FD must have a lease, deterministic name, validation, and close/remove path.
- Simplification decision: no preservation for individual terminal/workspace teardown; no crash recovery; no cross-version restore beyond an exact protocol match.
- Correctness decision: preserve PTY masters, not child process handles. Shells survive because systemd holds the master FD while the old server exits.
- Effectiveness decision: use `Type=notify` to remove the current "active before usable" race and to make restart readiness observable.

## Test Plan

- Unit tests:
  - manifest atomic write/read, permission, nonce, expiry, fd-name parsing
  - layout hash mismatch, missing metadata, extra FD, missing FD, duplicate FD, stale version
  - startup closes all unmatched imports
  - `--restart` aborts without `--force` when live PTYs cannot be preserved
  - normal stop/SIGTERM never calls fdstore prep.
- Linux integration tests with fake notify socket:
  - verify `FDSTORE=1`, `FDNAME`, `FDPOLL=0`, fd counts, lease cleanup
  - imported PTY session can read/write/resize after reconstruction.
- systemd end-to-end test gated by env, e.g. `CHAN_SYSTEMD_FDSTORE_E2E=1`:
  - start user service, open terminals in shared tenant and workspace tenant
  - run shell commands that keep state
  - `chan devserver --service=systemd --restart`
  - reconnect windows and verify same session ids, layout, cwd, PTY output continuity, input still reaches original shell process.
- Fault injection / chaos baseline:
  - delete manifest after fd upload
  - delete `$CHAN_HOME/.chan` / workspace session metadata
  - drop one stored FD
  - add unknown FD
  - corrupt fd name
  - kill old process before/after prepare
  - kill new process during restore
  - force systemd fdstore capacity exhaustion
  - verify no leaked `/proc/<pid>/fd` growth and no restored session without validated metadata.
- Verification:
  - focused Rust tests for `chan-library` and `chan`
  - fd leak checks around restart loops
  - full pre-push gate after implementation.

## Assumptions

- Minimum supported Linux systemd must include fdstore, named FDs, `FDPOLL=0`, and fdstore removal. If unavailable, preserving restart is refused unless `--force`.
- Exact-version/protocol restore only for v1.
- The first implementation preserves live PTY continuity and layout identity, but not queued rich-prompt messages or in-memory scrollback beyond what the live PTY/client restore already provides.


## Failure Behavior Baseline

The feature is fail-closed. A PTY is restored only when fdstore state, manifest state, library identity, tenant prefix, and persisted window identity all line up. Every skipped restore must have a cleanup decision:

- Prepare failure before restart: abort `--restart` unless `--force` is supplied. Any FDs uploaded during the failed prepare are removed from systemd fdstore before returning the error.
- Prepare skip: do not store the PTY. The old process keeps normal shutdown behavior for that session; the CLI prints the skipped reason.
- Missing/unreadable manifest with inherited chan FDs: signal child pids encoded in fd names, remove fdstore entries, and remove all non-control standalone terminal rows because exact session-to-window mapping is gone. Workspace and control windows are not removed.
- Invalid/stale manifest: remove inherited fdstore entries, signal child pids, and remove exact standalone terminal rows when parsed metadata names them.
- Missing inherited FD for a manifest session: skip restore, signal the metadata child pid, remove the exact standalone terminal row when safe, and log the reason.
- Extra inherited chan FD: signal the child pid encoded in its fd name, remove it from fdstore, and log the orphan.
- Library id mismatch: skip all imports, signal metadata child pids, remove exact standalone terminal rows when safe, remove fdstore entries, and log the mismatch.
- Missing window id, missing persisted window, missing tenant prefix, session cap, spawn/import error, or restore race: skip restore, signal the metadata child pid, remove the exact standalone terminal row when safe, and log the reason.
- Workspace pane/tab mismatch: the server currently stores pane/tab ids in PTY metadata but does not parse the opaque client layout blob as an authoritative schema. A failed workspace-pane PTY restore must not discard the workspace window. Future typed layout metadata can tighten this; the baseline behavior is no fd leak and no standalone terminal-window ghost.

Errors go to the initiating CLI during prepare (`stderr`, with `--force` opt-in for destructive restart) and to the new devserver's stderr/journal during startup restore. The restore log is summarized as restored/skipped counts plus up to eight precise skipped reasons.

## Platform Scope

Linux/systemd is the only supported implementation. The Rust boundary is:

- `chan-systemd` exposes helpers only under `#[cfg(target_os = "linux")]` and has Linux-only dependencies.
- Library fdstore metadata/import APIs are `#[cfg(target_os = "linux")]`.
- Server restore and notify paths are `#[cfg(target_os = "linux")]`.
- The prepare HTTP route is stable on every platform, but non-Linux returns an explicit unsupported response instead of a missing endpoint.
- CLI `--service=systemd` dispatch remains a runtime Linux-only error on non-Linux targets.
