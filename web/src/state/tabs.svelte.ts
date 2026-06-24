// Tab + pane state.
//
// v1 layout: a binary split tree of panes. Each pane holds an ordered list
// of tabs and an active tab id. Splits can be horizontal or vertical and
// nested arbitrarily, but the UI exposes a small set of operations:
//   - openInActivePane(path)
//   - splitRight() / splitDown()
//   - moveTabTo(otherPaneId)
//   - closeTab()
//
// Drag-rearrange of tabs is deferred; for v1 the menu offers explicit
// actions instead.

import { api, sessionWindowId } from "../api/client";
import { ApiError } from "../api/errors";
import type { DraftPromoteResponse, TerminalRosterEntry } from "../api/types";
import type { FindRange } from "../editor/find";
import { stripTrailingWhitespaceText } from "../editor/tools";
import { uiConfirm } from "./confirm.svelte";
import { editorToolsPrefs } from "./editorTools.svelte";
import { classifyPath, isCsv, isEditableText, isJson } from "./fileTypes";
import type { FileKind } from "./kinds";
import {
  createTerminalKeyboardProtocolState,
  resetTerminalKeyboardProtocolState,
  restoreKeyboardProtocolState,
  serializeKeyboardProtocolState,
  type SerializedKeyboardProtocolState,
  type TerminalKeyboardProtocolState,
} from "../terminal/keymap";
import { notify } from "./notify.svelte";
import { isRichPromptVisible, showRichPromptForTab } from "./richPrompt.svelte";
import {
  defaultTeamConfig,
  type TeamDialogConfig,
  type TeamDialogRequest,
} from "./teamDialog.svelte";
// `isDraftPath` comes from the side-effect-free `workspace.svelte`
// leaf module (NOT store.svelte), so importing it here doesn't trigger
// store's eager draft-promotion-sink registration. See the cycle note
// below.
import { isDraftPath } from "./workspace.svelte";
// `uiPathPrompt` lives in store.svelte, which has a TOP-LEVEL side
// effect (`registerDraftPromotionSink(...)`) that calls back into THIS
// module. A static `import { uiPathPrompt } from "./store.svelte"`
// here would force store's module body to run during tabs' own
// module-eval (whenever a file imports tabs first), and store's eager
// sink registration would touch `draftPromotionSinks` before it is
// initialised -> a fatal init-order crash in every tabs-first test.
// So we import it LAZILY inside `saveDraftTabToWorkspace` (a dynamic
// import resolved at user-action time, long after both modules have
// finished initialising). This is the correct way to consume a
// cyclic dependency whose other side has an eager side effect.

let nextId = 1;
function id(prefix: string): string {
  return `${prefix}-${nextId++}`;
}

/// Render mode for a file tab.
///   - `wysiwyg`: markdown-class only. Live rendering of markdown
///     via the custom CodeMirror Wysiwyg extension.
///   - `source`: raw source text in a CodeMirror editor. Available
///     on every tab as the lowest-common-denominator surface.
///   - `pretty`: collapsible-tree renderer. JSON only today.
///   - `table`: tabular renderer with click-to-edit cells. CSV /
///     TSV only today.
export type Mode = "wysiwyg" | "source" | "pretty" | "table";
export type EditorSelection = { from: number; to: number };
export type OpenFileOptions = {
  initialSelection?: EditorSelection;
};

/// Default mode for a freshly opened file. JSON tabs land in
/// "pretty"; CSV/TSV tabs land in "table"; markdown-class tabs
/// stay on "wysiwyg"; everything else (other text formats) opens
/// in source mode because that's the only mode they have.
function defaultModeForPath(path: string, fileKind: FileKind): Mode {
  if (isJson(path)) return "pretty";
  if (isCsv(path)) return "table";
  return fileKind === "text" ? "source" : "wysiwyg";
}

/// Whether `mode` is a valid pair for the given path + file kind.
/// Workspaces the session-restore guard: a stale URL hash that pairs an
/// incompatible (path, mode) falls back to the default for that path.
function isModeValidForPath(
  mode: Mode,
  path: string,
  fileKind: FileKind,
): boolean {
  if (mode === "pretty") return isJson(path);
  if (mode === "table") return isCsv(path);
  if (mode === "wysiwyg") return fileKind !== "text";
  // source is valid on every tab.
  return mode === "source";
}

function validateRestoredMode(
  persisted: Mode | undefined,
  path: string,
  fileKind: FileKind,
): Mode {
  if (persisted && isModeValidForPath(persisted, path, fileKind)) {
    return persisted;
  }
  return defaultModeForPath(path, fileKind);
}

/// Per-tab find-on-page state. Lives only while the bar is open
/// (cleared on close); intentionally not serialized through
/// SerTab so a session restore doesn't re-open the bar with a
/// stale query.
export type FindState = {
  open: boolean;
  query: string;
  caseSensitive: boolean;
  matches: FindRange[];
  /// -1 when there are no matches; otherwise an index into
  /// `matches`. The active match gets the .find-match--current
  /// decoration; prev/next rotate this index modulo `matches.length`.
  currentIndex: number;
  /// True iff `matches.length` hit MAX_FIND_MATCHES on the last
  /// scan. The counter reads "10000+" instead of "N of M" when
  /// this is set so users know they're seeing a truncated count.
  truncated: boolean;
  /// Bumped every time app.find.open targets this tab. Lets an
  /// already-mounted FindBar re-focus its input instead of treating
  /// the command as a no-op.
  focusNonce: number;
};

export function makeFindState(): FindState {
  return {
    open: false,
    query: "",
    caseSensitive: false,
    matches: [],
    currentIndex: -1,
    truncated: false,
    focusNonce: 0,
  };
}

/// File-content tab: holds the editable buffer for any text-class
/// file (markdown documents, contact notes, and arbitrary source /
/// config text like .py, .json, .yaml).
export type FileTab = {
  kind: "file";
  /// File-kind discriminator inside the tab. Mirrors the wire kind
  /// in `TreeEntry.kind` and the unified taxonomy in `./kinds.ts`:
  ///   - `document`: markdown-class, wysiwyg + source available.
  ///   - `contact`: same as document with contact frontmatter; the
  ///     tab UX is identical, but downstream surfaces (inspector,
  ///     graph) treat it as a contact.
  ///   - `text`: any other editable text file. Source-mode only;
  ///     the wysiwyg toggle is hidden in the tab menu.
  /// Initialized from `classifyPath` on open (path-based; we don't
  /// have the wire kind in tabs.svelte without a circular import
  /// on store). Contact files therefore start out tagged as
  /// `document` and stay that way for the tab's lifetime; the
  /// inspector and tree continue to read the live wire kind so the
  /// "contact" identity surfaces everywhere it matters.
  fileKind: FileKind;
  id: string;
  path: string;
  /// In-memory buffer; flushed on save.
  content: string;
  /// Last persisted content (for dirty detection).
  saved: string;
  /// Mtime returned by the last successful read or save. The
  /// nanosecond form is the CAS token on subsequent saves so an
  /// external edit between reads is detected as a 409 conflict
  /// rather than silently overwriting the disk-side change.
  /// Null when the file didn't exist yet (saved-from-empty); the
  /// server treats Some(None) as "expecting a fresh file".
  savedMtime: number | null;
  savedMtimeNs?: string | null;
  mode: Mode;
  loading: boolean;
  loadProgress?: { loadedBytes: number; totalBytes: number | null };
  error: string | null;
  /// Structured recovery state for an open file whose backing path
  /// disappeared. Kept separate from `error` so the UI can offer
  /// Re-open / Find / Close instead of showing a raw OS error.
  fileMissing: FileMissingState | null;
  /// Whether the right-side inspector panel (file info: tags,
  /// backlinks, refs) is shown alongside the editor. Toggleable
  /// per tab; persisted in the URL hash.
  inspectorOpen: boolean;
  /// Whether the left-side outline pane is shown alongside the
  /// editor. Toggleable per tab; persisted in the URL hash.
  outlineOpen: boolean;
  /// Enclosing git repo, relative to the workspace root, for files that
  /// live inside one. Set on first load from FileResponse.repo_root;
  /// workspaces the per-file "git repo: <name>" scope option in the
  /// overlay picker. `null` for files outside any repo (or files
  /// whose repo coincides with the workspace itself).
  repoRoot: string | null;
  /// User-toggled "read mode" for this tab (the lamp in
  /// WikiStatusBar). Per-tab so multi-pane layouts can mix
  /// read/write without panes fighting over a global flag.
  /// Ephemeral; not serialized into the URL hash or session.json.
  readMode: boolean;
  /// Filesystem-level writability, sourced from the file response's
  /// `writable` field on each read. `false` forces the tab into
  /// read-only mode regardless of `readMode` and disables the lamp
  /// toggle so the user can't try to write a file the OS won't
  /// accept. The watcher refreshes this when permissions change.
  fsWritable: boolean;
  /// An external (non-self) write to this file's path landed on disk
  /// while the tab is open. The editor shows a dismissable "changed on
  /// disk" banner and does NOT auto-reload, so the caret never jumps to
  /// 1:1 mid-edit. Set by `flagExternalChange` from the watcher; cleared
  /// on reload (loadTabContent) or dismiss.
  /// Ephemeral - not serialized into the URL hash / session.json.
  externalChange?: boolean;
  /// Whether the floating style toolbar (top-left of the editor
  /// canvas) is mounted for this tab. The user's explicit show /
  /// hide preference from the tab menu (a layer above the hover-
  /// to-expand behavior). Defaults false so new tabs open with a
  /// clean canvas; users opt the toolbar in from the tab menu.
  /// Per-tab so a "reading" tab can keep the chrome hidden while an
  /// adjacent editing tab shows it.
  styleToolbarOpen: boolean;
  /// Per-tab find-on-page state. Undefined until the first
  /// app.find.open command lands on the tab so tabs that never
  /// use Find stay free of the extra object. Persists across the
  /// Wysiwyg <-> Source mode toggle (same backing text); cleared
  /// on tab close along with the tab itself.
  find?: FindState;
  /// User-toggled syntax highlighting in source mode. Default true.
  /// Only meaningful when the tab is in source mode (markdown's
  /// wysiwyg surface does its own syntax painting). For text-kind
  /// tabs whose extension has no registered CodeMirror language
  /// pack the toggle is still surfaced but is effectively a no-op.
  /// Per-tab so a "reading" tab can read code with plain text and
  /// an adjacent "editing" tab can keep syntax on.
  syntaxHighlight: boolean;
  /// Visualize trailing spaces and tabs in the mounted editor. Per-tab
  /// because it is an inspection aid, not a document property.
  highlightTrailingWhitespace: boolean;
  /// Whether the user last asked this tab to collapse fenced code
  /// blocks. The mounted editor performs the actual folds.
  codeBlocksCollapsed: boolean;
  /// Last known caret position (doc offsets), persisted across the
  /// Wysiwyg <-> Source mode toggle and across page reloads via
  /// the URL-hash session. The active editor pushes updates here
  /// on every selection change; the editor that mounts next reads
  /// it once on first content apply to restore the caret.
  caret?: { from: number; to: number };
  /// Per-tab inspector and outline widths so two file tabs side by
  /// side carry independent inspector/outline sizes. Fall back to
  /// `paneWidths.inspector` / `paneWidths.outline` when unset.
  inspectorWidth?: number;
  outlineWidth?: number;
};

export type FileMissingState = {
  path: string;
  fragment: string | null;
  /// Best guess at where the file moved to, populated by the
  /// missing-file suggest-reopen lookup. Set when a basename
  /// search returns a unique match at a different path; null
  /// when the lookup ran and found 0 or 2+ candidates (ambiguous
  /// - let the user use Find).
  suggestedPath?: string | null;
};

export type TerminalTab = {
  kind: "terminal";
  id: string;
  title: string;
  createdAt: number;
  broadcastEnabled: boolean;
  broadcastTargetIds: string[];
  terminalEnvTabName?: string;
  terminalEnvNamePromptDismissed?: boolean;
  terminalSessionId?: string;
  controlledTerminal?: boolean;
  lastAgentEchoSeq?: number;
  terminalActivity?: boolean;
  /// Refines terminalActivity. True while output is actively ARRIVING at
  /// this unfocused terminal (the unseen-output dot PULSES); flips false
  /// once output stops but is still unseen (the dot goes SOLID). Cleared
  /// with terminalActivity when the user sees it.
  terminalActivityPulsing?: boolean;
  /// MESSAGE depth of this session's server-side write queue (Rich Prompt
  /// messages + `cs terminal write` teammate pokes share one FIFO; a gemini
  /// text+chord pair counts once). Drives the tab-strip badge and the idle
  /// prompt label. 0 is stored as undefined (truthiness renders like
  /// terminalActivity); never persisted — every (re)attach re-syncs it from
  /// the WS `session` frame.
  queueDepth?: number;
  /// The ONE in-flight Rich Prompt message (submit is a no-op while set).
  /// Lives on the TAB, not the bubble component, so hiding/showing the
  /// bubble mid-pending keeps the state machine running. phase: "sent"
  /// (frame out, no ack yet) -> "queued" (server ack; depth = the ack's
  /// 1-based position) -> "delivered" (last write hit the PTY) | "rejected"
  /// (queue full) | "failed" (WS close / ack timeout / session end). Cancel/
  /// recall adds two terminal phases the bubble consumes: "recalled" (the
  /// `prompt-cancelled` ack removed a still-queued message — unlock + keep the
  /// draft text to edit + resubmit) | "drained" (the cancel raced a drain; the
  /// message already hit the PTY — surface it, don't silently re-edit). The
  /// bubble's $effect consumes terminal phases and clears this field.
  pendingPrompt?: {
    id: string;
    phase: "sent" | "queued" | "delivered" | "rejected" | "failed" | "recalled" | "drained";
    depth?: number;
  };
  cwd?: string;
  seedInput?: string;
  /// Transient: this freshly-spawned terminal still needs its per-tenant
  /// `Terminal-N` default name resolved from the server. The spawn helpers
  /// set it with a local placeholder; `TerminalTab.connect()` resolves the
  /// real name BEFORE opening the WS (so the session spawns with its final
  /// name and the cross-window roster / `cs term list` show it, not the
  /// placeholder), then clears the flag. Never persisted: a restored tab
  /// keeps its saved title and skips the fetch.
  pendingGlobalName?: boolean;
  /// Rich Prompt per-terminal draft path (`<draftsDir>/<name>/draft.md`) backing
  /// the bubble: the draft.md IS the prompt text and the folder holds pasted
  /// media. Created lazily on first open; discarded on terminal close.
  /// Persisted (SerTab.rpd) so a window reload rebinds + the close cleanup
  /// targets the right draft (no leak).
  richPromptDraftPath?: string;
  /// Broadcast group label. A group is a plain string, not an allocated
  /// resource: it "exists" iff >=1 terminal references it, and is
  /// implicitly destroyed when the last member closes. Defaults to
  /// "default" (via `terminalTabGroup`); change requires a restart so the
  /// SPA group and the server's per-session `tab_group` (set at spawn from
  /// the same value) never diverge. Scopes the Cmd+Shift+I client
  /// broadcast: input fans out only to same-group terminals.
  group?: string;
  /// Keyboard-protocol negotiation state (xterm modifyOtherKeys / kitty
  /// flags) the running program announced. Lives on the TAB, not the
  /// TerminalTab component, so it survives a component remount: a remount
  /// on reattach to a long-lived PTY would otherwise reset it to zero,
  /// and a long-lived agent never re-announces its protocol after the
  /// reconnect, so modified-Enter (Shift+Enter -> newline) falls through
  /// to a plain submit. Reset only on a fresh spawn; kept across reattach.
  /// A compact snapshot is serialized to the session hash (`kp`) so the
  /// state ALSO survives a page reload reattaching to a long-lived agent
  /// whose original negotiation has scrolled out of the reattach replay
  /// ring (the heap is gone on a reload, so the in-memory copy cannot
  /// help; the replay would only re-establish a still-recent negotiation).
  keyboardProtocol?: TerminalKeyboardProtocolState;
  /// Set iff this is a Team Work LEAD terminal with the spawn-agents dialog
  /// open over it (Cmd+P, pre-Bootstrap). Holds the live config draft so a
  /// window reload reopens the dialog over the restored lead terminal with
  /// exactly what the user was editing. Seeded in `createTeamWorkLeadTerminal`,
  /// mirrored from the dialog on every edit, cleared on Bootstrap (the terminal
  /// becomes a committed lead), and gone with the tab on Cancel. Serialized as
  /// `SerTab.twk` in the per-window session payload ONLY (never the shareable
  /// URL hash) since a member's `env` can carry secrets.
  teamWorkPending?: TeamDialogConfig;
};

export type GraphFilters = {
  link: boolean;
  tag: boolean;
  mention: boolean;
  language: boolean;
  img: boolean;
  folder: boolean;
  /// FileBucket toggles - mirrors the `GraphFilters` shape in
  /// `state/store.svelte.ts`. Both files declare a local `GraphFilters`
  /// (one for the per-tab state here, one for the overlay state in
  /// store); they stay in lockstep when extended.
  markdown: boolean;
  source: boolean;
};

export type GraphTab = {
  kind: "graph";
  id: string;
  title: string;
  mode: "semantic" | "filesystem" | "language";
  scopeId: string;
  depth: number;
  /// Expanded directory set for the filesystem-mode spine (double-click a
  /// directory to reveal its next degree). Workspace-relative dir paths;
  /// the scope root ("") is always expanded. Serialized so the
  /// expand/collapse state survives a window reload (File Browser parity).
  expanded: Record<string, boolean>;
  filters: GraphFilters;
  inspectorOpen: boolean;
  pendingSelectId: string | null;
  /// Live selection state. `selectedNodeId` is the graph node id the
  /// user clicked (kept here, not just in `GraphPanel.svelte`'s component
  /// state, so the tab title can peek it from outside the panel).
  /// `selectedNodeLabel` is the human-readable label cached at click time
  /// so the tab strip can render the title before graph data finishes
  /// reloading on restore.
  selectedNodeId?: string | null;
  selectedNodeLabel?: string | null;
  /// Per-tab inspector width. Falls back to `paneWidths.graph` when unset.
  inspectorWidth?: number;
};

export type BrowserTab = {
  kind: "browser";
  id: string;
  title: string;
  inspectorOpen: boolean;
  /// Per-tab view state so two File Browser tabs in the same pane don't
  /// share selection / scroll / expansion via module-level singletons.
  /// Populated by `FileBrowserSurface.svelte` on tab activate and
  /// snapshot-back on deactivate.
  selected?: string | null;
  /// Multi-selection set (FB capabilities: shift/cmd-click, shift+arrows,
  /// cmd+A, rubber-band). Per-tab alongside `selected` (the active
  /// cursor) so selecting in one File Browser tab does not leak into
  /// another. Unset / empty means a single-entry (or no) selection.
  selectedPaths?: string[];
  showWorkspace?: boolean;
  expanded?: string[];
  scroll?: number;
  /// Per-tab inspector width so two FB tabs can carry different inspector
  /// sizes. Falls back to `paneWidths.browser` when unset.
  inspectorWidth?: number;
};

/// Dashboard tab - read-only surface hosting the shortcut table and
/// info panels. No per-tab state today; the placeholder fields keep the
/// discriminated union symmetric with the other tab kinds and let future
/// additions include view state without re-walking the persistence layer.
export type DashboardTab = {
  kind: "dashboard";
  id: string;
  title: string;
  /// Persisted carousel slide index so a reload restores the user to the
  /// slide they were last on. 0 is the About slide; 1 is Workspace; 2 is
  /// the Indexing graph. The carousel's play/pause is server-persisted so
  /// the auto-rotate preference survives a reload independently.
  carouselSlide?: number;
  /// Slide indices the user switched off via the Dashboard tab's
  /// right-click menu. Disabled slots are skipped in auto-rotation and
  /// hidden from the pagination dots. Absent / empty means all slots are
  /// enabled (the default); at least one slot always stays enabled
  /// (enforced in `toggleDashboardSlot`).
  disabledSlots?: number[];
  /// Whether this tab's carousel auto-rotates. Absent / true = on (the
  /// default); `cs dashboard --carousel-off` creates the tab with this
  /// false. Distinct from the global `empty_pane_carousel_cycling`
  /// preference: a per-tab opt-out so one static dashboard does not stop
  /// every dashboard from rotating.
  autoRotate?: boolean;
};

/// Carousel slot count, shared by the on/off helpers below and the
/// restore-time clamp. The carousel template renders exactly these three
/// slides (About / Workspace / Search); keeping the count here lets the
/// helpers reason about "the last enabled slot" without importing the
/// component.
export const DASHBOARD_SLOT_COUNT = 3;

/// Whether slide `i` is currently shown for this Dashboard tab.
export function dashboardSlotEnabled(tab: DashboardTab, i: number): boolean {
  return !(tab.disabledSlots ?? []).includes(i);
}

/// Toggle slide `i` on/off. Refuses to disable the last enabled slot so
/// the carousel never goes blank. The disabled set is stored sorted and
/// cleared entirely when every slot is back on (pre-release: omit the
/// field rather than persist an empty array).
export function toggleDashboardSlot(tab: DashboardTab, i: number): void {
  const disabled = new Set(tab.disabledSlots ?? []);
  if (disabled.has(i)) {
    disabled.delete(i);
  } else {
    if (DASHBOARD_SLOT_COUNT - disabled.size <= 1) return;
    disabled.add(i);
  }
  const next = [...disabled].sort((a, b) => a - b);
  tab.disabledSlots = next.length > 0 ? next : undefined;
}

/// First enabled slide index. Falls back to 0, which the min-one-enabled
/// invariant makes unreachable.
export function firstEnabledSlot(tab: DashboardTab): number {
  for (let i = 0; i < DASHBOARD_SLOT_COUNT; i++) {
    if (dashboardSlotEnabled(tab, i)) return i;
  }
  return 0;
}

/// Next enabled slide index after `from`, wrapping. Used by the carousel
/// auto-rotate + arrow nav so they step over disabled slots.
export function nextEnabledSlot(tab: DashboardTab, from: number): number {
  for (let step = 1; step <= DASHBOARD_SLOT_COUNT; step++) {
    const cand = (from + step) % DASHBOARD_SLOT_COUNT;
    if (dashboardSlotEnabled(tab, cand)) return cand;
  }
  return from;
}

export type Tab =
  | FileTab
  | TerminalTab
  | GraphTab
  | BrowserTab
  | DashboardTab;

type ClosedTab = {
  paneId: string;
  tab: Tab;
};

/// Middle-elision for tab strip titles. Targets a 15-code-point cap as
/// `head[..]tail` (6 + 4 + 5). The bias toward the tail keeps extensions
/// visible for the common cases (`.md`, `.ts`, `.svelte`, `.json`).
///
/// Counts code points via `Array.from` so a surrogate pair never splits
/// in the middle (emoji, CJK supplementary, etc.). Strings <= 15 code
/// points render as-is; the dirty marker (filled circle) lives outside
/// the label string in the tab strip render so the rule applies cleanly
/// to the visible name only.
///
/// Callers that present the truncated label should keep the full
/// untruncated value in the surrounding `title="..."` HTML attribute
/// (typically via `tabTooltip()`) so hover reveals it.
export const TAB_TITLE_MAX_LENGTH = 15;
const TAB_TITLE_HEAD = 6;
const TAB_TITLE_TAIL = 5;
const TAB_TITLE_ELLIPSIS = "[..]";

export function truncateTabTitle(label: string): string {
  const chars = Array.from(label);
  if (chars.length <= TAB_TITLE_MAX_LENGTH) return label;
  const head = chars.slice(0, TAB_TITLE_HEAD).join("");
  const tail = chars.slice(-TAB_TITLE_TAIL).join("");
  return `${head}${TAB_TITLE_ELLIPSIS}${tail}`;
}

/// Title for a Graph tab. Selection wins over scope: when the user
/// has tapped a node, the tab strip reads as the node's label
/// (basename for files / dirs, `#tag` for tags, contact name, etc.).
/// No selection falls back to the scope-derived title cached on
/// `tab.title`. Even with a selection, the kind= prefix from
/// `graphTitle()` is kept so the tab strip identifies the lens
/// shape (path= / tag= / contact= / lang=). Titles without an `=`
/// (top-level Languages overview, bare strings) render the bare label.
export function graphTabLabel(t: GraphTab): string {
  const label = t.selectedNodeLabel?.trim();
  if (!label) return t.title;
  const equalsAt = t.title.indexOf("=");
  if (equalsAt <= 0) return label;
  return `${t.title.slice(0, equalsAt)}=${label}`;
}

/// Optional context for `browserTabLabel`. `workspaceName` is the
/// display name to render when the tab points at the workspace root
/// (no selection, or a file directly under root). `selectedIsDir`
/// disambiguates "the user clicked a directory row" vs "the user
/// clicked a file row" when the path string alone is ambiguous;
/// when omitted, a trailing slash on `selected` is the fallback
/// signal.
export type BrowserLabelCtx = {
  workspaceName?: string;
  selectedIsDir?: boolean;
};

