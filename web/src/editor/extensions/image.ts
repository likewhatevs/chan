// Inline image embedding.
//
// Behavior:
//   - Typing `![` opens a popover with three actions: pick from
//     existing drive images (search-as-you-type), upload a
//     local file, or paste a remote URL. The chosen source is
//     inserted as a TipTap `image` node which serializes to the
//     standard markdown `![alt](src)` shape.
//   - Drag-dropped image files and pasted clipboard images both
//     funnel through the same upload helper for consistency.
//   - Local drive paths resolve against `/api/files/` so the
//     editor renders them inline without leaking auth tokens
//     into the markdown source.
//
// Resize is left to phase 4; this file ships the picker, the
// node spec, and the upload helpers used by drag-drop / paste
// (those wirings live in Wysiwyg.svelte).

import Image from "@tiptap/extension-image";
import { mergeAttributes } from "@tiptap/core";

import { api, withTokenQuery } from "../../api/client";
import type { TreeEntry } from "../../api/types";
import { normalizeHref, relativizePath, resolveRelativePath } from "../links";
import { positionPopover, watchViewport } from "./popover";

/// Extensions we treat as inline images. Lower-case match against
/// the extension; matches the server's allowlist on
/// `POST /api/attachments`.
const IMAGE_EXTS = ["png", "jpg", "jpeg", "gif", "webp", "svg"] as const;

export function isImagePath(path: string): boolean {
  const dot = path.lastIndexOf(".");
  if (dot < 0) return false;
  const ext = path.slice(dot + 1).toLowerCase();
  return (IMAGE_EXTS as readonly string[]).includes(ext);
}

/// Parse a markdown image src to `{ src, width }`. Width is
/// encoded in the URL fragment as `#w=N` (pixels). Anything in
/// the fragment that isn't `w=...` is preserved on the cleaned
/// src so a user-authored anchor (rare for images) survives.
export function parseSrcFragment(src: string): { src: string; width: number | null } {
  const hash = src.indexOf("#");
  if (hash < 0) return { src, width: null };
  const base = src.slice(0, hash);
  const frag = src.slice(hash + 1);
  // Fragment may carry multiple `key=val` pairs separated by
  // `&` (rare but easy to support). Pick out `w` and rebuild
  // the fragment without it.
  const parts = frag.split("&");
  let width: number | null = null;
  const kept: string[] = [];
  for (const p of parts) {
    const eq = p.indexOf("=");
    const key = eq < 0 ? p : p.slice(0, eq);
    const val = eq < 0 ? "" : p.slice(eq + 1);
    if (key === "w") {
      const n = parseInt(val, 10);
      if (Number.isFinite(n) && n > 0) width = n;
    } else if (p) {
      kept.push(p);
    }
  }
  const cleanSrc = kept.length > 0 ? `${base}#${kept.join("&")}` : base;
  return { src: cleanSrc, width };
}

/// Inverse of `parseSrcFragment`: rewrite the `#w=N` fragment on
/// `src`, replacing any existing `w` and preserving the rest.
/// `width = null` strips the `w` segment.
export function buildSrcWithWidth(src: string, width: number | null): string {
  const { src: clean } = parseSrcFragment(src);
  const hash = clean.indexOf("#");
  const base = hash < 0 ? clean : clean.slice(0, hash);
  const otherFrag = hash < 0 ? "" : clean.slice(hash + 1);
  if (width == null) {
    return otherFrag ? `${base}#${otherFrag}` : base;
  }
  const wPart = `w=${width}`;
  return otherFrag ? `${base}#${otherFrag}&${wPart}` : `${base}#${wPart}`;
}

