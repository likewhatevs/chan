// Image atom widget for `![alt](src)` markdown.
//
// Per design.md spec #5, images are atomic widgets. Source revealed
// when selection intersects. EditorView.atomicRanges so caret motion
// skips the image in one keystroke.
//
// Behaviors:
//   - Renders `<img>` with `src` resolved against the editing file's
//     path (workspace-relative paths route through /api/files/{...} with
//     the auth token).
//   - Width fragment `#w=N` and alignment `#left` / `#right` parsed
//     from the src and applied as inline style + dataset.
//   - Bottom-right drag-resize handle visible on hover. Mousedown
//     starts a window-level mousemove/mouseup loop; the handle
//     mutates `img.style.width` live during drag. On mouseup the
//     final width commits to the source via `setImageWidth` +
//     `view.dispatch` - the widget then re-mounts at the persisted
//     width (no visible flicker; the inline style is identical).
//   - Click on the img (not the handle) fires `onImageClick(
//     { src, alt, pos }`) so step 8's image-action overlay can mount
//     the zoom + edit pills. Click handler is on the img; the
//     handle's mousedown stops propagation so it doesn't double-fire.
//
// v1 scope: no per-paste upload here (paste/drop flow lives in the
// image bubble + step 7). No alignment toggle from the widget itself
// (the bubble owns alignment edits).

import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
  WidgetType,
} from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import { type Extension, StateField } from "@codemirror/state";
import {
  parseImageSrc,
  resolveImageSrc,
  setImageWidth,
} from "../extensions/image";
import {
  clearImageDragIndicator,
  startImageDragIndicator,
} from "../image_drag_indicator";

const MIN_IMG_WIDTH = 40;
const USER_SCROLL_QUIET_MS = 900;

/// Custom dataTransfer MIME for the image drag-to-move gesture. The
/// editor-level `drop` handler (image_drop.ts) keys off this type to
/// tell an internal image move apart from an OS image-file drop. The
/// payload is the JSON `{ from, to }` source range of the dragged
/// `![alt](src)` node, captured fresh from the syntax tree at
/// dragstart.
export const IMAGE_MOVE_MIME = "application/x-chan-image-move";

/// Start an internal image-move drag from an image atom widget. Reads
/// the live Image node range (positions drift as the doc changes, so
/// we resolve from the stamped nodePos rather than trusting a cached
/// range) and stashes it on the dataTransfer so the drop handler can
/// relocate the source. Sets a `data-dragging` marker for styling.
function beginImageDrag(
  e: DragEvent,
  view: EditorView,
  nodePos: number,
  wrap: HTMLElement,
): void {
  const range = imageNodeRange(view, nodePos);
  if (!range || !e.dataTransfer) {
    // No resolvable source range -> let the browser do its default
    // (which for an <img> with a real src is a normal image drag); we
    // simply don't tag it as an internal move.
    return;
  }
  e.dataTransfer.setData(
    IMAGE_MOVE_MIME,
    JSON.stringify({ from: range.from, to: range.to }),
  );
  // `move` so the cursor shows the move affordance, not copy.
  e.dataTransfer.effectAllowed = "move";
  // Drag the image as the drag image so the user sees what they grab.
  const img = wrap.querySelector("img");
  if (img instanceof HTMLImageElement && img.complete) {
    e.dataTransfer.setDragImage(img, img.width / 2, img.height / 2);
  }
  wrap.dataset.dragging = "true";
  // Arm the live source-row indicator (drop-line + line badge) for the
  // duration of this move; dragover refreshes it, dragend clears it.
  startImageDragIndicator(view, range);
}
const scrollIntentUntil = new WeakMap<HTMLElement, number>();
const scrollIntentInstalled = new WeakSet<HTMLElement>();

function installUserScrollIntentTracker(scrollDOM: HTMLElement): void {
  if (scrollIntentInstalled.has(scrollDOM)) return;
  scrollIntentInstalled.add(scrollDOM);
  const mark = () => {
    scrollIntentUntil.set(scrollDOM, Date.now() + USER_SCROLL_QUIET_MS);
  };
  scrollDOM.addEventListener("wheel", mark, { passive: true });
  scrollDOM.addEventListener("touchmove", mark, { passive: true });
  scrollDOM.addEventListener("keydown", (event) => {
    if (
      event.key === "PageUp" ||
      event.key === "PageDown" ||
      event.key === "Home" ||
      event.key === "End" ||
      event.key === "ArrowUp" ||
      event.key === "ArrowDown"
    ) {
      mark();
    }
  });
}

function userScrollIntentActive(scrollDOM: HTMLElement): boolean {
  return Date.now() < (scrollIntentUntil.get(scrollDOM) ?? 0);
}

