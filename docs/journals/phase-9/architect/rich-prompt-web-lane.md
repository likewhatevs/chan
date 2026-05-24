# Rich Prompt Web Lane

Date: 2026-05-24
Owner: @@WebArchitect
Status: Web/Core integration slice complete, pending visual/iab validation

## Scope

This follows `architect/next-architect-handover.md` for the Web lane. It does
not use `docs/agents/bootstrap.md`.

Audited:

- `web/src/components/TerminalRichPrompt.svelte`
- `web/src/state/tabs.svelte.ts`
- `web/src/components/SpawnDialog.svelte`
- `web/src/components/TeamDialog.svelte`
- `web/src/state/teamDialog.svelte.ts`
- `web/src/state/teamOrchestrator.svelte.ts`
- `web/src/components/TerminalTab.svelte`
- `web/src/api/client.ts`

Page-break integration was external to this Rich Prompt lane during the Web
slice. On 2026-05-24 @@CoreArchitect accepted release ownership after
confirming the feature is already merged in `b0869b1`.

## Baseline Before Implementation

This section is the baseline captured before the implementation slice. See
Implementation Log for the current state.

Rich Prompt was terminal-overlay state, not draft-workspace state.

- Cmd+P routes through `showOrSpawnRichPromptInFocusedPane()`, which still
  toggles an existing terminal prompt off if active and only spawns a new
  terminal when the active tab is not a terminal.
- Hybrid Nav `P` already stages a fresh rich-prompt terminal, so the two
  entrypoints disagree.
- `TerminalRichPromptState` stores buffer, size, mode, collapse, page width,
  and submit mode only. It does not know a draft workspace, `draft.md`,
  `spool/`, watcher health, lifecycle phase, or close failure.
- Submit sends the buffer to the terminal and best-effort persists history as
  `Drafts/rich-prompt-N/prompt.md`. It does not archive the submitted buffer
  into the owning workspace as `prompt-N.md`, and it does not clear `draft.md`.
- Prompt close only hides the overlay and refocuses xterm. It does not close
  the shell, stop the watcher, discard a workspace, or report teardown status.
- The prompt toolbar still exposes New File from here, Spawn agent, New Team,
  Send, submit-mode toggle, collapse, and Close. The target drops New File and
  prompt-local Close, folds actions into a plus menu, and relies on terminal
  close for teardown.
- The prompt context menu still exposes Watch directory and Stop watching,
  which conflicts with the target where the watcher is internal to the Rich
  Prompt workspace and attached to `spool/events/`.
- `TeamDialog` is still named New Team in UI and state, with min 2 and max 16
  agents. Target name is Spawn agents, min 1, max 9.
- `runTeamBootstrap()` persists team config, writes templates, loads the team
  watcher, spawns worker terminals, then closes and respawns the lead terminal.
  It does not run the new preflight confirmation flow before broadcasting the
  identity/process prompt.

Important references:

- `TerminalRichPrompt.svelte`: New Team wiring around line 338, prompt header
  around line 462, context menu around line 604.
- `tabs.svelte.ts`: current Cmd+P 3-state behavior around line 1020.
- `TerminalTab.svelte`: current submit path around line 1035.
- `teamDialog.svelte.ts`: min/max and defaults around line 154.
- `teamOrchestrator.svelte.ts`: bootstrap chain around line 297.

## Target Ownership

Web owns:

- Visible Rich Prompt phase.
- Cmd+P and Cmd+. P entrypoint semantics.
- Prompt layout and controls.
- Spawn agents dialog state and copy/paste config UX.
- Live user feedback for create, preflight, broken, closing, and discard.
- Browser/iab verification dispatch.

Core owns:

- Atomic creation of the Rich Prompt draft workspace.
- Draft workspace path normalization and collision policy.
- `spool/` directory creation and safe names for agent identities.
- Watcher attach/detach error semantics.
- Draft discard to metadata trash.
- Boot warnings for broken active Rich Prompt workspaces.

The Web lane should not infer filesystem safety from local paths. It should
render server-returned phase and failure reasons.

## State Machine

The UI should model Rich Prompt as terminal-owned session state. Suggested
frontend type:

```ts
type RichPromptPhase =
  | "creating"
  | "active"
  | "submitted"
  | "preflight"
  | "broken"
  | "closing"
  | "discarded";

type RichPromptWorkspace = {
  phase: RichPromptPhase;
  draftPath: string;
  workspacePath: string;
  eventsPath: string;
  processPath: string;
  watcher: "attaching" | "attached" | "detached" | "failed";
  error?: string;
  submissionSeq: number;
};
```

