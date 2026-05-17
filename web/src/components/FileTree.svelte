<script lang="ts">
  // Recursive tree view of the drive.
  //
  // Builds a nested folder structure from the flat tree the API returns,
  // then renders rows with expand/collapse, click-to-open, and a context
  // menu for create/rename/delete.

  import {
    ChevronDown,
    ChevronRight,
    FilePlus,
    Folder,
    FolderOpen,
    FolderPlus,
    Network,
    Pencil,
    Search,
    Trash2,
  } from "lucide-svelte";
  import { clampMenu } from "./menuClamp";
  import type { TreeEntry } from "../api/types";
  import { isEditableText } from "../state/fileTypes";
  import { classifyFile, iconFor } from "../state/kinds";
  import { dirtyPaths, openInActivePane } from "../state/tabs.svelte";
  import {
    browserOverlay,
    browserSelection,
    fileOps,
    loadTreeDir,
    openFsGraphForDirectory,
    openFsGraphForFile,
    openSearchForDirectory,
    openSearchForFile,
    persistTreeExpanded,
    tree,
    treeExpanded,
  } from "../state/store.svelte";

  // Mime type recognized by Pane.onDrop. Keep in sync with Pane.svelte.
  const FILE_DRAG_MIME = "application/x-md-file";
  // Mime type used for intra-tree moves. Separate from FILE_DRAG_MIME
  // so Pane.onDrop (open-in-pane) does not pick up folder drags, and
  // so tree drops only react to drags that originated in the tree.
  const TREE_MOVE_MIME = "application/x-chan-tree-move";

  // Per-file unsaved-buffer indicator. Color comes from --info-text
  // in the global palette (see App.svelte).
  const editorDirty = $derived(dirtyPaths());

  // Path of the row currently highlighted as a drop target during DnD.
  // Empty string means the root <ul> (drop at drive root). null means
  // no row is being hovered.
  let dropTarget = $state<string | null>(null);

  function onFileDragStart(e: DragEvent, path: string, isDir: boolean): void {
    if (!e.dataTransfer) return;
    e.dataTransfer.effectAllowed = isDir ? "move" : "copyMove";
    const payload = JSON.stringify({ path, isDir });
    e.dataTransfer.setData(TREE_MOVE_MIME, payload);
    if (!isDir) {
      // Files are also droppable into editor panes (open in tab).
      // Folders are not, so they only carry the tree-move mime.
      e.dataTransfer.setData(FILE_DRAG_MIME, JSON.stringify({ path }));
    }
    // A plain-text fallback is friendly to other drop targets (e.g.
    // pasting the path into a code editor outside the app).
    e.dataTransfer.setData("text/plain", path);
  }

  /// Resolve the move source from a DragEvent. Returns null if the
  /// drag did not originate in the tree (e.g. external file drop).
  function readTreeDrag(e: DragEvent): { path: string; isDir: boolean } | null {
    const raw = e.dataTransfer?.getData(TREE_MOVE_MIME);
    if (!raw) return null;
    try {
      const v = JSON.parse(raw) as { path: string; isDir: boolean };
      if (typeof v.path === "string") return v;
    } catch {
      // fall through
    }
    return null;
  }

  /// True when dropping `src` into `destDir` is a no-op or invalid:
  /// same parent already, dropping a folder into itself or a
  /// descendant, or dropping at the same location.
  function isInvalidDrop(src: { path: string; isDir: boolean }, destDir: string): boolean {
    if (src.path === destDir) return true;
    if (src.isDir && (destDir === src.path || destDir.startsWith(`${src.path}/`))) {
      return true;
    }
    const srcParent = src.path.includes("/")
      ? src.path.slice(0, src.path.lastIndexOf("/"))
      : "";
    return srcParent === destDir;
  }

  /// Compute the target path for dropping `src` into `destDir`.
  /// destDir == "" means the drive root.
  function dropTargetPath(src: string, destDir: string): string {
    const base = src.split("/").pop() ?? src;
    return destDir === "" ? base : `${destDir}/${base}`;
  }

  function onRowDragOver(e: DragEvent, destDir: string): void {
    // Only react if the drag contains our tree-move payload. We can't
    // call getData() in dragover (only in drop), so probe types[] for
    // the mime we set in dragstart.
    if (!e.dataTransfer?.types.includes(TREE_MOVE_MIME)) return;
    e.preventDefault();
    // Stop the event from bubbling to the root <ul>'s ondragover,
    // which would otherwise overwrite our selection with the root.
    e.stopPropagation();
    e.dataTransfer.dropEffect = "move";
    dropTarget = destDir;
  }

  function onRowDragLeave(destDir: string): void {
    // Clear only if we're leaving the row we currently highlight, so
    // a child row's dragenter doesn't briefly unhighlight its parent.
    if (dropTarget === destDir) dropTarget = null;
  }

  async function onRowDrop(e: DragEvent, destDir: string): Promise<void> {
    dropTarget = null;
    const src = readTreeDrag(e);
    if (!src) return;
    e.preventDefault();
    e.stopPropagation();
    if (isInvalidDrop(src, destDir)) return;
    const target = dropTargetPath(src.path, destDir);
    await fileOps.moveTo(src.path, target);
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

  /// `<ul>` element handles keyboard navigation. Focused at mount
  /// so arrows / Enter are live as soon as the browser opens; the
  /// host overlay also re-focuses it on open via `focusTree`.
  let treeRootEl: HTMLUListElement | undefined = $state();

  /// Row -> DOM element map, populated via `bind:this` on each row.
  /// Used to scroll the active selection into view after keyboard
  /// movement so long lists don't lose the cursor off-screen.
  const rowEls = new Map<string, HTMLElement>();

  const root = $derived<Folder>(buildTree(tree.entries));

  /// Paths of contact-kind files (those with `chan.kind: contact`
  /// frontmatter). The server sets the discriminator on the listing
  /// payload; we lift it into a set so the row renderer doesn't have
  /// to walk the flat entries every time.
  const contactPaths = $derived<Set<string>>(
    new Set(
      tree.entries
        .filter((e) => !e.is_dir && e.kind === "contact")
        .map((e) => e.path),
    ),
  );

  /// Visible-row list in display order. Mirrors the walk in
  /// `rowIndexByPath` but keeps the per-node depth + isDir bits we
  /// need for keyboard navigation. Recomputed when the tree or the
  /// expansion set changes.
  type VisibleRow = { path: string; isDir: boolean; depth: number };
  const visibleRows = $derived.by(() => {
    const rows: VisibleRow[] = [];
    function walk(nodes: Node[], depth: number): void {
      for (const n of nodes) {
        rows.push({ path: n.path, isDir: n.kind === "dir", depth });
        if (n.kind === "dir" && expanded[n.path]) walk(n.children, depth + 1);
      }
    }
    walk(root.children, 0);
    return rows;
  });

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

  $effect(() => {
    const pending: string[] = [];
    function collect(nodes: Node[]): void {
      for (const n of nodes) {
        if (n.kind !== "dir" || !expanded[n.path]) continue;
        if (!tree.loadedDirs[n.path] && !tree.loadingDirs[n.path]) {
          pending.push(n.path);
        }
        collect(n.children);
      }
    }
    collect(root.children);
    for (const path of pending) void loadTreeDir(path);
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
    setExpanded(path, !expanded[path]);
  }

  function setExpanded(path: string, value: boolean): void {
    expanded[path] = value;
    persistTreeExpanded();
    if (value) void loadTreeDir(path);
  }

  async function onOpen(path: string): Promise<void> {
    await openInActivePane(path);
    // The user wanted to read or edit the file, not keep the picker
    // hovering over the editor. Mirrors the inspector's "Open"
    // button behaviour in FileBrowserOverlay.openSelected().
    if (browserOverlay.open) browserOverlay.open = false;
  }

  /// Single-click selects an entry; the FileBrowserTab side panel
  /// then renders its details. Files no longer auto-open on click;
  /// double-click (or the Open button in the panel) is the path
  /// to actually opening a file.
  ///
  /// A click also forces the inspector open: the user expects the
  /// metadata pane to surface the selection. If they explicitly
  /// closed the inspector earlier and then click another row, the
  /// click takes precedence (and reopens it on next reload via the
  /// URL hash).
  function selectPath(path: string): void {
    browserSelection.path = path;
    browserOverlay.inspectorOpen = true;
  }

  function showMenu(ev: MouseEvent, path: string, isDir: boolean): void {
    ev.preventDefault();
    // Stop the FileBrowserOverlay's drive-actions context menu from
    // also firing — row right-click stays row-scoped (rename, delete,
    // new under here); only empty-area right-clicks reach the parent.
    ev.stopPropagation();
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
  async function remove(path: string, isDir: boolean): Promise<void> {
    await fileOps.remove(path, isDir);
    menu = null;
  }

  function graphThis(path: string, isDir: boolean): void {
    menu = null;
    if (isDir) openFsGraphForDirectory(path);
    else openFsGraphForFile(path);
  }

  function searchThis(path: string, isDir: boolean): void {
    menu = null;
    if (isDir) openSearchForDirectory(path);
    else openSearchForFile(path);
  }

  /// Move the selection by one row in the visible list. Wraps
  /// nothing: stops at the ends. Scrolls the new row into view.
  function moveSelection(delta: number): void {
    const rows = visibleRows;
    if (rows.length === 0) return;
    const cur = browserSelection.path;
    const idx = cur ? rows.findIndex((r) => r.path === cur) : -1;
    let next: number;
    if (idx === -1) {
      next = delta > 0 ? 0 : rows.length - 1;
    } else {
      next = Math.max(0, Math.min(rows.length - 1, idx + delta));
    }
    const target = rows[next];
    if (!target) return;
    browserSelection.path = target.path;
    queueScrollIntoView(target.path);
  }

  function moveToFirst(): void {
    const rows = visibleRows;
    if (rows.length === 0) return;
    browserSelection.path = rows[0].path;
    queueScrollIntoView(rows[0].path);
  }

  function moveToLast(): void {
    const rows = visibleRows;
    if (rows.length === 0) return;
    browserSelection.path = rows[rows.length - 1].path;
    queueScrollIntoView(rows[rows.length - 1].path);
  }

  function queueScrollIntoView(path: string): void {
    // After Svelte re-renders the row (selection class swap), pull
    // it into view. requestAnimationFrame defers past the current
    // microtask so the DOM has the latest layout.
    requestAnimationFrame(() => {
      const el = rowEls.get(path);
      if (el) el.scrollIntoView({ block: "nearest" });
    });
  }

  /// Auto-scroll when the selection changes from outside (e.g.
  /// store.revealAndSelect after a successful folder create).
  /// Keyboard nav already calls queueScrollIntoView directly, so
  /// re-scrolling here is benign — the second rAF resolves on the
  /// same frame without flicker. Wait one more rAF than usual to
  /// give Svelte a chance to expand any newly-uncollapsed ancestor
  /// folders so the row's DOM element exists.
  $effect(() => {
    const path = browserSelection.path;
    if (!path) return;
    requestAnimationFrame(() => {
      const el = rowEls.get(path);
      if (el) el.scrollIntoView({ block: "nearest" });
    });
  });

  /// Walk to the parent folder of `path`. Returns "" for top-level
  /// rows; the caller decides whether to act on root selection.
  function parentOf(path: string): string {
    const i = path.lastIndexOf("/");
    return i === -1 ? "" : path.slice(0, i);
  }

  function findFirstChildOf(path: string): string | null {
    const rows = visibleRows;
    const idx = rows.findIndex((r) => r.path === path);
    if (idx === -1 || idx + 1 >= rows.length) return null;
    const parent = rows[idx];
    const next = rows[idx + 1];
    return next.depth > parent.depth ? next.path : null;
  }

  function onTreeKeydown(e: KeyboardEvent): void {
    // Ignore composing IME input and any modifier-laden chord we
    // don't bind: lets ⌘/Ctrl+arrow combos fall through to the
    // browser / OS.
    if (e.isComposing || e.metaKey || e.ctrlKey || e.altKey) return;
    const rows = visibleRows;
    const cur = browserSelection.path;
    const curRow = cur ? rows.find((r) => r.path === cur) : undefined;
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        moveSelection(1);
        break;
      case "ArrowUp":
        e.preventDefault();
        moveSelection(-1);
        break;
      case "ArrowRight": {
        if (!curRow) {
          e.preventDefault();
          moveToFirst();
          break;
        }
        if (curRow.isDir) {
          e.preventDefault();
          if (!expanded[curRow.path]) {
            setExpanded(curRow.path, true);
          } else {
            const child = findFirstChildOf(curRow.path);
            if (child) {
              browserSelection.path = child;
              queueScrollIntoView(child);
            }
          }
        }
        break;
      }
      case "ArrowLeft": {
        if (!curRow) {
          e.preventDefault();
          moveToFirst();
          break;
        }
        if (curRow.isDir && expanded[curRow.path]) {
          e.preventDefault();
          setExpanded(curRow.path, false);
        } else {
          e.preventDefault();
          const parent = parentOf(curRow.path);
          if (parent) {
            browserSelection.path = parent;
            queueScrollIntoView(parent);
          }
        }
        break;
      }
      case "Enter": {
        if (!curRow) break;
        e.preventDefault();
        if (curRow.isDir) {
          toggle(curRow.path);
        } else if (isEditableText(curRow.path)) {
          // Same flow as a double-click on the row: open in the
          // active pane and close the browser overlay so the user
          // lands on the editor.
          void onOpen(curRow.path);
        }
        break;
      }
      case "Home":
        e.preventDefault();
        moveToFirst();
        break;
      case "End":
        e.preventDefault();
        moveToLast();
        break;
      // Backspace (Mac "delete") and forward-Delete both trigger
      // removal. The destructive uiConfirm in fileOps.remove is the
      // safety gate; without it we'd want to keep this unbound.
      case "Backspace":
      case "Delete": {
        if (!curRow) break;
        e.preventDefault();
        void remove(curRow.path, curRow.isDir);
        break;
      }
    }
  }

  /// Pull keyboard focus to the tree. Called by the browser overlay
  /// when it opens so arrows are live without an extra click.
  export function focusTree(): void {
    treeRootEl?.focus();
  }

  // ---- find within visible/expanded entries ---------------------------------
  // Cmd+F in the File Browser overlay sets a query here; matches are
  // computed against the current `visibleRows` set, so only entries
  // already expanded into view are eligible. The current match is
  // moved into `browserSelection` so opening (Enter) and scrolling
  // reuse the existing selection plumbing.

  type FindCountCb = (total: number, current: number) => void;
  let findQueryState = $state<string>("");
  let findOnCount: FindCountCb | undefined;
  let findCurrentIndex = $state<number>(-1);
  /// Non-reactive cache of the query string the cursor was last
  /// seeded to. Lets the cursor-management effect distinguish a
  /// fresh query (reset to first match) from a same-query match set
  /// update (e.g. user expanded a sibling folder mid-search) where
  /// the cursor should stay where the user left it.
  let lastSeededQuery: string | null = null;

  const findMatchPaths = $derived.by<string[]>(() => {
    const q = findQueryState.trim().toLowerCase();
    if (!q) return [];
    const out: string[] = [];
    for (const r of visibleRows) {
      const name = r.path.split("/").pop() ?? r.path;
      if (name.toLowerCase().includes(q)) out.push(r.path);
    }
    return out;
  });
  const findMatchSet = $derived<Set<string>>(new Set(findMatchPaths));

  /// Cursor management for find. Runs in two distinct branches:
  ///
  ///   1. Query just changed (or first match-set seen for a query):
  ///      reset cursor to the first match, scroll it into view, and
  ///      record the new query in `lastSeededQuery`. Empty query
  ///      drops the cursor to -1.
  ///   2. Same query, match set updated (folder expanded / collapsed
  ///      while find was open): clamp cursor into range, but DO NOT
  ///      reset to 0 — that would fight findStep, which moves the
  ///      cursor and triggers this effect via the findCurrentIndex
  ///      / findMatchPaths read below.
  ///
  /// Always republish the count via findOnCount so the host counter
  /// stays in sync regardless of which branch ran.
  $effect(() => {
    const q = findQueryState;
    const paths = findMatchPaths;
    const n = paths.length;
    if (q !== lastSeededQuery) {
      lastSeededQuery = q;
      if (n === 0) {
        findCurrentIndex = -1;
      } else {
        findCurrentIndex = 0;
        browserSelection.path = paths[0]!;
        queueScrollIntoView(paths[0]!);
      }
      findOnCount?.(n, findCurrentIndex);
      return;
    }
    if (n === 0) {
      findCurrentIndex = -1;
    } else if (findCurrentIndex >= n) {
      findCurrentIndex = n - 1;
    } else if (findCurrentIndex < 0) {
      findCurrentIndex = 0;
    }
    findOnCount?.(n, findCurrentIndex);
  });

  /// Set the active find query. Empty string clears the highlight.
  /// `onCount` (optional) receives `(total, current0Based)` whenever
  /// the match set or cursor moves so the host can drive a counter.
  export function setFindQuery(q: string, onCount?: FindCountCb): void {
    findOnCount = onCount;
    findQueryState = q;
  }

  /// Step to the next / previous match (wraps). No-op when there are
  /// no matches.
  export function findStep(direction: 1 | -1): void {
    const n = findMatchPaths.length;
    if (n === 0) return;
    const cur = findCurrentIndex < 0 ? 0 : findCurrentIndex;
    findCurrentIndex = (cur + direction + n) % n;
    const path = findMatchPaths[findCurrentIndex]!;
    browserSelection.path = path;
    queueScrollIntoView(path);
    findOnCount?.(n, findCurrentIndex);
  }

  export function clearFind(): void {
    setFindQuery("");
  }

  /// True when this row's name matches the active find query. Used
  /// to paint the `.find-match` class on matching rows; the row at
  /// `findCurrentIndex` additionally gets `.find-match--current`.
  function rowMatchClass(path: string): string {
    if (!findMatchSet.has(path)) return "";
    const idx = findMatchPaths.indexOf(path);
    return idx === findCurrentIndex ? "find-match find-match--current" : "find-match";
  }

  /// Svelte action: register / unregister a row's DOM element in
  /// `rowEls` so `queueScrollIntoView` can find it without a global
  /// query. Cleans up on unmount.
  function trackRow(node: HTMLElement, path: string): { destroy(): void } {
    rowEls.set(path, node);
    return {
      destroy() {
        if (rowEls.get(path) === node) rowEls.delete(path);
      },
    };
  }

  /// Move the context-menu element out to <body> so its `position:
  /// fixed` resolves against the viewport. OverlayShell's `.panel`
  /// gets a transform on hover (and during the open animation), and
  /// any non-`none` transform on an ancestor reparents fixed-
  /// positioned descendants to that ancestor instead of the viewport
  /// — without this portal the right-click menu visibly drifts away
  /// from the click point, especially with the inspector pane open
  /// (per Alex's phase-3 screenshot). Mirrors HamburgerMenu's portal.
  function portal(node: HTMLElement): { destroy(): void } {
    document.body.appendChild(node);
    return {
      destroy() {
        node.parentNode?.removeChild(node);
      },
    };
  }

  /// Dismiss the context menu on any click outside it. Registered in
  /// the capture phase on `window` so we observe the click before
  /// `OverlayShell`'s bubble-phase `stopPropagation` swallows it
  /// (which would otherwise leave the menu stuck open for every
  /// click inside the file browser overlay).
  $effect(() => {
    const onDocClick = (e: MouseEvent): void => {
      if (!menu) return;
      const t = e.target as HTMLElement | null;
      if (t && t.closest(".ctx")) return;
      menu = null;
    };
    window.addEventListener("click", onDocClick, true);
    return () => window.removeEventListener("click", onDocClick, true);
  });
</script>

<ul
  class="tree"
  class:drop-root={dropTarget === ""}
  role="tree"
  tabindex="0"
  bind:this={treeRootEl}
  onkeydown={onTreeKeydown}
  ondragover={(e) => onRowDragOver(e, "")}
  ondragleave={() => onRowDragLeave("")}
  ondrop={(e) => onRowDrop(e, "")}
>
  {#each root.children as node (node.path)}
    {@render renderNode(node, 0)}
  {/each}
  {#if root.children.length === 0 && tree.loading}
    <li class="empty">
      <div class="empty-title">Loading files...</div>
    </li>
  {:else if root.children.length === 0 && tree.error}
    <li class="empty">
      <div class="empty-title">File listing failed</div>
      <div class="empty-detail">{tree.error}</div>
    </li>
  {:else if root.children.length === 0}
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
        class={`row dir ${rowMatchClass(node.path)}`}
        class:selected={browserSelection.path === node.path}
        class:zebra={rowIndex % 2 === 1}
        class:drop-target={dropTarget === node.path}
        style="padding-left: {depth * 12}px"
        oncontextmenu={(e) => showMenu(e, node.path, true)}
        role="treeitem"
        tabindex="-1"
        aria-expanded={!!expanded[node.path]}
        aria-selected={browserSelection.path === node.path}
        draggable="true"
        ondragstart={(e) => onFileDragStart(e, node.path, true)}
        ondragover={(e) => onRowDragOver(e, node.path)}
        ondragleave={() => onRowDragLeave(node.path)}
        ondrop={(e) => onRowDrop(e, node.path)}
        use:trackRow={node.path}
      >
        <button
          class="twirl"
          onclick={() => toggle(node.path)}
          aria-label={expanded[node.path] ? "collapse" : "expand"}
        >
          {#if expanded[node.path]}
            <ChevronDown size={14} strokeWidth={1.75} aria-hidden="true" />
          {:else}
            <ChevronRight size={14} strokeWidth={1.75} aria-hidden="true" />
          {/if}
        </button>
        <!-- GitHub-style folder glyph (open chevron + folder mirror the
             dark file-tree styling from request.md). The icon swaps
             between open / closed so a glance over the column reads
             expand state without parsing chevrons. -->
        <span class="row-icon dir-icon" aria-hidden="true">
          {#if expanded[node.path]}
            <FolderOpen size={14} strokeWidth={1.75} />
          {:else}
            <Folder size={14} strokeWidth={1.75} />
          {/if}
        </span>
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
          {#if node.children.length === 0 && tree.loadingDirs[node.path]}
            <li class="empty child-empty" style="padding-left: {(depth + 1) * 12}px">
              Loading...
            </li>
          {:else if node.children.length === 0 && tree.dirErrors[node.path]}
            <li class="empty child-empty" style="padding-left: {(depth + 1) * 12}px">
              {tree.dirErrors[node.path]}
            </li>
          {/if}
        </ul>
      {/if}
    {:else}
      {@const editable = isEditableText(node.path)}
      {@const contact = contactPaths.has(node.path)}
      {@const kind = classifyFile(node.path, contact ? "contact" : undefined)}
      {@const Icon = iconFor(kind)}
      <div
        class={`row file ${rowMatchClass(node.path)}`}
        class:selected={browserSelection.path === node.path}
        class:non-editable={!editable}
        class:contact
        class:zebra={rowIndex % 2 === 1}
        style="padding-left: {depth * 12 + 16}px"
        oncontextmenu={(e) => showMenu(e, node.path, false)}
        role="treeitem"
        tabindex="-1"
        aria-selected={browserSelection.path === node.path}
        draggable="true"
        ondragstart={(e) => onFileDragStart(e, node.path, false)}
        title={contact ? "contact" : editable ? undefined : "view-only (not an editable text file)"}
        use:trackRow={node.path}
      >
        <!-- Per-kind glyph leading the row. Same icon set used by
             the editor tab strip and (in time) by the inspector
             headers, so a file reads with the same icon wherever it
             surfaces. Contact rows still tint the filename via
             .row.contact > .name; the icon adds redundancy + a
             scannable column at the row's left edge. -->
        <span class="row-icon" aria-hidden="true">
          <Icon size={14} strokeWidth={1.75} />
        </span>
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
  <div class="ctx" use:portal use:clampMenu={{ x: menu.x, y: menu.y }}>
    {#if menu.isDir}
      <button onclick={() => newFile(menu!.path)}>
        <FilePlus size={16} strokeWidth={1.75} aria-hidden="true" />
        <span>New file</span>
      </button>
      <button onclick={() => newDir(menu!.path)}>
        <FolderPlus size={16} strokeWidth={1.75} aria-hidden="true" />
        <span>New folder</span>
      </button>
    {/if}
    <button onclick={() => graphThis(menu!.path, menu!.isDir)}>
      <Network size={16} strokeWidth={1.75} aria-hidden="true" />
      <span>Graph this</span>
    </button>
    <button onclick={() => searchThis(menu!.path, menu!.isDir)}>
      <Search size={16} strokeWidth={1.75} aria-hidden="true" />
      <span>Search this</span>
    </button>
    <button onclick={() => rename(menu!.path, menu!.isDir)}>
      <Pencil size={16} strokeWidth={1.75} aria-hidden="true" />
      <span>Rename / Move</span>
    </button>
    <button class="danger" onclick={() => remove(menu!.path, menu!.isDir)}>
      <Trash2 size={16} strokeWidth={1.75} aria-hidden="true" />
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
  .tree:focus {
    outline: none;
  }
  .tree:focus-visible .row.selected {
    box-shadow: inset 2px 0 0 var(--accent);
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
  /* Drop highlight during drag-and-drop move. Boxed outline + accent
     tint so the user sees exactly which folder will receive the drop,
     without disturbing the row height. */
  .row.drop-target {
    background: var(--accent-bg, var(--hover-bg));
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  /* Drop at drive root: outline the whole tree container so the
     user can tell root-drop is a valid target even when the cursor
     is over empty space below the last row. */
  .tree.drop-root {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .twirl {
    background: none;
    border: 0;
    cursor: pointer;
    width: 14px;
    height: 14px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
  }
  /* Folder glyph: pulls toward --accent so the folder column reads
     as the navigational scaffold separate from the per-file kind
     icons. Sits beside the chevron and before the folder name. */
  .row.dir .dir-icon {
    color: var(--g-folder);
    margin-right: 2px;
  }
  /* Cmd+F highlight on rows whose filename matches the active find
     query (FileBrowserOverlay drives setFindQuery). The current
     match gets a stronger ring so step-through (Enter / Shift+Enter)
     is visually obvious. Uses --warn-text so the highlight reads as
     "attention" without colliding with selection or hover bands. */
  .row.find-match {
    background: color-mix(in srgb, var(--warn-text, #e3b341) 16%, transparent);
  }
  .row.find-match--current {
    background: color-mix(in srgb, var(--warn-text, #e3b341) 28%, transparent);
    box-shadow: inset 0 0 0 1px var(--warn-text, #e3b341);
  }
  /* Per-kind glyph at the row's left edge. Mirrors the tab-strip
     icon (FileText / User / Image / etc.) so the file reads with the
     same glyph in both surfaces. Sits one step below the label hue
     so it doesn't compete with the filename for attention. */
  .row-icon {
    display: inline-flex;
    align-items: center;
    color: var(--text-secondary);
    flex-shrink: 0;
  }
  .row.contact .row-icon { color: var(--warn-text); }
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
  /* Contact files (`chan.kind: contact` frontmatter): paint the name
     in --warn-text so the row reads as "contact" at a glance. One
     palette tone for contacts across surfaces (file tree, inspector
     chip + ref border, editor wiki pill, graph mention nodes). */
  .row.contact > .name {
    color: var(--warn-text);
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
  .empty-detail {
    color: var(--text-secondary);
    line-height: 1.35;
    overflow-wrap: anywhere;
  }
  .child-empty {
    font-size: 12px;
    text-align: left;
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
  .ctx :global(svg) {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    color: var(--text-secondary);
  }
  .ctx button.danger :global(svg) { color: var(--warn-text); }
</style>
