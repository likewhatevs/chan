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
import { type Extension } from "@codemirror/state";
import {
  parseImageSrc,
  resolveImageSrc,
  setImageWidth,
} from "../../editor/extensions/image";
import { selectionInRange } from "../decorations/selection";

const MIN_IMG_WIDTH = 40;

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
    readonly onClick: ((args: ImageClickArgs) => void) | undefined,
  ) {
    super();
  }

  eq(other: ImageWidget): boolean {
    // Compare the source-of-truth fields. Two widgets producing the
    // same image at the same path can share their DOM across
    // unrelated transactions.
    return (
      this.alt === other.alt &&
      this.src === other.src &&
      this.fromPath === other.fromPath
    );
  }

  toDOM(view: EditorView): HTMLElement {
    const wrap = document.createElement("span");
    wrap.className = "cm-md-image-wrap";
    const { width, align } = parseImageSrc(this.src);
    if (align) wrap.dataset.align = align;

    const img = document.createElement("img");
    img.alt = this.alt;
    const resolved = resolveImageSrc(this.src, this.fromPath);
    if (resolved) img.src = resolved;
    if (width != null) img.style.width = `${width}px`;
    img.draggable = false;
    img.addEventListener("click", (e) => {
      if (!this.onClick) return;
      e.preventDefault();
      e.stopPropagation();
      this.onClick({ src: this.src, alt: this.alt, pos: this.nodePos });
    });
    wrap.appendChild(img);

    const handle = document.createElement("span");
    handle.className = "cm-md-image-handle";
    handle.addEventListener("mousedown", (e) =>
      startResize(e, wrap, img, view),
    );
    wrap.appendChild(handle);

    return wrap;
  }

  ignoreEvent(): boolean {
    return true;
  }
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
  const onMove = (ev: MouseEvent): void => {
    const newW = Math.max(MIN_IMG_WIDTH, Math.round(startW + (ev.clientX - startX)));
    img.style.width = `${newW}px`;
  };
  const onUp = (): void => {
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup", onUp);
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

export function imageDecorations(opts: ImageOptions): Extension {
  const plugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = scanImages(view, opts);
      }

      update(u: ViewUpdate): void {
        if (u.docChanged || u.viewportChanged || u.selectionSet) {
          this.decorations = scanImages(u.view, opts);
        }
      }
    },
    {
      decorations: (v) => v.decorations,
    },
  );
  return [
    plugin,
    EditorView.atomicRanges.of(
      (view) => view.plugin(plugin)?.decorations ?? Decoration.none,
    ),
  ];
}

function scanImages(view: EditorView, opts: ImageOptions): DecorationSet {
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
      if (selectionInRange(sel, outerFrom, outerTo)) return;
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
      const widget = new ImageWidget(
        alt,
        src,
        fromPath,
        outerFrom,
        opts.onImageClick,
      );
      decos.push({
        from: outerFrom,
        to: outerTo,
        deco: Decoration.replace({ widget }),
      });
    },
  });
  decos.sort((a, b) => a.from - b.from);
  return Decoration.set(
    decos.map((d) => d.deco.range(d.from, d.to)),
    true,
  );
}
