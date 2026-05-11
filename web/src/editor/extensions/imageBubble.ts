// `![alt](src)` image bubble.
//
// Mirrors the wiki bubble's non-focus-stealing popover pattern. Two
// modes track the caret position inside the `![alt](src)` source:
//
//   - "path": caret inside `(src)`. Shows a filterable list of the
//     drive's images plus a thumbnail preview of the highlighted
//     entry. Enter / click commits the chosen path.
//
//   - "alt": caret inside `[alt]`. Shows a single "alt: <typed text>"
//     echo row in place of the list so the user has visible feedback
//     while typing the alt attribute.
//
// Both modes share a footer carrying an upload button (left) and a
// keyboard hint (right). An error row above the footer surfaces
// upload failures (size cap, server error) without dismissing the
// bubble.
//
// The bubble doesn't take focus: the host (Wysiwyg.svelte) drives
// the keyboard through `BubbleHandle.handleKey`. The user's caret
// stays inside `![alt](src)` and their typing IS the search query.
// Enter on the path mode commits the highlighted result; Enter on
// the alt mode is forwarded to `opts.onCommit` so the host can fall
// through to the path's current src.

import { api, withTokenQuery } from "../../api/client";
import { openBubbleShell, type BubbleHandle } from "../bubble";
import { isImagePath, type ImageAlign } from "./image";

/// Server-side size cap. Mirrors the upload limit chan-server applies
/// to `/api/attachments` so the pre-flight check fails fast instead
/// of waiting for a 413 round-trip. Keep in sync with the server.
const MAX_UPLOAD_BYTES = 50 * 1024 * 1024;

export interface ImageBubbleOpts {
  /// Anchor element. Pass the caret-anchor shim so the wrap sits
  /// under the cursor.
  host: HTMLElement;
  /// Drive-relative directory to upload new images into. Null falls
  /// back to the server's configured `attachments_dir`. The host
  /// typically passes the editing file's directory so uploads land
  /// next to the note.
  uploadDir?: string | null;
  /// Fires when the user clicks a result row in the list. The host
  /// rewrites the `(src)` portion of the markdown to the picked
  /// path; the alt and the rest of the range stay intact.
  onClickPick: (src: string) => void;
  /// Fires after a successful upload, with the drive-relative path
  /// the server saved. Same insertion contract as `onClickPick`.
  onUpload: (src: string) => void;
  /// Fires on Enter. The host runs its accept path (replace the
  /// `![alt](src)` text with an image atom).
  onCommit?: () => void;
  /// Fires on Escape. The host runs its dismiss path (which may
  /// restore the original atom if the bubble was opened in
  /// edit-existing mode).
  onDismiss?: () => void;
  /// Fires when the user clicks an alignment button. `null` clears
  /// the fragment (default, centered); `"left"` / `"right"` set the
  /// matching bare-token fragment on the current `(src)` portion.
  /// The host rewrites the markdown text in place.
  onSetAlign?: (align: ImageAlign | null) => void;
}

export type ImageBubbleMode = "path" | "alt";

export interface ImageBubble extends BubbleHandle {
  /// Toggle between path-search and alt-text-echo modes. Called by
  /// the host's sync hook as the caret crosses the `[alt]` / `(src)`
  /// boundary.
  setMode(mode: ImageBubbleMode): void;
  /// Update the substring filter against the cached image catalog.
  /// Empty query shows the first 8 entries.
  setPathQuery(q: string): void;
  /// Update the alt-mode echo row. Empty value renders a muted hint
  /// telling the user Enter will fall back to the filename.
  setAlt(text: string): void;
  /// Move the active result selection by `delta` (+1 / -1) within
  /// the rendered results.
  moveActive(delta: number): void;
  /// Resolve the currently-highlighted path, or null if no results.
  /// The caller commits by replacing the `(src)` range with the
  /// returned string.
  accept(): string | null;
  /// True while a file is in flight to /api/attachments. The host's
  /// sync hook checks this so a selection-update fired by the OS
  /// file picker returning focus doesn't dismiss the bubble before
  /// the upload completes (which would delete the `![]()` markup
  /// the upload was supposed to fill in).
  isUploading(): boolean;
  /// Sync the visual `is-active` state on the alignment buttons to
  /// the alignment fragment currently parsed off the markdown src.
  /// `null` highlights the centered (default) button.
  setActiveAlign(align: ImageAlign | null): void;
  /// Enable / disable the alignment buttons. Disabled when the image
  /// shares its textblock with other content: aligning would either
  /// be a no-op (the wrap is still inline-block on a mixed line) or
  /// break paragraph flow, so the affordance is hidden instead.
  setAlignAvailable(available: boolean): void;
  /// Tear down DOM + listeners. Idempotent.
  dismiss(): void;
}