/// Short display label for a tab - the file's basename so the tab
/// strip stays scannable even when paths are deeply nested. The
/// full path is reachable via `tabTooltip` for disambiguation.
export function tabLabel(t: Tab, ctx?: BrowserLabelCtx): string {
  if (t.kind === "terminal") return terminalTabName(t);
  if (t.kind === "graph") return graphTabLabel(t);
  if (t.kind === "browser") return browserTabLabel(t, ctx);
  if (t.kind === "dashboard") return t.title;
  const p = t.path;
  if (!p) return p;
  const slash = p.lastIndexOf("/");
  return slash < 0 ? p : p.slice(slash + 1);
}

/// Files tab title is always a directory. File selection shows the
/// parent dir; directory selection shows that dir; no selection or
/// selection at workspace root shows the workspace display name.
/// Trailing slash is always rendered so the tab strip is unambiguous.
/// `ctx.workspaceName` is the display name for the workspace root
/// case; when absent, falls back to the tab's own `title` (default
/// `Files`) so unit tests that don't wire workspace context still work.
export function browserTabLabel(t: BrowserTab, ctx?: BrowserLabelCtx): string {
  const workspaceName = ctx?.workspaceName?.trim();
  const rootName = workspaceName || t.title;
  const selected = t.selected?.trim();
  if (!selected) return `${rootName}/`;
  const trailing = selected.endsWith("/");
  const cleaned = selected.replace(/\/+$/, "");
  if (!cleaned) return `${rootName}/`;
  const isDir = ctx?.selectedIsDir ?? trailing;
  const lastSlash = cleaned.lastIndexOf("/");
  if (isDir) {
    const dirName = lastSlash < 0 ? cleaned : cleaned.slice(lastSlash + 1);
    return `${dirName}/`;
  }
  if (lastSlash < 0) return `${rootName}/`;
  const parent = cleaned.slice(0, lastSlash);
  const parentSlash = parent.lastIndexOf("/");
  const parentName = parentSlash < 0 ? parent : parent.slice(parentSlash + 1);
  return `${parentName}/`;
}

/// Pane-local display label. Most tabs keep the basename. Duplicate
/// basenames collapse the group's shared prefix/suffix directories
/// and show only the shortest divergent ancestor; deeper divergent
/// tails render as `x/[...]/foo.md` to preserve tab-strip width.
export function tabLabelInPane(
  t: Tab,
  siblings: Tab[],
  ctx?: BrowserLabelCtx,
): string {
  if (t.kind !== "file") return tabLabel(t, ctx);
  const base = tabLabel(t);
  const duplicates = siblings.filter(
    (candidate): candidate is FileTab =>
      candidate.kind === "file" && tabLabel(candidate) === base,
  );
  if (duplicates.length <= 1) return base;

  const dirsById = new Map(
    duplicates.map((d) => [d.id, d.path.split("/").slice(0, -1)]),
  );
  const dirGroups = [...dirsById.values()];
  const prefixLen = commonPrefixLength(dirGroups);
  const suffixLen = commonSuffixLength(
    dirGroups.map((dirs) => dirs.slice(prefixLen)),
  );
  const targetDirs = dirsById.get(t.id) ?? [];
  const end = suffixLen > 0 ? targetDirs.length - suffixLen : targetDirs.length;
  const unique = targetDirs.slice(prefixLen, end);
  if (unique.length === 0) return t.path;
  if (unique.length === 1) return `${unique[0]}/${base}`;
  return `${unique[0]}/[...]/${base}`;
}

function commonPrefixLength(groups: string[][]): number {
  if (groups.length === 0) return 0;
  const max = Math.min(...groups.map((g) => g.length));
  let i = 0;
  for (; i < max; i++) {
    const value = groups[0]![i];
    if (!groups.every((g) => g[i] === value)) break;
  }
  return i;
}

function commonSuffixLength(groups: string[][]): number {
  if (groups.length === 0) return 0;
  const max = Math.min(...groups.map((g) => g.length));
  let i = 0;
  for (; i < max; i++) {
    const value = groups[0]![groups[0]!.length - 1 - i];
    if (!groups.every((g) => g[g.length - 1 - i] === value)) break;
  }
  return i;
}

/// Full path for a tab. Used as the tab's title attribute so two
/// files with the same basename in different directories can still be
/// told apart on hover.
export function tabTooltip(t: Tab): string {
  if (t.kind === "terminal") return terminalTabName(t);
  if (t.kind === "graph") {
    // Surface selection + scope so hover disambiguates two Graph tabs
    // viewing the same scope with different focal nodes, or two with
    // the same selection under different scopes.
    if (t.selectedNodeId) {
      return `Graph: ${t.scopeId} - ${t.selectedNodeId}`;
    }
    return `Graph: ${t.scopeId}`;
  }
  if (t.kind === "browser") {
    // Surface the per-tab selection so hover disambiguates two Files
    // tabs whose basenames collide (e.g. `index.md` in different dirs).
    return t.selected ? `File Browser: ${t.selected}` : "File Browser";
  }
  if (t.kind === "dashboard") return t.title;
  return t.path;
}

export function terminalTabName(t: TerminalTab): string {
  return t.title.trim() || "Terminal";
}

export const DEFAULT_TERMINAL_GROUP = "default";

/// The tab's broadcast group, normalized. An empty / unset group is
/// "default", so a brand-new terminal always belongs to a group and the
/// `"default"` string is never special-cased in code.
export function terminalTabGroup(t: TerminalTab): string {
  return t.group?.trim() || DEFAULT_TERMINAL_GROUP;
}

/// Set the tab's broadcast group (context-menu field). Stored normalized;
/// a blank value falls back to "default". The change takes effect on the
/// next spawn, so callers gate it behind a restart prompt to keep the SPA
/// group and the server-side `tab_group` consistent.
export function setTerminalGroup(t: TerminalTab, group: string): void {
  t.group = group.trim() || DEFAULT_TERMINAL_GROUP;
}

/// Return the tab's keyboard-protocol state, lazily creating it on first
/// use. `fresh` (a brand-new spawn, no surviving session) forces a clean
/// slate; a reattach keeps whatever the program previously negotiated so
/// modified-Enter keeps working across a component remount. See the
/// `keyboardProtocol` field on `TerminalTab`.
export function ensureTerminalKeyboardProtocol(
  tab: TerminalTab,
  fresh: boolean,
): TerminalKeyboardProtocolState {
  if (!tab.keyboardProtocol) {
    tab.keyboardProtocol = createTerminalKeyboardProtocolState();
  } else if (fresh) {
    resetTerminalKeyboardProtocolState(tab.keyboardProtocol);
  }
  return tab.keyboardProtocol;
}

function terminalTabsIn(state: LayoutState): TerminalTab[] {
  const out: TerminalTab[] = [];
  for (const node of Object.values(state.nodes)) {
    if (node.kind !== "leaf") continue;
    for (const tab of node.tabs) {
      if (tab.kind === "terminal") out.push(tab);
    }
  }
  return out;
}

function nextTerminalTitle(state: LayoutState = layout): string {
  let max = 0;
  for (const tab of terminalTabsIn(state)) {
    const title = terminalTabName(tab);
    const match = /^Terminal(?:-(\d+))?$/.exec(title);
    if (!match) continue;
    const n = match[1] ? Number(match[1]) : 1;
    if (Number.isInteger(n) && n > max) max = n;
  }
  return `Terminal-${max + 1}`;
}

/// Disambiguate a desired terminal name against the OTHER live terminals with a
/// numeric `-N` suffix (the same shape as the Terminal-N default + the cs-team
/// group `-N`). Two terminals must never share a name: `cs terminal write
/// --tab-name` targets BY NAME, so a duplicate would double-deliver / ambiguously
/// route the poke+queue, breaking the serialized-input model. `excludeTabId`
/// skips the tab being renamed so it never collides with itself.
///
/// Uniqueness is TENANT-WIDE, not per-window: a rename in one window must avoid
/// names already taken by terminals in OTHER windows (read from the
/// cross-window roster), and across groups (a name unique only within a group
/// would collide the moment a terminal is moved between groups). Comparison is
/// case-sensitive, matching the server's by-name targeting.
export function uniqueTerminalName(
  desired: string,
  excludeTabId?: string,
  excludeSessionId?: string,
): string {
  const base = desired.trim() || "Terminal";
  const taken = new Set(
    terminalTabsIn(layout)
      .filter((t) => t.id !== excludeTabId)
      .map((t) => terminalTabName(t)),
  );
  // Fold in terminals from OTHER windows. Skip this window's own entries (the
  // local layout above is authoritative for them, and its names can be ahead
  // of the roster on a not-yet-restarted rename) and this tab's own session
  // (so a no-op rename can keep its current name). An explicit
  // `excludeSessionId` (a cross-window MOVE re-attaching its OWN live PTY,
  // still listed under the source window's roster entry at drop time) takes
  // precedence so the moved terminal never collides with itself.
  const myWindow = sessionWindowId();
  const ownSession =
    excludeSessionId ??
    (excludeTabId
      ? terminalTabsIn(layout).find((t) => t.id === excludeTabId)?.terminalSessionId
      : undefined);
  for (const entry of terminalRoster) {
    if (entry.window_id === myWindow) continue;
    if (ownSession && entry.id === ownSession) continue;
    if (entry.tab_name) taken.add(entry.tab_name);
  }
  if (!taken.has(base)) return base;
  for (let n = 2; ; n += 1) {
    const candidate = `${base}-${n}`;
    if (!taken.has(candidate)) return candidate;
  }
}

/// Apply the per-tenant `Terminal-N` default name to a freshly-spawned
/// terminal, in BOTH terminal-only and workspace modes. The spawn helpers are
/// synchronous and create the tab with the local `nextTerminalTitle()`
/// placeholder; this fetches the next ordinal from the server and replaces the
/// placeholder so numbering is consistent across every window of the tenant (a
/// per-window count restarts at 1 in each new window). The counter is
/// per-tenant: standalone terminal windows share one tenant -> one global
/// sequence; each workspace has its own tenant -> a per-workspace sequence
/// shared by that workspace's windows. The server counter is atomic, so two
/// quick Cmd+T presses each resolve a DISTINCT name against a DISTINCT tab
/// object - no collision. `uniqueTerminalName` is a defensive dedup against
/// the still-present local placeholder; `excludeTabId` skips the target tab so
/// it never collides with its own placeholder. A failed fetch (offline) leaves
/// the local placeholder in place rather than throwing.
export async function applyGlobalTerminalName(tab: TerminalTab): Promise<void> {
  try {
    const name = (await api.terminalNextName()).trim();
    if (!name) return;
    // Mutate the LIVE tab from the layout, NOT the passed reference. A tab
    // object pushed into `$state` is only reactive through its Svelte proxy;
    // writing `tab.title` on the original (pre-push) object updates the data
    // but never re-renders the name. Re-find the proxy by id (this also
    // covers the tab being closed / moved out mid-fetch -> not found -> skip).
    const live = allTerminalTabs().find((t) => t.id === tab.id);
    if (!live) return;
    live.title = uniqueTerminalName(name, tab.id);
  } catch {
    // Keep the local placeholder name; the global counter is a nicety, not a
    // correctness requirement.
  }
}

/// Each pane (Hybrid in user-facing copy) holds an optional back-side
/// slot. The back is a per-surface configuration view scoped to the
/// type of the currently-active front tab. `pane.tabs` /
/// `pane.activeTabId` always describe the FRONT side; `showingBack`
/// toggles which surface renders. Both sides share `pane.theme`.
export type HybridTheme = "dark" | "light";

export type HybridSide = {
  /// Marker type for "this pane has been flipped at least once."
  /// Empty body: `pane.theme` owns the single per-Hybrid theme.
  /// `pane.back !== undefined` discriminates Hybrid vs never-flipped
  /// for menu gating in `Pane.svelte`.
};

export type Pane = {
  id: string;
  tabs: Tab[];
  activeTabId: string | null;
  /// Visible-side theme override (`undefined` = follow global).
  theme?: HybridTheme;
  /// Hidden side. `undefined` for never-flipped panes; the first
  /// `flipHybrid()` call lazily materializes it so `pane.back !== undefined`
  /// gates the Hybrid menu entries.
  back?: HybridSide;
  /// User-visible flag for "this Hybrid is currently flipped to its
  /// back-side configuration view". Independent of whether `back`
  /// exists: a pane that has been flipped twice has
  /// `back !== undefined` but `showingBack === false` again.
  /// Defaults to false.
  showingBack?: boolean;
};

export type FocusColor = "blue" | "orange" | "green" | "pink";

export type Split = {
  id: string;
  kind: "split";
  direction: "row" | "column";
  /// children IDs from `nodes` map.
  a: string;
  b: string;
  /// Fraction of the split's main-axis length given to child `a`.
  /// Range (0..1); 0.5 is even. Updated live by the divider drag in
  /// Workspace.svelte and persisted in the URL hash.
  ratio: number;
};

export type LeafNode = Pane & { kind: "leaf" };
export type SplitNode = Split;
export type Node = LeafNode | SplitNode;

export const layout = $state<{
  rootId: string;
  nodes: Record<string, Node>;
  activePaneId: string;
  focusColor: FocusColor;
}>(
  (() => {
    const pane: LeafNode = {
      kind: "leaf",
      id: id("pane"),
      tabs: [],
      activeTabId: null,
    };
    return {
      rootId: pane.id,
      nodes: { [pane.id]: pane },
      activePaneId: pane.id,
      focusColor: "blue",
    };
  })(),
);

export type LayoutState = typeof layout;

/// Staged spawn intent for Pane Mode number keys. The intent queues
/// here so the pane-mode "Enter commit / Esc discard" contract holds
/// for every keystroke; `commitPaneMode()` materializes it as part of
/// the seal. A second staging overwrites the first; Esc clears it
/// without spawning.
export type PaneModeSpawnKind = "terminal" | "browser" | "graph";
export type PaneModeSpawnIntent = {
  kind: PaneModeSpawnKind;
  ctx: SpawnContext;
};

/// Mouse-driven variant of Hybrid Nav. Entered by drag-from-dead-zone
/// (sets `grabPaneId` to the originating pane) or by double-click on
/// the dead zone (`grabPaneId` stays null until the user clicks and
/// drags inside a pane). Mouse handlers in transaction mode operate on
/// the full pane body, not just the top bar. Enter / Esc / Cmd+. exit
/// through the same paths as keyboard Nav.
export const paneMode = $state<{
  active: boolean;
  draft: LayoutState | null;
  spawnIntent: PaneModeSpawnIntent | null;
  transactionMode: boolean;
  grabPaneId: string | null;
  hoverPaneId: string | null;
  /// Queue of "new draft editor" intents staged during the current
  /// pane-mode session. Materialized on Enter (commit); discarded on
  /// Esc (cancel). Each entry pins the target paneId at press time so
  /// later focus changes don't redirect materialization.
  stagedDraftEditors: { paneId: string }[];
}>({
  active: false,
  draft: null,
  spawnIntent: null,
  transactionMode: false,
  grabPaneId: null,
  hoverPaneId: null,
  stagedDraftEditors: [],
});

/// Single-fire wobble bus. Each pane's entry holds a monotonic
/// counter; bumping it on a structural event (split / close /
/// pane-move) lets the Pane component re-trigger its CSS
/// animation by toggling the wobble class. Counters never reset
/// because the consumer only cares about the change, not the
/// value.
export const paneWobble = $state<{ versions: Record<string, number> }>({
  versions: {},
});

export function requestPaneWobble(paneId: string): void {
  if (!paneId) return;
  paneWobble.versions[paneId] = (paneWobble.versions[paneId] ?? 0) + 1;
}

/// The Hybrid flip no longer needs an event bus. The two-face card in
/// Pane.svelte rotates via a CSS transition driven directly by
/// `showingBack`, so toggling that boolean is the entire trigger. Only
/// the structural-change cue (`paneWobble`: split / close / swap)
/// keeps its versioned-counter bus; orientation changes do not.

export function activeLayout(): LayoutState {
  return paneMode.active && paneMode.draft ? paneMode.draft : layout;
}

function pane(id: string): LeafNode {
  const n = layout.nodes[id];
  if (!n || n.kind !== "leaf") throw new Error(`not a pane: ${id}`);
  return n;
}

export function activePane(): LeafNode {
  const current = activeLayout();
  const n = current.nodes[current.activePaneId];
  if (!n || n.kind !== "leaf") throw new Error(`not a pane: ${current.activePaneId}`);
  return n;
}

const CLOSED_TAB_LIMIT = 20;
const recentlyClosedTabs = $state<ClosedTab[]>([]);
const localTabDrops = new Set<string>();

function tabDropKey(paneId: string, tabId: string): string {
  return `${paneId}:${tabId}`;
}

export function markLocalTabDrop(fromPaneId: string, tabId: string): void {
  localTabDrops.add(tabDropKey(fromPaneId, tabId));
}

export function shouldCloseTabAfterDragEnd(
  paneId: string,
  tabId: string,
  dropEffect: string | undefined,
): boolean {
  // A cross-window drop that a target accepted (dropEffect === "move") leaves
  // the source tab still in this pane: remove it so the visual matches the
  // cross-window result. This now ALSO applies to terminals: all standalone
  // terminal windows share one `/terminal` tenant (one PTY registry), so the
  // target window re-attached to this SAME live PTY by id - a true MOVE. The
  // source close is made PTY-preserving by the drag-end's `markTerminalMovingOut`
  // (the close-sink then skips the WS `close` frame), so the terminal leaves
  // here with its shell + history intact and reappears in the target with no
  // duplicate. If the source pane then becomes empty, the close-on-last-tab
  // watcher closes the window - correct (no empty terminal window).
  if (dropEffect !== "move") return false;
  const localDrop = localTabDrops.delete(tabDropKey(paneId, tabId));
  const n = layout.nodes[paneId];
  if (!n || n.kind !== "leaf") return false;
  const stillHere = n.tabs.some((t) => t.id === tabId);
  return stillHere && !localDrop;
}

export function canReopenClosedTab(): boolean {
  return recentlyClosedTabs.length > 0;
}

export function clearRecentlyClosedTabsForTest(): void {
  recentlyClosedTabs.length = 0;
  localTabDrops.clear();
}

function rememberClosedTab(paneId: string, tab: Tab): void {
  recentlyClosedTabs.push({ paneId, tab: cloneTab(tab) });
  if (recentlyClosedTabs.length > CLOSED_TAB_LIMIT) {
    recentlyClosedTabs.splice(0, recentlyClosedTabs.length - CLOSED_TAB_LIMIT);
  }
}

export function reopenClosedTab(): boolean {
  const entry = recentlyClosedTabs.pop();
  if (!entry) return false;
  const targetNode = layout.nodes[entry.paneId];
  const target =
    targetNode && targetNode.kind === "leaf" ? targetNode : activePane();
  const tab = tabForReopen(entry.tab);
  if (tabIdExists(tab.id)) {
    tab.id = id(tab.kind === "terminal" ? "term" : "tab");
  }
  target.tabs.push(tab);
  target.activeTabId = tab.id;
  layout.activePaneId = target.id;
  return true;
}

function tabIdExists(tabId: string): boolean {
  return Object.values(layout.nodes).some(
    (node) => node.kind === "leaf" && node.tabs.some((tab) => tab.id === tabId),
  );
}

function tabForReopen(src: Tab): Tab {
  const tab = cloneTab(src);
  if (tab.kind === "terminal") {
    tab.terminalSessionId = undefined;
    tab.controlledTerminal = undefined;
    tab.lastAgentEchoSeq = undefined;
    tab.terminalEnvTabName = undefined;
    tab.terminalEnvNamePromptDismissed = undefined;
  }
  return tab;
}

/// Ensure the named tab has a FindState attached and open it.
/// Called by the chan:command "app.find.open" handler. Idempotent:
/// reopening a bar that's already open just refocuses the input
/// (FindBar's mount effect handles that side).
export function openFind(tabId: string): void {
  const found = findFileTabById(tabId);
  if (!found) return;
  if (!found.tab.find) found.tab.find = makeFindState();
  found.tab.find.open = true;
  found.tab.find.focusNonce += 1;
}

/// Close the find bar for the named tab. Leaves the query string
/// in place so reopening picks up where the user left off; the
/// matches array is cleared (FindBar's onDestroy already clears
/// the editor decorations).
export function closeFind(tabId: string): void {
  const found = findFileTabById(tabId);
  if (!found || !found.tab.find) return;
  found.tab.find.open = false;
  found.tab.find.matches = [];
  found.tab.find.currentIndex = -1;
  found.tab.find.truncated = false;
}

/// Active file tab of the focused pane, or null if the pane is
/// empty / its active tab isn't a file tab. Used by the host-
/// driven command bridge (App.svelte runCommand) so app.find.*
/// can target whichever tab the user is currently looking at
/// without each call site re-deriving the lookup.
export function activeFileTab(): FileTab | null {
  const p = activePane();
  if (!p.activeTabId) return null;
  const t = p.tabs.find((tab) => tab.id === p.activeTabId);
  if (!t || t.kind !== "file") return null;
  return t;
}

export function activeTerminalTab(): TerminalTab | null {
  const p = activePane();
  if (!p.activeTabId) return null;
  const t = p.tabs.find((tab) => tab.id === p.activeTabId);
  if (!t || t.kind !== "terminal") return null;
  return t;
}

/// Toggle broadcast SELECT-ALL / DESELECT-ALL for the active terminal.
/// Chord-driven (Cmd+Shift+I) equivalent of the per-tab "Select All" /
/// "Deselect All" button. No-op when the active tab isn't a terminal.
export function toggleActiveTerminalBroadcastSelectAll(): void {
  const tab = activeTerminalTab();
  if (tab) toggleTerminalGroupBroadcast(tab);
}

/// Select-all / deselect-all for a terminal's whole broadcast GROUP, spanning
/// every window of the tenant (not just the local layout). Local same-group
/// tabs flip directly (self via broadcastEnabled, others via targets);
/// same-group terminals in OTHER windows flip via the server, which routes a
/// `terminal_broadcast` command to their owning window. "All on" is computed
/// across both so one click consistently fills or clears the group. Backs the
/// menu button and the Cmd+Shift+I chord.
export function toggleTerminalGroupBroadcast(tab: TerminalTab): void {
  const group = terminalTabGroup(tab);
  const localTargets = allTerminalTabs().filter(
    (t) => terminalTabGroup(t) === group,
  );
  const crossMembers = crossWindowBroadcastMembers(tab);
  if (localTargets.length === 0 && crossMembers.length === 0) return;
  const selected = new Set(terminalBroadcastMemberIds(tab));
  const localAllOn = localTargets.every((t) =>
    t.id === tab.id ? tab.broadcastEnabled : selected.has(t.id),
  );
  const crossAllOn = crossMembers.every((m) => m.broadcast);
  const select = !(localAllOn && crossAllOn);
  for (const target of localTargets) {
    if (target.id === tab.id) {
      setTerminalBroadcastEnabled(tab, select);
    } else {
      setTerminalBroadcastTarget(tab, target.id, select);
    }
  }
  for (const member of crossMembers) {
    void api.setTerminalSessionBroadcast(member.id, select);
  }
}

/// Set a local terminal's broadcast toggle by its live session id. This is
/// the `terminal_broadcast` window-command entry point: another window's Select All /
/// per-row toggle reaches the owning window here, which flips the matching
/// tab so the normal `set-broadcast` sync + sign + fan run unchanged. No-op
/// when no local tab hosts that session.
export function setTerminalBroadcastBySession(sessionId: string, on: boolean): void {
  const tab = allTerminalTabs().find((t) => t.terminalSessionId === sessionId);
  if (tab) setTerminalBroadcastEnabled(tab, on);
}

/// Team Work lead-terminal factory. Spawns a fresh (normal) terminal in the
/// active pane and returns it. The Cmd+P flow uses the returned handle so the
/// Team dialog can delete the just-spawned terminal if the user cancels before
/// committing; the orchestrator delivers the lead's identity prompt through the
/// write queue (the lead is a normal terminal - no bubble).
export function createTeamWorkLeadTerminal(
  opts: OpenTerminalOptions = {},
): TerminalTab | null {
  const p = activePane();
  const tab = openTerminalInPane(p.id, opts);
  // Mark it as a pending Team Work lead carrying the dialog's initial config.
  // The dialog edits this draft in place (and a reload reopens the dialog from
  // it); see `TerminalTab.teamWorkPending`.
  if (tab) tab.teamWorkPending = defaultTeamConfig();
  return tab;
}

/// Locate the open Team Work spawn-agents dialog's lead terminal, if any: the
/// single terminal tab still flagged `teamWorkPending`. Returns the
/// `{leadTabId, leadPaneId}` the dialog reopens against, or null when no team
/// setup is in flight. Used on reload by `store.svelte.ts` to reopen the dialog
/// over the restored lead terminal (the tab ids regenerate on restore, so the
/// per-tab flag is the bridge back to the right pane + tab).
export function findTeamWorkPendingLead(): TeamDialogRequest | null {
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    for (const tab of node.tabs) {
      if (tab.kind === "terminal" && tab.teamWorkPending) {
        return { leadTabId: tab.id, leadPaneId: node.id };
      }
    }
  }
  return null;
}

