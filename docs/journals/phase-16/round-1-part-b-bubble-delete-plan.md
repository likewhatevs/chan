# Team Work BUBBLE DELETE - executable resume spec (@@LaneB)

Status at break (teardown): NOT started in code (clean tree @ 238639de, green).
This captures the FULL analysis so the delete executes fast post-break with no
re-investigation. Scope per @@Lead 16:58 + the release poke: delete the OLD
Team Work bubble COMPLETELY; lead becomes a normal terminal; KEEP TeamDialog +
orchestrator spawn/load + teamConfigPath + teamLead* + Cmd+P.

## HELD PIECE - RESOLVED (verified against code; do this, don't re-litigate)

primeTeamWork was the ONLY lead identity delivery. Verified in
teamOrchestrator.svelte.ts launchTeam/launchLead: the lead's identity prompt is
delivered ONLY via primeTeamWork(leadTab, prompt) (into the bubble). WORKERS get
NO orchestrator prompt-poke - they spawn with command/env ($CHAN_TAB_NAME) and
self-bootstrap via the generated bootstrap.md (the lead then drives them). So
there is NO pre-existing "bootstrap poke" to reuse. Deleting primeTeamWork
blindly STRANDS the lead. RESOLUTION (matches @@Host queue preference, @@Lead
confirmed RIGHT): AUTO-DELIVER the identityPrompt to the lead via the QUEUE
(prompt frame), with the lead's agent + a retry until the lead WS is open.

## WIRING (do first - the delete depends on it)

1. tabs.svelte.ts - make the prompt sink report whether the send went out, so
   the orchestrator can retry until the lead WS is open:
   - `type TerminalPromptSink = (data: string, agent?: string) => boolean;`
   - sendPromptToActiveTerminal: `return sink(data, agent);` (was `sink(...);
     return true;`).
   - ADD: `export function sendPromptToTerminal(tabId, data, agent?): boolean {
       const sink = terminalPromptSinks.get(tabId); return sink ? sink(data,
       agent) : false; }`
2. TerminalTab.svelte:
   - `send(frame): boolean` -> `if (!ws || ws.readyState !== WebSocket.OPEN)
     return false; ws.send(JSON.stringify(frame)); return true;`
   - `sendPrompt(data, agent?): boolean` -> `return send({ type: "prompt",
     data, ...(agent ? { agent } : {}) });`
   (sendInput ignores the boolean - fine.)
3. teamOrchestrator.svelte.ts - replace primeTeamWork(leadTab, prompt) + the
   `if (leadTab.teamWork) { agentTarget/submitMode }` block (lines ~407-421)
   with:
   ```
   const leadAgent = leadEntry.agent ?? "none";
   void deliverLeadIdentity(leadTab.id, prompt, leadAgent === "none" ? undefined : leadAgent);
   ```
   and add a module helper:
   ```
   async function deliverLeadIdentity(tabId: string, text: string, agent?: string): Promise<void> {
     for (let i = 0; i < 40; i++) {            // ~10s at 250ms; lead WS opens in ~1s
       if (sendPromptToTerminal(tabId, text, agent)) return;
       await new Promise((r) => setTimeout(r, 250));
     }
     notify("team lead did not connect; deliver its identity prompt manually");
   }
   ```
   - import: drop `primeTeamWork`, add `sendPromptToTerminal` from tabs.svelte;
     `notify` already imported; keep `setActivePane(ctx.leadPaneId)` (424).

## DELETE - tabs.svelte.ts (all reference the dead bubble after the rewire)

- `export type TeamWorkState = {...}` (377-434).
- `teamWork?: TeamWorkState;` field (282).
- `export function openActiveTeamWork()` (1059-1086) + `function
  blurTerminalHelperTextarea()` (1088-1106) [blur is only used by
  openActiveTeamWork - verify with grep].
- createTeamWorkLeadTerminal (1113-1120): delete the `openActiveTeamWork();`
  call (1119) so it's a plain spawn; update its doc comment ("opens the Team
  Work editor" -> just spawns the lead terminal).
- `export function primeTeamWork()` (1471-1488).
- defaults: `teamWork: undefined,` (1161 openTerminalInPane, 2770 the other tab
  ctor) + `teamWork: src.teamWork ? {...} : undefined,` (2339 the tab clone).
