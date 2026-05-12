// Inline image rendering.
//
// Minimal Image node: parses `![alt](src)` from markdown, renders an
// `<img>`. Drive-relative srcs (`foo.png`, `./foo.png`, `../foo.png`,
// `/abs.png`) are resolved against `/api/files/<path>` so the editor
// can display them inline; absolute URLs (http/data/blob) pass
// through. Missing files render the browser's default broken-image
// placeholder; we do not intercept errors.
//
// Insert and edit flows are deliberately not here. They will be built
// back up as the UX is designed.

import Image from "@tiptap/extension-image";
import { mergeAttributes } from "@tiptap/core";

import { withTokenQuery } from "../../api/client";
import { normalizeHref } from "../links";

const IMAGE_EXTS = ["png", "jpg", "jpeg", "gif", "webp", "svg"] as const;

export function isImagePath(path: string): boolean {
  const dot = path.lastIndexOf(".");
  if (dot < 0) return false;
  const ext = path.slice(dot + 1).toLowerCase();
  return (IMAGE_EXTS as readonly string[]).includes(ext);
}

export type ImageAlign = "left" | "right";

/// Split a markdown image src into the URL portion (used to fetch
/// the file) and the optional fragment params. Width travels as
/// `#w=N`; alignment as bare `#left` / `#right` (absent fragment =
/// the default, centered). The fragment stays in the markdown so
/// other renderers see a plain `file.png` (path-only after the
/// hash is stripped); chan-drive's link parser strips `#...` before
/// indexing, so fragments never leak into the graph.
///
/// Multiple fragment params are joined with `&` (e.g. `#w=200&left`).
/// Unknown parts are preserved on the returned `base` so they round-
/// trip through edits that don't touch them.
export function parseImageSrc(src: string): {
  base: string;
  width: number | null;
  align: ImageAlign | null;
} {
  if (!src) return { base: "", width: null, align: null };
  const hash = src.indexOf("#");
  if (hash < 0) return { base: src, width: null, align: null };
  const baseUrl = src.slice(0, hash);
  const fragment = src.slice(hash + 1);
  let width: number | null = null;
  let align: ImageAlign | null = null;
  const kept: string[] = [];
  for (const part of fragment.split("&")) {
    if (part === "left" || part === "right") {
      align = part;
      continue;
    }
    const eq = part.indexOf("=");
    const key = eq < 0 ? part : part.slice(0, eq);
    const val = eq < 0 ? "" : part.slice(eq + 1);
    if (key === "w") {
      const n = parseInt(val, 10);
      if (Number.isFinite(n) && n > 0) width = n;
      continue;
    }
    if (part) kept.push(part);
  }
  const base = kept.length === 0 ? baseUrl : `${baseUrl}#${kept.join("&")}`;
  return { base, width, align };
}

/// Rebuild an image src from its base + fragment params. Keeps any
/// unknown fragment parts the parser preserved on `base`; appends
/// `w=N` and the bare align token in a stable order so a round-trip
/// through parse/build is idempotent.
function buildImageSrc(
  base: string,
  width: number | null,
  align: ImageAlign | null,
): string {
  const parts: string[] = [];
  if (width != null) parts.push(`w=${width}`);
  if (align) parts.push(align);
  if (parts.length === 0) return base;
  return base.includes("#") ? `${base}&${parts.join("&")}` : `${base}#${parts.join("&")}`;
}