/// Read the live config draft for a Team Work lead terminal (the dialog seeds
/// its form from this on open / reload). Null when the request no longer maps
/// to a pending lead terminal.
export function teamWorkPendingConfig(
  req: TeamDialogRequest,
): TeamDialogConfig | null {
  return teamWorkLeadTab(req)?.teamWorkPending ?? null;
}

/// Mirror the dialog's live config draft onto its lead terminal so it rides the
/// session payload (`SerTab.twk`) and survives a reload. The dialog calls this
/// on every edit; the caller also schedules a session save.
export function setTeamWorkPendingConfig(
  req: TeamDialogRequest,
  config: TeamDialogConfig,
): void {
  const tab = teamWorkLeadTab(req);
  if (tab) tab.teamWorkPending = config;
}

/// Clear the pending-dialog flag once Team Work has bootstrapped: the lead
/// terminal becomes a committed lead and must no longer reopen the dialog on a
/// subsequent reload.
export function clearTeamWorkPending(req: TeamDialogRequest): void {
  const tab = teamWorkLeadTab(req);
  if (tab) tab.teamWorkPending = undefined;
}

/// Resolve a dialog request to its lead TerminalTab, or null if the pane/tab no
/// longer exists or isn't a terminal.
function teamWorkLeadTab(req: TeamDialogRequest): TerminalTab | null {
  const node = layout.nodes[req.leadPaneId];
  if (!node || node.kind !== "leaf") return null;
  const tab = node.tabs.find((t) => t.id === req.leadTabId);
  return tab && tab.kind === "terminal" ? tab : null;
}

export type OpenTerminalOptions = {
  cwd?: string;
  seedInput?: string;
  title?: string;
  sessionId?: string;
  controlledTerminal?: boolean;
  group?: string;
};

export function openTerminalInActivePane(opts: OpenTerminalOptions = {}): TerminalTab | null {
  return openTerminalInPane(activePane().id, opts);
}

export function openTerminalInPane(
  paneId: string,
  opts: OpenTerminalOptions = {},
): TerminalTab | null {
  const p = layout.nodes[paneId];
  if (!p || p.kind !== "leaf") return null;
  const cwd = opts.cwd?.trim();
  const seedInput = opts.seedInput?.trim();
  const title = opts.title?.trim();
  const group = opts.group?.trim();
  const tab: TerminalTab = {
    kind: "terminal",
    id: id("term"),
    // A passed name (cs terminal new --tab-name, reopen) is deduped so two
    // terminals never share a name; an unnamed spawn gets the unique Terminal-N.
    title: title ? uniqueTerminalName(title) : nextTerminalTitle(),
    createdAt: Date.now(),
    broadcastEnabled: false,
    broadcastTargetIds: [],
    terminalSessionId: opts.sessionId?.trim() || undefined,
    controlledTerminal: opts.controlledTerminal || undefined,
    cwd: cwd || undefined,
    seedInput: seedInput || undefined,
    group: group && group !== DEFAULT_TERMINAL_GROUP ? group : undefined,
  };
  p.tabs.push(tab);
  p.activeTabId = tab.id;
  layout.activePaneId = p.id;
  // Number from the per-tenant counter so Terminal-N stays consistent across
  // every window of the tenant (all terminal windows, or all windows of one
  // workspace). Only an UNNAMED spawn (no explicit title - e.g. `cs terminal
  // new --tab-name` passes one) takes the server name. Flagged here; the name
  // is resolved in `connect()` BEFORE the WS opens, so the session spawns
  // with its final name (the placeholder never reaches the server).
  if (!title) {
    tab.pendingGlobalName = true;
  }
  return tab;
}

/// Re-attach payload for a session-preserving cross-window terminal MOVE.
/// Carried in the cross-window drag (CROSS_TAB_MIME) and consumed by
/// `reattachTerminalInPane`. All standalone terminal windows share one
/// `/terminal` tenant (one PTY registry), so the target window can attach to
/// this SAME live PTY by `terminalSessionId` instead of spawning a fresh
/// shell. The target's fresh xterm is empty, so its attach always replays
/// the session's full ring (like a reload); only the echo-dedupe cursor
/// travels with the move.
export type TerminalMovePayload = {
  terminalSessionId: string;
  title?: string;
  /// The moved shell's real CHAN_TAB_NAME (the source tab's env-name snapshot),
  /// so the target can tell whether a conflict-forced `-N` rename leaves the
  /// env stale and must surface the restart warning.
  terminalEnvTabName?: string;
  lastAgentEchoSeq?: number;
  group?: string;
  cwd?: string;
};

/// Re-attach a MOVED terminal to its existing live PTY in the target window's
/// pane. Distinct from `openTerminalInPane({ sessionId })`: this preserves the
/// moved terminal's NAME verbatim (NO renumber - it's the same terminal, just
/// in a new window) and carries the echo-dedupe cursor so agent echo replay
/// stays suppressed. The source tab is removed WITHOUT killing
/// the PTY (see `closeTab`'s `keepSession`), so the net effect is the terminal
/// leaving the source and appearing here with the same shell + history and no
/// duplicate. The PTY lives in the shared registry, so the attach succeeds.
export function reattachTerminalInPane(
  paneId: string,
  payload: TerminalMovePayload,
): TerminalTab | null {
  const p = layout.nodes[paneId];
  if (!p || p.kind !== "leaf") return null;
  const sessionId = payload.terminalSessionId?.trim();
  if (!sessionId) return null;
  const group = payload.group?.trim();
  const tab: TerminalTab = {
    kind: "terminal",
    id: id("term"),
    // Preserve the moved terminal's name. Dedup against OTHER terminals only:
    // exclude this same session (still live in the source window's roster
    // entry at drop time) so the terminal never collides with ITSELF, and a
    // `-N` suffix is added ONLY on a real conflict with a different terminal.
    title: uniqueTerminalName(payload.title?.trim() || "Terminal", undefined, sessionId),
    createdAt: Date.now(),
    broadcastEnabled: false,
    broadcastTargetIds: [],
    terminalSessionId: sessionId,
    // Carry the moved shell's real CHAN_TAB_NAME. The WS re-attaches to the
    // SAME session id, so setTerminalSession's `wasFresh` stays false and does
    // not overwrite this. If a conflict forced a `-N` suffix above (title !=
    // env name), terminalEnvTabNameStale is then true and the existing
    // stale-env warning fires (restart to sync CHAN_TAB_NAME); with no suffix
    // the names match and no warning shows.
    terminalEnvTabName: payload.terminalEnvTabName,
    controlledTerminal: undefined,
    // Carry the echo-dedupe cursor so the re-attach (`connect()` sends
    // `sessionId` + `agentEchoSince`) keeps agent echoes suppressed. The
    // fresh xterm replays the full ring, same as a reload.
    lastAgentEchoSeq: payload.lastAgentEchoSeq,
    cwd: payload.cwd?.trim() || undefined,
    seedInput: undefined,
    group: group && group !== DEFAULT_TERMINAL_GROUP ? group : undefined,
  };
  p.tabs.push(tab);
  p.activeTabId = tab.id;
  layout.activePaneId = p.id;
  // Pull keyboard focus to the just-dropped terminal: making it the active
  // tab isn't enough on its own (the terminal's focus effect only grabs the
  // xterm on a focus pulse), so fire the same pulse a chord-driven tab switch
  // uses.
  bumpTabFocusPulse();
  return tab;
}

export type OpenGraphOptions = Partial<
  Pick<GraphTab, "mode" | "scopeId" | "depth" | "pendingSelectId" | "title" | "filters">
>;

const DEFAULT_GRAPH_FILTERS: GraphFilters = {
  link: true,
  tag: true,
  mention: true,
  language: true,
  img: true,
  folder: true,
  markdown: true,
  source: true,
};

export function openGraphInActivePane(opts: OpenGraphOptions = {}): GraphTab {
  return openGraphInPane(layout.activePaneId, opts);
}

export function openGraphInPane(paneId: string, opts: OpenGraphOptions = {}): GraphTab {
  const p = pane(paneId);
  const mode = opts.mode ?? "semantic";
  const scopeId = opts.scopeId ?? "workspace";
  // No dedup on spawn. Each invocation creates a fresh graph tab with
  // its own scope, filters, and pending selection so the user can
  // compare two views side-by-side. Callers that want "activate the
  // existing one" can find it on `pane.tabs` and set `activeTabId`.
  const tab: GraphTab = {
    kind: "graph",
    id: id("graph"),
    title: opts.title ?? graphTitle(mode, scopeId),
    mode,
    scopeId,
    depth: opts.depth ?? 1,
    expanded: { "": true },
    filters: opts.filters ? { ...opts.filters } : { ...DEFAULT_GRAPH_FILTERS },
    inspectorOpen: false,
    pendingSelectId: opts.pendingSelectId ?? null,
  };
  p.tabs.push(tab);
  p.activeTabId = tab.id;
  layout.activePaneId = p.id;
  return tab;
}

export function openBrowserInActivePane(
  opts: { select?: string | null } = {},
): BrowserTab {
  const p = activePane();
  // No dedup. Each press spawns a new browser tab with its own current
  // dir and inspector state.
  const tab: BrowserTab = {
    kind: "browser",
    id: id("browser"),
    title: nextBrowserTitle(),
    inspectorOpen: defaultBrowserInspectorOpen(),
    ...(opts.select ? { selected: opts.select } : {}),
  };
  p.tabs.push(tab);
  p.activeTabId = tab.id;
  layout.activePaneId = p.id;
  return tab;
}

/// Mirrors `nextTerminalTitle`: walk every existing browser tab,
/// find the highest "Files" / "Files N" number, return next. The
/// title is what `browserTabLabel`'s fallback path uses when the
/// workspace context isn't wired (unit tests, edge surfaces) AND it
/// also matters when two unselected FB tabs sit side-by-side - /// numbering disambiguates them in the tab strip.
function nextBrowserTitle(): string {
  let max = 0;
  let hasUnnumbered = false;
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    for (const tab of node.tabs) {
      if (tab.kind !== "browser") continue;
      const match = /^Files(?: (\d+))?$/.exec(tab.title);
      if (!match) continue;
      if (match[1] === undefined) {
        hasUnnumbered = true;
      } else {
        const n = Number(match[1]);
        if (Number.isInteger(n) && n > max) max = n;
      }
    }
  }
  if (!hasUnnumbered) return "Files";
  return `Files ${Math.max(max + 1, 2)}`;
}

function defaultBrowserInspectorOpen(): boolean {
  if (typeof window === "undefined") return true;
  return window.innerWidth >= 768;
}

/// Title for a Graph tab. The tab name reads as the basename of
/// whatever the user scoped the graph to (file basename, dir name,
/// contact name, `#tag`) so the tab strip identifies the subject
/// directly. Every title carries a `kind=` prefix so the tab strip
/// also identifies the lens shape (`path=` / `tag=` / `contact=` /
/// `lang=`). `mode === "language"` is a top-level lens (not per-scope)
/// and keeps its dedicated `Languages` label.
export function graphTitle(mode: GraphTab["mode"], scopeId: string): string {
  if (mode === "language") return "Languages";
  if (scopeId === "workspace" || scopeId === "global") return "path=workspace";
  if (scopeId.startsWith("file:")) {
    const name = graphScopeBasename(scopeId.slice("file:".length));
    return name ? `path=${name}` : "path=workspace";
  }
  if (scopeId.startsWith("dir:")) {
    const name = graphScopeBasename(scopeId.slice("dir:".length));
    return name ? `path=${name}/` : "path=workspace";
  }
  if (scopeId.startsWith("tag:")) {
    const tag = scopeId.slice("tag:".length);
    return `tag=${tag.startsWith("#") ? tag : `#${tag}`}`;
  }
  if (scopeId.startsWith("mention:")) {
    const mention = scopeId.slice("mention:".length);
    return `mention=${mention.startsWith("@@") ? mention : `@@${mention}`}`;
  }
  if (scopeId.startsWith("contact:")) {
    const name = graphScopeBasename(scopeId.slice("contact:".length));
    return `contact=${name || scopeId.slice("contact:".length)}`;
  }
  if (scopeId.startsWith("language:")) {
    return `lang=${scopeId.slice("language:".length)}`;
  }
  if (scopeId.startsWith("git_repo:")) {
    return graphScopeBasename(scopeId.slice("git_repo:".length));
  }
  // Unknown prefix shape: peel anything before the first colon
  // so the user at least sees the payload.
  const colon = scopeId.indexOf(":");
  if (colon > 0) return scopeId.slice(colon + 1);
  return scopeId;
}

function graphScopeBasename(path: string): string {
  const i = path.lastIndexOf("/");
  return i < 0 ? path : path.slice(i + 1);
}

export function renameTerminalTab(tab: TerminalTab, title: string): void {
  // Enforce unique terminal names (auto-disambiguate, never reject). Programmatic
  // callers (team spawn) get uniqueness here; the live rename input commits
  // through this on blur (it sets tab.title raw while typing to avoid a
  // per-keystroke cursor jump, then dedupes on commit).
  tab.title = uniqueTerminalName(title, tab.id);
  if (terminalEnvTabNameStale(tab)) tab.terminalEnvNamePromptDismissed = false;
}

export function terminalEnvTabNameStale(tab: TerminalTab): boolean {
  return Boolean(
    tab.terminalSessionId &&
      tab.terminalEnvTabName !== undefined &&
      terminalTabName(tab) !== tab.terminalEnvTabName,
  );
}

export function dismissTerminalEnvNamePrompt(tab: TerminalTab): void {
  tab.terminalEnvNamePromptDismissed = true;
}

export function markTerminalEnvNameRestarted(tab: TerminalTab): void {
  tab.terminalEnvTabName = terminalTabName(tab);
  tab.terminalEnvNamePromptDismissed = false;
}

export function setTerminalBroadcastEnabled(tab: TerminalTab, enabled: boolean): void {
  const members = terminalBroadcastGroupIds();
  if (enabled) {
    members.add(tab.id);
  } else {
    members.delete(tab.id);
  }
  applyTerminalBroadcastMembers(members);
}

export function toggleActiveTerminalBroadcast(): void {
  const tab = activeTerminalTab();
  if (!tab) return;
  setTerminalBroadcastEnabled(tab, !tab.broadcastEnabled);
}

export function setTerminalBroadcastTarget(
  tab: TerminalTab,
  targetId: string,
  enabled: boolean,
): void {
  void tab;
  const next = terminalBroadcastGroupIds();
  if (enabled) next.add(targetId);
  else next.delete(targetId);
  applyTerminalBroadcastMembers(next);
}

export function terminalBroadcastMemberIds(tab: TerminalTab): string[] {
  // Broadcast is scoped to the source's group: only same-group terminals
  // are ever members. A single-member group has no targets (no-op).
  const group = terminalTabGroup(tab);
  const inGroup = new Set(
    allTerminalTabs()
      .filter((t) => terminalTabGroup(t) === group)
      .map((t) => t.id),
  );
  return [...terminalBroadcastGroupIds()].filter((id) => inGroup.has(id));
}

export function removeTerminalFromBroadcastGroup(tab: TerminalTab, memberId: string): void {
  void tab;
  const next = terminalBroadcastGroupIds();
  next.delete(memberId);
  applyTerminalBroadcastMembers(next);
}

function terminalBroadcastGroupIds(): Set<string> {
  const validIds = new Set(allTerminalTabs().map((tab) => tab.id));
  const ids = new Set<string>();
  for (const tab of allTerminalTabs()) {
    if (tab.broadcastEnabled) ids.add(tab.id);
    for (const targetId of tab.broadcastTargetIds) ids.add(targetId);
  }
  return new Set([...ids].filter((id) => validIds.has(id)));
}

function applyTerminalBroadcastMembers(members: Set<string>): void {
  const next = new Set(members);
  for (const tab of allTerminalTabs()) {
    if (!next.has(tab.id)) {
      tab.broadcastEnabled = false;
      tab.broadcastTargetIds = [];
      continue;
    }
    tab.broadcastEnabled = true;
    tab.broadcastTargetIds = [...next].filter((id) => id !== tab.id);
  }
}

// --- Cross-window terminal roster ---
// `allTerminalTabs()` is this window's local layout only. The roster carries
// every live session across ALL windows of the tenant (all standalone
// terminal windows, or all windows of one workspace), so a window can see
// same-group terminals it does not host. Seeded from `api.terminalRoster()`
// on `/ws` (re)connect and refreshed by `terminal_roster` `/ws` frames.
let terminalRoster = $state<TerminalRosterEntry[]>([]);

/// Replace the roster wholesale (the server pushes full snapshots, so there
/// is no delta to reconcile). Called by the `/ws` `terminal_roster` handler
/// and the reconnect seed.
export function applyTerminalRoster(entries: TerminalRosterEntry[]): void {
  terminalRoster = entries;
  reconcileLocalGroupsFromRoster(entries);
}

/// Align each local tab's broadcast `group` with the server's roster truth.
/// The server can move a live session's `tab_group` without the SPA driving
/// it: a CLI / team-script bootstrap restarts the lead's pre-existing terminal
/// out of band (a shell cannot restart the tab running its own script, so the
/// server does it), which updates the session's server-side group but leaves
/// this window's `tab.group` at its old value. Broadcast scoping keys on the
/// local group, so without this the lead would sit alone in a stale group
/// while the freshly-spawned workers share the team group.
///
/// Reconciling here is safe against an unsaved UI group edit: that edit lives
/// in the component-local `groupDraft` and only reaches `tab.group` at a
/// confirmed restart (which respawns the session, so the roster already agrees
/// by the time it lands). A local/server mismatch therefore means the server
/// moved an existing session, and the server is authoritative.
function reconcileLocalGroupsFromRoster(entries: TerminalRosterEntry[]): void {
  for (const entry of entries) {
    const tab = findTerminalBySession(entry.id);
    if (tab && terminalTabGroup(tab) !== entry.tab_group) {
      setTerminalGroup(tab, entry.tab_group);
    }
  }
}

/// Same-group sessions in OTHER windows of this tenant. The broadcast menu
/// lists these read-only (cross-window broadcast is group-level by nature -
/// visible but not individually selectable) and the indicator counts the
/// ones that have opted in. Sessions with no `window_id` are skipped: they
/// cannot be attributed to a window, and this window's own sessions are
/// excluded (already shown as local tabs).
export function crossWindowBroadcastMembers(tab: TerminalTab): TerminalRosterEntry[] {
  const group = terminalTabGroup(tab);
  const myWindow = sessionWindowId();
  return terminalRoster.filter(
    (e) => e.tab_group === group && e.window_id != null && e.window_id !== myWindow,
  );
}

/// How many OTHER terminals this terminal's broadcast input reaches: local
/// same-window targets plus same-group cross-window members that have opted
/// in (broadcast on). Mirrors the server's cross-window fan gate, which only
/// delivers to members with their own broadcast toggle on. Drives the
/// broadcast indicator's count.
export function terminalBroadcastReachCount(tab: TerminalTab): number {
  const local = tab.broadcastTargetIds.length;
  const cross = crossWindowBroadcastMembers(tab).filter((e) => e.broadcast).length;
  return local + cross;
}

export function setTerminalSession(tab: TerminalTab, sessionId: string): void {
  const wasFresh = !tab.terminalSessionId || tab.terminalSessionId !== sessionId;
  tab.terminalSessionId = sessionId;
  if (wasFresh) {
    tab.lastAgentEchoSeq = undefined;
    tab.terminalEnvTabName = terminalTabName(tab);
    tab.terminalEnvNamePromptDismissed = false;
  }
}

export function setTerminalActivity(tab: TerminalTab, active: boolean): void {
  tab.terminalActivity = active || undefined;
  // Seeing the terminal (active=false) clears the pulse too: the dot is
  // gone entirely, not left mid-pulse.
  if (!active) tab.terminalActivityPulsing = undefined;
}

/// Drive the unseen-output dot's pulse. True while output is actively
/// arriving; false once it stops (the dot goes solid while unseen).
export function setTerminalActivityPulsing(tab: TerminalTab, pulsing: boolean): void {
  tab.terminalActivityPulsing = pulsing || undefined;
}

/// MESSAGE depth of this terminal's server-side write queue. 0 collapses to
/// undefined so badges/labels render on truthiness like terminalActivity.
export function setTerminalQueueDepth(tab: TerminalTab, depth: number): void {
  tab.queueDepth = depth > 0 ? depth : undefined;
}

/// Start tracking an in-flight Rich Prompt message: phase "sent" (the
/// `prompt` frame went out on an open socket; no ack yet).
export function beginPendingPrompt(tab: TerminalTab, id: string): void {
  tab.pendingPrompt = { id, phase: "sent" };
}

/// Resolve the in-flight prompt by id. Stale/foreign ids no-op: every
/// attached socket sees every `prompt-delivered`, so a second window (or a
/// reattach replay) must not flip a pending message it does not own.
export function resolvePendingPrompt(
  tab: TerminalTab,
  id: string,
  phase: "queued" | "delivered" | "rejected",
  depth?: number,
): void {
  const pending = tab.pendingPrompt;
  if (!pending || pending.id !== id) return;
  tab.pendingPrompt = { ...pending, phase, ...(depth !== undefined ? { depth } : {}) };
}

/// Resolve a `prompt-cancelled` ack (the recall round-trip). `removed:true`
/// means the still-queued message was pulled from the queue before it hit the
/// PTY → phase "recalled" (the bubble unlocks + keeps the draft text so the
/// user edits and resubmits with a fresh id, no double-delivery). `removed:
/// false` means it raced a drain and already delivered → phase "drained" (the
/// bubble surfaces "already sent" rather than letting the user silently
/// re-edit a delivered message). Stale/foreign ids no-op (same guard as
/// `resolvePendingPrompt` — every attached socket sees acks it doesn't own).
export function resolvePromptCancelled(tab: TerminalTab, id: string, removed: boolean): void {
  const pending = tab.pendingPrompt;
  if (!pending || pending.id !== id) return;
  tab.pendingPrompt = { ...pending, phase: removed ? "recalled" : "drained" };
}

/// On (re)attach, re-prove a RESTORED pending Rich Prompt message against the
/// server's authoritative `queued_prompt_ids` (FIFO order) so a queued message
/// survives a window reload (GAP 2 / the reload contract). If the restored id
/// is still in the queue, re-lock + re-show it with its position (index + 1);
/// if it's gone (drained/delivered before the reload) clear it so the editor
/// unlocks. Only acts on a restored in-flight phase ("queued"/"sent"); a
/// terminal phase is left for the bubble's own resolution. Mutates outside any
/// `$derived` (caller is the WS `session`-frame handler).
export function reproveRestoredPrompt(tab: TerminalTab, queuedIds: string[]): void {
  const pending = tab.pendingPrompt;
  if (!pending) return;
  // Only re-prove a "queued" message (the persisted/acked state). A live "sent"
  // (just submitted, pre-ack) is the ack flow's to resolve — a reattach session
  // frame must not race-clear it before its prompt-ack arrives.
  if (pending.phase !== "queued") return;
  const idx = queuedIds.indexOf(pending.id);
  if (idx >= 0) {
    tab.pendingPrompt = { ...pending, phase: "queued", depth: idx + 1 };
  } else {
    tab.pendingPrompt = undefined;
  }
}

/// Fail the in-flight prompt unconditionally (WS close / session end / ack
/// timeout — paths with no message id in hand). The bubble unlocks, keeps
/// the text, and labels honestly: the message may still be queued
/// server-side, but this client can no longer observe its delivery.
export function failPendingPrompt(tab: TerminalTab): void {
  const pending = tab.pendingPrompt;
  if (!pending) return;
  tab.pendingPrompt = { ...pending, phase: "failed" };
}

export function clearTerminalSession(tab: TerminalTab): void {
  tab.terminalSessionId = undefined;
  tab.lastAgentEchoSeq = undefined;
  tab.terminalActivity = undefined;
  tab.terminalActivityPulsing = undefined;
  tab.terminalEnvTabName = undefined;
  tab.terminalEnvNamePromptDismissed = false;
}

export function allTerminalTabs(): TerminalTab[] {
  return terminalTabsIn(layout);
}

/// Find a TerminalTab by its chan-server session id. Used by the team
/// orchestrator to locate the tab and populate its team-work buffer.
/// Returns null when no matching tab is open; the orchestrator silently
/// skips the lead-prompt step in that case.
export function findTerminalBySession(sessionId: string): TerminalTab | null {
  if (!sessionId) return null;
  for (const tab of allTerminalTabs()) {
    if (tab.terminalSessionId === sessionId) return tab;
  }
  return null;
}

export function hasGraphTab(): boolean {
  return Object.values(layout.nodes).some(
    (node) => node.kind === "leaf" && node.tabs.some((tab) => tab.kind === "graph"),
  );
}

export function hasBrowserTab(): boolean {
  return Object.values(layout.nodes).some(
    (node) => node.kind === "leaf" && node.tabs.some((tab) => tab.kind === "browser"),
  );
}

type TerminalInputSink = (data: string) => void;
const terminalInputSinks = new Map<string, TerminalInputSink>();
type TerminalCloseSink = () => boolean | void | Promise<boolean | void>;
const terminalCloseSinks = new Map<string, TerminalCloseSink>();

