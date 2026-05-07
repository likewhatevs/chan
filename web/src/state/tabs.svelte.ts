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

import { api } from "../api/client";
import { ApiError } from "../api/errors";
import { isMobile, isTablet } from "../api/native";
import { isEditableText } from "./fileTypes";
import { notify } from "./notify.svelte";

let nextId = 1;
function id(prefix: string): string {
  return `${prefix}-${nextId++}`;
}

export type Mode = "wysiwyg" | "source";

/// File-content tab: holds the editable buffer for a markdown file.
/// File tabs are the only tab kind today; every other surface
/// (file browser, search, graph, assistant, settings) is a
/// window-level overlay built on `OverlayShell.svelte`.
export type FileTab = {
  kind: "file";
  id: string;
  path: string;
  /// In-memory buffer; flushed on save.
  content: string;
  /// Last persisted content (for dirty detection).
  saved: string;
  /// Mtime returned by the last successful read or save. Used as
  /// the CAS token (expected_mtime) on subsequent saves so an
  /// external edit between reads is detected as a 409 conflict
  /// rather than silently overwriting the disk-side change.
  /// Null when the file didn't exist yet (saved-from-empty); the
  /// server treats Some(None) as "expecting a fresh file".
  savedMtime: number | null;
  mode: Mode;
  loading: boolean;
  error: string | null;
  /// Whether the inspector panel (outline body + future file info)
  /// is shown alongside the editor. Toggleable per tab; persisted
  /// in the URL hash.
  inspectorOpen: boolean;
  /// Enclosing git repo, relative to the drive root, for files that
  /// live inside one. Set on first load from FileResponse.repo_root;
  /// drives the per-file "git repo: <name>" scope option in the
  /// overlay picker. `null` for files outside any repo (or files
  /// whose repo coincides with the drive itself).
  repoRoot: string | null;
};

export type Tab = FileTab;

/// Display label for a tab. Used in the tab strip and serialization.
export function tabLabel(t: Tab): string {
  return t.path;
}

export type Pane = {
  id: string;
  tabs: Tab[];
  activeTabId: string | null;
};

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
    };
  })(),
);

function pane(id: string): LeafNode {
  const n = layout.nodes[id];
  if (!n || n.kind !== "leaf") throw new Error(`not a pane: ${id}`);
  return n;
}

export function activePane(): LeafNode {
  return pane(layout.activePaneId);
}

/// Fetch a file tab's content from disk and write it into the
/// (proxied) pane state. Resolves the proxied reference each time it
/// touches the tab: in Svelte 5, mutations through the original
/// object literal don't propagate to the array element.
async function loadTabContent(
  paneId: string,
  tabId: string,
  path: string,
): Promise<void> {
  const live = (): FileTab | undefined => {
    const node = layout.nodes[paneId];
    if (!node || node.kind !== "leaf") return undefined;
    const t = node.tabs.find((tab) => tab.id === tabId);
    return t && t.kind === "file" ? t : undefined;
  };
  try {
    const r = await api.read(path);
    const t = live();
    if (t) {
      t.content = r.content;
      t.saved = r.content;
      t.savedMtime = r.mtime ?? null;
      t.repoRoot = r.repo_root ?? null;
    }
  } catch (e) {
    const t = live();
    if (t) t.error = (e as Error).message;
  } finally {
    const t = live();
    if (t) t.loading = false;
  }
}

/// Open a file in a specific pane. If already open there, just focus.
export async function openInPane(paneId: string, path: string): Promise<void> {
  if (!isEditableText(path)) {
    notify(`'${path}' is not an editable text file (only .md and .txt)`);
    return;
  }
  const p = pane(paneId);
  const existing = p.tabs.find((t) => t.kind === "file" && t.path === path);
  if (existing) {
    p.activeTabId = existing.id;
    layout.activePaneId = paneId;
    return;
  }
  const newTab: FileTab = {
    kind: "file",
    id: id("tab"),
    path,
    content: "",
    saved: "",
    savedMtime: null,
    mode: "wysiwyg",
    loading: true,
    error: null,
    inspectorOpen: false,
    repoRoot: null,
  };
  p.tabs.push(newTab);
  p.activeTabId = newTab.id;
  layout.activePaneId = paneId;
  await loadTabContent(paneId, newTab.id, path);
}

export function openInActivePane(path: string): Promise<void> {
  return openInPane(layout.activePaneId, path);
}

/// Move the active pane's selection to the previous tab. Wraps from
/// the first tab back to the last (iTerm-style cycle), so repeated
/// presses keep rotating instead of dead-ending at the edges. No-op
/// when the pane is empty or the active tab is somehow not in the
/// tab list (shouldn't happen but keeps a bad state from crashing).
export function selectPrevTabInActivePane(): void {
  const p = activePane();
  if (p.tabs.length === 0 || p.activeTabId === null) return;
  const idx = p.tabs.findIndex((t) => t.id === p.activeTabId);
  if (idx < 0) return;
  const next = (idx - 1 + p.tabs.length) % p.tabs.length;
  p.activeTabId = p.tabs[next].id;
}

