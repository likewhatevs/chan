# v0.59.0-rc1: rolling release journal

Working journal for the v0.59.0 cycle. Unlike the per-release notes above, this is a rolling doc: appended to as each work stream from `dev/v0.59.0/request.md` lands on its branch, and reconciled into a final `release-0.59.0.md` at cut time. Nothing here is on `main` yet. Each section stands alone so the release summary can be assembled from these entries.

## Work streams (from `dev/v0.59.0/request.md`)

- [x] **`chan devserver` command**: reshape `--service` into explicit action verbs (branch `devserver-cmd`)
- [ ] Graph: grey out unselected nodes, first-order edge focus, auto-select root, `@@mention` "Graph from here" missing edges
- [ ] Index & dashboard: clickable indexing notification to Dashboard, per-path indexing states, no reload on tab switch
- [ ] Editor: directory-link click to file browser, list continuation glyphs, enumerated-list indent, `mermaid-to-excalidraw`
- [ ] Chan desktop: second-monitor hide/show window shrink, window-title glyphs
- [ ] UX: friendlier `cs open` from standalone, unblock `cs download`/`upload` in workspaces

---

## `chan devserver` command: explicit action verbs

**Branch:** `devserver-cmd` (worktree `../chan-devserver-cmd`, off `origin/main`). Not merged. **Status:** complete, gated green, empirically verified end-to-end on all reachable backends.

### The request (verbatim intent)

From `dev/v0.59.0/request.md`, "The `chan devserver` command". The starting behavior of `--service` auto-picks a backend (none / chan on Windows / systemd on Linux / launchd on macOS) and does one overloaded thing: create-or-update the service, restart if flags (port/bind) changed, then monitor `/healthz` to stay blocking (so it can front a tunnel). systemd additionally sets user linger, uses the fdstore to preserve PTYs across restarts, enables on boot, and `--stop` should stop and disable.

What the maintainer wanted:

- `chan devserver --service=none`: the default, `--bind`/`--port`, run in foreground.
- All other modes support `--start` (background), `--stop`, `--status`, `--restart`.
- The default "start-or-restart-if-flags-changed, then attach/block" becomes `--join`.
- If unix-domain sockets are not supported yet, add `--bind={path}` to switch to AF_UNIX and ignore/reject `--port`. The point: "not listen on a port and still make it work" (open to suggestions).

### Decisions (agreed with the maintainer up front)

1. **Defer Unix-domain sockets.** Reason surfaced during exploration: axum 0.7.9's `serve` is hardcoded to `TcpListener` (no generic `Listener` until axum 0.8), and `reqwest` cannot probe `/api/health` over a unix socket, so `--bind=/path.sock` needs a new hyper-util accept loop plus a unix-aware watchdog. Punted to a follow-up; `--bind` stays `Option<IpAddr>`, `--port` stays.
2. **Bare `--service=systemd`/`--service=launchd` requires an explicit verb** (error otherwise).
3. **Only `--join` blocks.** `--start`/`--restart`/`--stop`/`--status` return immediately (a behavior change for `--restart`, which used to attach).

### The deliverable

- **Model.** `--service=none` (default) is plain foreground on `--bind`/`--port`, no supervision. `--service=chan` is the self-managed foreground daemon (pidfile + flock). `--service=systemd`/`launchd` are detached background services. The per-OS auto-pick (`ServiceKind::Auto`) is removed; there is no implicit backend.
- **Verbs (systemd/launchd):** `--start` (write/enable/start, then return), `--stop` (stop and disable, so it does not come back on boot/login), `--restart` (rewrite unit for the current binary/addr, bounce, return; fdstore-preserves live PTYs unless `--force`), `--status`, `--join` (ensure running, start if down or attach if up, then block on the health watchdog; SIGINT detaches and the service keeps running). `--join` is the old default behavior, now explicit.
- **Verbs (chan):** bare `--service=chan` runs the foreground daemon; `--stop`/`--restart`/`--status` act on the pidfile; `--join` attaches to a running daemon (errors if none); `--start` is rejected (chan has no detached background; it is a foreground backend).
- **Validity matrix** is a pure `plan_devserver(service, action) -> Result<DevPlan, String>` plus `selected_devserver_action(...)`, both unit-tested, so the async dispatcher stays thin and every invalid `(service, action)` pair errors with a precise, actionable message.
- **Backend re-slicing (no behavior invented):** the systemd/launchd helpers were split from the overloaded functions. `join_*` is the attach + watchdog path, new `start_*` does the same setup without the watchdog and returns, `restart_*` lost its trailing watchdog, `stop_*` gained `disable` (systemctl disable / launchctl disable).
- **`CHAN_HOME` propagation fix** (bug discovered while setting up the isolated launchd test, see "What didn't"): the generated unit carries `Environment="CHAN_HOME=…"` and the plist an `EnvironmentVariables`/`CHAN_HOME` entry, but only when `CHAN_HOME` is set, so production behavior is unchanged.
- **Callers/docs/examples updated:** launcher connect-script samples got `--join` on the systemd/launchd examples (`demo.ts`, `mock.ts`, `NewWorkspaceDialog.svelte`); `design.md`, `crates/chan/design.md`, `docs/contributing/linux-and-macos.md`, the chan-server and desktop comments, and two user-facing error strings dropped the stale `--systemd`/`--launchd` flags; `CHANGELOG.md` gained Changed + Fixed entries.