/// Tab ids that are LEAVING this window via a session-preserving cross-window
/// move. When `closeTab` tears such a tab down, the terminal close-sink
/// (`closeTerminalForTab` in TerminalTab.svelte) consults this set and SKIPS
/// the WS `close` frame, so the PTY stays alive in the shared `/terminal`
/// registry for the target window to re-attach to. Window-local cleanup
/// (Rich Prompt draft, bubble entry) still runs - the tab really is gone from
/// THIS window. The set carries this state because the close-sink takes no args; it
/// is drained on consult (see `isTerminalMoving`) so a normal later close of a
/// re-created tab with a colliding id still kills its PTY.
const terminalsMovingOut = new Set<string>();

/// Mark a terminal tab as moving out (PTY kept alive) for the duration of the
/// drag-end close. Called by the source pane's drag-end before `closeTab`.
export function markTerminalMovingOut(tabId: string): void {
  terminalsMovingOut.add(tabId);
}

/// Whether `tabId` is leaving via a session-preserving move; consumes the
/// flag (one-shot) so it can't leak into a later real close. The terminal
/// close-sink calls this to decide whether to send the WS `close` frame.
export function isTerminalMoving(tabId: string): boolean {
  return terminalsMovingOut.delete(tabId);
}

/// Records whether the most recent terminal-tab close was a session-preserving
/// cross-window MOVE (vs a real close). `closeTab` sets it just before it
/// removes the tab — so the empty-window discard guard, which fires reactively
/// right after, reads the LAST close's intent deterministically (set before the
/// mutation, no effect-vs-teardown ordering race). A non-move close clears it.
let lastTerminalCloseWasMoveOut = false;

/// One-shot read for the window-discard guard: was the close that just emptied
/// this window a terminal move-out? If so the source's discard must DELETE its
/// blob but NOT reap — the moved PTY lives on, re-bound to the target window.
export function consumeLastCloseWasMoveOut(): boolean {
  const v = lastTerminalCloseWasMoveOut;
  lastTerminalCloseWasMoveOut = false;
  return v;
}

export function registerTerminalInputSink(tabId: string, sink: TerminalInputSink): () => void {
  terminalInputSinks.set(tabId, sink);
  return () => {
    if (terminalInputSinks.get(tabId) === sink) terminalInputSinks.delete(tabId);
  };
}

// Rich Prompt sender. Mirrors the input-sink registry but feeds the terminal
// WS `prompt` frame (NOT the raw `input` keystroke path): each TerminalTab
// registers a sink that enqueues into THIS session's server-side write queue,
// where it shares one FIFO with `cs terminal write` and auto-submits in order.
// Keeping it a separate registry lets the Rich Prompt bubble reach the active
// terminal without touching TerminalTab internals. The sink returns whether the
// frame actually went out (the WS was open) so callers can retry a freshly-
// spawned terminal whose socket has not connected yet (the team lead bootstrap).
type TerminalPromptSink = (data: string, agent?: string, id?: string) => boolean;
const terminalPromptSinks = new Map<string, TerminalPromptSink>();

export function registerTerminalPromptSink(
  tabId: string,
  sink: TerminalPromptSink,
): () => void {
  terminalPromptSinks.set(tabId, sink);
  return () => {
    if (terminalPromptSinks.get(tabId) === sink) terminalPromptSinks.delete(tabId);
  };
}

/// Send a prompt to a SPECIFIC terminal's write queue via its WS `prompt`
/// frame. Returns false when that terminal has no live prompt sink OR its WS
/// is not open yet (so the caller can retry). The team orchestrator uses this
/// to auto-deliver the lead's identity prompt to the freshly-spawned lead
/// terminal once its socket connects (the lead is a normal terminal now - no
/// bubble - so its identity arrives through the same queue as every prompt).
/// `id` tags the message for queue-visibility tracking (prompt-ack /
/// prompt-delivered frames). Omitted = legacy fire-and-forget — the team
/// orchestrator's lead-identity prompt stays untagged on purpose.
export function sendPromptToTerminal(
  tabId: string,
  data: string,
  agent?: string,
  id?: string,
): boolean {
  const sink = terminalPromptSinks.get(tabId);
  if (!sink) return false;
  return sink(data, agent, id);
}

/// Cancel/recall sink: send a `cancel-prompt` frame on a SPECIFIC terminal's
/// WS so the server removes a still-queued message by its `prompt_id`. Mirrors
/// the prompt sink so RichPrompt can reach the terminal's socket without owning
/// it. Returns false when the terminal has no live sink / its WS is closed.
type TerminalCancelSink = (id: string) => boolean;
const terminalCancelSinks = new Map<string, TerminalCancelSink>();

export function registerTerminalCancelSink(
  tabId: string,
  sink: TerminalCancelSink,
): () => void {
  terminalCancelSinks.set(tabId, sink);
  return () => {
    if (terminalCancelSinks.get(tabId) === sink) terminalCancelSinks.delete(tabId);
  };
}

export function sendCancelToTerminal(tabId: string, id: string): boolean {
  const sink = terminalCancelSinks.get(tabId);
  if (!sink) return false;
  return sink(id);
}

export function registerTerminalCloseSink(tabId: string, sink: TerminalCloseSink): () => void {
  terminalCloseSinks.set(tabId, sink);
  return () => {
    if (terminalCloseSinks.get(tabId) === sink) terminalCloseSinks.delete(tabId);
  };
}

async function runTerminalCloseSink(tab: TerminalTab): Promise<boolean> {
  const sink = terminalCloseSinks.get(tab.id);
  if (!sink) return true;
  const result = await sink();
  return result !== false;
}

async function runTerminalCloseSinks(tabs: Tab[]): Promise<boolean> {
  for (const tab of tabs) {
    if (tab.kind === "terminal" && !(await runTerminalCloseSink(tab))) {
      return false;
    }
  }
  return true;
}

/// Broadcast input is deliberately window-scoped. The target ids in
/// `broadcastTargetIds` are resolved only through this JS window's
/// `layout` registry (`allTerminalTabs()`), even though terminal
/// session data is persisted per `w=<window-label>` and multiple
/// windows can share a chan-server. A sink whose id is not present
/// in the current layout is skipped silently; do not fan out by sink
/// id alone or via a server-side bus without preserving this boundary.
export function broadcastTerminalInput(sourceTab: TerminalTab, data: string): void {
  if (!sourceTab.broadcastEnabled) return;
  // `terminalBroadcastMemberIds` already scopes to the source's group;
  // the group re-check below is defensive (a member could change group
  // between resolution and fan-out, though group change requires restart).
  const targets = new Set(terminalBroadcastMemberIds(sourceTab));
  if (targets.size === 0) return;
  const group = terminalTabGroup(sourceTab);
  for (const tab of allTerminalTabs()) {
    if (tab.id === sourceTab.id || !targets.has(tab.id)) continue;
    if (terminalTabGroup(tab) !== group) continue;
    terminalInputSinks.get(tab.id)?.(data);
  }
}

type CloseTabsOptions = {
  force?: boolean;
};

type DraftCloseDecision =
  | { action: "cancel" }
  | { action: "discard" }
  | { action: "save"; target: string };
type DraftPromotionSink = (path: string) => void | Promise<void>;

const draftPromotionSinks = new Set<DraftPromotionSink>();

export function registerDraftPromotionSink(
  sink: DraftPromotionSink,
): () => void {
  draftPromotionSinks.add(sink);
  return () => {
    draftPromotionSinks.delete(sink);
  };
}

function notifyDraftPromoted(path: string): void {
  for (const sink of draftPromotionSinks) {
    void sink(path);
  }
}

// The draft-CLOSE modal (close a draft tab: name a destination + Save,
// or Discard). This modal is the close path only; the explicit
// "Save to Workspace" action uses `saveDraftTabToWorkspace` via
// PathPromptModal instead.
export const draftCloseState = $state<{
  open: boolean;
  path: string;
  name: string;
  target: string;
  targetKind: "file" | "folder";
  hasAttachments: boolean;
  error: string | null;
  resolve: ((value: DraftCloseDecision) => void) | null;
}>({
  open: false,
  path: "",
  name: "",
  target: "",
  targetKind: "file",
  hasAttachments: false,
  error: null,
  resolve: null,
});

function uiDraftClose(opts: {
  path: string;
  name: string;
  target: string;
  targetKind: "file" | "folder";
  hasAttachments: boolean;
}): Promise<DraftCloseDecision> {
  return new Promise((resolve) => {
    draftCloseState.resolve?.({ action: "cancel" });
    draftCloseState.path = opts.path;
    draftCloseState.name = opts.name;
    draftCloseState.target = opts.target;
    draftCloseState.targetKind = opts.targetKind;
    draftCloseState.hasAttachments = opts.hasAttachments;
    draftCloseState.error = null;
    draftCloseState.resolve = resolve;
    draftCloseState.open = true;
  });
}

export function resolveDraftClose(action: "cancel" | "discard" | "save"): void {
  const r = draftCloseState.resolve;
  if (!r) return;
  const target = draftCloseState.target.trim();
  if (action === "save" && target.length === 0) {
    draftCloseState.error = "Choose a destination path";
    return;
  }
  draftCloseState.resolve = null;
  draftCloseState.open = false;
  draftCloseState.error = null;
  if (action === "save") {
    r({ action: "save", target });
  } else {
    r({ action });
  }
}

function isLiveTerminal(t: Tab): boolean {
  return t.kind === "terminal" && terminalInputSinks.has(t.id);
}

function isDraftTab(t: Tab): t is FileTab {
  return t.kind === "file" && isDraftPath(t.path);
}

function draftDefaultTarget(info: { name: string; has_attachments: boolean }): string {
  return info.has_attachments ? info.name : `${info.name}.md`;
}

function promotedEditorPath(promoted: DraftPromoteResponse): string {
  if (promoted.mode === "file") return promoted.path;
  const dir = promoted.path.replace(/\/+$/, "");
  return dir ? `${dir}/draft.md` : "draft.md";
}

async function reloadPromotedDraftTab(tab: FileTab, path: string): Promise<void> {
  const found = findFileTabById(tab.id);
  if (!found) return;
  const pathKind = classifyPath(path);
  found.tab.fileKind =
    pathKind === "document" || pathKind === "text" ? pathKind : "document";
  found.tab.path = path;
  found.tab.mode = defaultModeForPath(path, found.tab.fileKind);
  found.tab.loading = true;
  found.tab.error = null;
  found.tab.fileMissing = null;
  found.tab.repoRoot = null;
  found.tab.fsWritable = true;
  await loadTabContent(found.paneId, found.tab.id, path);
}

function closeRisk(t: Tab): "live-terminal" | null {
  if (isLiveTerminal(t)) return "live-terminal";
  return null;
}

async function confirmCloseTabs(
  tabs: Tab[],
  opts?: CloseTabsOptions,
): Promise<boolean> {
  if (opts?.force) return true;
  if (tabs.some(isDraftTab)) {
    notify("Close Drafts individually to save or discard them");
    return false;
  }
  for (const tab of tabs) {
    if (tab.kind !== "file") continue;
    if (!isDirty(tab)) continue;
    try {
      await performSave(tab);
    } catch (e) {
      tab.error = `save failed: ${(e as Error).message}`;
      return false;
    }
    if (isDirty(tab)) return false;
  }
  const risky = tabs.filter((t) => closeRisk(t) !== null);
  if (risky.length === 0) return true;
  const terminals = risky.filter((t) => closeRisk(t) === "live-terminal");
  const parts: string[] = [];
  if (terminals.length > 0) {
    const label =
      terminals.length === 1
        ? terminalTabName(terminals[0] as TerminalTab)
        : `${terminals.length} live terminals`;
    parts.push(`${label} is still running`);
  }
  return uiConfirm({
    title: "Close tab?",
    message: `${parts.join(" and ")}. Close anyway?`,
    confirmLabel: "Close",
    destructive: false,
  });
}

/// Fetch a file tab's content from disk and write it into the
/// (proxied) pane state. Resolves the proxied reference each time it
/// touches the tab: in Svelte 5, mutations through the original
/// object literal don't propagate to the array element.
const tabLoadVersions = new Map<string, number>();
const tabLoadControllers = new Map<string, AbortController>();

async function loadTabContent(
  paneId: string,
  tabId: string,
  path: string,
): Promise<void> {
  const loadVersion = (tabLoadVersions.get(tabId) ?? 0) + 1;
  tabLoadVersions.set(tabId, loadVersion);
  tabLoadControllers.get(tabId)?.abort();
  const controller = new AbortController();
  tabLoadControllers.set(tabId, controller);
  const live = (): FileTab | undefined => {
    if (tabLoadVersions.get(tabId) !== loadVersion) return undefined;
    const node = layout.nodes[paneId];
    if (!node || node.kind !== "leaf") return undefined;
    const t = node.tabs.find((tab) => tab.id === tabId);
    return t && t.kind === "file" ? t : undefined;
  };
  try {
    const start = live();
    if (start) {
      start.content = "";
      start.saved = "";
      start.savedMtime = null;
      start.savedMtimeNs = null;
      start.loading = true;
      start.loadProgress = { loadedBytes: 0, totalBytes: null };
      start.error = null;
      start.fileMissing = null;
    }
    const r = await api.readStream(path, {
      signal: controller.signal,
      onMeta(meta) {
        const t = live();
        if (!t) {
          controller.abort();
          return;
        }
        t.savedMtime = meta.mtime ?? null;
        t.savedMtimeNs = meta.mtime_ns ?? null;
        t.repoRoot = meta.repo_root ?? null;
        t.fsWritable = meta.writable ?? true;
        t.loadProgress = {
          loadedBytes: 0,
          totalBytes: meta.size ?? null,
        };
      },
      onChunk(chunk, progress) {
        const t = live();
        if (!t) {
          controller.abort();
          return;
        }
        t.content += chunk;
        t.loadProgress = progress;
      },
    });
    const t = live();
    if (t) {
      t.content = r.content;
      t.saved = r.content;
      t.savedMtime = r.mtime ?? null;
      t.savedMtimeNs = r.mtime_ns ?? null;
      t.repoRoot = r.repo_root ?? null;
      t.error = null;
      t.fileMissing = null;
      // Older servers omit `writable`; treat absent as writable so
      // the lamp behaves the way it did before this field existed.
      t.fsWritable = r.writable ?? true;
      // The buffer now matches disk; clear any pending external-change
      // banner (this load IS the reload the user opted into, or a
      // user-initiated replace).
      t.externalChange = false;
    }
  } catch (e) {
    if (controller.signal.aborted && tabLoadVersions.get(tabId) !== loadVersion) {
      return;
    }
    const t = live();
    if (t) {
      if (isMissingFileError(e)) {
        markFileMissing(t);
      } else {
        t.error = (e as Error).message;
        t.fileMissing = null;
        t.saved = t.content;
      }
    }
  } finally {
    if (tabLoadVersions.get(tabId) === loadVersion) {
      tabLoadControllers.delete(tabId);
      const t = live();
      if (t) {
        t.loading = false;
        t.loadProgress = undefined;
      }
    }
  }
}

/// Peek whether `path` opens as text without downloading it whole. Reuses the
/// server's content gate (`read_text_with_stat`, shared with `cs open`): the
/// stream read emits its meta for a plaintext file — we abort right after — and
/// fails with a 415 for a binary one. "error" (a real read failure, e.g. a
/// missing file) lets the caller fall through to a normal open so the editor tab
/// surfaces the actual cause.
async function probeOpenableAsText(
  path: string,
): Promise<"openable" | "binary" | "error"> {
  const controller = new AbortController();
  try {
    await api.readStream(path, {
      signal: controller.signal,
      onMeta() {
        // The server accepted it as text; stop the download here — the real
        // load re-reads it into the tab.
        controller.abort();
      },
    });
    return "openable"; // a small file finished before the abort landed
  } catch (e) {
    if (controller.signal.aborted) return "openable"; // aborted after meta = text
    if (e instanceof ApiError && e.status === 415) return "binary";
    return "error";
  }
}

/// Open a file in a specific pane. If already open there, just focus.
export async function openInPane(
  paneId: string,
  path: string,
  opts: OpenFileOptions = {},
): Promise<void> {
  // The extension may not be editable, but the file can still be plaintext (an
  // odd suffix, no extension). Peek the content and let the server's gate
  // decide — matching `cs open`. Editable-by-extension files skip the peek.
  // A binary file is refused (it stays view-only in the browser/inspector); a
  // real read error falls through to a normal open so the tab shows the cause.
  if (!isEditableText(path) && (await probeOpenableAsText(path)) === "binary") {
    notify(`'${path}' is not a text file`);
    return;
  }
  const p = pane(paneId);
  const pendingReopen =
    pendingMissingFileReopenTabId === null
      ? undefined
      : p.tabs.find(
          (t): t is FileTab =>
            t.kind === "file" &&
            t.id === pendingMissingFileReopenTabId &&
            t.fileMissing !== null,
        );
  if (pendingReopen) {
    pendingMissingFileReopenTabId = null;
    const pathKind = classifyPath(path);
    // A non-extension-editable file that passed the content peek is source-like
    // (an odd suffix, not markdown), so it opens in source mode, not wysiwyg.
    pendingReopen.fileKind =
      pathKind === "document" || pathKind === "text" ? pathKind : "text";
    pendingReopen.path = path;
    pendingReopen.content = "";
    pendingReopen.saved = "";
    pendingReopen.savedMtime = null;
    pendingReopen.savedMtimeNs = null;
    pendingReopen.mode = defaultModeForPath(path, pendingReopen.fileKind);
    pendingReopen.loading = true;
    pendingReopen.error = null;
    pendingReopen.fileMissing = null;
    pendingReopen.repoRoot = null;
    pendingReopen.fsWritable = true;
    if (opts.initialSelection) pendingReopen.caret = { ...opts.initialSelection };
    p.activeTabId = pendingReopen.id;
    layout.activePaneId = paneId;
    await loadTabContent(paneId, pendingReopen.id, path);
    return;
  }
  const existing = p.tabs.find(
    (t): t is FileTab => t.kind === "file" && t.path === path,
  );
  if (existing) {
    if (opts.initialSelection) existing.caret = { ...opts.initialSelection };
    p.activeTabId = existing.id;
    layout.activePaneId = paneId;
    return;
  }
  // Path-based classification picks the initial mode: markdown-class
  // files start in wysiwyg (the wisp of formatting they carry is
  // worth rendering); arbitrary source / config text starts in source
  // mode (wysiwyg would just render the raw bytes with no visible
  // benefit, plus the menu hides the toggle for text-kind tabs).
  const pathKind = classifyPath(path);
  // A non-extension-editable file that passed the content peek is source-like
  // (an odd suffix, not markdown), so it opens in source mode, not wysiwyg.
  const fileKind: FileKind =
    pathKind === "document" || pathKind === "text" ? pathKind : "text";
  const newTab: FileTab = {
    kind: "file",
    fileKind,
    id: id("tab"),
    path,
    content: "",
    saved: "",
    savedMtime: null,
    savedMtimeNs: null,
    mode: defaultModeForPath(path, fileKind),
    loading: true,
    error: null,
    fileMissing: null,
    inspectorOpen: false,
    outlineOpen: false,
    repoRoot: null,
    readMode: false,
    fsWritable: true,
    styleToolbarOpen: false,
    syntaxHighlight: true,
    highlightTrailingWhitespace: false,
    codeBlocksCollapsed: false,
  };
  if (opts.initialSelection) newTab.caret = { ...opts.initialSelection };
  p.tabs.push(newTab);
  p.activeTabId = newTab.id;
  layout.activePaneId = paneId;
  await loadTabContent(paneId, newTab.id, path);
}

export function openInActivePane(
  path: string,
  opts: OpenFileOptions = {},
): Promise<void> {
  return openInPane(layout.activePaneId, path, opts);
}

/// Open a wiki / markdown-link target, resolving an extension-less stem
/// to the real on-disk file before opening. A `[[note]]` pill (and a
/// `[[` picker pick) carries the raw stem `note`; the pill's kind probe
/// resolves it through `/api/resolve-link` (which tries `note.md` /
/// `note.txt` / `note`), so the pill renders as a valid link. But the
/// click previously handed that raw stem straight to `openInActivePane`,
/// and the file read route opens the path verbatim (no extension probe)
/// — so it 404'd and the tab flashed a false "document not found" for a
/// file that's right there on disk. Resolve through the SAME probe here
/// so the click opens `note.md`. A failed resolve falls back to the raw
/// target so a genuinely broken link still lands on the missing-file
/// banner with the real cause rather than swallowing the click.
export async function openLinkTarget(
  target: string,
  opts: OpenFileOptions = {},
): Promise<void> {
  let path = target;
  try {
    path = (await api.resolveLink(target)).path;
  } catch {
    // Unresolvable (broken link / network): open the raw target so the
    // editor surfaces the missing file instead of silently no-op'ing.
  }
  await openInActivePane(path, opts);
}

/// Move the active pane's selection to the previous tab. Wraps from
/// the first tab back to the last (iTerm-style cycle), so repeated
/// presses keep rotating instead of dead-ending at the edges. No-op
/// when the pane is empty or the active tab is somehow not in the
/// tab list (shouldn't happen but keeps a bad state from crashing).
///
/// Chord-driven tab switches need to also move keyboard focus to the
/// new active surface; otherwise the next keystroke lands in the prior
/// tab. Mouse-click tab switch already works because terminal tabs have
/// a `$effect` that fires on the `focused` prop flip, but some tab
/// kinds don't have an equivalent path and the chord-fired switch
/// leaves the previously-focused contenteditable holding OS focus until
/// something explicitly takes it.
///
/// Mechanism: a global $state counter bumped here. Tab-kind components
/// subscribe via $effect; when the pulse increments AND the tab is
/// focused, the component re-runs its focus routine in a microtask.
export const tabFocusPulse = $state({ value: 0 });
export function bumpTabFocusPulse(): void {
  tabFocusPulse.value += 1;
  // Blur the currently-focused element after bumping. The chord keydown
  // was synchronously dispatched while the prior tab's input had DOM
  // focus; even if the active tab changes, the prior element retains
  // `document.activeElement` until something explicitly takes focus.
  // Blurring parks focus on `<body>` so the new tab's pulse-triggered
  // focus call can land cleanly without racing the contenteditable.
  // SSR-safe; skips `<body>` so we don't blur the default focus owner.
  if (typeof document === "undefined") return;
  const el = document.activeElement;
  if (el instanceof HTMLElement && el !== document.body) {
    el.blur();
  }
}

export function selectPrevTabInActivePane(): void {
  const p = activePane();
  if (p.tabs.length === 0 || p.activeTabId === null) return;
  const idx = p.tabs.findIndex((t) => t.id === p.activeTabId);
  if (idx < 0) return;
  const next = (idx - 1 + p.tabs.length) % p.tabs.length;
  p.activeTabId = p.tabs[next].id;
  bumpTabFocusPulse();
}

export function selectNextTabInActivePane(): void {
  const p = activePane();
  if (p.tabs.length === 0 || p.activeTabId === null) return;
  const idx = p.tabs.findIndex((t) => t.id === p.activeTabId);
  if (idx < 0) return;
  const next = (idx + 1) % p.tabs.length;
  p.activeTabId = p.tabs[next].id;
  bumpTabFocusPulse();
}

/// Select the Nth tab in the active pane (0-indexed). Silent no-op
/// when the index is out of range, matching the browser behavior of
/// Cmd+9 jumping to the last tab only when nine or more exist.
export function selectTabAtIndexInActivePane(index: number): void {
  const p = activePane();
  if (index < 0 || index >= p.tabs.length) return;
  p.activeTabId = p.tabs[index].id;
  bumpTabFocusPulse();
}

function leafIdsInOrder(
  nodeId: string,
  out: string[] = [],
  state: LayoutState = activeLayout(),
): string[] {
  const n = state.nodes[nodeId];
  if (!n) return out;
  if (n.kind === "leaf") {
    out.push(n.id);
    return out;
  }
  leafIdsInOrder(n.a, out, state);
  leafIdsInOrder(n.b, out, state);
  return out;
}

export function selectPrevPane(): void {
  const current = activeLayout();
  const panes = leafIdsInOrder(current.rootId, [], current);
  if (panes.length < 2) return;
  const idx = panes.indexOf(current.activePaneId);
  if (idx < 0) return;
  current.activePaneId = panes[(idx - 1 + panes.length) % panes.length]!;
}

export function selectNextPane(): void {
  const current = activeLayout();
  const panes = leafIdsInOrder(current.rootId, [], current);
  if (panes.length < 2) return;
  const idx = panes.indexOf(current.activePaneId);
  if (idx < 0) return;
  current.activePaneId = panes[(idx + 1) % panes.length]!;
}

export function focusColorForWindow(): FocusColor {
  return layout.focusColor ?? "blue";
}

export function setWindowFocusColor(color: FocusColor): void {
  layout.focusColor = color;
  if (paneMode.draft) paneMode.draft.focusColor = color;
}

export function closeTab(
  paneId: string,
  tabId: string,
  opts?: CloseTabsOptions,
): Promise<void> {
  return closeTabAsync(paneId, tabId, opts);
}

