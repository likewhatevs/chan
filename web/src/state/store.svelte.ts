// Global app state, written with Svelte 5 runes ($state).
// One module-level singleton per concern; components import them directly.

import type { IndexStatus, LlmMessage, TreeEntry, DriveInfo } from "../api/types";
import { api, assistantHash16, openWatchSocket, type WsStatus } from "../api/client";
import {
  closeTab,
  layout,
  openInActivePane,
  restoreLayout,
  serializeLayout,
} from "./tabs.svelte";
import { isEditableText } from "./fileTypes";
import { setNotifyHandler } from "./notify.svelte";
import {
  availableScopeOptions,
  defaultScopeId,
  type ScopeOption,
} from "./scope.svelte";
import { refreshTabFromDisk, tabsForPath } from "./tabs.svelte";
export const drive = $state<{ info: DriveInfo | null }>({ info: null });

export const tree = $state<{ entries: TreeEntry[]; loading: boolean }>({
  entries: [],
  loading: false,
});

/**
 * Theme model.
 *
 *   - `themeChoice` is what the user picked: "system" follows the OS
 *     prefers-color-scheme media query live, "light" / "dark" lock
 *     the editor to that mode regardless of OS.
 *   - `theme` is the resolved value applied to the DOM (always one of
 *     "light" | "dark"). Components read this for variant styling.
 *
 * Persisted in the global config (`~/.chan/config.toml`), not in
 * localStorage. PATCH /api/config emits a `config_changed` WS event
 * so a flip in one window propagates to every other open window
 * live; that channel also survives WebView storage wipes.
 */
export type ThemeChoice = "system" | "light" | "dark";

function systemTheme(): "light" | "dark" {
  if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return "dark";
}

function effectiveTheme(choice: ThemeChoice): "light" | "dark" {
  return choice === "system" ? systemTheme() : choice;
}

export const ui = $state<{
  status: string | null;
  /// Used to nudge tabs to reload on external changes.
  lastWatch: number;
  ws: WsStatus;
  /// User's pick. Mirrored from the global config; written through
  /// `setThemeChoice`.
  themeChoice: ThemeChoice;
  /// Resolved value applied to `document.documentElement[data-theme]`.
  theme: "light" | "dark";
}>({
  status: null,
  lastWatch: 0,
  ws: "connecting",
  themeChoice: "system",
  theme: effectiveTheme("system"),
});

// Route leaf-module notify() calls to the shared status line.
setNotifyHandler((msg) => {
  ui.status = msg;
});

/** Apply the resolved theme to the DOM. Idempotent; safe to call
 *  before mount (used as the App's first-paint sync). */
function applyResolvedTheme(): void {
  document.documentElement.setAttribute("data-theme", ui.theme);
}

/** Reflect a server-sourced choice locally. No write-back; used by
 *  the boot path and the config_changed WS handler. */
function setThemeLocal(choice: ThemeChoice): void {
  ui.themeChoice = choice;
  ui.theme = effectiveTheme(choice);
  applyResolvedTheme();
}

/** Pick a theme. Optimistic local apply, then PATCH the global config
 *  so every other open window picks up the change over the WS
 *  `config_changed` event. */
export function setThemeChoice(choice: ThemeChoice): void {
  setThemeLocal(choice);
  void persistThemeChoice(choice);
}

let themePersistInflight: Promise<void> = Promise.resolve();
function persistThemeChoice(choice: ThemeChoice): Promise<void> {
  // Coalesce rapid clicks (system→light→dark) by chaining off the
  // prior write; the catch swallows so a transient failure doesn't
  // block the next write.
  themePersistInflight = themePersistInflight.catch(() => {}).then(async () => {
    const cfg = await api.config();
    if (cfg.preferences.theme === choice) return;
    await api.updateConfig({
      ...cfg,
      preferences: { ...cfg.preferences, theme: choice },
    });
  });
  return themePersistInflight;
}

/** First-paint DOM sync, before any component mounts. The actual
 *  theme value comes in via the bootstrap `/api/drive` fetch. */
export function applyInitialTheme(): void {
  applyResolvedTheme();
}

/** Mirror server preferences (theme, pane widths) into local state.
 *  Called on boot once `drive.info` is set, and again on every
 *  `config_changed` WS event. */
export function applyServerPreferences(): void {
  const prefs = drive.info?.preferences;
  if (!prefs) return;
  if (prefs.theme && prefs.theme !== ui.themeChoice) {
    setThemeLocal(prefs.theme);
  }
  if (prefs.pane_widths) {
    paneWidths.inspector = prefs.pane_widths.inspector;
    paneWidths.graph = prefs.pane_widths.graph;
    paneWidths.browser = prefs.pane_widths.browser;
  }
}

/** Subscribe to OS-level color-scheme changes. While the user is in
 *  "system" mode, OS toggles flip the editor live; for explicit
 *  "light"/"dark" the listener is a no-op. */
export function watchSystemTheme(): () => void {
  if (typeof window === "undefined" || !window.matchMedia) return () => {};
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = () => {
    if (ui.themeChoice !== "system") return;
    ui.theme = systemTheme();
    applyResolvedTheme();
  };
  mq.addEventListener("change", handler);
  return () => mq.removeEventListener("change", handler);
}

let unwatch: (() => void) | null = null;

