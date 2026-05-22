/// `fullstack-a-72` editor hang-recovery via localStorage buffer.
/// Persists unsaved editor content per-tab so a forced reload
/// (when the SPA hangs and the user has to close-and-reopen the
/// window) doesn't lose unsaved data.
///
/// Mechanism:
///
/// * On every editor mutation (debounced ~500ms),
///   `writeEditorBuffer(tabId, content)` stores
///   `{ content, updatedAt, path }` under
///   `chan:editor-buffer:<tabId>`.
/// * On editor mount, `readEditorBuffer(tabId)` returns the
///   stored buffer (or null). FileEditorTab compares the buffer
///   content to the disk content (`tab.saved`) — divergent =
///   surface a restore banner.
/// * On save success / tab close / discard,
///   `clearEditorBuffer(tabId)` removes the entry.
/// * On app load, `pruneEditorBuffers()` evicts entries beyond
///   `MAX_BUFFER_AGE_MS` (TTL) and total-size cap
///   (`MAX_BUFFER_BYTES`).
///
/// localStorage SSR-safety: every entry point gates on
/// `typeof localStorage !== "undefined"` so unit tests (vitest
/// node env) + SSR builds don't blow up. Returns null / no-op on
/// unavailable storage.

const BUFFER_KEY_PREFIX = "chan:editor-buffer:";
/// 7-day TTL — stale buffers from forgotten tabs evict on next
/// page load so localStorage doesn't accumulate forever.
const MAX_BUFFER_AGE_MS = 7 * 24 * 60 * 60 * 1000;
/// 10MB total cap across all editor buffers. localStorage quota
/// is typically 5-10MB per origin; keeping below the lower bound
/// avoids quota errors on busy tabs.
const MAX_BUFFER_BYTES = 10 * 1024 * 1024;

export interface EditorBuffer {
  /// Unsaved content as of the last debounced write.
  content: string;
  /// Wall-clock ms since epoch of the last write. Drives the
  /// TTL eviction + the restore-banner "from <relative time>"
  /// affordance (future polish).
  updatedAt: number;
  /// Drive-relative path of the tab when the buffer was
  /// written. Used to invalidate buffers if the user reopens
  /// the tab against a different path (unlikely but defensive
  /// — tab id collisions across drives shouldn't surface
  /// stale content from a different file).
  path: string;
}

function isStorageAvailable(): boolean {
  if (typeof localStorage === "undefined") return false;
  try {
    const probe = "__chan_editor_buffer_probe__";
    localStorage.setItem(probe, "1");
    localStorage.removeItem(probe);
    return true;
  } catch {
    return false;
  }
}

/// `fullstack-a-74` queued-write registry. Each
/// `queueBufferWrite` call replaces the pending write for that
/// tab id; `flushPendingBufferWrites` (typically invoked from a
/// `beforeunload` / `pagehide` handler in App.svelte) drains the
/// registry synchronously so a force-reload doesn't lose the
/// last 500ms of edits.
///
/// The mechanism deliberately lives in this module (not in the
/// FileEditorTab component) so the flush callback can run
/// independently of Svelte's component-lifecycle teardown,
/// which `window.location.reload()` skips. The mount-side
/// debounce in FileEditorTab pre-`-a-74` only flushed in the
/// component-cleanup return; force-reload doesn't run cleanups,
/// so the last 500ms of edits silently vanished.
interface PendingWrite {
  content: string;
  path: string;
  timer: ReturnType<typeof setTimeout> | null;
}
const pendingWrites = new Map<string, PendingWrite>();

const QUEUED_WRITE_DEBOUNCE_MS = 500;

/// Schedule a debounced buffer write for `tabId`. Calling
/// repeatedly for the same `tabId` cancels the prior pending
/// timer + reschedules with the latest content. The callback
/// writes via `writeEditorBuffer` so quota retries + SSR-safety
/// behave exactly the same as direct calls.
export function queueBufferWrite(
  tabId: string,
  content: string,
  path: string,
): void {
  const existing = pendingWrites.get(tabId);
  if (existing?.timer !== null && existing?.timer !== undefined) {
    clearTimeout(existing.timer);
  }
  const entry: PendingWrite = { content, path, timer: null };
  pendingWrites.set(tabId, entry);
  entry.timer = setTimeout(() => {
    writeEditorBuffer(tabId, entry.content, entry.path);
    pendingWrites.delete(tabId);
  }, QUEUED_WRITE_DEBOUNCE_MS);
}

/// Cancel any pending debounced write for `tabId`. Used by
/// FileEditorTab on graceful unmount (Cmd+W / tab close) or
/// when the editor transitions back to clean state.
export function cancelPendingBufferWrite(tabId: string): void {
  const entry = pendingWrites.get(tabId);
  if (entry?.timer !== null && entry?.timer !== undefined) {
    clearTimeout(entry.timer);
  }
  pendingWrites.delete(tabId);
}

