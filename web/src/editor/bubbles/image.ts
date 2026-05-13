// Image bubble for the `![query` trigger.
//
// Renders two action rows + a filtered list of in-drive image files:
//   - "Upload from disk..." opens the OS file picker; on selection,
//     uploads via api.uploadAttachment and commits the returned path.
//   - "Paste from clipboard" only enabled when the clipboard contains
//     an image (we don't pre-check; user-initiated paste handler
//     elsewhere in the editor is the better path for paste).
//   - Filtered list of in-drive images (api.list cached, in-memory
//     filter by query substring on path).
//
// On commit, replaces `![query` with `![](path)`. Alt text is left
// empty for v1; the user can edit it via the source-reveal flow
// (selection-intersect in the image atom widget).
//
// Upload errors render in the status footer; the list stays available
// so the user can fall back to in-drive selection.

import type { EditorView } from "@codemirror/view";
import { openBubbleShell } from "../bubble";
import { createCaretAnchor } from "./anchor";
import type { BubbleHandle } from "./types";
import { api } from "../../api/client";
import { isImagePath } from "../extensions/image";

export interface ImageBubbleOpts {
  view: EditorView;
  triggerStart: number;
  triggerEnd: number;
  initialQuery: string;
  /// Upload destination; defaults to the editing file's directory if
  /// known, otherwise the server's configured attachments_dir.
  uploadDir: string | null;
  onDismiss: () => void;
}

const RESULT_LIMIT = 5;
const MAX_UPLOAD_BYTES = 50 * 1024 * 1024;

interface ImageBubbleHandle extends BubbleHandle {
  setTriggerEnd(end: number): void;
}

let catalogCache: string[] | null = null;
let catalogInflight: Promise<string[]> | null = null;

async function loadImageCatalog(): Promise<string[]> {
  if (catalogCache !== null) return catalogCache;
  if (catalogInflight) return catalogInflight;
  catalogInflight = api
    .list()
    .then((entries) => {
      const out = entries
        .filter((e) => !e.is_dir && isImagePath(e.path))
        .map((e) => e.path)
        .sort((a, b) => a.localeCompare(b));
      catalogCache = out;
      return out;
    })
    .finally(() => {
      catalogInflight = null;
    });
  return catalogInflight;
}

/// Allow callers to invalidate the catalog after uploads land. The
/// cache is module-scoped so subsequent bubble opens see the new file.
export function invalidateImageCatalog(): void {
  catalogCache = null;
}

export function openImageBubble(opts: ImageBubbleOpts): ImageBubbleHandle {
  const anchor = createCaretAnchor(opts.view, opts.triggerStart);
  const shell = openBubbleShell({
    host: anchor.el,
    className: "md-image-bubble cm-bubble",
  });
  let query = opts.initialQuery;
  let triggerEnd = opts.triggerEnd;
  let catalog: string[] = [];
  let hits: string[] = [];
  let selectedIndex = 0;
  let alive = true;

  const actions = document.createElement("div");
  actions.className = "md-bubble-actions";
  shell.wrap.appendChild(actions);
  const uploadBtn = document.createElement("div");
  uploadBtn.className = "md-bubble-row md-bubble-action";
  uploadBtn.textContent = "Upload from disk...";
  uploadBtn.addEventListener("mousedown", (e) => {
    e.preventDefault();
    e.stopPropagation();
    triggerUpload();
  });
  actions.appendChild(uploadBtn);

  const list = document.createElement("div");
  list.className = "md-bubble-list";
  shell.wrap.appendChild(list);
  const status = document.createElement("div");
  status.className = "md-bubble-status";
  shell.wrap.appendChild(status);

  function filter(): void {
    const q = query.toLowerCase();
    hits = q.length === 0
      ? catalog.slice(0, RESULT_LIMIT)
      : catalog.filter((p) => p.toLowerCase().includes(q)).slice(0, RESULT_LIMIT);
    if (selectedIndex >= hits.length) selectedIndex = 0;
    render();
  }

  function render(): void {
    list.innerHTML = "";
    if (hits.length === 0) {
      status.textContent = catalog.length === 0
        ? "Loading images..."
        : query.length === 0
          ? "No images in drive"
          : "No matches";
      shell.reposition();
      return;
    }
    status.textContent = `${hits.length} result${hits.length === 1 ? "" : "s"} · ↵ to insert`;
    for (let i = 0; i < hits.length; i++) {
      const path = hits[i]!;
      const row = document.createElement("div");
      row.className = "md-bubble-row";
      if (i === selectedIndex) row.classList.add("md-bubble-row-selected");
      row.textContent = path;
      row.addEventListener("mousedown", (e) => {
        e.preventDefault();
        e.stopPropagation();
        commitPath(path);
      });
      list.appendChild(row);
    }
    shell.reposition();
  }

  function commitPath(path: string): void {
    const insert = `![](${path})`;
    opts.view.dispatch({
      changes: { from: opts.triggerStart, to: triggerEnd, insert },
      selection: { anchor: opts.triggerStart + insert.length },
    });
    dismiss();
  }

  function triggerUpload(): void {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "image/*";
    input.addEventListener("change", () => {
      const file = input.files?.[0];
      if (!file) return;
      if (file.size > MAX_UPLOAD_BYTES) {
        status.textContent = `File too large (max ${Math.floor(MAX_UPLOAD_BYTES / 1024 / 1024)}MB)`;
        return;
      }
      status.textContent = "Uploading...";
      api
        .uploadAttachment(file, opts.uploadDir)
        .then((res) => {
          if (!alive) return;
          invalidateImageCatalog();
          commitPath(res.path);
        })
        .catch((err) => {
          if (!alive) return;
          status.textContent = `Upload failed: ${err.message ?? err}`;
        });
    });
    input.click();
  }

  function dismiss(): void {
    if (!alive) return;
    alive = false;
    shell.dismiss();
    anchor.dismiss();
    opts.onDismiss();
  }

  loadImageCatalog()
    .then((cat) => {
      if (!alive) return;
      catalog = cat;
      filter();
    })
    .catch((err) => {
      if (!alive) return;
      status.textContent = `Catalog failed: ${err.message ?? err}`;
    });
  filter();

  return {
    handleKey(event) {
      if (event.key === "Escape") {
        dismiss();
        return true;
      }
      if (event.key === "Enter") {
        const path = hits[selectedIndex];
        if (path) {
          commitPath(path);
          return true;
        }
        return false;
      }
      if (event.key === "ArrowDown") {
        if (hits.length === 0) return false;
        selectedIndex = (selectedIndex + 1) % hits.length;
        render();
        return true;
      }
      if (event.key === "ArrowUp") {
        if (hits.length === 0) return false;
        selectedIndex = (selectedIndex - 1 + hits.length) % hits.length;
        render();
        return true;
      }
      return false;
    },
    setQuery(q) {
      if (q === query) return;
      query = q;
      filter();
    },
    setTriggerEnd(end) {
      triggerEnd = end;
    },
    reposition() {
      anchor.update(opts.view, opts.triggerStart);
      shell.reposition();
    },
    dismiss,
  };
}
