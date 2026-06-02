# Image-viewer prev/next: design brief (@@LaneD)

Design-first brief for @@Host's image-viewer navigation (wave-3). For
@@Lead/@@Host review before implementing. A few lines, as asked.

## What exists

`state/imageZoom.ts` `openImageZoom(src, fromPath)` is a self-contained
fullscreen overlay (backdrop + img, click/Esc to dismiss). Callers:
editor image click (Wysiwyg), file-browser "View / Zoom" (FileInfoBody),
inspector image refs (FileInfoBody). `tree.entries` + `isImage()`
(state/fileTypes.ts) give the file set; the editor doc's images come off
the syntax tree's Image nodes in document order.

## Set definition (the decisions)

1. **Dir scope (file browser):** FLAT current directory only, not
   recursive. The set = the images that sit in the SAME directory as the
   viewed image. Recommend flat (matches "same directory").
2. **Sort order:**
   - editor set = DOCUMENT order (image atoms top-to-bottom).
   - dir set = the file browser's visible order (the same sortTreeEntries
     order the tree shows), so prev/next matches what the user sees.
3. **Wrap-around at the ends:** recommend YES (next on the last image
   wraps to the first, prev on the first wraps to the last) -- standard
   viewer behaviour, and it avoids a dead-end button. (Alternative: stop
   + disable the button at each end.)
4. A set of ONE image (the only image in the doc/dir) shows no prev/next
   (single-image viewer, unchanged).

## API shape

```
export interface ZoomImage { src: string; fromPath: string | null }

export function openImageZoom(
  src: string,
  fromPath?: string | null,
  set?: ZoomImage[],   // ordered set INCLUDING the opened image
): void
```

- `set` optional + additive: existing single-src callers are unchanged
  (no prev/next). When provided, the viewer finds the opened image's
  index in `set` and renders prev/next + ArrowLeft/Right nav; each
  ZoomImage resolves via resolveImageSrc(src, fromPath) lazily.
- Editor passes `set` = the doc's images, each `{src, fromPath: docPath}`.
- File browser passes `set` = the dir's image entries, each
  `{src: path, fromPath: null}` (workspace-rooted), in tree order.

## UI

Prev (left edge) + next (right edge) chevron buttons over the backdrop +
ArrowLeft/ArrowRight keys; Esc/backdrop-click still dismiss. Buttons
hidden when the set has <= 1 image.

## Files (all @@LaneD)

`state/imageZoom.ts` (the set/index + prev/next UI + keys); the editor
side (Wysiwyg image-click collects the doc's Image-node srcs); the
file-browser side (FileInfoBody "View" collects the dir's image entries).
Browser-smoke BOTH entry points.
