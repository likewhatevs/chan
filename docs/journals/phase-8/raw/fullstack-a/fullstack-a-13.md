# fullstack-a-13: Editor image-insert snaps viewport to top + subsequent typing does not roll the view

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Inserting an image at (or near) the end of a long markdown
file must not throw the editor viewport to the top of the
document with the cursor parked ~3.2k px off-screen. The
cursor stays visible (or the view scrolls to keep it
visible), and subsequent typing rolls the view to follow the
cursor.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) editor
cluster: "image insert + EOL scroll rollover" originally
landed as part of `fullstack-a-5`. @@WebtestA's lane-A
sweep on 2026-05-20 found it still reproduces and is in
fact worse than the original report:

> Insert an image at the end of a long markdown doc (e.g.,
> README.md). The viewport snaps to the top of the document
> and the cursor is parked ~3.2k px below the visible
> window. Typing more characters advances the cursor offset
> (`c`) per character — the text is being written into the
> document — but the view does NOT scroll to follow the
> cursor. User has no way to see what they're typing without
> manually scrolling.

Repro on lane-A server (`/tmp/chan-test-phase8-wa/`,
README.md): Cmd+End → scroll bottom → type
`![](./test-image.png)` → observe viewport jump.

## Acceptance criteria

* Inserting an image at the end of a long doc keeps the
  cursor in view (or auto-scrolls to it within one
  paint frame).
* Typing additional characters after the image insert rolls
  the viewport along with the cursor (the standard
  "cursor follows typing" behaviour).
* No regression on inserting an image in the middle of a
  doc — the cursor remains where the user expects (just
  past the inserted `![](…)`).
* No regression on the EOL scroll rollover handling that
  was the intent of the earlier `fullstack-a-5` editor
  fix; if that earlier fix is the source of the new
  regression, amend it in place rather than reverting.

## How to start

1. Reproduce on the lane-A test server first (URL in
   `event-architect-alex.md` 2026-05-20). Open README.md,
   Cmd+End, insert the image markdown, watch the viewport.
2. The image-insert path is likely in
   `web/src/components/Editor*` or a CodeMirror /
   contenteditable shim. Find where the insert mutation
   commits + where the viewport scroll position is
   recomputed.
3. Likely root cause families:
   * The image-load reflow recomputes a layout height
     before the cursor's scroll-into-view fires, throwing
     the viewport to a stale offset.
   * The `scrollIntoView` call is gated on a stale state
     that the image insert path doesn't update.
   * A focus drop on image insertion lets the next typing
     event miss the auto-scroll path.
4. Pin with whatever test scaffolding the editor surface
   has. If no SPA-level test fits, visual verification on
   the lane-A server is acceptable.

## Coordination

* @@WebtestA verifies on lane-A drive once landed.

## 2026-05-20 — implementation note

Root cause: inline atom widgets (the image case being the worst)
have unknown rendered height between widget mount and the resource
finishing its byte load. The flow:

1. User types `![](./test-image.png)` at end of doc.
2. On the closing `)`, the markdown parser produces an `Image`
   node and the ViewPlugin in `web/src/editor/widgets/image.ts`
   atom-replaces the source text with an `<img>` widget. Width
   is fixed (250px from `#w=250` or the default), height is
   `auto`, src is the resolved `/api/files/...` URL.
3. CM6 finishes the transaction with the caret at end of doc
   and tracks it into view via the normal user-input scroll
   path. So far so good — the line containing the new widget
   measures ~22px (one text line height) because the img has
   no decoded bytes yet.
4. The browser fetches and decodes the image asynchronously.
   For a tall asset (the seeded `test-image.png` is ~2200px
   natural height clamped only on width), the line grows by
   ~2200px on the load reflow.
5. CM6 does NOT re-anchor the scroll on async layout shifts —
   its caret-tracking only runs on transactions. Nothing
   dispatches a transaction in response to the load. So
   scrollTop stays at its old "near the doc end" value while
   the caret's now-real document-y is ~2200px deeper. The
   caret ends up far below the viewport.

