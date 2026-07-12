// SPA-global file-drop guard.
//
// Without it, dropping an OS file anywhere the SPA doesn't intercept
// makes the webview navigate into a bare file view with no way back
// (chan-desktop builds with Tauri's drag-drop handler DISABLED so DOM
// DnD keeps working; WKWebView's default drop-navigation then fires on
// any unhandled drop -- and a plain browser tab behaves the same).
//
// Design: the guard acts ONLY on OS file drags (`dataTransfer.types`
// includes "Files"). In-page HTML5 drags -- pane tab moves, the file
// tree's row moves, the editor's image-atom move -- never carry the
// Files type, so their dragover/dropEffect semantics are untouched.
//
// For Files drags:
//   - window `dragover` (capture): always preventDefault (an
//     uncancelled dragover makes the release navigate); outside the
//     allowlisted drop zones additionally set `dropEffect = "none"` so
//     the cursor honestly shows not-allowed and no drop fires there.
//   - window `drop` (capture): preventDefault outside the zones --
//     belt for browsers that fire a drop despite the none effect.
//     Inside a zone the guard does nothing: the zone's own handlers
//     (editor image embed, CodeMirror's native text-file insert, the
//     terminal path-print) keep their behavior, preventDefault
//     included.
//   - window `drop` (bubble): the safety net. Runs after every zone
//     handler; anything still uncancelled (e.g. a read-only editor
//     that ignored the drop) is cancelled here, so no drop ever
//     reaches the webview's default navigation.
//
// A drop zone is any element under `[data-file-drop-zone]` (explicit
// opt-in: the markdown editors, the rich-prompt composer, terminal
// panes) or under `.cm-editor` (every editable CodeMirror instance
// handles file drops natively; read-only ones fall through to the
// bubble net).

const DROP_ZONE_SELECTOR = "[data-file-drop-zone], .cm-editor";

/// True when the drag carries OS files. In-page drags (tab moves,
/// tree moves, image-atom moves) never include the "Files" type.
export function isOsFileDrag(e: DragEvent): boolean {
  const types = e.dataTransfer?.types;
  if (!types) return false;
  // DataTransfer.types is a frozen array in modern browsers but a
  // DOMStringList in older WebKit; Array.prototype.includes works on
  // both via indexOf semantics -- normalise defensively.
  return Array.from(types).includes("Files");
}

/// True when the event target sits inside an allowlisted drop zone.
export function inFileDropZone(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest(DROP_ZONE_SELECTOR) !== null;
}

function onWindowDragOverCapture(e: DragEvent): void {
  if (!isOsFileDrag(e)) return;
  e.preventDefault();
  if (!inFileDropZone(e.target) && e.dataTransfer) {
    e.dataTransfer.dropEffect = "none";
  }
}

function onWindowDropCapture(e: DragEvent): void {
  if (!isOsFileDrag(e)) return;
  if (!inFileDropZone(e.target)) e.preventDefault();
}

function onWindowDropBubble(e: DragEvent): void {
  if (!isOsFileDrag(e)) return;
  if (!e.defaultPrevented) e.preventDefault();
}

/// Install the guard once at App boot. Returns a disposer (tests use
/// it; the App-lifetime install never tears down).
export function installFileDropGuard(win: Window = window): () => void {
  win.addEventListener("dragover", onWindowDragOverCapture, true);
  win.addEventListener("drop", onWindowDropCapture, true);
  win.addEventListener("drop", onWindowDropBubble, false);
  return () => {
    win.removeEventListener("dragover", onWindowDragOverCapture, true);
    win.removeEventListener("drop", onWindowDropCapture, true);
    win.removeEventListener("drop", onWindowDropBubble, false);
  };
}

// ---- terminal path-print escaping -----------------------------------
//
// Desktop-only: the terminal pane's drop handler reads the dropped
// files' absolute paths over the `read_dropped_paths` IPC (the DOM
// File API never exposes OS paths) and types them into the PTY.
// Escaping is POSIX single-quote: wrap in '…', embedded ' becomes
// '\'' (close quote, escaped literal quote, reopen). Multiple paths
// are space-separated with a single trailing space -- Terminal.app
// behavior, so the user can keep typing flags right after.

/// POSIX single-quote escape of one path.
export function escapePosixPath(path: string): string {
  return `'${path.replaceAll("'", "'\\''")}'`;
}

/// The exact string typed into the PTY for a set of dropped paths.
/// Empty input produces an empty string (silent no-op upstream).
export function shellEscapePaths(paths: readonly string[]): string {
  if (paths.length === 0) return "";
  return paths.map(escapePosixPath).join(" ") + " ";
}
