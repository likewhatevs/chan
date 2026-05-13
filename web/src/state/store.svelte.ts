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
import { appendDefaultMd, preserveExtension } from "./pathValidate";
import { setNotifyHandler } from "./notify.svelte";
import {
  availableScopeOptions,
  defaultScopeId,
  type ScopeOption,
} from "./scope.svelte";
import {
  clearTabError,
  refreshTabFromDisk,
  rekeyTabsForRename,
  tabsForPath,
} from "./tabs.svelte";
import { graphData, invalidateGraph, ensureGraphLoaded } from "./graphData.svelte";
import { SETTINGS_DISABLED, withTokenQuery } from "../api/transport";
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
    // Server hands back PaneWidths.search via #[serde(default)], so
    // older preferences.toml without the field still arrives with a
    // valid number rather than `undefined`.
    paneWidths.search = prefs.pane_widths.search ?? DEFAULT_PANE_WIDTHS.search;
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

/// Paths currently mid-rename. The watcher fires "Renamed" events
/// while `api.move` is still awaiting, which races with our own
/// `rekeyTabsForRename` call: `tabsForPath(oldPath)` still matches
/// (tabs haven't been rekeyed yet), the resulting `refreshTabFromDisk`
/// tries to load the now-gone old path and stamps a stale "io error:
/// No such file" onto the tab. Adding both endpoints to this Set
/// before `api.move` and clearing them after the rekey lets the
/// watcher handler skip the refresh for paths it would have raced.
const movingPaths = new Set<string>();

