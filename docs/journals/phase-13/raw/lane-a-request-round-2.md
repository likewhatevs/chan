# @@LaneA request - Phase 13 round 2

You are @@LaneA, the round-2 lead for the **Team Work** full-stack
revamp (the feature today called "Rich Prompt"). You MAY spawn up to 4
in-session subagents via the Agent tool. You report progress +
merge-ready slices to @@Alex; @@LaneB serializes merges to main and
cuts v0.18.0. You do NOT merge to main and do NOT push to origin.

## Recover context (read in order)

- `/Users/fiorix/dev/github.com/fiorix/chan/CLAUDE.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/design.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/crates/chan-workspace/design.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/roadmap-round-2.md`
  (your source of truth; images `image-2.png` current dialog,
  `image-3.png` current right-click menu)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/bootstrap-round-2.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/README.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-a/journal.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-alex-lane-a.md` (inbox)

## Worktree + branch

Source ONLY in `../chan-lane-a`. The dir exists from round 1 on a stale
branch; on your FIRST turn bring it to main with the round-2 branch:

```
git -C ../chan-lane-a status            # confirm clean
git -C ../chan-lane-a checkout -B phase-13-r2-lane-a main
```

Journals + channels + this request file live in the MAIN checkout at
`/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/` and
are edited by ABSOLUTE PATH (never the worktree copy).

## What "Team Work" must become

Roadmap area "Rich Prompt". Three intertwined jobs on the SAME files:
(A) delete the agent-task/event watcher backend + its frontend feed,
(B) reduce bubbles to a frontend-only stub, (C) rename + new Cmd+P flow
+ dialog redesign + bootstrap reorder. Because the deletion and the
rename rewrite the same files (`TerminalTab.svelte`, `tabs.svelte.ts`,
`client.ts`), the subagent split below keeps ONE owner per file.

### Rename decisions (apply; flag #2 below if you disagree)

- **Rename fully** (pre-release allows it): `TerminalRichPrompt.svelte`
  -> `TeamWork.svelte`; type `TerminalRichPromptState` -> `TeamWorkState`;
  state field `tab.richPrompt` -> `tab.teamWork`; all user-visible
  labels -> "Team Work". All inside your files + tests. Grep-audit every
  `richPrompt`/`RichPrompt` reference (incl. SerTab serialize/restore
  and regex-source-matching tests like `paneModeStaging.test.ts`,
  `teamLeadPrompt.test.ts`) before declaring done.
- **Keep stable**: the chord *id* `app.terminal.richPrompt`. It is
  referenced from Lane B's files too; renaming forces a cross-lane
  dance for zero user-visible gain. Keep `case "app.terminal.richPrompt"`
  in `App.svelte` and only swap the handler body; add a comment that the
  id is the legacy stable key. Lane B changes only the *label* string.

## Subagent A1 - Rust backend deletion (`crates/` only, fully isolated)

Order R1 -> R4 (one crate; clippy is the dead-code net):

1. **R1 `crates/chan-server/src/terminal_sessions.rs`**: remove watcher
   lifecycle (`set_watcher`, `clear_watcher`, `watcher_status`,
   `watcher_dir`, `watcher_preflight_config`, ~412-531),
   `dispatch_agent_event()` (~598), the `AgentEventEcho` SessionEvent
   variant + ring buffer + replay, `agent_mode`/`submit_mode` fields +
   setters, `format_poke_text`, `PreflightMonitor`, and watcher tests.
2. **R2 `crates/chan-server/src/routes/terminal.rs`**: remove
   `api_set_terminal_watcher`, `api_unset_terminal_watcher`,
   `api_terminal_watcher_events`, `api_terminal_event_reply`,
   `api_set_terminal_submit_mode` + helpers (`validate_event_reply`,
   `write_event_reply_atomic`, `resolve_watcher_dir`,
   `list_watcher_events`) + tests; drop
   `use crate::event_watcher::{...}` and the `watcher_preflight_config`
   call in `api_create_terminal`.
3. **R3 delete whole files**: `crates/chan-server/src/routes/rich_prompts.rs`,
   `crates/chan-server/src/event_watcher.rs`,
   `crates/chan-workspace/src/rich_prompts.rs` (+ its `pub mod` /
   `pub use` in `crates/chan-workspace/src/lib.rs`).
4. **R4 `crates/chan-server/src/lib.rs::router()`**: remove the 6 route
   registrations (`/api/drafts/rich-prompt`, `/api/rich-prompts*`,
   `/api/terminal/:session/watcher`, `.../watcher/events`,
   `.../event-reply`, `.../submit-mode`), the `mod event_watcher;` /
   `mod rich_prompts;` declarations, and the orphaned handler names in
   `use routes::{...}`.

**KEEP (do not break)**: `bus.rs` ScopeRegistry/WatchBridge (shared with
the content indexer), `routes/ws.rs`, chan-workspace content `watch`,
`self_writes.rs`. These are the content/editor watcher, independent of
agent events.

## Subagent A2 - Frontend foundation + spines (BLOCKING checkpoint)

Owns `tabs.svelte.ts`, `client.ts`, `App.svelte`, the file deletions.
**Land F0 and post the exported symbol names BEFORE A3/A4 start** -
both import from `tabs.svelte.ts`.

- **F0 `web/src/state/tabs.svelte.ts`**: remove `WatcherEvent`,
  `SurveyQuestion`, `SurveyOption`, `ScopeGrant`, `TerminalWatcherState`
  types and the `watcher?` field on `TerminalTab` (+ all serialize /
  restore of `watcher`). Rename type `TerminalRichPromptState` ->
  `TeamWorkState` and field `richPrompt` -> `teamWork`. Add the
  lead-terminal-creation helper the Cmd+P flow needs (returns the
  created `TerminalTab` so the dialog can delete it on Cancel).
  **CAUTION**: this file ALSO holds the content watcher
  (`flagExternalChange`, `loadTabContent`) - KEEP that; only
  `tab.watcher` (the `TerminalWatcherState`) is in scope.
- **F1 `web/src/api/client.ts`**: remove `setTerminalWatcher`,
  `unsetTerminalWatcher`, `terminalWatcherEvents`,
  `writeTerminalEventReply`, `setTerminalSubmitMode`, AND the forced
  rich-prompt-workspace methods `createRichPromptWorkspace`,
  `richPromptStatus`, `submitRichPromptWorkspace`,
  `closeRichPromptWorkspace` (they POST to `/api/rich-prompts*`, gone in
  A1) + the now-dead `RichPrompt*` response types.
- **F2 delete**: `web/src/state/watcherEvents.ts` (+ its test),
  `web/src/state/watcherScope.test.ts`, `web/src/api/watchScope.test.ts`.
- **F4 `web/src/App.svelte`**: change `spawnRichPromptFromContext` /
  the `case "app.terminal.richPrompt"` handler from "open rich prompt in
  focused pane" to the new flow: create a fresh Team Work Lead Terminal
  tab (markdown editor embedded at bottom, a Draft like Cmd+N), then
  `openTeamDialog` OVER it with the lead tab handle in the request. Also
  remove the `<SpawnDialog/>` mount + import here.
- **F11 delete**: `web/src/state/teamTemplate.ts` (+ tests),
  `docs/templates/team-process/*`, and the `?raw` entries that
  referenced them.

## Subagent A3 - Team Work component, dialog, orchestrator

Owns `TerminalRichPrompt.svelte`(->`TeamWork.svelte`),
`TeamDialog.svelte`, `teamDialog.svelte.ts`,
`teamOrchestrator.svelte.ts`, `SpawnDialog` deletion.

- **Rename + right-click menu** (`TeamWork.svelte`, menu ~530-601): keep
  Page width / Show source code / Show style toolbar; remove Spawn
  agent, Spawn agents, Copy metadata dir, Copy Spawn agents config; add
  a separator, then Bubble stack + Bubble tray, then a separator;
  Collapse prompt last.
- **Dialog redesign** (`TeamDialog.svelte` + `teamDialog.svelte.ts`):
  - "Your name" default "Alex" -> "Neo"; renders as `@@Neo`.
  - Keep "[x] Auto-prefix names with @@".
  - Replace "Team name" with a **Team configuration** New/Load toggle
    (same control style as Tabs/Split below):
    - New: "Path to configuration" defaulting to
      `/tmp/new-team-1/chan-team.toml`; info line "team management
      files will be created in /tmp/new-team-1".
    - Load: user enters a path; auto-validate or reject; on valid,
      prepopulate the New form from the config and stay editable;
      re-save the config (with edits) on Bootstrap.
  - Replace the agents slider with a **1-9 dropdown** (Number of
    agents: N).
  - MEMBERS rows add/remove with N; exactly one is the Lead (lands on
    the Lead Terminal). Relabel the "unassigned" chip to **"drag-me"**.
  - Keep the real-estate (Tabs in current Hybrid vs Split panes) UI.
  - Delete the "Copy config"/"Paste config" clipboard plumbing.
- **Bootstrap reorder** (`teamOrchestrator.svelte.ts`): the lead
  terminal already exists (created at Cmd+P). DELETE the
  close-host-then-respawn-lead block. New order: spawn the lead FIRST
  (into the open lead tab) then the workers; set `CHAN_TAB_NAME` per
  agent; place the identity prompt (below) into the lead's embedded
  editor; then broadcast **deselect-all**, then **enable only**
  lead+workers (use `setTerminalBroadcast*` to force-clear + set, NOT
  the toggle helper). Drop `placeTeamTemplates` and the `api.teamLoad`
  watcher attach.
- **Identity prompt** placed in the lead's editor:
  ```
  # Team work
  We are a team of {N}. Our host is {Host} and the team lead is {Lead}.
  You are $CHAN_TAB_NAME. Identify yourself and get ready to work with
  the rest of the team:
  - {Worker1}
  - {Worker2}
  ```
- **Cancel** path: delete the exact Lead Terminal tab created at Cmd+P
  (id in the dialog request); restore previous state.
- Delete `web/src/components/SpawnDialog.svelte` +
  `web/src/state/spawnDialog.svelte.ts`.

## Subagent A4 - TerminalTab + Bubble stub (highest-risk file)

Owns `TerminalTab.svelte`, `BubbleOverlay.svelte`, `Bubble.svelte`.

- **F3 `web/src/components/TerminalTab.svelte`**: remove the 5s watcher
  poll + `readWatcherEvents` import + `watcherStarted/Stopped/Detached`;
  rip out the rich-prompt-workspace archival block
  (`ensureRichPromptWorkspace`, `refreshRichPromptWorkspace`,
  `applyRichPromptWorkspace`, `discardRichPromptWorkspace`,
  `persistRichPromptSubmission`, `applyRichPromptSubmit`, ~1233-1468 -
  forced by A1's route deletion); rewrite `submitRichPrompt` to **reset
  `tab.teamWork.buffer = ""` after send** (roadmap "reset draft to
  empty"; this replaces `applyRichPromptSubmit`'s clear, now
  unconditional); drop the `watcherPath`/`bubbleCount` props to
  `<TeamWork>`.
- **F9 Bubble stub** (`BubbleOverlay.svelte` + `Bubble.svelte`): drop
  all `watcher`/`WatcherEvent`/`writeSurveyReply` dependencies; render a
  single STATIC example bubble from a local literal that demonstrates
  the survey modes (single-question, multi-question, the "F" follow-up
  affordance). "Bubble stack"/"Bubble tray" trigger it; clicking
  anything just dismisses it - no network, no filesystem.

## Coordination points (call out on the bus BEFORE editing)

- **F0 blocking checkpoint**: A2 lands `tabs.svelte.ts` and posts the
  exported `TeamWorkState` / helper names before A3/A4 start.
- **`<TeamWork>` prop contract** (A3 owns the component, A4 owns the
  parent `TerminalTab`): freeze first - drop `watcherPath`, `onSpawned`,
  `bubbleCount`; submit resets buffer.
- **`App.svelte` `<SpawnDialog/>` mount removal (A2) vs file deletion
  (A3)**: land in the same merge, or A2 after A3 confirms deletion.
- **A1 backend routes + A2/A4 calls**: land together to avoid a
  404-on-submit interim, or merge backend last (the wire only shrinks).

## Correctness risks (de-risk these)

1. Hidden rich-prompt-workspace coupling: deleting routes forces
   removing ~230 lines of `TerminalTab.svelte` archival plumbing the
   roadmap doesn't enumerate. Grep `RichPrompt` across `web/src/api/` +
   `TerminalTab.svelte` first.
2. Lead-first bootstrap: DELETE (don't reorder) the close-host/respawn
   dance; thread the lead tab id through the dialog request; Cancel
   deletes that tab. Rewrite `teamLeadRestart.test.ts` /
   `teamLeadPrompt.test.ts`.
3. Svelte-5 `$state`: keep the in-place mutation idiom for the buffer
   reset; remove (don't dangle) `$derived` like `bubbleCount` /
   `watcherPath`. **Browser-smoke** Cmd+Enter -> empty editor and
   Cancel -> tab removed.
4. Broadcast ordering: force-clear ALL then enable lead+workers (not the
   toggle); assert the final membership set in an orchestrator test.
5. Agent-submit chord: roadmap says Cmd+Enter behavior is UNCHANGED (only
   adds reset). Keep the client-side chord logic
   (`web/src/terminal/submitMode.ts`, `AGENT_SUBMIT_CHORD`); only the
   *server* submit-mode persistence route is deleted. Grep
   `AGENT_SUBMIT_CHORD`/`submitMode`/`agentTarget` for orphans.
6. `chan-team.toml` lives OUTSIDE the workspace sandbox (`/tmp/...`):
   deliberate app-level/dev-orchestration data, NOT notes content -
   do NOT route through `Workspace::write_text`. Scope the New/Load
   path read/write (small path-based capability vs the lead terminal's
   own shell cwd) and report your approach on the bus.

## Suggested slicing (you own the call)

- A1 backend deletion (parallel from turn 1; isolated to `crates/`).
- A2 frontend foundation (F0/F1/F2/F4/F11) - F0 is the blocking
  checkpoint that unblocks A3/A4.
- A3 component + dialog + orchestrator.
- A4 TerminalTab + bubble stub.

## Per-slice gate (mandatory before any "ready to merge")

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --no-default-features
(in web/)   npm run check  &&  npm run build  &&  npm test
```

Then append to `event-lane-a-alex.md`:

```
ready to merge: phase-13-r2-lane-a@<sha>  -  <one-line slice summary>
```

Browser-smoke (per `feedback_svelte_static_gate_misses_runtime`): the
Cmd+P lead-terminal flow, dialog Cancel/Bootstrap, submit-reset, the new
right-click menu order, and the bubble stub. chan-desktop smoke (per
`feedback_terminal_webgl_wkwebview`) for the broadcast deselect/enable
and any terminal-render-adjacent change.

## Coordination rules

- Append-only directional channels; never edit another agent's entries.
- Each turn, BEFORE acting, read `event-alex-lane-a.md` (inbox) and
  `event-lane-b-lane-a.md` (if it exists).
- Progress + merge-ready: append to `event-lane-a-alex.md`.
- Cross-lane to @@LaneB: append to `event-lane-a-lane-b.md`.
- Self-document in `lane-a/journal.md` (append a round-2 section).
- Subagents speak through you on the bus.

## First turn checklist

1. Bring the worktree to main on `phase-13-r2-lane-a` (above).
2. Read all recovery files.
3. Append an opening round-2 entry to `lane-a/journal.md`.
4. Kick A1 (isolated) + A2's F0 in parallel; gate F0 + post symbols;
   then release A3/A4.
5. Work each slice to the gate; report on `event-lane-a-alex.md`.

## Out of scope

Anything not in `roadmap-round-2.md`. Escalate scope creep on
`event-lane-a-alex.md`. Don't push to origin. Don't merge to main -
@@LaneB does that.