/// TipTap node spec. Extended from upstream `Image`:
/// - `inline: true` so images flow with surrounding text rather
///   than forcing a paragraph break.
/// - `allowBase64: true` lets the WebView show pasted clipboard
///   images inline before the upload completes (drag-drop flow).
/// - Width lives in the URL fragment of `attrs.src`
///   (`![alt](path#w=400)`). Markdown round-trip is automatic
///   because tiptap-markdown writes `attrs.src` verbatim. Other
///   markdown renderers ignore the fragment and show the image
///   at its native size.
/// - `addNodeView` renders the image with a small drag handle at
///   the bottom-right; dragging mutates the stored fragment so
///   the resize survives a save and reload.
///
/// `getFromPath` returns the path of the file being edited (drive-
/// rooted POSIX) so the node view can resolve relative srcs like
/// `../logo.png` against the file's directory before fetching from
/// `/api/files/`. Returning null/empty falls back to drive-root
/// resolution (legacy / no-source-file callers).
export function createImageNode(getFromPath: () => string | null) {
  return Image.extend({
  // The stock `@tiptap/extension-image` input rule converts
  // `![alt](src)` typed text into an image atom as soon as the
  // closing `)` lands. That fights our bubble flow, which
  // intentionally keeps the markdown text in the editor while
  // the user picks an image (so they can search, see a preview,
  // and edit alt). The host commits via setNodeAttribute /
  // replaceWith when the user accepts; until then there's no
  // atom. Drop the rule entirely.
  addInputRules() {
    return [];
  },
  addNodeView() {
    return ({ node, getPos, editor }) => {
      const wrap = document.createElement("span");
      wrap.className = "md-image-wrap";

      const img = document.createElement("img");
      img.draggable = false;
      const apply = (n: { attrs: { src?: unknown; alt?: unknown } }) => {
        const raw = (n.attrs.src as string | null) ?? "";
        if (!raw) {
          // Empty src: leave the `<img>` source-less so the browser
          // doesn't fire a request to `/api/files/` (which 404s
          // and clutters the console). The atom still renders the
          // alt text via the nodeview wrapper, so the user sees a
          // placeholder and can edit it.
          img.removeAttribute("src");
          img.alt = (n.attrs.alt as string | null) ?? "";
          img.style.removeProperty("width");
          return;
        }
        const { src, width } = parseSrcFragment(raw);
        img.src = resolveImageSrc(src, getFromPath());
        img.alt = (n.attrs.alt as string | null) ?? "";
        if (width != null) {
          img.style.width = `${width}px`;
        } else {
          img.style.removeProperty("width");
        }
      };
      apply(node);
      wrap.appendChild(img);

      const handle = document.createElement("span");
      handle.className = "md-image-handle";
      handle.title = "drag to resize";
      wrap.appendChild(handle);

      handle.addEventListener("mousedown", (e) => {
        e.preventDefault();
        e.stopPropagation();
        const startX = e.clientX;
        const startWidth = img.getBoundingClientRect().width;
        wrap.classList.add("resizing");
        const onMove = (ev: MouseEvent) => {
          const delta = ev.clientX - startX;
          // 40 px floor keeps the handle reachable; CSS
          // `max-width: 100%` on the img caps the upper bound
          // to the editor column without an explicit ceiling.
          const newWidth = Math.max(40, Math.round(startWidth + delta));
          img.style.width = `${newWidth}px`;
        };
        const onUp = () => {
          wrap.classList.remove("resizing");
          document.removeEventListener("mousemove", onMove);
          document.removeEventListener("mouseup", onUp);
          const finalWidth = Math.round(img.getBoundingClientRect().width);
          const pos = getPos?.();
          if (typeof pos !== "number") return;
          const current =
            (editor.state.doc.nodeAt(pos)?.attrs.src as string | null) ?? "";
          const nextSrc = buildSrcWithWidth(current, finalWidth);
          if (nextSrc === current) return;
          const tr = editor.state.tr.setNodeAttribute(pos, "src", nextSrc);
          editor.view.dispatch(tr);
        };
        document.addEventListener("mousemove", onMove);
        document.addEventListener("mouseup", onUp);
      });

      return {
        dom: wrap,
        update(updated) {
          if (updated.type !== node.type) return false;
          apply(updated as { attrs: { src?: unknown; alt?: unknown } });
          return true;
        },
        // Block ProseMirror's mutation-observer recursion into
        // the inline-styled img; otherwise drag-resize triggers
        // a re-parse loop that fights the live drag state.
        ignoreMutation() {
          return true;
        },
      };
    };
  },
  renderHTML({ HTMLAttributes }) {
    // Used when ProseMirror serializes the doc to HTML (e.g. copy
    // to clipboard). The node view above owns in-editor display.
    // Strip the width fragment for src and apply width via inline
    // style so a serialized HTML copy still renders right.
    const raw = (HTMLAttributes.src as string | null) ?? "";
    const { src, width } = parseSrcFragment(raw);
    const extra: Record<string, string> = { src: resolveImageSrc(src, getFromPath()) };
    if (width != null) extra.style = `width: ${width}px`;
    return ["img", mergeAttributes(HTMLAttributes, extra)];
  },
}).configure({
  inline: true,
  allowBase64: true,
});
}