/// Watcher event handler. Extracted so reconnectWatcher() can reuse
/// the exact same callbacks as bootstrap().
function onWatchEvent(e: unknown): void {
  ui.lastWatch = Date.now();
  // The /ws stream carries multiple frame types under different
  // `type` discriminators (see chan-server/src/bus.rs). Watch
  // events fall through to the legacy path below; progress events
  // route to the indexer-status sink so the bottom-left status pill
  // animates live as `Drive::reindex_with` walks the drive.
  const frameType = (e as { type?: string } | null)?.type;
  if (frameType === "progress") {
    applyProgressEvent(
      (e as { event?: ProgressFrame } | null)?.event ?? null,
    );
    return;
  }
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
  // Tags / wiki-links / mentions may have changed. Invalidate the
  // cached graph so the next inspector view sees fresh data, and if
  // an overlay is currently open re-fetch eagerly so the user sees
  // updates without re-clicking. The fetch is idempotent and
  // de-duped via `ensureGraphLoaded`.
  invalidateGraph();
  if (browserOverlay.open || graphOverlay.open) {
    void ensureGraphLoaded();
  }
  const inner = (e as { event?: { path?: string; to?: string } } | null)?.event;
  const paths = [inner?.path, inner?.to].filter(
    (p): p is string => typeof p === "string" && p.length > 0,
  );
  for (const p of paths) {
    // Skip watcher echoes for paths we're actively renaming: the
    // tab still holds the old path during the move's `await`, and a
    // refresh would read a vanished file and stamp a stale error.
    if (movingPaths.has(p)) continue;
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
        // URL hash wins on layout (copy-pasted links must reproduce
        // tabs verbatim), but personal UI prefs — tree-expansion,
        // assistant scope, etc — still come from session.json. The
        // hash deliberately doesn't carry these so a shared link
        // doesn't leak the recipient's folder state into the sender's
        // session.
        await restoreLayout(fromHash);
        if (!fresh) {
          const remote = await api.getSession();
          if (remote && !isLegacyLayoutPayload(remote)) {
            applySessionSidecars(remote as SessionPayload);
          }
        }
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
      // Per-overlay state from the hash lands on top of any
      // session-restored knobs so a shared URL always wins. Skipped
      // in fresh windows so the New-Window menu starts truly clean.
      if (!fresh) applyOverlaysFromHash();
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
    seedTreeExpansionIfFresh();
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
//
// Every visible surface round-trips through the URL hash so a
// copy-paste of the address bar reproduces the same screen on
// another browser: pane / tab tree under `s`, plus a per-overlay
// key (`files`, `search`, `graph`, `assist`, `settings`). Presence
// of an overlay key = that overlay is open; its value carries the
// scoped state (selected entry, query, scope+depth+filters,
// assistant context). Settings has no per-overlay state so its
// value is just `1`.

const HASH_LAYOUT = "s";
const HASH_SIDEBAR = "c"; // "1" if collapsed, absent if expanded
const HASH_BROWSER = "files";
const HASH_SEARCH = "search";
const HASH_GRAPH = "graph";
const HASH_ASSIST = "assist";
const HASH_SETTINGS = "settings";

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

/// Encode the graph filter chips as a 4-char string of `0`/`1`,
/// order: link, tag, mention, img. All-on (the default) returns
/// the empty string so a fresh graph open doesn't bloat the URL.
function encodeGraphFilters(f: GraphFilters): string {
  if (f.link && f.tag && f.mention && f.img) return "";
  const bit = (v: boolean) => (v ? "1" : "0");
  return `${bit(f.link)}${bit(f.tag)}${bit(f.mention)}${bit(f.img)}`;
}

function decodeGraphFilters(s: string): GraphFilters {
  // Empty / missing string = defaults (all on).
  if (!s) return { ...DEFAULT_GRAPH_FILTERS };
  const ch = (i: number) => (s[i] === "0" ? false : true);
  return { link: ch(0), tag: ch(1), mention: ch(2), img: ch(3) };
}

/// Split `<flag>:<rest>` where flag is a single `0`/`1` for an
/// inspector-open bit. Returns `[bit, rest]`; if no leading flag
/// is present the bit comes back as null and the original string
/// is returned as the rest (legacy hash format).
function splitInspectorBit(raw: string): [boolean | null, string] {
  if (raw.length >= 2 && raw[1] === ":" && (raw[0] === "0" || raw[0] === "1")) {
    return [raw[0] === "1", raw.slice(2)];
  }
  return [null, raw];
}

/// Apply overlay state encoded in `location.hash`. Called from
/// bootstrap after the layout (and session payload, where
/// applicable) has been restored, so the per-overlay knobs land
/// on top of any session-persisted defaults. Each key is optional;
/// missing means "overlay stays closed".
function applyOverlaysFromHash(): void {
  const params = hashParams();
  if (params.has(HASH_BROWSER)) {
    // Encoding: `<inspectorBit>:<path>`. Both fields optional.
    const [ins, path] = splitInspectorBit(params.get(HASH_BROWSER) ?? "");
    if (ins !== null) browserOverlay.inspectorOpen = ins;
    // Just plant the selection — DO NOT auto-expand ancestors. The
    // hash always carries the last-selected entry as the user moves
    // around, so reusing `revealAndSelect` here would clobber the
    // user's persisted collapse state on every reload (and re-save
    // the auto-expansion via persistTreeExpanded). If the selected
    // row isn't visible because an ancestor is collapsed, opening
    // that ancestor reveals it — the saved collapse wins.
    browserSelection.path = path || null;
    browserOverlay.open = true;
  }
  if (params.has(HASH_SEARCH)) {
    // Encoding: `<inspectorBit>:<query>`. Both fields optional.
    const [ins, query] = splitInspectorBit(params.get(HASH_SEARCH) ?? "");
    if (ins !== null) searchPanel.inspectorOpen = ins;
    searchPanel.query = query;
    searchPanel.open = true;
  }
  if (params.has(HASH_GRAPH)) {
    // Encoding: `<scopeId>|<depth>|<chips>|<inspectorBit>`. All
    // trailing fields optional; an empty value falls back to
    // defaults.
    const raw = params.get(HASH_GRAPH) ?? "";
    const [scope, depthStr, chips, ins] = raw.split("|");
    if (scope) graphOverlay.scopeId = scope;
    const depth = Number(depthStr);
    if (Number.isFinite(depth) && depth >= 1) graphOverlay.depth = depth;
    // Mutate fields in place; reassigning `graphOverlay.filters`
    // would orphan any consumer that captured the proxy reference
    // at mount time (e.g. `const show = graphOverlay.filters` in
    // GraphPanel.svelte).
    const f = decodeGraphFilters(chips ?? "");
    graphOverlay.filters.link = f.link;
    graphOverlay.filters.tag = f.tag;
    graphOverlay.filters.mention = f.mention;
    graphOverlay.filters.img = f.img;
    if (ins === "0" || ins === "1") graphOverlay.inspectorOpen = ins === "1";
    graphOverlay.open = true;
  }
  if (params.has(HASH_ASSIST)) {
    // Encoding: `<contextId>|<prompt>`. The prompt is everything
    // after the first `|` (so it may itself contain `|`).
    const raw = params.get(HASH_ASSIST) ?? "";
    const sep = raw.indexOf("|");
    if (sep === -1) {
      if (raw) assistantOverlay.contextId = raw;
      assistantOverlay.prompt = "";
    } else {
      const ctx = raw.slice(0, sep);
      if (ctx) assistantOverlay.contextId = ctx;
      assistantOverlay.prompt = raw.slice(sep + 1);
    }
    assistantOverlay.open = true;
  }
  if (params.has(HASH_SETTINGS) && !SETTINGS_DISABLED) {
    settingsOverlay.open = true;
  }
}

/// Write the current layout + overlay state to `location.hash` via
/// `history.replaceState` (so reloads are silent and the browser
/// back/forward stack stays clean). Empty values strip their key
/// entirely so a hash never grows orphans.
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
  // ---- overlay keys: presence = open ------------------------
  if (browserOverlay.open) {
    const ins = browserOverlay.inspectorOpen ? "1" : "0";
    params.set(HASH_BROWSER, `${ins}:${browserSelection.path ?? ""}`);
  } else {
    params.delete(HASH_BROWSER);
  }
  if (searchPanel.open) {
    const ins = searchPanel.inspectorOpen ? "1" : "0";
    params.set(HASH_SEARCH, `${ins}:${searchPanel.query ?? ""}`);
  } else {
    params.delete(HASH_SEARCH);
  }
  if (graphOverlay.open) {
    const chips = encodeGraphFilters(graphOverlay.filters);
    // Trim trailing separators when later fields are empty so
    // URLs stay readable (`graph=drive` instead of `graph=drive|1|`).
    const depth = String(graphOverlay.depth);
    const ins = graphOverlay.inspectorOpen ? "1" : "0";
    const insTail = graphOverlay.inspectorOpen ? `|${ins}` : "";
    let val = graphOverlay.scopeId;
    if (chips || insTail) val = `${val}|${depth}|${chips}${insTail}`;
    else if (depth !== "1") val = `${val}|${depth}`;
    params.set(HASH_GRAPH, val);
  } else {
    params.delete(HASH_GRAPH);
  }
  if (assistantOverlay.open) {
    let val = assistantOverlay.contextId;
    if (assistantOverlay.prompt) val = `${val}|${assistantOverlay.prompt}`;
    params.set(HASH_ASSIST, val);
  } else {
    params.delete(HASH_ASSIST);
  }
  if (settingsOverlay.open) {
    params.set(HASH_SETTINGS, "1");
  } else {
    params.delete(HASH_SETTINGS);
  }
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
  applySessionSidecars(p);
  if (p.layout) {
    await restoreLayout(p.layout);
  }
}

/// Apply the non-layout slices of a session payload: file-browser
/// tree-expansion + per-overlay scope/context. Pulled out of
/// `restoreSession` so the URL-hash bootstrap path (which owns the
/// layout but not the personal UI prefs) can still load these from
/// session.json. The hash is meant to be shareable; folder
/// open/closed state and assistant context are per-user and stay in
/// session.json regardless of where the layout came from.
function applySessionSidecars(p: SessionPayload): void {
  if (p.treeExpanded && typeof p.treeExpanded === "object") {
    treeExpanded.map = { "": true, ...p.treeExpanded };
    markTreeExpansionRestored();
  }
  const ov = p.overlays ?? {};
  if (ov.assistant?.contextId) {
    assistantOverlay.contextId = ov.assistant.contextId;
  }
  if (ov.graph?.scopeId) graphOverlay.scopeId = ov.graph.scopeId;
  if (ov.graph && typeof ov.graph.depth === "number") {
    graphOverlay.depth = ov.graph.depth;
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

/// Fire any pending session save synchronously via `fetch({ keepalive:
/// true })` so the request survives the page unload. Without this,
/// quick "expand folder; Cmd-R" cycles lose the toggle: the 750 ms
/// debounce hasn't elapsed, the page reloads, the in-flight payload
/// is discarded. Registered on `pagehide` (which also fires on bfcache
/// suspends, unlike `beforeunload`).
function flushSessionSaveOnExit(): void {
  if (sessionTimer) {
    clearTimeout(sessionTimer);
    sessionTimer = null;
  }
  const payload = serializeSession();
  const next = payload ? JSON.stringify(payload) : "";
  if (next === lastSessionSnapshot) return;
  lastSessionSnapshot = next;
  const url = withTokenQuery("/api/session?w=default");
  const body = payload === null ? "null" : next;
  try {
    fetch(url, {
      method: "PUT",
      headers: { "content-type": "application/json" },
      body,
      keepalive: true,
    }).catch(() => {});
  } catch {
    /* page is going away; nothing useful we can do */
  }
}

/// Register the pagehide flush once. Idempotent so HMR re-evaluations
/// don't stack listeners.
let pagehideHooked = false;
export function installSessionFlushHook(): void {
  if (pagehideHooked || typeof window === "undefined") return;
  pagehideHooked = true;
  window.addEventListener("pagehide", flushSessionSaveOnExit);
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

/// Wire shape of a chan-drive `ProgressEvent`, mirrored from
/// chan-core's `progress::ProgressEvent`. Pinned here because the
/// frontend doesn't import a generated type; the chan-server WS
/// bus.rs renders the same shape and we read it verbatim.
type ProgressFrame = {
  stage:
    | "GraphRebuild"
    | "IndexFile"
    | "EmbedBatch"
    | "RenameRewrite"
    | "Import"
    | "Reset"
    | "ModelLoad"
    | "Heartbeat";
  current: number;
  total: number;
  label: string | null;
};

/// Apply a single progress event to the live indexer status pill.
/// Two stages drive the Building animation:
///   - GraphRebuild: per-file walk during the graph pass.
///   - IndexFile: per-file step of the BM25 + dense build.
/// Other stages (EmbedBatch, Reset, ModelLoad, Heartbeat, Import,
/// RenameRewrite) don't override the indexer status today — they
/// live in their own surfaces (import wizard, etc.). The poller
/// continues to run; on the next idle tick it resets the pill to
/// the Idle counts.
function applyProgressEvent(ev: ProgressFrame | null): void {
  if (!ev) return;
  if (ev.stage === "GraphRebuild" || ev.stage === "IndexFile") {
    indexStatus.value = {
      state: "building",
      current: ev.current,
      total: ev.total,
      file: ev.label ?? "",
    };
  }
}

/// Long-running import progress surfaced in the bottom-left status
/// bar. Set by import wizards (currently just `ImportContactsModal`)
/// while a blocking request is in flight; cleared on completion.
/// Detail (counts, errors) stays in the modal's "done" step; the
/// bar's job is to be the always-visible ambient signal that
/// something is happening, even after the user has dismissed the
/// modal or moved to another overlay.
///
/// Same pattern is intended to host search-indexing progress and
/// assistant-thinking signals once those surfaces want a global
/// "in-flight" pill (today they live in their own panels).
export const importStatus = $state<{ value: { label: string } | null }>({
  value: null,
});

/// Open/closed state of the content-search command palette
/// (`SearchPanel.svelte`). Toggled by Cmd/Ctrl+K and by the
/// search button in the toolbar.
export const searchPanel = $state<{
  open: boolean;
  inspectorOpen: boolean;
  /// Live query bound to the SearchPanel input. Lifted out of the
  /// component so it round-trips through the URL hash: copy-paste of
  /// a chan URL with `search=foo` lands on the same query.
  query: string;
  /// Selected scope id (file:<path> / dir:<path> / git_repo:<root> /
  /// group:<key> / drive / global). Matches the same scope picker
  /// shape Graph + Assistant use, fed by availableScopeOptions().
  /// Today the server-side /api/search/content has no scope param,
  /// so the SearchPanel filters hits client-side against this id;
  /// `drive` and `global` mean "no filter".
  scopeId: string;
}>({
  open: false,
  inspectorOpen: false,
  query: "",
  scopeId: "drive",
});

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
  /// Live prompt buffer. Lifted out of InlineAssist so it
  /// round-trips through the URL hash: a copy-pasted chan URL
  /// reopens the assistant with whatever the user had typed but
  /// not yet submitted.
  prompt: string;
}>({
  open: false,
  contextId: "drive",
  prompt: "",
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
  const out = availableScopeOptions({
    driveLabel: "Whole drive",
    global: {
      label: "All drives (cross-drive, coming soon)",
      enabled: false,
    },
  });
  // Inject the currently-active tag scope as an option so the
  // dropdown can both display the selection and let the user pick
  // back to it after switching away. Tag scopes are entered via
  // openGraphForTag(nodeId, label) from the editor / inspectors and
  // don't appear in the layout-derived option list otherwise.
  if (graphOverlay.scopeId.startsWith("tag:")) {
    const nodeId = graphOverlay.scopeId.slice("tag:".length);
    if (nodeId) {
      const view = graphData.view;
      const node = view?.nodes.find((n) => n.kind === "tag" && n.id === nodeId);
      const label = node?.label ?? nodeId.replace(/^#/, "");
      out.unshift({
        id: graphOverlay.scopeId,
        kind: "tag",
        label: `tag: ${label}`,
        nodeId,
      });
    }
  }
  return out;
}

/** Build the dropdown options for the search overlay. Today the
 *  list is just the whole drive: /api/search/content has no scope
 *  param yet, so narrow scopes can't be honored honestly and we'd
 *  rather not show options that don't work. The selector UI stays
 *  live for visual parity with Graph + Assistant; when the backend
 *  grows a scope param, switch this body to the same
 *  availableScopeOptions(...) call those other surfaces use. */
export function availableSearchScopes(): ScopeOption[] {
  return [{ id: "drive", kind: "drive", label: "Whole drive" }];
}

/** Edge-kind / node-kind chip toggles on the graph. `link`, `tag`,
 *  `mention` are edge-kind filters (their edges plus any node only
 *  reachable through filtered-out edges drop). `img` is a node
 *  filter that hides every file node classified as an image. Lifted
 *  out of GraphPanel so the URL hash can round-trip the exact
 *  filter set. */
export type GraphFilters = {
  link: boolean;
  tag: boolean;
  mention: boolean;
  img: boolean;
};

export const DEFAULT_GRAPH_FILTERS: GraphFilters = {
  link: true,
  tag: true,
  mention: true,
  img: true,
};

export const graphOverlay = $state<{
  open: boolean;
  /** Same id encoding as assistantOverlay.contextId
   *  (`file:<path>` | `group:<key>` | `drive`). */
  scopeId: string;
  /** Hop radius from the scope's seed paths. 1 = the seed plus its
   *  immediate neighbors; 2 = neighbors-of-neighbors; etc. Drive
   *  scope ignores depth (it's the whole graph). */
  depth: number;
  /** Per-edge-kind / per-node-kind chip toggles. */
  filters: GraphFilters;
  /** Right-side details panel toggle. Lifted out of GraphPanel so
   *  it round-trips through the URL hash. */
  inspectorOpen: boolean;
  /** One-shot pre-selected node id, consumed by GraphPanel on the
   *  next open. Set by openGraphAtNode when launching the overlay
   *  from a tag/mention/date chip elsewhere in the UI. Cleared once
   *  the panel applies it. Not persisted in session. */
  pendingSelectId: string | null;
}>({
  open: false,
  scopeId: "drive",
  depth: 1,
  filters: { ...DEFAULT_GRAPH_FILTERS },
  inspectorOpen: false,
  pendingSelectId: null,
});

/** Open the graph overlay, snapping the scope to the active file
 *  when applicable. Idempotent, mirrors openAssistant. */
export function openGraph(): void {
  graphOverlay.scopeId = defaultScopeId();
  graphOverlay.pendingSelectId = null;
  graphOverlay.open = true;
  scheduleSessionSave();
}

/** Open the graph overlay at drive scope and pre-select the given
 *  node so its connections render in the inspector immediately.
 *  Used by tag/mention/date chips outside the graph (file browser
 *  inspector today; conceivably the editor margin later). Drive
 *  scope guarantees the node is in the rendered set regardless of
 *  the previously-saved scope. */
export function openGraphAtNode(nodeId: string): void {
  graphOverlay.scopeId = "drive";
  graphOverlay.pendingSelectId = nodeId;
  graphOverlay.open = true;
  // The file browser overlay paints above the graph (same overlay
  // tier; whichever opens last is on top), so leaving it up would
  // hide the graph the user just asked for. Close it here so this
  // call is "switch surfaces", not "stack a new one behind".
  browserOverlay.open = false;
  scheduleSessionSave();
}

/** Open the graph overlay scoped to a specific file and pre-select
 *  that file's node. The file tab menu's "Show in Graph" routes
 *  here so the resulting subgraph is the file's neighbourhood, not
 *  the entire drive — matching the user's mental model that
 *  invoking the graph FROM a file means "show me what's around
 *  THIS file". */
export function openGraphForFile(path: string): void {
  graphOverlay.scopeId = `file:${path}`;
  graphOverlay.pendingSelectId = path;
  graphOverlay.open = true;
  browserOverlay.open = false;
  scheduleSessionSave();
}

/** Open the graph overlay scoped to a tag, with the tag node itself
 *  pre-selected. The resulting subgraph is the tag's neighbourhood
 *  (every file referencing the tag, plus their depth-limited
 *  neighbours). Called from every "click a tag chip" surface:
 *  editor tag pills, FileInfoBody's tag list, search overlay tag
 *  hits, TagInfoBody's Open-in-Graph button. */
export function openGraphForTag(nodeId: string, _label: string): void {
  graphOverlay.scopeId = `tag:${nodeId}`;
  graphOverlay.pendingSelectId = nodeId;
  graphOverlay.open = true;
  browserOverlay.open = false;
  scheduleSessionSave();
}

// ---- settings overlay --------------------------------------------------
//
// Settings has no scope picker (it's per-device-global, applies
// everywhere), so this is a one-bit overlay state.

export const settingsOverlay = $state<{ open: boolean }>({ open: false });

/// True when the server forbids opening Settings (today: tunnel
/// mode with --tunnel-public). Sourced from the SPA shell meta tag
/// at module load. The pill and any other entry point check this
/// to grey themselves out; `openSettings` also gates on it so the
/// Cmd/Ctrl+, keybinding cannot work around the disabled button.
export const settingsDisabled = SETTINGS_DISABLED;

export function openSettings(): void {
  if (SETTINGS_DISABLED) return;
  settingsOverlay.open = true;
  scheduleSessionSave();
}

// ---- file browser overlay ----------------------------------------------
//
// The file browser is a window-level overlay (not a tab), so its
// open + inspector-open state lives here. One per window; the
// inspector toggle is window-scoped now (was per-tab when the
// browser was a tab kind) since there's only ever one instance.

/// On viewports >= this width the browser inspector defaults open.
/// Below it, the inspector starts closed so a phone-sized layout
/// gets the full screen for the tree. The user can always toggle.
const BROWSER_INSPECTOR_BREAKPOINT_PX = 768;

function defaultInspectorOpen(): boolean {
  if (typeof window === "undefined") return true;
  return window.innerWidth >= BROWSER_INSPECTOR_BREAKPOINT_PX;
}

export const browserOverlay = $state<{
  open: boolean;
  inspectorOpen: boolean;
}>({ open: false, inspectorOpen: defaultInspectorOpen() });

export function openBrowser(): void {
  browserOverlay.open = true;
  scheduleSessionSave();
}

// ---- overlay z-order stack ----------------------------------------------
//
// Window-level overlays (files / search / graph / assistant / settings)
// can stack: opening a second overlay while another is up should put the
// newcomer on top, and Escape should pop just the topmost, returning to
// the one underneath. The previous setup had every OverlayShell binding
// its own window-level Escape listener, which closed every open overlay
// at once.
//
// `overlayStack.ids` is the active z-order (last = top). App.svelte owns
// a single $effect that watches each overlay's `.open` flag and diffs
// against the stack: closed ids drop out, newly-opened ids get appended
// (so the most-recently-opened sits on top). OverlayShell renders with
// `z-index = 25000 + depth * 10` so paint order matches the stack.
//
// Escape lives in App.svelte too and only closes `topOverlay()`. The
// per-shell click-on-scrim still closes that shell directly; since only
// the topmost overlay is visually accessible, the scrim target is
// naturally the same as the stack top.

export type OverlayId =
  | "browser"
  | "search"
  | "graph"
  | "assistant"
  | "settings";

export const overlayStack = $state<{ ids: OverlayId[] }>({ ids: [] });

/// Index of `id` in the stack, or -1 when closed. Components pass the
/// index through to OverlayShell's z-index so newer overlays paint
/// above older ones.
export function overlayDepth(id: OverlayId): number {
  return overlayStack.ids.indexOf(id);
}

/// Id of the topmost open overlay, or `null` when nothing is up. Used
/// by the window-level Escape handler to close one overlay at a time.
export function topOverlay(): OverlayId | null {
  const n = overlayStack.ids.length;
  return n === 0 ? null : overlayStack.ids[n - 1];
}

/// Close one overlay by id. Mirrors the per-shell `close()` callbacks
/// (each sets `<overlay>.open = false`); the sync effect in App.svelte
/// drops it from the stack.
export function closeOverlay(id: OverlayId): void {
  switch (id) {
    case "browser":
      browserOverlay.open = false;
      return;
    case "search":
      searchPanel.open = false;
      return;
    case "graph":
      graphOverlay.open = false;
      return;
    case "assistant":
      assistantOverlay.open = false;
      return;
    case "settings":
      settingsOverlay.open = false;
      return;
  }
}

/// Diff the five overlay `.open` flags into `overlayStack.ids`:
/// remove ids whose overlay is closed, append ids that opened since
/// the last run. Append-only for newcomers means the most-recently
/// opened overlay always lands on top, which matches user intent
/// when they hit a chord to surface a new tool over the current one.
/// Called from a single $effect in App.svelte.
export function syncOverlayStack(): void {
  const open = new Set<OverlayId>();
  if (browserOverlay.open) open.add("browser");
  if (searchPanel.open) open.add("search");
  if (graphOverlay.open) open.add("graph");
  if (assistantOverlay.open) open.add("assistant");
  if (settingsOverlay.open) open.add("settings");
  // Drop closed entries while preserving the existing relative
  // order of those that remain.
  const kept = overlayStack.ids.filter((id) => open.has(id));
  // Append any open id that wasn't already in the stack.
  for (const id of open) {
    if (!kept.includes(id)) kept.push(id);
  }
  // Mutate only when something changed; otherwise the assignment
  // would still trigger consumers of `overlayStack.ids` even for
  // no-op runs (the effect runs on every store mutation).
  if (
    kept.length !== overlayStack.ids.length ||
    kept.some((id, i) => id !== overlayStack.ids[i])
  ) {
    overlayStack.ids = kept;
  }
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
  search: 280,
};

export const paneWidths = $state<{
  inspector: number;
  graph: number;
  browser: number;
  search: number;
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
      search: clamp(paneWidths.search),
    };
    widthsPersistInflight = widthsPersistInflight.catch(() => {}).then(async () => {
      const cfg = await api.config();
      const cur = cfg.preferences.pane_widths;
      if (
        cur &&
        cur.inspector === snapshot.inspector &&
        cur.graph === snapshot.graph &&
        cur.browser === snapshot.browser &&
        cur.search === snapshot.search
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

/// True once we've established the initial tree-expansion state for
/// this session (either from session.json or from the fresh-session
/// auto-expand seed). Skips the auto-expand on subsequent
/// `refreshTree` calls so a user who collapsed everything doesn't
/// have it re-expanded behind their back on the next watcher tick.
let treeExpansionSeeded = false;

/// Mark the expansion state as "owned" by the user (or the
/// session-restore path). Called by restoreSession when a
/// treeExpanded payload is present so the auto-seed doesn't
/// override it.
export function markTreeExpansionRestored(): void {
  treeExpansionSeeded = true;
}

/// First-paint default: expand every directory so a new user
/// doesn't land on a single collapsed root. Idempotent; only seeds
/// when no prior state exists. Called from `refreshTree` after
/// entries arrive.
function seedTreeExpansionIfFresh(): void {
  if (treeExpansionSeeded) return;
  treeExpansionSeeded = true;
  treeExpanded.map[""] = true;
  for (const e of tree.entries) {
    if (e.is_dir) treeExpanded.map[e.path] = true;
  }
}

/// Expand every directory in the current tree. Wired to the file
/// browser's expand-all header button. Mutates the existing map
/// proxy in place so consumers that captured `treeExpanded.map` at
/// mount time (FileTree.svelte) keep seeing the live state.
export function expandAllFolders(): void {
  treeExpanded.map[""] = true;
  for (const e of tree.entries) {
    if (e.is_dir) treeExpanded.map[e.path] = true;
  }
  treeExpansionSeeded = true;
  persistTreeExpanded();
}

/// Collapse every directory (top-level rows still render; their
/// children are hidden). Keeps the implicit root key alive so
/// FileTree's pre-order walk stays consistent. Mutates in place
/// for the same reason as `expandAllFolders`.
export function collapseAllFolders(): void {
  for (const k of Object.keys(treeExpanded.map)) {
    if (k !== "") delete treeExpanded.map[k];
  }
  treeExpanded.map[""] = true;
  treeExpansionSeeded = true;
  persistTreeExpanded();
}

/// Reveal a path in the file browser tree: expand every ancestor
/// folder so the row is visible, then set the browser selection to
/// it. FileTree's selection-change effect scrolls the row into
/// view. Called after a successful create / move so the user lands
/// next to whatever they just produced instead of having to hunt
/// for it. Walks the path segment-by-segment rather than the
/// entries list because the new entry may not be in the snapshot
/// yet (the tree refresh may still be in flight).
export function revealAndSelect(path: string): void {
  const parts = path.split("/");
  let acc = "";
  for (let i = 0; i < parts.length - 1; i++) {
    acc = acc ? `${acc}/${parts[i]}` : parts[i];
    treeExpanded.map[acc] = true;
  }
  treeExpanded.map[""] = true;
  browserSelection.path = path;
  // The expansion change counts as a user action — persist it so
  // the next launch keeps the new entry in view.
  persistTreeExpanded();
}

/// True when every directory in the current tree is expanded.
/// Drives the header toggle's glyph + title.
export function isFullyExpanded(): boolean {
  for (const e of tree.entries) {
    if (e.is_dir && !treeExpanded.map[e.path]) return false;
  }
  return true;
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

// ---- in-page confirm ----------------------------------------------------
//
// `window.confirm()` shares the same WKWebView gap as `window.prompt()`,
// and we want destructive-action confirmation on overwrite. Same shape
// as PromptState minus the input.

type ConfirmState = {
  open: boolean;
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  destructive: boolean;
  resolve: ((value: boolean) => void) | null;
};

export const confirmState = $state<ConfirmState>({
  open: false,
  title: "",
  message: "",
  confirmLabel: "OK",
  cancelLabel: "Cancel",
  destructive: false,
  resolve: null,
});

/// Show a confirm dialog. Resolves true on OK, false on Cancel / Esc /
/// outside-click. Pass `destructive: true` to style the OK button as a
/// warning so overwrite / delete reads correctly. Replaces `window.confirm`.
export function uiConfirm(opts: {
  title: string;
  message?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
}): Promise<boolean> {
  return new Promise((resolve) => {
    // If a confirm is already open, drop the previous one as cancelled.
    confirmState.resolve?.(false);
    confirmState.title = opts.title;
    confirmState.message = opts.message ?? "";
    confirmState.confirmLabel = opts.confirmLabel ?? "OK";
    confirmState.cancelLabel = opts.cancelLabel ?? "Cancel";
    confirmState.destructive = opts.destructive ?? false;
    confirmState.resolve = resolve;
    confirmState.open = true;
  });
}

/// Called by the modal component on OK / Cancel.
export function resolveConfirm(value: boolean): void {
  const r = confirmState.resolve;
  confirmState.resolve = null;
  confirmState.open = false;
  r?.(value);
}

// ---- in-page path prompt ------------------------------------------------
//
// Richer cousin of uiPrompt for typing relative paths: live folder
// autocomplete from the loaded tree, parent-creation hints, overwrite
// warnings, and client-side validation. Used by file create / move /
// rename. The plain uiPrompt stays around for label-only inputs (drive
// rename, etc.).
//
// `kind` distinguishes the two entity classes the user can be naming:
// a file (default `.md` will be appended on submit if no extension) or
// a folder. The modal uses it to label the status row and to decide
// what the autocomplete should suggest.
//
// `mode` controls how an existing target at the typed path is treated:
//   - "create": existing target is a hard error (cannot overwrite via
//     create; the user should rename or pick a different name).
//   - "move":   existing target is a soft warning; the caller will
//     fire a separate uiConfirm before performing the destructive
//     action.

export type PathPromptKind = "file" | "folder";
export type PathPromptMode = "create" | "move";

type PathPromptState = {
  open: boolean;
  title: string;
  defaultValue: string;
  kind: PathPromptKind;
  mode: PathPromptMode;
  /// Set on "move": the path being renamed. The modal hides this from
  /// the autocomplete (no point suggesting "move to itself") and the
  /// "overwrites" check ignores it (renaming `a` → `a` is a no-op,
  /// not an overwrite).
  sourcePath: string | null;
  /// Optional caller-supplied validator run against the effective
  /// (post-extension-resolution) path. Returns a human-readable
  /// reason when the path should be rejected, or null when it's
  /// fine. The modal surfaces the reason in the red status row and
  /// disables Submit, so the user fixes the input in-place instead
  /// of bouncing through a status-bar error after the dialog
  /// closes. Used today by createFile to enforce the .md/.txt
  /// editable-text gate up front.
  validate: ((effectivePath: string) => string | null) | null;
  resolve: ((value: string | null) => void) | null;
};

export const pathPromptState = $state<PathPromptState>({
  open: false,
  title: "",
  defaultValue: "",
  kind: "file",
  mode: "create",
  sourcePath: null,
  validate: null,
  resolve: null,
});

export function uiPathPrompt(opts: {
  title: string;
  defaultValue?: string;
  kind: PathPromptKind;
  mode: PathPromptMode;
  sourcePath?: string | null;
  validate?: (effectivePath: string) => string | null;
}): Promise<string | null> {
  return new Promise((resolve) => {
    pathPromptState.resolve?.(null);
    pathPromptState.title = opts.title;
    pathPromptState.defaultValue = opts.defaultValue ?? "";
    pathPromptState.kind = opts.kind;
    pathPromptState.mode = opts.mode;
    pathPromptState.sourcePath = opts.sourcePath ?? null;
    pathPromptState.validate = opts.validate ?? null;
    pathPromptState.resolve = resolve;
    pathPromptState.open = true;
  });
}

export function resolvePathPrompt(value: string | null): void {
  const r = pathPromptState.resolve;
  pathPromptState.resolve = null;
  pathPromptState.validate = null;
  pathPromptState.open = false;
  r?.(value);
}

/// File CRUD helpers shared by the sidebar header and the tree's
/// context menu / hover actions. They wrap the raw API with prompts,
/// confirmations, and a tree refresh, plus opening the new file in the
/// active pane on create. Surfacing one set of behaviors via several
/// affordances keeps the actions consistent regardless of which entry
/// point the user reaches for.

// `appendDefaultMd` moved to ../state/pathValidate so PathPromptModal
// can preview the auto-extension live; we re-import below.

// `preserveExtension` moved to ../state/pathValidate so the
// PathPromptModal can preview the rename-with-preserved-extension
// inline. We re-import above; the call below is now a defensive
// idempotent layer (the modal already resolved the extension).

/// Perform a move from `path` -> `target`. Shared by rename (CLI-style
/// prompt) and drag-and-drop. No-ops if source == target. Prompts for
/// overwrite confirmation if target already exists. Refreshes the tree
/// and re-keys conversations + open tabs so in-memory state follows the
/// rename without a refetch round-trip.
///
/// The server runs the rename + link-rewrite pass synchronously. For a
/// single-file rename with few backlinks this is sub-100ms; for a
/// directory rename touching dozens of inbound references it can take a
/// few hundred ms. We show a "Moving…" status indicator after a 200ms
/// delay so a fast rename doesn't flash an indicator, but a slow one
/// still tells the user the UI hasn't frozen.
const MOVING_STATUS_DELAY_MS = 200;
async function performMove(path: string, target: string): Promise<void> {
  if (target === path) return;
  const existing = tree.entries.find((e) => e.path === target);
  if (existing) {
    const what = existing.is_dir ? "folder" : "file";
    const confirmed = await uiConfirm({
      title: `Overwrite existing ${what}?`,
      message: `'${target}' already exists. The current ${what} will be replaced.`,
      confirmLabel: "Overwrite",
      destructive: true,
    });
    if (!confirmed) return;
  }
  let movingTimer: ReturnType<typeof setTimeout> | null = setTimeout(() => {
    ui.status = "Moving…";
    movingTimer = null;
  }, MOVING_STATUS_DELAY_MS);
  // Mark both endpoints so the watcher handler ignores echoes of
  // this rename while the move is in flight (see `movingPaths`).
  movingPaths.add(path);
  movingPaths.add(target);
  try {
    const resp = await api.move(path, target);
    await refreshTree();
    rekeyConversationsForRename(path, target);
    rekeyTabsForRename(path, target);
    // Defensive: if a watcher event slipped through before the Set
    // was populated (or in any future code path that bypasses it),
    // clear any "file not found" error sitting on the moved tab so
    // the user doesn't keep staring at a stale message.
    for (const { tabId } of tabsForPath(target)) {
      clearTabError(tabId);
    }
    revealAndSelect(target);
    // Nudge open tabs to re-check their underlying file. Server-side
    // self_writes dedupe suppresses the watcher echo for our own
    // rewrites, so without this bump a tab pointing at a rewritten
    // source would keep its stale buffer until the next save (which
    // would then surface as a CAS conflict).
    if (resp.rewritten.length > 0) {
      ui.lastWatch = Date.now();
    }
    const linkBits: string[] = [];
    if (resp.rewritten.length > 0) {
      linkBits.push(
        `${resp.rewritten.length} link${resp.rewritten.length === 1 ? "" : "s"} updated`,
      );
    }
    if (resp.conflicts.length > 0) {
      linkBits.push(
        `${resp.conflicts.length} conflict${resp.conflicts.length === 1 ? "" : "s"}`,
      );
    }
    ui.status =
      linkBits.length > 0
        ? `Moved '${target}' (${linkBits.join(", ")})`
        : null;
  } catch (e) {
    ui.status = `rename failed: ${(e as Error).message}`;
  } finally {
    if (movingTimer) clearTimeout(movingTimer);
    if (ui.status === "Moving…") ui.status = null;
    movingPaths.delete(path);
    movingPaths.delete(target);
  }
}

export const fileOps = {
  async createFile(parentPath: string): Promise<void> {
    // Pre-populate the input with the parent prefix so the user
    // only types the basename. Folder autocomplete still kicks in
    // once they touch any other folder under the same prefix.
    const defaultValue = parentPath ? `${parentPath}/` : "";
    const name = await uiPathPrompt({
      title: "new file (relative path; .md added if no extension)",
      defaultValue,
      kind: "file",
      mode: "create",
      // Enforce the editable-text gate inline. The modal calls this
      // against the effective path (post `.md` auto-append) so a
      // user who types `foo.foo` sees the rejection in the dialog
      // and can correct it, instead of submitting and getting a
      // status-bar error after the prompt closes.
      validate: (path) =>
        isEditableText(path)
          ? null
          : `'${path}' is not an editable text file (only .md and .txt)`,
    });
    if (!name) return;
    // The modal already validated against `isEditableText`, but
    // appendDefaultMd is idempotent and the cost is trivial; keep it
    // here as a defensive backstop in case any caller bypasses the
    // modal validator in the future.
    const path = appendDefaultMd(name);
    try {
      await api.create(path, false, "");
      await refreshTree();
      await openInActivePane(path);
      // Editor took over; close the file browser so the user lands
      // on the new tab instead of seeing the tree above the editor.
      // Mirrors the close-on-open behavior the inspector's Open
      // button uses.
      browserOverlay.open = false;
    } catch (e) {
      ui.status = `create failed: ${(e as Error).message}`;
    }
  },
  async createDir(parentPath: string): Promise<void> {
    const defaultValue = parentPath ? `${parentPath}/` : "";
    const path = await uiPathPrompt({
      title: "new folder",
      defaultValue,
      kind: "folder",
      mode: "create",
    });
    if (!path) return;
    try {
      await api.create(path, true);
      await refreshTree();
      // Folder creation leaves the user inside the file browser
      // (unlike file creation, which jumps straight into an editor
      // tab), so reveal the new folder and select it. Expands every
      // ancestor along the way so a `a/b/new-folder` create lands
      // visible even if `a` and `b` were collapsed.
      revealAndSelect(path);
    } catch (e) {
      ui.status = `create failed: ${(e as Error).message}`;
    }
  },
  /// Rename the drive (display name only, the on-disk directory
  /// stays put). The name is registry metadata returned in
  /// DriveInfo.name and shown in the file-browser header. No-op
  /// when the user cancels or the input is unchanged; on success
  /// the fresh DriveInfo from the PATCH response is written back
  /// into the drive store so the header updates without an extra
  /// refresh round-trip.
  async renameDrive(): Promise<void> {
    const current = drive.info?.name ?? "";
    const next = await uiPrompt("drive name", current);
    if (next === null) return;
    const trimmed = next.trim();
    if (trimmed === current) return;
    try {
      const info = await api.updatePreferences({ name: trimmed });
      drive.info = info;
    } catch (e) {
      ui.status = `rename failed: ${(e as Error).message}`;
    }
  },
  /// Rename or move a file / directory. `isDir` toggles the
  /// extension-preservation step: for files, if the user drops the
  /// existing extension during the prompt (typed `note` instead of
  /// `note.md`), put it back so the resulting path still routes
  /// through the editor's text gate. Directories don't have
  /// extensions so the input is taken verbatim.
  ///
  /// If the resolved target collides with an existing entry, we
  /// stop for a uiConfirm before issuing the move. The PathPrompt
  /// modal already shows a warning row, but the user might commit
  /// past it on Enter, so the confirm dialog is the second gate
  /// before any destructive overwrite.
  async rename(path: string, isDir = false): Promise<void> {
    const next = await uiPathPrompt({
      title: "new path",
      defaultValue: path,
      kind: isDir ? "folder" : "file",
      mode: "move",
      sourcePath: path,
    });
    if (!next || next === path) return;
    const target = isDir ? next : preserveExtension(path, next);
    await performMove(path, target);
  },
  /// Move a file or directory to a new path without prompting. Used
  /// by drag-and-drop in the file browser. Same overwrite-confirm and
  /// post-move bookkeeping as rename.
  async moveTo(from: string, to: string): Promise<void> {
    await performMove(from, to);
  },
  /// Delete a file (or directory) from the drive.
  ///
  /// Closes any open tabs pointing at the deleted path (or paths
  /// under it, for directory deletes) and drops the per-file
  /// assistant conversation so .chan/assistant/ doesn't accumulate
  /// orphan blobs.
  ///
  /// Prompts via uiConfirm with destructive styling. For directories
  /// the message includes the descendant count so the user sees the
  /// blast radius before confirming.
  async remove(path: string, isDir = false): Promise<void> {
    const name = path.split("/").pop() ?? path;
    let message: string;
    if (isDir) {
      const prefix = `${path}/`;
      const descendants = tree.entries.filter((e) =>
        e.path.startsWith(prefix),
      ).length;
      message =
        descendants === 0
          ? `Delete folder "${name}"?`
          : `Delete folder "${name}" and its ${descendants} item${descendants === 1 ? "" : "s"}?`;
    } else {
      message = `Delete "${name}"?`;
    }
    const ok = await uiConfirm({
      title: "Delete",
      message,
      confirmLabel: "Delete",
      destructive: true,
    });
    if (!ok) return;
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
  /// Duplicate a file in-place. Reads the source via the API so any
  /// unsaved buffer in the open tab is intentionally ignored — the
  /// duplicate mirrors what's on disk, not what's in the editor.
  /// Resolves the next free `name-copy.ext`, `name-copy-2.ext`, ...
  /// under the same directory, creates the file, refreshes the tree,
  /// and opens the new tab next to the source.
  async duplicateFile(path: string): Promise<void> {
    try {
      const src = await api.read(path);
      const target = nextDuplicateName(path);
      await api.create(target, false, src.content);
      await refreshTree();
      await openInActivePane(target);
      revealAndSelect(target);
    } catch (e) {
      ui.status = `duplicate failed: ${(e as Error).message}`;
    }
  },
};

/// Compute the next available "name-copy{,-N}.ext" sibling for
/// `path`. Looks at the current tree to avoid collisions; the
/// server still owns the final word (a concurrent create elsewhere
/// could race), and the create call will surface that error.
function nextDuplicateName(path: string): string {
  const slash = path.lastIndexOf("/");
  const dir = slash < 0 ? "" : path.slice(0, slash + 1);
  const base = slash < 0 ? path : path.slice(slash + 1);
  const dot = base.lastIndexOf(".");
  const stem = dot > 0 ? base.slice(0, dot) : base;
  const ext = dot > 0 ? base.slice(dot) : "";
  const has = (p: string): boolean => tree.entries.some((e) => e.path === p);
  let candidate = `${dir}${stem}-copy${ext}`;
  if (!has(candidate)) return candidate;
  for (let n = 2; n < 1000; n++) {
    candidate = `${dir}${stem}-copy-${n}${ext}`;
    if (!has(candidate)) return candidate;
  }
  // Fall through with a timestamp suffix to break the unlikely tie.
  return `${dir}${stem}-copy-${Date.now()}${ext}`;
}