Verified with a programmatic repro on the lane-A test server
(via the `cmTile.view` handle on the `.cm-content` element):
seeding caret-to-end → typing the 21-char insert via dispatch
landed scrollTop near doc-end. The settle frame ~600ms later
showed scrollHeight jumped from 4446 to 6625 (the 2179px load
reflow) while scrollTop stayed put — caret 85px-to-1900px+
below the viewport, exact distance scaling with the image's
natural height. Matches @@WebtestA's "~3.2k px below" report.

Fix: `web/src/editor/widgets/image.ts` ImageWidget.toDOM adds
an `img.addEventListener("load", ...)` (one-shot per img
element) that, when the image's source line is the same as
or adjacent to the caret line AND the caret is off-screen,
dispatches `EditorView.scrollIntoView(head, { y: "nearest" })`
to re-anchor. Listener installs in the success-load branch
only (not for the broken-image path that swaps to a fixed-size
badge); the `wrap.isConnected` guard skips fires on detached
widgets if the decoration replaced the widget before load
completed. The line-proximity gate keeps the listener from
fighting a user's deliberate scroll past a distant image that
just streamed into view — only loads near the active caret
re-anchor.

Why no test scaffold: jsdom doesn't lay out (no real img.load
fires with realistic heights, no scroller.scrollTop math), so
a vitest pin would only assert the listener exists, not that
it does the right thing. Visual verification on the lane-A
server is the practical bar; the task spec explicitly allows
this for the editor surface.

Files touched:

* `web/src/editor/widgets/image.ts` — load listener on the
  success-load img path, conditional `scrollIntoView` dispatch.

Pre-push gate (SPA portion): vitest 475/475 green;
`npm run check` 0 errors / 0 warnings; `npm run build` clean;
`cargo build -p chan` re-embeds the new bundle clean.

To verify on the lane-A server: restart the server so the
binary picks up the rebuilt bundle, open a long file like
README.md, Cmd+End to reach the end, type
`![](./test-image.png)` (or paste an image at end-of-doc),
observe that the caret stays in view after the image
finishes loading. The pre-existing `scrollIntoView: true` from
`fullstack-a-5` on the paste/drop dispatch handles the
insert-time tracking; the new load handler handles the
post-decode reflow. Both cooperate.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Outstanding work. The root cause is exactly right: CM6's
caret-tracking is transaction-scoped, so any async layout
shift (image decode, font swap, lazy-loaded iframe, etc.)
that grows a line below the caret leaves scrollTop stale.
The image case is just the worst-felt instance because tall
assets and slow networks compound it. The 4446 → 6625
scrollHeight measurement matching @@WebtestA's "~3.2k px"
report is the clean evidence of the root cause.

Fix is in the right layer (`web/src/editor/widgets/image.ts`
ImageWidget.toDOM) — image-decode is where the layout shift
originates, so reacting there is local and doesn't require
threading scroll knowledge through generic editor plumbing.
The three guards (success-load only / `wrap.isConnected` /
line-proximity gate against the caret) are all defensible:

* Success-load only: avoids fighting the broken-image badge
  swap (which is a fixed-size widget, no reflow).
* `wrap.isConnected`: skips firing on widgets the decoration
  replaced before load completed — clean lifecycle hygiene.
* Line-proximity gate: keeps the listener from re-anchoring
  on distant images that just happened to stream in, which
  would feel like the editor stealing the user's scroll.
  The "scroll only when near caret AND caret off-screen"
  predicate is the right composition.

The jsdom-can't-test-this acknowledgement is honest and the
right call — a test that asserts the listener exists adds
no coverage. Lane-A visual verification is the meaningful
gate; @@WebtestA picks up the re-verify post-commit.

Pre-push gate green across the full stack (vitest + check +
build + cargo re-embed). Clean.

**Commit clearance**: approved. Suggested commit subject:

```
Editor: re-anchor scroll on image decode reflow near caret (fullstack-a-13)
```

Push waits for Round-1 close.

Carry on with `fullstack-a-12` (graph inspector second-
ghost) next per the queue. `systacean-2` is now at
`4a04917` so the binary rebuild has the resolver fix in
HEAD — your verification leg can use the lane-A server
after a server restart picks up the rebuild.