/// Watcher event handler. Extracted so reconnectWatcher() can reuse
/// the exact same callbacks as bootstrap().
function onWatchEvent(e: unknown): void {
  ui.lastWatch = Date.now();
  const kind = (e as { kind?: string } | null)?.kind;
  if (kind === "config_changed") {
    // A sibling window flipped a setting (theme, fonts, drive name,
    // pane widths, default-drive root). Re-fetch and reflect.
    scheduleDriveRefresh();
    return;
  }
  if (kind === "session_changed") {
    // Per-window keying means we never share session.json with
    // siblings. Anything we'd react to here is a no-op today.
    return;
  }
  // Filesystem event from chan-server's WatchBroadcast. Server-side
  // dedupe already drops echoes of our own writes (1500 ms window),
  // so anything that lands here is an actual external edit.
  //
  // Two reactions:
  //   1. Refresh the tree + drive payload (file set / preferences
  //      may have changed).
  //   2. Refresh the buffer of any open tab pointing at the changed
  //      path so the editor view doesn't drift behind disk. Dirty
  //      buffers are left alone; the next save's CAS check surfaces
  //      the conflict via ConflictModal.
  void refreshTree();
  scheduleDriveRefresh();
  const inner = (e as { event?: { path?: string; to?: string } } | null)?.event;
  const paths = [inner?.path, inner?.to].filter(
    (p): p is string => typeof p === "string" && p.length > 0,
  );
  for (const p of paths) {
    for (const { tabId } of tabsForPath(p)) {
      void refreshTabFromDisk(tabId);
    }
  }
}

function onWatchStatus(status: WsStatus): void {
  ui.ws = status;
}

/// Tear down the existing watch subscription and start a new one.
/// Used by the disconnect overlay's manual retry button to skip the
/// reconnect backoff. Idempotent: a no-op if nothing is connected.
export function reconnectWatcher(): void {
  if (unwatch) {
    unwatch();
    unwatch = null;
  }
  unwatch = openWatchSocket(onWatchEvent, onWatchStatus);
}

export async function bootstrap(): Promise<void> {
  try {
    drive.info = await api.drive();
    applyServerPreferences();
    await refreshTree();
    // Restore prior layout, in priority order:
    //   1. URL hash: explicit ad-hoc state (copy-paste a URL).
    //   2. .chan/session.json on the server: persisted via
    //      api.putSession so the same panes/tabs come back next
    //      launch.
    //   3. Empty layout: App.svelte auto-opens the file browser
    //      overlay so the user has somewhere to start.
    // Must happen before the watcher starts so we don't fire spurious
    // refreshes mid-restore. Errors are non-fatal.
    //
    // The desktop app's "New Window" menu action passes `?fresh=1`
    // so a brand-new window starts with an empty pane (and the
    // browser overlay) instead of inheriting the shared session.json.
    // The marker is consumed here and stripped from the URL so
    // reload after the fresh open behaves normally.
    const fresh = readAndConsumeFreshFlag();
    const fromHash = fresh ? null : readLayoutHash();
    try {
      if (fromHash) {
        await restoreLayout(fromHash);
      } else if (!fresh) {
        const remote = await api.getSession();
        if (remote) {
          // Session payload may be the new wrapped shape OR a
          // legacy plain-layout body left over from a pre-update
          // file. Both paths restore correctly.
          if (isLegacyLayoutPayload(remote)) {
            await restoreLayout(remote);
          } else {
            await restoreSession(remote as SessionPayload);
          }
        }
      }
    } catch (e) {
      ui.status = `restore failed: ${(e as Error).message}`;
    }
    if (!unwatch) {
      unwatch = openWatchSocket(onWatchEvent, onWatchStatus);
    }
    startIndexStatusPoller();
  } catch (e) {
    ui.status = `bootstrap failed: ${(e as Error).message}`;
  }
}

export async function refreshTree(): Promise<void> {
  tree.loading = true;
  try {
    const entries = await api.list();
    entries.sort((a, b) => {
      // Directories first, then alphabetical.
      if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
      return a.path.localeCompare(b.path);
    });
    tree.entries = entries;
  } finally {
    tree.loading = false;
  }
}

export async function refreshDrive(): Promise<void> {
  drive.info = await api.drive();
  applyServerPreferences();
}

/// Debounced refresh of the drive payload (preferences + name).
/// The watcher fires a burst of events on file save; we don't want
/// to hammer the server with one /api/drive call per event.
let driveRefreshTimer: ReturnType<typeof setTimeout> | null = null;
export function scheduleDriveRefresh(): void {
  if (driveRefreshTimer) return;
  driveRefreshTimer = setTimeout(() => {
    driveRefreshTimer = null;
    void refreshDrive();
  }, 250);
}

// ---- URL hash bridge for layout + UI persistence ------------------------

const HASH_LAYOUT = "s";
const HASH_SIDEBAR = "c"; // "1" if collapsed, absent if expanded

function hashParams(): URLSearchParams {
  const h = window.location.hash;
  return new URLSearchParams(h.startsWith("#") ? h.slice(1) : h);
}

/// Read the `?fresh=1` URL marker (set by the desktop app's New
/// Window menu) and strip it from the address bar so a subsequent
/// reload behaves like a normal drive load. Returns true when
/// the flag was present.
function readAndConsumeFreshFlag(): boolean {
  const url = new URL(window.location.href);
  const fresh = url.searchParams.get("fresh") === "1";
  if (fresh) {
    url.searchParams.delete("fresh");
    window.history.replaceState({}, "", url.toString());
  }
  return fresh;
}

