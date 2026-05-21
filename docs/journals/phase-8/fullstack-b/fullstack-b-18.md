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

## 2026-05-21 — implementation note

Two SPA-only fixes, no chan-server or Tauri changes.

### A — Submit-mode re-sync on tab restore

* **`web/src/state/tabs.svelte.ts`** — inside the `if (kind === "t")`
  branch of the restore loop, after `richPromptFromSer(...)`
  resolves, fire `api.setTerminalSubmitMode(terminalSessionId,
  "agent")` when BOTH a `terminalSessionId` is present AND
  `richPrompt?.submitMode === "agent"`. Fire-and-forget with a
  `console.warn` on rejection so a stale session (404) or 5xx
  doesn't break the restore. Skipped entirely when the persisted
  mode is shell (server's default) or when no session id has
  attached yet (PUT would 404 unconditionally pre-attach).

### B — Shell-mode tooltip copy fix

* **`web/src/components/TerminalRichPrompt.svelte`** line 481:
  shell-mode tooltip changed from
  `"Submit mode: shell (Cmd+Enter sends a trailing newline)"`
  to
  `"Submit mode: shell (default; Cmd+Enter submits the buffer
  verbatim)"`. The agent-mode tooltip stayed as-is (its copy is
  already accurate — chord IS appended).

### Tests landed (vitest 555 → 558)

| Test                                                                | Pinned contract                                                           |
|---------------------------------------------------------------------|---------------------------------------------------------------------------|
| `re-syncs server-side submit-mode on tab restore`                   | Spy on `api.setTerminalSubmitMode` → on restore of an agent-mode tab, the PUT fires with `("term_rpsm_restore", "agent")`. |
| `skips submit-mode resync on tab restore when mode is shell`        | Server's default is shell; an explicit re-sync would be noise. PUT must NOT fire when persisted mode is shell. |
| `skips submit-mode resync when no terminalSessionId is present`     | Pre-attach tabs (no session id) would have the PUT 404 unconditionally; skip until attach. |

### Acceptance criteria — verification

| Criterion                                                                            | State                                                                                              |
|--------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| Re-sync PUT fires after deserialize when `submitMode === "agent"`                    | Landed in the restore loop's `if (kind === "t")` branch.                                            |
| Idempotent (safe if server already has the right mode)                               | PUT semantics on the server (per `-b-13`) — sending "agent" when server is already "agent" is a no-op 204. |
| Vitest pin: tab restore with `rpsm: "a"` → `setTerminalSubmitMode("agent")` fires    | Landed (test A above).                                                                              |
| Shell-mode tooltip removes the misleading "(sends a trailing newline)"               | Replaced with accurate copy: `"shell (default; Cmd+Enter submits the buffer verbatim)"`.            |
| Agent-mode tooltip unchanged                                                         | Yes — chord-append behavior accurately described already.                                            |
| Pre-push gate                                                                         | Svelte-check (3980 files / 0 errors) + vitest (555 → 558) green. Rust unchanged.                    |

### Coordination footprint

* SPA-only — no chan-server, no Tauri.
* No file overlap with `-b-17` or `-b-19`.
* The companion **survey-reply broadcast** bug (filed
  2026-05-20 — three implementation options unresolved) is NOT
  in this task's scope per the task body's explicit framing.
  That one needs a design decision and stays for Round-2
  Wave-2.

### Suggested commit subject

```
Rich prompt: re-sync submit-mode on tab restore + tooltip copy fix (fullstack-b-18)
```

Touches:
* `web/src/state/tabs.svelte.ts`
* `web/src/state/tabs.test.ts`
* `web/src/components/TerminalRichPrompt.svelte`

Holding for @@Architect commit clearance. Push waits until the
v0.11.2 commit-grouping cut.
