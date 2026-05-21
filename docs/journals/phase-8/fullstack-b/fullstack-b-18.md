# fullstack-b-18: Submit-mode persistence on reload + tooltip copy fix

Owner: @@FullStackB
Date: 2026-05-21

## Goal

Two related submit-mode wire bugs combined since both touch
the same `-b-13` SerTab / server-side submit-mode path:

1. SerTab `rpsm` persists SPA-side across page reload but
   server-side `Session.agent_mode` doesn't re-sync on
   remount → toolbar UI says Agent, server emits Shell
   chord. Fix: SPA re-fires `setTerminalSubmitMode` on tab
   restore.
2. Shell-mode toggle's tooltip reads "Submit mode: shell
   (Cmd+Enter sends a trailing newline)" but the submit
   handler is `sendUserInput(source)` pass-through — no
   newline is appended. Update tooltip copy to match
   actual behaviour.

## Background

Bug entries:

* [`../phase-8-bugs.md`](../phase-8-bugs.md) — "Rich prompt
  submit-mode doesn't survive page reload (server-side
  state desync)" (filed 2026-05-20).
* @@WebtestB lane-B verification append on `-b-13` flagged
  the tooltip copy nit (2026-05-21).

Root cause: `-b-13` server-side commit landed
`Session.agent_mode: AtomicBool` which defaults to false
(Shell) on every session spawn. Reload path: SPA
reconstitutes `TerminalRichPromptState.submitMode` from
SerTab on remount but does NOT replay the
`PUT /api/terminal/:session/submit-mode` to sync the
server side. If the session id changes across reload (or
chan-server restarts), the new server-side `agent_mode`
defaults to Shell. UI shows Agent; server emits Shell
chord. Mismatched state.

## Authorization

**Authorization: yes**, covers:

* `web/src/state/tabs.svelte.ts` (or wherever rich-prompt
  state reconstitution from SerTab lives).
* `web/src/components/TerminalRichPrompt.svelte` (the
  toolbar toggle's tooltip + any mount-effect changes).
* `web/src/components/BubbleOverlay.svelte` if needed for
  the tooltip nit (sibling toolbar render site).

No chan-server / Tauri changes expected (this is SPA-side
re-sync logic + a string update).

@@FullStackB may proceed without further @@Alex confirmation.

## Acceptance criteria

### A — Submit-mode re-sync on tab restore

* On rich-prompt state restore from SerTab (mount or
  WebSocket reconnect), if `submitMode === "agent"`, fire
  `api.setTerminalSubmitMode(sessionId, "agent")` as a
  follow-up. Idempotent — safe to fire even if the server
  already has the right mode.
* Single source of truth: server-side `Session.agent_mode`
  governs dispatch; SPA's job is to keep server in sync
  with the persisted SerTab value.
* Vitest pin: mock tab restore with `rpsm: "a"` →
  assert `setTerminalSubmitMode` API call fires with
  `"agent"` argument.

### B — Tooltip copy fix

* Shell-mode toggle's tooltip removes the misleading
  "(Cmd+Enter sends a trailing newline)" phrase. Replace
  with accurate copy describing actual behaviour:
  "Submit mode: shell (default; Cmd+Enter submits buffer
  verbatim)" — or whatever wording reflects what
  `sendUserInput(source)` actually does.
* Agent-mode toggle tooltip stays as today (the chord
  appending IS what happens; copy already accurate).
* Pre-push gate: clean.

## How to start

1. Grep `tabs.svelte.ts` for the `submitMode` /
   `TerminalRichPromptState.submitMode` field handling.
   Find the deserialize path.
2. Add the re-sync call after deserialize completes (with
   a small delay if needed for the session to be reachable
   — typically a `tick()` or `setTimeout(0)` is enough).
3. Wire any error-handling (server error on the PUT
   shouldn't break the SPA — log + retry / fall back to
   shell mode is acceptable).
4. Locate the toolbar toggle's tooltip — likely in
   `TerminalRichPrompt.svelte` adjacent to the
   shell/agent toggle button mount.
5. Update the tooltip text.
6. Write the vitest pin.
7. Test locally: flip to Agent mode → reload page →
   confirm survey-reply or Cmd+Enter dispatches the agent
   chord (not `\n`). Check the tooltip on the Shell-mode
   button reads accurately.
8. Append commit-readiness.

## Coordination

* Independent of other v0.11.2 tasks.
* **Rides v0.11.2 mini-wave** per
  [`../architect/commit-plan-v0.11.2.md`](../architect/commit-plan-v0.11.2.md).
  Parallelisable.
* The companion **survey-reply broadcast** bug (filed
  2026-05-20 — three implementation options unresolved)
  is NOT in v0.11.2 scope; that one needs a design
  decision and stays for Round-2 wave-2. This task fixes
  the SAME `-b-13` submit-mode wire's reload-persistence
  issue without touching the broadcast plumbing.

## Open questions

(populated as you investigate)
