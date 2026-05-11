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

/// Split a markdown image src into the URL portion (used to fetch
/// the file) and an optional pixel width parsed from the `#w=N`
/// fragment. The width travels in the URL fragment so the markdown
/// stays portable: other renderers ignore the fragment and show the
/// image at its natural size. Multiple fragment params separated by
/// `&` are supported (only `w` is used today; anything else is kept
/// on the returned base so it round-trips).
export function parseImageSrc(src: string): { base: string; width: number | null } {
  if (!src) return { base: "", width: null };
  const hash = src.indexOf("#");
  if (hash < 0) return { base: src, width: null };
  const baseUrl = src.slice(0, hash);
  const fragment = src.slice(hash + 1);
  let width: number | null = null;
  const kept: string[] = [];
  for (const part of fragment.split("&")) {
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
  return { base, width };
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

/// TipTap node spec. The factory closes over `getFromPath` so the
/// renderHTML always sees the live editing file when resolving
/// relative srcs (the prop reference is captured by the caller).
export function createImageNode(getFromPath: () => string | null) {
  return Image.extend({
    renderHTML({ HTMLAttributes }) {
      const raw = (HTMLAttributes.src as string | null) ?? "";
      const resolved = resolveImageSrc(raw, getFromPath());
      const { width } = parseImageSrc(raw);
      // Empty src: serialize without a `src` attribute so the
      // browser doesn't fire a useless GET to `/api/files/`. The
      // alt text (if any) still renders.
      const extra: Record<string, string> = {};
      if (resolved) extra.src = resolved;
      if (width != null) extra.style = `width: ${width}px`;
      return ["img", mergeAttributes(HTMLAttributes, extra)];
    },
  }).configure({
    inline: true,
    allowBase64: true,
  });
}