/// Synchronously flush every pending debounced write. Intended
/// for the `beforeunload` / `pagehide` window-event handlers so
/// a force-reload (`window.location.reload()`, browser refresh,
/// Cmd+W on the chan-desktop window) persists the in-flight
/// buffers BEFORE the page tears down. Cancels all timers +
/// writes synchronously; returns the number of entries flushed.
export function flushPendingBufferWrites(): number {
  let flushed = 0;
  for (const [tabId, entry] of pendingWrites) {
    if (entry.timer !== null) clearTimeout(entry.timer);
    writeEditorBuffer(tabId, entry.content, entry.path);
    flushed += 1;
  }
  pendingWrites.clear();
  return flushed;
}

export function bufferKey(tabId: string): string {
  return `${BUFFER_KEY_PREFIX}${tabId}`;
}

export function writeEditorBuffer(
  tabId: string,
  content: string,
  path: string,
): void {
  if (!isStorageAvailable()) return;
  const entry: EditorBuffer = { content, updatedAt: Date.now(), path };
  try {
    localStorage.setItem(bufferKey(tabId), JSON.stringify(entry));
  } catch (e) {
    // Quota exceeded or other storage error. Best-effort: try a
    // prune-then-retry once. If the retry also fails, give up
    // silently — losing the buffer is bad but throwing in the
    // editor mutation path is worse.
    pruneEditorBuffers();
    try {
      localStorage.setItem(bufferKey(tabId), JSON.stringify(entry));
    } catch {
      console.warn("[chan] editorBuffer: storage write failed", e);
    }
  }
}

export function readEditorBuffer(tabId: string): EditorBuffer | null {
  if (!isStorageAvailable()) return null;
  const raw = localStorage.getItem(bufferKey(tabId));
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Partial<EditorBuffer>;
    if (
      typeof parsed?.content !== "string" ||
      typeof parsed?.updatedAt !== "number" ||
      typeof parsed?.path !== "string"
    ) {
      // Malformed entry from a prior schema or hand-edited
      // storage. Clear + return null so the editor doesn't try
      // to restore garbage.
      localStorage.removeItem(bufferKey(tabId));
      return null;
    }
    return {
      content: parsed.content,
      updatedAt: parsed.updatedAt,
      path: parsed.path,
    };
  } catch {
    localStorage.removeItem(bufferKey(tabId));
    return null;
  }
}

export function clearEditorBuffer(tabId: string): void {
  if (!isStorageAvailable()) return;
  localStorage.removeItem(bufferKey(tabId));
}

/// Eviction sweep. Runs at app load + on any quota-exceeded
/// write retry. Two passes:
///
///   1. TTL: drop entries older than MAX_BUFFER_AGE_MS.
///   2. Size cap: if remaining entries exceed MAX_BUFFER_BYTES
///      total, drop oldest until under the cap.
///
/// Returns the number of evicted entries (for tests + ops
/// telemetry).
export function pruneEditorBuffers(): number {
  if (!isStorageAvailable()) return 0;
  const now = Date.now();
  const entries: Array<{ key: string; updatedAt: number; bytes: number }> = [];
  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i);
    if (!key || !key.startsWith(BUFFER_KEY_PREFIX)) continue;
    const raw = localStorage.getItem(key);
    if (!raw) continue;
    try {
      const parsed = JSON.parse(raw) as Partial<EditorBuffer>;
      if (typeof parsed?.updatedAt !== "number") continue;
      entries.push({
        key,
        updatedAt: parsed.updatedAt,
        bytes: raw.length,
      });
    } catch {
      // Corrupt entry counts as immediately-evictable.
      entries.push({ key, updatedAt: 0, bytes: raw.length });
    }
  }
  let evicted = 0;
  // Pass 1: TTL.
  for (const e of entries) {
    if (now - e.updatedAt > MAX_BUFFER_AGE_MS) {
      localStorage.removeItem(e.key);
      evicted += 1;
    }
  }
  // Pass 2: size cap (oldest-first eviction).
  const remaining = entries.filter((e) => now - e.updatedAt <= MAX_BUFFER_AGE_MS);
  let totalBytes = remaining.reduce((sum, e) => sum + e.bytes, 0);
  if (totalBytes <= MAX_BUFFER_BYTES) return evicted;
  remaining.sort((a, b) => a.updatedAt - b.updatedAt);
  for (const e of remaining) {
    if (totalBytes <= MAX_BUFFER_BYTES) break;
    localStorage.removeItem(e.key);
    totalBytes -= e.bytes;
    evicted += 1;
  }
  return evicted;
}

/// Convenience helper used by FileEditorTab: returns the buffer
/// when divergent vs the disk content (i.e. the user has
/// unsaved changes that are NOT reflected in `tab.saved`).
/// Returns null when there's no buffer, when the buffer matches
/// the disk content (clean state), or when the buffer's path
/// doesn't match the tab's current path (defensive — wrong-file
/// content shouldn't restore).
export function divergentBufferOrNull(
  tabId: string,
  tabPath: string,
  diskContent: string,
): EditorBuffer | null {
  const buf = readEditorBuffer(tabId);
  if (!buf) return null;
  if (buf.path !== tabPath) {
    clearEditorBuffer(tabId);
    return null;
  }
  if (buf.content === diskContent) return null;
  return buf;
}
