# fullstack-a-65 — Editor bug bundle: right-click selects whole line + image-as-text on tab-switch + New Directory dialog selection

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Three small editor bugs bundled. All from
[`../alex/addendun-a.md`](../alex/addendun-a.md)
"## Bugs":

1. **Right-click on editor selects whole line**
   (shouldn't). When user right-clicks expecting the
   context menu, the click ALSO selects a whole line.
   Should just open the menu without selecting.
2. **Image-as-editable-text after tab switch**.
   Editor → terminal → editor returns showing images
   as raw markdown text; needs cursor click/move to
   render. Probably a re-render gate that's not
   triggered on tab focus restore.
3. **New Directory dialog selects whole path**.
   Should place cursor at end of path (ready to type
   new name), not select-all.

## Reference

[`../alex/addendun-a.md`](../alex/addendun-a.md):

> When we right-click the editor's document expecting
> to see the right-click menu, it also selects a whole
> line.. it shouldnt

> When switching tabs from editor to terminal and
> back to editor, the editor returns showing the
> images as editable text.. need to click or move
> the cursor for the whole doc to render again

> The New Directory dialog comes up with the whole
> path selected; this is not helpful for the user, who
> wants to click New Directory and be able to type
> the name of the directory; we should simply place
> the cursor at the end of the path, ready to type
> the new name

## Acceptance

1. **Right-click no select**: right-click on editor
   document opens menu WITHOUT selecting a line.
2. **Image re-render on tab focus**: tab switch back
   to editor re-runs the markdown render so images
   display correctly without needing a cursor poke.
3. **New Directory dialog cursor at end**: opening
   New Directory dialog places cursor at end of the
   pre-populated path, NOT selection-all.

### Tests

Vitest pins per fix.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  green.

## Coordination

* @@FullStackA. SPA-only.
* Bundled because all three are small + same lane
  + tightly-coupled to editor / dialog surfaces.

## Authorization

Yes for editor + dialog SPA files + tests + task tail
+ outbound.

## Numbering

This is `-a-65`.

## 2026-05-22 — ready for review

Six-file change. SPA-only; no Rust touched.

### Bug 1 — Right-click selects whole line

**Root cause**: CodeMirror's default mousedown
handler runs on `button === 2` (right-click)
and interprets it as a selection gesture
(triple-click-equivalent on certain selection
states). The outer `oncontextmenu` handler in
`FileEditorTab.svelte` only fires AFTER the
mousedown selection has settled.

**Fix**: new
`web/src/editor/right_click_no_select.ts`
extension exporting `rightClickNoSelect()`.
Returns `true` from the mousedown handler when
`e.button === 2`, telling CodeMirror "this
event is handled, don't run your default
selection logic." The outer
`oncontextmenu={onEditorContext}` still fires
and pops the tab menu — the contextmenu event
is separate from mousedown.

Wired into Wysiwyg + Source extension lists.

### Bug 2 — Image-as-text after tab switch

**Root cause**: Image decorations in
`imageDecorations` skip rendering when the
editor's viewport measurement is stale. On
tab-switch back to an editor tab (via chord or
mouse), the editor view's viewport calc may
not refresh until the user pokes the cursor —
so images remain rendered as raw markdown
text.

**Fix**: `view.requestMeasure()` added to
three sites:

* Wysiwyg `focus()` export (called by
  `-a-64`'s tabFocusPulse machinery on chord
  switch).
* Source `focus()` export (parity).
* Wysiwyg `onMount` after `view = new
  EditorView(...)` — covers fresh mounts
  where the host element is mid-animation /
  zero-size.

`view.requestMeasure()` schedules a measure
cycle that re-runs decorations against the
current viewport.

### Bug 3 — New Directory dialog selects whole path

**Root cause**:
`web/src/components/PathPromptModal.svelte`'s
focus block at the modal-open effect calls
`inputEl?.select()` (select-all) in the else
branch. For folder-create the pre-populated
defaultValue is the parent path; selecting it
forces the user to delete-all before typing.

**Fix**: new branch in the dialog-open
$effect for `kind === "folder" && mode ===
"create"` → places cursor at end via
`setSelectionRange(end, end)`. Other modes
(rename / move / file-non-default-name) keep
the prior select-all behaviour.

### Acceptance

1. Right-click on editor opens menu WITHOUT
   selecting a line ✓ (extension active in
   both Wysiwyg + Source).
2. Tab-switch back to editor re-renders
   images ✓ (requestMeasure on
   focus() + onMount).
3. New Directory dialog opens with cursor at
   end ✓ (folder+create branch).

### Tests

`editorBugBundle.test.ts` (new): 9 raw-source
pins (3 per bug — rightClickNoSelect
extension shape + wiring; requestMeasure on
focus() + onMount; PathPromptModal cursor-at-
end branch + preserved fullstack-a-15 case +
default select-all branch).

### Gate

* vitest **784 / 784** (+9 net from
  `-a-64`'s 775).
* svelte-check 0 errors / 0 warnings across
  4005 files.
* npm build clean.
* Rust gate not re-run.

### Decisions

* **Extension over per-component handler** for
  bug 1 — reusable, lives alongside other CM6
  domEventHandlers in `editor/`, doesn't
  pollute the Svelte component.
* **requestMeasure() in both focus() AND
  onMount** for bug 2 — `focus()` covers the
  chord-driven path (`-a-64`'s pulse);
  `onMount` covers fresh mounts where the
  parent pane is animating in.
* **folder+create branch only** for bug 3 —
  the existing file+create branch
  (fullstack-a-15) had a different intent
  (replace the default `Untitled` stem); the
  default else-branch (select-all) stays for
  rename/move/attach modes where typing
  should replace the whole input.

### Suggested commit subject

```
Editor bugs: right-click no select + image re-render on tab switch + new-dir cursor at end (fullstack-a-65)
```

Single commit. Three small fixes bundled per
task scope.

### Files for `git add` (per-path discipline)

* `web/src/editor/right_click_no_select.ts` (new)
* `web/src/editor/Wysiwyg.svelte`
* `web/src/editor/Source.svelte`
* `web/src/components/PathPromptModal.svelte`
* `web/src/components/editorBugBundle.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-65.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