async function closeTabAsync(
  paneId: string,
  tabId: string,
  opts?: CloseTabsOptions,
): Promise<void> {
  const p = pane(paneId);
  const idx = p.tabs.findIndex((t) => t.id === tabId);
  if (idx < 0) return;
  const tab = p.tabs[idx];
  // Capture move-out intent NOW, before the terminal close-sink below consumes
  // `terminalsMovingOut`. A cross-window MOVE marks the tab moving-out
  // (Pane.svelte drag-end) right before this call.
  const movingOut = tab.kind === "terminal" && terminalsMovingOut.has(tabId);
  if (isDraftTab(tab) && !opts?.force) {
    if (!(await handleDraftTabClose(tab))) return;
  } else if (!(await confirmCloseTabs([tab], opts))) {
    return;
  }
  if (tab.kind === "terminal") {
    if (!(await runTerminalCloseSink(tab))) return;
  }
  // Record the close's move-out intent for the empty-window discard guard. Set
  // unconditionally (false for a real close) so a prior move-out can't leak into
  // a later genuine discard, and set right before the splice so the reactive
  // empty-window `$effect` reads it deterministically.
  lastTerminalCloseWasMoveOut = movingOut;
  rememberClosedTab(paneId, tab);
  p.tabs.splice(idx, 1);
  if (p.activeTabId === tabId) {
    p.activeTabId = p.tabs[Math.max(0, idx - 1)]?.id ?? null;
  }
  // Flip is strictly for panes with >= 1 tab; closing the last tab
  // drops any flip so the pane lands on its empty front (welcome)
  // instead of a stuck back-config surface the chord cannot undo.
  if (p.tabs.length === 0) p.showingBack = false;
  // Do NOT auto-collapse an empty Hybrid pane. Closing the last tab
  // should leave the pane in place rendering the empty landing so the
  // Hybrid structure survives a transient empty state. Use the explicit
  // `closePane` action to dismiss the pane.
}

/// Remove a terminal tab whose session was EXPLICITLY closed server-side
/// (the user / another window / `cs terminal close` deleted it — the
/// `closed{reason:"explicit"}` frame in TerminalTab.svelte). Unlike
/// `closeTab`, this skips the confirm prompt and the WS close sink: the
/// session is already gone, so there is nothing to confirm or to tell the
/// server. Drops the dead tab from its pane with the same active-tab /
/// empty-pane bookkeeping as `closeTab` (no pane auto-collapse, not added to
/// the reopen-closed list — the session can't be reattached). Under Option A
/// a terminal-only window is ephemeral, so once the dead tab is gone the
/// debounced session save deletes the window's blob if no durable content
/// remains. No-op if the tab is no longer in the layout.
export function removeExplicitlyClosedTerminalTab(tabId: string): void {
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    const idx = node.tabs.findIndex((t) => t.id === tabId);
    if (idx < 0) continue;
    node.tabs.splice(idx, 1);
    if (node.activeTabId === tabId) {
      node.activeTabId = node.tabs[Math.max(0, idx - 1)]?.id ?? null;
    }
    if (node.tabs.length === 0) node.showingBack = false;
    return;
  }
}

/// Server-side seed for a brand-new draft (see
/// `crates/chan-server/src/routes/drafts.rs::NEW_DRAFT_CONTENT`).
/// A draft whose buffer matches this string and is clean against disk
/// has not been edited since creation. Treat it as empty so the close
/// path skips the "Close Draft" modal for pristine drafts.
const NEW_DRAFT_SEED = "# Draft\n";

async function handleDraftTabClose(tab: FileTab): Promise<boolean> {
  // A draft whose backing file vanished (the user moved or deleted it
  // on disk) has nothing to inspect, save, or discard. Close it like
  // any missing-file tab instead of 404-ing on inspectDraft and
  // trapping the tab open (no Cmd+W / Ctrl+D / X would dismiss it).
  if (tab.fileMissing) return true;
  try {
    const contentIsEmpty = tab.content.trim().length === 0;
    const isPristineSeed = !isDirty(tab) && tab.content === NEW_DRAFT_SEED;
    if (!contentIsEmpty && isDirty(tab)) {
      await performSave(tab);
      if (isDirty(tab)) return false;
    }
    const info = await api.inspectDraft(tab.path);
    if ((contentIsEmpty || isPristineSeed) && !info.has_attachments) {
      await api.discardDraft(tab.path);
      notify("Draft discarded");
      return true;
    }
    const decision = await uiDraftClose({
      path: tab.path,
      name: info.name,
      target: draftDefaultTarget(info),
      targetKind: info.has_attachments ? "folder" : "file",
      hasAttachments: info.has_attachments,
    });
    if (decision.action === "cancel") return false;
    if (decision.action === "discard") {
      await api.discardDraft(tab.path);
      notify("Draft discarded");
      return true;
    }
    if (isDirty(tab)) {
      await performSave(tab);
      if (isDirty(tab)) return false;
    }
    const promoted = await api.promoteDraft(tab.path, decision.target);
    notifyDraftPromoted(promoted.path);
    notify(`Draft saved to ${promoted.path}`);
    return true;
  } catch (e) {
    tab.error = `draft close failed: ${(e as Error).message}`;
    return false;
  }
}

export async function saveDraftTabToWorkspace(tab: FileTab): Promise<boolean> {
  if (!isDraftTab(tab)) return false;
  try {
    // Lazy import to break the eager cyclic dependency with
    // store.svelte (see the import-site comment at the top of this
    // module). Resolved at user-action time, never at module-eval.
    const { uiPathPrompt } = await import("./store.svelte");
    if (isDirty(tab)) {
      await performSave(tab);
      if (isDirty(tab)) return false;
    }
    const info = await api.inspectDraft(tab.path);
    // The draft Save reuses PathPromptModal (autocomplete, live status
    // row, pre-flight validation). The draft's shape decides the dialog
    // kind, detected server-side via `has_attachments`:
    //   - lone draft.md -> a FILE target (`.md` auto-append + the
    //     editable-text check).
    //   - a draft workspace (user pasted images / opened a terminal /
    //     wrote files in the draft dir) -> a DIRECTORY target (modal's
    //     `folder` Dir-only mode: no `.md` append, trailing `/` allowed)
    //     plus a notice explaining the whole directory is saved.
    const target = info.has_attachments
      ? await uiPathPrompt({
          title: "save draft to workspace (directory)",
          defaultValue: info.name ? `${info.name}/` : "",
          kind: "folder",
          mode: "create",
          notice:
            "This draft has attachments, so the whole draft directory " +
            "is saved as a directory at the path below.",
        })
      : await uiPathPrompt({
          title: "save draft to workspace (.md added if no extension)",
          defaultValue: draftDefaultTarget(info),
          kind: "file",
          mode: "create",
          // Same editable-text gate as `fileOps.createFile`: reject
          // non-editable targets in the dialog so the error surfaces
          // before the close instead of after.
          validate: (path) =>
            isEditableText(path)
              ? null
              : `'${path}' is not an editable text file (only .md and .txt)`,
        });
    if (target === null) return false;
    // The modal resolved and validated the path (`.md` append for the
    // file case, trailing-slash folder for the dir case). `promoteDraft`
    // takes it verbatim; the trailing slash on a directory target is
    // harmless.
    if (isDirty(tab)) {
      await performSave(tab);
      if (isDirty(tab)) return false;
    }
    const promoted = await api.promoteDraft(tab.path, target);
    notifyDraftPromoted(promoted.path);
    await reloadPromotedDraftTab(tab, promotedEditorPath(promoted));
    notify(`Draft saved to ${promoted.path}`);
    return true;
  } catch (e) {
    tab.error = `draft save failed: ${(e as Error).message}`;
    return false;
  }
}

/// Drop every tab in every pane. Pane structure is preserved; only the
/// tabs go. Used by mobile reset flows so the editor stops showing a
/// now-deleted file after the user wipes the workspace.
export async function closeAllTabs(opts?: CloseTabsOptions): Promise<void> {
  const entries = Object.values(layout.nodes).flatMap((node) =>
    node.kind === "leaf" ? node.tabs.map((tab) => ({ paneId: node.id, tab })) : [],
  );
  if (!(await confirmCloseTabs(entries.map((entry) => entry.tab), opts))) return;
  for (const entry of entries) rememberClosedTab(entry.paneId, entry.tab);
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    node.tabs.length = 0;
    node.activeTabId = null;
  }
}

export async function closeOtherTabsInPane(
  paneId: string,
  keepTabId: string,
  opts?: CloseTabsOptions,
): Promise<void> {
  const p = pane(paneId);
  const closing = p.tabs.filter((t) => t.id !== keepTabId);
  if (closing.length === 0) return;
  if (!(await confirmCloseTabs(closing, opts))) return;
  const closeIds = new Set<string>();
  for (const tab of closing) {
    if (tab.kind === "terminal" && !(await runTerminalCloseSink(tab))) continue;
    rememberClosedTab(paneId, tab);
    closeIds.add(tab.id);
  }
  p.tabs = p.tabs.filter((t) => t.id === keepTabId || !closeIds.has(t.id));
  p.activeTabId = p.tabs.find((t) => t.id === keepTabId)?.id ?? p.tabs[0]?.id ?? null;
}

export async function closeTabsInPane(
  paneId: string,
  opts?: CloseTabsOptions,
): Promise<boolean> {
  const p = pane(paneId);
  const closing = [...p.tabs];
  if (!(await confirmCloseTabs(closing, opts))) return false;
  if (!(await runTerminalCloseSinks(closing))) return false;
  for (const tab of closing) {
    rememberClosedTab(paneId, tab);
  }
  p.tabs = [];
  p.activeTabId = null;
  // Empty pane is never flipped.
  p.showingBack = false;
  return true;
}

/// "Close pane" button. Two cases:
///   - non-root: discard all tabs and collapse the pane (sibling takes
///     the parent split's place).
///   - root pane: there must always be at least one pane on screen, so
///     just clear the tabs (returns to the empty "no file open" state).
export async function closePane(
  paneId: string,
  opts?: CloseTabsOptions,
): Promise<boolean> {
  const p = pane(paneId);
  const closing = [...p.tabs];
  if (!(await confirmCloseTabs(closing, opts))) return false;
  if (!(await runTerminalCloseSinks(closing))) return false;
  for (const tab of closing) rememberClosedTab(paneId, tab);
  p.tabs.length = 0;
  p.activeTabId = null;
  if (paneId !== layout.rootId) {
    collapseEmptyPane(paneId);
  }
  return true;
}

/// Reorder a tab within its pane. `toIndex` is the destination index
/// in the post-removal array (so e.g. moving tab 0 to index 2 in a
/// list of 4 tabs lands the tab as the new index 2).
export function reorderTab(paneId: string, tabId: string, toIndex: number): void {
  const p = pane(paneId);
  const from = p.tabs.findIndex((t) => t.id === tabId);
  if (from < 0) return;
  const clamped = Math.max(0, Math.min(toIndex, p.tabs.length - 1));
  if (from === clamped) return;
  // Snapshot the tab before splicing so the proxied entry doesn't get
  // re-wrapped in a way that confuses callers (see moveTab below).
  const src = p.tabs[from]!;
  const moved = cloneTab(src);
  p.tabs.splice(from, 1);
  p.tabs.splice(clamped, 0, moved);
  p.activeTabId = moved.id;
}

/// Plain-data copy of a tab. The deep proxy that wraps `Tab` array
/// elements doesn't survive splice + insert cleanly across panes, so
/// we re-build a fresh object literal.
function cloneTab(src: Tab): Tab {
  if (src.kind === "terminal") {
    return {
      kind: "terminal",
      id: src.id,
      title: src.title,
      createdAt: src.createdAt,
      broadcastEnabled: src.broadcastEnabled,
      broadcastTargetIds: [...src.broadcastTargetIds],
      terminalEnvTabName: src.terminalEnvTabName,
      terminalEnvNamePromptDismissed: src.terminalEnvNamePromptDismissed,
      terminalSessionId: src.terminalSessionId,
      controlledTerminal: src.controlledTerminal,
      lastAgentEchoSeq: src.lastAgentEchoSeq,
      cwd: src.cwd,
      seedInput: src.seedInput,
      pendingGlobalName: src.pendingGlobalName,
    };
  }
  if (src.kind === "graph") {
    return {
      kind: "graph",
      id: src.id,
      title: src.title,
      mode: src.mode,
      scopeId: src.scopeId,
      depth: src.depth,
      expanded: { ...src.expanded },
      filters: { ...src.filters },
      inspectorOpen: src.inspectorOpen,
      pendingSelectId: src.pendingSelectId,
      selectedNodeId: src.selectedNodeId,
      selectedNodeLabel: src.selectedNodeLabel,
    };
  }
  if (src.kind === "browser") {
    return {
      kind: "browser",
      id: src.id,
      title: src.title,
      inspectorOpen: src.inspectorOpen,
      // Carry the per-tab File Browser view state across a clone, the
      // same way the graph branch above carries its own. Without this a
      // split / move / reopen-closed (Cmd+Shift+T) drops the user's
      // expanded directories, selection, scroll, and workspace toggle —
      // the reopened tab snaps back to a collapsed root. Arrays are
      // copied (not aliased) so the clone and source don't share a
      // mutable reference.
      selected: src.selected,
      selectedPaths: src.selectedPaths ? [...src.selectedPaths] : undefined,
      showWorkspace: src.showWorkspace,
      expanded: src.expanded ? [...src.expanded] : undefined,
      scroll: src.scroll,
      inspectorWidth: src.inspectorWidth,
    };
  }
  if (src.kind === "dashboard") {
    return {
      kind: "dashboard",
      id: src.id,
      title: src.title,
      // Preserve the per-tab carousel cursor + slot on/off set across a
      // clone (split / move). Only emit them when set so a default
      // Dashboard tab clones to the same minimal shape as before.
      ...(typeof src.carouselSlide === "number"
        ? { carouselSlide: src.carouselSlide }
        : {}),
      ...(src.disabledSlots && src.disabledSlots.length > 0
        ? { disabledSlots: [...src.disabledSlots] }
        : {}),
      ...(src.autoRotate === false ? { autoRotate: false } : {}),
    };
  }
  return {
    kind: "file",
    fileKind: src.fileKind,
    id: src.id,
    path: src.path,
    content: src.content,
    saved: src.saved,
    savedMtime: src.savedMtime,
    savedMtimeNs: src.savedMtimeNs ?? null,
    mode: src.mode,
    loading: src.loading,
    error: src.error,
    fileMissing: src.fileMissing ? { ...src.fileMissing } : null,
    inspectorOpen: src.inspectorOpen,
    outlineOpen: src.outlineOpen,
    repoRoot: src.repoRoot,
    readMode: src.readMode,
    fsWritable: src.fsWritable,
    styleToolbarOpen: src.styleToolbarOpen,
    syntaxHighlight: src.syntaxHighlight,
    highlightTrailingWhitespace: src.highlightTrailingWhitespace,
    codeBlocksCollapsed: src.codeBlocksCollapsed,
    caret: src.caret ? { ...src.caret } : undefined,
    // Find state is per-tab UI state; drop it when the tab moves
    // panes so the destination opens fresh without a half-mounted
    // bar pointing at a now-defunct adapter.
  };
}

function cloneNode(src: Node): Node {
  if (src.kind === "split") {
    return {
      kind: "split",
      id: src.id,
      direction: src.direction,
      a: src.a,
      b: src.b,
      ratio: src.ratio,
    };
  }
  return {
    kind: "leaf",
    id: src.id,
    tabs: src.tabs.map((tab) => cloneTab(tab)),
    activeTabId: src.activeTabId,
    ...(src.theme ? { theme: src.theme } : {}),
    // HybridSide is an empty marker. Preserve "back has been
    // materialized" by cloning an empty object; pane.back !== undefined
    // still discriminates Hybrid vs never-flipped.
    ...(src.back ? { back: {} } : {}),
    ...(src.showingBack ? { showingBack: true } : {}),
  };
}

function cloneLayoutState(src: LayoutState): LayoutState {
  const nodes: Record<string, Node> = {};
  for (const [id, node] of Object.entries(src.nodes)) {
    nodes[id] = cloneNode(node);
  }
  return {
    rootId: src.rootId,
    nodes,
    activePaneId: src.activePaneId,
    focusColor: src.focusColor,
  } as LayoutState;
}

export function enterPaneMode(): void {
  if (paneMode.active) return;
  paneMode.draft = cloneLayoutState(layout);
  paneMode.active = true;
  paneMode.spawnIntent = null;
  paneMode.transactionMode = false;
  paneMode.grabPaneId = null;
  paneMode.hoverPaneId = null;
  paneMode.stagedDraftEditors = [];
}

/// Mouse-driven Nav entry. `grabPaneId` is the pane the user started
/// dragging from (drag-with-payload), or null when entered via
/// double-click on the dead zone (standby until the user clicks and
/// drags inside any pane). Idempotent if already in transaction mode.
export function enterPaneModeTransaction(grabPaneId: string | null): void {
  if (!paneMode.active) {
    paneMode.draft = cloneLayoutState(layout);
    paneMode.active = true;
    paneMode.spawnIntent = null;
  }
  paneMode.transactionMode = true;
  paneMode.grabPaneId = grabPaneId;
}

/// Set the current grab pane while in transaction mode. Used when the
/// user clicks and drags inside any pane after entering via double-click,
/// or re-grabs a different pane mid-transaction. No-op outside
/// transaction mode.
export function paneModeSetGrab(paneId: string | null): void {
  if (!paneMode.transactionMode) return;
  paneMode.grabPaneId = paneId;
}

/// Track the pane currently under the cursor while a grab is held.
/// Drives the drop-target highlight. No-op outside transaction mode.
export function paneModeSetHover(paneId: string | null): void {
  if (!paneMode.transactionMode) return;
  paneMode.hoverPaneId = paneId;
}

export function commitPaneMode(): void {
  if (!paneMode.active || !paneMode.draft) return;
  // Apply any staged spawn intent into the draft before sealing so the
  // new tab lands as part of the same transaction. Callers that prime
  // side effects for a staged spawn (e.g. `revealAndSelect` for a
  // browser intent) should peek `paneMode.spawnIntent` before calling.
  if (paneMode.spawnIntent) {
    const { kind, ctx } = paneMode.spawnIntent;
    if (kind === "terminal") paneModeOpenTerminal(ctx);
    else if (kind === "browser") paneModeOpenBrowser(ctx);
    else if (kind === "graph") paneModeOpenGraph(ctx);
  }
  const next = cloneLayoutState(paneMode.draft);
  layout.rootId = next.rootId;
  layout.nodes = next.nodes;
  layout.activePaneId = next.activePaneId;
  paneMode.active = false;
  paneMode.draft = null;
  paneMode.spawnIntent = null;
  paneMode.transactionMode = false;
  paneMode.grabPaneId = null;
  paneMode.hoverPaneId = null;
  paneMode.stagedDraftEditors = [];
}

export function cancelPaneMode(): void {
  killStagedTerminalSessions();
  paneMode.active = false;
  paneMode.draft = null;
  paneMode.spawnIntent = null;
  paneMode.transactionMode = false;
  paneMode.grabPaneId = null;
  paneMode.hoverPaneId = null;
  paneMode.stagedDraftEditors = [];
}

/// Kill the PTYs of terminals that exist ONLY in the draft. Staged
/// panes RENDER, so a staged terminal's component mounts for real and
/// spawns a shell (in terminal-only windows every staged split gets
/// one automatically); just dropping the draft on Esc orphaned that
/// shell in the registry until idle-prune. Run each staged terminal's
/// registered close sink — the same explicit-close path `closeTab`
/// uses (kills the session, discards the Rich Prompt draft) — BEFORE
/// the draft stops rendering, while the components and their sinks are
/// still mounted. Committed tabs share ids with the live layout and
/// are never staged, so moves/clones are naturally excluded.
function killStagedTerminalSessions(): void {
  const draft = paneMode.draft;
  if (!paneMode.active || !draft) return;
  const staged = paneModeStagedTabIds();
  if (staged.size === 0) return;
  for (const node of Object.values(draft.nodes)) {
    if (node.kind !== "leaf") continue;
    for (const t of node.tabs) {
      if (t.kind === "terminal" && staged.has(t.id)) {
        void runTerminalCloseSink(t);
      }
    }
  }
}

/// Stage a tab spawn for commit. Replaces any previously-staged intent;
/// pressing `1` then `2` results in the second intent alone. No-op
/// outside Pane Mode.
export function paneModeStageSpawn(
  kind: PaneModeSpawnKind,
  ctx: SpawnContext,
): void {
  if (!paneMode.active) return;
  paneMode.spawnIntent = { kind, ctx };
}

export function clearPaneModeSpawnIntent(): void {
  paneMode.spawnIntent = null;
}

type Direction = "left" | "right" | "up" | "down";

function draftLayout(): LayoutState | null {
  return paneMode.active ? paneMode.draft : null;
}

function parentOf(state: LayoutState, childId: string): SplitNode | null {
  for (const node of Object.values(state.nodes)) {
    if (node.kind === "split" && (node.a === childId || node.b === childId)) {
      return node;
    }
  }
  return null;
}

function containsLeaf(state: LayoutState, nodeId: string, leafId: string): boolean {
  const node = state.nodes[nodeId];
  if (!node) return false;
  if (node.kind === "leaf") return node.id === leafId;
  return containsLeaf(state, node.a, leafId) || containsLeaf(state, node.b, leafId);
}

function extremeLeaf(state: LayoutState, nodeId: string, direction: Direction): string | null {
  const node = state.nodes[nodeId];
  if (!node) return null;
  if (node.kind === "leaf") return node.id;
  if (direction === "left") return extremeLeaf(state, node.b, direction);
  if (direction === "right") return extremeLeaf(state, node.a, direction);
  if (direction === "up") return extremeLeaf(state, node.b, direction);
  return extremeLeaf(state, node.a, direction);
}

function neighbourLeaf(state: LayoutState, leafId: string, direction: Direction): string | null {
  const wantAxis: SplitNode["direction"] =
    direction === "left" || direction === "right" ? "row" : "column";
  let current = leafId;
  let parent = parentOf(state, current);
  while (parent) {
    const inA = containsLeaf(state, parent.a, leafId);
    if (parent.direction === wantAxis) {
      if ((direction === "left" || direction === "up") && !inA) {
        return extremeLeaf(state, parent.a, direction);
      }
      if ((direction === "right" || direction === "down") && inA) {
        return extremeLeaf(state, parent.b, direction);
      }
    }
    current = parent.id;
    parent = parentOf(state, current);
  }
  return null;
}

export function paneModeMoveFocus(direction: Direction): void {
  const draft = draftLayout();
  if (!draft) return;
  const next = neighbourLeaf(draft, draft.activePaneId, direction);
  if (next) draft.activePaneId = next;
}

export function paneModeSwap(direction: Direction): void {
  const draft = draftLayout();
  if (!draft) return;
  const nextId = neighbourLeaf(draft, draft.activePaneId, direction);
  if (!nextId) return;
  paneModeSwapWith(draft.activePaneId, nextId);
}

/// Swap two arbitrary panes' contents by id. The directional
/// `paneModeSwap` reduces to this once the neighbour resolves.
/// Transaction-mode mouse drag uses this directly: grab pane is
/// `grabId`, drop target is `dropId`. Focus moves to the destination
/// so subsequent swaps chain naturally.
export function paneModeSwapWith(grabId: string, dropId: string): void {
  const draft = draftLayout();
  if (!draft) return;
  if (grabId === dropId) return;
  const a = draft.nodes[grabId];
  const b = draft.nodes[dropId];
  if (!a || a.kind !== "leaf" || !b || b.kind !== "leaf") return;
  const aTabs = a.tabs;
  const aActive = a.activeTabId;
  a.tabs = b.tabs;
  a.activeTabId = b.activeTabId;
  b.tabs = aTabs;
  b.activeTabId = aActive;
  draft.activePaneId = b.id;
  // Both panes had their content swapped, so both should
  // wobble so the user's eye tracks where their content
  // landed and which slot now holds whatever was displaced.
  requestPaneWobble(a.id);
  requestPaneWobble(b.id);
}

function nearestAncestorSplit(
  state: LayoutState,
  leafId: string,
  axis: SplitNode["direction"],
): SplitNode | null {
  let current = leafId;
  let parent = parentOf(state, current);
  while (parent) {
    if (parent.direction === axis) return parent;
    current = parent.id;
    parent = parentOf(state, current);
  }
  return null;
}

/// Hybrid Nav resize. `positive=true` shifts the divider toward the
/// right (row axis) or the bottom (column axis); `positive=false`
/// shifts it toward the left / top. Bracket-direction equals
/// divider-direction, independent of which side of the split the
/// active pane sits on. `ratio` is A's share of the split (A is the
/// left / top child), so `positive` maps directly to the delta sign.
export function paneModeResize(
  axis: SplitNode["direction"],
  positive: boolean,
  amount: number,
): void {
  const draft = draftLayout();
  if (!draft) return;
  const split = nearestAncestorSplit(draft, draft.activePaneId, axis);
  if (!split) return;
  const delta = positive ? amount : -amount;
  split.ratio = Math.max(0.05, Math.min(0.95, split.ratio + delta));
}

