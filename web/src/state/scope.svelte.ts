// Shared scope picker for the floating overlays (search, graph).
// Both surfaces care about the same question — "what
// part of my world are we working on right now?" — and render the
// same dropdown:
//
//   - one entry per file currently visible in any leaf pane
//     (file scope; the most common pick),
//   - a "all N visible files" entry when 2+ files are visible
//     (group scope; same key for the same set so a re-arrange
//     doesn't fragment state),
//   - a "directory" entry when the active browser tab has a
//     directory selected (dir scope; a subtree of the drive narrower
//     than the drive but broader than a single file),
//   - a "git repo" entry per repo covering visible files (git_repo
//     scope; a project subset of the drive),
//   - a "whole drive" entry that always exists (drive scope),
//   - a "global" entry for cross-drive scope (every chan drive
//     this user has touched). Surfaced as a placeholder for now;
//     enabled flips on once backend cross-drive indexing exists.
//
// The id format is the discriminator the consumer stores:
// `file:<path>`, `group:<key>`, `dir:<path>`, `git_repo:<root>`,
// `drive`, or `global`. Each consumer holds the chosen id (e.g. a
// graph tab's `scopeId`); the helpers here turn it into a typed
// ScopeOption.

import { layout } from "./tabs.svelte";
// The `dir` scope reads the file browser's current selection and looks
// the entry up in the tree to distinguish directory from file.
// store.svelte imports from this module too, so the cycle is real;
// both reads happen lazily inside functions (not at module init),
// which Vite resolves cleanly.
import { browserSelection, tree } from "./store.svelte";

/// Picker option, as a discriminated union so consumers can
/// pattern-match on `kind` and access the kind-specific fields
/// (path for file/dir, repo path for git_repo, key+paths for
/// group, nothing extra for drive or global). `enabled` defaults
/// true; consumers render it as disabled in the dropdown when
/// false (e.g. global before cross-drive indexing ships).
export type ScopeOption =
  | {
      id: string;
      kind: "file";
      label: string;
      path: string;
      enabled?: boolean;
      /// True when the underlying tab is read-only (filesystem-
      /// locked or user-toggled). Read-only files stay searchable
      /// and visible; consumers can mark them in their dropdowns.
      readOnly?: boolean;
    }
  | {
      id: string;
      kind: "dir";
      label: string;
      /// Directory path relative to the drive root. Empty string
      /// means the drive root itself; consumers should treat that
      /// case the same as `drive` scope.
      path: string;
      enabled?: boolean;
    }
  | {
      id: string;
      kind: "git_repo";
      label: string;
      /// Repo path relative to the drive root.
      root: string;
      enabled?: boolean;
    }
  | {
      id: string;
      kind: "group";
      label: string;
      key: string;
      paths: string[];
      enabled?: boolean;
    }
  | {
      id: string;
      kind: "tag";
      label: string;
      /// Graph node id of the tag (e.g. `#search`). The graph
      /// scoping logic seeds BFS from this id directly — no need to
      /// resolve to a path list like the file-kind scopes do.
      nodeId: string;
      enabled?: boolean;
    }
  | { id: "drive"; kind: "drive"; label: string; enabled?: boolean }
  | { id: "global"; kind: "global"; label: string; enabled?: boolean };

/// Stable group key from a list of paths: sorted + joined with `|`
/// so two groups with the same set produce the same key. Used to
/// detect "the same group as before" across layout shuffles.
export function scopeKey(paths: readonly string[]): string {
  return [...paths].sort().join("|");
}

/// Drive-relative parent directory of `path`. Returns "" for paths
/// at the drive root (no parent) and for the empty string. Directories
/// follow the same rule as files; the caller decides whether to
/// treat the empty parent as "drive scope" or skip.
export function parentDir(path: string): string {
  const i = path.lastIndexOf("/");
  return i === -1 ? "" : path.slice(0, i);
}

