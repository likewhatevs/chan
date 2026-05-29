# @@Systacean task 5: PTY session registry in chan-server

Owner: @@Systacean
Status: REVIEW
Depends on: [systacean-2](./systacean-2.md) (clean indexer baseline,
no overlap), [backend-2](./backend-2.md) (per-window session blob
shape we'll piggy-back on).
Source: [architect-tmux-1](./architect-tmux-1.md), Option 4
(confirmed by Alex).
Coordinates with: [frontend-4](./frontend-4.md) on the wire contract.

## Goal

Decouple PTY lifetime from the WebSocket so terminal tabs survive
browser/window reloads. chan-server owns a registry of long-lived
PTY sessions; the WebSocket becomes a pure attach/detach transport.

## Wire contract (Systacean ↔ Frontend)

Define together with @@Frontend before either lane lands code. First
draft:

* `GET /api/terminal/ws?session=<id>&since=<seq>&cols=<n>&rows=<n>&tab_name=<...>`
  * `session` optional. If absent or unknown, create a new session
    and report the assigned id in the first server frame.
  * `since` optional. If present, replay ring slice with `seq > since`
    before resuming live output.
  * `cols` / `rows` set initial PTY size; subsequent resize comes via
    a control frame (existing terminal protocol).
  * `tab_name` continues to populate `CHAN_TAB_NAME` in the PTY env.
* First server frame after upgrade: a JSON control frame with
  `{type:"session", id:<string>, seq:<u64>, missed_bytes:<u64>}`.
  `missed_bytes > 0` tells the client some scrollback fell off the
  ring; client can show a banner.
* Output frames are binary (current shape) with an implicit
  monotonic seq tracked on the server side; clients store the last
  `seq` they processed and replay against it on reattach.
* Server-initiated close: control frame
  `{type:"closed", reason:"idle"|"drive"|"shutdown"|"explicit"|"capped"}`
  before the WebSocket closes, so the client UX can surface the
  reason cleanly.
* Client explicit close: text control frame `{type:"close"}`. This is
  sent only for user-driven terminal close / restart; browser reload
  detaches without sending it.
* `seq` / `since` are byte offsets. After the initial `session.seq`
  baseline, the client advances `last_seq` by the byte length of each
  binary output frame it processes.

Reconcile this draft with @@Frontend in [frontend-4](./frontend-4.md)
before either lane writes the code.

## Scope

### Registry module

* New module: `crates/chan-server/src/terminal_sessions.rs`.
* `pub struct Registry` lives on `AppState`. Methods:
  * `create(opts) -> SessionHandle` — spawns the PTY, the stdout-
    pumping tokio task, the broadcast channel, and the ring buffer.
    Picks an unguessable random id (use the same RNG style as
    `pick_socket_path` in `mcp_bridge.rs`).
  * `attach(id, since) -> Option<AttachHandle>` — returns the
    broadcast receiver + a snapshot of ring bytes after `since`
    + the current `seq`. None if id unknown.
  * `send_input(id, &[u8])` — writes to the PTY master.
  * `resize(id, cols, rows)` — updates winsize on the PTY master;
    broadcasts a control frame to other attachees so they can
    `fit()` to match.
  * `close(id, reason)` — kills the PTY, drops the entry,
    broadcasts a closed control frame.
  * `prune_idle()` — called periodically from a background tokio
    task; closes sessions with no attached clients AND no PTY
    output for the configured idle timeout.
* `pub struct Session`:
  * `pty_master: Box<dyn MasterPty + Send>` (portable-pty),
  * `child: Box<dyn Child + Send>`,
  * `output_tx: broadcast::Sender<Bytes>` (live stream),
  * `ring: Mutex<RingBuffer>` (recent bytes for replay),
  * `seq: AtomicU64`,
  * `last_activity: AtomicI64` (unix seconds; updated on input or
    output, not on attach/detach),
  * `attach_count: AtomicUsize`,
  * `winsize: Mutex<PtySize>`.

### Ring buffer

* Bounded byte ring (default 1 MiB, configurable via the same config
  story below). Tracks a `start_seq` offset so attachers with
  `since < start_seq` get a `missed_bytes = start_seq - since`
  number.
* On overflow, drop oldest bytes; advance `start_seq` accordingly.

### Lifecycle

* **Create.** Spawn under the drive root cwd, with the existing PTY
  env (HOME, CHAN_TAB_NAME, CHAN_MCP_*, *_MCP_SERVER_JSON, etc.).
* **Attach.** Stream replay then live broadcast. Multiple attachees
  share input/output. Increment `attach_count`.
* **Detach.** WebSocket close → decrement `attach_count`. PTY keeps
  running. Do not reset `last_activity`.
* **Idle prune.** Background task wakes every ~minute, closes
  sessions where `attach_count == 0 && now - last_activity >
  idle_timeout`. Reason: `"idle"`.
* **Drive close.** Iterate all sessions for the drive, close with
  reason `"drive"`. Single-drive instance today, so this is just
  "shutting down the registry."
* **Server shutdown.** Same as drive close, reason `"shutdown"`.
* **Explicit close.** Client sends a control frame requesting close
  → registry closes with reason `"explicit"`.
* **Cap exceeded.** Creation past the soft cap returns an error
  surfaced as a `{type:"error", reason:"capped"}` frame, then close.

### Route rewrite

* `crates/chan-server/src/routes/terminal.rs` becomes a thin handler:
  parse query, decide create vs attach, hand off to `Registry`, and
  bridge socket frames in both directions until either side closes.
* All the existing env-var construction (`CHAN_MCP_SERVER_JSON` and
  friends) moves into the `Registry::create` path so it applies once
  per session, not per WebSocket attach.

### Config

* `terminal.idle_timeout_secs` (default 1800).
* `terminal.session_cap` (default 32).
* `terminal.ring_bytes` (default `1 << 20`).
* Threaded through `crates/chan-server/src/config.rs` and surfaced on
  `/api/config` for read (no UI gating this phase).

## Acceptance criteria

* All five lifecycle paths covered by unit tests (create / attach /
  detach / idle expiry / drive close).
* Ring overflow test asserts `missed_bytes > 0` is reported.
* Two simultaneous attaches on the same id share IO correctly.
* The existing `conditional_pty_programs_validate_real_terminal`
  test (the one Webtest A baselined against) still passes.
* Full pre-push gate green: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`,
  `cargo build --no-default-features`, `cargo test`,
  `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build`.

## Hardening expectations

* Re-read the WebSocket-bridge inner loop with a "client sends a
  flood of input then disconnects" mental model — the broadcast
  channel must not back-pressure the PTY stdout reader.
* The session id RNG must be cryptographically strong (`rand::thread_rng`
  + 16 bytes is enough; do not roll a counter-based id).
* Confirm `mcp_bridge`'s socket path / cleanup is unaffected.

## Progress

* 2026-05-17 @@Systacean: picked up after update check found the new
  terminal persistence task. Inspecting current one-WebSocket-one-PTY
  route and AppState lifecycle before introducing the registry.

## Completion notes

* Implemented `crates/chan-server/src/terminal_sessions.rs` and put
  a shared `TerminalRegistry` on `AppState`. PTY lifetime now belongs
  to the registry, not the WebSocket; attach handles only increment /
  decrement attachment count and bridge replay + live output.
* Wire contract reconciled with [frontend-4](./frontend-4.md):
  first frame is `{type:"session", id, seq, missed_bytes}`, replay
  output is binary, `seq` / `since` are byte offsets, `{type:"close"}`
  means explicit user close, and server close frames carry
  `idle` / `drive` / `shutdown` / `explicit` / `capped` reasons.
  The old `{type:"ready"}` frame is still sent after replay for
  backward compatibility with the existing terminal client.
* Added `[terminal]` config defaults:
  `idle_timeout_secs = 1800`, `session_cap = 32`,
  `ring_bytes = 1048576`; exposed through server config,
  preferences, `/api/config`, web API types, and `chan config`
  get/set keys.
* Lifecycle coverage:
  `ring_overflow_reports_missed_bytes`,
  `session_ids_are_hex_and_distinct`,
  `prune_idle_removes_detached_sessions`,
  `cap_exceeded_refuses_create`,
  `drive_close_removes_sessions`,
  `two_attaches_share_io`, plus the existing
  `conditional_pty_programs_validate_real_terminal`.
* Full gate green: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`,
  `cargo build --no-default-features`, `cargo test`,
  `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build`.