export function paneModeEqualize(): void {
  const draft = draftLayout();
  if (!draft) return;
  const parent = parentOf(draft, draft.activePaneId);
  if (parent) parent.ratio = 0.5;
}

/// Insert `newPane` next to `originalId` inside a layout state. Same
/// shape as the live-layout `insertSiblingPane` helper, but operates
/// on any LayoutState so it works for the Cmd+K pane-mode draft.
function insertSiblingPaneIn(
  state: LayoutState,
  originalId: string,
  newPane: LeafNode,
  direction: SplitNode["direction"],
  placement: "before" | "after",
): void {
  const original = state.nodes[originalId];
  if (!original || original.kind !== "leaf") return;
  const entries = Object.values(state.nodes);
  const parent = entries.find(
    (n): n is SplitNode =>
      n.kind === "split" && (n.a === original.id || n.b === original.id),
  );
  const split: SplitNode = {
    kind: "split",
    id: id("split"),
    direction,
    a: placement === "before" ? newPane.id : original.id,
    b: placement === "before" ? original.id : newPane.id,
    ratio: 0.5,
  };
  state.nodes[newPane.id] = newPane;
  state.nodes[split.id] = split;
  if (parent) {
    if (parent.a === original.id) parent.a = split.id;
    else parent.b = split.id;
  } else {
    state.rootId = split.id;
  }
}

/// Cmd+K mode `/` and `\\` keybinds. Splits the focused pane in the
/// draft tree only; Enter seals the split and any tabs spawned during
/// the mode, Esc rolls everything back. Structural actions are
/// constrained to right + down.
/// Standalone terminal windows carry `?kind=terminal`. Read here directly
/// (rather than importing the store's `ui.terminalOnly`) to avoid a
/// tabs <-> store import cycle.
function isTerminalWindow(): boolean {
  try {
    return new URLSearchParams(location.search).get("kind") === "terminal";
  } catch {
    return false;
  }
}

export function paneModeSplit(direction: "row" | "column"): void {
  const draft = draftLayout();
  if (!draft) return;
  const original = draft.nodes[draft.activePaneId];
  if (!original || original.kind !== "leaf") return;
  const newPane: LeafNode = {
    kind: "leaf",
    id: id("pane"),
    tabs: [],
    activeTabId: null,
  };
  insertSiblingPaneIn(draft, original.id, newPane, direction, "after");
  draft.activePaneId = newPane.id;
  // Terminal-only windows never have an empty pane: a freshly-split pane gets
  // its own terminal so the new split is immediately usable.
  if (isTerminalWindow()) paneModeOpenTerminal();
}

/// Context for a Pane Mode spawn key. The Cmd+K 1/2/3/4 handlers
/// resolve the focused tab into this shape before calling the spawn
/// helpers, so a new terminal lands on the source file's parent
/// directory and a new Graph tab can scope to (and pre-select) the
/// source node.
///
/// `dir` is the directory the spawn anchors to (terminal cwd, new-file
/// parent, graph dir-scope, file-browser fallback). `""` means root.
///
/// `file` is the file the source tab points at, when applicable.
/// File-Browser and Graph spawns prefer it for "select this exact node";
/// terminal / new-file always fall back to `dir`.
export type SpawnContext = {
  dir: string;
  file?: string;
};

/// Cmd+K mode `1`. Spawn a new terminal tab inside the draft's focused
/// pane. The session WebSocket opens only when the tab mounts, so an
/// Esc rollback leaves no backend state behind.
export function paneModeOpenTerminal(ctx?: SpawnContext): void {
  const draft = draftLayout();
  if (!draft) return;
  const p = draft.nodes[draft.activePaneId];
  if (!p || p.kind !== "leaf") return;
  const cwd = ctx?.dir?.trim();
  const tab: TerminalTab = {
    kind: "terminal",
    id: id("term"),
    title: nextTerminalTitle(draft),
    createdAt: Date.now(),
    broadcastEnabled: false,
    broadcastTargetIds: [],
    terminalSessionId: undefined,
    controlledTerminal: undefined,
    cwd: cwd || undefined,
    seedInput: undefined,
  };
  p.tabs.push(tab);
  p.activeTabId = tab.id;
  // Number from the per-tenant counter (see applyGlobalTerminalName). A
  // pane-mode spawn never carries an explicit title, so every split-spawn is
  // server-numbered; the name resolves in `connect()` before the WS opens.
  tab.pendingGlobalName = true;
}

/// Cmd+K mode `2`. Spawn a fresh File Browser tab inside the draft's
/// focused pane. Every press is a new tab so the user can pile up
/// multiple browser views. When `ctx` carries a file or dir, the
/// inspector opens so the auto-selected node lands with its info pane
/// already visible.
export function paneModeOpenBrowser(ctx?: SpawnContext): void {
  const draft = draftLayout();
  if (!draft) return;
  const p = draft.nodes[draft.activePaneId];
  if (!p || p.kind !== "leaf") return;
  const hasCtx = !!(ctx?.file || ctx?.dir);
  const tab: BrowserTab = {
    kind: "browser",
    id: id("browser"),
    title: "Files",
    inspectorOpen: hasCtx ? true : defaultBrowserInspectorOpen(),
  };
  p.tabs.push(tab);
  p.activeTabId = tab.id;
}

/// Cmd+K mode `3`. Spawn a fresh Graph tab inside the draft's focused
/// pane. Same no-dedup semantic as `paneModeOpenBrowser`. When `ctx`
/// carries a file or dir, scope the new tab to that node and pre-select
/// it; GraphPanel pops the inspector on `pendingSelectId`.
export function paneModeOpenGraph(ctx?: SpawnContext): void {
  const draft = draftLayout();
  if (!draft) return;
  const p = draft.nodes[draft.activePaneId];
  if (!p || p.kind !== "leaf") return;
  const mode: GraphTab["mode"] = "semantic";
  let scopeId = "workspace";
  let pendingSelectId: string | null = null;
  if (ctx?.file) {
    scopeId = `file:${ctx.file}`;
    pendingSelectId = ctx.file;
  } else if (ctx?.dir) {
    scopeId = `dir:${ctx.dir}`;
    pendingSelectId = ctx.dir;
  }
  const tab: GraphTab = {
    kind: "graph",
    id: id("graph"),
    title: graphTitle(mode, scopeId),
    mode,
    scopeId,
    depth: 1,
    expanded: { "": true },
    filters: { ...DEFAULT_GRAPH_FILTERS },
    inspectorOpen: false,
    pendingSelectId,
  };
  p.tabs.push(tab);
  p.activeTabId = tab.id;
}

/// Spawn a Dashboard tab inside the named pane (live layout). Mirrors
/// the shape of `openTerminalInPane` + `openBrowserInActivePane`:
/// append and flip active. No-op if the pane id doesn't resolve to a leaf.
export function openDashboardInPane(paneId: string): void {
  const node = layout.nodes[paneId];
  if (!node || node.kind !== "leaf") return;
  const tab: DashboardTab = {
    kind: "dashboard",
    id: id("dashboard"),
    title: "Dashboard",
  };
  node.tabs.push(tab);
  node.activeTabId = tab.id;
}

export function openDashboardInActivePane(): void {
  openDashboardInPane(layout.activePaneId);
}

/// Stage a "new draft editor" intent onto the currently-focused pane.
/// Materialization is async (needs `api.createDraft()` to mint the
/// file), so the intent queues to commit-time. Multiple presses queue
/// multiple staged drafts, each targeting the pane focused at press
/// time. `paneModeMaterializeStagedDrafts()` is the commit-time
/// resolver.
export interface StagedDraftEditor {
  paneId: string;
}
export function paneModeStageDraftEditor(): void {
  if (!paneMode.active || !paneMode.draft) return;
  const paneId = paneMode.draft.activePaneId;
  paneMode.stagedDraftEditors.push({ paneId });
}

/// Return the set of tab ids that exist in the draft but not in the
/// live layout. Consumers (Pane.svelte's tab strip) render these as
/// dimmed "ghost rows" while pane mode is open. Derived fresh each
/// call; cheaper than a parallel index given the small tab count.
export function paneModeStagedTabIds(): Set<string> {
  if (!paneMode.active || !paneMode.draft) return new Set();
  const live = new Set<string>();
  for (const node of Object.values(layout.nodes)) {
    if (node.kind === "leaf") {
      for (const t of node.tabs) live.add(t.id);
    }
  }
  const staged = new Set<string>();
  for (const node of Object.values(paneMode.draft.nodes)) {
    if (node.kind === "leaf") {
      for (const t of node.tabs) {
        if (!live.has(t.id)) staged.add(t.id);
      }
    }
  }
  return staged;
}

/// Move a tab from one pane to another. If `toIndex` is omitted the tab
/// is appended. Source pane collapses if it becomes empty.
export function moveTab(
  fromPaneId: string,
  tabId: string,
  toPaneId: string,
  toIndex?: number,
): void {
  if (fromPaneId === toPaneId) {
    if (toIndex !== undefined) reorderTab(fromPaneId, tabId, toIndex);
    return;
  }
  const from = pane(fromPaneId);
  const to = pane(toPaneId);
  const idx = from.tabs.findIndex((t) => t.id === tabId);
  if (idx < 0) return;
  // Pull a plain snapshot of the tab. The proxied element won't survive
  // splice + push cleanly across pane boundaries; copying its fields
  // sidesteps the question.
  const src = from.tabs[idx]!;
  const moved = cloneTab(src);
  from.tabs.splice(idx, 1);
  if (from.activeTabId === tabId) {
    from.activeTabId = from.tabs[Math.max(0, idx - 1)]?.id ?? null;
  }
  if (toIndex === undefined || toIndex >= to.tabs.length) {
    to.tabs.push(moved);
  } else {
    to.tabs.splice(Math.max(0, toIndex), 0, moved);
  }
  to.activeTabId = moved.id;
  layout.activePaneId = to.id;
  if (from.tabs.length === 0 && layout.rootId !== from.id) {
    collapseEmptyPane(from.id);
  }
}

export type PaneDropEdge = "left" | "right" | "top" | "bottom";

/// Detach a tab into a new sibling pane at the requested edge of the
/// target leaf. This is the body-drop counterpart to `moveTab`:
/// tab-bar drops merge tab lists, body-edge drops split the target
/// pane and make the moved tab the new sibling content.
export function detachTabToPaneEdge(
  fromPaneId: string,
  tabId: string,
  targetPaneId: string,
  edge: PaneDropEdge,
): void {
  const fromNode = layout.nodes[fromPaneId];
  const targetNode = layout.nodes[targetPaneId];
  if (!fromNode || fromNode.kind !== "leaf") return;
  if (!targetNode || targetNode.kind !== "leaf") return;

  const idx = fromNode.tabs.findIndex((t) => t.id === tabId);
  if (idx < 0) return;
  if (fromPaneId === targetPaneId && fromNode.tabs.length <= 1) return;

  const moved = cloneTab(fromNode.tabs[idx]!);
  fromNode.tabs.splice(idx, 1);
  if (fromNode.activeTabId === tabId) {
    fromNode.activeTabId = fromNode.tabs[Math.max(0, idx - 1)]?.id ?? null;
  }
  if (fromNode.tabs.length === 0 && fromNode.id !== targetNode.id && layout.rootId !== fromNode.id) {
    collapseEmptyPane(fromNode.id);
  }

  const target = layout.nodes[targetPaneId];
  if (!target || target.kind !== "leaf") return;
  const newPane: LeafNode = {
    kind: "leaf",
    id: id("pane"),
    tabs: [moved],
    activeTabId: moved.id,
  };
  const direction: SplitNode["direction"] =
    edge === "left" || edge === "right" ? "row" : "column";
  const placement: "before" | "after" =
    edge === "left" || edge === "top" ? "before" : "after";
  insertSiblingPane(target.id, newPane, direction, placement);
  layout.activePaneId = newPane.id;
}

function collapseEmptyPane(emptyId: string): void {
  // Find the parent split.
  const entries = Object.values(layout.nodes);
  const parent = entries.find(
    (n): n is SplitNode => n.kind === "split" && (n.a === emptyId || n.b === emptyId),
  );
  if (!parent) return;
  const siblingId = parent.a === emptyId ? parent.b : parent.a;
  const grand = entries.find(
    (n): n is SplitNode => n.kind === "split" && (n.a === parent.id || n.b === parent.id),
  );
  if (grand) {
    if (grand.a === parent.id) grand.a = siblingId;
    else grand.b = siblingId;
  } else {
    // parent is root.
    layout.rootId = siblingId;
  }
  delete layout.nodes[parent.id];
  delete layout.nodes[emptyId];
  // If the active pane was the emptied one, fall back to the sibling.
  if (layout.activePaneId === emptyId) layout.activePaneId = firstLeafId(siblingId);
  // Wobble the leaf that just absorbed the freed space so the
  // user has a visual anchor on where their attention should
  // land. If the sibling was a split, wobble its first leaf;
  // the rest of the subtree just inherits the new dimensions
  // without wobble (cheaper, and the focal point is enough).
  const absorber = firstLeafId(siblingId);
  if (absorber) requestPaneWobble(absorber);
}


/// Splits are uncapped. Kept as a function (rather than removing
/// every call site) because Pane.svelte and `splitActive` both go
/// through it; if a future surface needs a cap, this is the choke
/// point.
export function canSplit(): boolean {
  return true;
}

export function splitActive(direction: "row" | "column"): void {
  splitPane(layout.activePaneId, direction, "after");
  // Terminal-only windows never have an empty pane: the new split pane (now
  // active) gets its own terminal.
  if (isTerminalWindow()) openTerminalInActivePane({});
}

/// Materialize an R×C grid of panes starting from `startPaneId`.
/// Returns the pane IDs in row-major order (`cells[r * cols + c]`).
///
/// Strategy:
///   1. Build a top row of `cols` panes by splitting horizontally
///      from the starting pane `cols - 1` times. Each split adds a
///      pane to the RIGHT.
///   2. For each column-head, split vertically `rows - 1` times.
///      Each split adds a pane BELOW.
///
/// Side effect: `layout.activePaneId` ends on the bottom-right pane.
/// Callers that need focus on a specific pane restore it afterwards.
/// For `1×1` grids the helper short-circuits with `[startPaneId]`.
export function buildSplitGrid(
  startPaneId: string,
  rows: number,
  cols: number,
): string[] {
  if (rows <= 1 && cols <= 1) return [startPaneId];
  // Step 1: top row of column-heads.
  const columnHeads: string[] = [startPaneId];
  for (let c = 1; c < cols; c += 1) {
    splitPane(columnHeads[c - 1], "row", "after");
    columnHeads.push(layout.activePaneId);
  }
  // Step 2: build down each column to populate the rest of the
  // grid. `grid[r][c]` is the pane at row r, column c.
  const grid: string[][] = Array.from({ length: rows }, () =>
    Array<string>(cols).fill(""),
  );
  for (let c = 0; c < cols; c += 1) {
    grid[0][c] = columnHeads[c];
    for (let r = 1; r < rows; r += 1) {
      splitPane(grid[r - 1][c], "column", "after");
      grid[r][c] = layout.activePaneId;
    }
  }
  // Flatten row-major to match the dialog's slot ordering
  // (`TeamRealEstate.slots[i]` is cell i in row-major).
  const flat: string[] = [];
  for (let r = 0; r < rows; r += 1) {
    for (let c = 0; c < cols; c += 1) flat.push(grid[r][c]);
  }
  return flat;
}

export function splitPane(
  paneId: string,
  direction: "row" | "column",
  placement: "before" | "after" = "after",
): void {
  if (!canSplit()) return;
  const original = pane(paneId);
  // New panes are born empty and must NOT inherit the original's flip.
  // A flipped state belongs strictly to a pane with >= 1 tab; copying
  // showingBack onto a 0-tab pane produces a stuck "flipped empty pane"
  // the chord cannot undo.
  const newPane: LeafNode = {
    kind: "leaf",
    id: id("pane"),
    tabs: [],
    activeTabId: null,
  };
  insertSiblingPane(original.id, newPane, direction, placement);
  layout.activePaneId = newPane.id;
  requestPaneWobble(original.id);
  requestPaneWobble(newPane.id);
}

function insertSiblingPane(
  originalId: string,
  newPane: LeafNode,
  direction: SplitNode["direction"],
  placement: "before" | "after",
): void {
  const original = pane(originalId);
  // Find parent of original so we can replace original with a new split.
  const entries = Object.values(layout.nodes);
  const parent = entries.find(
    (n): n is SplitNode => n.kind === "split" && (n.a === original.id || n.b === original.id),
  );
  const split: SplitNode = {
    kind: "split",
    id: id("split"),
    direction,
    a: placement === "before" ? newPane.id : original.id,
    b: placement === "before" ? original.id : newPane.id,
    ratio: 0.5,
  };
  layout.nodes[newPane.id] = newPane;
  layout.nodes[split.id] = split;
  if (parent) {
    if (parent.a === original.id) parent.a = split.id;
    else parent.b = split.id;
  } else {
    layout.rootId = split.id;
  }
}

export function setActivePane(paneId: string): void {
  const current = activeLayout();
  if (current.nodes[paneId]?.kind !== "leaf") return;
  // Single-shot wobble cue on keyboard / click pane-switch, mirroring
  // the bounce already fired by split / close / pane-move. Only fire
  // when the active pane actually CHANGES so re-clicks on the same
  // pane (already-focused) stay quiet; otherwise the wobble would
  // re-trigger on every mousedown that lands on the focused pane.
  const previousActive = current.activePaneId;
  // Focus changes must NOT touch any pane's `showingBack`. The flip is
  // a per-pane boolean owned solely by flipHybrid on the focused,
  // non-empty pane; each pane keeps its own state independently and
  // across reloads. Coupling this to focus causes moving focus to visibly
  // flip sibling tabs.
  current.activePaneId = paneId;
  if (previousActive !== paneId) requestPaneWobble(paneId);
}

/// Flip the pane between its front (content tabs) and its back
/// (per-surface configuration view). `pane.tabs` always stays the
/// front's tab list. `pane.theme` is the single per-Hybrid theme
/// value; this function only toggles `showingBack`.
export function flipHybrid(paneId: string): void {
  const node = activeLayout().nodes[paneId];
  if (!node || node.kind !== "leaf") return;
  // Empty pane has no surface to flip; guard so the chrome animation
  // does not fire on an empty pane.
  if (node.tabs.length === 0) return;
  if (!node.back) {
    // Lazy init: materialise an empty back marker so `pane.back !== undefined`
    // gates the hamburger Theme / Flip entries. `pane.theme` owns the
    // single per-Hybrid theme value.
    node.back = {};
  }
  // The two-face card transitions off `showingBack` directly, so the
  // boolean toggle is the whole flip trigger (no orientation bus).
  node.showingBack = !node.showingBack;
}

export function setMode(tab: Tab, mode: Mode): void {
  if (tab.kind === "file") tab.mode = mode;
}

/// Flip the active pane's file tab between source and its rendered surface
/// (markdown → wysiwyg, JSON → pretty, CSV/TSV → table). Gated to files that
/// HAVE a rendered surface: plain text (.rs/.py/.toml/Makefile) has only source
/// mode, so the Mod+E chord is a NO-OP there instead of forcing an invalid
/// wysiwyg render. `defaultModeForPath` yields the rendered mode for renderable
/// files and "source" for source-only ones — the same split FileEditorTab's
/// in-menu "Show Source Code" toggle uses (`hasRenderedMode` /
/// `renderedModeForTab`). Routed via the Mod+E chord; the right-click "Show
/// Source Code" entry runs FileEditorTab's `doToggleMode` (which adds caret
/// remapping). This helper does the basic flip and lets the editor's internal
/// effect handle position fidelity. No-op when the active tab isn't a file tab.
export function toggleActiveFileTabMode(): void {
  const node = layout.nodes[layout.activePaneId];
  if (!node || node.kind !== "leaf") return;
  const tab = node.tabs.find((t) => t.id === node.activeTabId);
  if (!tab || tab.kind !== "file") return;
  const rendered = defaultModeForPath(tab.path, tab.fileKind);
  if (rendered === "source") return;
  setMode(tab, tab.mode === "source" ? rendered : "source");
}

/// Tab-state mutators. These exist so child components (FileEditorTab
/// etc.) don't write tab.X = ... directly on a non-bindable prop - /// Svelte 5's ownership tracking warns about that pattern. Centralizing
/// the writes here also gives us one place to add side-effects
/// (persistence, telemetry) later.
export function setTabCaret(tab: FileTab, from: number, to: number): void {
  tab.caret = { from, to };
}
export function setTabInspectorOpen(tab: FileTab, open: boolean): void {
  tab.inspectorOpen = open;
}
export function setTabOutlineOpen(tab: FileTab, open: boolean): void {
  tab.outlineOpen = open;
}
export function setTabStyleToolbarOpen(tab: FileTab, open: boolean): void {
  tab.styleToolbarOpen = open;
}
export function setTabSyntaxHighlight(tab: FileTab, on: boolean): void {
  tab.syntaxHighlight = on;
}
export function setTabHighlightTrailingWhitespace(tab: FileTab, on: boolean): void {
  tab.highlightTrailingWhitespace = on;
}
export function setTabCodeBlocksCollapsed(tab: FileTab, collapsed: boolean): void {
  tab.codeBlocksCollapsed = collapsed;
}

/// Whether a tab represents an unsaved buffer.
export function isDirty(t: Tab): boolean {
  if (t.kind !== "file") return false;
  if (t.loading) return false;
  return t.content !== t.saved;
}

// ---- autosave + CAS conflict prompt -------------------------------------

// Debounce window for idle autosave. Picked short enough that data loss
// from a crash is small, long enough that bursty typing doesn't
// hammer the disk + watcher.
const AUTOSAVE_DEBOUNCE_MS = 800;
const autosaveTimers = new Map<string, ReturnType<typeof setTimeout>>();
const savingTabs = new Set<string>();
const saveAgainAfterCurrent = new Set<string>();
let pendingMissingFileReopenTabId: string | null = null;

/// Conflict dialog state. Populated when a save returns 409 (an
/// external edit landed since we last read this tab). Mounted by
/// ConflictModal.svelte; closed via reloadConflictedTab,
/// overwriteConflictedTab, or dismissConflict.
export const conflictDialog = $state<{
  open: boolean;
  /// Tab the conflict is for. Null when the dialog is closed.
  tabId: string | null;
  /// Tab path for display in the dialog (the user shouldn't have to
  /// guess which tab triggered the prompt).
  path: string;
  /// Mtime currently on disk per the server's 409 body. Used as the
  /// next CAS token whether the user reloads (refetch with this
  /// token) or overwrites (write with this token; another conflict
  /// re-prompts if a third edit landed in the meantime).
  currentMtime: number | null;
  currentMtimeNs: string | null;
}>({ open: false, tabId: null, path: "", currentMtime: null, currentMtimeNs: null });

export function dismissConflict(): void {
  conflictDialog.open = false;
  conflictDialog.tabId = null;
  conflictDialog.path = "";
  conflictDialog.currentMtime = null;
  conflictDialog.currentMtimeNs = null;
}