Transitions:

- `creating`: Cmd+P spawns a new terminal tab, requests a Rich Prompt
  workspace, and focuses the editor. Success enters `active`; failure keeps the
  terminal visible with persistent status.
- `active`: user edits `draft.md`; watcher is attached to `spool/events/`.
  Missing, unreadable, unsafe, or detached watcher state enters `broken`.
- `submitted`: submit flushes `draft.md`, sends the full buffer to the
  terminal with a final newline if needed, archives the prior prompt as
  `prompt-N.md`, then recreates blank `draft.md` and returns to `active`.
- `preflight`: Spawn agents clicked. Dialog validates config, spawns
  terminals, asks the user to confirm agents are up, then sends the broadcast
  identity/process prompt. Cancellation returns to `active`; fatal spawn
  failure enters `closing`.
- `broken`: persistent warning tied to the terminal. Editing can continue if
  possible, but submit and spawn should either be disabled or require an
  explicit retry path, depending on Core error shape.
- `closing`: terminal close requested. Notify user, close shell when possible,
  stop watcher, discard workspace to metadata trash, then remove UI. Failure
  remains visible and the terminal should stay open if manual recovery is
  still possible.
- `discarded`: teardown complete. Rich Prompt state is removed from the tab and
  should not restore on reload.

Reload restore rule:

- If a terminal session and Rich Prompt workspace both restore, reattach
  watcher and enter `active` or `broken`.
- If the terminal session is missing but the workspace exists, show `broken`
  with a terminal-restart action or discard action.
- If the workspace is missing or unsafe, show `broken` and block silent close.

## UI Shape

Target component split:

- `TerminalRichPrompt.svelte`: composer shell only. It renders the draft editor,
  event counter, agent picker, mic placeholder, submit/stop, and compact row.
- `RichPromptMenu.svelte`: plus menu with Spawn agents, Copy metadata path,
  Copy Spawn agent configuration, and Collapse/Expand.
- `SpawnAgentsDialog.svelte`: rename of TeamDialog with min 1, max 9,
  copy/paste config, validation, preflight status, and confirmation.
- `richPromptSession.svelte.ts`: state helpers for transitions and visible
  status. Keep server calls behind `api`.

Controls:

- Plus button opens the menu.
- Event counter shows inbound/outbound counts from the attached watcher.
- Agent picker is `none | claude | codex | gemini` at minimum. It should drive
  submit behavior, replacing the current separate shell/agent icon toggle.
- Collapse keeps only the bottom row from plus through submit/stop, hiding mic
  and submit/stop per spec.
- No prompt-local Close button. Terminal close owns teardown.
- Style toolbar toggle stays explicit. Remove any remaining auto-hide behavior
  if found during implementation.

## API Assumptions For Core

Preferred server contract:

- `POST /api/rich-prompts`
  - input: terminal session id, optional initial name.
  - output: draft path, workspace path, events path, process path.
  - effect: create `draft.md`, `spool/process.md`, `spool/events/`,
    `spool/journals/`, `spool/tasks/`.
- `POST /api/rich-prompts/:id/submit`
  - input: expected draft buffer metadata and submit sequence.
  - effect: archive `draft.md` as `prompt-N.md`, write blank `draft.md`.
- `POST /api/rich-prompts/:id/close`
  - effect: stop watcher and discard workspace to metadata trash.
- `GET /api/rich-prompts/:id/status`
  - output: phase, watcher status, broken reason if any.

If Core prefers to extend existing `/api/drafts/*` and `/api/terminal/*`
routes, Web still needs one atomic "create workspace plus spool" operation.
Creating the workspace through multiple `api.create()` calls would expose
half-built Rich Prompt sessions on reload.

## Implementation Order

1. Align entrypoints: make Cmd+P and Cmd+. P always create a fresh terminal
   with Rich Prompt armed. Remove the old toggle semantics from top-level
   command routing. Keep Hybrid Nav `P` behavior aligned.
2. Add draft-workspace fields to `TerminalRichPromptState` or a nested
   `RichPromptWorkspace` and persist only the identifiers needed for reload.
3. Replace prompt close with terminal-owned teardown. Wire terminal tab close
   through the `closing` transition when a Rich Prompt workspace is active.
