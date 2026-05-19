# fullstack-20: spawn-from-rich-prompt UI + pre-flight survey

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Build the rich-prompt UI that lets the user (or
@@Architect via watcher event) spawn a new agent into a
freshly-created terminal tab. Renders the pre-flight
survey when chan-server emits one (login required,
auth needed, setup wizard, etc.).

Backend partner: `systacean-12`'s `POST /api/terminals`
endpoint. Reuses the bubble overlay + survey rendering
from the wave-A substrate (`fullstack-18`).

## Relevant links

* Backend: [../systacean/systacean-12.md](../systacean/systacean-12.md).
* @@Alex's intent:
  [../request.md](../request.md) — "Session setup"
  section + the engineering addendum.
* Survey rendering substrate: `fullstack-18` (TUI
  density numbered options).

## Acceptance criteria

### Spawn affordance

* Rich prompt grows a "Spawn agent" affordance (menu
  item / button in the context menu, alongside Watch
  directory / etc. that landed in `fullstack-13`).
* Click → opens a small dialog with three fields:
  * **Tab name** (e.g. `@@CodexPair`).
  * **Command** (the full CLI to run, including model
    flags / dangerously-skip-permissions / etc.).
  * **Env** (optional key=value lines).
* Submit calls `POST /api/terminals` with the body.
* Success (`201`) → new terminal tab appears in the
  active pane with the chosen name, command running.
* Failure → surface error inline in the dialog.

### Pre-flight survey rendering

* When the spawned PTY emits a pre-flight signal,
  chan-server fires a `pre-flight` event (type
  reserved in the schema; SPA needs to handle this).
* The bubble overlay renders the pre-flight as a
  survey using the existing numbered-option machinery
  from `fullstack-18`:
  * Question text = the matched pre-flight message
    (e.g. "Gemini needs login. What now?").
  * Options: `1) Open the terminal` (focus the spawn
    tab so user can log in), `2) Kill the spawn`,
    `3) Retry now`.
  * Single-topic; same keyboard `1`/`2`/`3` reply.
* Show a small **spinner + elapsed time counter**
  next to the bubble while waiting for the user to
  pick. Timeout at e.g. 5 minutes; if no pick by
  timeout, show "Spawn idle — retry now?" as a
  one-button refresh.

### Restart / close

* Spawned tabs gain the normal terminal restart +
  close affordances (already exist on tab strip).
* Restart on a spawned tab calls
  `POST /api/terminals/<session>/restart`.

## Out of scope

* @@Architect-side automation (the spawn-via-event
  path from a watched event). The endpoint is
  callable from there independently; this task is
  just the rich-prompt manual UI.
* Pre-flight signal MATCHING logic — that's chan-
  server's job in `systacean-12`. SPA just renders
  the survey.
* Agent-specific configuration UIs.

## How to start

1. Wait for `systacean-12` to land OR coordinate the
   endpoint shape ahead of time.
2. New dialog component
   `web/src/components/SpawnDialog.svelte`. Reuse the
   text input primitives from the watcher-set dialog.
3. `pre-flight` event handler: add the new event
   type to the SPA event reader. Route to the bubble
   overlay with the survey shape above.
4. Spinner + counter: small visual primitive next to
   the bubble; tick every second.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@Systacean on the endpoint shape + pre-flight signal
matcher list; with @@WebtestA on the rich-prompt UI
walkthrough; with @@WebtestB on the end-to-end spawn
flow. Ping via `alex/event-fullstack-architect.md`.

## 2026-05-19 04:55 BST — implementation note

Implemented the frontend side against the documented `systacean-12` API shape.

Changed behavior:
* Rich prompt now has a Spawn agent button and context-menu row.
* Added `SpawnDialog.svelte` with Tab name, Command, and optional KEY=value env lines.
* Submit calls `POST /api/terminals` and opens a controlled terminal tab attached to the returned session id.
* Controlled spawned tabs persist a small marker so restart routes through `POST /api/terminals/<session>/restart`.
* SPA event parsing accepts `pre-flight` with optional `session` / `tab_label` metadata.
* Bubble overlay renders `pre-flight` as a single-topic numbered survey: open terminal, kill spawn, retry now, with spinner + elapsed counter and 5-minute retry-only timeout state.

Verification so far:
* `npm run test -- BubbleOverlay TerminalRichPrompt watcherEvents tabs`
* `npm run check`
* `npm run build`

Coordination note: `systacean-12` backend edits are present in the shared worktree but not committed by this lane. I will stage only frontend + this task file when landing.

## 2026-05-19 04:56 BST — backend gate

Frontend implementation is ready locally and frontend verification is green, but I am not landing it ahead of `systacean-12` because the visible Spawn agent affordance would call `/api/terminals` before the backend endpoint is on `main`.

Current frontend verification:
* `npm run test -- BubbleOverlay TerminalRichPrompt watcherEvents tabs`
* `npm run test -- BubbleOverlay watcherEvents`
* `npm run check`
* `npm run build`

Landing plan once `systacean-12` is committed/pushed: rerun `scripts/pre-push`, stage only the frontend files plus this task file, commit, push, and ping Architect.
