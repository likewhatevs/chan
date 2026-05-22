# Agent spawn protocol

chan-server exposes an HTTP control channel for creating
named terminal tabs, executing agent CLIs inside them,
and managing their lifecycle. The intended caller is an
orchestrating agent (e.g. an `{lead-handle}` running
inside a chan terminal) that needs to spawn helper
agents on demand.

> **Status note**: the contract below is the design
> shape from `systacean-12`. If you're reading this
> before that task lands, treat it as the target. After
> it lands the description here is authoritative.

## Endpoints

All routes are under the same chan-server origin and
require the per-launch bearer token in the
`Authorization: Bearer <token>` header (or the `t=<token>`
query param that the SPA uses).

### `POST /api/terminals`

Create a new terminal tab in the active pane.

Body:

```json
{
  "name": "@@AgentName",
  "command": "claude --model=claude-opus-4-7 --dangerously-skip-permissions",
  "env": {
    "EXTRA": "value"
  }
}
```

* `name` — the tab's display name. The chan-server
  event watcher uses this to route `poke\n` dispatches
  by matching the `to` field of incoming events. Use
  the `@@Name` convention.
* `command` — the full CLI to run, including flags. No
  argv-array form; chan splits on the shell quoting
  you provide.
* `env` — optional extras merged into the PTY
  environment on top of chan's default `CHAN_*` env
  plumbing.

Response: `201 Created`

```json
{
  "session": "<terminal-session-id>",
  "tab_label": "@@AgentName"
}
```

### `POST /api/terminals/<session>/restart`

Restart the PTY in place (same name, same command).
Returns `204 No Content`.

### `DELETE /api/terminals/<session>`

Close the terminal tab. Returns `204 No Content`.

## Pre-flight signals

When the spawned command emits stdout that matches one
of chan-server's pre-flight patterns (login required,
authentication needed, setup wizard, etc.), chan-server
fires a `pre-flight` event into the orchestrating tab's
watcher directory:

```json
{
  "id": "<unique>",
  "type": "pre-flight",
  "from": "@@AgentName",
  "to": "{lead-handle}",
  "topic": "spawn-pre-flight",
  "questions": [
    {
      "header": "Setup?",
      "text": "Gemini needs login. What now?",
      "options": [
        {"key": "1", "label": "Open the terminal"},
        {"key": "2", "label": "Kill the spawn"},
        {"key": "3", "label": "Retry now"}
      ]
    }
  ],
  "scope": "one-shot"
}
```

The rich-prompt bubble overlay renders this as a normal
single-topic survey (per `fullstack-18` / `fullstack-20`).
Reply via the SPA writes the survey-reply through
chan-server's reply endpoint; chan-server reacts:

* `1` → focus the spawn tab so the user can complete
  setup interactively.
* `2` → `DELETE /api/terminals/<session>` to close.
* `3` → `POST /api/terminals/<session>/restart` to retry.

## Sample workflow

1. An orchestrating agent decides to spawn a helper
   agent.
2. It POSTs `/api/terminals` with the helper's name +
   CLI.
3. chan-server creates the tab, launches the PTY, gets
   the tab name into the watcher dispatch registry.
4. If the helper boots cleanly, that's it — the helper
   is now reachable by routing events at `@@HelperName`.
5. If the helper emits a pre-flight signal first,
   chan-server fires the `pre-flight` event into the
   orchestrator's watcher dir; user picks; chan-server
   acts.
6. Helper does its work, sends events back via atomic
   writes (see [atomic-writes.md](./atomic-writes.md)).

## No state outside disk

Spawned tabs participate in the per-window
`session.json` layout persistence. A chan-server
restart reattaches via the existing terminal-session
machinery (or surfaces detached sessions, depending on
the restart path).