function findFileTabById(tabId: string): { paneId: string; tab: FileTab } | null {
  for (const [paneId, node] of Object.entries(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    const tab = node.tabs.find((t) => t.id === tabId);
    if (tab && tab.kind === "file") return { paneId, tab };
  }
  return null;
}

/// Discard the in-memory buffer for the conflicted tab and re-fetch
/// from disk. The user picked Reload: their unsaved edits go away,
/// the disk version takes over.
export async function reloadConflictedTab(): Promise<void> {
  const tabId = conflictDialog.tabId;
  dismissConflict();
  if (!tabId) return;
  const found = findFileTabById(tabId);
  if (!found) return;
  await loadTabContent(found.paneId, found.tab.id, found.tab.path);
}

/// Adopt the server-reported on-disk mtime as the new CAS token and
/// save the in-memory buffer. The CAS check matches, so the write
/// goes through (unless ANOTHER external edit landed since the 409
/// was issued, in which case we re-prompt).
export async function overwriteConflictedTab(): Promise<void> {
  const tabId = conflictDialog.tabId;
  const currentMtime = conflictDialog.currentMtime;
  const currentMtimeNs = conflictDialog.currentMtimeNs;
  dismissConflict();
  if (!tabId) return;
  const found = findFileTabById(tabId);
  if (!found) return;
  found.tab.savedMtime = currentMtime;
  found.tab.savedMtimeNs = currentMtimeNs;
  await performSave(found.tab);
}

/// Single source of truth for "send this tab's content to the
/// server". Both autosave and explicit saveTab funnel through here.
/// On 409, opens the conflict dialog and returns; the dialog's
/// Reload / Overwrite buttons workspace the recovery.
///
/// Format-specific pre-checks live here so the gate is uniform
/// across autosave and Cmd+S. Today only JSON is validated:
/// writing invalid JSON onto disk would surface as a parse error
/// the next time a tool / our own pretty viewer reads the file,
/// which is too late to recover the user's typo. Refusing the
/// write at the editor boundary keeps the file system honest.
async function performSave(t: FileTab): Promise<void> {
  if (savingTabs.has(t.id)) {
    saveAgainAfterCurrent.add(t.id);
    return;
  }
  savingTabs.add(t.id);
  try {
    do {
      saveAgainAfterCurrent.delete(t.id);
      await performSaveOnce(t);
    } while (saveAgainAfterCurrent.has(t.id) && t.content !== t.saved);
  } finally {
    savingTabs.delete(t.id);
    saveAgainAfterCurrent.delete(t.id);
  }
}

async function performSaveOnce(t: FileTab): Promise<void> {
  if (t.loading) {
    t.error = "file is still loading";
    return;
  }
  if (isJson(t.path)) {
    const reason = validateJsonBuffer(t.content);
    if (reason !== null) {
      t.error = `JSON parse error: ${reason}`;
      return;
    }
  }
  const path = t.path;
  const sourceContent = t.content;
  const stripOnSave = editorToolsPrefs.stripTrailingWhitespaceOnSave;
  const content = stripOnSave
    ? stripTrailingWhitespaceText(sourceContent)
    : sourceContent;
  const expectedMtimeNs = t.savedMtimeNs ?? null;
  const expectedMtime = t.savedMtime;
  try {
    const r = await api.write(path, content, expectedMtimeNs, expectedMtime);
    if (stripOnSave && content !== sourceContent && t.content === sourceContent) {
      t.content = content;
    }
    t.saved = content;
    t.savedMtime = r.mtime ?? null;
    t.savedMtimeNs = r.mtime_ns ?? null;
    t.error = null;
    t.fileMissing = null;
    mirrorToSiblings(path, content, t.id);
  } catch (e) {
    if (e instanceof ApiError && e.status === 409) {
      const data = e.data as {
        current_mtime?: number | null;
        current_mtime_ns?: string | null;
      } | null;
      conflictDialog.open = true;
      conflictDialog.tabId = t.id;
      conflictDialog.path = t.path;
      conflictDialog.currentMtime = data?.current_mtime ?? null;
      conflictDialog.currentMtimeNs = data?.current_mtime_ns ?? null;
      return;
    }
    throw e;
  }
}

/// Return null when `src` parses as JSON, otherwise the
/// JSON.parse error message. An empty / whitespace-only buffer is
/// accepted: a fresh `.json` file the user has not yet typed into
/// is allowed to round-trip empty.
function validateJsonBuffer(src: string): string | null {
  if (src.trim() === "") return null;
  try {
    JSON.parse(src);
    return null;
  } catch (e) {
    return (e as Error).message;
  }
}

/// Schedule (or reschedule) a save for `tab` in `pane`. Multiple
/// rapid calls coalesce: only the last one's timer fires.
export function scheduleAutosave(paneId: string, tabId: string): void {
  const existing = autosaveTimers.get(tabId);
  if (existing) clearTimeout(existing);
  const timer = setTimeout(async () => {
    autosaveTimers.delete(tabId);
    const node = layout.nodes[paneId];
    if (!node || node.kind !== "leaf") return;
    const t = node.tabs.find((tab) => tab.id === tabId);
    if (!t || t.kind !== "file") return;
    if (t.loading || t.content === t.saved) return;
    try {
      await performSave(t);
    } catch (e) {
      t.error = `autosave failed: ${(e as Error).message}`;
    }
  }, AUTOSAVE_DEBOUNCE_MS);
  autosaveTimers.set(tabId, timer);
}

/// After a save, sync the new content into every other tab pointing
/// at the same path so the duplicate views don't drift. Skip tabs
/// that have their own dirty buffer (they have local edits the
/// user hasn't saved yet, and overwriting would silently destroy
/// work). Those stay divergent and the user can resolve manually;
/// the dirty dot already signals the divergence.
function mirrorToSiblings(path: string, content: string, originId: string): void {
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    for (const sib of node.tabs) {
      if (sib.kind !== "file") continue;
      if (sib.id === originId) continue;
      if (sib.path !== path) continue;
      // Honor an unsaved buffer in the sibling; don't clobber.
      if (sib.content !== sib.saved) continue;
      sib.content = content;
      sib.saved = content;
      sib.error = null;
      sib.fileMissing = null;
    }
  }
}

export function isMissingFileError(e: unknown): boolean {
  if (e instanceof ApiError && e.status === 404) return true;
  const msg = String((e as Error | null)?.message ?? e).toLowerCase();
  return (
    msg.includes("no such file") ||
    msg.includes("not found") ||
    msg.includes("os error 2") ||
    msg.includes("enoent")
  );
}

function missingFragment(content: string): string | null {
  const normalized = content
    .split(/\r?\n/)
    .map((line) => line.trim())
    .find((line) => line.length >= 8);
  if (!normalized) return null;
  return normalized.length > 80 ? normalized.slice(0, 80) : normalized;
}

function markFileMissing(t: FileTab): void {
  t.error = null;
  t.loading = false;
  t.fileMissing = {
    path: t.path,
    fragment: missingFragment(t.content || t.saved),
  };
}

/// Set of paths with unsaved changes across all panes. Used by the
/// file tree to surface the same dirty indicator the per-tab UI shows.
export function dirtyPaths(): Set<string> {
  const out = new Set<string>();
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    for (const t of node.tabs) {
      if (t.kind === "file" && !t.loading && t.content !== t.saved) out.add(t.path);
    }
  }
  return out;
}

// ---- serialization for URL-based persistence -----------------------------

// Tree-shaped serialized form. We store the layout as a recursive
// shape without IDs; restore generates fresh IDs. Tabs carry just
// enough to reconstruct each variant.
//   k: "f" file tab (the only kind today; older sessions may carry
//      "b" browser, "s" settings, "g" graph, "h" health, all silently
//      dropped on restore since each became a window-level overlay).
//   p,m,o only meaningful for files
//   a: active tab in this pane (one per pane)
//   f: focused pane (one per layout)
type SerTab = {
  k?: "f" | "b" | "s" | "g" | "h" | "t" | "d";
  p?: string;
  n?: string;
  m?: Mode;
  a?: 1;
  o?: 1;
  /// Outline pane (left-side) visibility. Default off, so we only
  /// emit `ol: 1` when the user has opted the outline in.
  ol?: 1;
  /// Style toolbar visibility. Default is "hidden" for new tabs;
  /// we only emit `s: 1` when the user explicitly enabled it so
  /// the common case keeps the hash short. Restores without the
  /// field land on the default.
  s?: 1;
  /// User-toggled read mode. Default is write; we only emit
  /// `r: 1` when the user explicitly flipped a tab into read
  /// mode so the common case keeps the hash short. fsWritable
  /// is NOT persisted (it's a disk property; refreshed on first
  /// loadTabContent).
  r?: 1;
  /// Persisted caret as `[from, to]`. Omitted when at offset 0 so
  /// fresh tabs keep the hash short. The active editor mirrors
  /// `tab.caret` here on every selection change.
  c?: [number, number];
  /// Syntax-highlight toggle. Default is "on" so we only emit
  /// `h: 0` when the user has explicitly disabled highlighting
  /// for this tab. Restores without the field land on default-on.
  h?: 0;
  /// Terminal PTY session id. Only emitted in the per-window
  /// session payload, never in the shareable URL hash.
  tsid?: string;
  /// Terminal was created through the HTTP control channel; restart
  /// uses the server-side restart endpoint.
  tc?: 1;
  /// Last injected agent-event echo sequence the browser handled.
  /// Used only for replaying missed Team Work watcher dispatches.
  tae?: number;
  /// Terminal broadcast group. Emitted only when non-default so a
  /// reattach after reload keeps the terminal in its group (and the
  /// SPA group stays consistent with the server's per-session tab_group).
  tg?: string;
  /// Negotiated keyboard-protocol snapshot (modifyOtherKeys / kitty
  /// flags). Emitted only for a live session with non-default state so
  /// Shift+Enter -> newline survives a reload reattaching to a long-lived
  /// agent. See `TerminalTab.keyboardProtocol`.
  kp?: SerializedKeyboardProtocolState;
  /// Rich Prompt per-terminal draft path (<draftsDir>/<name>/draft.md). Persisted
  /// so a reload rebinds the per-terminal Rich Prompt draft + the close
  /// cleanup deletes the right draft folder. Per-window session payloads only.
  rpd?: string;
  /// Rich Prompt in-flight message (id + phase) so a queued message survives a
  /// window reload (GAP 2 / the reload contract): on reattach it is re-proved
  /// against the `session` frame's `queued_prompt_ids` (still queued → re-lock +
  /// re-show with position; drained → clear). Only the actionable in-flight
  /// phases ("sent"/"queued") are persisted. Per-window session payloads only.
  pp?: { id: string; ph: "sent" | "queued" };
  /// Rich Prompt bubble was visible — reshow it on reload so a restored queued
  /// message is actionable without re-toggling Cmd+Shift+P. Session payloads only.
  rpv?: 1;
  /// Team Work spawn-agents dialog config draft for a pending LEAD terminal, so
  /// a reload reopens the dialog with exactly what the user was editing. Stored
  /// verbatim (the session blob isn't size-constrained like the URL hash).
  /// Per-window session payloads ONLY — kept out of the shareable URL hash since
  /// a member's `env` can carry secrets. See `TerminalTab.teamWorkPending`.
  twk?: TeamDialogConfig;
  /// Graph tab state.
  gm?: "s" | "f" | "l";
  gs?: string;
  gd?: number;
  gi?: 1;
  gf?: string;
  /// Expanded directory paths for the filesystem spine (graph expand /
  /// collapse). Workspace-relative; the root ("") is always expanded and
  /// omitted. Absent when nothing beyond the root is open.
  ge?: string[];
  gp?: string;
  /// Persisted live selection: `gn` is the last-tapped graph node id,
  /// `gnl` is the cached human-readable label so the tab title renders
  /// before graph data finishes reloading.
  gn?: string;
  gnl?: string;
  /// Browser tab state.
  bi?: 1;
  /// Per-tab File Browser view state. Selection (`bs`), workspace-info-
  /// showing flag (`bd`), expanded directory paths (`be`), and scroll
  /// offset (`bsc`). All optional; absence means default state.
  bs?: string;
  bd?: 1;
  be?: string[];
  bsc?: number;
  /// Per-tab inspector / outline widths. `iw` covers BrowserTab +
  /// GraphTab + FileTab; `ow` is FileTab only (outline pane). Emitted
  /// only when set so single-tab hashes stay clean.
  iw?: number;
  ow?: number;
  /// DashboardTab carousel slide cursor. 0 (the About slide, the
  /// default) is omitted to keep the hash compact.
  cs?: number;
  /// DashboardTab disabled slot indices. Omitted when empty (all slots
  /// enabled, the default).
  ds?: number[];
  /// DashboardTab auto-rotate flag. Omitted when true (the default);
  /// emitted as false when the tab opted out of auto-rotation.
  ar?: boolean;
};
type SerFocusColor = "o" | "g" | "p";
type SerHybridTheme = "d" | "l";
type SerLeaf = {
  k: "l";
  t: SerTab[];
  f?: 1;
  wc?: SerFocusColor;
  pc?: SerFocusColor;
  /// Per-pane Hybrid back-side state. The back is a per-surface
  /// configuration view; `pane.theme` is the single per-Hybrid theme.
  /// Wire-compat: legacy `bt` and `hb` fields from older sessions are
  /// parsed and discarded on rehydrate; the front-side `ht` wins.
  /// `ht`: per-Hybrid theme override.
  /// `sb`: `1` when the pane is currently flipped to its back view.
  /// `bm`: `1` when the pane has been flipped at least once.
  ht?: SerHybridTheme;
  sb?: 1;
  bm?: 1;
  /// Legacy: back-side tabs and back-side theme override from older
  /// sessions. Ignored on rehydrate for wire compat.
  bt?: SerTab[];
  hb?: SerHybridTheme;
};
type SerSplit = {
  k: "s";
  d: "r" | "c";
  a: SerNode;
  b: SerNode;
  r?: number;
  wc?: SerFocusColor;
};
type SerNode = SerLeaf | SerSplit;

function serializeFocusColor(color: FocusColor | undefined): { wc?: SerFocusColor } {
  if (color === "orange") return { wc: "o" };
  if (color === "green") return { wc: "g" };
  if (color === "pink") return { wc: "p" };
  return {};
}

function restoreFocusColor(color: SerFocusColor | undefined): FocusColor {
  if (color === "o") return "orange";
  if (color === "g") return "green";
  if (color === "p") return "pink";
  return "blue";
}

/// Expanded directory paths for a graph tab, excluding the always-open
/// root. Used to serialize the filesystem-spine expand/collapse state.
function graphExpandedList(expanded: Record<string, boolean>): string[] {
  return Object.keys(expanded).filter((k) => k && expanded[k]);
}

/// Rebuild a graph tab's expanded set from its serialized path list. The
/// root ("") is always expanded.
function graphExpandedFromList(
  list: string[] | undefined,
): Record<string, boolean> {
  const map: Record<string, boolean> = { "": true };
  if (Array.isArray(list)) {
    for (const p of list) if (typeof p === "string" && p) map[p] = true;
  }
  return map;
}

function encodeGraphTabFilters(f: GraphFilters): string {
  // The leading `2` is a version sentinel so the decoder can tell a
  // legacy payload (no prefix; missing `d`/`s` default ON) from a
  // current payload (prefix present; missing `d`/`s` mean explicit OFF).
  return [
    "2",
    f.link ? "l" : "",
    f.tag ? "t" : "",
    f.mention ? "m" : "",
    f.language ? "a" : "",
    f.img ? "i" : "",
    f.folder ? "f" : "",
    f.markdown ? "d" : "",
    f.source ? "s" : "",
  ].join("");
}

function decodeGraphTabFilters(s: string | undefined): GraphFilters {
  const src = s ?? "2ltmaifds";
  // A leading `2` marks the current payload format. Without it the
  // payload is from an older session and `markdown` / `source` default
  // ON; with it, missing chars are explicit OFF.
  const isV2 = src.startsWith("2");
  return {
    link: src.includes("l"),
    tag: src.includes("t"),
    mention: src.includes("m"),
    language: src.includes("a"),
    img: src.includes("i"),
    folder: src.includes("f"),
    markdown: isV2 ? src.includes("d") : true,
    source: isV2 ? src.includes("s") : true,
  };
}

function restoreGraphMode(mode: SerTab["gm"]): GraphTab["mode"] {
  if (mode === "f") return "filesystem";
  if (mode === "l") return "language";
  return "semantic";
}

/// "Copy link to graph": a graph tab serializes to a
/// `chan://graph?...` URL that reproduces the view when opened from a
/// markdown file. A custom-scheme URL survives a paste into markdown
/// intact and is trivial to detect on a link click. Round-trips
/// scope / depth / mode / filters / selection via `parseGraphLink`.
export const GRAPH_LINK_PREFIX = "chan://graph?";

export function graphLinkFor(tab: GraphTab): string {
  const params = new URLSearchParams();
  params.set("s", tab.scopeId);
  if (tab.depth !== 1) params.set("d", String(tab.depth));
  params.set(
    "m",
    tab.mode === "filesystem" ? "f" : tab.mode === "language" ? "l" : "s",
  );
  params.set("f", encodeGraphTabFilters(tab.filters));
  if (tab.selectedNodeId) params.set("n", tab.selectedNodeId);
  return `${GRAPH_LINK_PREFIX}${params.toString()}`;
}

export type ParsedGraphLink = {
  mode: GraphTab["mode"];
  scopeId: string;
  depth: number;
  filters: GraphFilters;
  selectedNodeId: string | null;
};

/// Parse a `chan://graph?...` link back into the fields needed to open a
/// graph tab. Returns null when the string is not a graph link or has no
/// scope. Lenient on the rest: missing depth -> 1, missing mode ->
/// semantic, missing filters -> all-on (decodeGraphTabFilters default).
export function parseGraphLink(link: string): ParsedGraphLink | null {
  const trimmed = link.trim();
  if (!trimmed.startsWith(GRAPH_LINK_PREFIX)) return null;
  let params: URLSearchParams;
  try {
    params = new URLSearchParams(trimmed.slice(GRAPH_LINK_PREFIX.length));
  } catch {
    return null;
  }
  const scopeId = params.get("s");
  if (!scopeId) return null;
  const depthRaw = params.get("d");
  const depth = depthRaw ? Number.parseInt(depthRaw, 10) : 1;
  const modeChar = params.get("m");
  return {
    mode:
      modeChar === "f"
        ? "filesystem"
        : modeChar === "l"
          ? "language"
          : "semantic",
    scopeId,
    depth: Number.isFinite(depth) && depth > 0 ? depth : 1,
    filters: decodeGraphTabFilters(params.get("f") ?? undefined),
    selectedNodeId: params.get("n"),
  };
}

/// Walk the layout starting at `nodeId`, producing a serializable tree.
function serializeTab(
  t: Tab,
  isActive: boolean,
  opts: SerializeLayoutOptions,
): SerTab {
  const active = isActive ? { a: 1 as const } : {};
  if (t.kind === "terminal") {
    return {
      k: "t",
      n: t.title,
      ...(terminalTabGroup(t) !== DEFAULT_TERMINAL_GROUP
        ? { tg: terminalTabGroup(t) }
        : {}),
      ...(opts.terminalSessions && t.terminalSessionId
        ? {
            tsid: t.terminalSessionId,
            ...(t.controlledTerminal ? { tc: 1 as const } : {}),
            ...(typeof t.lastAgentEchoSeq === "number" &&
            Number.isFinite(t.lastAgentEchoSeq) &&
            t.lastAgentEchoSeq > 0
              ? { tae: Math.floor(t.lastAgentEchoSeq) }
              : {}),
            ...(() => {
              const kp = serializeKeyboardProtocolState(t.keyboardProtocol);
              return kp ? { kp } : {};
            })(),
          }
        : {}),
      ...(opts.terminalSessions && t.richPromptDraftPath
        ? { rpd: t.richPromptDraftPath }
        : {}),
      // Persist a QUEUED Rich Prompt message (id + phase) + bubble visibility
      // so it survives a reload (GAP 2). Only "queued" (acked + in the queue)
      // is persisted: "sent" is a sub-300ms pre-ack transient, and terminal
      // phases resolve before any save. On reattach it's re-proved against the
      // session frame's queued_prompt_ids (kept+positioned, or cleared).
      ...(opts.terminalSessions && t.pendingPrompt?.phase === "queued"
        ? { pp: { id: t.pendingPrompt.id, ph: "queued" as const } }
        : {}),
      ...(opts.terminalSessions && isRichPromptVisible(t.id) ? { rpv: 1 as const } : {}),
      // Pending Team Work dialog config: session payloads only (never the
      // shareable URL hash). Reopens the dialog with the user's in-progress
      // config on reload.
      ...(opts.terminalSessions && t.teamWorkPending ? { twk: t.teamWorkPending } : {}),
      ...active,
    };
  }
  if (t.kind === "graph") {
    return {
      k: "g",
      gm: t.mode === "filesystem" ? "f" : t.mode === "language" ? "l" : "s",
      gs: t.scopeId,
      ...(t.depth !== 1 ? { gd: t.depth } : {}),
      ...(t.inspectorOpen ? { gi: 1 as const } : {}),
      gf: encodeGraphTabFilters(t.filters),
      ...(graphExpandedList(t.expanded).length
        ? { ge: graphExpandedList(t.expanded) }
        : {}),
      ...(t.pendingSelectId ? { gp: t.pendingSelectId } : {}),
      // Persist the live selection so reload restores both the selected
      // node and the selection-driven tab title before data reloads.
      ...(t.selectedNodeId ? { gn: t.selectedNodeId } : {}),
      ...(t.selectedNodeLabel ? { gnl: t.selectedNodeLabel } : {}),
      ...(t.inspectorWidth && t.inspectorWidth > 0
        ? { iw: Math.round(t.inspectorWidth) }
        : {}),
      ...active,
    };
  }
  if (t.kind === "browser") {
    const expanded = t.expanded?.filter((p) => p.length > 0) ?? [];
    return {
      k: "b",
      ...(t.inspectorOpen ? { bi: 1 as const } : {}),
      ...(t.selected ? { bs: t.selected } : {}),
      ...(t.showWorkspace ? { bd: 1 as const } : {}),
      ...(expanded.length > 0 ? { be: expanded } : {}),
      ...(t.scroll && t.scroll > 0 ? { bsc: Math.round(t.scroll) } : {}),
      ...(t.inspectorWidth && t.inspectorWidth > 0
        ? { iw: Math.round(t.inspectorWidth) }
        : {}),
      ...active,
    };
  }
  if (t.kind === "dashboard") {
    return {
      k: "d",
      // Persist the carousel slide so reload restores the user to the
      // slide they were on. Skip at default (0) to keep the hash short.
      ...(typeof t.carouselSlide === "number" && t.carouselSlide > 0
        ? { cs: t.carouselSlide }
        : {}),
      // Persist the disabled slot set; omit when empty (all-on default).
      ...(t.disabledSlots && t.disabledSlots.length > 0
        ? { ds: t.disabledSlots }
        : {}),
      // Persist the auto-rotate opt-out; omit when on (the default).
      ...(t.autoRotate === false ? { ar: false } : {}),
      ...active,
    };
  }
  // Only file tabs left; omit `k:"f"` since `"f"` is the default.
  // Skip the caret field when it sits at the doc start. New tabs
  // (and never-focused restored tabs) have caret==undefined; only
  // emit it when the user has moved off offset 0.
  const c =
    t.caret && (t.caret.from !== 0 || t.caret.to !== 0)
      ? { c: [t.caret.from, t.caret.to] as [number, number] }
      : {};
  return {
    p: t.path,
    m: t.mode,
    ...active,
    ...(t.inspectorOpen ? { o: 1 as const } : {}),
    ...(t.outlineOpen ? { ol: 1 as const } : {}),
    ...(t.styleToolbarOpen ? { s: 1 as const } : {}),
    ...(t.readMode ? { r: 1 as const } : {}),
    ...(t.syntaxHighlight ? {} : { h: 0 as const }),
    ...(t.inspectorWidth && t.inspectorWidth > 0
      ? { iw: Math.round(t.inspectorWidth) }
      : {}),
    ...(t.outlineWidth && t.outlineWidth > 0
      ? { ow: Math.round(t.outlineWidth) }
      : {}),
    ...c,
  };
}

function serializeHybridTheme(
  theme: HybridTheme | undefined,
): SerHybridTheme | undefined {
  if (theme === "dark") return "d";
  if (theme === "light") return "l";
  return undefined;
}

function serializeNode(
  nodeId: string,
  opts: SerializeLayoutOptions,
): SerNode | null {
  const n = layout.nodes[nodeId];
  if (!n) return null;
  if (n.kind === "leaf") {
    const tabs: SerTab[] = n.tabs.map((t) =>
      serializeTab(t, t.id === n.activeTabId, opts),
    );
    const out: SerLeaf = {
      k: "l",
      t: tabs,
      ...(n.id === layout.activePaneId ? { f: 1 as const } : {}),
    };
    // Hybrid back-side state is inlined into SerLeaf so the URL hash
    // and per-window session both round-trip the flip-aware layout.
    // Never-flipped panes emit nothing extra. `bm` marks "has been
    // flipped at least once" so a Hybrid with no per-side theme still
    // serializes its Hybrid-ness.
    const ht = serializeHybridTheme(n.theme);
    if (ht) out.ht = ht;
    if (n.back !== undefined) out.bm = 1;
    if (n.showingBack) out.sb = 1;
    return out;
  }
  const a = serializeNode(n.a, opts);
  const b = serializeNode(n.b, opts);
  if (!a || !b) return null;
  // Only emit `r` if the split has been resized off the 50/50
  // default. Tiny rounding kindness so the URL hash stays short.
  const rRound = Math.round(n.ratio * 1000) / 1000;
  const rField = Math.abs(rRound - 0.5) > 0.001 ? { r: rRound } : {};
  return { k: "s", d: n.direction === "row" ? "r" : "c", a, b, ...rField };
}

/// Snapshot of the layout for persistence in the URL hash. Returns
/// `null` if the layout is uninteresting (a single empty pane), so we
/// don't litter the URL when there's nothing to save.
type SerializeLayoutOptions = {
  terminalSessions?: boolean;
};

export function serializeLayout(opts: SerializeLayoutOptions = {}): SerNode | null {
  const tree = serializeNode(layout.rootId, opts);
  if (!tree) return null;
  const serialized = {
    ...tree,
    ...serializeFocusColor(layout.focusColor),
  };
  if (serialized.k === "l" && serialized.t.length === 0 && !serialized.wc) return null;
  return serialized;
}