/// Strict-interior selection test for image edit-mode entry.
/// `selectionInRange` (the inline-mark helper) treats caret AT a
/// boundary as "intersecting". That rule makes sense for bold /
/// italic markers - touching the `*` reveals it - but for images
/// the atomic widget replaces the entire source span, so the only
/// position the caret can EVER hold via clicks is one of the two
/// outer boundaries (atomicRanges snaps the click). Treating those
/// as "editing" means any click on the image's source line flips
/// the widget into edit mode, which is what the user sees as a
/// stray click landing in the source. Arrow-key entry and the
/// Edit button both land the caret STRICTLY inside (the URL slot's
/// urlFrom / urlFrom+1), so a strict interior test still catches
/// the intentional entry paths.
function imageEditEntered(
  sel: import("@codemirror/state").EditorSelection,
  from: number,
  to: number,
): boolean {
  for (const r of sel.ranges) {
    if (r.empty) {
      if (r.head > from && r.head < to) return true;
    } else {
      if (r.from < to && r.to > from) return true;
    }
  }
  return false;
}

/// Lucide Copy + Check icons inlined as SVG strings - the widget is
/// raw DOM, not Svelte, so we can't reuse lucide-svelte components.
/// Compact 12px icons with stroke weights tuned for the image
/// widget's small action row.
const COPY_ICON_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>';
const CHECK_ICON_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>';

/// Copy the image at `src` to the clipboard at its natural pixel
/// dimensions. Fragment hints (`#w=`, `#h=`, `#left`, `#right`) are
/// stripped before fetching so we always pull the original asset,
/// not a resized variant. The blob is re-encoded to PNG via canvas
/// because PNG is the only image MIME most browsers accept on the
/// async Clipboard API, and the canvas pass naturally captures the
/// image at its natural width/height.
async function copyImageToClipboard(
  src: string,
  fromPath: string | null,
): Promise<void> {
  const cleanSrc = src.split("#")[0]!;
  const url = resolveImageSrc(cleanSrc, fromPath);
  if (!url) throw new Error("cannot resolve image url");
  const resp = await fetch(url);
  if (!resp.ok) throw new Error(`fetch ${resp.status}`);
  const blob = await resp.blob();
  const bitmap = await createImageBitmap(blob);
  const canvas = document.createElement("canvas");
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("no 2d context");
  ctx.drawImage(bitmap, 0, 0);
  bitmap.close?.();
  const pngBlob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob(
      (b) => (b ? resolve(b) : reject(new Error("toBlob failed"))),
      "image/png",
    );
  });
  await navigator.clipboard.write([
    new ClipboardItem({ "image/png": pngBlob }),
  ]);
}

/// Line decoration applied to the line AFTER an inline-floated image.
/// CSS clears the float there so only the SAME line as the image
/// flows beside it; the next paragraph drops below the image. Reused
/// across all matching images per pass.
const CLEAR_AFTER_IMAGE = Decoration.line({
  attributes: { class: "cm-md-image-clear-after" },
});

export interface ImageClickArgs {
  src: string;
  alt: string;
  /// Position of the Image node's start in the source - useful for
  /// the action overlay (step 8) to anchor itself or trigger an
  /// edit-bubble open at the right offset.
  pos: number;
}

interface ImageActionPayload {
  src: string;
  alt: string;
  fromPath: string | null;
  nodePos: number;
  onClick: ((args: ImageClickArgs) => void) | undefined;
}

export interface ImageOptions {
  /// Read the editing file's workspace-rooted path. Used to resolve
  /// relative img sources against the right directory. `null` keeps
  /// sources workspace-rooted (no relativization).
  getCurrentPath: () => string | null;
  /// Optional click handler for the image action overlay (step 8).
  onImageClick?: (args: ImageClickArgs) => void;
}

class ImageWidget extends WidgetType {
  constructor(
    readonly alt: string,
    readonly src: string,
    readonly fromPath: string | null,
    readonly nodePos: number,
    /// True when the image is the only content on its source line
    /// (no surrounding text). Standalone images take the block-level
    /// layout - alignment moves the image LEFT / CENTER / RIGHT
    /// within its own line via flex justify-content. Inline images
    /// (mixed with paragraph text) keep the existing float layout
    /// so text wraps around them.
    readonly standalone: boolean,
    /// True when the widget is rendered AS A BLOCK PREVIEW above an
    /// editable source line (caret is inside the image's source).
    /// In edit mode the float-around-text layout doesn't apply -     /// the preview is a sibling to the source row, not a replacement.
    readonly editing: boolean,
    /// True when the editor's `EditorView.editable` facet is on at
    /// scan time. Captured here (rather than read live inside toDOM)
    /// and folded into `eq()` so a read-only -> writable flip forces a
    /// re-render. Without this the drag-to-move affordance (set in
    /// toDOM) sticks at whatever the facet was when the widget first
    /// rendered: the value prop arrives async, so the first render can
    /// land before editability settles, and `eq()` would then keep the
    /// stale non-draggable DOM forever. Named `writable` (not
    /// `editable`) because `WidgetType` exposes a getter-only
    /// `editable` member - assigning to a field of that name throws.
    readonly writable: boolean,
    readonly onClick: ((args: ImageClickArgs) => void) | undefined,
  ) {
    super();
  }

