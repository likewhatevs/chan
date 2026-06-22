// The unified per-window file-transfer model: one source for the transfer
// bubble that `cs upload` / `cs download` surface. It replaces the split
// upload-status + desktop-download stores so a single bubble shows both kinds,
// bound to the live XHR progress + cancel the API client already exposes.
//
// Per-window + reload survival: the records (minus the live cancel/retry
// handles) plus the bubble's shown/hidden flag persist to sessionStorage keyed
// by sessionWindowId(), mirroring the layout-reload snapshot. A reload destroys
// the in-flight XHR, so on restore an "active" record becomes "interrupted" (a
// terminal state) rather than a frozen progress bar — never a "42% forever" lie.
// A download can be retried from its persisted source; an upload cannot (the
// File bytes do not survive the reload), so it restores Dismiss-only.

import { sessionWindowId } from "../api/client";

export type TransferKind = "upload" | "download";

/// active: in flight. done/cancelled/failed: terminal, this session.
/// interrupted: was in flight when the window reloaded — the XHR is gone.
export type TransferState =
  | "active"
  | "done"
  | "cancelled"
  | "failed"
  | "interrupted";

export interface Transfer {
  id: string;
  kind: TransferKind;
  /// Display name: a single file's name, or "N files" for a multi-file upload.
  filename: string;
  /// 0..1 while a content-length is known; null for an indeterminate transfer.
  progress: number | null;
  state: TransferState;
  error: string | null;
  /// Download success: the saved path (shown in the done row). null otherwise.
  savedPath: string | null;
  /// A download's source, persisted so an interrupted download can be retried
  /// after a reload. null for uploads (the File cannot be persisted).
  source: { path: string; isDir: boolean } | null;
  /// Live abort handle, set only while active. NOT persisted.
  cancel: (() => void) | null;
  /// Live retry handle for an interrupted/failed download, reconstructed on
  /// restore from `source`. NOT persisted.
  retry: (() => void) | null;
}

interface TransfersState {
  items: Transfer[];
  /// The bubble's shown/hidden state (persisted, restored exactly).
  shown: boolean;
}

export const transfers = $state<TransfersState>({ items: [], shown: false });

const STORE_KEY = "chan.transfers";

function storeKey(): string {
  return `${STORE_KEY}:${sessionWindowId()}`;
}

/// The persisted shape: records without the live handles, plus shown/hidden.
interface PersistedTransfer {
  id: string;
  kind: TransferKind;
  filename: string;
  progress: number | null;
  state: TransferState;
  error: string | null;
  savedPath: string | null;
  source: { path: string; isDir: boolean } | null;
}

function persist(): void {
  if (typeof window === "undefined") return;
  try {
    const payload = {
      items: transfers.items.map(
        (t): PersistedTransfer => ({
          id: t.id,
          kind: t.kind,
          filename: t.filename,
          progress: t.progress,
          state: t.state,
          error: t.error,
          savedPath: t.savedPath,
          source: t.source,
        }),
      ),
      shown: transfers.shown,
    };
    window.sessionStorage.setItem(storeKey(), JSON.stringify(payload));
  } catch {
    // sessionStorage unavailable / quota: the bubble degrades to in-memory.
  }
}

let nextId = 1;
function transferId(): string {
  return `xfer-${nextId++}`;
}

function find(id: string): Transfer | undefined {
  return transfers.items.find((t) => t.id === id);
}

/// The count of in-flight transfers in THIS window — the per-window
/// active-transfer signal the desktop close guard queries (over /ws). A window
/// with a non-zero count must not close silently.
export function activeTransferCount(): number {
  return transfers.items.filter((t) => t.state === "active").length;
}

/// The sink that pushes the active-transfer count to the server over the window
/// `/ws` ({"type":"transfers","active":<n>}). `store` registers it against the
/// watch socket; we call it whenever the count could change. null in tests / on
/// a surface with no watch socket.
let signalSink: ((active: number) => void) | null = null;