- serializer EMIT block `...(opts.terminalSessions && t.teamWork ? { rpb...rpsq
  } : {})` (3726-3756) - KEEP the rpd emit right above it (3722-3724).
- SerTab fields: rpn (3502), rpsq (3503), the "Team Work draft state" comment +
  rpb (3527-3528), rph/rpo/rpm/rpc(+comment)/rppw(+comment)/rpsm(+comment)/rpa
  (3534-3546). KEEP rpd + its comment (3529-3533).
- `function teamWorkFromSer()` (4186-4265).
- restore-1: `const teamWork = teamWorkFromSer(sertab, savedTerm);` (4002) +
  `teamWork,` in the tab object (4032). KEEP the richPromptDraftPath line.
- restore-2: `const teamWork = teamWorkFromSer(savedTerm); if (teamWork)
  liveTerms[j]!.teamWork = teamWork;` (4294-4295). KEEP the rpd line (4296).
(Line numbers are pre-delete; shift as you go - prefer text anchors / delete
bottom-up.)

## DELETE - TerminalTab.svelte

- `import TeamWork from "./TeamWork.svelte";` (~99).
- focus-gate: replace `if (tab.teamWork?.open) { ...focusNonce...; return; }
  term?.focus();` (~298-305) with just `term?.focus();`.
- `function teamWorkUsesAgentSubmit()` + `function submitTeamWork()` (the
  rewired one) - both gone (no mount calls them now).
- the `{#if tab.teamWork?.open}<TeamWork .../>{/if}` mount (~1714) + the
  `.terminal-host` `style:margin-bottom={tab.teamWork?.open ? ...}` reservation
  (~1740) -> drop the style:margin-bottom entirely.
- KEEP: sendPrompt (Rich Prompt sink uses it), the <RichPrompt {tab}/> mount,
  registerTerminalPromptSink, the close-sink discardDraft.

## WHOLESALE DELETE (git rm)

web/src/components/{TeamWork.svelte, TeamWork.test.ts, teamWorkAutoFocus.test.ts,
teamWorkCursorAlignment.test.ts, teamWorkPlaceholderExtension.test.ts,
teamWorkFollowUp.test.ts}.

## KEEP (do NOT touch)

TeamDialog.svelte/state, teamOrchestrator spawn/load (only the primeTeamWork
delivery changes), teamConfigPath, teamLead*, newTeamButton/teamBootstrap/
teamLoad, App.svelte spawnTeamWorkFromContext + Cmd+Alt+P + case
app.terminal.teamWork, shortcuts app.terminal.teamWork id, EmptyPaneWelcome/Pane
"Team Work" spawn entries, submitMode.ts (NEW teamDialog + tests use
AGENT_SUBMIT_CHORDS), Rich Prompt (all of it).

## TESTS to update (lean on web-check to find them all)

- TerminalTab.test.ts: the "Team Work revamp (source contract)" describe
  (submitTeamWork/teamWorkUsesAgentSubmit/<TeamWork> mount asserts) - delete it
  (bubble gone).
- altSpaceTeamWorkRemoved.test.ts: bubble-era; asserts a "Team-work entry
  points ... Mod+. p" comment (stale: Hybrid-p removed in the decouple) - delete
  it (Alt+Space-removal coverage is obsolete; Cmd+P/Cmd+Alt+P dialog entry is
  covered in paneModeKeymap.test.ts).
- Any tabs.test.ts / Pane.test.ts / PathPromptModal.test.ts /
  toastAutoDismissSweep.test.ts stray teamWork refs -> excise the bubble
  assertions (web-check will name them).
- richPromptComponent/richPromptTerminalWiring: unaffected (Rich Prompt kept).

## VERIFY

make web-check GREEN (svelte-check catches every dangling ref). Browser-smoke:
regular terminal has NO Team Work bubble/menu; Cmd+P -> dialog -> new/load
spawns a lead that AUTO-RECEIVES its identity prompt via the queue (reads
bootstrap.md, drives workers); Rich Prompt still works everywhere. Pathspec
commit; post sha.
