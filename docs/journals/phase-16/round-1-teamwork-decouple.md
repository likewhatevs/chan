# Decouple the Team Work bubble: tie it to the LEAD terminal only

SUPERSEDES round-1-teamwork-deletion.md (NOTHING is deleted wholesale now).
@@Host reframed: PRESERVE the old bubble, do NOT delete it; DECOUPLE it from
regular terminals and tie it to the team-LEAD terminal.

## The final model (confirmed with @@Host)

- **Rich Prompt** = UNIVERSAL, unchanged. Every terminal (regular AND the
  lead) has it via Cmd+Shift+P show/hide. Do NOT touch Rich Prompt.
- **Team Work bubble** (`TeamWork.svelte`, the old composer) = PRESERVED, but
  tied to the LEAD terminal ONLY. It appears because the Cmd+P -> dialog
  (configure new / load existing team) workflow creates a lead terminal and the
  orchestrator primes its bubble. It is NO LONGER a per-terminal thing you can
  summon anywhere.
- **Lead terminal** has BOTH (Rich Prompt via Cmd+Shift+P + the Team Work
  bubble from the workflow). **Regular terminal** has ONLY Rich Prompt.
- Cmd+P stays the team-workflow entry point. Don't mix Team Work with the
  regular terminal.

## KEEP (do NOT delete - this reverses the old deletion map)

- `TeamWork.svelte` + its bubble tests - PRESERVED.
- `tab.teamWork` state (`TeamWorkState`, the field, defaults, serializer/
  restore) - KEEP; it is how the bubble binds to the lead terminal + persists.
- the `<TeamWork>` mount + its height reservation in TerminalTab.svelte - KEEP
  (the bubble still renders on the lead terminal when `tab.teamWork` is set).
- `submitTeamWork` - KEEP the function but REWIRE it (see "SUBMIT THROUGH THE
  QUEUE" below).
- `teamOrchestrator` priming the bubble (`primeTeamWork` line ~407 + the
  agentTarget/submitMode set) - KEEP. Decision (a) (rewire to Rich Prompt) is
  MOOT/dropped.
- The whole NEW workflow: TeamDialog, teamDialog state, teamConfigPath,
  teamLead*, createTeamWorkLeadTerminal, spawnTeamWorkFromContext, the Cmd+P
  entry, newTeamButton/teamBootstrapOrchestrator/teamLoadFlow, backend
  team_config.rs - KEEP.

## REMOVE (the decoupling: any-terminal access to the bubble)

The ONLY removals are the entry points that let a user summon the Team Work
bubble on an ARBITRARY / regular terminal:

- The "Show/Hide Team Work" right-click menu row + `toggleTeamWorkFromMenu` in
  TerminalTab.svelte. (The menu toggle is exactly the "anyone could bring it up
  and down anytime" access @@Host is removing.)
- `openActiveTeamWork` (tabs.svelte.ts) AS A USER ACTION on the active
  terminal, and `paneModeOpenTeamWorkTerminal` + the Hybrid-Nav `p` handler
  (App ~561) + its import (App ~92) IF they open the bubble on a non-lead
  terminal. Trace each: if it is a user-facing "open the bubble on this
  terminal" path -> REMOVE; if it is the workflow/orchestrator setting the
  bubble on the LEAD -> KEEP.
- Net: after this, `tab.teamWork` is ONLY ever set by the team workflow on the
  lead terminal - never by a manual per-terminal toggle.

## SUBMIT THROUGH THE QUEUE (@@Host: both bubbles always use the queue)

The Team Work bubble must submit through the cs-write QUEUE, like Rich Prompt -
NOT the old keystroke path. Today `submitTeamWork` uses `sendUserInput()` +
AGENT_SUBMIT_CHORD (raw keystrokes). REWIRE it to send the `prompt` frame
(`{type:"prompt", data, agent}`) over the terminal WS - the SAME producer Rich
Prompt uses (landed in 3d6d144e) - so the bubble's submit enqueues + drains
serialized with everything else. The bubble already knows its agent
(`agentTarget`/`submitMode`), so PASS that as the frame's `agent` (this also
fixes the per-agent-chord caveat: a codex/gemini lead gets the right chord
because the frame carries the agent, not the claude default). Net: Rich Prompt
AND the Team Work bubble both submit via the queue, always.

## LITMUS

- Does it let a USER open the Team Work bubble on an arbitrary terminal?
  -> REMOVE.
- Does it render/submit/persist the bubble on the LEAD terminal as part of the
  workflow? -> KEEP.
- Is it Rich Prompt? -> DO NOT TOUCH.

## VERIFY

- `make web-check` GREEN. Tests that asserted a user-toggleable Team Work on
  any terminal should be updated to the lead-only model (don't just delete
  coverage - reflect the new behavior).
- Browser-smoke on :8787: a regular terminal has NO "Show/Hide Team Work" menu
  entry and cannot summon the bubble; Cmd+Shift+P Rich Prompt still works
  everywhere; Cmd+P -> dialog -> new/load still spawns a lead terminal whose
  Team Work bubble appears (primed). The lead terminal ALSO has Rich Prompt
  (Cmd+Shift+P).
- Pathspec commits; post shas; flag any path you cannot cleanly classify as
  user-summon (remove) vs workflow-on-lead (keep).
