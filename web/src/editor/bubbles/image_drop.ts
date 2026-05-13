// Editor-level drop / paste handlers for image files.
//
// Separate from the image bubble: drop / paste fire on the EditorView
// itself (no `![` typing required). On a successful upload we insert
// `![](path)` at the drop / paste position; the image atom widget
// renders it on the next decoration tick.
//
// This module only handles IMAGE files. Plain text drops / pastes
// fall through to CM6's defaults (markdown text, etc.).

import { EditorView } from "@codemirror/view";
import type { Extension } from "@codemirror/state";
import { api } from "../../api/client";
import { invalidateImageCatalog } from "./image";
import { relativizePath } from "../links";

const MAX_UPLOAD_BYTES = 50 * 1024 * 1024;
const DEFAULT_INSERT_WIDTH_PX = 250;

export interface ImageDropOptions {
  /// Upload destination; defaults to the editing file's directory if
  /// known, otherwise the server's configured attachments_dir. Read
  /// lazily so swapping tabs picks up the new path.
  getUploadDir: () => string | null;
  /// Editing file's drive-rooted path. The uploaded `path` is
  /// relativized against this so the inserted `![](src)` resolves
  /// correctly through resolveImageSrc.
  getCurrentPath: () => string | null;
}

export function imageDropHandlers(opts: ImageDropOptions): Extension {
  return EditorView.domEventHandlers({
    drop(event, view) {
      const files = event.dataTransfer?.files;
      if (!files || files.length === 0) return false;
      const images = Array.from(files).filter((f) => f.type.startsWith("image/"));
      if (images.length === 0) return false;
      event.preventDefault();
      const pos = posFromEvent(view, event);
      uploadAndInsertAll(
        view,
        images,
        pos,
        opts.getUploadDir(),
        opts.getCurrentPath(),
      );
      return true;
    },
    paste(event, view) {
      const items = event.clipboardData?.items;
      if (!items) return false;
      const images: File[] = [];
      for (const item of Array.from(items)) {
        if (item.kind === "file" && item.type.startsWith("image/")) {
          const f = item.getAsFile();
          if (f) images.push(f);
        }
      }
      if (images.length === 0) return false;
      event.preventDefault();
      const pos = view.state.selection.main.head;
      uploadAndInsertAll(
        view,
        images,
        pos,
        opts.getUploadDir(),
        opts.getCurrentPath(),
      );
      return true;
    },
  });
}

function posFromEvent(view: EditorView, event: DragEvent): number {
  const coords = { x: event.clientX, y: event.clientY };
  const pos = view.posAtCoords(coords);
  if (pos !== null) return pos;
  return view.state.selection.main.head;
}

function uploadAndInsertAll(
  view: EditorView,
  files: File[],
  pos: number,
  uploadDir: string | null,
  currentPath: string | null,
): void {
  // Upload sequentially so we can chain the inserts at adjacent
  // positions (each insert shifts subsequent positions; we map
  // through view.state.tr.changes' resolution after each).
  let cursor = pos;
  void (async () => {
    for (const file of files) {
      if (file.size > MAX_UPLOAD_BYTES) continue;
      try {
        const res = await api.uploadAttachment(file, uploadDir);
        invalidateImageCatalog();
        const pathArg = currentPath
          ? relativizePath(res.path, currentPath)
          : res.path;
        // Default new images to 250px wide. The widget reads
        // `#w=N` from the src fragment and clamps via CSS; the
        // user can drag the corner handle to resize. Dropped /
        // pasted images are almost always too big at intrinsic
        // size on a notes page, hence the small default.
        const insert = `![](${pathArg}#w=${DEFAULT_INSERT_WIDTH_PX})\n`;
        view.dispatch({
          changes: { from: cursor, to: cursor, insert },
          selection: { anchor: cursor + insert.length },
        });
        cursor += insert.length;
      } catch (err) {
        console.error("[chan] image upload failed", err);
        return;
      }
    }
  })();
}