4. Replace toolbar/header actions with the target bottom-row and plus menu.
5. Rename TeamDialog flow to Spawn agents. Change min/max to 1/9 and add
   copy/paste config.
6. Add preflight confirmation before identity/process prompt broadcast.
7. Add broken-state surfaces for watcher/workspace failures once Core returns
   concrete status.
8. Page-break integration only after owner confirms source readiness.

## Test Plan

Unit and component tests:

- `tabs.svelte.ts`: Cmd+P always spawns a new terminal with Rich Prompt open,
  even when the active tab is already a terminal with a prompt.
- Terminal close with active Rich Prompt enters closing and calls teardown.
- Submit appends a final newline in shell mode when missing.
- Agent submit path sends paste/buffer plus agent submit behavior, with no
  duplicate newline.
- Submit archives old prompt and clears the current draft state.
- Spawn agents min 1, max 9, exactly one lead, copy/paste config round trip.
- Preflight failure keeps dialog open and reports the failing agent.
- Broken workspace state disables or guards submit/spawn as designed.

Live Browser/iab baseline before fixes:

- Cmd+P from editor creates a new terminal and focuses prompt editor.
- Cmd+P from terminal creates another terminal, not a toggle.
- Reload with active prompt restores terminal, draft, and watcher state.
- Closing the terminal tears down shell, watcher, and draft workspace.
- Delete active draft workspace with `rm -rf`; UI enters broken state.
- `chmod` draft, `spool`, `events`, and trash to remove permissions; UI reports
  persistent failure.
- Terminal opened from draft cwd keeps MCP `Drafts/...` list/read/resolve
  working.
- Spawn agents with 1, 2, and 9 agents covers config copy/paste and preflight.
- Existing bare `---` source visibility remains intact in WYSIWYG.

Evidence format:

- Git SHA and launch command.
- URL and viewport.
- Exact steps.
- Console errors.
- Screenshot for visual checks.
- Server stderr excerpt for watcher/teardown failures.

## Open Decisions

- Whether a Rich Prompt workspace should be named by terminal id, monotonic
  `rich-prompt-N`, or a user-facing label.
- Whether `spool/process.md` should be generated from a bundled template or
  returned by Core.
- Whether event counts are derived from watcher memory, server status, or
  `GET /api/terminal/:session/watcher/events`.
- Whether agent picker should auto-detect the terminal process or remain a
  user-controlled mode.
- Exact copy/paste config format for Spawn agents. JSON is simpler for Web;
  TOML may match chan-drive team config.

## Implementation Log

2026-05-24: First Web/Core integration slice landed:

- Cmd+P and Hybrid Nav `P` both create a fresh terminal with Rich Prompt open.
- `TerminalRichPromptState` now carries Core workspace identity, paths, phase,
  submission sequence, busy state, and error text.
- Web API client covers Core routes:
  - `POST /api/rich-prompts`
  - `GET /api/rich-prompts/:name/status?session=<id>`
  - `POST /api/rich-prompts/:name/submit`
  - `POST /api/rich-prompts/:name/close`
- `TerminalTab.svelte` creates the workspace once the terminal session exists,
  reconciles restored workspaces through the status route, attaches the watcher
  path returned by Core, and surfaces broken watcher/workspace errors.
- Submit still sends the buffer to the terminal, appending a final newline in
  shell mode when missing. It archives the exact editor buffer through Core and
  clears the prompt buffer only if the user has not edited it while the submit
  request was in flight.
- Terminal tab close now awaits Core Rich Prompt close. If Core returns
  `phase: "broken"` or the request fails, Web keeps the terminal tab visible.
- Prompt UI shows workspace status and can copy the metadata directory path.
- Spawn agents UI was renamed, min/max changed to 1/9, and JSON copy/paste for
  dialog config was added.

Verification:

- `npm run test -- --run src/components/richPromptHistoryPersist.test.ts
  src/state/tabs.test.ts src/components/TerminalRichPrompt.test.ts`
- `npm run check`
- `npm run test -- --run` hit one `Pane.test.ts` timeout under full-suite
  jsdom/xterm load after 1399 passes and 11 skips. The isolated rerun passed:
  `npm run test -- --run src/components/Pane.test.ts`.
