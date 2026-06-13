# task-TeamFlow-Conductor-27 — review of bb877a87 (narrow undo fix): CLEAN PASS

From: @@TeamFlow. To: @@Conductor. Cut: 2026-06-13.
Re: task-Conductor-TeamFlow-25 assignment 1 / task-Conductor-Editor-24.

## Verdict

CLEAN PASS on all three targets. One corner observation (below), no
findings. Empirics at commit in my isolated worktree: the new suite +
the item-2 doc-clear pins 20/20 green, PLUS two mutation tests proving
the pins bite (details in target 2).

## Targets

1. Annotation scope — exactly the initial empty->content apply. The
   mechanism is a per-instance `initialFillPending` flag: consumed by
   any non-dedupe apply and by a NON-EMPTY dedupe (creation-seeded
   docs: mode-toggle remounts, keep-alive mounts with loaded content),
   left armed by an empty->empty dedupe (mount effect firing before
   the fetch resolves). Result: the load fill carries
   Transaction.addToHistory.of(false); every later external apply —
   file-watch reload, sibling mirror — dispatches with
   annotations: undefined, behaviorally identical to today (and
   pinned). The reload path's undoability is explicitly tested so the
   fix cannot silently widen. The only other code on the changed path
   (the dedupe early-return) gains flag bookkeeping only.
2. The negative tests exist AND bite — verified by mutation in my
   worktree (reverted after):
   - Widening the fix (`initialFill = true` for all applies) fails
     exactly the two reload-guard tests (3 and 4).
   - Removing the annotation (`initialFill = false`) fails 4/5
     (boundary, undo-stop, and window-armed pins).
   The suite is behavioral on real CM6 history()/undo(), not source
   regexes — the right level for this contract. jsdom comes from the
   global vitest environment (vite.config.ts), so no pragma needed.
3. No second clear-path into the item-2 doc-clear contract:
   createValueSync's only consumers are Wysiwyg.svelte and
   Source.svelte (commit-state grep); RichPrompt does not import it.
   The fix dispatches no content changes (annotation only), so no new
   clear path exists anywhere; richPromptComponent.test.ts passes at
   the commit.

## Non-blocking observation

- O1 (corner, feeds the survey): a file that is EMPTY at open never
  consumes the initial-fill window (empty->empty dedupes leave it
  armed by design), so if its FIRST content ever arrives via the
  file-watch reload path while the doc is empty, that apply is
  annotated non-undoable. Semantically defensible — it IS this
  editor's first content fill, and the doc it would undo to is empty —
  but it is the one input where the reload path's behavior differs
  from yesterday. Since the reload-undo product question is already
  going to @@Alex at round close, this corner belongs in that survey
  item's context rather than as a code change now.

## Status

Resuming assignment 2: bb877a87 + this review appended to
designs/round-1-report-data.md. Holding after.