  eq(other: ImageWidget): boolean {
    return (
      this.alt === other.alt &&
      this.src === other.src &&
      this.fromPath === other.fromPath &&
      this.standalone === other.standalone &&
      this.editing === other.editing &&
      this.writable === other.writable
    );
  }

  toDOM(view: EditorView): HTMLElement {
    ensureDeselectListener(view);
    installUserScrollIntentTracker(view.scrollDOM);
    const wrap = document.createElement("span");
    wrap.className = "cm-md-image-wrap";
    // Stamp the source position on the wrap so the document-level
    // keydown listener (Delete / Enter on a selected image) can
    // round-trip back into the syntax tree without holding a
    // reference to `this`.
    wrap.dataset.imagePos = String(this.nodePos);
    // `writable` is captured at scan time and carried on the widget so
    // a read-only <-> writable flip re-renders (see the eq() note).
    const editable = this.writable;
    // Wrap-level mousedown catches clicks that don't land on a
    // child with its own handler (img, handle, action buttons,
    // broken-image badge). For a normal image the img covers the
    // whole interior so this rarely fires; for a BROKEN image the
    // padded badge sits inside a wider wrap and the surrounding
    // gap used to fall through to CM6's default caret placement,
    // landing the caret inside the image source and flipping the
    // widget into edit mode just from clicking near it. Swallow
    // here and treat it as a select. The badge's own handler
    // wins for clicks on the badge itself (it's a descendant and
    // stops propagation).
    wrap.addEventListener("mousedown", (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();
      clearImageSelection(view);
      wrap.dataset.selected = "true";
    });
    if (this.standalone) wrap.dataset.standalone = "true";
    if (this.editing) wrap.dataset.editing = "true";
    const { width, align } = parseImageSrc(this.src);
    if (align) wrap.dataset.align = align;

    const img = document.createElement("img");
    img.alt = this.alt;
    const resolved = resolveImageSrc(this.src, this.fromPath);
    if (resolved) img.src = resolved;
    if (width != null) img.style.width = `${width}px`;
    // Drag-to-move: in writable mode the rendered image is a drag
    // handle. Dragging it to a different row relocates its
    // `![alt](src)` markdown (the editor-level `drop` handler in
    // image_drop.ts reads the source range from dataTransfer and moves
    // it). Left/center/right + width ride in the src fragment and move
    // verbatim; this only changes the ROW. The draggable lives on the
    // IMG, not the wrap: CodeMirror manages (and resets) the
    // draggable property on the widget root DOM, but leaves the child
    // img alone, so the img is the reliable drag source. Read-only
    // mode keeps the img non-draggable (no source edits there).
    img.draggable = editable;
    if (editable) {
      img.addEventListener("dragstart", (e) => {
        beginImageDrag(e, view, this.nodePos, wrap);
      });
      img.addEventListener("dragend", () => {
        wrap.removeAttribute("data-dragging");
        clearImageDragIndicator(view);
      });
    }
    // Broken-image placeholder: when the resolved URL 404s or
    // resolution itself returned empty (relative path against a
    // null fromPath in chat bubbles, etc.), swap in a visible
    // "missing image" badge so the user sees something is wrong
    // instead of an invisible empty span. Mirror the markdown
    // source in the badge so the user can spot the bad path.
    const showBroken = () => {
      wrap.dataset.broken = "true";
      img.remove();
      const badge = document.createElement("span");
      badge.className = "cm-md-image-broken";
      const icon = document.createElement("span");
      icon.className = "cm-md-image-broken-icon";
      icon.textContent = "🖼";
      icon.setAttribute("aria-hidden", "true");
      badge.appendChild(icon);
      const label = document.createElement("span");
      label.className = "cm-md-image-broken-label";
      label.textContent = this.alt
        ? `${this.alt} (image not found: ${this.src})`
        : `image not found: ${this.src}`;
      badge.appendChild(label);
      // Click on the broken badge -> reveal source so the user can
      // fix the path. The whole point of clicking a broken image is
      // "show me what reference is wrong"; landing in the URL
      // achieves that without an Edit button (the hover actions row
      // is hidden on broken via existing CSS).
      badge.addEventListener("mousedown", (e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        e.stopPropagation();
        placeCaretInImageUrl(view, this.nodePos);
      });
      wrap.insertBefore(badge, wrap.firstChild);
    };
    if (!resolved) {
      // Empty resolution = no point even trying to load.
      queueMicrotask(showBroken);
    } else {
      img.addEventListener("error", showBroken, { once: true });
      // Inline atom widgets have unknown height until the resource
      // loads; when the bytes arrive the containing line can grow by
      // hundreds (or thousands, for unbounded images) of pixels. CM's
      // own caret-tracking only runs on transactions, so an async load
      // that happens after the user typed `![](path)` leaves the
      // caret stranded far below the viewport with no follow-up
      // scroll. Re-anchor the scroll once the image lands, but only
      // when the caret is on or next to THIS image's source line -       // anywhere else means the user is editing elsewhere while a
      // distant image streams in, and re-scrolling would fight their
      // deliberate position.
      img.addEventListener(
        "load",
        () => {
          if (!wrap.isConnected) return;
          installUserScrollIntentTracker(view.scrollDOM);
          if (userScrollIntentActive(view.scrollDOM)) return;
          const head = view.state.selection.main.head;
          // The old gate `Math.abs(headLine - imgLine) > 1 return`
          // was too restrictive. The assumption was "if user is
          // editing far from the image, the image load won't move
          // their position." But a tall image rendering ABOVE the
          // caret pushes the entire layout down -- the caret moves
          // off-screen even though the user hasn't touched
          // anything. Repro: list-at-bottom
          // + image above -> image renders -> list pushes down
          // -> caret vanishes from viewport.
          //
          // The viewport-check below already gates correctly:
          // if the caret is still visible, return (no
          // disturbance to "deliberate position"). If the
          // caret is off-screen, restore visibility - that's
          // the desired UX regardless of distance to the
          // image.
          const cb = view.coordsAtPos(head);
          if (!cb) return;
          const sb = view.scrollDOM.getBoundingClientRect();
          if (cb.top >= sb.top && cb.bottom <= sb.bottom) return;
          view.dispatch({
            effects: EditorView.scrollIntoView(head, { y: "nearest" }),
          });
        },
        { once: true },
      );
    }
    img.addEventListener("mousedown", (e) => {
      if (e.button !== 0) return;
      // Preventing default on the img mousedown blocks the native
      // HTML5 drag the draggable img relies on (write mode), so only
      // do it in read-only mode. `stopPropagation` still stops CM6
      // from placing a caret. A modifier-click (zoom) is handled below
      // and never starts a drag; a plain press that turns into a drag
      // fires dragstart (relocate), a plain press that stays put fires
      // the select ring on mouseup.
      if (!editable) e.preventDefault();
      e.stopPropagation();
      // Cmd/Ctrl-click -> trigger the host's onClick handler (zoom).
      // Plain click -> mark the wrap as selected (visual ring; no
      // caret motion). Entering edit mode is explicit now: the
      // Edit button next to the View button, or arrow-key navigation
      // INTO the image's source markers. Earlier behaviour
      // (clicking dropped the caret inside the URL and the bubble
      // auto-opened) made every interaction with an image - picking
      // it for zoom, taking a screenshot, just clicking past it -       // flip the widget into source-edit mode, which read as a bug.
      if ((e.metaKey || e.ctrlKey) && this.onClick) {
        this.onClick({ src: this.src, alt: this.alt, pos: this.nodePos });
        return;
      }
      // Selection ring: a single `data-selected` attribute on the
      // wrap. A document-level mousedown listener (installed once
      // below; see clearImageSelection) drops the ring when the
      // user clicks outside any image wrap.
      clearImageSelection(view);
      wrap.dataset.selected = "true";
    });
    wrap.appendChild(img);

    // Read-only contexts (user-toggled read mode, fs-locked file)
    // suppress the write-side chrome: no resize handle and no Edit
    // button. The View / zoom button stays so the user can fullscreen
    // the image. `editable` was resolved once at the top of toDOM
    // (same live-facet check the date / wiki widgets use).
    if (editable) {
      const handle = document.createElement("span");
      handle.className = "cm-md-image-handle";
      handle.addEventListener("mousedown", (e) =>
        startResize(e, wrap, img, view),
      );
      wrap.appendChild(handle);
    }

    // Hover action overlay. In writable mode we offer Edit + View;
    // in read-only we offer View only.
    const actions = document.createElement("span");
    actions.className = "cm-md-image-actions";
    if (editable) {
      const editBtn = document.createElement("button");
      editBtn.type = "button";
      editBtn.className = "cm-md-image-action";
      editBtn.textContent = "Edit";
      editBtn.addEventListener("mousedown", (e) => {
        e.preventDefault();
        e.stopPropagation();
        placeCaretInImageUrl(view, this.nodePos);
      });
      actions.appendChild(editBtn);
    }
    // Copy sits last in the row (Edit - View - Copy), available in
    // read-only too. Icon-only so the row stays compact; transient
    // Check feedback on success.
    const copyBtn = document.createElement("button");
    copyBtn.type = "button";
    copyBtn.className = "cm-md-image-action cm-md-image-copy";
    copyBtn.title = "copy image to clipboard";
    copyBtn.setAttribute("aria-label", "copy image to clipboard");
    copyBtn.innerHTML = COPY_ICON_SVG;
    copyBtn.addEventListener("mousedown", (e) => {
      e.preventDefault();
      e.stopPropagation();
      void copyImageToClipboard(this.src, this.fromPath).then(
        () => {
          copyBtn.innerHTML = CHECK_ICON_SVG;
          setTimeout(() => {
            copyBtn.innerHTML = COPY_ICON_SVG;
          }, 1200);
        },
        () => {
          // Surface failure briefly via the title attr. No toast
          // surface to land this in; the user will retry if they
          // care.
          const prev = copyBtn.title;
          copyBtn.title = "copy failed";
          setTimeout(() => {
            copyBtn.title = prev;
          }, 1200);
        },
      );
    });
    const zoomBtn = document.createElement("button");
    zoomBtn.type = "button";
    zoomBtn.className = "cm-md-image-action";
    zoomBtn.textContent = "View";
    zoomBtn.addEventListener("mousedown", (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (this.onClick) {
        this.onClick({ src: this.src, alt: this.alt, pos: this.nodePos });
      }
    });
    actions.appendChild(zoomBtn);
    actions.appendChild(copyBtn);
    wrap.appendChild(actions);

    // Per-image data the document-level keymap (ensureDeselectListener)
    // needs to route Cmd+Enter (view) and Cmd+C (copy) without having
    // to walk the syntax tree. The keymap finds the wrap via the
    // `data-selected` ring, then reads this property to dispatch the
    // same action the hover-overlay buttons would.
    (wrap as HTMLElement & { _chanImg?: ImageActionPayload })._chanImg = {
      src: this.src,
      alt: this.alt,
      fromPath: this.fromPath,
      nodePos: this.nodePos,
      onClick: this.onClick,
    };

    return wrap;
  }