- `npm run build`
- `cargo build -p chan`
- `git diff --check`
- Live route walk against embedded `./target/debug/chan serve --no-browser`
  using an isolated `HOME=/tmp/chan-rich-prompt-home`:
  - created terminal session
  - created Rich Prompt workspace
  - status returned `phase: "active"` and watcher `state: "attached"`
  - submit archived `Drafts/rich-prompt/prompt-1.md` with exact content
  - injected `event-live.json` under `spool/events/`
  - watcher events endpoint returned the event
  - close returned `phase: "discarded"`
  - watcher endpoint returned 409 after close

Known gaps:

- The in-app Browser plugin tool was not available in this session, so the
  live walk used HTTP route checks against the embedded server rather than a
  visual browser/iab click-through.

2026-05-24 follow-up: Web code parts completed:

- Prompt header now matches the Phase 9 target: plus actions, event counter,
  agent picker, Send, and collapse. Prompt-local Close is removed.
- Prompt-local New File and manual Watch/Stop watcher actions are removed.
- Plus menu now carries Spawn agent, Spawn agents, Copy metadata dir, Copy
  Spawn agents config, Collapse/Expand, and bubble stack/tray.
- Agent picker persists `none | claude | codex | gemini` through the tab
  serializer and drives server submit mode. Restored agent targets resync the
  terminal submit mode.
- Spawn agents bootstrap now spawns worker terminals first, asks for preflight
  confirmation, and only then stages identity prompts. Cancel keeps the dialog
  open through the existing rejected-promise path.
- `TerminalRichPrompt.svelte` no longer accepts unused prompt-close or manual
  watcher props. Terminal tab close remains the only Rich Prompt teardown path.

Additional verification:

- `npm run test -- --run src/components/TerminalRichPrompt.test.ts
  src/components/newTeamButton.test.ts
  src/components/teamBootstrapOrchestrator.test.ts src/state/tabs.test.ts
  src/components/toastAutoDismissSweep.test.ts
  src/components/PathPromptModal.test.ts` passed: 6 files, 181 tests.
- `npm run check` passed with 0 errors and 0 warnings.
- `npm run build` passed with existing Vite bundle-size and ineffective dynamic
  import warnings.
- `cargo build -p chan` passed. Core-side dead-code warnings remain in
  `crates/chan-drive/src/fd_budget.rs`.
- `git diff --check` passed.

Visual/iab validation:

- Run completed from worktree `b0869b1` with uncommitted Rich Prompt changes
  present. No code edits were made by the validation agent.
- Launch command:
  `HOME=/tmp/chan-iab-home ./target/debug/chan serve --no-browser
  /tmp/chan-iab-drive`
- App load passed with no current-run console or page errors.
- Literal Cmd+P from editor and terminal did not trigger in iab. This is
  classified as validation-contract mismatch for browser/iab, not a Web lane
  regression: current shortcut docs and registry use Cmd+Alt+P for macOS web
  plus native, with Mod+. p as the universal path. The staged Mod+. p path
  passed from both editor and terminal and created fresh Rich Prompt terminals.
- Prompt header passed: plus actions, event count, agent picker, Send,
  collapse, and no prompt-local close/X.
- Plus menu passed: Spawn agent, Spawn agents, Copy metadata dir after
  workspace creation, Copy Spawn agents config, Collapse prompt, Bubble stack,
  and Bubble tray are present.
- Removed menu items passed: New File from here, Watch directory, and Stop
  watching are absent.
- Agent picker persistence passed: codex selection survived reload.
- Submit behavior was partial because iab could not type or clipboard text into
  CodeMirror. Blank unchanged submit kept focus and left the prompt blank.
- Spawn agents dialog was partial because iab could not drive slider or paste
  config. Dialog opens, supports the 1-agent state, and exposes config
  copy/paste controls.
- Terminal close passed: confirmation appears for a running terminal, cancel
  keeps it visible, confirmed close removes the terminal and its Rich Prompt
  workspace.
- Server stderr showed expected BM25-only model fallback plus repeated Core
  watcher warnings: `watcher event stream lost scope; requesting rebuild` for
  path-less event / rename during Rich Prompt workspace activity.

Remaining follow-up:

- Optional native/desktop validation for literal Cmd+P. Browser/iab validation
  should use Cmd+Alt+P or Mod+. p.
- Non-empty CodeMirror submit and full Spawn agents preflight visual checks in
  a browser environment that can type into CodeMirror and use clipboard APIs.
- Core classified path-less watcher rebuild warnings as notify noise and
  closed the follow-up in `affe1e7`.
