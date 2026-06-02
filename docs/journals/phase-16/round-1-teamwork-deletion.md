# Delete the OLD Team Work prompt bubble ONLY (@@LaneB, from @@Host)

CORRECTED SCOPE (supersedes the earlier "whole GUI" version). @@Host:
"all of the OLD team work stuff should be gone, the NEW team work must remain
- the new one also has the dialog setup and load, and those remain."

## The OLD vs NEW boundary (grounded)

- **OLD = the prompt-composer BUBBLE.** `TeamWork.svelte` is literally the old
  rich prompt renamed (git `c4a4adc6 chore(team-work): scrub all 'rich prompt'
  identifiers -> team work`). It is driven by the `tab.teamWork` state and the
  "Show/Hide Team Work" toggle. The NEW Rich Prompt replaces it. -> DELETE.
- **NEW = the Team Work DIALOG + setup + load + orchestration.**
  `TeamDialog.svelte` ("Team Work dialog... Bootstrap runs the lead-first
  orchestrator"), the spawn/load flow, and the orchestrator. @@Host: these
  REMAIN. -> KEEP.

LITMUS TEST for any reference: touches `tab.teamWork` / renders `<TeamWork>` /
the prompt composer/placeholder -> DELETE. Touches `TeamDialog` /
`openTeamDialog` / `runTeamBootstrap` / `createTeamWorkLeadTerminal` / the
load/setup flow -> KEEP.

## HARD CONSTRAINTS

- Frontend only. KEEP backend team_config.rs (the dialog/load + CLI use it).
- Do NOT break the new Rich Prompt (same files): keep
  registerTerminalPromptSink/sendPrompt/`<RichPrompt/>`/Cmd+Shift+P/the
  "Show/Hide Rich Prompt" menu entry.
- Do NOT break the NEW Team Work dialog/spawn/load.
- `make web-check` GREEN is the dangling-ref gate. Pathspec commits.

## DELETE (OLD bubble only)

Wholesale (bubble-only files):
- web/src/components/TeamWork.svelte
- web/src/components/TeamWork.test.ts
- web/src/components/teamWorkAutoFocus.test.ts
- web/src/components/teamWorkCursorAlignment.test.ts
- web/src/components/teamWorkPlaceholderExtension.test.ts
- web/src/components/teamWorkFollowUp.test.ts
- web/src/components/altSpaceTeamWorkRemoved.test.ts  (VERIFY: it tests the old
  bubble; if it actually guards NEW behavior, keep - read it first)

Excise (the `tab.teamWork` bubble state + its drivers; KEEP dialog/spawn):
- TerminalTab.svelte: drop `import TeamWork`; the `{#if tab.teamWork?.open}
  <TeamWork/>` mount + its height/margin reservation; ensureTeamWork/
  openTeamWork/closeTeamWork/toggleTeamWorkFromMenu/teamWorkUsesAgentSubmit/
  submitTeamWork; the "Show/Hide Team Work" right-click menu button. KEEP Rich
  Prompt. (TeamDialog is not in this file.)
- tabs.svelte.ts: drop the `TeamWorkState` type; `teamWork?` field;
  `teamWork: undefined` default; `openActiveTeamWork()` (sets tab.teamWork.open
  = the bubble); `loadTeamWorkSeedText()`. KEEP `createTeamWorkLeadTerminal`
  (NEW spawn) + registerTerminalPromptSink/sendPromptToActiveTerminal.
- App.svelte: drop ONLY chords/handlers that open the BUBBLE
  (openActiveTeamWork path). KEEP `import TeamDialog`, the TeamDialog render,
  `spawnTeamWorkFromContext()` (-> createTeamWorkLeadTerminal + openTeamDialog),
  the `case "app.terminal.teamWork"` (opens the dialog), the teamDialog
  paneChordBlocked guard.
- Test files: excise ONLY the `tab.teamWork`/bubble assertions; keep
  dialog/spawn assertions (tabs.test.ts, TerminalTab.test.ts, Pane.test.ts,
  PathPromptModal.test.ts, toastAutoDismissSweep.test.ts, chordEscapeRegistry.
  test.ts - the bubble chord only). cmdPRichPrompt3State.test.ts references
  createTeamWorkLeadTerminal (NEW) - keep that.

## RESOLVE-BY-LITMUS (ambiguous; classify, do not guess)

- `paneModeOpenTeamWorkTerminal` (tabs.svelte.ts) + the Cmd+Alt+P mac fallback
  + the Hybrid-Nav `p` handler: does each open the BUBBLE (openActiveTeamWork /
  tab.teamWork) or the DIALOG (spawnTeamWorkFromContext / openTeamDialog)?
  Bubble -> DELETE; dialog/spawn -> KEEP. Read each and tag.
- The 3 files you flagged: newTeamButton, teamBootstrapOrchestrator,
  teamLoadFlow -> these are the NEW dialog/spawn/load/orchestration = KEEP
  (confirm none of them drive the old `tab.teamWork` bubble).
- KEEP wholesale (NEW team work): TeamDialog.svelte/.test.ts,
  teamDialog.svelte.ts/.test.ts, teamOrchestrator.svelte.ts/.test.ts,
  teamConfigPath.ts, teamLeadPrompt.test.ts, teamLeadRestart.test.ts.

## VERIFY

- `make web-check` GREEN (catches dangling refs from the excisions).
- Browser-smoke on :8787: the OLD bubble + its "Show/Hide Team Work" toggle are
  GONE; the NEW Team Work dialog/spawn/load STILL works; Rich Prompt STILL
  works. App boots with no console errors.
- Pathspec commits; post shas; flag any ref you could not cleanly classify.