  ignoreEvent(): boolean {
    return true;
  }
}

/// Drop the `data-selected` ring from any image wrap that has it.
/// Called from the per-widget click handler before lighting up the
/// new selection, and from a document-level mousedown listener
/// (installed once on first widget mount) so a click anywhere
/// outside an image clears the ring.
function clearImageSelection(view: EditorView): void {
  for (const el of view.dom.querySelectorAll(
    ".cm-md-image-wrap[data-selected]",
  )) {
    (el as HTMLElement).removeAttribute("data-selected");
  }
}

/// Per-view flag so the document-level "click-outside clears
/// selection" + keyboard listeners install exactly once even when
/// many image widgets render. Stored on the EditorView's DOM so
/// they get torn down with the view.
function ensureDeselectListener(view: EditorView): void {
  const dom = view.dom as HTMLElement & { _chanImgDeselect?: boolean };
  if (dom._chanImgDeselect) return;
  dom._chanImgDeselect = true;
  document.addEventListener("mousedown", (e) => {
    const t = e.target as Node | null;
    if (!t) return;
    // Click inside an image wrap (or its hover overlay buttons)
    // leaves selection alone - the widget's own mousedown will
    // re-set the ring on the clicked wrap.
    if ((t as Element).closest?.(".cm-md-image-wrap")) return;
    clearImageSelection(view);
  });
  document.addEventListener("keydown", (e) => {
    const selected = view.dom.querySelector(
      ".cm-md-image-wrap[data-selected]",
    ) as HTMLElement | null;
    if (!selected) return;
    const posAttr = selected.dataset.imagePos;
    if (posAttr === undefined) return;
    const hintPos = Number(posAttr);
    if (!Number.isFinite(hintPos)) return;
    const payload = (selected as HTMLElement & {
      _chanImg?: ImageActionPayload;
    })._chanImg;
    const hasMod = e.metaKey || e.ctrlKey;
    // Cmd/Ctrl+Enter - same as clicking the View button (zoom modal).
    if (hasMod && e.key === "Enter" && !e.altKey && !e.shiftKey) {
      e.preventDefault();
      if (payload?.onClick) {
        payload.onClick({
          src: payload.src,
          alt: payload.alt,
          pos: payload.nodePos,
        });
      }
      clearImageSelection(view);
      return;
    }
    // Cmd/Ctrl+C - same as clicking the Copy button. We only consume
    // the key when no text range is selected, so a regular text copy
    // (range selection that happens to span an image) keeps working.
    if (
      hasMod &&
      (e.key === "c" || e.key === "C") &&
      !e.altKey &&
      !e.shiftKey &&
      view.state.selection.main.empty
    ) {
      if (payload) {
        e.preventDefault();
        void copyImageToClipboard(payload.src, payload.fromPath);
      }
      return;
    }
    // Plain Enter - same as clicking the Edit button.
    if (!hasMod && !e.altKey && !e.shiftKey && e.key === "Enter") {
      e.preventDefault();
      placeCaretInImageUrl(view, hintPos);
      clearImageSelection(view);
      return;
    }
    // Other modifier combos: leave alone so Cmd+A / Cmd+S / etc. keep
    // working when the image happens to be selected.
    if (hasMod || e.altKey) return;
    if (e.key === "Backspace" || e.key === "Delete") {
      e.preventDefault();
      const range = imageNodeRange(view, hintPos);
      if (!range) return;
      // Delete the whole `![alt](src)` source. Caret lands where
      // the image used to start so typing continues in place.
      view.dispatch({
        changes: { from: range.from, to: range.to, insert: "" },
        selection: { anchor: range.from },
      });
      clearImageSelection(view);
      view.focus();
      return;
    }
  });
}

