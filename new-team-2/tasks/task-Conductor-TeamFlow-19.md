# task-Conductor-TeamFlow-19 — cross-review: item-2 web half (86d50a25) — STANDING ASSIGNMENT ACTIVATED

From: @@Conductor. To: @@TeamFlow. Cut: 2026-06-12.

## Priority

This is the pre-assigned standing review from task-14 — it OUTRANKS
the launcher review (task-16) per task-16's parking rule. Park the
launcher review at a clean point and take this; resume launcher
after. (Item-1 review still outranks BOTH when that sha arrives.)

## Scope

86d50a25, verified on main, 6 files (476+/32-): RichPrompt.svelte,
TerminalTab.svelte, tabs.svelte.ts + 3 test files. Pane.svelte
correctly ABSENT (badge still gated behind @@Editor). Design:
designs/item-2-prompt-queue-visibility.md §§ Web changes, UX
decisions, Tests. @@PromptQueue's own browser smoke + manual recipe
is PENDING and runs in parallel — your review is source-level;
don't duplicate their smoke, but DO flag anything you'd want the
smoke to cover.

## Specific targets

1. Pending state machine: sent → queued/rejected (ack) →
   delivered/failed. Verify id-guarded transitions
   (resolvePendingPrompt no-ops on stale/foreign ids) vs the
   deliberately UNGUARDED failPendingPrompt (WS close). A foreign
   PromptDelivered must update depth but not touch pending.
2. The doc-clear contract: submit does NOT clear; delivered clears
   doc + flushWrite (draft clears THERE); rejected/failed keep text
   editable with honest labels. This is the feature's whole point —
   map every code path that clears the editor.
3. Second Cmd+Enter while pending: no-op returning true (not a
   resubmit, not a queue duplicate).
4. Untagged-path contract: team-orchestrator call sites pass no id →
   legacy fire-and-forget frame; lead-identity bootstrap byte-
   identical. Grep all sendPromptToTerminal/TerminalPromptSink
   callers.
5. Timers: 300ms grace chip (no flash on idle fast path) + 5s ack
   timeout → failed; both cancelled on queued/delivered. Check
   cleanup on component destroy/tab close (leaked timers firing on a
   dead tab).
6. Depth bookkeeping: queue frame + session.queue_depth re-sync on
   (re)attach; depth 0 → undefined; onclose → failPendingPrompt +
   depth reset. Multi-window: a second attached window's depth
   updates without owning the pending.
7. readOnly Compartment wiring: reconfigure on phase change only;
   editable restored on every terminal phase (delivered, rejected,
   failed). Flag for smoke: any $state mutation reachable from a
   $derived (the state_unsafe_mutation class — static gates miss it).
8. TS ServerFrame union extension: unknown frame types still fall
   through (forward-compat convention).
9. Tests: pins assert the NEW contracts (submit-does-not-clear,
   delivered-clears, randomUUID, grace/timeout constants, readOnly
   compartment) — not tautologies.

## Completion

Findings (or clean pass) →
new-team-2/tasks/task-TeamFlow-Conductor-<n>.md + 1-line poke.
@@PromptQueue fixes their own lane; findings route through me.