export function selectNextTabInActivePane(): void {
  const p = activePane();
  if (p.tabs.length === 0 || p.activeTabId === null) return;
  const idx = p.tabs.findIndex((t) => t.id === p.activeTabId);
  if (idx < 0) return;
  const next = (idx + 1) % p.tabs.length;
  p.activeTabId = p.tabs[next].id;
}

/// Select the Nth tab in the active pane (0-indexed). Silent no-op
/// when the index is out of range, matching the browser behavior of
/// Cmd+9 jumping to the last tab only when nine or more exist.
export function selectTabAtIndexInActivePane(index: number): void {
  const p = activePane();
  if (index < 0 || index >= p.tabs.length) return;
  p.activeTabId = p.tabs[index].id;
}

export function closeTab(paneId: string, tabId: string): void {
  const p = pane(paneId);
  const idx = p.tabs.findIndex((t) => t.id === tabId);
  if (idx < 0) return;
  p.tabs.splice(idx, 1);
  if (p.activeTabId === tabId) {
    p.activeTabId = p.tabs[Math.max(0, idx - 1)]?.id ?? null;
  }
  // Collapse empty pane if it has a sibling: replace parent split with sibling.
  if (p.tabs.length === 0 && layout.rootId !== p.id) {
    collapseEmptyPane(p.id);
  }
}

/// Drop every tab in every pane. Used by the M4-D mobile reset
/// flow so the editor doesn't keep showing a now-deleted file
/// after the user wipes the drive. Pane structure is left
/// alone (the workspace's split tree survives), only the tabs go.
export function closeAllTabs(): void {
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    node.tabs.length = 0;
    node.activeTabId = null;
  }
}

/// "Close pane" button. Two cases:
///   - non-root: discard all tabs and collapse the pane (sibling takes
///     the parent split's place).
///   - root pane: there must always be at least one pane on screen, so
///     just clear the tabs (returns to the empty "no file open" state).
export function closePane(paneId: string): void {
  const p = pane(paneId);
  p.tabs.length = 0;
  p.activeTabId = null;
  if (paneId !== layout.rootId) {
    collapseEmptyPane(paneId);
  }
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
  return {
    kind: "file",
    id: src.id,
    path: src.path,
    content: src.content,
    saved: src.saved,
    savedMtime: src.savedMtime,
    mode: src.mode,
    loading: src.loading,
    error: src.error,
    inspectorOpen: src.inspectorOpen,
    repoRoot: src.repoRoot,
  };
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
}


/// True when adding another split would exceed the platform's cap:
/// iPhone (mobile + non-tablet) caps at zero (no splits at all);
/// iPad (mobile + tablet) caps at one split (two panes total).
/// Desktop / web are uncapped. Used by Pane.svelte to disable the
/// split buttons + by `splitActive` as a hard guard.
export function canSplit(): boolean {
  if (!isMobile()) return true;
  if (!isTablet()) return false;
  // Tablet: allow exactly one split. We count split nodes in the
  // layout; a single split means one root SplitNode and two leaves.
  const splits = Object.values(layout.nodes).filter(
    (n): n is SplitNode => n.kind === "split",
  ).length;
  return splits === 0;
}

export function splitActive(direction: "row" | "column"): void {
  if (!canSplit()) return;
  const original = activePane();
  const newPane: LeafNode = {
    kind: "leaf",
    id: id("pane"),
    tabs: [],
    activeTabId: null,
  };
  // Find parent of original so we can replace original with a new split.
  const entries = Object.values(layout.nodes);
  const parent = entries.find(
    (n): n is SplitNode => n.kind === "split" && (n.a === original.id || n.b === original.id),
  );
  const split: SplitNode = {
    kind: "split",
    id: id("split"),
    direction,
    a: original.id,
    b: newPane.id,
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
  layout.activePaneId = newPane.id;
}

export function setActivePane(paneId: string): void {
  if (layout.nodes[paneId]?.kind === "leaf") layout.activePaneId = paneId;
}

export function setMode(tab: Tab, mode: Mode): void {
  if (tab.kind === "file") tab.mode = mode;
}

/// Whether a tab represents an unsaved buffer.
export function isDirty(t: Tab): boolean {
  return t.content !== t.saved;
}

// ---- autosave + CAS conflict prompt -------------------------------------

// Debounce window for idle autosave. Picked short enough that data loss
// from a crash is small, long enough that bursty typing doesn't
// hammer the disk + watcher.
const AUTOSAVE_DEBOUNCE_MS = 800;
const autosaveTimers = new Map<string, ReturnType<typeof setTimeout>>();

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
}>({ open: false, tabId: null, path: "", currentMtime: null });

export function dismissConflict(): void {
  conflictDialog.open = false;
  conflictDialog.tabId = null;
  conflictDialog.path = "";
  conflictDialog.currentMtime = null;
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
  dismissConflict();
  if (!tabId) return;
  const found = findFileTabById(tabId);
  if (!found) return;
  found.tab.savedMtime = currentMtime;
  await performSave(found.tab);
}