/// Outer source range of the Image node anchored near `hintPos`,
/// or null when the syntax tree has moved on (rare; transient
/// during edits). Shared by Delete + Enter handlers.
function imageNodeRange(
  view: EditorView,
  hintPos: number,
): { from: number; to: number } | null {
  const tree = syntaxTree(view.state);
  let node: import("@lezer/common").SyntaxNode | null = tree.resolveInner(
    hintPos,
    1,
  );
  while (node && node.name !== "Image") node = node.parent ?? null;
  if (!node || node.name !== "Image") return null;
  return { from: node.from, to: node.to };
}

function placeCaretInImageUrl(view: EditorView, hintPos: number): void {
  // hintPos is the Image node's start as captured when the widget
  // was constructed. Looking up via syntaxTree is more reliable than
  // posAtDOM on the wrap - the wrap may sit at line.from when the
  // widget renders as block-above (edit mode), where resolveInner
  // walks up through Paragraph / Document and never reaches the
  // Image node. Using the captured nodePos lands directly inside
  // the Image.
  const tree = syntaxTree(view.state);
  let node: import("@lezer/common").SyntaxNode | null = tree.resolveInner(
    hintPos,
    1,
  );
  while (node && node.name !== "Image") node = node.parent ?? null;
  if (!node || node.name !== "Image") return;
  const cursor = node.cursor();
  if (!cursor.firstChild()) return;
  let urlFrom = -1;
  let urlTo = -1;
  do {
    if (cursor.name === "URL") {
      urlFrom = cursor.from;
      urlTo = cursor.to;
      break;
    }
  } while (cursor.nextSibling());
  if (urlFrom < 0 || urlTo < 0) return;
  // Bias the caret to a position strictly inside the URL slot when
  // possible. Landing at urlTo (the boundary between URL and the
  // closing `)` LinkMark) is ambiguous for `resolveInner(pos, 0)` in
  // the bubble's urlSlotAtCaret trigger - it can resolve to either
  // sibling node, and at least in the broken-image flow the
  // ambiguity prevents the raw-mode trigger from firing. The bubble
  // would either fall through to wrap-mode (inserting `![](new)`
  // ADJACENT to the broken source) or open nothing at all. Landing
  // at urlFrom + 1 (one char past the `(` LinkMark) sits cleanly
  // inside the URL leaf so resolveInner reaches Image without
  // boundary ambiguity. For an empty URL (`![alt]()`) there's no
  // interior - urlFrom == urlTo - and the bubble's LinkMark-based
  // fallback handles that case.
  const anchor = urlFrom < urlTo ? urlFrom + 1 : urlFrom;
  view.dispatch({ selection: { anchor } });
  view.focus();
}

