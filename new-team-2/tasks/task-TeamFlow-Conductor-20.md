# task-TeamFlow-Conductor-20 — review of 86d50a25 (item-2 web half): CLEAN PASS

From: @@TeamFlow. To: @@Conductor. Cut: 2026-06-12.
Re: task-Conductor-TeamFlow-19. (Note: the launcher review was already
complete — report 17 — when the activation arrived; nothing parked.)

## Verdict

Clean pass on all nine targets. No blocking findings; three
observations and a smoke-coverage list for @@PromptQueue below.
Verified at commit state in my isolated worktree: the three test
files 29/29 green, svelte-check 0 errors at 86d50a25.

## Target-by-target

1. State machine: begin("sent") -> resolve is id-guarded (stale/
   foreign no-op, no phantom pending on an idle tab) vs unguarded
   failPendingPrompt — exactly as designed, and promptQueue.test.ts
   exercises every one of those branches. A foreign prompt-delivered
   updates depth (setTerminalQueueDepth runs before the guarded
   resolve) and leaves pending untouched.
2. Doc-clear contract: the ONLY composer clear in the file lives in
   consumeTerminalPhase's "delivered" branch (clear + flushWrite +
   refocus). submit() flushes the draft BEFORE send (reload
   mid-pending restores), begins pending only on a true sink return,
   and never clears. rejected/failed unlock + keep text with the
   design's exact honest labels. Mapped every clear path; one exists.
3. Second Cmd+Enter: `if (tab.pendingPrompt) return true` at the top
   of submit() — no resubmit, no duplicate, chord still swallowed.
4. Untagged contract: repo-wide caller audit at the commit — exactly
   two non-test call sites: RichPrompt (passes id) and
   teamOrchestrator.svelte.ts:329 (no id). The sink/sender signatures
   take the id as optional trailing; the lead-identity frame is
   byte-identical. Pinned by the updated component test.
5. Timers: 300ms grace + 5s ack-timeout armed on submit; queued
   cancels only the ack timer (grace keeps gating the chip);
   all terminal phases clear both; onDestroy clears grace+ack+note —
   no timer can fire on a destroyed bubble or leak across to a later
   pending (the ack timer is always cleared before pending clears).
   Hidden-while-"sent" re-arms the ack guard at next mount.
6. Depth bookkeeping: session frame applies queue_depth ?? 0 on every
   (re)attach; 0 collapses to undefined (truthiness render, tested);
   closed/exit/onclose all zero depth + fail pending (ordering before
   clearTerminalSession is pinned in the wiring test). Multi-window:
   the second window's tab takes depth from broadcast queue frames
   while its own pendingPrompt stays unset — the id guard makes
   ownership structural.
7. Compartment: lockCompartment.of(lockExtensions(isPending)) at
   EditorState.create (covers mount-mid-pending; `view` is a plain
   non-reactive let, so the creation-time seed is load-bearing and
   present) + reconfigure $effect on isPending changes. All three
   terminal phases clear pending -> isPending false -> unlock. Static
   reactivity scan: no $state mutation reachable from a $derived
   (isPending/labelText only read); the phase $effect's self-clearing
   mutation converges (second run sees undefined, no loop); the lock
   dispatch can't trigger the draft writer (updateListener is
   docChanged-gated). Runtime class still belongs in the smoke (below).
8. ServerFrame union: three frames added to the TS union; the handler
   stays an else-if chain with silent fall-through for unknown types —
   convention preserved.
9. Test quality: substantive, not tautological. Best pin: the
   negative `not.toContain('insert: ""')` is SCOPED to the extracted
   submit() body (re-adding a submit-time clear fails the test), while
   the delivered-clear is asserted inside consumeTerminalPhase.
   randomUUID, constants, compartment wiring, frame arms, and the
   closed/exit ordering are all pinned. promptQueue.test.ts covers the
   store transitions including the replace-after-terminal case.

## Non-blocking observations

- O1: failPendingPrompt overwrites an UNCONSUMED terminal phase: a
  "delivered" that resolved while the bubble was hidden, followed by a
  socket close before reshow, re-labels as failed and keeps the text —
  a visible recoverable duplicate, inside the accepted decision-3
  class (errs on keep-text, never data loss). Optional v2 hardening:
  skip fail when phase is already terminal.
- O2: mount catch-up shows the chip immediately for an in-flight
  pending (deliberate, commented deviation from the 300ms grace, which
  is for the submit-while-open fast path). Reads as intended UX.
- O3: a "queued" pending has no delivery timeout (only "sent" has the
  5s ack guard) — correct per design: queued is server-confirmed and
  delivery can legitimately wait behind a busy agent; socket loss is
  the failure signal there.

## Flags for @@PromptQueue's pending smoke

1. Runtime reactivity (the state_unsafe_mutation class the static
   gates miss): exercise submit -> queued -> delivered live in the
   browser; watch for Svelte 5 effect errors on the lock reconfigure
   and label flips.
2. Hide mid-pending, resolve while hidden, reshow: the mount catch-up
   path (delivered-while-hidden must clear on reshow, not re-show the
   text; failed-while-hidden must show the transient note).
3. Reload mid-pending: draft text restored read-write, depth re-synced
   from the session frame, queued copy still delivers (decision-3
   duplicate visibility).
4. Rejected path at cap (queue full ack) — keep-text + transient note.
5. Multi-window: depth updates in the non-owning window; its composer
   never locks.
6. O1's edge if cheap: deliver while hidden, kill the serve, reshow.

## Status

Holding. Item-1 restructure review still outranks when its sha
arrives; launcher review already delivered (task 17).
