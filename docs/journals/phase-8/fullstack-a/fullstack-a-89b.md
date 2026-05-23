# fullstack-a-89b — Rich prompt cursor / placeholder visible Y-misalignment (3rd-round follow-up; empirical-first)

Owner: @@FullStackA
Cut: 2026-05-23 by @@Architect
Status: dispatched
Priority: HIGH (3rd round on user-flagged UX bug)

## Goal

@@Alex 2026-05-23 (exact spec — literal):

```
{cursor}{space}{default-text}
```

* Cursor on the left.
* Single space after cursor.
* Default placeholder text immediately after the
  space.
* All on the SAME ROW.

Behavior:
* If user types → space + default text disappear in
  favor of what they typed.
* If user deletes the input whole → default text
  appears again.

This is canonical CM6 placeholder behavior +
should be a 1-3 line CSS fix to undo whatever's
pushing the placeholder to a separate row.

## Reference

@@Alex 2026-05-23: "im still very dissatisfied with
this cursor for the rich prompt that we cannot get
right" — 3rd round of feedback.

Screenshot shows the cursor `|` sitting visibly
HIGHER than the placeholder text baseline. The
cursor's top edge is ABOVE the placeholder text;
the cursor's bottom is at roughly placeholder
baseline.

### Saga history (all shipped, none closed visually)

* `-a-84` (3869a07): CSS overlay X-offset 10px.
* `-a-87` (0bcfbe7): CSS overlay line-height
  1.5 → 1.8 to match cm-line.
* `-a-89` (5845fa0): switched from CSS overlay
  to CM6 `placeholder` extension. @@WebtestA
  walked HOLD because cursor + placeholder
  matched at x-axis BUT y-axis still showed a
  ~12px gap (per their reported coords:
  cursor y=464.5, placeholder y=476).

## Empirical-first directive

Per the architect-side lesson from the Drafts saga
(EMPIRICAL > code-review when feature doesn't
visibly work):

### Mandatory diagnostic steps

1. **Open chan in a browser**. Build fresh
   binary first (pkill + cargo build + npm build
   + restart per `feedback_fresh_binary_rewalks`).
2. **Open browser devtools**. Inspect the empty
   rich prompt.
3. **Capture exact pixel positions** for:
   * `.cm-cursor` (or `.cm-cursor-primary`):
     top, left, width, height.
   * `.cm-placeholder`: top, left, width, height,
     line-height, baseline.
   * `.cm-line` (the line containing both):
     top, height, padding-top, padding-bottom,
     line-height.
4. **Identify the exact pixel delta** between
   cursor visual top edge and placeholder text
   visual top edge.
5. **Identify the root cause** from the pixel
   measurements. Candidates:
   * cm-line padding pushing the placeholder text
     below the cursor's top edge.
   * Placeholder font-size + line-height creates
     a smaller box than cm-line's natural height.
   * CM6's default cursor styling extends beyond
     the typeable area.
   * CSS reset / inherited line-height conflict.
6. **Fix the actual cause** identified empirically.

### Fix shape (TBD per audit)

Likely one of:
* `.cm-cursor { height: 1em; vertical-align:
  baseline }` to shrink cursor to text height.
* `.cm-line { padding-top: 0 }` to remove top
  padding causing the offset.
* `.cm-placeholder { line-height: <match cm-line> }`
  with explicit pixel value matching cursor height.
* A wrapping element style change.

## Acceptance

1. **Cursor + placeholder share visible Y-axis
   baseline** — measured empirically in browser
   devtools post-fix. Cursor top edge aligns
   (within 1px) with placeholder text top edge,
   OR cursor baseline aligns with placeholder
   text baseline.
2. **Visually**: cursor + placeholder text look
   like ONE unit — the cursor blinks at the
   exact starting position of the placeholder
   text, as if the placeholder were already-
   typed content.
3. **Screenshot confirmation** required: take a
   screenshot in browser at 1x DPI and verify
   visually before declaring HOLD.
4. **No regression** on hide-on-type / re-show
   on full delete.

### Tests

Vitest pin on whatever CSS landed + a brief
empirical-walk note in the task tail with the
pixel measurements before + after.

### Gate

`npm test` / `check` / `build` green + browser
visual verification.

## Coordination

* @@FullStackA. SPA-only.
* 3rd round on this UX bug — go DEEP empirically
  before committing the fix. Don't trust audit-
  grep alone.
* @@WebtestA can pair on the empirical
  measurement step if useful.

## Authorization

Yes for `web/src/components/TerminalRichPrompt.svelte`
+ `web/src/editor/*.svelte` if needed + CSS module
+ tests + task tail + outbound.

## Numbering

This is `-a-89b` (filename for clarity; conceptual
slice 2 of `-a-89`).

## Out of scope

* Re-architecting away from CM6 placeholder extension
  (`-a-89` is correct; we just need to close the
  visual gap).
* Re-styling beyond the alignment fix.

## Supersedes

* Nothing — additive on `-a-89`.

## Reverse: what NOT to do this round

* DON'T guess at line-height / top / padding
  values without measuring first.
* DON'T ship without a browser screenshot
  verifying the visual.
* DON'T claim HOLD until the visual matches
  the "cursor at the start of text" intent.
