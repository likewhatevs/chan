# systacean-12: HTTP agent control channel (spawn / name / execute / restart)

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-19

## Goal

Land the chan-server HTTP control channel that lets an
agent (in our process: @@Architect specifically) create a
new terminal tab in the current pane, name it, execute an
agent CLI inside it, and restart it. This is the
substrate for Round 2's programmatic agent spawning;
`fullstack-20` builds the rich-prompt UI on top.

@@Alex picked HTTP (not MCP) for this back-channel —
setup-2 Q5 amendment in the architect journal — because
HTTP keeps the surface simple and lets us reuse the
existing bearer-token auth.

## Relevant links

* Round 2 capacity proposal:
  [../architect/journal.md](../architect/journal.md)
  "2026-05-18 21:00 BST" entry.
* @@Alex's intent:
  [../request.md](../request.md) — "Session setup"
  section.

## Acceptance criteria

### Endpoints

* `POST /api/terminals` — create a new terminal tab in
  the active pane.
  * Body: `{ "name": "@@AgentName", "command": "<cli with args>", "env": { ... } }`
  * `name` becomes the tab's display name (visible in the
    tab strip + reachable via the watcher dispatch).
  * `command` is the CLI to spawn (e.g. `claude
    --model=opus-4-7 --dangerously-skip-permissions ...`,
    or `codex login` for first-time auth, etc.).
  * `env` is optional extra env vars merged into the
    PTY environment (chan's existing `CHAN_*` env
    plumbing still applies on top).
  * Returns `201 Created` with `{ "session": "<id>",
    "tab_label": "<name>" }`.
* `POST /api/terminals/<session>/restart` — restart the
  PTY in place (same name, same command).
  * Reuse the existing restart machinery from the
    terminal menu.
  * Returns `204 No Content`.
* `DELETE /api/terminals/<session>` — close the terminal
  tab.
  * Returns `204 No Content`.

### Auth

* Reuse the per-launch bearer token (same as all other
  chan-server APIs).
* `--no-token` mode (tunnel): the gateway-in-front-of-
  drive.chan.app is the trust boundary; same posture as
  the existing API surface. No new token shape required.

### Pre-flight signals

* When the spawned command's first stdout line matches
  any of:
  * `please log in` / `authentication required` / `not
    authenticated`
  * `gemini setup required` / `claude setup`
  * etc. (small configurable list to start)
  
  chan-server emits an event into the watcher dir (if
  one is active for the orchestrating tab) of type
  `pre-flight` with the matching text. The UI in
  `fullstack-20` renders a survey from this.
* Otherwise the spawn is silent — the agent's stdout
  flows to the PTY as normal.

### No state outside disk

* Spawned tabs are first-class terminal sessions; they
  participate in the existing session.json layout
  persistence. A chan-server restart re-launches them
  per the existing reattach machinery (or surfaces them
  as detached, depending on what's already there).

### Tests

* Unit tests on each endpoint shape (success + error
  cases).
* Integration test: spawn a `bash -c 'echo hi; exit 0'`
  command; confirm the tab appears with the right name
  and the PTY captures `hi`.

## Out of scope

* Pre-flight survey UI (that's `fullstack-20`).
* The orchestration SKILL (that's `architect-1`).
* Detailed agent-specific pre-flight scripts. Keep the
  signal-matching small and configurable; agent-specific
  helpers can layer on later.

## How to start

1. New routes under
   `crates/chan-server/src/routes/terminal.rs` (or a
   sibling `routes/terminals.rs` if cleaner — plural to
   avoid colliding with the existing per-session path).
2. Spawn machinery: reuse the existing PTY launch path
   used by the `chan open` work and the terminal menu's
   "New Terminal" affordance.
3. Tab name resolution: feed `name` into the existing
   tab registry so the watcher dispatch path can find
   the new tab by `@@Name` lookup immediately.
4. Pre-flight signal matcher: small inline matcher on
   the first N lines of stdout per spawned PTY.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@FullStack on the endpoint shape before they build the
spawn UI. Ping via
`alex/event-systacean-architect.md`.