/// Relativize a drive-rooted path to the directory of `fromPath`,
/// emitting a `./` or `../` prefixed src. Pass-through when the
/// input is an absolute URL (http/data/blob) or when `fromPath` is
/// null/empty (no source file known).
export function relativizeImageSrc(src: string, fromPath: string | null): string {
  if (!fromPath) return src;
  if (/^(https?:|data:|blob:)/i.test(src)) return src;
  // Strip any width fragment before relativizing so we don't carry
  // `#w=N` through the path math, then re-attach it after.
  const { src: clean, width } = parseSrcFragment(src);
  const rel = relativizePath(clean, fromPath);
  return width != null ? `${rel}#w=${width}` : rel;
}

/// Open the inline image picker anchored at `host`. Resolves with
/// the markdown `src` to insert, or `null` if the user dismisses.
/// `src` is either a relative drive path (e.g.
/// `attachments/2026-...png`) or an absolute URL. The caller is
/// responsible for inserting the node and removing the trigger
/// text from the editor.
///
/// `uploadDir` is the drive-relative directory the upload action
/// targets; when null, the server falls back to its configured
/// attachments_dir. Pass `dirname(currentPath)` so an upload from
/// `Recipes/Pasta.md` lands next to that file.
export function showImagePicker(
  host: HTMLElement,
  pick: (result: { src: string; alt: string } | null) => void,
  uploadDir: string | null = null,
  initial?: { src: string; alt: string },
): void {
  const wrap = document.createElement("div");
  wrap.className = "md-pick md-pick-image";
  wrap.style.position = "absolute";
  // Same z-index as the wiki picker so it floats above any
  // overlay (InlineAssist sits at 25000).
  wrap.style.zIndex = "30000";

  // Search box for filtering existing drive images.
  const search = document.createElement("input");
  search.placeholder = "search images...";
  search.className = "md-pick-input";
  wrap.appendChild(search);

  const list = document.createElement("ul");
  list.className = "md-pick-list";
  wrap.appendChild(list);

  // Footer carries the two non-search actions: upload from disk
  // and paste an external URL. Kept below the list so the
  // keyboard-driven flow (search + arrow-down + enter) is the
  // primary path; the footer is for the rest.
  const footer = document.createElement("div");
  footer.className = "md-pick-footer";
  wrap.appendChild(footer);

  const uploadBtn = document.createElement("button");
  uploadBtn.type = "button";
  uploadBtn.className = "md-pick-action";
  uploadBtn.textContent = "Upload image…";
  footer.appendChild(uploadBtn);

  const urlInput = document.createElement("input");
  urlInput.type = "url";
  urlInput.placeholder = "or paste URL (https://…)";
  urlInput.className = "md-pick-url";
  footer.appendChild(urlInput);

  // Alt-text field: auto-populated from the picked filename, but
  // editable. Commit happens here (Enter), so picking from the
  // list / upload / URL doesn't immediately insert; it stages the
  // src and focuses this field for an optional override.
  const altInput = document.createElement("input");
  altInput.type = "text";
  altInput.placeholder = "alt text";
  altInput.className = "md-pick-alt";
  footer.appendChild(altInput);

  /// Default alt = filename stem of the chosen src. Strip any
  /// query / fragment so URLs like `https://x.test/foo.png?w=1`
  /// give "foo" not "foo?w=1".
  const altFromSrc = (src: string): string => {
    const last = src.split("/").pop() ?? src;
    const clean = last.split("?")[0].split("#")[0] ?? last;
    return clean.replace(/\.[^./]+$/, "");
  };

  /// Path the user has chosen but not yet committed. Committing
  /// is gated on the alt field's Enter so the user always gets a
  /// chance to override the auto-populated alt text.
  let stagedSrc: string | null = null;
  const stageAndFocusAlt = (src: string): void => {
    // Only refresh the alt field when the src actually changes.
    // Picking the same src twice (e.g. opening the picker on an
    // existing image and pressing Enter on the auto-highlighted
    // list match) must not clobber a user-edited alt.
    if (stagedSrc !== src) {
      altInput.value = altFromSrc(src);
    }
    stagedSrc = src;
    altInput.focus();
    altInput.select();
  };

  // "Edit existing" pre-fill. Stages the current src so Enter on
  // alt commits unchanged, fills the alt input, and seeds the
  // search box with the current src so the list highlights /
  // narrows to it. Focus goes to the search input (selected) so
  // the user can immediately type to change the image; if they
  // only want to edit alt, they Tab into it.
  if (initial) {
    stagedSrc = initial.src;
    altInput.value = initial.alt;
    search.value = initial.src;
  }
  const commitStaged = (): void => {
    if (!stagedSrc) return;
    const alt = altInput.value;
    const src = stagedSrc;
    cleanup();
    pick({ src, alt });
  };

  // Hidden file input the upload button forwards to. Lives on
  // document.body, NOT inside `wrap`: on iOS the photo-picker
  // sheet's dismissal can fire a mousedown on the underlying
  // webview which trips `onAway`, removing `wrap` (and any
  // descendant input) from the DOM before `change` arrives. With
  // the input on document.body it stays mounted long enough for
  // the change event to land. The input is also removed by
  // cleanup() so we don't accumulate orphaned <input> nodes.
  const fileInput = document.createElement("input");
  fileInput.type = "file";
  fileInput.accept = IMAGE_EXTS.map((e) => `.${e}`).join(",");
  fileInput.style.display = "none";
  document.body.appendChild(fileInput);
  // Track whether the file dialog is currently open so onAway
  // doesn't dismiss the popover when the OS picker sheet steals
  // focus on iOS. Cleared on either change (success/cancel) or
  // window focus regained.
  let pickInFlight = false;

  let active = 0;
  let entries: string[] = [];

  const renderList = () => {
    list.innerHTML = "";
    entries.forEach((path, i) => {
      const li = document.createElement("li");
      li.textContent = path;
      li.className = i === active ? "active" : "";
      li.onmousedown = (ev) => {
        ev.preventDefault();
        stageAndFocusAlt(path);
      };
      list.appendChild(li);
    });
  };

  /// Image-search runs against the same `/api/files` flat tree
  /// the file browser uses; we filter client-side because the
  /// drive is small enough that it fits in one shot, and
  /// adding a server-side filter just for this picker would
  /// duplicate the indexing surface.
  let allImages: string[] = [];
  let imagesLoaded = false;
  const loadImages = async () => {
    if (imagesLoaded) return;
    try {
      const tree: TreeEntry[] = await api.list();
      allImages = tree
        .filter((e) => !e.is_dir && isImagePath(e.path))
        .map((e) => e.path);
      imagesLoaded = true;
    } catch {
      // Empty list is the worst case; the user can still upload.
      allImages = [];
      imagesLoaded = true;
    }
  };

  const filter = (q: string) => {
    const needle = q.trim().toLowerCase();
    if (!needle) {
      // Empty query shows the most recent images at the top.
      // We don't have an mtime here without re-querying; the
      // /api/files response carries it but the picker discarded
      // it. Cheap proxy: keep the original (alphabetical) order.
      entries = allImages.slice(0, 20);
    } else {
      entries = allImages
        .filter((p) => p.toLowerCase().includes(needle))
        .slice(0, 20);
    }
    active = 0;
    renderList();
  };

  search.addEventListener("input", () => filter(search.value));
  search.addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown") {
      active = Math.min(active + 1, entries.length - 1);
      renderList();
      e.preventDefault();
    } else if (e.key === "ArrowUp") {
      active = Math.max(active - 1, 0);
      renderList();
      e.preventDefault();
    } else if (e.key === "Enter") {
      const sel = entries[active];
      if (sel) {
        stageAndFocusAlt(sel);
      }
      e.preventDefault();
    } else if (e.key === "Escape") {
      cleanup();
      pick(null);
      e.preventDefault();
      // Same rationale as wikiLink: stop the bubble so a parent
      // overlay (InlineAssist) doesn't also dismiss.
      e.stopPropagation();
    }
  });

  uploadBtn.onmousedown = (e) => e.preventDefault();
  uploadBtn.onclick = () => {
    pickInFlight = true;
    fileInput.click();
  };
  fileInput.onchange = async () => {
    pickInFlight = false;
    const file = fileInput.files?.[0];
    if (!file) return;
    uploadBtn.disabled = true;
    uploadBtn.textContent = "uploading…";
    try {
      const { path } = await api.uploadAttachment(file, uploadDir);
      stageAndFocusAlt(path);
      uploadBtn.disabled = false;
      uploadBtn.textContent = "Upload image…";
    } catch (e) {
      uploadBtn.disabled = false;
      uploadBtn.textContent = "Upload image…";
      const msg = (e as Error).message || "upload failed";
      // Surface inline; resetting the button text gives the
      // user another shot.
      footer.title = msg;
    }
  };
  // iOS doesn't always fire `change` when the user cancels the
  // picker sheet; the focus event reliably fires on return so we
  // use it as a fallback to clear the in-flight flag. Without
  // this, an Esc/cancel would leave the popover unable to be
  // dismissed by clicking outside.
  const onWindowFocus = () => {
    // Defer the clear: on iOS the focus event arrives slightly
    // before `change` does, and we need pickInFlight true while
    // change lands so the popover doesn't go away mid-upload.
    setTimeout(() => {
      pickInFlight = false;
    }, 300);
  };
  window.addEventListener("focus", onWindowFocus);

  urlInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      const v = urlInput.value.trim();
      if (v) {
        e.preventDefault();
        stageAndFocusAlt(v);
      }
    } else if (e.key === "Escape") {
      cleanup();
      pick(null);
      e.preventDefault();
      e.stopPropagation();
    }
  });

  altInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      commitStaged();
    } else if (e.key === "Escape") {
      cleanup();
      pick(null);
      e.preventDefault();
      e.stopPropagation();
    }
  });

  const onAway = (ev: MouseEvent) => {
    // While the OS file dialog is open (iOS photo sheet, desktop
    // open-file dialog), the underlying webview can fire spurious
    // mousedowns; treating those as "click outside" tears the
    // popover down before `change` lands and the file is dropped.
    if (pickInFlight) return;
    if (!wrap.contains(ev.target as globalThis.Node)) {
      cleanup();
      pick(null);
    }
  };

  document.body.appendChild(wrap);
  positionPopover(host, wrap);
  const stopWatch = watchViewport(host, wrap);
  const cleanup = () => {
    document.removeEventListener("mousedown", onAway);
    window.removeEventListener("focus", onWindowFocus);
    stopWatch();
    fileInput.remove();
    wrap.remove();
  };
  document.addEventListener("mousedown", onAway);

  void loadImages().then(() => {
    // `search.value` is the seeded query when `initial` was passed;
    // otherwise it's the empty string, matching the old "show all"
    // behaviour on first open.
    filter(search.value);
    if (wrap.isConnected) positionPopover(host, wrap);
  });
  setTimeout(() => {
    search.focus();
    // Pre-select so a keystroke replaces the seeded path; Enter
    // (no changes) is still a valid commit because stagedSrc is
    // already populated when `initial` was passed.
    if (initial) search.select();
  }, 0);
}