/// Longest common parent directory across `paths`. Drives the
/// auto-added "common ancestor" scope option per request.md when
/// multiple files share an enclosing directory. Returns "" when the
/// paths have no shared ancestor below the drive root.
export function commonAncestor(paths: readonly string[]): string {
  if (paths.length === 0) return "";
  const first = paths[0]!.split("/");
  first.pop();
  let prefix: string[] = first;
  for (let i = 1; i < paths.length && prefix.length > 0; i++) {
    const segs = paths[i]!.split("/");
    segs.pop();
    let j = 0;
    while (j < prefix.length && j < segs.length && prefix[j] === segs[j]) j++;
    prefix = prefix.slice(0, j);
  }
  return prefix.join("/");
}

/// Paths for every file tab currently active in any leaf pane.
/// Returns each path at most once, sorted alphabetically. Drives
/// every overlay's "context dropdown" + the cleanup pass that
/// prunes group state whose context no longer exists.
export function visibleFilePaths(): string[] {
  const out = new Set<string>();
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    const active = node.tabs.find((t) => t.id === node.activeTabId);
    if (active?.kind === "file" && active.path) out.add(active.path);
  }
  return [...out].sort();
}

/// Lookup the read-only state of a path among the active tabs.
/// True when at least one open tab on this path is in read mode
/// (user-toggled or filesystem-locked); false when no open tab
/// claims the path or every open tab is writable. Drives the
/// `(read-only)` tag on file scope options.
function pathIsReadOnly(path: string): boolean {
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    const active = node.tabs.find((t) => t.id === node.activeTabId);
    if (!active || active.kind !== "file" || active.path !== path) continue;
    if (active.readMode || !active.fsWritable) return true;
  }
  return false;
}

/// Path of the directory currently selected in the file browser
/// (dock or tab), or `null` when there's no selection or the
/// selection is a file rather than a directory. Drives the
/// `dir:<path>` scope option and `defaultScopeId`'s browser-aware
/// branch. (The browser overlay was retired; the shared
/// `browserSelection` is the selection signal now.)
export function selectedDirPath(): string | null {
  const path = browserSelection.path;
  if (!path) return null;
  // The tree entry tells us whether the selection is a directory.
  // Missing entry: drop the option rather than mis-categorize.
  const entry = tree.entries.find((e) => e.path === path);
  if (!entry || !entry.is_dir) return null;
  return path;
}

/// Distinct git repo roots covered by the currently visible files.
/// Each entry is a relative path under the drive root. Drives the
/// "git repo: <name>" entry in the overlay scope picker: a file
/// that lives inside a git repo (Sentinel-only file when the user
/// has chosen the drive's chan-marked directory, or git-repo files
/// when nested) gets a project-bound scope option. Files outside
/// any repo contribute nothing here.
export function visibleRepoRoots(): string[] {
  const out = new Set<string>();
  for (const node of Object.values(layout.nodes)) {
    if (node.kind !== "leaf") continue;
    const active = node.tabs.find((t) => t.id === node.activeTabId);
    if (active && active.kind === "file" && active.repoRoot) {
      out.add(active.repoRoot);
    }
  }
  return [...out].sort();
}

