# backsystacean-10: expose PTY CWD on terminal session metadata

Owner: @@Backsystacean
Status: REVIEW

## Goal

Expose the live current working directory of a terminal's PTY
process so the frontend can fill the CWD-dependent menu rows
that [frontend-2](./frontend-2.md) added but had to leave as
fallbacks. Without this, "Copy path to CWD", "Show Dir",
"Graph dir", and the CWD-seeded "New File" all render but show
the placeholder status `PTY did not report CWD` instead of
acting.

## Background

[frontend-2](./frontend-2.md) (REVIEW) shipped the new
terminal right-click menu with the CWD-dependent rows wired and
the fallback status in place. The rows render today; they need
this backend lane to function.

The shell's CWD is observable from the parent process via
`readlink /proc/<pid>/cwd` on Linux and
`proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, ...)` on macOS. The
terminal session registry already tracks the child PID; this
lane just publishes the resolved CWD as part of the session
metadata.

## Scope

* Add a `cwd` field to the terminal session metadata that
  `mcp_bridge`-style consumers and the WebSocket terminal route
  surface to the client.
* CWD discovery is platform-specific:
  * Linux: `std::fs::read_link("/proc/<pid>/cwd")`.
  * macOS: `libproc::proc_pidinfo(pid, PROC_PIDVNODEPATHINFO,
    ...)` or a small wrapper using `libproc` crate (already
    pulled in by chan-server for terminal work, or add it if
    not).
  * Other Unix: best-effort, return `None` if unavailable.
  * Windows: not supported this phase; return `None`.
* CWD is recomputed on demand (per request from the frontend
  or on a low cadence). The shell can `cd` at any time, so a
  cached value goes stale fast. Recompute on every poll is
  fine; the syscall is cheap.
* Surface: extend the existing per-session metadata endpoint
  (or add `cwd` to the frame the WebSocket already emits on
  attach / status). Frontend can read it on-demand when the
  user opens the right-click menu.
* CWD path is **absolute** filesystem path. The frontend
  converts to drive-relative by stripping the drive root
  prefix; if the CWD escapes the drive root, treat as
  unavailable (display the fallback). This matches the
  chan-drive sandbox philosophy: only paths inside the drive
  are addressable in the UI.

## Out of scope

* Following the CWD via inotify / kqueue (no need; on-demand
  syscall is fast enough).
* Windows support.
* CWD for foreground child processes that aren't the shell
  itself (the shell's CWD is the natural surface; if the user
  runs `cd && vim`, vim inherits and reports its own CWD which
  equals the shell's).

## Acceptance criteria

* Terminal session metadata includes a `cwd` field, either
  populated with the absolute path or `null` when unavailable.
* On the seeded test drive: opening a terminal, running
  `cd code`, then opening the right-click menu, the CWD-
  dependent rows act on `code/` rather than showing the
  fallback.
* CWD outside the drive root resolves to `null` (fallback) on
  the client side.

## Tests

* `cargo test -p chan-server terminal` covers a fresh PTY's
  CWD field on Linux + macOS (unix-gated; the test can
  `chdir` the shell via stdin and then probe the metadata).
* Pre-push gate green.

## Review and hardening

* @@Backsystacean self-review for the `libproc` / `/proc`
  fallback semantics on weird platforms (kfreebsd, illumos —
  return `None`, don't error).
* @@WebtestA live verification: cd in a terminal, open the
  right-click menu, confirm the rows route to the expected
  path.

## Relevant links

* Frontend consumer: [frontend-2](./frontend-2.md)
* Terminal session registry:
  `crates/chan-server/src/terminal_sessions.rs`
* Terminal route:
  `crates/chan-server/src/routes/terminal.rs`

## Progress notes

* 2026-05-18 - Stored the spawned PTY child PID on `Session`
  and added `AttachHandle::cwd()`. Linux uses
  `/proc/<pid>/cwd`; macOS uses a safe on-demand `lsof` probe
  because `chan-server` forbids `unsafe_code`, which rules out
  direct `proc_pidinfo` FFI in this crate. Other platforms return
  `None`.
* 2026-05-18 - Added WebSocket `cwd` request/response frames and
  included `cwd` on the initial `ready` frame. The server only
  reports CWDs that resolve inside the drive root.
* 2026-05-18 - Wired the terminal menu's CWD-dependent actions:
  copy CWD path, reveal directory in the file browser, graph
  directory, and seed New File from the terminal CWD.

## Completion notes

Ready for review. Verification:

* `cargo test -p chan-server terminal -- --test-threads=1`
* `cargo test -p chan-server -- --test-threads=1`
* `npm run check` from `web/`
* `scripts/pre-push`
