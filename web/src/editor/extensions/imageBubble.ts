// Image insertion / edit bubble.
//
// Mirrors the wiki bubble's contract: non-focus-stealing popover
// anchored under the caret, the host owns the keyboard, and the
// bubble emits picks via callbacks. Two modes:
//
//   - "path": typing inside `(path)` filters drive images live;
//     ArrowUp/Down browses the result list. The upload button
//     uploads a local file and resolves with the new drive path.
//   - "alt": caret sits inside `[alt]`. The bubble surfaces the
//     current alt text in the place results would otherwise be,
//     so the user gets feedback while editing.
//
// Mode is set by the host (Wysiwyg) based on caret position
// inside the source markdown the editor injects when entering
// edit-existing or `![`-typing mode.

import { api, withTokenQuery } from "../../api/client";
import type { TreeEntry } from "../../api/types";
import { isImagePath } from "./image";
import { positionPopover, watchViewport } from "./popover";

export type ImageBubbleMode = "path" | "alt";

export interface ImageBubbleOpts {
  host: HTMLElement;
  /// Drive-relative directory uploads target (passed to
  /// `api.uploadAttachment`). When null, the server uses its
  /// configured `attachments_dir`.
  uploadDir?: string | null;
  /// Fires when the user clicks a list item. The host commits
  /// the same way it would for an Enter on the active row.
  onClickPick: (src: string) => void;
  /// Fires when an upload completes successfully. The host
  /// inserts the new drive path the same way as a list pick.
  onUpload: (src: string) => void;
}

export interface ImageBubbleHandle {
  /// Switch between "path" (search results visible) and "alt"
  /// (alt-text echo visible). No-op when already in that mode.
  setMode(mode: ImageBubbleMode): void;
  /// Update the path-search query and re-render the list. Safe
  /// to call before the image catalog has loaded; the filter
  /// re-applies once it lands.
  setPathQuery(q: string): void;
  /// Update the alt-text echo. Empty values render as a muted
  /// placeholder so the user sees the slot exists.
  setAlt(text: string): void;
  /// Move the active result in path mode. No-op in alt mode or
  /// when the result list is empty.
  moveActive(delta: number): void;
  /// Resolve the highlighted result, or null when path mode has
  /// no results. The host treats null as "fall back to whatever
  /// the user typed in the editor".
  pickActive(): string | null;
  /// Tear down DOM + listeners. Idempotent.
  dismiss(): void;
}