/// Single source of truth for "send this tab's content to the
/// server". Both autosave and explicit saveTab funnel through here.
/// On 409, opens the conflict dialog and returns; the dialog's
/// Reload / Overwrite buttons drive the recovery.
async function performSave(t: FileTab): Promise<void> {
  try {
    const r = await api.write(t.path, t.content, t.savedMtime);
    t.saved = t.content;
    t.savedMtime = r.mtime ?? null;
    t.error = null;
    mirrorToSiblings(t.path, t.content, t.id);
  } catch (e) {
    if (e instanceof ApiError && e.status === 409) {
      const data = e.data as { current_mtime?: number | null } | null;
      conflictDialog.open = true;
      conflictDialog.tabId = t.id;
      conflictDialog.path = t.path;
      conflictDialog.currentMtime = data?.current_mtime ?? null;
      return;
    }
    throw e;
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
    }
  }
}

/// Set of paths with unsaved changes across all panes. Used by the
/// file tree to surface the same dirty indicator the per-tab UI shows.
export function dirtyPaths(): Set<string> {
  const out = new Set<string>();
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    for (const t of node.tabs) {
      if (t.kind === "file" && t.content !== t.saved) out.add(t.path);
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
  k?: "f" | "b" | "s" | "g" | "h";
  p?: string;
  m?: Mode;
  a?: 1;
  o?: 1;
};
type SerLeaf = { k: "l"; t: SerTab[]; f?: 1 };
type SerSplit = { k: "s"; d: "r" | "c"; a: SerNode; b: SerNode; r?: number };
type SerNode = SerLeaf | SerSplit;

/// Walk the layout starting at `nodeId`, producing a serializable tree.
function serializeNode(nodeId: string): SerNode | null {
  const n = layout.nodes[nodeId];
  if (!n) return null;
  if (n.kind === "leaf") {
    const tabs: SerTab[] = n.tabs.map((t) => {
      const active = t.id === n.activeTabId ? { a: 1 as const } : {};
      // Only file tabs exist; omit `k:"f"` since "f" is the default
      // (smaller hash).
      return {
        p: t.path,
        m: t.mode,
        ...active,
        ...(t.inspectorOpen ? { o: 1 as const } : {}),
      };
    });
    return {
      k: "l",
      t: tabs,
      ...(n.id === layout.activePaneId ? { f: 1 as const } : {}),
    };
  }
  const a = serializeNode(n.a);
  const b = serializeNode(n.b);
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
export function serializeLayout(): SerNode | null {
  const tree = serializeNode(layout.rootId);
  if (!tree) return null;
  if (tree.k === "l" && tree.t.length === 0) return null;
  return tree;
}

/// Replace the live layout with the deserialized tree, then kick off a
/// content load for every tab. The DOM updates as content arrives;
/// tabs initially appear in a "loading…" state.
export async function restoreLayout(s: SerNode): Promise<void> {
  // Clear current state.
  for (const k of Object.keys(layout.nodes)) delete layout.nodes[k];

  let activePaneId: string | null = null;
  const tabsToLoad: { paneId: string; tabId: string; path: string }[] = [];

  function build(node: SerNode): string {
    if (node.k === "l") {
      const p: LeafNode = {
        kind: "leaf",
        id: id("pane"),
        tabs: [],
        activeTabId: null,
      };
      for (const sertab of node.t) {
        const kind = sertab.k ?? "f";
        // Browser ("b"), graph ("g"), settings ("s"), and health
        // ("h") used to be tab kinds that round-tripped through the
        // session. All four are overlays now; silently drop any
        // saved entries from older session.json files instead of
        // leaving the user with unrecoverable orphans.
        if (kind !== "f") continue;
        const tab: FileTab = {
          kind: "file",
          id: id("tab"),
          path: sertab.p ?? "",
          content: "",
          saved: "",
          savedMtime: null,
          mode: sertab.m ?? "wysiwyg",
          loading: true,
          error: null,
          inspectorOpen: !!sertab.o,
          // repoRoot is filled in by loadTabContent on first read;
          // restored sessions start with null and get the real value
          // once the file fetches.
          repoRoot: null,
        };
        p.tabs.push(tab);
        if (sertab.a) p.activeTabId = tab.id;
        if (tab.path) {
          tabsToLoad.push({ paneId: p.id, tabId: tab.id, path: tab.path });
        }
      }
      // If no tab was marked active but there are tabs, focus the first.
      if (!p.activeTabId && p.tabs.length > 0) p.activeTabId = p.tabs[0]!.id;
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

/// Refresh a non-dirty tab's content from disk. Called when the
/// watcher fires an event for an open file's path. If the buffer
/// is dirty we leave it alone; the user's next save will hit a 409
/// and the conflict dialog will surface the situation.
export async function refreshTabFromDisk(tabId: string): Promise<void> {
  const found = findFileTabById(tabId);
  if (!found) return;
  if (found.tab.content !== found.tab.saved) return;
  await loadTabContent(found.paneId, found.tab.id, found.tab.path);
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
