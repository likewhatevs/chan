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

/// Resolve a markdown image src to a URL the browser can load.
/// Local drive paths route through `/api/files/` with the launch
/// token; absolute URLs (http/data/blob) pass through. Drive-rooted
/// and parent-relative sources go through `normalizeHref` against
/// `fromPath`'s directory so the resolver chan-drive uses for graph
/// edges produces the same canonical path that reaches `/api/files/`.
/// Empty input returns "" so callers can omit the attribute.
export function resolveImageSrc(src: string, fromPath?: string | null): string {
  if (!src) return "";
  if (/^(https?:|data:|blob:)/i.test(src)) return src;
  const sourceDir = fromPath
    ? fromPath.split("/").slice(0, -1).join("/")
    : "";
  const driveRooted = normalizeHref(src, sourceDir) ?? src;
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
      // Empty src: serialize without a `src` attribute so the
      // browser doesn't fire a useless GET to `/api/files/`. The
      // alt text (if any) still renders.
      const extra: Record<string, string> = resolved ? { src: resolved } : {};
      return ["img", mergeAttributes(HTMLAttributes, extra)];
    },
  }).configure({
    inline: true,
    allowBase64: true,
  });
}
