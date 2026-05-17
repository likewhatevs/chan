# @@Backend task 3: scope terminal MCP env to the CHAN_ namespace + honor a per-WS opt-out

Owner: @@Backend
Status: REVIEW
Severity: MEDIUM — partial reversal of [backend-1](./backend-1.md)'s
terminal MCP env work, plus a per-tab opt-out hook for
[frontend-10](./frontend-10.md).
Source: Alex's 2026-05-17 callout. "If these variables like
CLAUDE_MCP_SERVER_JSON belong to CLAUDE / CODEX / etc., let's not
set them. We should only set our own namespace, meaning prefix
with CHAN_."

## What changes

Today `crates/chan-server/src/routes/terminal.rs` exports eight env
vars per PTY session:

* `CHAN_MCP_SERVER_NAME`, `CHAN_MCP_SOCKET`, `CHAN_MCP_COMMAND`,
  `CHAN_MCP_COMMAND_JSON`, `CHAN_MCP_SERVER_JSON` — chan's own
  namespace, keep.
* `CLAUDE_MCP_SERVER_JSON`, `CODEX_MCP_SERVER_JSON`,
  `GEMINI_MCP_SERVER_JSON` — third-party CLI namespaces, **drop**.

Why drop the third-party aliases:

* The CLI may already be configured by the user (wrappers,
  per-machine MCP config, `~/.codex/config.toml`, etc.). Stamping
  an env var on top of that is a collision risk and an "owns the
  namespace" policy violation.
* Each CLI evolves its env-var contract independently. Carrying
  three CLI-flavoured copies of the same JSON is duplication that
  ages poorly.
* The CHAN_-prefixed vars are sufficient for any CLI or script
  that wants to discover the chan MCP server: read
  `CHAN_MCP_SERVER_JSON`, do whatever the CLI's own MCP config
  shape requires.

## Per-WS opt-out

[frontend-10](./frontend-10.md) wants a per-terminal-tab toggle
"Set MCP env" so a user can launch a tab with a vanilla shell
environment. Wire that through as a WebSocket query parameter:

* New optional query param on `/api/terminal/ws`:
  `mcp_env=on|off`. Default `on`.
* When `mcp_env=off`, do not set any of the `CHAN_MCP_*` env vars
  for the new PTY session (existing sessions are unaffected —
  env is fixed at exec time).
* The toggle is per session, not per attach. Reattaches to the
  same session don't re-read it; only fresh-session creation
  takes the new query param.

## Acceptance criteria

* `crates/chan-server/src/routes/terminal.rs` no longer constructs
  or sets `CLAUDE_MCP_SERVER_JSON`, `CODEX_MCP_SERVER_JSON`, or
  `GEMINI_MCP_SERVER_JSON`. Remove the helper that builds those
  aliases.
* `CHAN_MCP_SERVER_NAME`, `CHAN_MCP_SOCKET`, `CHAN_MCP_COMMAND`,
  `CHAN_MCP_COMMAND_JSON`, `CHAN_MCP_SERVER_JSON` continue to be
  set by default for new PTY sessions (when the MCP bridge is up).
* Opening `/api/terminal/ws?...&mcp_env=off` creates a new
  session whose PTY env contains **none** of the `CHAN_MCP_*`
  vars.
* `CHAN_TAB_NAME` is unrelated and stays.
* Update `crates/chan-server/Cargo.toml` / `chan-server/README.md`
  / any docs that listed the three CLI-flavoured names. Cross-
  check `README.md` (repo root) and `design.md` — they currently
  mention `CLAUDE_MCP_SERVER_JSON` etc. and need to be reworded
  to the "CHAN_ namespace only" story.
* Full pre-push gate: `cargo fmt --check`, `cargo clippy
  --all-targets -- -D warnings`, `cargo build --no-default-features`,
  `cargo test`, `npm --prefix web run check`, `npm --prefix web
  test -- --run`, `npm --prefix web run build`.

## Test expectations

* Existing terminal-route tests probably assert on the eight env
  vars (Webtest A's round-1/2 smoke quoted them). Update those to
  the five-var contract.
* Add a unit test for the `mcp_env=off` branch: spawning a
  session with the param yields a child whose env (probe via the
  same path the route uses) carries no `CHAN_MCP_*` keys.

## Coordination

* [frontend-10](./frontend-10.md) reads `mcp_env=off` off the
  per-tab toggle and appends it to the WS URL. Reconcile the
  query-param name with @@Frontend before either lane lands code.
* @@Webtest A re-runs the terminal MCP env check (the
  `env | grep CHAN_MCP` step). Expectation flips from
  "eight vars, three CLI-flavoured" to "five vars, all CHAN_-
  prefixed, none third-party".
* Documentation sweep: the existing PASS notes in
  [webtest-1](./webtest-1.md) and [webtest-2](./webtest-2.md)
  mention the eight-var contract; those task files do not need
  rewriting (they're historical), but the [summary.md](./summary.md)
  highlight should be updated to reflect the new scope.
* The "Embedded terminal sets ENV variables for claude / codex /
  gemini so they pick up chan's MCP server by default" line in
  the request checklist needs a footnote: the spirit (external
  CLIs auto-discover the chan MCP server) is preserved through
  `CHAN_MCP_SERVER_JSON`; the literal CLI-flavoured aliases are
  intentionally dropped.

## Out of scope

* Persisting the per-tab toggle in the session blob — that's
  [frontend-10](./frontend-10.md)'s job.
* Honoring `mcp_env=off` on reattach to an existing session — env
  is fixed at exec time; changing the toggle after the session
  exists requires a fresh session.
* Any change to `mcp_bridge.rs` or the MCP server itself.

## Progress

* 2026-05-17 @@Backend picked up after the round-14 update check.
* Added `mcp_env=on|off` parsing on `/api/terminal/ws`; the value
  is carried into `CreateOptions` and affects fresh PTY session
  creation only. Existing-session reattach still keeps the env
  fixed at the original exec.
* Removed the third-party MCP aliases from PTY env construction.
  New sessions now set only `CHAN_MCP_SERVER_NAME`,
  `CHAN_MCP_SOCKET`, `CHAN_MCP_COMMAND`,
  `CHAN_MCP_COMMAND_JSON`, and `CHAN_MCP_SERVER_JSON` when MCP env
  is enabled and the bridge is available.
* Added a focused terminal-route test for `mcp_env=false` and
  tightened the real-PTY env probe to assert the third-party
  aliases are absent.
* Updated live docs / close-out notes to the CHAN-only namespace
  story: `README.md`, `CLAUDE.md`, `design.md`,
  `chan-pre-release-phase-5/summary.md`,
  `chan-pre-release-phase-5/architect-2.md`, and
  `chan-pre-release-phase-5/systacean-7.md`.

## Completion notes

* Test added: `routes::terminal::tests::mcp_env_off_omits_chan_mcp_vars`.
* Verification:
  * `cargo fmt --check`
  * `cargo clippy -p chan-server --all-targets -- -D warnings`
  * `cargo clippy --all-targets -- -D warnings`
  * `cargo test -p chan-server`
  * `cargo build --no-default-features`
  * `cargo test`
  * `npm --prefix web run check`
  * `npm --prefix web test -- --run`
  * `npm --prefix web run build`