function startResize(
  e: MouseEvent,
  wrap: HTMLElement,
  img: HTMLImageElement,
  view: EditorView,
): void {
  e.preventDefault();
  e.stopPropagation();
  const startX = e.clientX;
  const startW = img.getBoundingClientRect().width;
  // Drag threshold: a plain click on the handle (no movement) should
  // NOT commit a width. Without this guard, clicking the handle on
  // an image that has no explicit `#w=N` reads the rendered width
  // (the editor canvas width when the image is wide) and writes it
  // back as `#w=<canvas-width>`, shrinking the image visibly the
  // next render.
  const DRAG_THRESHOLD_PX = 3;
  let moved = false;
  const onMove = (ev: MouseEvent): void => {
    const dx = ev.clientX - startX;
    if (!moved && Math.abs(dx) < DRAG_THRESHOLD_PX) return;
    moved = true;
    const newW = Math.max(MIN_IMG_WIDTH, Math.round(startW + dx));
    img.style.width = `${newW}px`;
  };
  const onUp = (): void => {
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup", onUp);
    if (!moved) return;
    const finalW = Math.round(img.getBoundingClientRect().width);
    commitImageWidth(view, wrap, finalW);
  };
  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup", onUp);
}

function commitImageWidth(
  view: EditorView,
  wrap: HTMLElement,
  width: number,
): void {
  const wrapPos = view.posAtDOM(wrap);
  if (wrapPos < 0) return;
  // Walk the syntax tree from wrapPos out to the enclosing Image node
  // (the wrap sits at the Image's source range, so resolveInner
  // typically lands inside Image directly).
  const tree = syntaxTree(view.state);
  let node = tree.resolveInner(wrapPos, 1);
  while (node && node.name !== "Image") node = node.parent ?? null!;
  if (!node || node.name !== "Image") return;
  const cursor = node.cursor();
  if (!cursor.firstChild()) return;
  let urlFrom = -1;
  let urlTo = -1;
  do {
    if (cursor.name === "URL") {
      urlFrom = cursor.from;
      urlTo = cursor.to;
      break;
    }
  } while (cursor.nextSibling());
  if (urlFrom < 0) return;
  const oldSrc = view.state.doc.sliceString(urlFrom, urlTo);
  const newSrc = setImageWidth(oldSrc, width);
  if (newSrc === oldSrc) return;
  view.dispatch({
    changes: { from: urlFrom, to: urlTo, insert: newSrc },
  });
}

