# @@Architect task tmux-1: terminal session persistence design memo

Owner: @@Architect (memo); implementation split across @@Systacean
and @@Frontend (see "Implementation handoff" below).
Status: DECIDED — Alex confirmed Option 4 (chan-native PTY session
registry, no external compatibility). Implementation task files
follow.

## Goal

Pick an implementation direction for "every terminal tab we create is
backed by a tmux `-CC` tab as well", so that reloading a browser
window or chan-desktop window does not kill the tabs.

The request line:

> Reloading windows with terminal tabs will likely kill the tabs;
> we need to integrate with tmux's `-CC` protocol natively; this
> means the backend server will require tmux to exist (or if there
> is a decent rust implementation of tmux CC (we want to be
> compatible, too) we could use it instead of forking; other than
> that, the idea is that every tab we create is backed by a tmux
> `-CC` tab as well.

## Background

* Today, `crates/chan-server/src/routes/terminal.rs` spawns a raw
  `portable-pty` per tab. Closing the WebSocket terminates the PTY
  (sigchld, etc), so a reload kills the shell.
* tmux's `-CC` (control mode) speaks a documented line-oriented
  protocol that lets a client list / attach / detach / multiplex
  sessions hosted by a long-lived `tmux server`. iTerm2 and a few
  Rust crates already implement consumer sides.
* The session lifetime moves from "chan-server process" to "tmux
  server process". chan-server orchestrates which tab maps to which
  tmux session/window, and pipes IO through the control-mode socket.

## Approach options

Three viable shapes, each with a different cost / portability profile:

1. **External tmux dependency.** chan-server invokes the user's
   `tmux` binary in `-CC` mode and treats it as the session backend.
   * Pro: minimum code, maximum compatibility with the ecosystem
     (users can attach with another tmux client).
   * Pro: tmux already solves persistence, resize, scrollback,
     security boundaries.
   * Con: chan now has a hard runtime dep ("single binary, no
     runtime deps" line in CLAUDE.md moves). Need to handle
     "tmux missing" gracefully and document install.
   * Con: cross-platform: tmux on Windows is non-trivial; even on
     macOS, version skew across Homebrew/system tmux is real.
2. **Embed a Rust tmux-CC client + spawn tmux.** Same external-tmux
   dep, but pull the protocol parsing into a Rust crate (existing
   `tmux-cc`-style crates or our own) so we control the parser and
   can keep tmux a runtime dep only.
   * Pro: cleaner Rust boundaries, easier to unit-test.
   * Con: same runtime-dep tradeoff.
3. **Embed a tmux-compatible server in Rust.** Reuse or build a
   pure-Rust implementation of the tmux session model that speaks
   `-CC` over its own socket; chan-server runs it inline.
   * Pro: keeps the "single binary, no runtime deps" promise.
   * Pro: full control over persistence semantics (e.g. survive
     `chan serve` restart, not just window reload).
   * Con: implementation cost (tmux's session/window/pane model
     plus the control-mode protocol is non-trivial).
   * Con: compatibility risk: a third-party client may rely on
     undocumented tmux behaviour.

## Option 4 — chan-native PTY session registry (Alex's proposal; recommended)

Alex's framing supersedes the tmux options above. Constraint:

> "what would be a good alternative that we could do entirely in chan's
> codebase, without compatibility with anything else, so long we can
> support keeping up the sessions alive on the web service and the UI
> is freely to reload"

Decouple PTY lifetime from the WebSocket. chan-server owns long-lived
PTY sessions in-process; the WebSocket becomes a pure attach/detach
transport.

### Architecture

* **Session registry in chan-server.** New module (suggested:
  `crates/chan-server/src/terminal_sessions.rs`, plus a thin handler
  in `routes/terminal.rs`). Each entry owns:
  * the `portable_pty` child + its `MasterPty`,
  * a tokio task pumping stdout into a per-session broadcast channel,
  * an in-memory ring buffer of recent output bytes,
  * a monotonic `seq` per emitted chunk so reconnecting clients can
    fast-forward,
  * the last requested PTY winsize.
  Sessions are keyed by an opaque random `session_id` (cryptographically
  random — knowing the id alone is not a capability, the bearer-token
  middleware still gates the WebSocket upgrade).
* **WebSocket attaches, never owns.**
  `GET /api/terminal/ws?session=<id>&since=<seq>` attaches to an
  existing session (with optional fast-forward) or creates a new
  session if `session` is missing or unknown. On attach, replay the
  ring slice after `since`, then live-stream. Multiple clients can
  attach to the same session; their inputs interleave and outputs
  mirror. The broadcast-input feature drops out for free.
* **Tab identity is a session id.** The frontend already persists tab
  metadata in the per-window session blob (work [backend-2](./backend-2.md)
  just landed). Add a `terminal_session_id` field to each terminal
  tab descriptor. On reload, the frontend reads its tab list, opens
  a fresh WebSocket per terminal tab with the stored id, and the
  ring replay re-paints the user's view.
* **Lifecycle on the server, not the browser.** A session lives until:
  * the client sends an explicit close intent (closing the tab),
  * the drive closes or chan-server shuts down,
  * a configurable idle timeout fires (default suggestion: 30 minutes
    with no attached clients AND no PTY output). Reloads inside the
    window just reattach.

### What's out of scope on purpose

* Surviving a `chan serve` restart or machine reboot. Sessions are
  in-process. That gap is what Option 3 (Rust tmux-compatible server)
  would have closed; per Alex's framing, we don't need it.
* External tmux-client compatibility. Nothing outside chan's web /
  desktop frontends attaches to these sessions.

### Why Option 4 beats Options 1-3 for the stated constraint

* **vs Option 1 (external tmux dep)**: no runtime dep, no cross-
  platform tmux story, holds the "single binary, no runtime deps"
  line in `CLAUDE.md`.
* **vs Option 2 (Rust tmux-CC parser around external tmux)**: same
  runtime-dep saving plus no protocol/parser to maintain.
* **vs Option 3 (Rust tmux-compatible server)**: a fraction of the
  implementation cost. We persist PTYs; we do not implement tmux's
  session/window/pane model or the control-mode protocol.

### Risks / tradeoffs

* **Ring buffer bound.** Scrollback while detached is bounded by the
  in-memory ring (suggest ~1 MB per session, configurable). Long
  disconnects beyond the ring show a "missed N bytes" marker; the
  live stream continues normally.
* **Winsize conflict.** Two clients attached → recommend
  last-resize-wins, with a server frame telling other clients the
  new size so xterm.js `fit()`s to match.
* **Process group / signals.** No change. Ctrl+C still hits the
  PTY's foreground process group. If a client detaches mid-run, a
  long-running command keeps running — the whole point.
* **Resource cost.** One PTY child + one ring per tab. Bounded by
  tab count. Worth a soft cap (e.g. 32 sessions per drive) with a
  clear error on creation past the cap.
* **Auth.** Same bearer-token gate. Unguessable session ids prevent
  enumeration. Need to make sure a stale `session=<id>` from one
  launch can't accidentally attach to a session id in a later launch
  (the registry is per-process, so this falls out, but document it).

### Implementation outline (when greenlit)

1. New `terminal_sessions::Registry` on `AppState`, with create /
   attach / detach / close methods. Background task per session
   pumps PTY stdout into a broadcast channel + the ring.
2. Rewrite `routes/terminal::api_terminal_ws` to call attach/create
   instead of constructing the PTY inline. The handler becomes
   small: parse query, attach or create, then bridge frames in both
   directions until either side closes.
3. Frontend: persist `terminal_session_id` in the tab descriptor
   (lives in `web/src/state/tabs.svelte.ts` already). On WebSocket
   open, pass the id + the last `seq` we got. On server-side close,
   surface "session ended" cleanly and clear the saved id.
4. Config: idle timeout knob + session cap; thread through the same
   way the `search.aggression` knob from [systacean-3](./systacean-3.md)
   does, so the config story stays consistent.
5. Tests: unit tests on the registry (create / attach / detach / ring
   overflow / idle expiry / cap). End-to-end smoke owned by
   @@Webtest with a tab-reload scenario.

### Defaults locked in (Architect's call, Alex can redirect)

The Option 4 follow-up questions were not separately answered. Going
with the memo's recommended defaults so implementation can start:

1. Direction: **Option 4 confirmed by Alex.**
2. Idle timeout default: **30 minutes** (config-overridable).
3. Soft cap on sessions per drive: **32** (config-overridable).
4. Drive close: **kill sessions immediately** — matches the
   "drive close = scope end" mental model used elsewhere; if Alex
   wants the detach+timeout variant later, it's a knob change, not
   a redesign.

If any default lands wrong in practice, change it through the same
config path; nothing in the API contract depends on the specific
numbers.

## Implementation handoff

* [systacean-5](./systacean-5.md) — chan-server PTY session registry,
  lifecycle, ring buffer, idle timeout, session cap, route rewrite,
  unit tests. Defines the wire shape the frontend reads.
* [frontend-4](./frontend-4.md) — `terminal_session_id` in the tab
  descriptor; WebSocket `session=<id>&since=<seq>` query; "session
  ended" UX when the server closes a session; reattach on reload
  with ring replay.
* End-to-end reload + multi-attach validation owed to @@Webtest A
  in a follow-up smoke pass on a build that has both lanes landed.

## Earlier proposal — superseded by Option 4

Before Alex proposed Option 4, the initial recommendation was to
start with **Option 1 (external tmux dep)** behind a `--no-tmux`
feature flag, mainly to buy persistence quickly while reusing tmux's
session model. Option 4 wins on the no-runtime-dep line, on
implementation cost, and on the actual goal — neither external
tmux-client compatibility nor `chan serve` restart survival was
required. The Options 1-3 sketches are kept above for context.

The original open questions for Alex (1: tmux as runtime dep, 2:
persistence scope, 3: compatibility goal, 4: feature-flag during
design) are subsumed by the Option 4 direction.

## Progress

* 2026-05-17 @@Architect drafted the original three-option memo.
* 2026-05-17 Alex proposed Option 4 (chan-native PTY session
  registry, no external compatibility). @@Architect captured it
  in this memo with full sketch + tradeoffs.
* 2026-05-17 Alex confirmed Option 4. Implementation handed off to
  [systacean-5](./systacean-5.md) (chan-server registry +
  lifecycle + tests) and [frontend-4](./frontend-4.md) (tab
  descriptor + reattach plumbing). End-to-end smoke owed to
  @@Webtest A on a build that has both lanes landed.

## Completion notes

* This memo is a decision record. The dispatch table in
  [journal](./journal.md) now tracks
  [systacean-5](./systacean-5.md) and [frontend-4](./frontend-4.md);
  this file stays here as the why behind those tasks.
