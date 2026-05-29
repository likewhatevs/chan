# @@Frontend task 10: terminal-tab "Set MCP env" toggle + info + inject command

Owner: @@Frontend
Status: REVIEW (frontend side; backend-3 still gates live opt-out)
Depends on: [backend-3](./backend-3.md) for the `mcp_env=on|off`
query-param contract on `/api/terminal/ws`.
Source: Alex's 2026-05-17 callout. "We should definitely have a
setting for this in the terminal tab menu, like toggle set MCP
env and an info bubble explaining + button to inject the command
to show them in the terminal."

## What to add

In the terminal tab title menu (today's home for "Rename tab" /
"Broadcast input"):

1. **Toggle: "Set MCP env vars"** (default ON).
   * Persisted per-tab in the per-window session blob alongside
     `terminalSessionId` / `lastSeq` / broadcast-input targets.
   * Drives the `mcp_env=on|off` query param when a new PTY
     session is created. Reattaches to an existing session do
     **not** re-send this — env is fixed at exec time.
   * Flipping the toggle on an existing tab is a no-op until the
     user starts a new session (either via the existing
     "Restart" menu action or by closing + reopening the tab).
     Surface this in the info bubble.
2. **Info bubble** next to the toggle (small `?` or i-icon → popover):
   * One sentence: "When on, chan sets `CHAN_MCP_SOCKET`,
     `CHAN_MCP_SERVER_JSON`, and friends in the PTY env so
     external agent CLIs can discover the chan MCP server
     automatically. Turn this off to launch a vanilla shell."
   * Note: applies to new sessions only.
3. **Button: "Show MCP env in terminal"** below the toggle:
   * On click, write
     `env | sort | grep '^CHAN_MCP_'` (followed by `\n`) to the
     terminal input stream so the user can see what's set in
     their current session. Match the broadcast-input writer
     path so the keypress goes through the same WS frame the user
     would have sent.
   * Disabled if the toggle is off **and** the session was created
     with `mcp_env=off` (nothing to show).

## Where it lives in code

* `web/src/components/TerminalTab.svelte`: the title-menu Svelte
  block already has "Rename" / "Broadcast input" entries. Add the
  toggle, info bubble, and button next to those. Reuse the same
  menu primitives (don't roll a parallel popover component).
* `web/src/state/tabs.svelte.ts`: extend the terminal tab
  descriptor with `mcpEnv?: boolean` (default `true` if absent).
  Persist alongside the existing terminal fields.
* `web/src/api/client.ts` (or wherever the WS URL is assembled —
  the `terminalWsPath()` helper from frontend-4 is the canonical
  spot): append `mcp_env=off` only when the descriptor's `mcpEnv`
  is explicitly `false`. Default-on means no extra query param.

## Acceptance criteria

* Toggle appears in the terminal tab title menu, default ON,
  persists across reload (per-window session blob).
* Toggling OFF on a tab and then triggering a fresh session
  (close + reopen, or "Restart" if that exists) yields a PTY
  whose `env | grep CHAN_MCP` is empty.
* Toggling ON later and starting a fresh session restores the
  CHAN_MCP_* env.
* Info bubble copy reads cleanly and explains the new-session-only
  semantics.
* "Show MCP env in terminal" button writes
  `env | sort | grep '^CHAN_MCP_'` into the running terminal
  session as if the user had typed + Enter. No effect on
  broadcast-input target set.
* Two browser tabs / chan-desktop windows behave correctly:
  the per-window session blob isolation from frontend-7 means
  each tab's toggle is independent.
* `npm --prefix web run check` + `npm --prefix web test -- --run`
  + `npm --prefix web run build` all green.

## Test expectations

* Unit test in `web/src/components/TerminalTab.test.ts` (or
  wherever the existing terminal-tab tests live) for:
  * Default `mcpEnv === true` persists across save/load of the
    descriptor.
  * `terminalWsPath()` omits `mcp_env` when default, appends
    `mcp_env=off` when `mcpEnv === false`.
  * The inject-command path writes the canonical command bytes
    to the PTY input helper (mock the helper, assert call args).

## Hardening

* The toggle is per tab, not per drive. A user with multiple
  terminal tabs in the same window can have a mix of MCP-on and
  MCP-off shells. Confirm the session-blob serialization handles
  that correctly (the descriptor is per-tab already).
* If the existing TerminalTab "Restart" action is present, make
  sure it picks up the new toggle when starting the replacement
  session (not the previous session's value).

## Coordination

* [backend-3](./backend-3.md) for the `mcp_env=on|off` query
  param. Reconcile the param name before either lane ships code.
* @@Webtest A re-smokes the terminal MCP env check on a build
  with both backend-3 and frontend-10 — flips the eight-var
  expectation to five-var and confirms `mcp_env=off` produces a
  bare PTY env.

## Out of scope

* Surfacing this in a global Settings page. The per-tab menu is
  the right UX for an env decision that's "this terminal, this
  session".
* Migrating the existing tabs' descriptors. Absent `mcpEnv`
  defaults to `true` (current behaviour) so old session blobs
  keep working.
* Reattach-time env changes — calls out clearly in the info
  bubble.

## Progress

* 2026-05-17 @@Frontend started after coordination poke.
* Added per-terminal-tab `mcpEnv` desired state, defaulting to ON
  for old descriptors and new tabs.
* Added `sessionMcpEnv` sidecar state for the currently attached PTY
  session so the UI can distinguish "next shell preference" from
  "current shell was launched with MCP env".
* Persisted both fields only in the per-window session layout, not in
  the shareable URL hash.
* Extended `terminalWsPath()` to append `mcp_env=off` only for fresh
  sessions when the tab toggle is OFF; reattach URLs omit the param.
* Added the title-menu toggle, info popover, and "Show MCP env in
  terminal" action. The action writes the canonical grep command via
  the same terminal input path as typed user input.
* Added focused tests for URL generation, session serialization, and
  command injection.

## Completion notes

Diff locations:

* `web/src/components/TerminalTab.svelte`
* `web/src/state/tabs.svelte.ts`
* `web/src/state/tabs.test.ts`
* `web/src/terminal/session.ts`
* `web/src/terminal/session.test.ts`
* `web/src/terminal/mcpEnv.ts`
* `web/src/terminal/mcpEnv.test.ts`

Verification:

* `npm --prefix web run check`
* `npm --prefix web test -- --run` (18 files / 158 tests)
* `npm --prefix web run build` (existing Vite warnings only)

Remaining dependency:

* Live `mcp_env=off` behavior waits on [backend-3](./backend-3.md),
  which owns honoring the query param and dropping the CLI-flavoured
  env aliases from the PTY process.