**Touched files (11):** `crates/chan/src/lib.rs` (bulk), `crates/chan/src/devserver_daemon.rs`, `crates/chan-server/src/devserver.rs`, `desktop/src-tauri/src/devserver.rs`, `design.md`, `crates/chan/design.md`, `docs/contributing/linux-and-macos.md`, `CHANGELOG.md`, and the three launcher files.

### The tests

Static gate (macOS, all green): `cargo fmt --check`; `RUSTFLAGS="-D warnings" cargo clippy -p chan --all-targets`; `cargo test -p chan --lib` (100, including new `plan_devserver` validity matrix, `selected_devserver_action`, action-group parse, and `CHAN_HOME`-propagation tests for both the systemd unit and the launchd plist, with and without `CHAN_HOME`); `cargo test -p chan --test devserver_resilience` (9 foreground SIGINT/SIGTERM/SIGKILL, flock release, tenant PTY reap, `chan close` sync, all unchanged, confirming the default foreground path is untouched); `cargo build -p chan --no-default-features`; `make web-check` (svelte-check + vitest + build for both SPAs); plus `chan devserver --help` and every error path by hand.

Runtime end-to-end (empirically verified, not just gated):

- **systemd** (lima VM, real `systemctl --user`, aarch64 Ubuntu): bare `--service=systemd` errors; `--start` returns, active + enabled, `/api/health` 200; `--status`; `--restart` returns, still active; `--join` attaches + blocks, SIGINT detaches and the unit survives; `--stop` leaves it inactive and disabled. Re-run with `CHAN_HOME` set: the unit carried `Environment="CHAN_HOME=…"`, systemd accepted it, config isolated to the override dir.
- **chan daemon** (lima VM, flock + pidfile): `--start` rejected; empty-state `--status`/`--join` handled; bare run brings up the foreground daemon with `daemon.json`/`daemon.lock` and health 200; `--join` attach/detach; `--restart` takeover (old pid dies, new pid serves on the preserved port); `--stop` clears the pidfile and the process exits.
- **launchd** (macOS, real `gui/$uid` domain), isolated via `CHAN_HOME` pointed at a throwaway dir: bare errors; all verbs walked; `--start` returns; plist carried `CHAN_HOME`; `--restart` returns; `--join` attach/detach; `--stop` deregistered and disabled; all config/token/log landed in the override and `~/.chan/devserver` was never created; plist + agent removed on cleanup.

### What worked

- The reshape was mostly a re-slice of already-verified building blocks, so behavior parity held: the foreground resilience suite passed untouched, and `--join` reproduces the old default exactly.
- All four backends (none, chan, systemd, launchd) verified against real supervisors, including the two behavior changes that mattered most: `--start`/`--restart` return, and `--stop` disables.
- `CHAN_HOME` isolation genuinely works for supervised services, proven by the launchd run leaving the real `~/.chan` completely untouched.

### What did not work / issues found

- **`CHAN_HOME` split-brain bug (found + fixed).** Setting `CHAN_HOME` on the supervisor alone was insufficient and actually broken: launchd/systemd spawn the service with a fresh environment, so the service used the real `~/.chan` while the supervisor read the isolated config, the token handshake would time out, and `--start` would fail. Fixed by baking `CHAN_HOME` into the unit/plist. This is why isolating the launchd test required a code change rather than just an env var.
- **Unix-domain sockets deferred**, not delivered. The `--bind={path}` ask is unmet this round (axum 0.7.9 is `TcpListener`-only and reqwest cannot probe a unix socket). Needs a hyper-util accept loop plus a unix-aware watchdog; tracked as a follow-up.
- **`chan --service=chan --restart` blocks** (it re-serves in the foreground). Inherent to a foreground backend; the "returns" contract only applies to systemd/launchd. Documented.
- **launchd is not CI-reachable** (needs a macOS GUI login domain), so it can only be verified locally, which was done here. systemd is likewise not in CI (no user manager); the lima VM is the exercise path.
- **VM full build snag (worked around):** `cargo build -p chan` in the aarch64 lima VM fails in candle's `gemm-f16` (inline asm needs the `fullfp16` CPU feature). Not our code; sidestepped for testing with `--no-default-features` (drops candle; BM25 search and the whole devserver remain). Flagging in case the aarch64-linux release build hits the same.
- **Pre-existing VM state surprise:** an old `chan-devserver.service` (from earlier manual testing, on port 9800, pointing at `~/.local/bin/chan`) was already active and masked the first clean `--start`; `--start` correctly reported "already running" and returned. Cleared it (which also exercised `--stop`=stop+disable) before the clean run.

### Follow-ups

- Unix-domain-socket `--bind=/path.sock` (deferred). Likely an axum 0.8 upgrade or a scoped hyper-util accept loop for the unix path, plus a unix-socket health probe for the supervised watchdog.
- Consider whether the aarch64-linux release build needs a `gemm`/`fullfp16` target-feature or `--no-default-features` accommodation (separate from this work).

---

## Session notes (process retrospective)

Honest lowlights from the agent (me) this session, worth recording so the pattern does not repeat:

- **Hard-wrapped Markdown.** I first wrote this journal wrapped at ~80 columns. House style for `.md` is free-flowing prose (one paragraph or bullet per line; only tables stay near ~80 cols). Rewrote unwrapped, and captured the rule in memory.
- **Introduced em dashes.** My first-pass comments, docs, and this journal used the `—` character, against the no-em-dash house rule. Fixed my own additions and, at the maintainer's direction, a follow-up commit purges the pre-existing em dashes in the touched files.
- **Scope discipline held elsewhere:** deferred unix sockets up front rather than half-building them, and kept `~/.chan` untouched while testing launchd (via `CHAN_HOME` isolation, which surfaced the propagation bug).