/// Resolve a markdown image src to a URL the browser can load.
/// Local drive paths route through `/api/files/` with the launch
/// token; absolute URLs (http/data/blob) pass through. Drive-rooted
/// and parent-relative sources go through `normalizeHref` against
/// `fromPath`'s directory so the resolver chan-drive uses for graph
/// edges produces the same canonical path that reaches `/api/files/`.
/// `#w=N` width fragments are stripped before the URL is built (the
/// width is applied as inline style on the `<img>` instead).
/// Empty input returns "" so callers can omit the attribute.
export function resolveImageSrc(src: string, fromPath?: string | null): string {
  if (!src) return "";
  const { base } = parseImageSrc(src);
  if (!base) return "";
  if (/^(https?:|data:|blob:)/i.test(base)) return base;
  const sourceDir = fromPath
    ? fromPath.split("/").slice(0, -1).join("/")
    : "";
  const driveRooted = normalizeHref(base, sourceDir) ?? base;
  const encoded = driveRooted
    .split("/")
    .map((s) => encodeURIComponent(s))
    .join("/");
  return withTokenQuery(`/api/files/${encoded}`);
}

/// Build the new src string for an image after resizing. `width`
/// of null strips the `#w=N` fragment; align and other fragment
/// parts round-trip untouched.
export function setImageWidth(src: string, width: number | null): string {
  const { base, align } = parseImageSrc(src);
  return buildImageSrc(base, width, align);
}

/// Build the new src string after toggling alignment. `null` clears
/// the align fragment (centered, the default); width and unknown
/// fragment parts round-trip untouched.
export function setImageAlign(src: string, align: ImageAlign | null): string {
  const { base, width } = parseImageSrc(src);
  return buildImageSrc(base, width, align);
}

