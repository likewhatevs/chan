// Image atom widget for `![alt](src)` markdown.
//
// Per design.md spec #5, images are atomic widgets. Source revealed
// when selection intersects. EditorView.atomicRanges so caret motion
// skips the image in one keystroke.
//
// Behaviors:
//   - Renders `<img>` with `src` resolved against the editing file's
//     path (drive-relative paths route through /api/files/{...} with
//     the auth token).
//   - Width fragment `#w=N` and alignment `#left` / `#right` parsed
//     from the src and applied as inline style + dataset.
//   - Bottom-right drag-resize handle visible on hover. Mousedown
//     starts a window-level mousemove/mouseup loop; the handle
//     mutates `img.style.width` live during drag. On mouseup the
//     final width commits to the source via `setImageWidth` +
//     `view.dispatch` — the widget then re-mounts at the persisted
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
import { selectionInRange } from "../decorations/selection";

const MIN_IMG_WIDTH = 40;

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
  /// Position of the Image node's start in the source — useful for
  /// the action overlay (step 8) to anchor itself or trigger an
  /// edit-bubble open at the right offset.
  pos: number;
}

export interface ImageOptions {
  /// Read the editing file's drive-rooted path. Used to resolve
  /// relative img sources against the right directory. `null` keeps
  /// sources drive-rooted (no relativization).
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
    /// layout — alignment moves the image LEFT / CENTER / RIGHT
    /// within its own line via flex justify-content. Inline images
    /// (mixed with paragraph text) keep the existing float layout
    /// so text wraps around them.
    readonly standalone: boolean,
    /// True when the widget is rendered AS A BLOCK PREVIEW above an
    /// editable source line (caret is inside the image's source).
    /// In edit mode the float-around-text layout doesn't apply —
    /// the preview is a sibling to the source row, not a replacement.
    readonly editing: boolean,
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
      this.editing === other.editing
    );
  }

  toDOM(view: EditorView): HTMLElement {
    const wrap = document.createElement("span");
    wrap.className = "cm-md-image-wrap";
    if (this.standalone) wrap.dataset.standalone = "true";
    if (this.editing) wrap.dataset.editing = "true";
    const { width, align } = parseImageSrc(this.src);
    if (align) wrap.dataset.align = align;

    const img = document.createElement("img");
    img.alt = this.alt;
    const resolved = resolveImageSrc(this.src, this.fromPath);
    if (resolved) img.src = resolved;
    if (width != null) img.style.width = `${width}px`;
    img.draggable = false;
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
      wrap.insertBefore(badge, wrap.firstChild);
    };
    if (!resolved) {
      // Empty resolution = no point even trying to load.
      queueMicrotask(showBroken);
    } else {
      img.addEventListener("error", showBroken, { once: true });
    }
    img.addEventListener("mousedown", (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();
      // Cmd/Ctrl-click -> trigger the host's onClick handler
      // (which opens the image-zoom modal). Plain click drops the
      // caret inside the URL so the image bubble auto-opens via the
      // imageUrlAtCaret trigger.
      if ((e.metaKey || e.ctrlKey) && this.onClick) {
        this.onClick({ src: this.src, alt: this.alt, pos: this.nodePos });
        return;
      }
      placeCaretInImageUrl(view, this.nodePos);
    });
    wrap.appendChild(img);

    // Read-only contexts (assistant chat replies, user-toggled
    // read mode, fs-locked file) suppress the write-side chrome:
    // no resize handle and no Edit button. The View / zoom button
    // stays so the user can fullscreen the image. Same live-facet
    // check the date / wiki widgets use.
    const editable = view.state.facet(EditorView.editable);

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
    wrap.appendChild(actions);

    return wrap;
  }

  ignoreEvent(): boolean {
    return true;
  }
}

function placeCaretInImageUrl(view: EditorView, hintPos: number): void {
  // hintPos is the Image node's start as captured when the widget
  // was constructed. Looking up via syntaxTree is more reliable than
  // posAtDOM on the wrap — the wrap may sit at line.from when the
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
  let urlTo = -1;
  do {
    if (cursor.name === "URL") {
      urlTo = cursor.to;
      break;
    }
  } while (cursor.nextSibling());
  if (urlTo < 0) return;
  view.dispatch({ selection: { anchor: urlTo } });
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

/// Redirect the caret into an Image's URL slot when the user's
/// arrow key would have skipped over the atom. Without this, keyboard
/// navigation past an image lands the caret at the Image's outer
/// boundary — selection-intersect reveals source but the caret is
/// in alt-text or just outside the URL, so the user has to keep
/// arrowing to reach the URL portion where the bubble fires.
///
/// We catch the boundary landing (cur.head === Image.from || === to)
/// when the previous selection was outside the Image, and dispatch
/// a follow-up that lands the caret at URL.from (rightward motion)
/// or URL.to (leftward). The bubble's imageUrlAtCaret trigger then
/// fires on the next update tick and the user is editing.
export function imageCaretRedirect(): Extension {
  return EditorView.updateListener.of((u) => {
    if (!u.selectionSet || u.docChanged) return;
    const prev = u.startState.selection.main;
    const cur = u.state.selection.main;
    if (!cur.empty || !prev.empty) return;
    if (prev.head === cur.head) return;
    const tree = syntaxTree(u.state);
    let node: import("@lezer/common").SyntaxNode | null = tree.resolveInner(
      cur.head,
      0,
    );
    while (node && node.name !== "Image") node = node.parent;
    if (!node) return;
    // Only redirect on the atomic-range jump landing (caret at
    // Image.from or Image.to). Caret already inside the URL via
    // mouse-click or earlier redirect is left alone.
    if (cur.head !== node.from && cur.head !== node.to) return;
    if (prev.head >= node.from && prev.head <= node.to) return;
    const cursor = node.cursor();
    if (!cursor.firstChild()) return;
    const linkMarks: Array<{ from: number; to: number }> = [];
    do {
      if (cursor.name === "LinkMark") {
        linkMarks.push({ from: cursor.from, to: cursor.to });
      }
    } while (cursor.nextSibling());
    if (linkMarks.length < 4) return;
    const urlFrom = linkMarks[2]!.to;
    const urlTo = linkMarks[3]!.from;
    const target = prev.head < cur.head ? urlFrom : urlTo;
    u.view.dispatch({ selection: { anchor: target } });
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
        if (u.docChanged || u.viewportChanged || u.selectionSet) {
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
      const editing = selectionInRange(sel, outerFrom, outerTo);
      // Editing mode: skip the inline replace so the source `![alt](url)`
      // stays as editable text. The block-above preview comes from
      // scanImagesBlock (StateField). Float-clear line decoration is
      // also skipped — the preview is a separate block, no float.
      if (editing) return;
      const widget = new ImageWidget(
        alt,
        src,
        fromPath,
        outerFrom,
        standalone,
        false,
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
      if (!selectionInRange(sel, outerFrom, outerTo)) return;
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
