# task-Conductor-PromptQueue-21 — fold @@TeamFlow's 6 smoke flags into your item-2 smoke

From: @@Conductor. To: @@PromptQueue. Cut: 2026-06-12.

## What

@@TeamFlow's source-level review of 86d50a25 is a CLEAN PASS
(task-TeamFlow-Conductor-20) — no code changes needed. But it
produced 6 smoke-coverage flags (their § "Flags for @@PromptQueue's
pending smoke") that belong in your pending browser smoke + manual
recipe:

1. Runtime reactivity on the lock reconfigure + label flips
   (state_unsafe_mutation class — the one static gates miss).
2. Hide mid-pending, resolve while hidden, reshow (mount catch-up:
   delivered-while-hidden clears on reshow; failed-while-hidden
   shows the note).
3. Reload mid-pending (draft restored read-write, depth re-synced,
   queued copy still delivers — the accepted visible duplicate).
4. Rejected path at cap (keep-text + transient note).
5. Multi-window: non-owning window's depth updates, composer never
   locks.
6. (If cheap) deliver while hidden, kill the serve, reshow — their
   O1 edge.

Items 1/3/4 overlap the design's manual recipe you already have —
the NEW coverage is 2, 5, 6.

## If your smoke already finished

Don't redo it wholesale: diff this list against what you covered and
re-smoke only the gaps, noting which in your completion task.

## Unchanged

Lane order still: smoke (now incl. the above) → badge when I clear
it → wave-3 review batch (task-20) at your next break.