/// Replace the live layout with the deserialized tree, then kick off a
/// content load for every tab. The DOM updates as content arrives;
/// tabs initially appear in a "loading..." state.
export async function restoreLayout(
  s: SerNode,
  sessionLayout: SerNode | null = null,
): Promise<void> {
  // Clear current state.
  for (const k of Object.keys(layout.nodes)) delete layout.nodes[k];
  layout.focusColor = restoreFocusColor(s.wc ?? sessionLayout?.wc);

  let activePaneId: string | null = null;
  const tabsToLoad: { paneId: string; tabId: string; path: string }[] = [];
  const sessionLeaves = serializedLeaves(sessionLayout);
  let leafIndex = 0;

  function build(node: SerNode): string {
    if (node.k === "l") {
      const sessionLeaf = sessionLeaves[leafIndex++] ?? null;
      const savedTerms =
        sessionLeaf?.t.filter((t) => (t.k ?? "f") === "t") ?? [];
      let termIndex = 0;
      const p: LeafNode = {
        kind: "leaf",
        id: id("pane"),
        tabs: [],
        activeTabId: null,
      };
      for (const sertab of node.t) {
        const kind = sertab.k ?? "f";
        if (kind === "g") {
          const mode = restoreGraphMode(sertab.gm);
          const scopeId = sertab.gs || "workspace";
          // Prefer `gn` (persisted live selection) as the post-restore
          // seed so the user lands on the same focal node. The graph
          // load consumes `pendingSelectId` once; `selectedNodeId`
          // stays so the tab title remains selection-driven.
          const selectedNodeId =
            typeof sertab.gn === "string" ? sertab.gn : null;
          const selectedNodeLabel =
            typeof sertab.gnl === "string" ? sertab.gnl : null;
          const tab: GraphTab = {
            kind: "graph",
            id: id("graph"),
            title: graphTitle(mode, scopeId),
            mode,
            scopeId,
            depth: Number.isFinite(sertab.gd) ? Math.max(0, Number(sertab.gd)) : 1,
            expanded: graphExpandedFromList(sertab.ge),
            filters: decodeGraphTabFilters(sertab.gf),
            inspectorOpen: sertab.gi === 1,
            pendingSelectId: sertab.gp ?? selectedNodeId,
            selectedNodeId,
            selectedNodeLabel,
            ...(typeof sertab.iw === "number" && sertab.iw > 0
              ? { inspectorWidth: sertab.iw }
              : {}),
          };
          p.tabs.push(tab);
          if (sertab.a) p.activeTabId = tab.id;
          continue;
        }
        if (kind === "b") {
          const tab: BrowserTab = {
            kind: "browser",
            id: id("browser"),
            title: "Files",
            inspectorOpen: sertab.bi === 1,
            ...(typeof sertab.bs === "string" ? { selected: sertab.bs } : {}),
            ...(sertab.bd === 1 ? { showWorkspace: true } : {}),
            ...(Array.isArray(sertab.be) && sertab.be.length > 0
              ? { expanded: sertab.be.filter((p) => typeof p === "string") }
              : {}),
            ...(typeof sertab.bsc === "number" && sertab.bsc > 0
              ? { scroll: sertab.bsc }
              : {}),
            ...(typeof sertab.iw === "number" && sertab.iw > 0
              ? { inspectorWidth: sertab.iw }
              : {}),
          };
          p.tabs.push(tab);
          if (sertab.a) p.activeTabId = tab.id;
          continue;
        }
        // Settings ("s") and health ("h") are overlays now; silently
        // drop saved entries from older sessions.
        if (kind === "t") {
          const savedTerm = savedTerms[termIndex++];
          const terminalSessionId = sertab.tsid ?? savedTerm?.tsid;
          const group = (sertab.tg ?? savedTerm?.tg)?.trim();
          // Restore the negotiated keyboard protocol for a reattaching
          // session so Shift+Enter -> newline survives a reload even when
          // the agent's original negotiation has scrolled out of replay.
          const kpSnapshot = terminalSessionId
            ? (sertab.kp ?? savedTerm?.kp)
            : undefined;
          // Restore an in-flight Rich Prompt message (GAP 2). It re-locks the
          // bubble on mount; the `session` frame's `queued_prompt_ids` then
          // re-proves it (still queued → keep + position; drained → clear) via
          // reproveRestoredPrompt.
          const pp = sertab.pp ?? savedTerm?.pp;
          // Restore a pending Team Work dialog config (#4 reload-survival). The
          // hash carries no `twk` (session-only), so a hash reload sources it
          // from the positional `savedTerm` graft, same as `tsid`. Presence
          // makes `findTeamWorkPendingLead` reopen the dialog post-restore.
          const twk = sertab.twk ?? savedTerm?.twk;
          const tab: TerminalTab = {
            kind: "terminal",
            id: id("term"),
            title: sertab.n || "Terminal",
            createdAt: Date.now(),
            broadcastEnabled: false,
            broadcastTargetIds: [],
            terminalSessionId,
            controlledTerminal: sertab.tc === 1 || savedTerm?.tc === 1,
            group: group && group !== DEFAULT_TERMINAL_GROUP ? group : undefined,
            keyboardProtocol: kpSnapshot
              ? restoreKeyboardProtocolState(kpSnapshot)
              : undefined,
            lastAgentEchoSeq:
              terminalSessionId &&
              typeof (sertab.tae ?? savedTerm?.tae) === "number" &&
              Number.isFinite(sertab.tae ?? savedTerm?.tae)
                ? Math.max(0, Math.floor((sertab.tae ?? savedTerm?.tae)!))
                : undefined,
            richPromptDraftPath: (sertab.rpd ?? savedTerm?.rpd) || undefined,
            ...(pp && (pp.ph === "sent" || pp.ph === "queued")
              ? { pendingPrompt: { id: pp.id, phase: pp.ph } }
              : {}),
            ...(twk ? { teamWorkPending: twk } : {}),
          };
          p.tabs.push(tab);
          if (sertab.a) p.activeTabId = tab.id;
          // Reshow the bubble so the restored queued message is visible +
          // actionable without re-toggling Cmd+Shift+P (GAP 2).
          if (sertab.rpv ?? savedTerm?.rpv) showRichPromptForTab(tab.id);
          continue;
        }
        if (kind === "d") {
          // Sanitize the disabled-slot set to in-range indices; ignore
          // it entirely if it would leave no slot enabled (malformed
          // hash) so the carousel can never restore blank.
          const rawDs = Array.isArray(sertab.ds)
            ? [...new Set(sertab.ds)]
                .filter(
                  (n) =>
                    Number.isInteger(n) && n >= 0 && n < DASHBOARD_SLOT_COUNT,
                )
                .sort((a, b) => a - b)
            : [];
          const disabledSlots =
            rawDs.length > 0 && rawDs.length < DASHBOARD_SLOT_COUNT
              ? rawDs
              : [];
          const tab: DashboardTab = {
            kind: "dashboard",
            id: id("dashboard"),
            title: "Dashboard",
            ...(disabledSlots.length > 0 ? { disabledSlots } : {}),
          };
          // Restore the carousel slide when the hash carries one, clamping
          // off a disabled slot to the first enabled one. Absence falls
          // back to the About slide (slot 0) unless that slot is disabled.
          if (typeof sertab.cs === "number" && sertab.cs > 0) {
            const want = Math.max(0, Math.floor(sertab.cs));
            tab.carouselSlide = dashboardSlotEnabled(tab, want)
              ? want
              : firstEnabledSlot(tab);
          } else if (!dashboardSlotEnabled(tab, 0)) {
            tab.carouselSlide = firstEnabledSlot(tab);
          }
          if (sertab.ar === false) tab.autoRotate = false;
          p.tabs.push(tab);
          if (sertab.a) p.activeTabId = tab.id;
          continue;
        }
        if (kind !== "f") continue;
        // Recompute fileKind from the path. Cheaper than persisting
        // it (the hash already carries the path) and keeps a session
        // restored after a chan upgrade aligned with the current
        // classifier instead of a stale snapshot.
        const restoredPath = sertab.p ?? "";
        const restoredPathKind = classifyPath(restoredPath);
        const restoredFileKind: FileKind =
          restoredPathKind === "document" || restoredPathKind === "text"
            ? restoredPathKind
            : "document";
        const tab: FileTab = {
          kind: "file",
          fileKind: restoredFileKind,
          id: id("tab"),
          path: restoredPath,
          content: "",
          saved: "",
          savedMtime: null,
          savedMtimeNs: null,
          // Trust the persisted mode when it is a valid pair for
          // this tab's path; otherwise fall back to the default.
          // Guards: a markdown-only "wysiwyg" mode persisted for a
          // .py file restores to source; a "pretty" persisted for a
          // non-JSON text file restores to source.
          mode: validateRestoredMode(sertab.m, restoredPath, restoredFileKind),
          loading: true,
          error: null,
          fileMissing: null,
          inspectorOpen: !!sertab.o,
          outlineOpen: !!sertab.ol,
          // repoRoot is filled in by loadTabContent on first read;
          // restored sessions start with null and get the real value
          // once the file fetches.
          repoRoot: null,
          // Restore the user-toggled read mode if it was persisted.
          // fsWritable is NOT carried in the session payload - it's
          // a disk property; the first loadTabContent refreshes it
          // (and an `!fsWritable` will dominate even if readMode is
          // false, so we don't need to fake it here).
          readMode: sertab.r === 1,
          fsWritable: true,
          // Absent means default-off; `s: 1` means the style toolbar is on.
          styleToolbarOpen: sertab.s === 1,
          // Default-on. `h: 0` in the hash means user disabled
          // highlight on this tab; any other value (absent / 1)
          // restores to default-on.
          syntaxHighlight: sertab.h !== 0,
          highlightTrailingWhitespace: false,
          codeBlocksCollapsed: false,
          // Restored caret rides through to the editor via tab.caret;
          // the editor lands it once content finishes loading.
          caret:
            Array.isArray(sertab.c) && sertab.c.length === 2
              ? { from: sertab.c[0], to: sertab.c[1] }
              : undefined,
          ...(typeof sertab.iw === "number" && sertab.iw > 0
            ? { inspectorWidth: sertab.iw }
            : {}),
          ...(typeof sertab.ow === "number" && sertab.ow > 0
            ? { outlineWidth: sertab.ow }
            : {}),
        };
        p.tabs.push(tab);
        if (sertab.a) p.activeTabId = tab.id;
        if (tab.path) {
          tabsToLoad.push({ paneId: p.id, tabId: tab.id, path: tab.path });
        }
      }
      // If no tab was marked active but there are tabs, focus the first.
      if (!p.activeTabId && p.tabs.length > 0) p.activeTabId = p.tabs[0]!.id;
      // The back is a configuration view, not a tab collection.
      // `bm` is the wire signal for "pane has been flipped at least
      // once." Legacy `bt` / `hb` are accepted but discarded; their
      // presence implies the pane was a Hybrid, so materialise the
      // empty back marker to keep `pane.back !== undefined` accurate.
      if (
        node.bm ||
        node.hb ||
        (node.bt && node.bt.length > 0)
      ) {
        p.back = {};
      }
      if (node.ht) p.theme = node.ht === "d" ? "dark" : "light";
      // The flip persists across reloads, but only for non-empty panes.
      // A stale `sb` on an empty pane is dropped defensively.
      if (node.sb && p.tabs.length > 0) p.showingBack = true;
      layout.nodes[p.id] = p;
      if (node.f) activePaneId = p.id;
      return p.id;
    }
    const split: SplitNode = {
      kind: "split",
      id: id("split"),
      direction: node.d === "r" ? "row" : "column",
      a: build(node.a),
      b: build(node.b),
      ratio: typeof node.r === "number" ? node.r : 0.5,
    };
    layout.nodes[split.id] = split;
    return split.id;
  }

  layout.rootId = build(s);
  layout.activePaneId = activePaneId ?? firstLeafId(layout.rootId);

  // Load all tab contents in parallel; failures land in tab.error.
  await Promise.all(
    tabsToLoad.map((t) => loadTabContent(t.paneId, t.tabId, t.path)),
  );
}

function serializedLeaves(node: SerNode | null, out: SerLeaf[] = []): SerLeaf[] {
  if (!node) return out;
  if (node.k === "l") {
    out.push(node);
    return out;
  }
  serializedLeaves(node.a, out);
  serializedLeaves(node.b, out);
  return out;
}

/// True when a serialized layout carries durable content — at least one
/// non-terminal tab (a file/browser/graph/hybrid/dashboard surface).
/// Terminal tabs (`k:"t"`) are ephemeral: the PTY dies on restart and a
/// saved `tsid` only respawns a fresh shell, so a window whose tabs are ALL
/// terminals (or has none) is not worth persisting as a saved window. Used
/// by `serializeSession()` so a terminal-only window deletes its blob
/// instead of saving it; mirrors the backend `session_blob_is_empty`
/// all-terminal rule (chan-workspace) that prunes existing such phantoms.
export function layoutHasDurableContent(layout: SerNode | null): boolean {
  for (const leaf of serializedLeaves(layout)) {
    // Front tabs plus legacy Hybrid back-tabs (`bt`), for parity with the
    // backend walk; `bt` is discarded on rehydrate but a stray non-terminal
    // there still counts as content we should not silently drop.
    for (const tab of [...leaf.t, ...(leaf.bt ?? [])]) {
      if ((tab.k ?? "f") !== "t") return true;
    }
  }
  return false;
}

/// True when a serialized layout has at least one terminal tab carrying a
/// `tsid` — i.e. a live server-side PTY to RE-ATTACH on reload. A terminal
/// without a tsid (not yet connected, or its session ended) has nothing to
/// reattach, so persisting a reload snapshot of it would only spawn a stray
/// fresh PTY when restored. Gates the all-terminal reload snapshot in
/// store.svelte.ts so a tsid-less terminal layout is never snapshotted.
export function layoutHasReattachableTerminal(layout: SerNode | null): boolean {
  for (const leaf of serializedLeaves(layout)) {
    for (const tab of [...leaf.t, ...(leaf.bt ?? [])]) {
      if ((tab.k ?? "f") === "t" && !!tab.tsid) return true;
    }
  }
  return false;
}

/// True when a serialized layout is worth persisting for its STRUCTURE alone,
/// even with no durable content and no reattachable PTY: a split (more than one
/// pane, so empty panes survive) or a terminal-only window. Restoring it
/// recreates the panes and spawns FRESH shells for the terminals — the PTYs are
/// gone after a restart or a workspace off->on, and the layout is what we keep.
/// Gates the on-disk session save (store.svelte.ts) so a terminal-only or
/// empty-split window no longer restores blank. A single empty pane stays
/// unpersisted (it is just the default window).
export function layoutHasPersistableStructure(layout: SerNode | null): boolean {
  if (!layout) return false;
  const leaves = serializedLeaves(layout);
  if (leaves.length > 1) return true;
  for (const leaf of leaves) {
    for (const tab of [...leaf.t, ...(leaf.bt ?? [])]) {
      if ((tab.k ?? "f") === "t") return true;
    }
  }
  return false;
}

/// Copy terminal PTY session metadata from a per-window session layout
/// onto the live layout after a shareable URL-hash layout restore.
/// The hash deliberately omits `tsid`; this graft keeps reloads
/// from abandoning the server-side PTY while still keeping copied URLs
/// free of private terminal ids.
export function hydrateTerminalSessionsFromLayout(sessionLayout: SerNode | null): void {
  const sessionLeaves = serializedLeaves(sessionLayout);
  const livePaneIds = leafIdsInOrder(layout.rootId);
  for (let i = 0; i < livePaneIds.length; i++) {
    const live = layout.nodes[livePaneIds[i]!];
    const saved = sessionLeaves[i];
    if (!live || live.kind !== "leaf" || !saved) continue;
    const liveTerms = live.tabs.filter((t): t is TerminalTab => t.kind === "terminal");
    const savedTerms = saved.t.filter((t) => (t.k ?? "f") === "t");
    for (let j = 0; j < liveTerms.length; j++) {
      const savedTerm = savedTerms[j];
      if (!savedTerm) continue;
      if (savedTerm.tsid) {
        liveTerms[j]!.terminalSessionId = savedTerm.tsid;
        liveTerms[j]!.lastAgentEchoSeq =
          typeof savedTerm.tae === "number" && Number.isFinite(savedTerm.tae)
            ? Math.max(0, Math.floor(savedTerm.tae))
            : undefined;
      }
      if (savedTerm.rpd) liveTerms[j]!.richPromptDraftPath = savedTerm.rpd;
      if (savedTerm.twk) liveTerms[j]!.teamWorkPending = savedTerm.twk;
    }
  }
}

function firstLeafId(nodeId: string): string {
  const n = layout.nodes[nodeId];
  if (!n) return layout.rootId;
  if (n.kind === "leaf") return n.id;
  return firstLeafId(n.a);
}

export async function saveTab(t: Tab): Promise<void> {
  if (t.kind !== "file") return;
  await performSave(t);
}

/// Clear the transient error banner on a file tab. Used after a
/// rename completes so a watcher race that briefly set "no such
/// file" doesn't linger past the rekey. No-op if no tab matches.
export function clearTabError(tabId: string): void {
  const found = findFileTabById(tabId);
  if (!found) return;
  if (found.tab.error) found.tab.error = null;
  if (found.tab.fileMissing) found.tab.fileMissing = null;
}

export function markTabFileMissing(tabId: string): void {
  const found = findFileTabById(tabId);
  if (!found) return;
  markFileMissing(found.tab);
  void runSuggestReopenLookup(tabId, found.tab.path);
}

/// Debounced watcher-event reaction for "Removed" / "Renamed"
/// frames on an open file's path. Atomic-write patterns (temp +
/// rename) make the file vanish for a few milliseconds before
/// reappearing under the same name; chan's own self-write
/// dedupe usually suppresses the echo but races still leak
/// through, and external editors that don't go through
/// chan-server bypass the dedupe entirely. Delay the missing-
/// check long enough for the path to come back if it's going
/// to, then resolve via a path stat.
const pendingMissingChecks = new Map<string, ReturnType<typeof setTimeout>>();
const MISSING_CHECK_DEBOUNCE_MS = 150;

export function scheduleMissingFileCheck(tabId: string, path: string): void {
  const prior = pendingMissingChecks.get(tabId);
  if (prior !== undefined) clearTimeout(prior);
  const timer = setTimeout(() => {
    pendingMissingChecks.delete(tabId);
    void resolveMissingFileCheck(tabId, path);
  }, MISSING_CHECK_DEBOUNCE_MS);
  pendingMissingChecks.set(tabId, timer);
}

/// Cancel any pending missing-file check for `tabId`. Called
/// when a subsequent watcher frame (e.g. a "Created" that
/// follows a temp + rename "Removed") confirms the file is
/// back; the check would resolve the same way but cancelling
/// avoids the extra read.
export function cancelMissingFileCheck(tabId: string): void {
  const prior = pendingMissingChecks.get(tabId);
  if (prior === undefined) return;
  clearTimeout(prior);
  pendingMissingChecks.delete(tabId);
}

async function resolveMissingFileCheck(
  tabId: string,
  path: string,
): Promise<void> {
  const found = findFileTabById(tabId);
  if (!found) return;
  // Watcher event was for a stale path that the tab no longer
  // points at (rename rekey happened in between). Drop.
  if (found.tab.path !== path) return;
  const tab = found.tab;
  if (tab.content !== tab.saved) {
    // Buffer is dirty. Don't clobber the user's in-flight
    // typing; just probe existence and clear / set fileMissing.
    try {
      await api.read(path);
      tab.fileMissing = null;
      tab.error = null;
    } catch (e) {
      if (isMissingFileError(e)) markTabFileMissing(tabId);
      // Other errors (network etc.) leave the tab as-is.
    }
    return;
  }
  // Clean buffer - full reload is safe. loadTabContent fires
  // markFileMissing on a genuine 404 in its catch branch.
  await loadTabContent(found.paneId, tabId, path);
  if (tab.fileMissing) {
    void runSuggestReopenLookup(tabId, path);
  }
}

/// Best-effort "did the file just move?" lookup. Runs after a
/// genuine missing-file detection. Searches the workspace by
/// basename + filters to exact basename matches at a path
/// different from the original; only surfaces a suggestion
/// when there's a unique candidate. Ambiguous results leave
/// `suggestedPath` null so the user is steered to Find.
async function runSuggestReopenLookup(
  tabId: string,
  path: string,
): Promise<void> {
  const basename = path.split("/").pop();
  if (!basename) return;
  let candidates: string[] = [];
  try {
    const hits = await api.search(basename, 5);
    candidates = hits
      .map((h) => h.path)
      .filter((p) => p !== path && p.split("/").pop() === basename);
  } catch {
    // Search failure is non-blocking; missing-file UX is still
    // usable without the suggestion.
    return;
  }
  const found = findFileTabById(tabId);
  if (!found || !found.tab.fileMissing) return;
  if (found.tab.path !== path) return;
  found.tab.fileMissing.suggestedPath =
    candidates.length === 1 ? candidates[0] : null;
}

/// Try to reload the missing file at its ORIGINAL path. Used
/// by the missing-file panel's Re-open button. Returns true
/// when the load succeeded (the panel goes away in that
/// branch); false when the file is still gone (caller falls
/// through to FB navigation so the user can manually pick the
/// moved file).
export async function attemptInPlaceReopen(
  tabId: string,
): Promise<boolean> {
  const found = findFileTabById(tabId);
  if (!found) return false;
  const path = found.tab.path;
  found.tab.loading = true;
  await loadTabContent(found.paneId, tabId, path);
  const after = findFileTabById(tabId);
  return after !== null && after.tab.fileMissing === null;
}

export function beginMissingFileReopen(tabId: string): void {
  const found = findFileTabById(tabId);
  if (!found || found.tab.fileMissing === null) return;
  pendingMissingFileReopenTabId = tabId;
  const node = layout.nodes[found.paneId];
  if (node?.kind === "leaf") node.activeTabId = tabId;
  layout.activePaneId = found.paneId;
}

/// Refresh a non-dirty tab's content from disk. Used by user-initiated
/// flows that intend to adopt the new disk content (e.g. file replace).
/// If the buffer is dirty, it is left alone. Not used for watcher events;
/// watcher events must not silently reload an open doc (see `flagExternalChange`).
export async function refreshTabFromDisk(tabId: string): Promise<void> {
  const found = findFileTabById(tabId);
  if (!found) return;
  if (found.tab.content !== found.tab.saved) return;
  await loadTabContent(found.paneId, found.tab.id, found.tab.path);
}

/// A watcher event reported an external (non-self) write to this open
/// file's path. Do NOT reload: replacing the doc snaps the caret to
/// 1:1 while the user is typing. Raise the dismissable "changed on disk"
/// banner instead; the user opts into the reload, or their next save
/// hits the 409 conflict modal. Applies to clean and dirty buffers
/// (self-write dedupe already drops echoes of our own writes).
export function flagExternalChange(tabId: string): void {
  const found = findFileTabById(tabId);
  if (found) found.tab.externalChange = true;
}

/// Dismiss the "changed on disk" banner without reloading (the user
/// chose to keep editing). It re-raises on the next external write.
export function dismissExternalChange(tabId: string): void {
  const found = findFileTabById(tabId);
  if (found) found.tab.externalChange = false;
}

export async function reloadTabFromDisk(tabId: string): Promise<void> {
  const found = findFileTabById(tabId);
  if (!found) return;
  await loadTabContent(found.paneId, found.tab.id, found.tab.path);
}

/// Aggregate read-only state across the whole window: true iff
/// every leaf pane has at least one file tab AND every active
/// file tab is in read mode (either user-toggled `readMode` or
/// filesystem-locked via `!fsWritable`). Walks layout.nodes so
/// callers reading this from a $derived re-run when any of those
/// fields flip. Returns false when there are no open file tabs
/// at all (an empty workspace isn't conceptually "in read mode").
export function isWindowFullyReadOnly(): boolean {
  let sawFile = false;
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    if (node.tabs.length === 0) continue;
    const active = node.tabs.find((t) => t.id === node.activeTabId);
    if (!active || active.kind !== "file") continue;
    sawFile = true;
    if (!active.readMode && active.fsWritable) return false;
  }
  return sawFile;
}

/// Look up every open tab for `path`, regardless of pane. The
/// watcher subscriber uses this to fan an external-edit event out
/// to the (potentially multiple) tabs viewing the same file.
export function tabsForPath(path: string): { paneId: string; tabId: string }[] {
  const out: { paneId: string; tabId: string }[] = [];
  for (const [paneId, node] of Object.entries(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    for (const t of node.tabs) {
      if (t.kind === "file" && t.path === path) {
        out.push({ paneId, tabId: t.id });
      }
    }
  }
  return out;
}

/// Rewrite tab paths in place after a rename/move. Handles both
/// the single-file rename (exact path match) and the directory
/// rename (every tab whose path starts with `from/`). Editor state
/// - buffer, cursor, dirty flag, savedMtime - is preserved so the
/// rename feels like a relabel rather than a close+reopen. The
/// server already moved the bytes atomically; mtime stays valid
/// for the moved file, so the next save's CAS check still works.
///
/// Tabs that were dirty stay dirty after the rename: the user's
/// unsaved buffer follows the file. If the new path doesn't accept
/// it (kind change, etc.) the next save surfaces the failure via
/// the existing error channel; we don't need to special-case here.
export function rekeyTabsForRename(from: string, to: string): void {
  const dirPrefix = `${from}/`;
  const newDirPrefix = `${to}/`;
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    for (const t of node.tabs) {
      if (t.kind !== "file") continue;
      if (t.path === from) {
        t.path = to;
      } else if (t.path.startsWith(dirPrefix)) {
        t.path = newDirPrefix + t.path.slice(dirPrefix.length);
      }
    }
  }
}