export function setTransferSignalSink(sink: ((active: number) => void) | null): void {
  signalSink = sink;
}

function emitSignal(): void {
  signalSink?.(activeTransferCount());
}

/// Start tracking a new active transfer; returns its id. `cancel` aborts the
/// in-flight XHR. `source` is set for downloads so a later interrupt can retry.
export function beginTransfer(opts: {
  kind: TransferKind;
  filename: string;
  cancel: (() => void) | null;
  source?: { path: string; isDir: boolean } | null;
}): string {
  const id = transferId();
  transfers.items.push({
    id,
    kind: opts.kind,
    filename: opts.filename,
    progress: null,
    state: "active",
    error: null,
    savedPath: null,
    source: opts.source ?? null,
    cancel: opts.cancel,
    retry: null,
  });
  persist();
  emitSignal();
  return id;
}

export function setTransferProgress(id: string, progress: number | null): void {
  const t = find(id);
  if (!t || t.state !== "active") return;
  t.progress = progress;
  persist();
}

export function finishTransfer(id: string, savedPath: string | null = null): void {
  const t = find(id);
  if (!t) return;
  t.state = "done";
  t.progress = 1;
  t.cancel = null;
  t.retry = null;
  t.savedPath = savedPath;
  persist();
  emitSignal();
}

export function cancelTransfer(id: string): void {
  const t = find(id);
  if (!t) return;
  t.state = "cancelled";
  t.cancel = null;
  persist();
  emitSignal();
}

export function failTransfer(id: string, error: string): void {
  const t = find(id);
  if (!t) return;
  t.state = "failed";
  t.cancel = null;
  t.error = error;
  persist();
  emitSignal();
}

/// Remove a terminal transfer row (the bubble's per-row dismiss).
export function dismissTransfer(id: string): void {
  const i = transfers.items.findIndex((t) => t.id === id);
  if (i < 0) return;
  transfers.items.splice(i, 1);
  persist();
  emitSignal();
}

export function showTransfers(): void {
  transfers.shown = true;
  persist();
}

export function hideTransfers(): void {
  transfers.shown = false;
  persist();
}

export function toggleTransfers(): void {
  transfers.shown = !transfers.shown;
  persist();
}

/// Restore the persisted bubble on boot. Terminal states restore exactly; an
/// "active" record (its XHR died with the reload) restores as "interrupted".
/// `reconstructDownloadRetry` rebuilds the retry handle for an interrupted
/// download from its source (uploads get none — the File is gone).
export function restoreTransfers(
  reconstructDownloadRetry: (source: { path: string; isDir: boolean }) => () => void,
): void {
  if (typeof window === "undefined") return;
  let raw: string | null = null;
  try {
    raw = window.sessionStorage.getItem(storeKey());
  } catch {
    return;
  }
  if (!raw) return;
  let parsed: { items?: PersistedTransfer[]; shown?: boolean };
  try {
    parsed = JSON.parse(raw) as { items?: PersistedTransfer[]; shown?: boolean };
  } catch {
    return;
  }
  const items = Array.isArray(parsed.items) ? parsed.items : [];
  transfers.items = items.map((p): Transfer => {
    const interrupted = p.state === "active";
    const state: TransferState = interrupted ? "interrupted" : p.state;
    const retry =
      interrupted && p.kind === "download" && p.source
        ? reconstructDownloadRetry(p.source)
        : null;
    return {
      id: p.id,
      kind: p.kind,
      filename: p.filename,
      // An interrupted transfer has no meaningful progress; drop the stale
      // fraction so the bar never shows a frozen mid-transfer value.
      progress: interrupted ? null : p.progress,
      state,
      error: p.error,
      savedPath: p.savedPath,
      source: p.source,
      cancel: null,
      retry,
    };
  });
  transfers.shown = parsed.shown === true;
  // After a reload every record is terminal/interrupted (count 0), but emit so
  // the server's per-socket count is correct from the first announce.
  emitSignal();
}
