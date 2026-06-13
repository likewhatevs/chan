# task-Conductor-Editor-24 — items 4+1 + review ACCEPTED; undo-past-load narrow fix AUTHORIZED

From: @@Conductor. To: @@Editor. Cut: 2026-06-13.
Re: task-Editor-Conductor-23.

## Acceptance

Items 4+1 ACCEPTED (ffbcc3ff, dadd5e64) — the Chrome evidence is
thorough (scrollTop 3112→3112, undo survival, 218-decoration
instant paint, the honest untestable-in-Chrome list with the
stash-and-reprobe control for restore focus). Your TeamFlow x3
review: CLEAN accepted; the test-title nit goes on the follow-ups
list (retitle at next touch, no task). WKWebView-pending items 1-6
are folded into the consolidated round-close checklist (item-1/4
walks + session restore + DnD + drop allowlist + memory sanity).
@@TeamFlow's source review of dadd5e64 is in flight; findings (if
any) come to you as tasks.

## Undo-past-load wipe — DECISION + AUTHORIZATION

Your escalation was right (data-loss class, shared infra, product
question attached). Decision, split:

1. AUTHORIZED NOW, your lane: the NARROW fix — the INITIAL
   empty→content applyExternal in web/src/editor/base.ts
  (createValueSync) gets addToHistory(false) (or the equivalent
   non-undoable annotation). Undoing to the empty pre-load doc is
   never a wanted state, and autosave wiping the file on it is
   plainly a bug. Item 1 widened the window; the round bar is "no
   known bug ships" — this closes it.
   - Scope: initial-load apply ONLY. Do NOT annotate the
     file-watch-reload path (see 2).
   - Test: vitest pin that the initial apply is non-undoable (and
     that a normal user edit after load IS undoable — don't
     over-annotate).
   - Verify: your own repro from the smoke (undo-spam right after
     open → doc must stop at the loaded content; autosave never
     sees empty), Chrome is sufficient for this one (engine-
     independent CM6 transaction semantics).
   - Gate: make web-check re-run after final edit; pathspec-atomic
     commit, base.ts + test only.
   - Cross-review: I'll route to @@TeamFlow (smallest diff, related
     surface, they're already deep in dadd5e64).
2. DEFERRED to the round-close survey (@@Alex): whether
   undo-after-file-watch-reload should also be non-undoable, or is
   a wanted "recover from unwanted external overwrite" path. Both
   positions are defensible — that's a product call, not ours.
   Your fix must leave that path's behavior unchanged either way.

## Cleanup

/tmp/editor-lane-ws: delete at your convenience (round teardown at
the latest). Noted your HMR-of-peer-WIP observation for the retro —
real pattern, worth a process line next round.

## After this

The narrow fix is your last in-round item (B2 stays unstarted —
confirmed). Then hold for any dadd5e64 review findings + the
round-close WKWebView walk (you offered to drive the checklist —
accepted, I'll coordinate you + @@Desktop when the tree settles).
