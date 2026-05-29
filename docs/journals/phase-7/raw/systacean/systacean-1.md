# systacean-1: chan open CLI

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-18

## Goal

Add a new `chan open <path>` subcommand that, when invoked
from inside a chan-spawned terminal, instructs **the
chan-server of the same window** to open the path:

* `.md` files load as a new editor tab (create the file if it
  doesn't exist, atomically through `chan_drive::Drive`).
* Directories or non-`.md` files load the file browser at
  that path.

The CLI must respect cross-window restrictions: a `chan open`
from window A's terminal opens in window A only, not in some
other chan window running on the same machine.

## Relevant links

* [../request.md](../request.md) Enhancements, the `chan open`
  bullet (most recent enhancement, references
  `$CHAN_TAB_NAME` / `$CHAN_DRIVE_NAME` / session-id options).
* [../architect/journal.md](../architect/journal.md) "Extended
  requests" trail.
* CLAUDE.md "MCP server only" section — chan-server already
  exposes an in-process MCP server over a Unix-domain socket
  via `crates/chan-server/src/mcp_bridge.rs`. Terminal sessions
  receive `CHAN_MCP_SERVER_JSON` plus companion
  `CHAN_MCP_*` discovery env vars. This is the obvious
  transport for `chan open`.
* `crates/chan/src/main.rs` — clap definitions + `cmd_*`
  dispatchers; that's where the new subcommand lands.
* `crates/chan-server/src/routes/` — likely needs a new
  handler if MCP isn't a natural fit (it should be).

## Acceptance criteria

### CLI surface

* `chan open <path>` subcommand. `<path>` can be relative
  (resolved against the current shell working directory) or
  absolute.
* `chan open -h` shows accurate help (no false claims).
* Behavior when *not* inside a chan-spawned terminal: exit
  non-zero with a clear message ("not running inside a chan
  session; chan open requires `$CHAN_SESSION_ID` (or whatever
  you settle on) to identify the window"). Do not guess.

### Window/session identity

* Decide the env-var contract. Propose in an append to this
  task file before implementing; @@Architect signs off via
  event. Constraints:
  * Must identify *which window* / chan-server to talk to.
    `$CHAN_TAB_NAME` is per-PTY and not enough alone.
  * Multi-drive: if more than one drive is registered, we may
    also need the drive name. Decide if `chan open` is
    drive-scoped (path resolves against the calling
    terminal's drive) or absolute-path-scoped.
  * Multi-window: distinct chan-server processes on the same
    machine must not collide. The MCP socket path or the
    bearer token from launch already distinguish them; pick
    one and document the choice.
* Implement: chan-server exports the chosen env var into
  every PTY it spawns; the CLI reads it and connects.

### `.md` file open

* `chan open foo.md` (existing file): opens a new editor tab
  in the target window. Does not modify file contents.
* `chan open new.md` (missing file): atomically creates an
  empty `new.md` via `chan_drive::Drive::write_text` first,
  then opens it. Refuses if the path is outside the drive
  sandbox.
* `.md` is the only file-create surface; for other
  extensions, missing files are an error ("file does not
  exist; chan open creates `.md` files only").

### Directory / non-`.md` open

* `chan open ./images` (directory): opens the file browser
  at that path.
* `chan open ./photo.png` (non-`.md` file): opens the file
  browser at the file's parent directory and selects the
  file. Reason: chan's editor doesn't render non-text files
  natively, but the browser is the natural surface.

### Transport

* Prefer the existing MCP UDS socket; add a tool method on
  the in-process MCP server (`crates/chan-llm/src/mcp/...`)
  that the chan binary calls. Reusing MCP keeps the trust
  boundary contained to one socket.
* If MCP doesn't fit naturally (e.g., the tool surface is
  intended for *external* agents and `chan open` is a
  first-party action), call out the reasoning in an append
  and propose an alternative (a small admin socket on
  chan-server) before implementing.

### Shell completion

* Generate completions for bash + zsh (clap_complete handles
  this).
* Completion for `chan open` lists files and directories
  relative to the caller's CWD; do not bias toward drive
  contents (CWD is where shell completion runs anyway).
* Defer fish completion if it's not free from clap_complete.

### Tests

* Unit test the env-var detection + the error case
  (`chan open` outside a chan terminal).
* Integration test the create-then-open path against a
  throwaway drive.

## Out of scope

* In-editor "Link to File" UI in the rich prompt — separate
  wave-2 fullstack task.
* Re-using `chan open` for arbitrary CLI invocations from
  outside terminals.

## How to start

1. Propose the env-var contract + transport choice (MCP vs
   admin socket) in an append below. Wait for @@Architect
   sign-off via event.
2. Wire the env var into the PTY spawn path in chan-server.
3. Implement the CLI subcommand in `crates/chan`.
4. Plumb the call into chan-server through the chosen
   transport.
5. Tests + completion.

## Hand-off

Pre-push gate must pass green: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test`,
`cd web && npm run build` (no svelte-check change needed
unless you touch frontend).

When ready for walkthrough or review, append "Specialist
review requested" and fire
`alex/event-systacean-architect.md` (type `poke`).

## 2026-05-18 11:34 BST - Env contract and transport proposal

Proposal:

* Export `CHAN_WINDOW_ID` into every chan-spawned PTY. Value is the
  frontend session/window id from `?w=<id>` or the browser-tab generated
  session id. This is the routing key for "same window".
* Continue exporting `CHAN_TAB_NAME` as display context only. It is not
  identity and must not route `chan open`.
* Reuse the existing `CHAN_MCP_SOCKET` as the per-server capability and
  drive identity. The socket path already distinguishes concurrent
  `chan serve` processes on the same machine and avoids drive registry
  guessing. `chan open` outside a chan terminal exits non-zero if either
  `CHAN_WINDOW_ID` or `CHAN_MCP_SOCKET` is missing.
* Resolve `<path>` against the caller's current working directory in the
  CLI, canonicalize enough to determine whether it maps under the served
  drive, then send the absolute path plus `CHAN_WINDOW_ID` to the server.
  The server converts to a drive-relative path and applies
  `chan_drive::Drive` gates for existence, file creation, and sandboxing.

Transport recommendation:

* Do not add this to the existing MCP tool set. The MCP bridge currently
  exposes drive tools for external agents, and `chan open` is a
  first-party UI/window command. Adding a UI-side-effect tool would mix
  agent drive access with frontend command dispatch.
* Add a small first-party admin message over the same UDS listener or a
  sibling UDS listener owned by `chan-server`. My preference is a sibling
  `CHAN_CONTROL_SOCKET` because it keeps the MCP protocol pure and lets
  `chan open` use one JSON request/response without pretending to be an
  MCP client.
* The control request shape can be:

```json
{"type":"open_path","window_id":"...","path":"/abs/or/cwd-resolved/path"}
```

  Response is a small JSON status or error. The server then broadcasts a
  window-targeted WebSocket event, e.g.
  `{"type":"window_command","window_id":"...","command":"open_path",...}`.
  The frontend ignores commands whose `window_id` does not match
  `sessionWindowId()`.

Why this satisfies the cross-window rule:

* The socket selects the `chan-server` process and drive.
* `CHAN_WINDOW_ID` selects one frontend window within that server.
* Browser tabs or multiple Tauri windows for the same drive no longer
  collide, because they have distinct session/window ids.

Implementation impact after sign-off:

* Add `window_id` to terminal WebSocket creation from the frontend and to
  `CreateOptions`.
* Export `CHAN_WINDOW_ID` and `CHAN_CONTROL_SOCKET` in PTY env.
* Add the control socket, CLI subcommand, server open-path handler, and
  frontend `window_command` handling.
* Add tests for missing env, PTY env export, path classification, and
  markdown create-through-drive behavior.

## 2026-05-18 — @@Architect review: APPROVED for commit (gated on @@Alex)

Implementation review of the 12:21 BST commit-readiness append below.

End-to-end including the frontend `window_command` handler — nicely beyond
the original scope split. I had `fullstack-5` queued for that piece;
**no longer needed**, you took it. Credit to @@Systacean.

All four amendments are in:

1. `CHAN_DRIVE_NAME` exported as display-only — verified.
2. Browser-tab session-id fallback wired (sessionStorage) — verified.
3. "no such window" error path on the control socket — verified.
4. `chan open` only requires `CHAN_WINDOW_ID` + `CHAN_CONTROL_SOCKET`,
   not `CHAN_MCP_SOCKET` — verified.

Verification chain (cargo fmt, clippy `-D warnings`, full `cargo test`,
svelte-check, npm build, completions generate clean) is comprehensive.

Flagged risks acknowledged:

* **Cross-task overlap in `web/src/state/store.svelte.ts`** — your additions
  (`window_command` state) coexist with @@FullStack's side-pane state from
  `fullstack-1`. Both already live in the working tree without conflict;
  whichever commits first, the other is a trivial merge. We commit
  `systacean-1` first (it's smaller in surface area outside the shared file
  and the side-pane piece still wants a webtest walkthrough).
* **No live browser walkthrough** — fair. I'm queueing a small follow-up
  walkthrough task for @@WebtestB (terminal lane) once they're stable on
  webtest-b-1, since `chan open` interacts most heavily with the terminal
  side. Tracking as a future `webtest-b-N` cut, not a blocker for commit.

Trailing-whitespace catch in `docs/journals/phase-7/alex/journal.md` —
fixed (was a stray space after `Date: 2026-05-18 ` plus a casing nit on
the title); `git diff --check` should pass now.

### Commit clearance

**APPROVED from @@Architect's side.** Per project rule (CLAUDE.md) only
@@Alex authorizes commits, so **hold the commit until @@Alex says go**. I'm
relaying this to @@Alex via chat now.

Proposed commit message looks good as drafted. One micro-tweak suggestion
if you want it (non-blocking):

```text
Add window-scoped chan open

Add a first-party control socket for chan-spawned terminals, export
CHAN_WINDOW_ID, CHAN_CONTROL_SOCKET (and CHAN_DRIVE_NAME for display)
into PTY environments, and add the chan open CLI command. Markdown
paths open editor tabs and missing markdown files are created through
chan-drive before opening; other paths route to the file browser.
Frontend filters window_command broadcasts by window_id. Add bash/zsh
completion generation through clap_complete.
```

(adds the display-only `CHAN_DRIVE_NAME`, the frontend filter behavior,
and the bash/zsh-specific completion line).

### What's next for you

Move on to `systacean-2` (write-timeout investigation) while we wait
for commit clearance. Don't touch systacean-1's branch until @@Alex
authorizes.

## 2026-05-18 12:21 BST - Commit readiness

Implementation complete after @@Alex approval in chat.

Files changed for this task:

* `Cargo.toml`
* `Cargo.lock`
* `crates/chan/Cargo.toml`
* `crates/chan/src/main.rs`
* `crates/chan-server/src/control_socket.rs`
* `crates/chan-server/src/lib.rs`
* `crates/chan-server/src/mcp_bridge.rs`
* `crates/chan-server/src/routes/terminal.rs`
* `crates/chan-server/src/state.rs`
* `crates/chan-server/src/terminal_sessions.rs`
* `web/src/components/TerminalTab.svelte`
* `web/src/state/store.svelte.ts`
* `web/src/terminal/session.ts`
* `web/src/terminal/session.test.ts`

Behavior:

* Added `chan open <path>`.
* Added `CHAN_WINDOW_ID` and `CHAN_CONTROL_SOCKET` to chan-spawned PTYs.
* Kept MCP pure; first-party UI commands use a sibling Unix control socket.
* Existing `.md` files open as editor tabs; missing `.md` files are created
  through `chan_drive::Drive::write_text` before opening.
* Directories open the file browser; non-`.md` files open the browser at the
  parent directory and select the file.
* Added `chan completions <shell>` through `clap_complete`; `open` path
  argument carries `ValueHint::AnyPath`.

Verification:

* `cargo fmt --check`
* `cargo check -p chan-server -p chan`
* `cargo test -p chan open_env_requires_window_id_and_control_socket`
* `cargo test -p chan-server control_socket`
* `cargo test -p chan-server open_path_creates_markdown_and_broadcasts_window_command`
* `cargo test -p chan-server conditional_pty_programs_validate_real_terminal -- --nocapture`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test`
* `cd web && npm test -- --run src/terminal/session.test.ts`
* `cd web && npm run check`
* `cd web && npm run build`
* `cargo run -p chan -- completions bash`
* `cargo run -p chan -- completions zsh`

Scoped whitespace check passed for the task files. Full `git diff --check`
currently fails on unrelated trailing whitespace in
`docs/journals/phase-7/alex/journal.md`.

Known risks:

* No live browser walkthrough was run by this agent; the frontend event path is
  covered by TypeScript validation and the server-side broadcast payload test.
* `web/src/state/store.svelte.ts` also contains unrelated side-pane changes from
  another task; this task only added `window_command` handling there.

Proposed commit message:

```text
Add window-scoped chan open

Add a first-party control socket for chan-spawned terminals, export
CHAN_WINDOW_ID and CHAN_CONTROL_SOCKET into PTY environments, and add
the chan open CLI command. Markdown paths open editor tabs and missing
markdown files are created through chan-drive before opening; other
paths route to the file browser. Add shell completion generation.
```

## 2026-05-18 — @@Architect sign-off

Proposal APPROVED with four small amendments. Proceed to
implementation (Rust + CLI). The frontend bits (window_id in
WebSocket creation + window_command handler) will be a small
@@FullStack task cut once you finalize the wire protocol;
flag me when the JSON shape is locked.

### Amendments

1. **Keep `CHAN_DRIVE_NAME` as display-only.** Export it
   alongside `CHAN_TAB_NAME` so users can put it in shell
   prompts (`echo $CHAN_DRIVE_NAME`). It must NOT route
   `chan open` — that's still `CHAN_WINDOW_ID` +
   `CHAN_CONTROL_SOCKET`. This is a courtesy export, no
   logic.
2. **Make the browser-tab fallback explicit.** Tauri windows
   carry `?w=<label>`; browser tabs don't. The server must
   accept and round-trip a client-generated session id (e.g.,
   stored in `sessionStorage`, generated on first connect)
   when the URL has no `w=`. Document this fallback in the
   PTY env wiring so future readers don't assume Tauri-only.
3. **Add a "no such window" error path.** If `chan open`
   targets a `window_id` that's no longer connected (window
   closed, server restarted), the control-socket response is
   `{"type":"error","code":"no_such_window","window_id":"..."}`
   and the CLI prints "target chan window is no longer open"
   then exits non-zero. Don't silently succeed.
4. **Control-socket discoverability.** Export
   `CHAN_CONTROL_SOCKET` next to `CHAN_MCP_SOCKET` (same env
   shape). The CLI only needs the control socket; do NOT
   require `CHAN_MCP_SOCKET` for `chan open` — that
   conflates first-party UI with the agent-facing surface.
   So drop the "exits non-zero if `CHAN_MCP_SOCKET` is
   missing" check; only require `CHAN_WINDOW_ID` +
   `CHAN_CONTROL_SOCKET`.

### Design pieces that look right and don't need changes

* Separate `CHAN_CONTROL_SOCKET` rather than a new MCP tool —
  correct call. Trust boundary stays clean.
* `{"type":"open_path","window_id":...,"path":"..."}` request
  shape — fine. Add the error shape from amendment #3.
* Window-targeted WebSocket broadcast with client-side filter
  — acceptable for round 1. Server-side routing by window_id
  is a follow-up if connection counts grow.
* Path canonicalization in CLI + re-validation server-side
  through `chan_drive::Drive` gates — defense in depth, keep.

### Implementation order suggestion (non-binding)

1. Export `CHAN_WINDOW_ID`, `CHAN_DRIVE_NAME`,
   `CHAN_CONTROL_SOCKET` in PTY env. Wire `window_id` into
   terminal `CreateOptions` + WebSocket creation
   client-side. Tests for env presence.
2. Add the control socket listener + open_path handler.
   Tests with a mock window_id.
3. CLI subcommand + clap_complete for bash/zsh.
4. Frontend `window_command` handler — cut as
   `fullstack-N` once the wire shape is final; @@FullStack
   handles it.

### Out-of-scope reminders

* "Link to File" UI in the rich prompt is a separate wave-2
  task (request.md mentions it under the rich-prompt
  bullet); not your scope here.
