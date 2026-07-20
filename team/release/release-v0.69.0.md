# Release v0.69.0

Delivery round run 2026-07-15: four implementation lanes plus an integrator on one shared worktree (branch `0.69.0-rc1`), fully unattended from investigation to rc1. Coordination artifacts live in the untracked `dev/v0.69.0/` tree of the round host's checkout.

## Scope

Seven items (one withdrawn as a machine-local non-defect). Desktop: gateway devserver windows (`https://*.devserver.chan.app`) gain the full workspace IPC vocabulary through one origin-scoped capability - upload picker, Downloads saves, all six clipboard commands, reload/zoom/devtools chords - guarded by a new origin-aware ACL parity test that recomputes every capability's reach against the SPA's audited invoke vocabulary. CLI/SPA: `cs paste` gets a typed 30s timeout (exit 124), a two-second waiting notice, single-access clipboard reads, and a chan-owned in-window Paste/Cancel card replacing the browser's unanswerable floating prompt. Gateway: profile-service sweeps devservers offline longer than `DEVSERVER_RETENTION_MINUTES` (default 15) from a proxy-snapshot liveness mark, never deleting on a failed snapshot tick. Connectivity: an app-level ping/pong heartbeat on `/ws` plus read-deadlines and a wall-clock wake detector turn post-sleep zombie windows into the existing Reconnecting overlay within a minute, recycle terminal PTY sockets losslessly, and drive a new `Unreachable` devserver status (red launcher dot + disconnect button) off the feed sockets rather than the always-green fresh-dial poll. Launcher: machine cards collapse behind a window-count toggle whose state survives reloads and desktop restarts (config-backed store); a global Open command runs `cs open` semantics from a path dialog or inline `Open <path>` with strict focus restore. Wording: one-word "devserver" across labels, docs, site, and comments, grep-enforced at the round gate.

## Branch And Commits

`0.69.0-rc1` cut from v0.68.0 (`1aa9c7aa`); 25 commits to the rc tip `37d68023`; rc pins at `8fd00d21`. The GA commit strips the rc pins, dates the changelog, pins the fedora specs, and adds this document.

## Validation

An 8-agent investigation pass produced the lane briefs (verified anchors, pre-taken rulings); a 4-agent adversarial pass re-checked every anchor before spawn and caught four brief defects pre-launch. Every commit was own-gated by its lane and re-verified by the integrator from an isolated worktree. Committed e2e growth this round: gateway-zone gained a scenario dispatcher plus `sweeper` (7 asserts: stale delete, grant cascade, live mark, redial) and `watchdog` (7 asserts: keepalive answered, frozen-socket silence, no-onclose zombie, SIGCONT heal) scenarios; browser-smoke gained paste grant/deny, launcher-open (dialog, inline arg, binary error), and collapse-persistence checks. Final composite gate on the tip: full `make pre-push` green, full gateway cargo tests against Postgres, gateway-zone all scenarios all-assertions-passed, browser-smoke 9/9, wording grep clean. The `release.yml publish=false` dry run (29420257505) passed on all platforms including macOS sign/notarize; artifacts validated locally (CLI executes at version, deb/rpm metadata correct, DMG + updater signature present).

The sleep-wake mechanism was empirically reproduced before implementation (SIGSTOP'd proxy relay: half-open sockets never fire onclose while fresh dials succeed; the 300s bridge cut arrives as a delayed close only after resume). Two flake classes surfaced mid-round and both got mechanism fixes: the PTY probe tests matched their own markers inside the terminal echo (fixed class-wide with runtime-built markers, proven at 2.4x-nproc load), and a web-vitest parallel-timeout class was contained by the documented isolated-rerun rule.

## Release Workflow

rc validation via `publish=false` dispatch (run 29420257505). GA is the standard tag-push publish; downstream publication fires on the release run's completion (COPR + PPA verified post-publish).

## Operators

See the CHANGELOG Operators section: optional `DEVSERVER_RETENTION_MINUTES` on profile-service (sweeper requires `DEVSERVER_ADMIN_TOKEN`/`DEVSERVER_ADMIN_URL`; a sweep deletes the row's shares and label), `cs paste`/`cs copy` timeout is exit 124, front proxies must not deny `clipboard-read` via Permissions-Policy, and desktops now grant native IPC to windows on the devserver wildcard origin.

## Known Limitations

Self-hosted gateways on domains other than `devserver.chan.app` do not receive the desktop IPC grants (static origin list; runtime-minted capabilities are the follow-up). The per-gesture upload-token hardening remains deferred. Desktop auto-recovery from a >24h expired gate cookie is manual (the overlay's Reconnect button re-mints). A devserver swept while merely powered off loses its label and shares permanently until re-granted. `read_dropped_paths` stays excluded from devserver windows on every origin by design.