/// TipTap node spec. The factory closes over `getFromPath` so the
/// renderHTML always sees the live editing file when resolving
/// relative srcs. `renderHTML` covers the static path (copy /
/// initial render); `addNodeView` overrides the in-editor render
/// so we can attach a bottom-right drag handle for resizing.
export function createImageNode(getFromPath: () => string | null) {
  return Image.extend({
    renderHTML({ HTMLAttributes }) {
      const raw = (HTMLAttributes.src as string | null) ?? "";
      const resolved = resolveImageSrc(raw, getFromPath());
      const { width, align } = parseImageSrc(raw);
      // Empty src: serialize without a `src` attribute so the
      // browser doesn't fire a useless GET to `/api/files/`. The
      // alt text (if any) still renders.
      const extra: Record<string, string> = {};
      if (resolved) extra.src = resolved;
      if (width != null) extra.style = `width: ${width}px`;
      if (align) extra["data-align"] = align;
      return ["img", mergeAttributes(HTMLAttributes, extra)];
    },
    addStorage() {
      return {
        // Defensive markdown serializer. The default image
        // serializer in prosemirror-markdown writes
        //   `![${state.esc(node.attrs.alt)}](${state.esc(node.attrs.src)})`
        // and `state.esc` calls `.replace` on its argument, so a
        // node whose `src` or `alt` ended up `null` / `undefined`
        // (malformed `![]()` patterns, partially-typed images, or
        // a `![[...]]` chunk that fooled the parser) crashes the
        // entire getMarkdown() pass, which then strands autosave.
        // Coerce both attrs to strings before we hand them off so
        // the round-trip is always safe even when the doc holds a
        // half-formed image.
        markdown: {
          serialize(
            state: unknown,
            node: { attrs: Record<string, unknown> },
          ) {
            const alt =
              typeof node.attrs.alt === "string" ? node.attrs.alt : "";
            const src =
              typeof node.attrs.src === "string" ? node.attrs.src : "";
            const title =
              typeof node.attrs.title === "string" ? node.attrs.title : "";
            const s = state as {
              esc(input: string): string;
              write(text: string): void;
            };
            const titlePart = title
              ? ` "${title.replace(/"/g, '\\"')}"`
              : "";
            s.write(`![${s.esc(alt)}](${s.esc(src)}${titlePart})`);
          },
          parse: { setup() {} },
        },
      };
    },
    addNodeView() {
      // Wrap the `<img>` in a span with a drag-resize handle pinned
      // to the bottom-right corner. The handle is muted until the
      // wrap is hovered or the image is PM-selected; dragging it
      // updates `img.style.width` live, and the final pixel width
      // is committed into the src as the `#w=N` fragment on mouseup.
      return ({ node, getPos, editor }) => {
        const wrap = document.createElement("span");
        wrap.className = "md-image-wrap";
        const img = document.createElement("img");
        img.draggable = false;
        const apply = (n: typeof node) => {
          const raw = (n.attrs.src as string | null) ?? "";
          const resolved = resolveImageSrc(raw, getFromPath());
          const { width, align } = parseImageSrc(raw);
          if (resolved) img.src = resolved;
          else img.removeAttribute("src");
          img.alt = (n.attrs.alt as string | null) ?? "";
          if (width != null) img.style.width = `${width}px`;
          else img.style.removeProperty("width");
          wrap.classList.toggle("align-left", align === "left");
          wrap.classList.toggle("align-right", align === "right");
        };
        // Toggle the `is-alone` class based on whether the parent
        // textblock holds only this image. CSS keys block-level
        // layout (centered + align-left / align-right) off this
        // class. We can't rely on `:only-child` because PM serializes
        // text neighbors as text NODES, which are not "siblings" for
        // the `:only-child` selector — a mixed `text ![](img) text`
        // line still has the wrap as the only ELEMENT child, which
        // would falsely activate block mode.
        const updateAlone = () => {
          const pos = getPos?.();
          if (typeof pos !== "number") return;
          const $pos = editor.state.doc.resolve(pos);
          wrap.classList.toggle("is-alone", $pos.parent.childCount === 1);
        };
        apply(node);
        updateAlone();
        // Node view's `update` is called only when the IMAGE node's
        // attrs / content change; sibling text edits don't trigger
        // it. Subscribe to the editor's `update` so the aloneness
        // class stays accurate as the user types around the image.
        editor.on("update", updateAlone);
        wrap.appendChild(img);

        const handle = document.createElement("span");
        handle.className = "md-image-handle";
        handle.title = "drag to resize";
        wrap.appendChild(handle);

        handle.addEventListener("mousedown", (e) => {
          // PM would otherwise interpret the mousedown as a click
          // on the image atom and re-route to its own selection /
          // overlay handler. preventDefault keeps the editor's
          // selection alive; stopPropagation keeps the overlay
          // click handler from running.
          e.preventDefault();
          e.stopPropagation();
          const startX = e.clientX;
          const startW = img.getBoundingClientRect().width;
          wrap.classList.add("is-resizing");
          const onMove = (ev: MouseEvent): void => {
            const delta = ev.clientX - startX;
            // Floor at 40px so the handle stays reachable; CSS
            // `max-width: 100%` on the img caps the upper bound to
            // the column width without us needing an explicit ceiling.
            const next = Math.max(40, Math.round(startW + delta));
            img.style.width = `${next}px`;
          };
          const onUp = (): void => {
            wrap.classList.remove("is-resizing");
            document.removeEventListener("mousemove", onMove);
            document.removeEventListener("mouseup", onUp);
            const finalW = Math.round(img.getBoundingClientRect().width);
            const pos = getPos?.();
            if (typeof pos !== "number") return;
            const current =
              (editor.state.doc.nodeAt(pos)?.attrs.src as string | null) ?? "";
            const nextSrc = setImageWidth(current, finalW);
            if (nextSrc === current) return;
            editor.view.dispatch(
              editor.state.tr.setNodeAttribute(pos, "src", nextSrc),
            );
          };
          document.addEventListener("mousemove", onMove);
          document.addEventListener("mouseup", onUp);
        });

        return {
          dom: wrap,
          update(updated) {
            if (updated.type !== node.type) return false;
            apply(updated);
            updateAlone();
            return true;
          },
          // PM would otherwise try to reconcile the inline-styled
          // img's mutations into a fresh node view on every drag
          // tick, fighting the live width state. Ignore mutations
          // inside the wrap; the explicit `update` call from PM
          // already covers attribute changes.
          ignoreMutation() {
            return true;
          },
          destroy() {
            editor.off("update", updateAlone);
          },
        };
      };
    },
  }).configure({
    inline: true,
    allowBase64: true,
  });
}