/// Upload helper for drag-drop / paste / picker flows. Resolves
/// with the drive-relative path written by the server.
///
/// `dir` is the drive-relative directory to save into. Pass the
/// editing file's directory so the upload lands next to it; pass
/// null to use the server's configured attachments_dir.
export async function uploadImageFile(
  file: File,
  dir: string | null = null,
): Promise<string> {
  const { path } = await api.uploadAttachment(file, dir);
  return path;
}

/**
 * Resolve a markdown image `src` to a URL the WebView can load.
 * Local drive paths get rewritten through `/api/files/` with the
 * token in a `?t=` query (the only way to authenticate `<img src>`,
 * which can't carry an `Authorization` header). Absolute URLs
 * (http/data/blob) pass through.
 *
 * Drive-rooted (`/images/x.png`) and parent-relative (`../x.png`)
 * sources go through `normalizeHref` against `fromPath`'s directory,
 * the same resolver chan-drive uses for graph edges, so a single
 * canonical drive-relative path is what reaches `/api/files/`.
 * Falls back to the literal src if resolution fails (shouldn't
 * happen for a well-formed local image, but keeps the old image
 * loading path if a malformed src slips through).
 */
export function resolveImageSrc(src: string, fromPath?: string | null): string {
  if (/^(https?:|data:|blob:)/i.test(src)) return src;
  const sourceDir = fromPath
    ? fromPath.split("/").slice(0, -1).join("/")
    : "";
  const driveRooted = normalizeHref(src, sourceDir) ?? src;
  // Encode each path segment but keep the slashes; `/api/files/*`
  // accepts the same encoding the file editor uses elsewhere.
  const encoded = driveRooted
    .split("/")
    .map((s) => encodeURIComponent(s))
    .join("/");
  return withTokenQuery(`/api/files/${encoded}`);
}