interface CatalogEntry {
  path: string;
}

/// URI-encode a drive-relative path one segment at a time, keeping
/// the `/` separators intact. Same shape `resolveImageSrc` uses so
/// the preview hits the same `/api/files/...` URL the editor would
/// load if the atom were already inserted.
function encodePath(path: string): string {
  return path
    .split("/")
    .map((seg) => encodeURIComponent(seg))
    .join("/");
}

export function openImageBubble(opts: ImageBubbleOpts): ImageBubble {
  const shell = openBubbleShell({
    host: opts.host,
    className: "md-image-bubble",
  });
  const { wrap } = shell;

  // Preview slot: img inside a wrap so we can hide the whole block
  // with one toggle. Shown in path mode when there's an active
  // result; hidden in alt mode and when results are empty.
  const preview = document.createElement("div");
  preview.className = "md-image-bubble-preview is-hidden";
  const previewImg = document.createElement("img");
  preview.appendChild(previewImg);
  wrap.appendChild(preview);

  // Result list. Each row is the drive-relative path; mousedown
  // commits via `onClickPick` (preventDefault keeps editor focus).
  const list = document.createElement("ul");
  list.className = "md-image-bubble-list";
  wrap.appendChild(list);

  // Alt-mode echo row. Hidden in path mode. Two children: a small
  // "alt:" label and a live value span.
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

  // Error row. Hidden by default; surfaced when an upload fails.
  // Sits above the footer so it doesn't move the keyboard hint.
  const error = document.createElement("div");
  error.className = "md-image-bubble-error is-hidden";
  wrap.appendChild(error);

  // Footer: upload button (left) + alignment group (middle) + accept
  // hint (right). The hidden <input type="file"> lives on
  // document.body so closing the bubble doesn't tear it down
  // mid-dialog.
  const footer = document.createElement("div");
  footer.className = "md-image-bubble-footer";
  const uploadBtn = document.createElement("button");
  uploadBtn.type = "button";
  uploadBtn.className = "md-image-bubble-upload";
  uploadBtn.textContent = "Upload…";
  // mousedown preventDefault keeps the editor's selection alive
  // through the click; the OS file picker would otherwise steal
  // focus and ProseMirror would collapse the selection.
  uploadBtn.addEventListener("mousedown", (ev) => {
    ev.preventDefault();
  });

  // Alignment group. Three buttons (left / center / right) that
  // toggle the `#left` / `#right` fragment on `(src)`. Centered =
  // no fragment, which keeps the markdown clean for the common case.
  // The fragment never reaches the graph index: chan-drive's link
  // parser strips `#...` before resolving paths.
  const alignGroup = document.createElement("div");
  alignGroup.className = "md-image-bubble-align";
  const makeAlignBtn = (
    title: string,
    label: string,
    handler: () => void,
  ): HTMLButtonElement => {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "md-image-bubble-align-btn";
    btn.title = title;
    btn.textContent = label;
    btn.addEventListener("mousedown", (ev) => {
      ev.preventDefault();
    });
    btn.addEventListener("click", handler);
    return btn;
  };
  const alignLeftBtn = makeAlignBtn("align left (#left)", "⇤", () => {
    opts.onSetAlign?.("left");
  });
  const alignCenterBtn = makeAlignBtn("center (default)", "↔", () => {
    opts.onSetAlign?.(null);
  });
  const alignRightBtn = makeAlignBtn("align right (#right)", "⇥", () => {
    opts.onSetAlign?.("right");
  });
  alignGroup.appendChild(alignLeftBtn);
  alignGroup.appendChild(alignCenterBtn);
  alignGroup.appendChild(alignRightBtn);

  // OK button: explicit commit affordance for mouse users, matching
  // the calendar bubble pattern. Same effect as pressing Enter; the
  // keyboard-shortcut hint is folded into the title attribute so
  // the footer doesn't grow a third row.
  const okBtn = document.createElement("button");
  okBtn.type = "button";
  okBtn.className = "md-image-bubble-ok";
  okBtn.textContent = "OK";
  okBtn.title = "insert the highlighted image (Enter)";
  okBtn.addEventListener("mousedown", (ev) => {
    ev.preventDefault();
  });
  okBtn.addEventListener("click", () => {
    opts.onCommit?.();
  });
  footer.appendChild(uploadBtn);
  footer.appendChild(alignGroup);
  footer.appendChild(okBtn);
  wrap.appendChild(footer);

  // Hidden file input. Re-used across uploads; reset before each
  // open so a re-pick of the same file still fires `change`.
  const fileInput = document.createElement("input");
  fileInput.type = "file";
  fileInput.accept = "image/*";
  fileInput.style.display = "none";
  document.body.appendChild(fileInput);

  let mode: ImageBubbleMode = "path";
  let catalog: CatalogEntry[] = [];
  let catalogLoaded = false;
  let entries: CatalogEntry[] = [];
  let active = 0;
  let lastQuery = "";
  let alive = true;
  /// Upload-in-flight flag. Read by the host's syncImageBubble so a
  /// selection update fired by the OS file picker returning focus
  /// can't dismiss the bubble and delete the `![]()` markup before
  /// the response lands.
  let uploading = false;
  /// True when keyboard focus is parked on the upload button
  /// (ArrowDown past the last result lands here). The button gets
  /// an `is-active` class for visual feedback; Enter triggers the
  /// file picker instead of committing the highlighted result.
  let uploadFocused = false;

  const renderUploadFocus = (): void => {
    uploadBtn.classList.toggle("is-active", uploadFocused);
  };

  const setError = (msg: string | null): void => {
    if (!msg) {
      error.classList.add("is-hidden");
      error.textContent = "";
    } else {
      error.classList.remove("is-hidden");
      error.textContent = msg;
    }
    shell.reposition();
  };

  const renderPreview = (): void => {
    if (mode !== "path" || entries.length === 0) {
      preview.classList.add("is-hidden");
      previewImg.removeAttribute("src");
      return;
    }
    const entry = entries[active] ?? entries[0];
    if (!entry) {
      preview.classList.add("is-hidden");
      return;
    }
    const url = withTokenQuery(`/api/files/${encodePath(entry.path)}`);
    previewImg.src = url;
    preview.classList.remove("is-hidden");
  };

  const renderList = (): void => {
    list.innerHTML = "";
    if (mode !== "path") {
      list.classList.add("is-hidden");
      return;
    }
    list.classList.remove("is-hidden");
    if (entries.length === 0) {
      list.classList.add("is-empty");
      okBtn.classList.add("is-hidden");
      shell.reposition();
      return;
    }
    list.classList.remove("is-empty");
    okBtn.classList.remove("is-hidden");
    entries.forEach((entry, i) => {
      const li = document.createElement("li");
      li.textContent = entry.path;
      if (i === active) li.classList.add("active");
      li.addEventListener("mousedown", (ev) => {
        ev.preventDefault();
        active = i;
        opts.onClickPick(entry.path);
      });
      list.appendChild(li);
    });
    shell.reposition();
  };

  const renderAlt = (value: string): void => {
    if (mode !== "alt") {
      altRow.classList.add("is-hidden");
      return;
    }
    altRow.classList.remove("is-hidden");
    if (value.trim().length === 0) {
      altValue.classList.add("is-empty");
      altValue.textContent = "(empty — Enter to use filename)";
    } else {
      altValue.classList.remove("is-empty");
      altValue.textContent = value;
    }
    shell.reposition();
  };

  const renderAll = (): void => {
    renderList();
    renderPreview();
    if (mode === "path") {
      altRow.classList.add("is-hidden");
    }
    shell.reposition();
  };

  const filterCatalog = (q: string): CatalogEntry[] => {
    if (!catalogLoaded) return [];
    const trimmed = q.trim();
    if (!trimmed) return catalog.slice(0, 8);
    const lc = trimmed.toLowerCase();
    return catalog.filter((e) => e.path.toLowerCase().includes(lc)).slice(0, 8);
  };

  const refreshPathResults = (): void => {
    entries = filterCatalog(lastQuery);
    active = 0;
    renderAll();
  };

  // Load the catalog once on open. The list is small enough (a
  // drive's image set) that filtering in memory beats per-keystroke
  // round-trips, and it sidesteps the catalog endpoint's lack of
  // a media-type filter.
  void (async () => {
    try {
      const tree = await api.list();
      if (!alive) return;
      catalog = tree
        .filter((e) => !e.is_dir && isImagePath(e.path))
        .map((e) => ({ path: e.path }))
        .sort((a, b) => a.path.localeCompare(b.path));
      catalogLoaded = true;
      if (mode === "path") refreshPathResults();
    } catch {
      if (!alive) return;
      catalog = [];
      catalogLoaded = true;
      if (mode === "path") refreshPathResults();
    }
  })();

  // Wire upload. The click is dispatched programmatically from the
  // button's click event so the mousedown preventDefault above keeps
  // the editor selection alive while the OS dialog opens.
  uploadBtn.addEventListener("click", () => {
    setError(null);
    fileInput.value = "";
    fileInput.click();
  });
  fileInput.addEventListener("change", () => {
    const file = fileInput.files?.[0];
    if (!file) return;
    if (file.size > MAX_UPLOAD_BYTES) {
      const mb = (file.size / (1024 * 1024)).toFixed(1);
      setError(`file too large: ${mb} MB (max 50 MB)`);
      fileInput.value = "";
      return;
    }
    uploading = true;
    uploadBtn.disabled = true;
    const prevLabel = uploadBtn.textContent;
    uploadBtn.textContent = "uploading…";
    void api
      .uploadAttachment(file, opts.uploadDir ?? null)
      .then((res) => {
        if (!alive) return;
        uploadBtn.disabled = false;
        uploadBtn.textContent = prevLabel ?? "Upload…";
        fileInput.value = "";
        // Hand the path off to the host BEFORE clearing the
        // upload-in-flight flag. The host's onUpload synchronously
        // dispatches replaceImagePathInSource + acceptImageBubble;
        // those trigger onUpdate cycles whose syncImageBubble
        // reads `isUploading()`. Without the guard, the caret
        // moving past the `(...)` after the text replace would
        // see mode=outside and call dismiss → restore → delete
        // the markup we just rewrote. Clearing the flag after
        // onUpload returns keeps both transactions guarded.
        opts.onUpload(res.path);
        uploading = false;
      })
      .catch((e: unknown) => {
        if (!alive) return;
        uploading = false;
        uploadBtn.disabled = false;
        uploadBtn.textContent = prevLabel ?? "Upload…";
        fileInput.value = "";
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
      });
  });

  // Initial paint
  renderAll();

  return {
    setMode(next: ImageBubbleMode): void {
      if (!alive) return;
      if (mode === next) return;
      mode = next;
      uploadFocused = false;
      renderUploadFocus();
      renderAll();
    },
    setPathQuery(q: string): void {
      if (!alive) return;
      lastQuery = q;
      if (mode !== "path") return;
      uploadFocused = false;
      renderUploadFocus();
      refreshPathResults();
    },
    setAlt(text: string): void {
      if (!alive) return;
      if (mode !== "alt") return;
      renderAlt(text);
    },
    moveActive(delta: number): void {
      if (!alive) return;
      if (mode !== "path") return;
      // ArrowDown past the last result (or with no results) parks
      // focus on the upload button so the user can reach it without
      // the mouse. ArrowUp lifts focus back into the list.
      if (uploadFocused) {
        if (delta < 0) {
          uploadFocused = false;
          if (entries.length > 0) active = entries.length - 1;
          renderUploadFocus();
          renderList();
          renderPreview();
        }
        return;
      }
      if (entries.length === 0) {
        if (delta > 0) {
          uploadFocused = true;
          renderUploadFocus();
        }
        return;
      }
      if (delta > 0 && active === entries.length - 1) {
        uploadFocused = true;
        renderUploadFocus();
        return;
      }
      active = Math.max(0, Math.min(entries.length - 1, active + delta));
      renderList();
      renderPreview();
    },
    accept(): string | null {
      if (!alive) return null;
      if (mode !== "path") return null;
      if (entries.length === 0) return null;
      const entry = entries[active] ?? entries[0];
      return entry ? entry.path : null;
    },
    isUploading(): boolean {
      return uploading;
    },
    setActiveAlign(align: ImageAlign | null): void {
      if (!alive) return;
      alignLeftBtn.classList.toggle("is-active", align === "left");
      alignCenterBtn.classList.toggle("is-active", align === null);
      alignRightBtn.classList.toggle("is-active", align === "right");
    },
    setAlignAvailable(available: boolean): void {
      if (!alive) return;
      alignGroup.classList.toggle("is-disabled", !available);
      const title = available
        ? null
        : "alignment unavailable: image shares the line with text";
      for (const btn of [alignLeftBtn, alignCenterBtn, alignRightBtn]) {
        btn.disabled = !available;
        if (title) btn.title = title;
        else if (btn === alignLeftBtn) btn.title = "align left (#left)";
        else if (btn === alignCenterBtn) btn.title = "center (default)";
        else btn.title = "align right (#right)";
      }
    },
    dismiss(): void {
      if (!alive) return;
      alive = false;
      fileInput.remove();
      shell.dismiss();
    },
    handleKey(event: KeyboardEvent): boolean {
      if (!alive) return false;
      switch (event.key) {
        case "Enter":
          if (uploadFocused) {
            // Trigger the file picker rather than commit. The user
            // navigated to the upload slot; Enter there means "open
            // the upload dialog".
            uploadBtn.click();
            return true;
          }
          opts.onCommit?.();
          return true;
        case "Escape":
          opts.onDismiss?.();
          return true;
        case "ArrowDown":
          this.moveActive(1);
          return true;
        case "ArrowUp":
          this.moveActive(-1);
          return true;
      }
      return false;
    },
  };
}
