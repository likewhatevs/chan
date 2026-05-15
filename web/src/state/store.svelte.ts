// Global app state, written with Svelte 5 runes ($state).
// One module-level singleton per concern; components import them directly.

import type { IndexStatus, LlmMessage, TreeEntry, DriveInfo } from "../api/types";
import { ApiError, api, assistantHash16, authToken, openWatchSocket, type WsStatus } from "../api/client";
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
  /// Set when the SPA shell loaded but bootstrap's first API call
  /// returned 401 and there was no token in the URL or sessionStorage.
  /// Drives `MissingTokenOverlay`. Users land here when they copy the
  /// loopback URL out of the address bar but lose the `?t=...` token
  /// the server prints at launch.
  authMissing: boolean;
}>({
  status: null,
  lastWatch: 0,
  ws: "connecting",
  themeChoice: "system",
  theme: effectiveTheme("system"),
  authMissing: false,
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
    // Older servers don't ship `outline`; fall back to the default
    // so the file-editor outline pane has a sane width on first use.
    paneWidths.outline = prefs.pane_widths.outline ?? DEFAULT_PANE_WIDTHS.outline;
    // Older servers don't ship `assistant`; fall back to the default
    // so the assistant overlay's inspector has a sane width.
    paneWidths.assistant =
      prefs.pane_widths.assistant ?? DEFAULT_PANE_WIDTHS.assistant;
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
  // chan-llm session events: delta text, tool calls, tool results,
  // terminal done/error. Filter by session_id so a stale frame from
  // a previous turn (or a sibling window) doesn't bleed into the
  // current bubble.
  if (
    frameType === "llm.delta" ||
    frameType === "llm.tool_call" ||
    frameType === "llm.tool_result" ||
    frameType === "llm.done" ||
    frameType === "llm.error"
  ) {
    const f = e as {
      session_id?: string;
      text?: string;
      error?: string;
      call?: { id?: string; name?: string; args?: unknown };
      result?: { id?: string; output?: unknown };
    };
    if (!assistantStream.sessionId || f.session_id !== assistantStream.sessionId) {
      return;
    }
    if (frameType === "llm.delta") {
      assistantStream.text += f.text ?? "";
    } else if (frameType === "llm.tool_call") {
      // Narrate the call inline in the chat by appending a `tool`
      // turn to the currently-active conversation. The user sees
      // chips like `reading docs/foo.md` / `searching "X"` /
      // `listing drive` form as the model works through its tool
      // loop. Skip `write_file` (bare or MCP-namespaced): those
      // calls become richer edit cards via handleResponse so the
      // user can Apply / Discard.
      const rawName = typeof f.call?.name === "string" ? f.call.name : "";
      const callId = typeof f.call?.id === "string" ? f.call.id : "";
      if (rawName && callId) {
        const bare = bareToolName(rawName);
        if (bare !== "write_file") {
          const conv = currentAssistantConversation();
          if (conv) {
            const now = Date.now();
            conv.turns.push({
              kind: "tool",
              event: {
                tool_call_id: callId,
                name: bare,
                label: labelForToolCall(rawName, f.call?.args),
                status: "running",
                result_summary: null,
                created_at: now,
              },
              created_at: now,
            });
          }
        }
      }
    } else if (frameType === "llm.tool_result") {
      // Capture every tool_result chan-llm emits during this turn so
      // the synchronous /api/llm/complete consumer (InlineAssist) can
      // rebuild the message history. The HTTP response only carries
      // assistant text + tool_calls — tool results live exclusively
      // on the WS side channel.
      const id = f.result?.id;
      if (typeof id === "string" && id.length > 0) {
        assistantStream.toolResults[id] = f.result?.output ?? null;
        // Find the matching tool chip in the active conversation
        // (walk backward — the call is usually the most-recent
        // running entry) and flip it to ok/error with a short
        // result summary. The chip stays in the scrollback as
        // permanent history.
        const conv = currentAssistantConversation();
        if (conv) {
          for (let i = conv.turns.length - 1; i >= 0; i--) {
            const t = conv.turns[i];
            if (t && t.kind === "tool" && t.event.tool_call_id === id) {
              const out = f.result?.output;
              const err = isErrorOutput(out);
              t.event.status = err ? "error" : "ok";
              t.event.result_summary = summarizeToolResult(out);
              break;
            }
          }
        }
      }
    } else if (frameType === "llm.error") {
      assistantStream.error = f.error ?? "stream error";
    }
    // llm.done is just a marker; the POST handler completes via the
    // synchronous JSON response. We don't end the stream here because
    // the caller's `finally` block does it after the response is
    // committed to the conversation log.
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
    // Friendly path for the common "copied the URL without the token"
    // case: the SPA shell is static and loads fine, but the first
    // /api call comes back 401. Surface the missing-token overlay
    // instead of a terse status-bar message buried in the corner.
    if (e instanceof ApiError && e.status === 401 && authToken() === null) {
      ui.authMissing = true;
      return;
    }
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
const HASH_SCOPE_HISTORY = "scopes";

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
    // Encoding: `<inspectorBit>:<contextId>|<prompt>`. The inspector
    // bit + colon is optional (legacy hashes without it parse as
    // contextId starting at offset 0); the prompt is everything
    // after the first `|` (so it may itself contain `|`).
    const raw = params.get(HASH_ASSIST) ?? "";
    const [ins, body] = splitInspectorBit(raw);
    if (ins !== null) assistantOverlay.inspectorOpen = ins;
    const sep = body.indexOf("|");
    if (sep === -1) {
      if (body) assistantOverlay.contextId = body;
      assistantOverlay.prompt = "";
    } else {
      const ctx = body.slice(0, sep);
      if (ctx) assistantOverlay.contextId = ctx;
      assistantOverlay.prompt = body.slice(sep + 1);
    }
    assistantOverlay.open = true;
  }
  if (params.has(HASH_SETTINGS) && !SETTINGS_DISABLED) {
    settingsOverlay.open = true;
  }
  if (params.has(HASH_SCOPE_HISTORY)) {
    // Going through `openScopeHistory` rather than flipping `.open`
    // directly so the refresh fires on the restored open path the
    // same way it does for a fresh user-triggered open.
    openScopeHistory();
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
    const ins = assistantOverlay.inspectorOpen ? "1" : "0";
    let body = assistantOverlay.contextId;
    if (assistantOverlay.prompt) body = `${body}|${assistantOverlay.prompt}`;
    params.set(HASH_ASSIST, `${ins}:${body}`);
  } else {
    params.delete(HASH_ASSIST);
  }
  if (settingsOverlay.open) {
    params.set(HASH_SETTINGS, "1");
  } else {
    params.delete(HASH_SETTINGS);
  }
  if (scopeHistoryOverlay.open) {
    params.set(HASH_SCOPE_HISTORY, "1");
  } else {
    params.delete(HASH_SCOPE_HISTORY);
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

/// InlineAssist registers a pending-save flush callback here so the
/// pagehide hook can reach it without importing the component.
/// Mirrors the session-flush shape: the callback uses keepalive fetch
/// so the PUT survives the page unload. Without this, a user who
/// types + sends + closes the window inside the 400 ms scheduleSave
/// debounce loses the most recent edit.
let assistantSaveFlush: (() => void) | null = null;
export function setAssistantSaveFlush(cb: (() => void) | null): void {
  assistantSaveFlush = cb;
}
function flushAssistantSavesOnExit(): void {
  assistantSaveFlush?.();
}

/// Register the pagehide flush once. Idempotent so HMR re-evaluations
/// don't stack listeners. Also tears down any in-flight assistant
/// request on the way out: leaving the AbortController alive across
/// page unload doesn't accomplish anything (the fetch is doomed
/// either way), and emitting the abort before the WebSocket closes
/// gives the server a chance to clean up the chan-llm subprocess on
/// the matching side (claude / gemini CLIs would otherwise linger
/// until their stream timeout fires).
let pagehideHooked = false;
export function installSessionFlushHook(): void {
  if (pagehideHooked || typeof window === "undefined") return;
  pagehideHooked = true;
  window.addEventListener("pagehide", flushSessionSaveOnExit);
  window.addEventListener("pagehide", flushAssistantSavesOnExit);
  window.addEventListener("pagehide", cancelAssistantStream);
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
/// Live narration of one tool call inside the assistant's chat
/// timeline. Created by the `llm.tool_call` WS handler the moment
/// the model invokes a tool, then updated to `ok` / `error` (with
/// a short result summary) when the matching `llm.tool_result`
/// frame arrives. Rendered inline in the chat scrollback as a
/// compact chip; persists alongside regular turns so reopening
/// the overlay shows the full tool-loop history.
///
/// Excludes `write_file` calls: those get their own richer edit
/// card via the existing `AssistantPendingEdit` pathway because
/// they require user action (Apply / Discard).
export type AssistantToolEvent = {
  tool_call_id: string;
  /// Bare tool name (the `mcp__<server>__` prefix is stripped by
  /// the WS handler before construction). Backend-agnostic.
  name: string;
  /// Pre-rendered human-readable label, e.g. `reading docs/foo.md`.
  /// Same shape used for the status pill so the two surfaces stay
  /// in sync without recomputing.
  label: string;
  status: "running" | "ok" | "error";
  /// Short tail like `12 hits`, `1.2 KB`, `67 entries`. Null while
  /// the call is still in flight; null when the result shape
  /// didn't yield an obvious summary (the chip still renders
  /// status alone).
  result_summary: string | null;
  created_at: number;
};

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
/// Render mode applied to every chat bubble (user and assistant).
/// Lives as a global preference rather than per-bubble: the user
/// picks once and every conversation reflects the choice. Same
/// persistence shape as `pageWidth` (localStorage), so a reload
/// or restart restores the last selection.
///
///   - "editor": read-only chan Wysiwyg (default). Renders the
///     full editor with wiki / tag / mention / date widgets.
///   - "rendered": sanitized GFM HTML via marked + DOMPurify.
///   - "source": raw markdown text in a monospace block.
export type BubbleDisplayMode = "editor" | "rendered" | "source";

// v2 of the key: the default flipped from "editor" back to
// "rendered". Bumping the key drops the stale per-browser cache so
// users land on the new default; explicit picks happen against the
// new key going forward.
const BUBBLE_MODE_STORAGE_KEY = "chan.assistant.bubbleMode.v2";
const DEFAULT_BUBBLE_MODE: BubbleDisplayMode = "rendered";

function readBubbleMode(): BubbleDisplayMode {
  try {
    const raw = localStorage.getItem(BUBBLE_MODE_STORAGE_KEY);
    if (raw === "editor" || raw === "rendered" || raw === "source") return raw;
  } catch {
    // localStorage can throw in private-mode Safari; fall through.
  }
  return DEFAULT_BUBBLE_MODE;
}

export const bubbleDisplayMode = $state<{ value: BubbleDisplayMode }>({
  value: readBubbleMode(),
});

/// Update the global bubble render mode and persist. Tabs / windows
/// that read `bubbleDisplayMode.value` reactively will rerender;
/// the storage event hook below also picks the change up in
/// sibling browser tabs.
export function setBubbleDisplayMode(m: BubbleDisplayMode): void {
  if (bubbleDisplayMode.value === m) return;
  bubbleDisplayMode.value = m;
  try {
    localStorage.setItem(BUBBLE_MODE_STORAGE_KEY, m);
  } catch {
    // Same safety net as readBubbleMode.
  }
}

/// Subscribe to localStorage changes from sibling tabs / windows so
/// flipping the mode in one window propagates everywhere live.
/// Mirrors `watchPageWidth`. Returns a disposer; call once from
/// the app bootstrap.
export function watchBubbleDisplayMode(): () => void {
  if (typeof window === "undefined") return () => {};
  const handler = (e: StorageEvent) => {
    if (e.key !== BUBBLE_MODE_STORAGE_KEY) return;
    const next = e.newValue;
    if (next === "editor" || next === "rendered" || next === "source") {
      bubbleDisplayMode.value = next;
    }
  };
  window.addEventListener("storage", handler);
  return () => window.removeEventListener("storage", handler);
}

/// Per-turn auto-apply state. The composer toggle next to Send sets
/// this; every /api/llm/complete request forwards it as
/// `auto_apply_writes` so the MCP bridge sees the live value when
/// claude-cli / gemini-cli's MCP child connects. Persisted to
/// localStorage so the user's last choice survives reload; default
/// is `false` (safe: writes pause for review through the diff card).
const AUTO_APPLY_STORAGE_KEY = "chan.assistant.autoApply";

function readAutoApply(): boolean {
  try {
    return localStorage.getItem(AUTO_APPLY_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

export const autoApplyWrites = $state<{ value: boolean }>({
  value: readAutoApply(),
});

export function setAutoApplyWrites(v: boolean): void {
  if (autoApplyWrites.value === v) return;
  autoApplyWrites.value = v;
  try {
    localStorage.setItem(AUTO_APPLY_STORAGE_KEY, v ? "true" : "false");
  } catch {
    // private-mode Safari et al.
  }
}

export function watchAutoApplyWrites(): () => void {
  if (typeof window === "undefined") return () => {};
  const handler = (e: StorageEvent) => {
    if (e.key !== AUTO_APPLY_STORAGE_KEY) return;
    autoApplyWrites.value = e.newValue === "true";
  };
  window.addEventListener("storage", handler);
  return () => window.removeEventListener("storage", handler);
}

export type AssistantTurn =
  | { kind: "user"; content: string; created_at?: number }
  | {
      kind: "assistant";
      content: string;
      created_at?: number;
      citations?: import("../api/types").ContentHit[];
    }
  | { kind: "edit"; edit: AssistantPendingEdit; created_at?: number }
  | { kind: "tool"; event: AssistantToolEvent; created_at?: number };

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
  /// True when an assistant response landed while the user wasn't
  /// looking at this conversation (overlay closed, or open on a
  /// different scope). Cleared the next time the user opens the
  /// overlay on this conversation. Surfaced on file tabs as a
  /// bell-icon swap so the user knows a background reply is
  /// waiting. Persists with the conversation so a chan relaunch
  /// preserves the unread signal.
  hasUnread?: boolean;
  /// Position in `turns` at the moment the user picked "close,
  /// keep running" on a request still in flight. Anything added
  /// after this index landed while the user was away; the chat
  /// renders a one-time divider at this position so the user
  /// can see where they left off. Cleared when the user reopens
  /// the conversation (one-shot signal, not a permanent bookmark).
  lastSeenTurnIndex?: number;
  /// ms-since-epoch the conversation was first created. Set when
  /// the first turn lands; preserved across all subsequent saves
  /// so the scope-history overlay can show how long a thread has
  /// been alive. Backfilled on load from the earliest turn's
  /// `created_at` for blobs written before this field existed.
  created_at?: number;
  /// ms-since-epoch of the most recent persistence write. Updated
  /// on every save (file + group); read by the scope-history
  /// overlay for the "last activity" timestamp.
  last_touched?: number;
  /// Origin-relative URL (pathname + hash, no search params) at
  /// the moment of the most recent save. The pane / tab layout is
  /// encoded in the hash, so re-opening this URL in a new window
  /// rehydrates the same pane configuration that was active when
  /// the user last interacted with this scope. Search params are
  /// stripped because they include the auth token; the new
  /// window's origin contributes its own.
  url?: string;
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

/** Drop the drive conversation, in memory AND on disk. */
export function clearDriveConversation(): void {
  assistantConversations.drive = null;
  void deleteDriveConversation();
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

export function blobKeyForGroupHash(hash: string): string {
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

/// Snapshot the current pane / tab layout as an origin-relative
/// URL. The layout encoder writes it to `location.hash`; the
/// auth token rides on the search params which we drop so the
/// stored URL can be opened in a different window / launch with
/// its own token. Returns null in non-browser contexts (tests).
export function currentLayoutUrl(): string | undefined {
  if (typeof window === "undefined") return undefined;
  return window.location.pathname + window.location.hash;
}

/// Earliest `created_at` across `turns`, or undefined if none
/// carry a timestamp. Used to backfill `AssistantConversation.created_at`
/// when loading a blob that predates the field.
export function earliestTurnCreatedAt(turns: AssistantTurn[]): number | undefined {
  let earliest: number | undefined;
  for (const t of turns) {
    if (t.created_at === undefined) continue;
    if (earliest === undefined || t.created_at < earliest) earliest = t.created_at;
  }
  return earliest;
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
      created_at?: number;
      last_touched?: number;
      url?: string;
    };
    const turns = parsed.turns ?? [];
    assistantConversations.byGroup[key] = {
      messages: parsed.messages ?? [],
      turns,
      created_at: parsed.created_at ?? earliestTurnCreatedAt(turns),
      last_touched: parsed.last_touched,
      url: parsed.url,
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
  // Stamp the conversation as we go so the in-memory view stays in
  // sync with what we just wrote (the scope-history overlay reads
  // these fields without a re-fetch).
  if (conv.created_at === undefined) {
    conv.created_at = earliestTurnCreatedAt(conv.turns) ?? now;
  }
  conv.last_touched = now;
  conv.url = currentLayoutUrl();
  try {
    await api.putAssistantBlob(blobKey, {
      schema_version: 1,
      kind: "group",
      key,
      paths,
      messages: conv.messages,
      turns: conv.turns,
      created_at: conv.created_at,
      last_touched: now,
      url: conv.url,
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

// ---- drive conversation persistence -------------------------------------
//
// The drive-scope conversation used to live in-memory only because
// the retrieval-driven excerpts felt too lossy to replay later. The
// scope-history overlay flips that calculus: a persisted thread is
// the only way the user can revisit a previous drive Q&A.
//
// Same blob store as file / group conversations; fixed key so the
// listing endpoint (`/api/assistant/conversations`) surfaces it
// without a manifest. `loadDriveConversation` is idempotent and
// safe to call any time the drive scope is about to be opened.

export const DRIVE_BLOB_KEY = "drive.json";

export async function loadDriveConversation(): Promise<void> {
  if (assistantConversations.drive) return; // already in memory
  try {
    const raw = await api.getAssistantBlob(DRIVE_BLOB_KEY);
    if (!raw) return;
    if (assistantConversations.drive) return; // race
    const parsed = raw as {
      messages?: LlmMessage[];
      turns?: AssistantTurn[];
      created_at?: number;
      last_touched?: number;
      url?: string;
    };
    const turns = parsed.turns ?? [];
    assistantConversations.drive = {
      messages: parsed.messages ?? [],
      turns,
      created_at: parsed.created_at ?? earliestTurnCreatedAt(turns),
      last_touched: parsed.last_touched,
      url: parsed.url,
    };
  } catch {
    // Server unreachable / decode error: leave drive null so the
    // next submit creates a fresh thread.
  }
}

export async function saveDriveConversation(
  conv: AssistantConversation,
): Promise<void> {
  const now = Date.now();
  if (conv.created_at === undefined) {
    conv.created_at = earliestTurnCreatedAt(conv.turns) ?? now;
  }
  conv.last_touched = now;
  conv.url = currentLayoutUrl();
  try {
    await api.putAssistantBlob(DRIVE_BLOB_KEY, {
      schema_version: 1,
      kind: "drive",
      messages: conv.messages,
      turns: conv.turns,
      created_at: conv.created_at,
      last_touched: now,
      url: conv.url,
    });
  } catch {
    // Best-effort: server outage doesn't block the in-memory thread.
  }
}

export async function deleteDriveConversation(): Promise<void> {
  try {
    await api.deleteAssistantBlob(DRIVE_BLOB_KEY);
  } catch {
    // Same best-effort policy as the group eviction path.
  }
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
  /// Style toolbar visibility. Mirrors the per-tab knob in the
  /// file editor; the toolbar mounts only when this flips on,
  /// and the prompt's top padding grows to keep the first line
  /// clear of the floating pill.
  styleToolbarOpen: boolean;
  /// Inspector pane visibility. Matches the FileBrowser / Graph /
  /// Search inspectors: a right-side aside with provider / model /
  /// max-tokens controls scoped to the active assistant. Auto-opens
  /// on the first visit to a new scope (see `assistantScopesSeen`);
  /// after that the user toggles it via the header menu.
  inspectorOpen: boolean;
}>({
  open: false,
  contextId: "drive",
  prompt: "",
  styleToolbarOpen: false,
  inspectorOpen: false,
});

/// Set of scope ids the assistant has been opened against. Used by
/// InlineAssist to auto-open the inspector on the first visit to a
/// new scope so the user is prompted to pick a model right away.
/// Persisted to localStorage (per-machine, per-browser) because
/// this is purely a UX nudge — losing it on a fresh browser just
/// re-pops the inspector once.
const ASSISTANT_SCOPES_SEEN_KEY = "chan.assistantScopesSeen";

function loadScopesSeen(): Set<string> {
  try {
    const raw = localStorage.getItem(ASSISTANT_SCOPES_SEEN_KEY);
    if (!raw) return new Set();
    const arr = JSON.parse(raw) as unknown;
    if (!Array.isArray(arr)) return new Set();
    return new Set(arr.filter((s): s is string => typeof s === "string"));
  } catch {
    return new Set();
  }
}

function saveScopesSeen(set: Set<string>): void {
  try {
    localStorage.setItem(ASSISTANT_SCOPES_SEEN_KEY, JSON.stringify([...set]));
  } catch {
    // localStorage can be full / blocked (private mode); the inspector
    // will just re-pop next visit, which is the worst outcome.
  }
}

export const assistantScopesSeen = $state<{ ids: Set<string> }>({
  ids: loadScopesSeen(),
});

/// Mark a scope id as seen and persist. Idempotent. InlineAssist
/// calls this once on every overlay open: if the id was already in
/// the set the inspector stays in whatever state the user left it,
/// otherwise the caller flips `inspectorOpen = true` for first-time
/// onboarding.
export function markAssistantScopeSeen(id: string): boolean {
  if (assistantScopesSeen.ids.has(id)) return true;
  assistantScopesSeen.ids.add(id);
  // Trigger Svelte's reactivity by reassigning the wrapper. The Set
  // itself is mutated in place; the wrapper swap forces consumers
  // that read `assistantScopesSeen.ids` to re-evaluate.
  assistantScopesSeen.ids = new Set(assistantScopesSeen.ids);
  saveScopesSeen(assistantScopesSeen.ids);
  return false;
}

/// Fullscreen side-by-side diff view for a pending assistant edit.
/// Opened from the edit card's "Diff" button; reads the file's
/// current bytes from the open tab (if any) or from disk so the
/// left side shows what's on the user's machine right now and the
/// right side shows the assistant's proposal. The overlay carries
/// its own Apply / Save-as / Discard actions so the user can act
/// without bouncing back to the chat.
export const diffOverlay = $state<{
  open: boolean;
  /// The pending edit being diffed. Captured by reference so the
  /// overlay's Apply / Discard buttons mutate the same record the
  /// chat scrollback renders.
  edit: AssistantPendingEdit | null;
  /// File path, mirrored from edit.path so the overlay can render
  /// it independent of any later edit mutation.
  path: string;
  /// Original content for the left pane. Resolved at open-time:
  /// open-tab buffer wins, falls back to api.read. Empty string
  /// when the path doesn't exist yet (the proposal is creating a
  /// brand-new file).
  original: string;
  /// True while the overlay is fetching the original content; the
  /// view shows a small "loading…" indicator instead of empty L.
  loading: boolean;
  /// Error string from the api.read fallback; non-null only when
  /// both the open-tab and disk reads failed. Surfaced in the
  /// overlay header so the user knows the diff baseline is empty.
  error: string | null;
}>({
  open: false,
  edit: null,
  path: "",
  original: "",
  loading: false,
  error: null,
});

/// Open the diff overlay for `edit`, resolving the original
/// content from the open tab (if the path is loaded) or from
/// disk. Returns immediately; the overlay shows a brief
/// "loading…" placeholder while the read completes.
export function openDiffOverlay(edit: AssistantPendingEdit): void {
  diffOverlay.edit = edit;
  diffOverlay.path = edit.path;
  diffOverlay.original = "";
  diffOverlay.loading = true;
  diffOverlay.error = null;
  diffOverlay.open = true;
  // First: check open tabs for a buffer at this path. Wins over
  // disk because a user may have unsaved edits that the assistant's
  // proposal will replace; the diff should be against what they
  // see, not what's persisted.
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    for (const t of node.tabs) {
      if (t.kind === "file" && t.path === edit.path) {
        diffOverlay.original = t.content;
        diffOverlay.loading = false;
        return;
      }
    }
  }
  // Fall back to disk. Treat 404 as "new file" (left side empty);
  // any other error surfaces in the header.
  void api.read(edit.path).then(
    (resp) => {
      diffOverlay.original = resp.content ?? "";
      diffOverlay.loading = false;
    },
    (e) => {
      const msg = (e as Error).message ?? String(e);
      if (msg.includes("404") || msg.toLowerCase().includes("not found")) {
        diffOverlay.original = "";
        diffOverlay.loading = false;
      } else {
        diffOverlay.original = "";
        diffOverlay.error = msg;
        diffOverlay.loading = false;
      }
    },
  );
}

export function closeDiffOverlay(): void {
  diffOverlay.open = false;
  diffOverlay.edit = null;
  diffOverlay.path = "";
  diffOverlay.original = "";
  diffOverlay.loading = false;
  diffOverlay.error = null;
}

/// Streaming buffer for the assistant turn currently in flight.
/// Fed by `llm.delta` WS frames in `onWatchEvent`; consumed by
/// InlineAssist to render a live-updating assistant bubble in
/// place of the static "thinking…" placeholder.
///
/// Lifecycle: `beginAssistantStream` flips `sessionId` to the id
/// shipped on the /api/llm/complete request; deltas arriving with
/// a different id are dropped. `endAssistantStream` clears the
/// buffer once the synchronous JSON response has been folded into
/// the conversation log so a stale tail can't reappear on the
/// next submit.
export const assistantStream = $state<{
  sessionId: string | null;
  /// Context id the in-flight turn is bound to (file:<path> /
  /// group:<key> / drive). Captured at stream-begin so tool-turn
  /// attribution stays correct even when the user closes the
  /// overlay and switches to a different file mid-request via
  /// the "close, keep running" Esc choice.
  contextId: string | null;
  text: string;
  /// Tool results chan-llm emitted during this turn, keyed by tool
  /// call id. Populated by the `llm.tool_result` WS frame handler;
  /// consumed by InlineAssist's `handleResponse` to rebuild a
  /// well-formed message history (assistant turn -> tool messages)
  /// so the next round's request doesn't break Anthropic's strict
  /// tool_use/tool_result pairing.
  toolResults: Record<string, unknown>;
  /// Non-null when the backend emitted `llm.error` over the WS
  /// before (or instead of) the HTTP response landing. The HTTP
  /// path surfaces its own error via the catch block in
  /// InlineAssist.submit; this field is for the streaming side.
  error: string | null;
}>({
  sessionId: null,
  contextId: null,
  text: "",
  toolResults: {},
  error: null,
});

export function beginAssistantStream(
  sessionId: string,
  contextId: string,
): void {
  assistantStream.sessionId = sessionId;
  assistantStream.contextId = contextId;
  assistantStream.text = "";
  // Replace the map outright (not just clear keys) so any prior-
  // turn reference held by a `$derived` block stops tracking the
  // new buffer; Svelte 5 proxies the assignment into a fresh
  // reactive object.
  assistantStream.toolResults = {};
  assistantStream.error = null;
}

/// End the in-flight stream. The optional `sessionId` argument
/// scopes the clear to a specific request: a late-arriving
/// background request whose user has already started a new one
/// must NOT clobber the new stream's state. When the id doesn't
/// match the current stream, the call is a no-op.
export function endAssistantStream(sessionId?: string): void {
  if (sessionId && assistantStream.sessionId !== sessionId) return;
  assistantStream.sessionId = null;
  assistantStream.contextId = null;
  assistantStream.text = "";
  assistantStream.toolResults = {};
  assistantStream.error = null;
}

/// AbortController for the in-flight `/api/llm/complete` request.
/// Module-level (non-reactive) because AbortController carries
/// internal mutable state Svelte's proxy would mangle. InlineAssist
/// sets this at request-start and clears it in its finally block;
/// external callers (tab close, file delete, page unload) use the
/// helpers below to tear down a stream that targets a scope the
/// user has just walked away from.
let assistantInflightCtl: AbortController | null = null;

/// Called from InlineAssist.submit() to publish the AbortController
/// so it can be cancelled from outside the component.
export function setAssistantInflight(ctl: AbortController | null): void {
  assistantInflightCtl = ctl;
}

/// Cancel the in-flight assistant stream, whatever its scope. The
/// abort wakes InlineAssist.submit()'s catch arm; the existing
/// AbortError path emits "assistant stopped" and clears the stream.
export function cancelAssistantStream(): void {
  assistantInflightCtl?.abort();
}

/// Cancel the in-flight stream only when it targets the given
/// context id. Returns true when a cancellation actually fired so
/// callers can decide whether to surface a status message. Used by
/// the tab-close / file-delete paths: closing the tab the assistant
/// is currently working on means the user has walked away from
/// that scope and the request should not keep running.
export function cancelAssistantStreamForContext(contextId: string): boolean {
  if (!assistantInflightCtl) return false;
  if (assistantStream.contextId !== contextId) return false;
  assistantInflightCtl.abort();
  return true;
}

/// Cancel the in-flight stream when it targets the file scope OR a
/// group scope that includes this path. Group keys have the form
/// `group:<path1>|<path2>|...`; closing one member tab tears down
/// the whole group conversation (per the InlineAssist v3 contract),
/// so the matching in-flight request must die with it.
export function cancelAssistantStreamForPath(path: string): boolean {
  if (cancelAssistantStreamForContext(`file:${path}`)) return true;
  const ctx = assistantStream.contextId;
  if (!assistantInflightCtl || !ctx || !ctx.startsWith("group:")) return false;
  const members = ctx.slice("group:".length).split("|");
  if (!members.includes(path)) return false;
  assistantInflightCtl.abort();
  return true;
}

/// True when an assistant response landed on a conversation
/// scoped to `path` that the user hasn't viewed yet. Tab strips
/// swap the file icon for a bell when this is true. Covers both
/// single-file conversations and group conversations that include
/// `path` as a member.
export function assistantHasUnreadForPath(path: string): boolean {
  if (assistantConversations.byFile[path]?.hasUnread) return true;
  for (const [key, conv] of Object.entries(assistantConversations.byGroup)) {
    if (!conv.hasUnread) continue;
    if (key.split("|").includes(path)) return true;
  }
  return false;
}

/// True when `path` falls inside the assistant's currently-active
/// scope. Tab strips use this together with
/// `assistantStream.sessionId !== null` to render a flashing
/// "assistant working" dot on file tabs whose content is feeding
/// the in-flight turn.
///
/// Scope semantics:
///   - file:<path>   → single path match
///   - group:<key>   → path is one of the sorted paths joined by `|`
///   - drive         → no specific file scope; returns false (the
///                     drive turn doesn't single out any tab)
export function pathInAssistantScope(path: string): boolean {
  const id = assistantOverlay.contextId;
  if (!id) return false;
  if (id.startsWith("file:")) return id.slice("file:".length) === path;
  if (id.startsWith("group:")) {
    const key = id.slice("group:".length);
    return key.split("|").includes(path);
  }
  return false;
}

/// Resolve the currently-active assistant conversation from
/// `assistantOverlay.contextId`. Used by the `llm.tool_call` /
/// `llm.tool_result` WS handlers to find the conversation they
/// should narrate into. Returns null when the conversation hasn't
/// been seeded yet (lazy seed happens in InlineAssist.submit
/// before the request fires, so this only nulls for in-flight
/// turns whose conv vanished — e.g. user clicked `/clear` mid-
/// request, very rare).
function currentAssistantConversation(): AssistantConversation | null {
  // Use the contextId captured at stream-begin so a user who
  // closed the overlay and switched files mid-request still gets
  // their tool turns routed to the right conversation. Falls back
  // to `assistantOverlay.contextId` for compatibility if a code
  // path ever forgot to seed contextId on the stream (defensive).
  const id = assistantStream.contextId ?? assistantOverlay.contextId;
  if (!id) return null;
  if (id === "drive") {
    return assistantConversations.drive;
  }
  if (id.startsWith("file:")) {
    return assistantConversations.byFile[id.slice("file:".length)] ?? null;
  }
  if (id.startsWith("group:")) {
    return assistantConversations.byGroup[id.slice("group:".length)] ?? null;
  }
  return null;
}

/// Truthy when a tool result JSON shape signals failure. chan-llm
/// emits errors as `{"error": "..."}`, while success shapes carry
/// tool-specific keys (content / hits / entries / bytes / etc.).
function isErrorOutput(output: unknown): boolean {
  if (!output || typeof output !== "object") return false;
  const o = output as Record<string, unknown>;
  return typeof o.error === "string";
}

/// One-line summary of a tool result for the chip's tail (e.g.
/// `12 hits`, `1.2 KB`, `67 entries`). Returns null when the
/// result shape doesn't yield an obvious metric; the chip still
/// renders with status alone.
function summarizeToolResult(output: unknown): string | null {
  if (!output || typeof output !== "object") return null;
  const o = output as Record<string, unknown>;
  if (typeof o.error === "string") {
    return o.error.length <= 60 ? o.error : `${o.error.slice(0, 59)}…`;
  }
  if (Array.isArray(o.hits)) return `${o.hits.length} hits`;
  if (Array.isArray(o.entries)) return `${o.entries.length} entries`;
  if (Array.isArray(o.files)) return `${o.files.length} files`;
  // graph_tags returns { tags: [...] }
  if (Array.isArray(o.tags)) return `${o.tags.length} tags`;
  // graph_neighbors returns { out: [...], in: [...] } — collapse to
  // one count so the chip stays readable.
  if (Array.isArray(o.out) || Array.isArray(o.in)) {
    const out = Array.isArray(o.out) ? o.out.length : 0;
    const inLen = Array.isArray(o.in) ? o.in.length : 0;
    return `${out} out · ${inLen} in`;
  }
  // repo_report returns { totals: {files, code, ...}, ... }
  if (o.totals && typeof o.totals === "object") {
    const t = o.totals as Record<string, unknown>;
    const files = typeof t.files === "number" ? t.files : null;
    const code = typeof t.code === "number" ? t.code : null;
    if (files !== null && code !== null) return `${files} files · ${code} LOC`;
    if (files !== null) return `${files} files`;
  }
  if (typeof o.content === "string") return formatBytes(o.content.length);
  if (typeof o.bytes === "number" && Number.isFinite(o.bytes)) {
    return formatBytes(o.bytes);
  }
  return null;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

/// Normalize a tool call name. Anthropic API and Ollama emit the
/// bare tool name (e.g. `write_file`); the claude-cli and
/// gemini-cli backends emit MCP-namespaced names (`mcp__chan__write_file`)
/// because chan's tools are exposed to those agents via the chan-llm
/// MCP bridge. Strip the `mcp__<server>__` prefix so downstream
/// dispatch (status-bar label, edit-card filter, etc.) stays
/// backend-agnostic.
export function bareToolName(name: string): string {
  if (!name.startsWith("mcp__")) return name;
  const rest = name.slice("mcp__".length);
  const idx = rest.indexOf("__");
  if (idx < 0) return name;
  return rest.slice(idx + 2);
}

/// Short, human-readable label for a tool call. Truncates path /
/// query strings so a single line still fits in the status pill
/// even on a narrow window. Unknown tool names fall through to
/// their raw name so a future addition shows up legibly without
/// a frontend bump.
function labelForToolCall(name: string, input: unknown): string {
  const bare = bareToolName(name);
  const args = (input ?? {}) as Record<string, unknown>;
  const shortPath = (p: unknown): string => {
    if (typeof p !== "string" || p.length === 0) return "";
    return p.length <= 40 ? p : `…${p.slice(p.length - 39)}`;
  };
  if (bare === "read_file") return `reading ${shortPath(args.path) || "file"}`;
  if (bare === "write_file") {
    return `proposing edit to ${shortPath(args.path) || "file"}`;
  }
  if (bare === "list_files") {
    const prefix = typeof args.prefix === "string" ? args.prefix : "";
    return prefix ? `listing ${shortPath(prefix)}` : "listing drive";
  }
  if (bare === "search_content") {
    const q = typeof args.query === "string" ? args.query : "";
    const trimmed = q.length <= 32 ? q : `${q.slice(0, 31)}…`;
    return q ? `searching "${trimmed}"` : "searching drive";
  }
  if (bare === "read_image") return `viewing ${shortPath(args.path) || "image"}`;
  if (bare === "repo_report") {
    const prefix = typeof args.prefix === "string" ? args.prefix : "";
    if (prefix) return `scanning ${shortPath(prefix)}`;
    const paths = Array.isArray(args.paths) ? args.paths.length : 0;
    if (paths > 0) return `scanning ${paths} files`;
    return "scanning drive";
  }
  if (bare === "graph_neighbors") {
    const dir = typeof args.direction === "string" ? args.direction : "both";
    const path = shortPath(args.path) || "file";
    const arrow = dir === "out" ? "→" : dir === "in" ? "←" : "↔";
    return `graph ${arrow} ${path}`;
  }
  if (bare === "graph_tags") return "listing tags";
  if (bare === "graph_files_with_tag") {
    const tag = typeof args.tag === "string" ? args.tag : "";
    return tag ? `files tagged ${tag}` : "files for tag";
  }
  // Native claude-cli / gemini-cli tools the agent reaches for when
  // our chan MCP catalog doesn't cover a need. Argument field names
  // come from claude-code's published tool schemas (Read uses
  // file_path; Grep / Glob use pattern + optional path; Bash uses
  // command; WebFetch uses url; WebSearch uses query; Task uses
  // description). We mirror those keys so the chip surfaces the
  // user-visible piece of context (the path / query / pattern)
  // instead of the bare tool name.
  if (bare === "Read") return `reading ${shortPath(args.file_path) || "file"}`;
  if (bare === "Write") return `writing ${shortPath(args.file_path) || "file"}`;
  if (bare === "Edit" || bare === "MultiEdit") {
    return `editing ${shortPath(args.file_path) || "file"}`;
  }
  if (bare === "Glob") {
    const pat = typeof args.pattern === "string" ? args.pattern : "";
    return pat ? `glob ${shortPath(pat)}` : "glob";
  }
  if (bare === "Grep") {
    const pat = typeof args.pattern === "string" ? args.pattern : "";
    const trimmed = pat.length <= 32 ? pat : `${pat.slice(0, 31)}…`;
    return pat ? `grep "${trimmed}"` : "grep";
  }
  if (bare === "Bash") {
    const cmd = typeof args.command === "string" ? args.command : "";
    const trimmed = cmd.length <= 40 ? cmd : `${cmd.slice(0, 39)}…`;
    return cmd ? `$ ${trimmed}` : "shell";
  }
  if (bare === "WebFetch") {
    const url = typeof args.url === "string" ? args.url : "";
    return url ? `fetching ${shortPath(url)}` : "fetching url";
  }
  if (bare === "WebSearch" || bare === "ToolSearch") {
    const q = typeof args.query === "string" ? args.query : "";
    const trimmed = q.length <= 32 ? q : `${q.slice(0, 31)}…`;
    return q ? `searching "${trimmed}"` : "searching";
  }
  if (bare === "Task") {
    const desc = typeof args.description === "string" ? args.description : "";
    const trimmed = desc.length <= 36 ? desc : `${desc.slice(0, 35)}…`;
    return desc ? `subtask: ${trimmed}` : "subtask";
  }
  if (bare === "TodoWrite") return "updating todos";
  // Generic fallback: when the tool name isn't one we know, try
  // the common arg keys for a hint. Order from most-specific to
  // most-generic so a tool with both `path` and `query` shows the
  // path (which is usually the load-bearing context).
  for (const key of ["file_path", "path", "url", "query", "pattern", "command"]) {
    const v = args[key];
    if (typeof v === "string" && v.length > 0) {
      const short = v.length <= 36 ? v : `${v.slice(0, 35)}…`;
      return `${bare} ${short}`;
    }
  }
  return bare;
}

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
 *  the layout shifted).
 *
 *  Selection prefill: if the user had real text selected when
 *  they invoked the assistant (e.g. via Cmd+P from the editor),
 *  seed the prompt with that selection as a markdown blockquote
 *  so the model sees the explicit reference. Runs ONLY on this
 *  user-initiated open path so reload-driven `assistantOverlay.open`
 *  flips (URL hash restore) don't re-quote whatever the browser
 *  may have left selected, which would otherwise grow / clobber
 *  the round-tripped prompt on every refresh. */
export function openAssistant(): void {
  // Only seed the prompt with the current text selection when this
  // is a fresh open. Re-invoking on an already-open overlay (the
  // user clicked the pill while it was up) leaves whatever they
  // had typed in place so an accidental click doesn't clobber an
  // in-progress prompt with a quoted selection.
  const wasOpen = assistantOverlay.open;
  assistantOverlay.contextId = defaultScopeId();
  if (!wasOpen) {
    const sel = captureWindowSelection();
    if (sel) assistantOverlay.prompt = formatQuotePrefill(sel);
  }
  assistantOverlay.open = true;
  scheduleSessionSave();
}

/// Snapshot any non-empty plain-text selection in the document.
/// Used only by openAssistant; the assistant's open-effect no
/// longer reads window.getSelection on reload to avoid the
/// browser-preserved-selection re-quote loop.
function captureWindowSelection(): string | null {
  if (typeof window === "undefined") return null;
  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0) return null;
  const text = sel.toString();
  return text.trim().length === 0 ? null : text;
}

/// Format a selection as a markdown blockquote prefix for the
/// assistant prompt: each line gets `> `, blank inner lines
/// become bare `>`, then we terminate with two blank lines so
/// the caret lands one empty line below the quote (the +1 line
/// gap the user expects between the reference and where they
/// start typing).
export function formatQuotePrefill(text: string): string {
  const normalised = text.replace(/\r\n?/g, "\n").replace(/\n$/, "");
  const quoted = normalised
    .split("\n")
    .map((l) => (l.length === 0 ? ">" : `> ${l}`))
    .join("\n");
  return `${quoted}\n\n\n`;
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
  // Same trick for file scopes entered via openGraphForFile or the
  // inspector's "Graph this" button on a file/image/contact node.
  // The file may not be open in any pane (so availableScopeOptions
  // didn't include it), but the user just asked for it to be the
  // scope — surfacing it in the dropdown lets them switch away and
  // back without losing the selection, and prevents the snap-back
  // effect in GraphPanel from clobbering scopeId on the next tick.
  if (graphOverlay.scopeId.startsWith("file:")) {
    const path = graphOverlay.scopeId.slice("file:".length);
    if (path && !out.some((o) => o.id === graphOverlay.scopeId)) {
      out.unshift({
        id: graphOverlay.scopeId,
        kind: "file",
        label: path,
        path,
        readOnly: false,
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
  graphOverlay.depth = 1;
  graphOverlay.pendingSelectId = nodeId;
  graphOverlay.open = true;
  // Stack on top of whatever overlay invoked us (typically the
  // file browser via a tag chip). OverlayShell's z-index follows
  // `overlayStack.ids`, so the graph paints above and Escape
  // pops just the graph — returning to the browser instead of
  // dismissing both at once.
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
  graphOverlay.depth = 1;
  graphOverlay.pendingSelectId = path;
  graphOverlay.open = true;
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
  graphOverlay.depth = 1;
  graphOverlay.pendingSelectId = nodeId;
  graphOverlay.open = true;
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
  | "settings"
  | "scope-history";

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
    case "scope-history":
      scopeHistoryOverlay.open = false;
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
  if (scopeHistoryOverlay.open) open.add("scope-history");
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

// ---- scope history overlay ----------------------------------------------
//
// Window-level overlay that lists every persisted assistant
// conversation (per-file, per-group, drive) in one place. The
// aggregator below pulls from the three on-disk stores:
//
//   - file blobs:  per-path JSON under .chan/assistant/<sha>.json,
//                  identified by listing the assistant-blob keys and
//                  ignoring the manifest / group / drive blobs.
//   - group LRU:   `g_index.json` manifest carries `{key, paths,
//                  last_touched}` for the last GROUP_LRU_MAX threads
//                  without us reading every per-group blob first.
//   - drive:       single `drive.json` blob.
//
// Filter chips mirror the graph overlay shape (on/off per kind).

export type ScopeHistoryKind = "file" | "group" | "drive";

export type ScopeHistoryEntry = {
  /// Stable id for keying. `file:<path>`, `group:<key>`, `drive`.
  id: string;
  kind: ScopeHistoryKind;
  /// Human title: file path / "N files" / "Drive".
  title: string;
  /// File paths included in this scope. Empty for drive.
  paths: string[];
  /// Saved URL for "Open in new window". May be undefined for
  /// blobs written before the field existed.
  url?: string;
  created_at?: number;
  last_touched?: number;
  /// `turns.length` — the chat scrollback size; cheap proxy for
  /// thread depth in the bubble preview.
  turn_count: number;
  /// `key` is the group's raw `sortedKey` — needed to look up the
  /// in-memory conversation and to drive resume / clear. Undefined
  /// for file (use `paths[0]`) and drive (no key).
  group_key?: string;
};

/// Sort key for the scope-history list. Each option has a fixed
/// natural direction so the user can't pick "title descending":
///   - recent: `last_touched` desc (fallback `created_at`)
///   - created: `created_at` desc (fallback `last_touched`)
///   - title:   alphabetic asc (case-insensitive)
///   - turns:   `turn_count` desc
export type ScopeHistorySort = "recent" | "created" | "title" | "turns";

export const scopeHistoryOverlay = $state<{
  open: boolean;
  /// Filter chips. Independent on/off toggles per scope kind;
  /// initially all on so the list is complete on first open.
  filters: { file: boolean; group: boolean; drive: boolean };
  /// Active sort key. See `ScopeHistorySort` for the per-option
  /// natural direction.
  sortBy: ScopeHistorySort;
  /// In-memory cache of the aggregated list; refreshed on open
  /// and after deletions so the overlay doesn't refetch on every
  /// scroll.
  entries: ScopeHistoryEntry[];
  /// True while `refreshScopeHistory` is in flight. Drives a small
  /// loading indicator in the overlay header.
  loading: boolean;
  /// Last error from the listing fetch, surfaced inline.
  error: string | null;
  /// Id of the entry whose inline read-only peek is currently
  /// expanded, or null when no bubble is expanded. Group scopes
  /// drive this through "preview"; file / drive scopes never
  /// expand in place (they resume directly). Persisted on the
  /// overlay state so closing + reopening the overlay restores
  /// the previous expansion without a fresh fetch.
  expandedId: string | null;
  /// Cached turns for `expandedId`. Lives alongside the id so a
  /// reopen renders immediately; togglePeek refreshes it whenever
  /// the id changes.
  expandedTurns: AssistantTurn[];
  /// True while `togglePeek` is awaiting fresh turns from disk.
  /// The bubble shows a "loading…" placeholder during the gap.
  expandedLoading: boolean;
}>({
  open: false,
  filters: { file: true, group: true, drive: true },
  sortBy: "recent",
  entries: [],
  loading: false,
  error: null,
  expandedId: null,
  expandedTurns: [],
  expandedLoading: false,
});

/// Open the assistant overlay on the history tab and (re)load
/// the entries. Scope history lives as a tab inside the assistant
/// (not its own floating panel) so the user navigates one surface;
/// `scopeHistoryOverlay.open` is the source of truth for "the
/// history tab is active". The reload is fire-and-forget; the
/// list renders with the previous cache until the fresh listing
/// lands so the open feels instant. Idempotent.
export function openScopeHistory(): void {
  scopeHistoryOverlay.open = true;
  assistantOverlay.open = true;
  void refreshScopeHistory();
  scheduleSessionSave();
}

/// Switch the assistant back to the chat tab. Leaves the assistant
/// overlay open: the user explicitly asked for the history view to
/// hide, not for the whole panel to close.
export function closeScopeHistory(): void {
  scopeHistoryOverlay.open = false;
}

/// Aggregate the three on-disk stores into a single list and
/// publish it to `scopeHistoryOverlay.entries`. Idempotent and
/// safe to call repeatedly; the loading flag debounces the UI.
export async function refreshScopeHistory(): Promise<void> {
  scopeHistoryOverlay.loading = true;
  scopeHistoryOverlay.error = null;
  try {
    const out: ScopeHistoryEntry[] = [];

    // ---- group entries (cheap: manifest carries paths) ------------
    try {
      const idx = await loadGroupIndex();
      for (const e of idx.entries) {
        // Best-effort enrich with full blob data so we get
        // `created_at` / `url` / `turn_count`. If it fails, fall
        // back to the manifest fields.
        let created_at: number | undefined;
        let url: string | undefined;
        let turn_count = 0;
        try {
          const raw = await api.getAssistantBlob(blobKeyForGroupHash(e.hash));
          if (raw) {
            const parsed = raw as {
              turns?: AssistantTurn[];
              created_at?: number;
              url?: string;
            };
            turn_count = parsed.turns?.length ?? 0;
            created_at = parsed.created_at ?? earliestTurnCreatedAt(parsed.turns ?? []);
            url = parsed.url;
          }
        } catch {
          // ignore, fall through to manifest-only fields.
        }
        out.push({
          id: `group:${e.key}`,
          kind: "group",
          title: e.paths.length === 1 ? e.paths[0] : `${e.paths.length} files`,
          paths: e.paths.slice(),
          url,
          created_at,
          last_touched: e.last_touched,
          turn_count,
          group_key: e.key,
        });
      }
    } catch {
      // Group manifest is best-effort; an empty drive still produces
      // a usable list from the file + drive sources below.
    }

    // ---- file entries (need to fetch each blob for path + meta) --
    // The per-file blob key is sha256(path)[..16].json and the path
    // is stored inside the JSON; we can't recover it from the key
    // alone. New blobs (slice 2) include `path` + metadata; older
    // ones may not, in which case we skip the entry rather than
    // surface a hash-keyed mystery row.
    try {
      const keys = await api.listAssistantBlobs();
      for (const key of keys) {
        if (
          key === GROUP_INDEX_KEY ||
          key === DRIVE_BLOB_KEY ||
          key.startsWith("g_")
        ) {
          continue;
        }
        try {
          const raw = await api.getAssistantBlob(key);
          if (!raw) continue;
          const parsed = raw as {
            path?: string;
            turns?: AssistantTurn[];
            created_at?: number;
            last_touched?: number;
            url?: string;
          };
          if (!parsed.path) continue;
          const turns = parsed.turns ?? [];
          out.push({
            id: `file:${parsed.path}`,
            kind: "file",
            title: parsed.path,
            paths: [parsed.path],
            url: parsed.url,
            created_at: parsed.created_at ?? earliestTurnCreatedAt(turns),
            last_touched: parsed.last_touched,
            turn_count: turns.length,
          });
        } catch {
          // Skip unreadable entries; rest of the list still loads.
        }
      }
    } catch {
      // Listing failure is non-fatal; group + drive entries above /
      // below still render.
    }

    // ---- drive entry ----------------------------------------------
    try {
      const raw = await api.getAssistantBlob(DRIVE_BLOB_KEY);
      if (raw) {
        const parsed = raw as {
          turns?: AssistantTurn[];
          created_at?: number;
          last_touched?: number;
          url?: string;
        };
        const turns = parsed.turns ?? [];
        out.push({
          id: "drive",
          kind: "drive",
          title: "Drive",
          paths: [],
          url: parsed.url,
          created_at: parsed.created_at ?? earliestTurnCreatedAt(turns),
          last_touched: parsed.last_touched,
          turn_count: turns.length,
        });
      }
    } catch {
      // Drive blob missing is fine; the list just omits it.
    }

    scopeHistoryOverlay.entries = out;
  } catch (e) {
    scopeHistoryOverlay.error = (e as Error).message ?? String(e);
  } finally {
    scopeHistoryOverlay.loading = false;
  }
}

/// Resume a saved scope in the current window. File / drive flip
/// the assistant overlay to that scope (file scopes also pop the
/// file open so the visible-files derivation picks it up); group
/// scopes can't bind in-place because the group key is derived
/// from currently visible files, so we route the caller to the
/// peek expansion instead. Returns whether the resume landed; the
/// overlay uses the false return as a hint to expand the bubble
/// inline.
export async function resumeScopeHistoryEntry(
  entry: ScopeHistoryEntry,
): Promise<boolean> {
  if (entry.kind === "file") {
    const path = entry.paths[0];
    if (!path) return false;
    // File needs to be visible for the scope option to surface;
    // openInActivePane is the smallest layout change that gets us
    // there. The caller has already gated on path existence.
    await openInActivePane(path);
    closeScopeHistory();
    openAssistant();
    assistantOverlay.contextId = `file:${path}`;
    return true;
  }
  if (entry.kind === "drive") {
    closeScopeHistory();
    openAssistant();
    assistantOverlay.contextId = "drive";
    return true;
  }
  // group: caller should expand the bubble inline; nothing to do
  // here. Returning false signals the overlay to render the peek.
  return false;
}

/// Open a saved scope in a new window via the snapshotted URL.
/// The URL is origin-relative (no auth token), so the new window
/// inherits whatever token the launcher injects. Returns false
/// when no URL is captured (old blobs predate slice 2); the
/// overlay hides the button in that case.
export function openScopeHistoryInNewWindow(entry: ScopeHistoryEntry): boolean {
  if (!entry.url) return false;
  if (typeof window === "undefined") return false;
  window.open(entry.url, "_blank", "noopener");
  return true;
}

/// Render the saved conversation as a markdown document and save
/// it under the drive's answers_dir. The frontmatter captures the
/// scope kind, paths, and timestamps so the exported file is
/// self-describing; the body is one heading per turn. Returns the
/// drive-relative path written, or throws on failure.
export async function exportScopeHistoryToDrive(
  entry: ScopeHistoryEntry,
): Promise<string> {
  const turns = await fetchScopeHistoryTurns(entry);
  const md = renderScopeHistoryMarkdown(entry, turns);
  const stem = scopeHistoryExportName(entry);
  const { path } = await api.saveAnswerMarkdown({ content: md, name: stem });
  return path;
}

/// Delete a saved scope from disk and drop the in-memory mirror.
/// Refreshes the overlay's entry list so the row disappears.
export async function deleteScopeHistoryEntry(
  entry: ScopeHistoryEntry,
): Promise<void> {
  if (entry.kind === "file") {
    const path = entry.paths[0];
    if (!path) return;
    await api.deleteConversation(path);
    delete assistantConversations.byFile[path];
  } else if (entry.kind === "group") {
    const key = entry.group_key;
    if (!key) return;
    // clearGroupConversation drops both the in-memory bucket AND
    // the on-disk blob via clearGroupConversationOnDisk; the
    // manifest entry is rewritten in the same pass.
    clearGroupConversation(key);
  } else {
    await deleteDriveConversation();
    assistantConversations.drive = null;
  }
  await refreshScopeHistory();
}

/// Wipe every scope history entry from disk + memory. Iterates
/// through the currently-cached entries (the same set the user
/// sees in the overlay) so the action is bounded by what the
/// listing surfaced; entries written by another window after the
/// refresh land are left alone until the next refresh, which
/// will pick them up. The final `refreshScopeHistory` repopulates
/// the entry list (now empty) and resets the loading flag.
export async function clearAllScopeHistory(): Promise<void> {
  // Snapshot the entry list so we don't mutate the underlying
  // reactive array as we iterate.
  const snapshot = scopeHistoryOverlay.entries.slice();
  for (const e of snapshot) {
    try {
      if (e.kind === "file") {
        const path = e.paths[0];
        if (path) {
          await api.deleteConversation(path);
          delete assistantConversations.byFile[path];
        }
      } else if (e.kind === "group") {
        if (e.group_key) clearGroupConversation(e.group_key);
      } else {
        await deleteDriveConversation();
        assistantConversations.drive = null;
      }
    } catch {
      // Best-effort: a single failed delete shouldn't abort the
      // whole sweep. The refresh below will reflect whatever did
      // land.
    }
  }
  // Reset peek state so a now-deleted bubble doesn't stay
  // notionally expanded in the overlay's state.
  scopeHistoryOverlay.expandedId = null;
  scopeHistoryOverlay.expandedTurns = [];
  await refreshScopeHistory();
}

/// Fetch the persisted turns for a scope entry. Used by the
/// inline peek (group scopes) and by the markdown export.
export async function fetchScopeHistoryTurns(
  entry: ScopeHistoryEntry,
): Promise<AssistantTurn[]> {
  if (entry.kind === "file") {
    const path = entry.paths[0];
    if (!path) return [];
    const raw = await api.getConversation(path);
    if (!raw) return [];
    return (raw as { turns?: AssistantTurn[] }).turns ?? [];
  }
  if (entry.kind === "group") {
    const key = entry.group_key;
    if (!key) return [];
    const hash = await assistantHash16(key);
    const raw = await api.getAssistantBlob(blobKeyForGroupHash(hash));
    if (!raw) return [];
    return (raw as { turns?: AssistantTurn[] }).turns ?? [];
  }
  const raw = await api.getAssistantBlob(DRIVE_BLOB_KEY);
  if (!raw) return [];
  return (raw as { turns?: AssistantTurn[] }).turns ?? [];
}

export function scopeHistoryExportName(entry: ScopeHistoryEntry): string {
  if (entry.kind === "file") {
    const path = entry.paths[0] ?? "scope";
    const slash = path.lastIndexOf("/");
    const stem = slash >= 0 ? path.slice(slash + 1) : path;
    const dot = stem.lastIndexOf(".");
    const bare = dot > 0 ? stem.slice(0, dot) : stem;
    return `assistant-${bare}`;
  }
  if (entry.kind === "group") {
    return `assistant-group-${entry.paths.length}-files`;
  }
  return "assistant-drive";
}

export function renderScopeHistoryMarkdown(
  entry: ScopeHistoryEntry,
  turns: AssistantTurn[],
): string {
  const lines: string[] = [];
  const title =
    entry.kind === "file"
      ? entry.paths[0]
      : entry.kind === "group"
        ? `Group (${entry.paths.length} files)`
        : "Drive";
  lines.push(`# Assistant conversation — ${title}`);
  lines.push("");
  lines.push(`- kind: ${entry.kind}`);
  if (entry.paths.length > 0) {
    lines.push(`- files: ${entry.paths.length}`);
    for (const p of entry.paths) lines.push(`  - ${p}`);
  }
  if (entry.created_at) {
    lines.push(`- started: ${new Date(entry.created_at).toISOString()}`);
  }
  if (entry.last_touched) {
    lines.push(`- last activity: ${new Date(entry.last_touched).toISOString()}`);
  }
  lines.push(`- turns: ${turns.length}`);
  lines.push("");
  for (const t of turns) {
    if (t.kind === "user") {
      lines.push("## You");
      lines.push("");
      lines.push(t.content);
      lines.push("");
    } else if (t.kind === "assistant") {
      lines.push("## Assistant");
      lines.push("");
      lines.push(t.content);
      lines.push("");
    } else if (t.kind === "edit") {
      lines.push(`## Edit proposal — ${t.edit.path}`);
      lines.push("");
      if (t.edit.summary) {
        lines.push(`> ${t.edit.summary}`);
        lines.push("");
      }
      lines.push("```");
      lines.push(t.edit.content);
      lines.push("```");
      lines.push("");
    } else {
      // tool turn: short summary line so the transcript stays
      // readable without dragging in protocol-only payloads.
      lines.push(
        `_${t.event.label} (${t.event.status}${t.event.result_summary ? `: ${t.event.result_summary}` : ""})_`,
      );
      lines.push("");
    }
  }
  return lines.join("\n");
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
  outline: 220,
  assistant: 280,
};

export const paneWidths = $state<{
  inspector: number;
  graph: number;
  browser: number;
  search: number;
  outline: number;
  assistant: number;
}>({ ...DEFAULT_PANE_WIDTHS });

/// Currently inspected entry in the File Browser tab. Module-level
/// (shared across browser tabs); selection is ephemeral so the
/// minor cross-tab leakage is acceptable and avoids per-tab plumbing.
export const browserSelection = $state<{
  path: string | null;
  showDrive: boolean;
}>({
  path: null,
  showDrive: false,
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
      outline: clamp(paneWidths.outline),
      assistant: clamp(paneWidths.assistant),
    };
    widthsPersistInflight = widthsPersistInflight.catch(() => {}).then(async () => {
      const cfg = await api.config();
      const cur = cfg.preferences.pane_widths;
      if (
        cur &&
        cur.inspector === snapshot.inspector &&
        cur.graph === snapshot.graph &&
        cur.browser === snapshot.browser &&
        cur.search === snapshot.search &&
        cur.outline === snapshot.outline &&
        cur.assistant === snapshot.assistant
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

/// Persist the chosen date format so the next `@today` / `@date`
/// expansion uses it as the default. Called by the date popover's
/// format-change callback (commit path). Idempotent — skips the PATCH
/// when the server already has the same value.
let dateFormatPersistInflight: Promise<unknown> = Promise.resolve();
export function persistDateFormat(formatId: string): void {
  dateFormatPersistInflight = dateFormatPersistInflight
    .catch(() => {})
    .then(async () => {
      const cfg = await api.config();
      if (cfg.preferences.date_format === formatId) return;
      const next = await api.updateConfig({
        ...cfg,
        preferences: { ...cfg.preferences, date_format: formatId },
      });
      // Mirror the response into drive.info so the next macro
      // expansion reads the new default without a fresh /api/drive
      // round-trip.
      if (drive.info) {
        drive.info = {
          ...drive.info,
          preferences: {
            ...drive.info.preferences,
            date_format: next.preferences.date_format,
          },
        };
      }
    });
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
  browserSelection.showDrive = false;
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
        // Cancel any in-flight assistant request bound to this file's
        // scope BEFORE we drop the conversation. The user just told
        // us this file is going away; finishing the turn would either
        // (a) save a result against a now-orphaned conversation or
        // (b) try to `write_file` the deleted path. closeTab above
        // already cancels for the standard close-tab path; this is
        // the belt-and-braces case for files closed elsewhere or for
        // group conversations whose context id keys on the membership
        // set rather than `file:<path>`.
        cancelAssistantStreamForPath(p);
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