/// Build the dropdown options from the current layout. Each
/// overlay supplies its own label for the "drive" entry (the
/// other scope kinds are derived from the layout and need no
/// per-surface customization). Pass a `global` entry to surface
/// the cross-drive scope as the broadest pick; pass
/// `enabled: false` to render it as a disabled "coming soon"
/// row until backend cross-drive support lands.
///
/// Order in the returned list (narrow → broad): individual files,
/// group of all visible files, git repos covering visible files
/// (when applicable), drive, global. The picker renders them in
/// this order so "narrower" picks land at the top where the
/// keyboard cursor naturally falls.
export function availableScopeOptions(opts: {
  driveLabel: string;
  global?: { label: string; enabled?: boolean };
}): ScopeOption[] {
  const files = visibleFilePaths();
  const out: ScopeOption[] = files.map((path) => {
    const readOnly = pathIsReadOnly(path);
    return {
      id: `file:${path}`,
      kind: "file",
      label: readOnly ? `${path} (read-only)` : path,
      path,
      readOnly,
    };
  });
  if (files.length >= 2) {
    const key = scopeKey(files);
    out.push({
      id: `group:${key}`,
      kind: "group",
      label: `all ${files.length} visible files`,
      key,
      paths: files,
    });
  }
  const dirPath = selectedDirPath();
  if (dirPath) {
    const slash = dirPath.lastIndexOf("/");
    const name = slash >= 0 ? dirPath.slice(slash + 1) : dirPath;
    out.push({
      id: `dir:${dirPath}`,
      kind: "dir",
      label: `directory: ${name}/`,
      path: dirPath,
    });
  }
  // Auto-derived dir scopes — per request.md, when the scope is a
  // single .md file include its parent directory, and when multiple
  // files share the scope (group) include their first common
  // ancestor. Both surface as `dir:<path>` so the consumer doesn't
  // care how the option got into the list.
  const dirScopes = new Set<string>(
    out.filter((o) => o.kind === "dir").map((o) => o.id),
  );
  function pushDir(path: string, prefix: string): void {
    if (!path) return;
    const id = `dir:${path}`;
    if (dirScopes.has(id)) return;
    dirScopes.add(id);
    const slash = path.lastIndexOf("/");
    const name = slash >= 0 ? path.slice(slash + 1) : path;
    out.push({
      id,
      kind: "dir",
      label: `${prefix}: ${name}/`,
      path,
    });
  }
  for (const path of files) pushDir(parentDir(path), "parent dir");
  if (files.length >= 2) pushDir(commonAncestor(files), "common ancestor");
  for (const root of visibleRepoRoots()) {
    // Display label: just the repo's basename (the rightmost
    // path segment) since the path is relative to the drive
    // and the user already knows which drive they're in.
    const slash = root.lastIndexOf("/");
    const name = slash >= 0 ? root.slice(slash + 1) : root;
    out.push({
      id: `git_repo:${root}`,
      kind: "git_repo",
      label: `git repo: ${name}`,
      root,
    });
  }
  out.push({ id: "drive", kind: "drive", label: opts.driveLabel });
  if (opts.global) {
    out.push({
      id: "global",
      kind: "global",
      label: opts.global.label,
      enabled: opts.global.enabled ?? true,
    });
  }
  return out;
}

/// Pick a default scope id matching what's "in front of" the user
/// right now: the active pane's active file when it's a file tab,
/// the selected file or directory when the active tab is a browser,
/// else "drive" (always present). Shared between every overlay's
/// open-from-toolbar entry point and global keybinding so both
/// snap to the same pick.
export function defaultScopeId(): string {
  // Active pane is a file browser: its current selection wins (the
  // user is looking at the tree, so route the next overlay action at
  // the selected dir/file).
  const activeNode = layout.nodes[layout.activePaneId];
  const activeTab =
    activeNode && activeNode.kind === "leaf"
      ? activeNode.tabs.find((tab) => tab.id === activeNode.activeTabId)
      : undefined;
  if (activeTab?.kind === "browser") {
    const sel = browserSelection.path;
    if (sel) {
      const entry = tree.entries.find((e) => e.path === sel);
      if (entry?.is_dir) return `dir:${sel}`;
      if (entry && !entry.is_dir) return `file:${sel}`;
    }
  }
  // Two or more leaf panes with distinct active files: the group
  // scope is the natural "everything in front of me" pick. Without
  // this, opening an overlay from a multi-pane layout would default
  // to whichever pane happened to be focused last, hiding the other
  // panes' files from the result set.
  const visible = visibleFilePaths();
  if (visible.length >= 2) return `group:${scopeKey(visible)}`;
  // Otherwise: the active pane's active file.
  if (activeTab?.kind === "file" && activeTab.path) return `file:${activeTab.path}`;
  return "drive";
}
