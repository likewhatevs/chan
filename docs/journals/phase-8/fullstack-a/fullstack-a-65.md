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
