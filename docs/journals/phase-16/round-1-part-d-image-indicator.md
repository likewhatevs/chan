# Image-drag source-row indicator: design (@@LaneD)

Light design-first proposal for @@Host's image-drag follow-up (wave-3).
For @@Lead/@@Host review before implementing.

## What exists

Dragging a rendered image atom repositions its `![](src)` to a new
source row:

- `widgets/image.ts` dragstart stamps `IMAGE_MOVE_MIME` with the source
  range `{from,to}` and sets `data-dragging` (the source image dims to
  0.4 while in flight); dragend clears it.
- `bubbles/image_drop.ts` `dragover` opts the editor in (preventDefault +
  dropEffect=move) so the drop fires; `drop` runs `moveImageSource`,
  which lands the markdown at `doc.lineAt(dropPos).from` (whole row when
  the image shares a line with text).
- The target source line is computable from the drag Y at any moment:
  `view.posAtCoords({x,y})` -> `doc.lineAt(pos)` -> line number + text.

There is NO live indicator today; the user only sees the result on drop.

## The ask

During the vertical drag, show LIVE which SOURCE ROW the image will land
on, tracking the pointer, so the user knows the landing line before they
release.

## Indicator options

- **A (recommended): drop-line + line badge.** A thin accent rule across
  the editor at the TOP of the target line (the exact insert point),
  plus a small badge near the pointer reading `line N` and the target
  line's text (truncated, ~40 chars). Shows BOTH the precise insert
  point and which row, by number and content. Closest to "know exactly
  which line it lands on".
- **B: target-line highlight.** The whole target line gets an accent
  background band that tracks the pointer. Simplest; shows the row but
  not the insert point, line number, or text.
- **C: drop-line only.** Just the accent rule, no badge. Shows the
  insert point but not the line identity.

Recommendation: **A**. One open sub-call for @@Host: badge content -
`line N` only, or `line N` + the line's text snippet? I lean on the
snippet (a row is easier to recognize by its text than a bare number).

## Behavior

- Tracks the pointer: updates on every `dragover` (cheap; only redraws
  when the target line changes).
- No-op target: dropping on the image's own row is already a no-op in
  `moveImageSource`; the indicator hides (or reads "stays here") there
  so it never implies a move that won't happen.
- Clears on drop / dragend / dragleave out of the editor.
- `pointer-events: none` so it never intercepts the drag.
- WYSIWYG only (image atoms only render there); source mode is untouched.

## Implementation sketch (after approval)

- A CM6 `StateField` holds the target line (`from` offset or null), set/
  cleared by `StateEffect`s.
- `image_drop.ts` `dragover` dispatches the set effect (computed line);
  `drop` + dragleave dispatch clear; `image.ts` dragend dispatches clear.
- Drop-line = a line `Decoration` with a top border on the target line
  (or a `layer()` overlay); badge = a small `pointer-events:none` DOM
  element following the cursor (mirrors the existing editor bubbles),
  text = `line N - <snippet>`.
- Files (all @@LaneD): `Wysiwyg.svelte` (wire the field/extension +
  badge), `bubbles/image_drop.ts` (dragover set / drop clear),
  `widgets/image.ts` (dragend clear).

## Verification

Unlike the cross-window tab DnD, this is a single-window internal HTML5
drag of a rendered atom, so it IS browser-smokeable in Chrome: drag an
image up/down and watch the drop-line + badge track the pointer and name
the target row (Svelte/CM6 reactivity static gates miss).