/// Parse the layout encoding from `location.hash` (if any). Tolerant of
/// missing/malformed input.
function readLayoutHash(): ReturnType<typeof serializeLayout> {
  const raw = hashParams().get(HASH_LAYOUT);
  if (!raw) return null;
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

/// Write the current layout to `location.hash` via
/// `history.replaceState` (so reloads are silent and the browser
/// back/forward stack stays clean). Empty layout strips the key
/// entirely.
export function persistStateToHash(): void {
  const ser = serializeLayout();
  const url = new URL(window.location.href);
  const params = hashParams();
  if (!ser) {
    params.delete(HASH_LAYOUT);
  } else {
    params.set(HASH_LAYOUT, JSON.stringify(ser));
  }
  // Drop the legacy sidebar-collapsed key from any pre-existing
  // saved URL hash so it doesn't sit there forever.
  params.delete(HASH_SIDEBAR);
  const next = params.toString();
  url.hash = next ? `#${next}` : "";
  history.replaceState(null, "", url.toString());
}

/// Back-compat alias used elsewhere in the tree.
export const persistLayoutToHash = persistStateToHash;

// ---- session persistence (per-window, server-side) ---------------------
//
// PUT/GET hit `<state>/sessions/<drive-key>/<window-id>.json`. The
// payload is the layout shape from `serializeLayout()` plus a
// `treeExpanded` map (file browser folder state) and an `overlays`
// block (settings/search/assistant/graph open state + per-overlay
// knobs). Round-tripping these means each window restores exactly
// what was on screen, including which overlay was up and what scope
// it was looking at. Debounced more than the URL-hash write since
// this hits the disk.
const SESSION_DEBOUNCE_MS = 750;
let sessionTimer: ReturnType<typeof setTimeout> | null = null;
let lastSessionSnapshot: string | null = null;

/// Wrapped session payload. Forward-compat: missing fields fall
/// back to defaults on restore so adding a new overlay type later
/// doesn't invalidate old session.json files.
type SessionPayload = {
  /// Pane / tab tree (output of `serializeLayout()`).
  layout?: ReturnType<typeof serializeLayout>;
  /// File browser tree-expansion map.
  treeExpanded?: Record<string, boolean>;
  /// Per-overlay context. The `open` flag was intentionally
  /// dropped (overlays always start closed on launch); older
  /// session bodies may still include it and are silently
  /// ignored on read.
  overlays?: {
    assistant?: { open?: boolean; contextId?: string };
    graph?: { open?: boolean; scopeId?: string; depth?: number };
    /// Legacy fields from older session.json shapes; left here
    /// so a fresh schema doesn't reject them at read time.
    settings?: { open?: boolean };
    search?: { open?: boolean };
  };
  /// Legacy field from the deleted RecentsSheet bottom drawer.
  /// Read-but-ignore on restore so older session.json files load
  /// cleanly.
  mobileRecents?: string[];
};

function serializeSession(): SessionPayload | null {
  const layout = serializeLayout();
  const treeMap: Record<string, boolean> = {};
  for (const [k, v] of Object.entries(treeExpanded.map)) {
    if (v) treeMap[k] = true;
  }
  // We persist per-overlay knobs (assistant context, graph scope
  // and depth) but NOT the `open` flag. Auto-opening an overlay on
  // app launch is hostile UX: a session saved while an overlay was
  // open used to trap the user with no visible way to dismiss it
  // on the next launch.
  const overlays = {
    assistant: { contextId: assistantOverlay.contextId },
    graph: {
      scopeId: graphOverlay.scopeId,
      depth: graphOverlay.depth,
    },
  };
  // Skip when there's literally nothing worth persisting.
  if (!layout && Object.keys(treeMap).length === 0) {
    return null;
  }
  return {
    ...(layout ? { layout } : {}),
    ...(Object.keys(treeMap).length > 0 ? { treeExpanded: treeMap } : {}),
    overlays,
  };
}

async function restoreSession(p: SessionPayload): Promise<void> {
  // Apply tree-expansion + per-overlay context up front so the
  // layout restore below sees consistent state. Overlay `open`
  // flags are intentionally ignored on restore so a user who quit
  // the app with an overlay up doesn't get stuck behind it on the
  // next launch.
  if (p.treeExpanded && typeof p.treeExpanded === "object") {
    treeExpanded.map = { "": true, ...p.treeExpanded };
  }
  const ov = p.overlays ?? {};
  if (ov.assistant?.contextId) {
    assistantOverlay.contextId = ov.assistant.contextId;
  }
  if (ov.graph?.scopeId) graphOverlay.scopeId = ov.graph.scopeId;
  if (ov.graph && typeof ov.graph.depth === "number") {
    graphOverlay.depth = ov.graph.depth;
  }
  if (p.layout) {
    await restoreLayout(p.layout);
  }
}

/// True when `value` looks like the legacy unwrapped layout shape
/// (a SerNode with `k`). Used to migrate old session.json bodies in
/// place without a migration step on the server.
function isLegacyLayoutPayload(value: unknown): value is ReturnType<typeof serializeLayout> {
  return (
    !!value &&
    typeof value === "object" &&
    "k" in (value as Record<string, unknown>)
  );
}

export function scheduleSessionSave(): void {
  if (sessionTimer) clearTimeout(sessionTimer);
  sessionTimer = setTimeout(() => {
    sessionTimer = null;
    const payload = serializeSession();
    const next = payload ? JSON.stringify(payload) : "";
    if (next === lastSessionSnapshot) return;
    lastSessionSnapshot = next;
    if (!payload) {
      // No canonical delete; overwrite with null. Server treats
      // null on read as "no session yet".
      void api.putSession(null);
    } else {
      void api.putSession(payload);
    }
  }, SESSION_DEBOUNCE_MS);
}

export function teardown(): void {
  unwatch?.();
  unwatch = null;
  stopIndexStatusPoller();
}

// ---- search-index status poller -----------------------------------------

/// Latest snapshot of the indexer state. `null` until the first
/// poll completes (or if the endpoint is unreachable).
export const indexStatus = $state<{ value: IndexStatus | null }>({
  value: null,
});

/// Open/closed state of the content-search command palette
/// (`SearchPanel.svelte`). Toggled by Cmd/Ctrl+K and by the
/// search button in the toolbar.
export const searchPanel = $state<{ open: boolean }>({ open: false });

/// Per-file assistant conversation state. Keyed by the file's
/// drive-relative path. The conversation persists across
/// overlay close/open so the user can dismiss a proposal, close
/// the dialog, come back, and keep talking. `/clear` (or the
/// Clear button) wipes a single file's entry.
///
/// In-memory only for v1; lost on full app reload. Persisting to
/// localStorage or `.chan/` is a follow-up if the use case
/// demands it.
export type AssistantPendingEdit = {
  toolCallId: string;
  path: string;
  content: string;
  summary: string | null;
  /// "pending" until the user acts; then "applied" or
  /// "dismissed". The card stays in the scrollback after action
  /// so the conversation log is honest.
  status: "pending" | "applied" | "dismissed";
};

/// Each turn carries the ms-since-epoch when it was added to the
/// conversation. Optional so older persisted conversations (saved
/// before timestamps existed) keep loading; the UI just falls back
/// to "no time" for those entries.
///
/// Assistant turns optionally carry the `citations` retrieved for
/// the round that produced them; the drive-Q&A flow renders
/// these as a clickable "Sources" list below the answer. File and
/// group contexts leave it absent (their context is the open file
/// content, not a search excerpt).
export type AssistantTurn =
  | { kind: "user"; content: string; created_at?: number }
  | {
      kind: "assistant";
      content: string;
      created_at?: number;
      citations?: import("../api/types").ContentHit[];
    }
  | { kind: "edit"; edit: AssistantPendingEdit; created_at?: number };

export type AssistantConversation = {
  /// Verbatim message log we ship to /api/llm/complete. Includes
  /// the system prompt as the first entry. tool_use / tool_result
  /// pairs live here too so the model sees its own past turns
  /// correctly across rounds.
  messages: LlmMessage[];
  /// UI scrollback. Mirrors `messages` minus the system prompt
  /// and protocol-only tool_result entries; rendered in the
  /// chat panel.
  turns: AssistantTurn[];
};

/**
 * Three storage buckets for assistant conversations, scoped by
 * context kind:
 *
 *   - `byFile`: keyed by the file's drive-relative path. Each
 *     entry round-trips through
 *     `.chan/assistant/<sha256(path)[..16]>.json` so a single-file
 *     conversation survives across runs. Hashing happens inside
 *     api.getConversation/putConversation/deleteConversation; the
 *     in-memory map keys stay as raw paths.
 *   - `byGroup`: keyed by a stable group ID derived from the
 *     sorted paths joined with `|`. Persisted as an LRU of the
 *     last `GROUP_LRU_MAX` group threads via a manifest blob
 *     (`g_index.json`) plus per-group blobs
 *     (`g_<sha256(key)[..16]>.json`). The manifest is re-ordered
 *     (MRU first) on every save; entries past the cap are evicted
 *     both on disk and in memory so the map can't grow unbounded
 *     across long sessions.
 *   - `drive`: a single conversation for the drive Q&A context.
 *     In-memory across overlay open/close so a user can dismiss
 *     the overlay and come back to the same thread; lost on full
 *     app reload.
 */
export const assistantConversations = $state<{
  byFile: Record<string, AssistantConversation>;
  byGroup: Record<string, AssistantConversation>;
  drive: AssistantConversation | null;
}>({ byFile: {}, byGroup: {}, drive: null });

/** Drop the in-memory entry for a single-file context. */
export function clearFileConversation(path: string): void {
  delete assistantConversations.byFile[path];
}

/// Rekey conversations after a rename. Single-file rename moves
/// `byFile[from]` -> `byFile[to]`. Directory rename moves every
/// `byFile[from/...]` -> `byFile[to/...]`. Server-side persistence
/// is handled by `move_conversations_for_rename` in the same
/// /api/move call; this helper keeps the in-memory map aligned.
export function rekeyConversationsForRename(from: string, to: string): void {
  const moves: Array<[string, string]> = [];
  for (const key of Object.keys(assistantConversations.byFile)) {
    if (key === from) {
      moves.push([key, to]);
    } else if (key.startsWith(`${from}/`)) {
      moves.push([key, `${to}${key.slice(from.length)}`]);
    }
  }
  for (const [oldKey, newKey] of moves) {
    const conv = assistantConversations.byFile[oldKey];
    if (!conv) continue;
    assistantConversations.byFile[newKey] = conv;
    delete assistantConversations.byFile[oldKey];
  }
}

/** Drop the in-memory entry for a group context AND its persisted
 *  blob + manifest entry. Used by /clear and the Clear button. */
export function clearGroupConversation(key: string): void {
  delete assistantConversations.byGroup[key];
  void clearGroupConversationOnDisk(key);
}

/** Drop the drive conversation entirely. */
export function clearDriveConversation(): void {
  assistantConversations.drive = null;
}

// ---- group LRU persistence ----------------------------------------------
//
// Group conversations live in `byGroup` (keyed by the sorted-paths
// `|`-joined string from scope.svelte's `scopeKey`). Each bucket
// entry is mirrored to its own assistant blob; the most recent
// `GROUP_LRU_MAX` of those are tracked in a manifest blob so the
// next launch knows which threads to restore.
//
// Why LRU, not unbounded: a long session can touch many distinct
// pane configurations (split, swap, close a tab, open another),
// each producing a different group key. Persisting every one
// would leak orphan blobs into `.chan/assistant/`; the cap puts
// a hard ceiling on disk + memory growth.

const GROUP_LRU_MAX = 10;
/// Manifest blob key. Matches the `<name>.json` shape every other
/// assistant blob uses; the `g_` prefix makes it easy to spot in a
/// `list_assistant` listing.
const GROUP_INDEX_KEY = "g_index.json";

type GroupIndexEntry = {
  /// 16-hex SHA-256 prefix of `key`. Doubles as the per-group blob
  /// key (combined with the `g_` prefix and `.json` suffix), so the
  /// manifest is enough to find every persisted thread on disk.
  hash: string;
  /// Raw sortedKey from scope.svelte's `scopeKey(paths)`. Stored
  /// so the manifest is self-describing for diagnostics and so a
  /// future pane configuration that produces the same set of
  /// visible files can rehydrate the right thread by string match.
  key: string;
  paths: string[];
  last_touched: number;
};

type GroupIndex = {
  schema_version: number;
  /// MRU first.
  entries: GroupIndexEntry[];
};

function blobKeyForGroupHash(hash: string): string {
  return `g_${hash}.json`;
}

async function loadGroupIndex(): Promise<GroupIndex> {
  try {
    const raw = await api.getAssistantBlob(GROUP_INDEX_KEY);
    if (!raw) return { schema_version: 1, entries: [] };
    const parsed = raw as Partial<GroupIndex>;
    return {
      schema_version: parsed.schema_version ?? 1,
      entries: Array.isArray(parsed.entries) ? parsed.entries : [],
    };
  } catch {
    return { schema_version: 1, entries: [] };
  }
}

async function saveGroupIndex(idx: GroupIndex): Promise<void> {
  try {
    await api.putAssistantBlob(GROUP_INDEX_KEY, idx);
  } catch {
    // Manifest write failure is non-fatal: the per-group blob is
    // already on disk, and the next save rebuilds the manifest.
  }
}

/// Lazy load the persisted group conversation for `key` into
/// `byGroup[key]`. No-op if the bucket already has an entry or no
/// blob exists. Race-safe: if a concurrent submit creates the
/// in-memory entry while the disk read is in flight, the on-disk
/// version is discarded so the user's just-pushed turn isn't
/// clobbered.
export async function loadGroupConversation(key: string): Promise<void> {
  if (assistantConversations.byGroup[key]) return;
  try {
    const hash = await assistantHash16(key);
    const raw = await api.getAssistantBlob(blobKeyForGroupHash(hash));
    if (!raw) return;
    if (assistantConversations.byGroup[key]) return; // race
    const parsed = raw as {
      messages?: LlmMessage[];
      turns?: AssistantTurn[];
    };
    assistantConversations.byGroup[key] = {
      messages: parsed.messages ?? [],
      turns: parsed.turns ?? [],
    };
  } catch {
    // Server unreachable / decode error: leave the bucket empty so
    // the next submit creates a fresh thread.
  }
}

/// Persist a group conversation and bump its position in the LRU
/// manifest. Evicts entries beyond the cap from both disk and the
/// in-memory map.
export async function saveGroupConversation(
  key: string,
  paths: string[],
  conv: AssistantConversation,
): Promise<void> {
  const hash = await assistantHash16(key);
  const blobKey = blobKeyForGroupHash(hash);
  const now = Date.now();
  try {
    await api.putAssistantBlob(blobKey, {
      schema_version: 1,
      kind: "group",
      key,
      paths,
      messages: conv.messages,
      turns: conv.turns,
      last_touched: now,
    });
  } catch {
    // Skip manifest update so we don't promote an entry whose
    // blob isn't actually on disk.
    return;
  }
  const idx = await loadGroupIndex();
  const remaining = idx.entries.filter((e) => e.hash !== hash);
  remaining.unshift({ hash, key, paths, last_touched: now });
  const kept = remaining.slice(0, GROUP_LRU_MAX);
  for (const e of remaining.slice(GROUP_LRU_MAX)) {
    void api.deleteAssistantBlob(blobKeyForGroupHash(e.hash));
    delete assistantConversations.byGroup[e.key];
  }
  await saveGroupIndex({ schema_version: 1, entries: kept });
}

async function clearGroupConversationOnDisk(key: string): Promise<void> {
  try {
    const hash = await assistantHash16(key);
    await api.deleteAssistantBlob(blobKeyForGroupHash(hash));
    const idx = await loadGroupIndex();
    const next = idx.entries.filter((e) => e.hash !== hash);
    if (next.length !== idx.entries.length) {
      await saveGroupIndex({ schema_version: 1, entries: next });
    }
  } catch {
    // Best-effort: if we can't reach the server, the in-memory
    // drop is already done; the disk leftover gets cleaned up the
    // next time eviction runs.
  }
}

// ---- assistant overlay --------------------------------------------------
//
// Global overlay state. Replaces the per-tab `assistantOpen` flag of
// v0.x: Cmd/Ctrl+H now opens one overlay regardless of which tab is
// focused, and the context dropdown at the top of that overlay
// switches between (a) any file currently visible across the
// layout, (b) the group of all visible files when more than one is
// on screen, and (c) the drive-wide Q&A flow that used to live
// inside the SearchPanel.

/**
 * Discriminator for which conversation the overlay is currently
 * showing. Encoded as a single string so `<select>`'s value binding
 * works without parallel state.
 *
 *   - `file:<path>` — single-file context.
 *   - `group:<key>` — group context (key = sorted paths joined `|`).
 *   - `drive`    — Drive Q&A context.
 */
export type AssistantContextId = string;

export const assistantOverlay = $state<{
  open: boolean;
  contextId: AssistantContextId;
}>({
  open: false,
  contextId: "drive",
});

/** Build the context dropdown options for the assistant overlay.
 *  Thin wrapper over the shared scope helper so other overlays
 *  (search, graph) can reuse the same shape with their own labels
 *  for the "drive" / "global" entries. The global entry surfaces
 *  the eventual cross-drive context but is rendered disabled
 *  until backend cross-drive indexing exists. */
export function availableAssistantContexts(): ScopeOption[] {
  return availableScopeOptions({
    driveLabel: "Drive Q&A",
    global: { label: "Global Q&A (cross-drive, coming soon)", enabled: false },
  });
}

/** Open the assistant overlay, snapping the context to the
 *  active file when applicable. Idempotent: opening an already-
 *  open overlay just resets the context to the latest sensible
 *  pick (handy when the user clicked the toolbar button after
 *  the layout shifted). */
export function openAssistant(): void {
  assistantOverlay.contextId = defaultScopeId();
  assistantOverlay.open = true;
  scheduleSessionSave();
}

// ---- graph overlay -----------------------------------------------------
//
// Same shape as the assistant overlay (open + scope picker), plus a
// `depth` knob for how far the file/group scopes expand into their
// neighbors in the link graph.

/** Build the dropdown options for the graph overlay. The "drive"
 *  entry is labelled differently (the graph isn't doing Q&A; it's
 *  showing the whole network), but everything else matches the
 *  assistant's options exactly so a user reading both surfaces sees
 *  the same set of file / group entries. The global entry surfaces
 *  the eventual cross-drive graph but is disabled until backend
 *  cross-drive indexing exists. */
export function availableGraphScopes(): ScopeOption[] {
  return availableScopeOptions({
    driveLabel: "Whole drive",
    global: {
      label: "All drives (cross-drive, coming soon)",
      enabled: false,
    },
  });
}

export const graphOverlay = $state<{
  open: boolean;
  /** Same id encoding as assistantOverlay.contextId
   *  (`file:<path>` | `group:<key>` | `drive`). */
  scopeId: string;
  /** Hop radius from the scope's seed paths. 1 = the seed plus its
   *  immediate neighbors; 2 = neighbors-of-neighbors; etc. Drive
   *  scope ignores depth (it's the whole graph). */
  depth: number;
}>({
  open: false,
  scopeId: "drive",
  depth: 1,
});

/** Open the graph overlay, snapping the scope to the active file
 *  when applicable. Idempotent, mirrors openAssistant. */
export function openGraph(): void {
  graphOverlay.scopeId = defaultScopeId();
  graphOverlay.open = true;
  scheduleSessionSave();
}

// ---- settings overlay --------------------------------------------------
//
// Settings has no scope picker (it's per-device-global, applies
// everywhere), so this is a one-bit overlay state.

export const settingsOverlay = $state<{ open: boolean }>({ open: false });

export function openSettings(): void {
  settingsOverlay.open = true;
  scheduleSessionSave();
}

// ---- file browser overlay ----------------------------------------------
//
// The file browser is a window-level overlay (not a tab), so its
// open + inspector-open state lives here. One per window; the
// inspector toggle is window-scoped now (was per-tab when the
// browser was a tab kind) since there's only ever one instance.

export const browserOverlay = $state<{
  open: boolean;
  inspectorOpen: boolean;
}>({ open: false, inspectorOpen: false });

export function openBrowser(): void {
  browserOverlay.open = true;
  scheduleSessionSave();
}

// ---- side-panel widths --------------------------------------------------
//
// Widths of the file editor inspector, graph details, and file
// browser panels. Per-machine global preferences (mirrored from
// the global config). The right comfortable width tracks screen
// real estate rather than content, so it stays out of session.json.
// Cross-window sync rides the `config_changed` WS event.

const PANE_WIDTH_MIN = 140;
const PANE_WIDTH_MAX = 600;
const DEFAULT_PANE_WIDTHS = {
  inspector: 220,
  graph: 260,
  browser: 240,
};

export const paneWidths = $state<{
  inspector: number;
  graph: number;
  browser: number;
}>({ ...DEFAULT_PANE_WIDTHS });

/// Currently inspected entry in the File Browser tab. Module-level
/// (shared across browser tabs); selection is ephemeral so the
/// minor cross-tab leakage is acceptable and avoids per-tab plumbing.
export const browserSelection = $state<{ path: string | null }>({
  path: null,
});

let widthsPersistTimer: ReturnType<typeof setTimeout> | null = null;
let widthsPersistInflight: Promise<void> = Promise.resolve();
const PANE_WIDTHS_DEBOUNCE_MS = 200;

/// Persist the current widths. Called by ResizeHandle's onChange on
/// every drag tick; debounced so a sweep across the screen lands as
/// one PATCH instead of dozens.
export function persistPaneWidths(): void {
  if (widthsPersistTimer) clearTimeout(widthsPersistTimer);
  widthsPersistTimer = setTimeout(() => {
    widthsPersistTimer = null;
    const snapshot = {
      inspector: clamp(paneWidths.inspector),
      graph: clamp(paneWidths.graph),
      browser: clamp(paneWidths.browser),
    };
    widthsPersistInflight = widthsPersistInflight.catch(() => {}).then(async () => {
      const cfg = await api.config();
      const cur = cfg.preferences.pane_widths;
      if (
        cur &&
        cur.inspector === snapshot.inspector &&
        cur.graph === snapshot.graph &&
        cur.browser === snapshot.browser
      ) {
        return;
      }
      await api.updateConfig({
        ...cfg,
        preferences: { ...cfg.preferences, pane_widths: snapshot },
      });
    });
  }, PANE_WIDTHS_DEBOUNCE_MS);
}

function clamp(n: number): number {
  if (!Number.isFinite(n)) return DEFAULT_PANE_WIDTHS.inspector;
  return Math.max(PANE_WIDTH_MIN, Math.min(PANE_WIDTH_MAX, Math.round(n)));
}

/// Expanded-folder map for the file browser tree. Lifted out of
/// `FileTree.svelte` so the state survives tab switches (the
/// component unmounts every time the active tab changes). Shared
/// across all browser tabs in the window; per-window because two
/// windows on the same drive may be navigating different folders.
///
/// Lives inside the per-window `session.json` payload (round-tripped
/// through `serializeLayout` / `restoreLayout`) so it survives
/// chan-app close + reopen without bloating the user's drive
/// directory.
export const treeExpanded = $state<{ map: Record<string, boolean> }>({
  map: { "": true },
});

/// Trigger a session save so the change reaches disk. Pane / tab
/// edits already call `scheduleSessionSave`; this thin wrapper keeps
/// the call site at the toggle point semantically clear.
export function persistTreeExpanded(): void {
  scheduleSessionSave();
}

/// Poll cadence: fast while the indexer is doing work or has errored,
/// slow when idle (so we still pick up CLI-driven `chan index` runs
/// in the background without hammering the server every second).
const FAST_POLL_MS = 1500;
const SLOW_POLL_MS = 10_000;

let indexPollTimer: ReturnType<typeof setTimeout> | null = null;

/// Kick off the polling loop. Idempotent: calling it twice keeps a
/// single chain alive.
export function startIndexStatusPoller(): void {
  if (indexPollTimer) return;
  void pollIndexStatusOnce();
}

export function stopIndexStatusPoller(): void {
  if (indexPollTimer) {
    clearTimeout(indexPollTimer);
    indexPollTimer = null;
  }
}

async function pollIndexStatusOnce(): Promise<void> {
  let nextDelay = SLOW_POLL_MS;
  try {
    const s = await api.indexStatus();
    indexStatus.value = s;
    // Building / reindexing / error → fast poll so the pill updates
    // promptly. Idle → slow poll.
    nextDelay = s.state === "idle" ? SLOW_POLL_MS : FAST_POLL_MS;
  } catch {
    // Server unreachable or 503 (search disabled): slow-poll. Don't
    // surface as a status-bar error; the pill itself shows "n/a".
    indexStatus.value = null;
    nextDelay = SLOW_POLL_MS;
  } finally {
    indexPollTimer = setTimeout(() => void pollIndexStatusOnce(), nextDelay);
  }
}

// ---- in-page prompt -----------------------------------------------------
//
// `window.prompt()` is not implemented by macOS WKWebView; Tauri silently
// drops it. We replace the few prompt-driven flows (new file / folder /
// rename) with a small in-page modal driven by this state. Same code path
// works in regular browsers too, so there's only one prompt UX to design.

type PromptState = {
  open: boolean;
  title: string;
  defaultValue: string;
  resolve: ((value: string | null) => void) | null;
};

export const promptState = $state<PromptState>({
  open: false,
  title: "",
  defaultValue: "",
  resolve: null,
});

/// Show a single-input modal. Resolves with the user's text on OK or
/// `null` on Cancel / dismiss. Replaces `window.prompt`.
export function uiPrompt(
  title: string,
  defaultValue = "",
): Promise<string | null> {
  return new Promise((resolve) => {
    // If a prompt is already open, reject the previous one as cancelled.
    promptState.resolve?.(null);
    promptState.title = title;
    promptState.defaultValue = defaultValue;
    promptState.resolve = resolve;
    promptState.open = true;
  });
}

/// Called by the modal component on OK / Cancel.
export function resolvePrompt(value: string | null): void {
  const r = promptState.resolve;
  promptState.resolve = null;
  promptState.open = false;
  r?.(value);
}

/// File CRUD helpers shared by the sidebar header and the tree's
/// context menu / hover actions. They wrap the raw API with prompts,
/// confirmations, and a tree refresh, plus opening the new file in the
/// active pane on create. Surfacing one set of behaviors via several
/// affordances keeps the actions consistent regardless of which entry
/// point the user reaches for.

/// Append `.md` to a relative path when the basename has no real
/// extension. "Real extension" = a `.` past position 0 with content
/// after it. So `note` → `note.md`, `sub/note` → `sub/note.md`,
/// `note.txt` stays, `note.` (trailing dot) → `note..md` is avoided
/// by stripping the trailing dot first. Hidden-style names like
/// `.gitignore` get `.md` tacked on intentionally: this is a notes
/// app, the user typed a name, not a Unix dotfile.
function appendDefaultMd(path: string): string {
  const stripped = path.endsWith(".") ? path.slice(0, -1) : path;
  const slash = stripped.lastIndexOf("/");
  const basename = slash >= 0 ? stripped.slice(slash + 1) : stripped;
  const dot = basename.lastIndexOf(".");
  if (dot <= 0) return `${stripped}.md`;
  return stripped;
}

/// Re-attach the original file's extension to a rename target when
/// the user dropped it during the prompt. A renamed `note.md` ->
/// `humus` rounds back up to `humus.md`. If the user explicitly
/// chose a different extension (`humus.txt`) we leave it alone, and
/// if the original had no extension we don't invent one. Hidden-
/// style basenames (where the only `.` is at position 0) are
/// treated as extension-less so a leading-dot file doesn't claim
/// the rest of its name as the "extension". Mirrors
/// `appendDefaultMd`'s "real extension" predicate.
function preserveExtension(oldPath: string, newPath: string): string {
  const oldBase = basenameOf(oldPath);
  const oldDot = oldBase.lastIndexOf(".");
  if (oldDot <= 0) return newPath;
  const oldExt = oldBase.slice(oldDot);
  const newBase = basenameOf(newPath);
  const newDot = newBase.lastIndexOf(".");
  if (newDot > 0) return newPath;
  return newPath + oldExt;
}

function basenameOf(path: string): string {
  const slash = path.lastIndexOf("/");
  return slash >= 0 ? path.slice(slash + 1) : path;
}

export const fileOps = {
  async createFile(parentPath: string): Promise<void> {
    const name = await uiPrompt(
      "new file (relative path), e.g. note or sub/note (.md added if no extension)",
    );
    if (!name) return;
    const raw = parentPath ? `${parentPath}/${name}` : name;
    // Default to .md when the user didn't type one. "No extension"
    // means the basename has no `.` past position 0 (hidden files
    // like `.gitignore` still get .md tacked on, which is the
    // friendly outcome for a notes app: the user typed a name, not
    // a hidden file). Existing extensions are preserved; non-text
    // ones still hit the editable-text gate below and get rejected.
    const path = appendDefaultMd(raw);
    if (!isEditableText(path)) {
      ui.status = `'${path}' is not an editable text file (only .md and .txt)`;
      return;
    }
    try {
      await api.create(path, false, "");
      await refreshTree();
      await openInActivePane(path);
    } catch (e) {
      ui.status = `create failed: ${(e as Error).message}`;
    }
  },
  async createDir(parentPath: string): Promise<void> {
    const name = await uiPrompt("new folder name");
    if (!name) return;
    const path = parentPath ? `${parentPath}/${name}` : name;
    try {
      await api.create(path, true);
      await refreshTree();
    } catch (e) {
      ui.status = `create failed: ${(e as Error).message}`;
    }
  },
  /// Rename or move a file / directory. `isDir` toggles the
  /// extension-preservation step: for files, if the user drops the
  /// existing extension during the prompt (typed `note` instead of
  /// `note.md`), put it back so the resulting path still routes
  /// through the editor's text gate. Directories don't have
  /// extensions so the input is taken verbatim.
  async rename(path: string, isDir = false): Promise<void> {
    const next = await uiPrompt("new path", path);
    if (!next || next === path) return;
    const target = isDir ? next : preserveExtension(path, next);
    if (target === path) return;
    try {
      await api.move(path, target);
      await refreshTree();
      // Re-key in-memory assistant conversations so the chat
      // history follows the file. The server already renamed the
      // on-disk JSON in the same /api/move call; this keeps the
      // in-memory map in sync without a refetch round-trip.
      // Handles both single-file renames (exact key match) and
      // directory renames (every key under `path/`).
      rekeyConversationsForRename(path, target);
    } catch (e) {
      ui.status = `rename failed: ${(e as Error).message}`;
    }
  },
  /// Delete a file (or directory) from the drive.
  ///
  /// Closes any open tabs pointing at the deleted path (or paths
  /// under it, for directory deletes) and drops the per-file
  /// assistant conversation so .chan/assistant/ doesn't accumulate
  /// orphan blobs.
  ///
  /// We deliberately don't confirm: the disk write is irreversible
  /// either way and Chrome/Safari throttle repeated confirm()
  /// dialogs.
  async remove(path: string): Promise<void> {
    try {
      await api.remove(path);
      await Promise.all([refreshTree(), refreshDrive()]);
      const underDeleted = (p: string) =>
        p === path || p.startsWith(`${path}/`);
      // Snapshot (paneId, tabId) pairs to close BEFORE mutating
      // layout, since closeTab may collapse the pane mid-iteration.
      const toClose: Array<[string, string]> = [];
      const deletedFilePaths: string[] = [];
      for (const node of Object.values(layout.nodes)) {
        if (node.kind !== "leaf") continue;
        for (const t of node.tabs) {
          if (t.kind !== "file") continue;
          if (underDeleted(t.path)) {
            toClose.push([node.id, t.id]);
            deletedFilePaths.push(t.path);
          }
        }
      }
      for (const [paneId, tabId] of toClose) {
        closeTab(paneId, tabId);
      }
      for (const p of deletedFilePaths) {
        clearFileConversation(p);
        // 404 is a harmless no-op (deleteConversation is idempotent),
        // so we don't special-case never-persisted paths.
        void api.deleteConversation(p);
      }
    } catch (e) {
      ui.status = `delete failed: ${(e as Error).message}`;
    }
  },
};