/// Sync the `data-selected` ring on image wraps with the caret.
/// Atomic ranges make arrow-key navigation jump from "before" to
/// "after" an image in a single keystroke; we want that landing
/// (caret at Image.from or Image.to) to visually SELECT the image
/// - same ring the click handler lights up - so the user can then
/// Cmd/Ctrl+Enter into edit mode or Backspace to delete. Stepping
/// the caret off the boundary clears the ring on the next update.
///
/// We deliberately do NOT redirect the caret inside the URL slot
/// anymore. The old behaviour flipped the widget into edit mode on
/// every keyboard or click landing near the image, which the user
/// experienced as a stray click "landing in the source". Edit mode
/// is now an explicit verb: the Edit button on the hover overlay,
/// or Cmd/Ctrl+Enter while the image is selected (see the keydown
/// handler in ensureDeselectListener).
export function imageCaretRedirect(): Extension {
  return EditorView.updateListener.of((u) => {
    if (!u.selectionSet && !u.docChanged && !u.viewportChanged) return;
    const cur = u.state.selection.main;
    let selectedPos: number | null = null;
    if (cur.empty) {
      const tree = syntaxTree(u.state);
      // Try both bias directions - caret AT a boundary may resolve
      // to either the Image node or its sibling depending on which
      // side of the boundary `resolveInner` lands on.
      for (const bias of [-1, 1] as const) {
        let node: import("@lezer/common").SyntaxNode | null =
          tree.resolveInner(cur.head, bias);
        while (node && node.name !== "Image") node = node.parent ?? null;
        if (
          node &&
          node.name === "Image" &&
          (cur.head === node.from || cur.head === node.to)
        ) {
          selectedPos = node.from;
          break;
        }
      }
    }
    // Wipe stale rings (covers the "selection moved off image" path
    // and the "doc edit shifted positions" path). Then re-set the
    // ring on the wrap whose Image.from matches the captured pos.
    for (const el of u.view.dom.querySelectorAll(
      ".cm-md-image-wrap[data-selected]",
    )) {
      (el as HTMLElement).removeAttribute("data-selected");
    }
    if (selectedPos !== null) {
      const wrap = u.view.dom.querySelector(
        `.cm-md-image-wrap[data-image-pos="${selectedPos}"]`,
      ) as HTMLElement | null;
      if (wrap) wrap.dataset.selected = "true";
    }
  });
}

export function imageDecorations(opts: ImageOptions): Extension {
  // Inline replace + line clear decorations live in a ViewPlugin so
  // they recompute on viewport changes (cheap, scoped to visible
  // tree). CM6 forbids ViewPlugins from emitting block decorations,
  // so the editing-mode block preview lives in a separate StateField
  // below.
  const plugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = scanImagesInline(view, opts);
      }

      update(u: ViewUpdate): void {
        // Re-scan on a read-only <-> writable flip too: the editable
        // state is baked into each ImageWidget (drag-to-move
        // affordance), so a facet reconfigure that doesn't otherwise
        // touch doc / selection / viewport must still rebuild the
        // widgets, or the affordance stays stuck at the pre-flip
        // value.
        const editableChanged =
          u.startState.facet(EditorView.editable) !==
          u.state.facet(EditorView.editable);
        if (
          u.docChanged ||
          u.viewportChanged ||
          u.selectionSet ||
          editableChanged
        ) {
          this.decorations = scanImagesInline(u.view, opts);
        }
      }
    },
    {
      decorations: (v) => v.decorations,
    },
  );
  // Block preview StateField. Recomputes per-transaction on doc or
  // selection change. Only emits widgets for images whose source range
  // intersects the selection (caret-in-image edit mode). Doc-wide tree
  // walk is cheap relative to a typical doc's image count.
  const blockField = StateField.define<DecorationSet>({
    create(state): DecorationSet {
      return scanImagesBlock(state, opts);
    },
    update(value, tr): DecorationSet {
      if (!tr.docChanged && !tr.selection) return value.map(tr.changes);
      return scanImagesBlock(tr.state, opts);
    },
    provide: (f) => EditorView.decorations.from(f),
  });
  return [
    plugin,
    blockField,
    EditorView.atomicRanges.of(
      (view) => view.plugin(plugin)?.decorations ?? Decoration.none,
    ),
  ];
}

