# Phase-15 round-3 - @@LaneC (Team Work + Survey)

You are @@LaneC. Read `round-3-bootstrap.md` (process) and `round-3-status.md`
(active wave) first; the technical source is `round-3-plan.md` (Theme 1). You
own Team Work + the Survey rebuild - the round's biggest scope. Spawn subagents
(split backend vs frontend).

## Your files (no other lane edits these)

- web/src/state/teamDialog.svelte.ts, teamOrchestrator.svelte.ts,
  teamConfigPath.ts
- web/src/components/TeamDialog.svelte, TeamWork.svelte, BubbleOverlay.svelte;
  web/src/state/bubbleStub.svelte.ts + the rebuilt survey overlay
- crates/chan-server/src/routes/team_config.rs + a new survey reply route/bus
- the team `bootstrap.md` generator

Do NOT edit the CLI / control-socket / submitMode / desktop files - those are
@@LaneD. Your `cs terminal survey` TRANSPORT goes THROUGH @@LaneD (touch point);
you build the UX + backend route + followup files.

## Your work scope, by wave

### Wave 1 - team in the workspace + delete the Rich-prompt

- Move the team INTO the workspace under a user-chosen `{team-name}/` dir via
  `Workspace::write_text` (sandbox + atomic), NOT the current outside-sandbox
  /tmp path (teamConfigPath.ts:7, team_config.rs). Structure:
  - `{team-name}/config.toml`
  - `{team-name}/bootstrap.md`
  - `{team-name}/tasks/task-{from}-{to}-{n}.md` (owned by `to`, append-only)
  - `{team-name}/journals/journal-{member}.md` (owned by each member)
  - `{team-name}/followups/followup-{from}-{to}-{n}.md` (owned by `to`)
- Validate on reload (reuse the `<=9` cap from teamDialog.svelte.ts:159 +
  structural checks).
- Generate `bootstrap.md`: the process for all members, the roster, reveal
  @@Host and @@Lead, the poke 1-liner + the hold-for-@@Lead distribution flow.
- DELETE the "Rich prompt" widget + all dead bubble-stub code (bubbleStub, stub
  BubbleOverlay payloads, TeamWork menu entries, leftover rich-prompt refs).
- This wave has NO @@LaneD dependency; stay out of main.rs / control_socket.

### Wave 2 - survey rebuild (needs @@LaneD Wave-1 cs-shell)

- `cs terminal survey` raises bubbles over a tab / group: single-question,
  markdown body, up to 4 vertically aligned options + an `[F]` follow-up.
- DIVISION OF LABOUR (the survey contract, @@Architect arbitrates): @@LaneD
  builds the TRANSPORT - the `cs terminal survey` command (in chan-shell), the
  `control_socket` TermSurvey frame, the WindowCommand that shows the overlay,
  and carrying the reply back to the BLOCKED CLI (the command returns the chosen
  option synchronously). YOU build the SPA overlay (real, reply round-trip,
  replacing the stub), the reply payload, and the `[F]` -> followup file:
  `{team-name}/followups/followup-...md`, pre-populated with header/title,
  date+time, "Agents: this is a follow up, not ready; check again later", the
  original prompt, and @@Host comment placeholders; return its path to the CLI.
- The agent-type field on the team config (so the bootstrap uses the right
  submit encoding) consumes @@LaneD's per-agent submit map.
- Browser-smoke + a REAL-agent smoke: survey raised over a running claude tab,
  the option returns to the CLI, `[F]` writes the followup file.

### Wave 3 - polish + bootstrap.md finalize + smoke

- Survey/team polish; finalize the `bootstrap.md` content; and (with @@LaneD)
  the multi-agent submit / team-work plumbing smoke tests. Carryover buffer.

## Touch points (the survey contract - @@Architect holds it)

- C<->D (Wave 1->2): chan-shell must land in @@LaneD Wave-1 before your survey
  command. Agree the survey payload/reply SHAPE (a shared type) at the W1/W2
  boundary; @@LaneD adds the frame + transport, you render + reply.
- You consume @@LaneD's per-agent submit map for the team-config agent field.

## Completion (each wave)

Gated-green + local merge + journal entry + poke @@Architect "wave N done".