export function openImageBubble(opts: ImageBubbleOpts): ImageBubbleHandle {
  const wrap = document.createElement("div");
  wrap.className = "md-image-bubble";
  wrap.style.position = "absolute";
  wrap.style.zIndex = "30000";

  // Live preview: thumbnail of the active list entry. Hidden in
  // alt mode and when there are no results. The image hits
  // `/api/files/<path>` with the auth token in a query so it
  // renders inline like the editor's own image atom does.
  const preview = document.createElement("div");
  preview.className = "md-image-bubble-preview is-empty";
  const previewImg = document.createElement("img");
  previewImg.draggable = false;
  preview.appendChild(previewImg);
  wrap.appendChild(preview);

  const list = document.createElement("ul");
  list.className = "md-image-bubble-list";
  wrap.appendChild(list);

  // Alt-text echo row, visible only in "alt" mode. Replaces the
  // result list in the same vertical slot so the bubble doesn't
  // grow when switching modes.
  const altRow = document.createElement("div");
  altRow.className = "md-image-bubble-alt is-hidden";
  const altLabel = document.createElement("span");
  altLabel.className = "md-image-bubble-alt-label";
  altLabel.textContent = "alt:";
  const altValue = document.createElement("span");
  altValue.className = "md-image-bubble-alt-value";
  altRow.appendChild(altLabel);
  altRow.appendChild(altValue);
  wrap.appendChild(altRow);

  const footer = document.createElement("div");
  footer.className = "md-image-bubble-footer";
  const uploadBtn = document.createElement("button");
  uploadBtn.type = "button";
  uploadBtn.className = "md-image-bubble-upload";
  uploadBtn.textContent = "Upload image…";
  footer.appendChild(uploadBtn);
  const acceptHint = document.createElement("span");
  acceptHint.className = "md-image-bubble-accept";
  acceptHint.textContent = "⏎ to accept";
  footer.appendChild(acceptHint);
  wrap.appendChild(footer);

  document.body.appendChild(wrap);
  positionPopover(opts.host, wrap);
  const stopWatch = watchViewport(opts.host, wrap);

  // Hidden file input. Lives on document.body for the same iOS-
  // photo-sheet reason `showImagePicker` documents.
  const fileInput = document.createElement("input");
  fileInput.type = "file";
  fileInput.accept = ".png,.jpg,.jpeg,.gif,.webp,.svg";
  fileInput.style.display = "none";
  document.body.appendChild(fileInput);

  let alive = true;
  let allImages: string[] = [];
  let imagesLoaded = false;
  let entries: string[] = [];
  let active = 0;
  let mode: ImageBubbleMode = "path";
  let currentQuery = "";

  // Upload wiring. preventDefault on mousedown keeps the editor
  // selection intact so the host's commit transaction lands on
  // the same spot the user was editing.
  uploadBtn.addEventListener("mousedown", (e) => e.preventDefault());
  uploadBtn.addEventListener("click", () => {
    fileInput.click();
  });
  fileInput.addEventListener("change", async () => {
    const f = fileInput.files?.[0];
    if (!f) return;
    uploadBtn.disabled = true;
    const previousLabel = uploadBtn.textContent;
    uploadBtn.textContent = "uploading…";
    try {
      const { path } = await api.uploadAttachment(f, opts.uploadDir ?? null);
      if (!alive) return;
      opts.onUpload(path);
    } catch {
      // Reset the button so the user can retry. The host stays
      // open on failure (no commit happens).
      uploadBtn.disabled = false;
      uploadBtn.textContent = previousLabel ?? "Upload image…";
    }
  });

  const renderPreview = (): void => {
    if (mode !== "path") {
      preview.classList.add("is-empty");
      previewImg.removeAttribute("src");
      return;
    }
    const activePath = entries[active];
    if (!activePath) {
      preview.classList.add("is-empty");
      previewImg.removeAttribute("src");
      return;
    }
    preview.classList.remove("is-empty");
    // Drive-rooted paths only here (the catalog filters out non-
    // drive entries earlier); encode each segment and append the
    // auth token so `<img>` can authenticate via query string.
    const encoded = activePath
      .split("/")
      .map(encodeURIComponent)
      .join("/");
    previewImg.src = withTokenQuery(`/api/files/${encoded}`);
  };

  const renderList = (): void => {
    list.innerHTML = "";
    entries.forEach((p, i) => {
      const li = document.createElement("li");
      li.textContent = p;
      if (i === active) li.classList.add("active");
      li.addEventListener("mousedown", (ev) => {
        ev.preventDefault();
        opts.onClickPick(p);
      });
      list.appendChild(li);
    });
    renderPreview();
    if (wrap.isConnected) positionPopover(opts.host, wrap);
  };

  const applyFilter = (q: string): void => {
    const needle = q.trim().toLowerCase();
    if (!needle) {
      entries = allImages.slice(0, 8);
    } else {
      entries = allImages
        .filter((p) => p.toLowerCase().includes(needle))
        .slice(0, 8);
    }
    active = 0;
    renderList();
  };

  const ensureLoaded = (): void => {
    if (imagesLoaded) return;
    void api
      .list()
      .then((tree: TreeEntry[]) => {
        if (!alive) return;
        allImages = tree
          .filter((e) => !e.is_dir && isImagePath(e.path))
          .map((e) => e.path);
        imagesLoaded = true;
        applyFilter(currentQuery);
      })
      .catch(() => {
        if (!alive) return;
        allImages = [];
        imagesLoaded = true;
        applyFilter(currentQuery);
      });
  };

  ensureLoaded();
  // First paint with whatever cache we have (likely empty until
  // the fetch lands).
  applyFilter("");

  return {
    setMode(m): void {
      if (mode === m) return;
      mode = m;
      if (mode === "path") {
        list.classList.remove("is-hidden");
        altRow.classList.add("is-hidden");
      } else {
        list.classList.add("is-hidden");
        altRow.classList.remove("is-hidden");
      }
      renderPreview();
      if (wrap.isConnected) positionPopover(opts.host, wrap);
    },
    setPathQuery(q): void {
      currentQuery = q;
      if (imagesLoaded) applyFilter(q);
    },
    setAlt(text): void {
      if (text) {
        altValue.textContent = text;
        altValue.classList.remove("is-empty");
      } else {
        altValue.textContent = "(empty — Enter to use filename)";
        altValue.classList.add("is-empty");
      }
    },
    moveActive(delta): void {
      if (!alive || mode !== "path" || entries.length === 0) return;
      active = Math.max(0, Math.min(entries.length - 1, active + delta));
      renderList();
    },
    pickActive(): string | null {
      if (mode !== "path" || entries.length === 0) return null;
      return entries[active] ?? null;
    },
    dismiss(): void {
      if (!alive) return;
      alive = false;
      stopWatch();
      wrap.remove();
      fileInput.remove();
    },
  };
}