function scanImagesInline(view: EditorView, opts: ImageOptions): DecorationSet {
  const { state } = view;
  const sel = state.selection;
  const { from, to } = view.viewport;
  const decos: Array<{ from: number; to: number; deco: Decoration }> = [];
  const fromPath = opts.getCurrentPath();
  const editable = state.facet(EditorView.editable);
  syntaxTree(state).iterate({
    from,
    to,
    enter(node) {
      if (node.name !== "Image") return;
      const outerFrom = node.from;
      const outerTo = node.to;
      // Read the alt text (between the first and second LinkMark) and
      // the URL (between `(` and `)`).
      const cursor = node.node.cursor();
      if (!cursor.firstChild()) return;
      const linkMarks: Array<{ from: number; to: number }> = [];
      let urlFrom = -1;
      let urlTo = -1;
      do {
        if (cursor.name === "LinkMark") {
          linkMarks.push({ from: cursor.from, to: cursor.to });
        } else if (cursor.name === "URL") {
          urlFrom = cursor.from;
          urlTo = cursor.to;
        }
      } while (cursor.nextSibling());
      if (linkMarks.length < 4 || urlFrom < 0) return;
      const altFrom = linkMarks[0]!.to;
      const altTo = linkMarks[1]!.from;
      const alt = state.doc.sliceString(altFrom, altTo);
      const src = state.doc.sliceString(urlFrom, urlTo);
      const line = state.doc.lineAt(outerFrom);
      const standalone =
        line.text.trim() === state.doc.sliceString(outerFrom, outerTo).trim();
      const editing = imageEditEntered(sel, outerFrom, outerTo);
      // Editing mode: skip the inline replace so the source `![alt](url)`
      // stays as editable text. The block-above preview comes from
      // scanImagesBlock (StateField). Float-clear line decoration is
      // also skipped - the preview is a separate block, no float.
      if (editing) return;
      const widget = new ImageWidget(
        alt,
        src,
        fromPath,
        outerFrom,
        standalone,
        false,
        editable,
        opts.onImageClick,
      );
      decos.push({
        from: outerFrom,
        to: outerTo,
        deco: Decoration.replace({ widget }),
      });
      // Inline (non-standalone) image with left/right align: float
      // keeps wrapping subsequent lines around the image. Add
      // clear:both on the next line so only the same line flows
      // beside the image.
      if (!standalone) {
        const { align } = parseImageSrc(src);
        if (align === "left" || align === "right") {
          const nextLineNum = line.number + 1;
          if (nextLineNum <= state.doc.lines) {
            const nextLine = state.doc.line(nextLineNum);
            decos.push({
              from: nextLine.from,
              to: nextLine.from,
              deco: CLEAR_AFTER_IMAGE,
            });
          }
        }
      }
    },
  });
  decos.sort((a, b) => a.from - b.from);
  return Decoration.set(
    decos.map((d) => d.deco.range(d.from, d.to)),
    true,
  );
}

function scanImagesBlock(
  state: import("@codemirror/state").EditorState,
  opts: ImageOptions,
): DecorationSet {
  const sel = state.selection;
  const decos: Array<{ from: number; to: number; deco: Decoration }> = [];
  const fromPath = opts.getCurrentPath();
  const editable = state.facet(EditorView.editable);
  // Doc-wide tree walk. StateFields don't see viewport, but the doc-
  // wide pass emits widgets only for selection-intersected images, so
  // the cost is bounded by selection count, not doc size.
  syntaxTree(state).iterate({
    from: 0,
    to: state.doc.length,
    enter(node) {
      if (node.name !== "Image") return;
      const outerFrom = node.from;
      const outerTo = node.to;
      if (!imageEditEntered(sel, outerFrom, outerTo)) return;
      const cursor = node.node.cursor();
      if (!cursor.firstChild()) return;
      const linkMarks: Array<{ from: number; to: number }> = [];
      let urlFrom = -1;
      let urlTo = -1;
      do {
        if (cursor.name === "LinkMark") {
          linkMarks.push({ from: cursor.from, to: cursor.to });
        } else if (cursor.name === "URL") {
          urlFrom = cursor.from;
          urlTo = cursor.to;
        }
      } while (cursor.nextSibling());
      if (linkMarks.length < 4 || urlFrom < 0) return;
      const altFrom = linkMarks[0]!.to;
      const altTo = linkMarks[1]!.from;
      const alt = state.doc.sliceString(altFrom, altTo);
      const src = state.doc.sliceString(urlFrom, urlTo);
      const line = state.doc.lineAt(outerFrom);
      const standalone =
        line.text.trim() === state.doc.sliceString(outerFrom, outerTo).trim();
      const widget = new ImageWidget(
        alt,
        src,
        fromPath,
        outerFrom,
        standalone,
        true,
        editable,
        opts.onImageClick,
      );
      decos.push({
        from: line.from,
        to: line.from,
        deco: Decoration.widget({ widget, side: -1, block: true }),
      });
    },
  });
  decos.sort((a, b) => a.from - b.from);
  return Decoration.set(
    decos.map((d) => d.deco.range(d.from, d.to)),
    true,
  );
}
