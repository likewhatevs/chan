<script lang="ts">
  // Recursive tree view of the drive.
  //
  // Builds a nested folder structure from the flat tree the API returns,
  // then renders rows with expand/collapse, click-to-open, and a context
  // menu for create/rename/delete.

  import type { TreeEntry } from "../api/types";
  import { isEditableText } from "../state/fileTypes";
  import { dirtyPaths, openInActivePane } from "../state/tabs.svelte";
  import {
    browserSelection,
    fileOps,
    persistTreeExpanded,
    tree,
    treeExpanded,
  } from "../state/store.svelte";

  // Mime type recognized by Pane.onDrop. Keep in sync with Pane.svelte.
  const FILE_DRAG_MIME = "application/x-md-file";

  // Per-file unsaved-buffer indicator. Color comes from --info-text
  // in the global palette (see App.svelte).
  const editorDirty = $derived(dirtyPaths());

  function onFileDragStart(e: DragEvent, path: string): void {
    if (!e.dataTransfer) return;
    e.dataTransfer.effectAllowed = "copy";
    e.dataTransfer.setData(FILE_DRAG_MIME, JSON.stringify({ path }));
    // A plain-text fallback is friendly to other drop targets (e.g.
    // pasting the path into a code editor outside the app).
    e.dataTransfer.setData("text/plain", path);
  }

  type Folder = {
    kind: "dir";
    name: string;
    path: string;
    children: Node[];
  };
  type File = {
    kind: "file";
    name: string;
    path: string;
    size: number;
    mtime: number | null;
  };
  type Node = Folder | File;

  // Shared across all browser tabs and tab-switch unmounts. See the
  // store module for the rationale.
  const expanded = treeExpanded.map;
  let menu = $state<{ x: number; y: number; path: string; isDir: boolean } | null>(null);

  const root = $derived<Folder>(buildTree(tree.entries));

  /// Visible row index by path, in display order. Walked the same
  /// way the renderer walks (pre-order, recursing into folders that
  /// are currently expanded). Drives zebra striping: even rows get
  /// the default background, odd rows pick up `--zebra-bg`. The map
  /// rebuilds whenever the tree or the expansion set changes; for
  /// thousands of nodes this is still cheaper than the layout pass
  /// that follows.
  const rowIndexByPath = $derived.by(() => {
    const m = new Map<string, number>();
    let i = 0;
    function walk(nodes: Node[]): void {
      for (const n of nodes) {
        m.set(n.path, i++);
        if (n.kind === "dir" && expanded[n.path]) walk(n.children);
      }
    }
    walk(root.children);
    return m;
  });

  function buildTree(entries: TreeEntry[]): Folder {
    const root: Folder = { kind: "dir", name: "", path: "", children: [] };
    const dirs = new Map<string, Folder>([["", root]]);
    for (const e of entries) {
      const parts = e.path.split("/");
      const name = parts.pop()!;
      const parentPath = parts.join("/");
      let parent = dirs.get(parentPath);
      if (!parent) {
        parent = ensureDir(root, dirs, parentPath);
      }
      if (e.is_dir) {
        const dir: Folder = { kind: "dir", name, path: e.path, children: [] };
        parent.children.push(dir);
        dirs.set(e.path, dir);
      } else {
        parent.children.push({
          kind: "file",
          name,
          path: e.path,
          size: e.size,
          mtime: e.mtime,
        });
      }
    }
    sortRecursive(root);
    return root;
  }

  function ensureDir(
    root: Folder,
    dirs: Map<string, Folder>,
    path: string,
  ): Folder {
    if (dirs.has(path)) return dirs.get(path)!;
    const parts = path.split("/");
    let cur = root;
    let acc = "";
    for (const p of parts) {
      acc = acc ? `${acc}/${p}` : p;
      let next = dirs.get(acc);
      if (!next) {
        next = { kind: "dir", name: p, path: acc, children: [] };
        cur.children.push(next);
        dirs.set(acc, next);
      }
      cur = next;
    }
    return cur;
  }

  function sortRecursive(f: Folder): void {
    f.children.sort((a, b) => {
      if (a.kind !== b.kind) return a.kind === "dir" ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    for (const c of f.children) if (c.kind === "dir") sortRecursive(c);
  }

  function toggle(path: string): void {
    expanded[path] = !expanded[path];
    persistTreeExpanded();
  }

  async function onOpen(path: string): Promise<void> {
    await openInActivePane(path);
  }

  /// Single-click selects an entry; the FileBrowserTab side panel
  /// then renders its details. Files no longer auto-open on click;
  /// double-click (or the Open button in the panel) is the path
  /// to actually opening a file.
  function selectPath(path: string): void {
    browserSelection.path = path;
  }

  function showMenu(ev: MouseEvent, path: string, isDir: boolean): void {
    ev.preventDefault();
    menu = { x: ev.clientX, y: ev.clientY, path, isDir };
  }

  async function newFile(parentPath: string): Promise<void> {
    await fileOps.createFile(parentPath);
    menu = null;
  }
  async function newDir(parentPath: string): Promise<void> {
    await fileOps.createDir(parentPath);
    menu = null;
  }
  async function rename(path: string, isDir: boolean): Promise<void> {
    await fileOps.rename(path, isDir);
    menu = null;
  }
  async function remove(path: string): Promise<void> {
    await fileOps.remove(path);
    menu = null;
  }
</script>

<svelte:window onclick={() => (menu = null)} />

<ul class="tree" role="tree">
  {#each root.children as node (node.path)}
    {@render renderNode(node, 0)}
  {/each}
  {#if root.children.length === 0}
    <li class="empty">
      <div class="empty-title">No files</div>
      <div class="empty-actions">
        <button onclick={() => fileOps.createFile("")}>Create new file</button>
        <button onclick={() => fileOps.createDir("")}>Create new folder</button>
      </div>
    </li>
  {/if}
</ul>

{#snippet renderNode(node: Node, depth: number)}
  {@const rowIndex = rowIndexByPath.get(node.path) ?? 0}
  <li>
    {#if node.kind === "dir"}
      <div
        class="row dir"
        class:selected={browserSelection.path === node.path}
        class:zebra={rowIndex % 2 === 1}
        style="padding-left: {depth * 12}px"
        oncontextmenu={(e) => showMenu(e, node.path, true)}
        role="treeitem"
        tabindex="-1"
        aria-expanded={!!expanded[node.path]}
        aria-selected={browserSelection.path === node.path}
      >
        <button class="twirl" onclick={() => toggle(node.path)}>
          {expanded[node.path] ? "▾" : "▸"}
        </button>
        <!-- Click on folder name: toggle expand AND select. Selecting
             keeps the side panel synced with what the user is
             investigating; toggling preserves the existing browse
             affordance. -->
        <span
          class="name"
          onclick={() => { toggle(node.path); selectPath(node.path); }}
          onkeydown={() => {}}
          role="button"
          tabindex="0"
        >{node.name}/</span>
      </div>
      {#if expanded[node.path]}
        <ul class="children">
          {#each node.children as child (child.path)}
            {@render renderNode(child, depth + 1)}
          {/each}
        </ul>
      {/if}
    {:else}
      {@const editable = isEditableText(node.path)}
      <div
        class="row file"
        class:selected={browserSelection.path === node.path}
        class:non-editable={!editable}
        class:zebra={rowIndex % 2 === 1}
        style="padding-left: {depth * 12 + 16}px"
        oncontextmenu={(e) => showMenu(e, node.path, false)}
        role="treeitem"
        tabindex="-1"
        aria-selected={browserSelection.path === node.path}
        draggable="true"
        ondragstart={(e) => onFileDragStart(e, node.path)}
        title={editable ? undefined : "view-only (not an editable text file)"}
      >
        <!-- Single click selects (mirrors graph tab semantics);
             double click opens. Both stop propagation so the row's
             implicit focus / drag handlers don't double-fire.
             Non-editable files never bind dblclick so the gesture
             can't even attempt to open a binary file in the editor. -->
        <button
          class="name"
          onclick={() => selectPath(node.path)}
          ondblclick={editable ? () => void onOpen(node.path) : undefined}
        >{node.name}</button>
        {#if editorDirty.has(node.path)}
          <span class="dirty-dot unsaved" title="unsaved changes" aria-label="unsaved">●</span>
        {/if}
      </div>
    {/if}
  </li>
{/snippet}

{#if menu}
  <div class="ctx" style="left: {menu.x}px; top: {menu.y}px">
    {#if menu.isDir}
      <button onclick={() => newFile(menu!.path)}>
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2 1.75C2 .784 2.784 0 3.75 0h5.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 12.25 16h-8.5A1.75 1.75 0 0 1 2 14.25V1.75zm1.75-.25a.25.25 0 0 0-.25.25v12.5c0 .138.112.25.25.25h8.5a.25.25 0 0 0 .25-.25V6h-2.75A1.75 1.75 0 0 1 8 4.25V1.5H3.75zM9.5 1.5v2.75c0 .138.112.25.25.25h2.5l-2.75-3z" />
        </svg>
        <span>New file</span>
      </button>
      <button onclick={() => newDir(menu!.path)}>
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5l-1.4-1.55A1.75 1.75 0 0 0 4.81 1H1.75z" />
        </svg>
        <span>New folder</span>
      </button>
    {/if}
    <button onclick={() => rename(menu!.path, menu!.isDir)}>
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M11.013 1.427a1.75 1.75 0 0 1 2.474 0l1.086 1.086a1.75 1.75 0 0 1 0 2.474l-8.61 8.61c-.21.21-.47.364-.756.445l-3.251.93a.75.75 0 0 1-.927-.928l.929-3.25a1.75 1.75 0 0 1 .445-.758l8.61-8.61zm.585.745L3.45 10.32a.25.25 0 0 0-.064.108l-.558 1.953 1.953-.558a.25.25 0 0 0 .108-.064l8.148-8.147a.25.25 0 0 0 0-.354l-1.086-1.086a.25.25 0 0 0-.353 0z" />
      </svg>
      <span>Rename / Move</span>
    </button>
    <button class="danger" onclick={() => remove(menu!.path)}>
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M11 1.75V3h2.25a.75.75 0 0 1 0 1.5H2.75a.75.75 0 0 1 0-1.5H5V1.75C5 .784 5.784 0 6.75 0h2.5C10.216 0 11 .784 11 1.75zM4.496 6.675l.66 6.6a.25.25 0 0 0 .249.225h5.19a.25.25 0 0 0 .249-.225l.66-6.6a.75.75 0 0 1 1.492.149l-.66 6.6A1.748 1.748 0 0 1 10.595 15h-5.19a1.75 1.75 0 0 1-1.741-1.575l-.66-6.6a.75.75 0 1 1 1.492-.15zM6.5 1.75v1.25h3V1.75a.25.25 0 0 0-.25-.25h-2.5a.25.25 0 0 0-.25.25z" />
      </svg>
      <span>Delete</span>
    </button>
  </div>
{/if}

<style>
  .tree, .children {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    font-size: 15px;
    color: var(--text);
  }
  /* Zebra striping: subtle alternating tint so the eye can track
     across long lists. Listed before :hover and .selected so those
     stronger states override at the same specificity. */
  .row.zebra { background: var(--zebra-bg); }
  .row:hover { background: var(--hover-bg); }
  .twirl {
    background: none;
    border: 0;
    cursor: pointer;
    width: 14px;
    height: 14px;
    padding: 0;
    color: var(--text-secondary);
  }
  .name {
    background: none;
    border: 0;
    cursor: pointer;
    padding: 0;
    text-align: left;
    flex: 1;
    color: inherit;
    font: inherit;
  }
  /* View-only files (PNG, etc.): dim and italicize so the user can
     tell at a glance that these won't open in the editor. The future
     media browser will be the first-class surface for them. */
  .row.non-editable > .name {
    color: var(--text-secondary);
    font-style: italic;
    cursor: default;
  }
  /* Selection highlight. Reads as a soft band rather than the full
     accent color so it doesn't fight the dirty / git status dots
     that share the row. */
  .row.selected { background: var(--hover-bg); }
  .row.selected > .name { color: var(--text); font-weight: 600; }
  /* Same look as the tab-strip dirty indicator so the two views agree. */
  .dirty-dot {
    font-size: 16px;
    line-height: 1;
    margin-left: 4px;
  }
  .dirty-dot.unsaved { color: var(--info-text); }
  .empty {
    color: var(--text-secondary);
    padding: .75rem .5rem;
    text-align: center;
  }
  .empty-title {
    color: var(--text);
    font-weight: 600;
    margin-bottom: .35rem;
    font-size: 15px;
  }
  .empty-actions {
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: center;
  }
  .empty-actions button {
    background: none;
    border: 0;
    color: var(--link);
    cursor: pointer;
    font: inherit;
    font-size: 14px;
    padding: 2px 4px;
  }
  .empty-actions button:hover { text-decoration: underline; }
  .ctx {
    position: fixed;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,.4);
    z-index: 10000;
    display: flex;
    flex-direction: column;
    min-width: 180px;
  }
  .ctx button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: .35rem .6rem;
    background: none;
    border: 0;
    text-align: left;
    cursor: pointer;
    color: inherit;
    font: inherit;
  }
  .ctx button:hover { background: var(--hover-bg); }
  .ctx button.danger { color: var(--warn-text); }
  .ctx svg {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    fill: currentColor;
    color: var(--text-secondary);
  }
  .ctx button.danger svg { color: var(--warn-text); }
</style>
