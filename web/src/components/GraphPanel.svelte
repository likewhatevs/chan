<script lang="ts">
  // Graph view overlay: Cytoscape.js renderer over chan's GraphView
  // payload. fcose handles force-directed layout; pan / zoom / node
  // drag / hover / selection all come from Cytoscape's built-ins.
  //
  // Scope (top-bar dropdown) drives a BFS over the full graph that
  // produces a visible-id set; the per-edge-kind chips compose with
  // it. Both filters are applied as a `display: none` toggle on the
  // existing Cytoscape elements, so layout positions are stable
  // across filter changes.
  //
  // Pinning: in file / group / git_repo / dir scope, the seed file
  // nodes are repositioned to the canvas center (or fanned around
  // it for multi-seed) and locked, then a gentle fcose pass relaxes
  // neighbours. Drive / global scope leaves all nodes free.

  import {
    ArrowLeft,
    ArrowRight,
    Maximize2,
    Minimize2,
    Settings,
  } from "lucide-svelte";
  import {
    overlayMaximized,
    setOverlayMaximized,
  } from "../state/pageWidth.svelte";

  import { api } from "../api/client";
  import type {
    FsGraphNode,
    FsGraphResponse,
    GraphView,
    GraphViewEdge,
    GraphViewNode,
    LanguageGraphResponse,
  } from "../api/types";
  import { openInActivePane, type GraphTab } from "../state/tabs.svelte";
  import {
    availableGraphScopes,
    graphReloadSignal,
    graphOverlay,
    openSettings,
    paneWidths,
    persistPaneWidths,
    revealPathInBrowser,
    scopeFsGraphFromHere,
    tree,
  } from "../state/store.svelte";
  import { onDestroy } from "svelte";
  import { type ScopeOption, defaultScopeId } from "../state/scope.svelte";
  import ResizeHandle from "./ResizeHandle.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";
  import DriveInfoBody from "./DriveInfoBody.svelte";
  import Inspector from "./Inspector.svelte";
  import OverlayShell from "./OverlayShell.svelte";
  import InspectorBody, { type InspectorSelection } from "./InspectorBody.svelte";
  import GraphCanvas from "./GraphCanvas.svelte";
  import KindChip from "./KindChip.svelte";
  import { classifyFile as classifyFileKind, type FileKind } from "../state/kinds";
  import { chordFor } from "../state/shortcuts";
  import { FS_GRAPH_DEPTH_MAX, graphDepthCap } from "../graph/depth";

  let {
    tab,
    onClose,
  }: {
    tab?: GraphTab;
    onClose?: () => void;
  } = $props();

  const graphState = $derived(tab ?? graphOverlay);
  const visible = $derived(tab ? true : graphOverlay.open);

  /// Dropdown options derived from the live layout; relabels
  /// "drive" as "Whole drive".
  const scopeOptions = $derived<ScopeOption[]>(availableGraphScopes());

  const currentScope = $derived<ScopeOption | null>(
    scopeOptions.find((o) => o.id === graphState.scopeId) ?? null,
  );

  /// Snap to a sensible scope on open if the saved scopeId no longer
  /// resolves (file closed since last open, group set changed). Skip
  /// while the overlay is closed so background layout changes do not
  /// rewrite saved graph state.
  $effect(() => {
    if (!visible) return;
    if (!currentScope) graphState.scopeId = defaultScopeId();
  });

  function close(): void {
    if (onClose) onClose();
    else graphOverlay.open = false;
  }

  // ---- types -------------------------------------------------------------

  // The graph view renders documents (files), images (also file
  // nodes, split by extension at element-build time), tags, and
  // mentions. Dates are still filtered out at load: chan-drive's
  // graph index has stopped emitting date edges (issue #17), but
  // older indexes may still contain them.
  /// `group` is a synthetic edge kind: cytoscape-only, never emitted
  /// by chan-drive's graph index. We add `group` edges from a
  /// synthetic hub node (id `SCOPE_HUB_ID`, kind `scope`) to every
  /// file in a multi-file scope (`currentScope.kind === "group"`) so
  /// the canvas shows which files the user has pinned together.
  type RenderedEdgeKind = "link" | "tag" | "mention" | "contains" | "language" | "group";
  /// Stable id for the synthetic scope hub node. Prefixed with `__`
  /// so it can't collide with a real file path.
  const SCOPE_HUB_ID = "__scope_hub__";
  type RenderedEdge = GraphViewEdge & { kind: RenderedEdgeKind };
  type RenderedNode = Extract<
    GraphViewNode,
    { kind: "file" | "tag" | "mention" | "language" | "folder" }
  >;
  /// Chip toggles. `link`, `tag`, `mention` are edge-kind filters
  /// (the visual element they govern is the edge plus any node only
  /// reachable through edges of that kind). `img` and `folder` are
  /// node filters: flipping them off hides every file node whose
  /// path classifies as image / directory, along with any edge
  /// touching one.
  type FilterKind = "link" | "tag" | "mention" | "language" | "img" | "folder";

  // ---- state -------------------------------------------------------------

  let nodes: RenderedNode[] = $state([]);
  let edges: RenderedEdge[] = $state([]);
  let fsNodes: FsGraphNode[] = $state([]);
  let fsTruncated = $state(false);
  let driveDepthProbe: FsGraphResponse | null = $state(null);
  let driveDepthProbeLoading = $state(false);
  let languageMaxDepth = $state(0);
  let loading = $state(true);
  let error: string | null = $state(null);
  let watchReloadTimer: ReturnType<typeof setTimeout> | null = null;
  let seenGraphReloadNonce = graphReloadSignal.nonce;

  /// Chip toggles live on `graphState.filters` (module state) so
  /// they round-trip through the URL hash. Local proxy aliases keep
  /// the existing read sites compact.
  const show = $derived(graphState.filters);
  const filesystemMode = $derived(
    graphState.mode === "filesystem" &&
      (currentScope?.kind === "file" ||
        currentScope?.kind === "dir" ||
        currentScope?.kind === "drive" ||
        currentScope?.kind === "global"),
  );
  const languageMode = $derived(graphState.mode === "language");
  const depthCap = $derived.by(() => {
    if (languageMode) return Math.max(1, languageMaxDepth);
    if (loading && currentScope?.kind === "dir" && nodes.length === 0) {
      return DEPTH_MAX;
    }
    return graphDepthCap({
      scope: currentScope,
      nodes,
      fsGraph: filesystemMode
        ? { nodes: fsNodes, truncated: fsTruncated }
        : currentScope?.kind === "drive" || currentScope?.kind === "global"
          ? driveDepthProbe
          : null,
      hardMax: DEPTH_MAX,
      fsMax: FS_GRAPH_DEPTH_MAX,
    });
  });

  // Currently inspected node, surfaced in the side details panel.
  // Tap a node to set this; tap empty space to clear it. Nodes never
  // auto-open on click; the panel's Open button is the only path to
  // opening a file from here.
  let selectedId = $state<string | null>(null);

  /// Hamburger menu. Same affordance as the file browser / search
  /// overlays. The depth slider, reload, and details toggle all
  /// live inside it now so the bar above the canvas keeps only the
  /// stateful selectors (scope, filter chips).
  let menu: HamburgerMenu | undefined = $state();
  let menuOpen = $state(false);
  /// Bigger than the other overlays because the menu carries the
  /// scope-conditional depth slider on top of toggle + reload.
  const POPOVER_HEIGHT = 200;
  const POPOVER_WIDTH = 260;
  /// Cap matches the slider's `max` attribute below. Lifting it past
  /// 5 gave room for sparse drives where the seed file's neighborhood
  /// fans out wider than the previous limit allowed; 10 is well
  /// short of the diameter of any realistic drive.
  const DEPTH_MAX = 10;

  /// Files split into "doc", "img", or "contact" by the same rules
  /// the GraphCanvas renderer uses: image classification is
  /// extension-based, contact comes from the wire `node_kind:
  /// "contact"` stamp the indexer applies to chan-drive's
  /// `contacts()` set, everything else is a doc. Mirrored here
  /// because `hiddenImageIds` / `counts` / `inspectorSelection`
  /// need the kind upfront for chip filtering.
  function classifyFile(
    path: string,
    nodeKind: "contact" | undefined,
  ): "doc" | "img" | "contact" {
    if (/\.(png|jpe?g|gif|webp|svg|avif|bmp)$/i.test(path)) return "img";
    if (nodeKind === "contact") return "contact";
    return "doc";
  }

  function toggleInspector(): void {
    graphState.inspectorOpen = !graphState.inspectorOpen;
    menu?.close();
  }

  function doToggleOverlayMaximized(): void {
    setOverlayMaximized(!overlayMaximized.on);
    menu?.close();
  }

  async function reloadGraph(): Promise<void> {
    menu?.close();
    if (currentScope?.kind === "drive" || currentScope?.kind === "global") {
      driveDepthProbe = null;
      await loadDriveDepthProbe();
    }
    await load();
  }

  async function loadDriveDepthProbe(): Promise<void> {
    if (driveDepthProbeLoading) return;
    driveDepthProbeLoading = true;
    try {
      driveDepthProbe = await api.fsGraph({
        scope: "directory",
        path: "",
        depth: FS_GRAPH_DEPTH_MAX,
      });
    } catch {
      driveDepthProbe = null;
    } finally {
      driveDepthProbeLoading = false;
    }
  }

  function doOpenSettings(): void {
    menu?.close();
    openSettings();
  }

  function onGraphContextMenu(e: MouseEvent): void {
    const t = e.target as HTMLElement | null;
    // Don't hijack right-click on the scope select or input controls
    // in the bar; let the browser's native UI fire there.
    if (t?.closest("select, input, .scope-select, .filters")) return;
    e.preventDefault();
    menu?.openAtCursor(e.clientX, e.clientY);
  }

  // ---- derived: scope-filtered render set --------------------------------
  //
  // Two filters compose to decide what's drawn:
  //
  //   (1) the SCOPE picker in the header (file / group / drive).
  //       For file and group, BFS out from the seed paths up to
  //       graphState.depth hops. Drive = no filter.
  //   (2) the per-edge-kind chips (link / tag). Edges whose kind
  //       is filtered out are dropped, and any non-file node
  //       attached only via filtered edges drops too.
  //
  // (1) runs first so the BFS sees the full graph (depth = "graph
  // hops away"). (2) is a render-time filter that can change without
  // re-walking the graph.

  /// Set of node ids included by the current scope. `null` means
  /// "no scope filter" — drive scope (current behaviour) or the
  /// global scope (placeholder; once cross-drive indexing lands
  /// it'll need its own logic, but treating it as "no filter"
  /// today returns the same set as drive since chan only knows
  /// about one drive at a time).
  const scopedNodeIds = $derived.by<Set<string> | null>(() => {
    if (!currentScope) return null;
    if (currentScope.kind === "drive" || currentScope.kind === "global") {
      return null;
    }
    // Tag scope: seed with the tag node itself; BFS expands across
    // every doc that references it (depth 1) and further along
    // those docs' edges (depth 2+). No path resolution needed —
    // the node id IS the seed.
    if (currentScope.kind === "tag") {
      const seedIds = new Set<string>([currentScope.nodeId]);
      const visited = new Set(seedIds);
      let frontier = new Set(seedIds);
      for (let i = 0; i < graphState.depth; i++) {
        const next = new Set<string>();
        for (const e of edges) {
          if (frontier.has(e.source) && !visited.has(e.target)) {
            next.add(e.target);
            visited.add(e.target);
          } else if (frontier.has(e.target) && !visited.has(e.source)) {
            next.add(e.source);
            visited.add(e.source);
          }
        }
        if (next.size === 0) break;
        frontier = next;
      }
      return visited;
    }
    let seedPaths: string[];
    if (currentScope.kind === "git_repo" || currentScope.kind === "dir") {
      const root =
        currentScope.kind === "git_repo" ? currentScope.root : currentScope.path;
      const prefix = root + "/";
      seedPaths = nodes
        .filter(
          (n) =>
            n.kind === "file" &&
            (n.path === root || n.path.startsWith(prefix)),
        )
        .map((n) => (n.kind === "file" ? n.path : ""))
        .filter((p) => p);
    } else {
      seedPaths =
        currentScope.kind === "file"
          ? [currentScope.path]
          : currentScope.paths;
    }
    const seedIds = new Set<string>();
    for (const n of nodes) {
      if (n.kind === "file" && seedPaths.includes(n.path)) seedIds.add(n.id);
    }
    if (seedIds.size === 0) return seedIds;
    const visited = new Set(seedIds);
    let frontier = new Set(seedIds);
    for (let i = 0; i < graphState.depth; i++) {
      const next = new Set<string>();
      for (const e of edges) {
        if (frontier.has(e.source) && !visited.has(e.target)) {
          next.add(e.target);
          visited.add(e.target);
        } else if (frontier.has(e.target) && !visited.has(e.source)) {
          next.add(e.source);
          visited.add(e.source);
        }
      }
      if (next.size === 0) break;
      frontier = next;
    }
    return visited;
  });

  /// File-node ids the `img` filter currently hides. Pulled out so
  /// both visibleEdges (drop edges touching a hidden image) and
  /// visibleNodeIds (skip the node entirely) agree.
  const hiddenImageIds = $derived.by(() => {
    const ids = new Set<string>();
    if (show.img) return ids;
    for (const n of nodes) {
      if (n.kind === "file" && classifyFile(n.path, n.node_kind) === "img") ids.add(n.id);
    }
    return ids;
  });

  /// Contact-kind file-node ids hidden when the contact chip is off.
  /// The chip is wired off the `mention` filter slot (which is now
  /// user-labeled "contact") so toggling it has the same shape as the
  /// img toggle: hide the nodes AND any edges touching them. Without
  /// the node hide, the user would just see the contact rectangles
  /// floating with their mention edges gone — half a filter.
  const hiddenContactIds = $derived.by(() => {
    const ids = new Set<string>();
    if (show.mention) return ids;
    for (const n of nodes) {
      if (n.kind === "file" && n.node_kind === "contact") ids.add(n.id);
    }
    return ids;
  });

  /// Directory node ids hidden when the folder chip is off. Only meaningful
  /// in filesystem mode where directory-kind nodes are emitted; in
  /// markdown / language modes there are no directory nodes so the set
  /// stays empty and the toggle is a no-op.
  const hiddenFolderIds = $derived.by(() => {
    const ids = new Set<string>();
    if (show.folder) return ids;
    for (const n of nodes) {
      if (n.kind === "folder") ids.add(n.id);
    }
    return ids;
  });

  function edgeVisibleByChip(kind: RenderedEdgeKind): boolean {
    if (kind === "contains") return show.folder;
    if (kind === "group") return true;
    return show[kind];
  }

  const visibleEdges = $derived(
    edges.filter(
      (e) =>
        edgeVisibleByChip(e.kind) &&
        !hiddenImageIds.has(e.source) &&
        !hiddenImageIds.has(e.target) &&
        !hiddenContactIds.has(e.source) &&
        !hiddenContactIds.has(e.target) &&
        !hiddenFolderIds.has(e.source) &&
        !hiddenFolderIds.has(e.target) &&
        (scopedNodeIds === null ||
          (scopedNodeIds.has(e.source) && scopedNodeIds.has(e.target))),
    ),
  );
  const visibleNodeIds = $derived.by(() => {
    const ids = new Set<string>();
    for (const n of nodes) {
      if (scopedNodeIds !== null && !scopedNodeIds.has(n.id)) continue;
      if (hiddenFolderIds.has(n.id)) continue;
      if (n.kind === "file" && !hiddenImageIds.has(n.id) && !hiddenContactIds.has(n.id)) {
        ids.add(n.id);
      } else if (n.kind === "folder") {
        ids.add(n.id);
      }
    }
    for (const e of visibleEdges) {
      ids.add(e.source);
      ids.add(e.target);
    }
    return ids;
  });

  /// Chip counts. Edge-kind chips report the underlying edge count;
  /// the `img` chip reports image-file-node count since that's what
  /// it actually toggles. The mention chip (user label "contact")
  /// adds the contact-file count on top of mention edges so the
  /// number reflects everything the toggle hides.
  const counts = $derived.by(() => {
    const c: Record<FilterKind, number> = {
      link: 0,
      tag: 0,
      mention: 0,
      language: 0,
      img: 0,
      folder: 0,
    };
    for (const e of edges) {
      if (e.kind === "contains") c.folder++;
      else c[e.kind]++;
    }
    for (const n of nodes) {
      if (n.kind === "folder") {
        c.folder++;
        continue;
      }
      if (n.kind !== "file") continue;
      const cls = classifyFile(n.path, n.node_kind);
      if (cls === "img") c.img++;
      else if (cls === "contact") c.mention++;
    }
    return c;
  });

  // ---- side-panel derived state ------------------------------------------

  const nodeById = $derived(new Map(nodes.map((n) => [n.id, n])));

  const selectedNode = $derived<RenderedNode | null>(
    // `selectedId !== null` (not a truthy test): the drive-root
    // node carries id="" in the drive-scope merged view, and a
    // truthy test would silently null it out.
    selectedId !== null ? (nodeById.get(selectedId) ?? null) : null,
  );
  const fsNodeById = $derived(new Map(fsNodes.map((n) => [n.id, n])));
  const selectedFsNode = $derived<FsGraphNode | null>(
    // The drive-root directory has id="" (empty path = drive root), so
    // `selectedId` is checked with `!== null` rather than a truthy
    // test — otherwise clicking the root node silently no-op's the
    // inspector.
    filesystemMode && selectedId !== null
      ? (fsNodeById.get(selectedId) ?? null)
      : null,
  );

  /// True when the graph claims the node is a real file but the
  /// current tree listing doesn't have its path. This happens when
  /// the search index hasn't been rebuilt after a bulk drive change
  /// (drive switch, mass delete). Treat these as ghosts so the
  /// inspector renders an inline summary instead of FileInfoBody's
  /// "click a file" empty state.
  const treeHasPath = $derived(new Set(tree.entries.map((e) => e.path)));
  const isFileGhost = $derived<boolean>(
    selectedNode != null &&
      selectedNode.kind === "file" &&
      (selectedNode.missing || !treeHasPath.has(selectedNode.path)),
  );
  let ghostIndexerHint = $state<string | null>(null);

  function indexerGhostHint(status: string | undefined, queueDepth: number | undefined): string | null {
    if (status === "settling") {
      const n = Math.max(0, Math.floor(queueDepth ?? 0));
      return `indexer is catching up (${n} event(s) pending)`;
    }
    if (status === "rebuilding") return "indexer is rebuilding (full pass)";
    return null;
  }

  $effect(() => {
    if (
      !visible ||
      !isFileGhost ||
      selectedNode?.kind !== "file" ||
      selectedNode.missing
    ) {
      ghostIndexerHint = null;
      return;
    }
    let cancelled = false;
    let timer: ReturnType<typeof setInterval> | null = null;
    async function poll(): Promise<void> {
      try {
        const health = await api.health();
        if (cancelled) return;
        ghostIndexerHint = indexerGhostHint(
          health.indexer?.status,
          health.indexer?.queue_depth,
        );
      } catch {
        if (!cancelled) ghostIndexerHint = null;
      }
    }
    void poll();
    timer = setInterval(() => void poll(), 1000);
    return () => {
      cancelled = true;
      if (timer) clearInterval(timer);
    };
  });

  /// Documents that reference the currently-selected tag or mention
  /// node, restricted to nodes drawn in the current subgraph. Passed
  /// to InspectorBody as `documentsOverride` so the inspector stays
  /// consistent with what the user can see on the canvas; without it
  /// TagInfoBody would fall back to the full-graph
  /// `documentsReferencing` lookup.
  const selectionDocumentsInScope = $derived.by(() => {
    if (
      !selectedNode ||
      (selectedNode.kind !== "tag" && selectedNode.kind !== "mention")
    ) {
      return undefined;
    }
    const out: Extract<RenderedNode, { kind: "file" }>[] = [];
    for (const e of edges) {
      if (e.target !== selectedNode.id) continue;
      const source = nodeById.get(e.source);
      if (source && source.kind === "file") out.push(source);
    }
    return out;
  });

  function openSelectedFile(): void {
    if (selectedNode && selectedNode.kind === "file" && !selectedNode.missing) {
      void openInActivePane(selectedNode.path);
      close();
    }
  }

  /// Try to resolve a mention/contact label to a real .md file on
  /// disk: scan tree.entries for a contact-kind entry whose basename
  /// (sans .md) includes the mention label case-insensitively. Loose
  /// match on purpose — `alice` should hit `Contacts/Alice Chen.md`
  /// without requiring an exact server-side resolution table on the
  /// frontend. Returns the first match; null when nothing fits.
  function resolveContactToPath(label: string): string | null {
    const needle = label.replace(/^@@/, "").toLowerCase().trim();
    if (!needle) return null;
    for (const e of tree.entries) {
      if (e.is_dir || e.kind !== "contact") continue;
      const base = e.path.split("/").pop() ?? e.path;
      const stem = base.replace(/\.md$/i, "").toLowerCase();
      if (stem.includes(needle)) return e.path;
    }
    return null;
  }

  /// Path the currently-selected mention/contact node resolves to,
  /// or null when the mention is unresolved (no contact file on
  /// disk yet). Drives whether the inspector renders the "Open in
  /// this pane" and "Set as Scope" buttons for mention rows.
  const selectedContactPath = $derived<string | null>(
    selectedNode && selectedNode.kind === "mention"
      ? resolveContactToPath(selectedNode.label)
      : null,
  );

  /// "Show in file browser" handler for image nodes in the inspector.
  /// FileInfoBody only renders the button when this is set + the
  /// selection is an image, so it's safe to bind for every file.
  function revealSelectedFile(): void {
    if (selectedNode && selectedNode.kind === "file" && !selectedNode.missing) {
      revealPathInBrowser(selectedNode.path, { inspectorOpen: true });
      close();
    }
  }

  /// "Show File" / "Show Directory" handler for fs-mode nodes. Same
  /// pattern as revealSelectedFile but pulls the path off the
  /// FsGraphNode so it works for directories (which have no semantic-
  /// graph counterpart in selectedNode) and for files surfaced
  /// only via the fs-graph. path === "" is the drive root;
  /// revealAndSelect handles it by clearing the tree selection
  /// and opening the browser at the drive level.
  function revealSelectedFsEntry(): void {
    if (
      selectedFsNode &&
      (isFsDirectory(selectedFsNode) || selectedFsNode.kind === "file") &&
      selectedFsNode.path !== undefined
    ) {
      revealPathInBrowser(selectedFsNode.path, { inspectorOpen: true });
      close();
    }
  }

  function selectFromList(n: RenderedNode): void {
    // GraphCanvas reads `selectedId` reactively and applies the
    // selection ring + first-degree label reveal itself, so all
    // this surface has to do is mirror the id.
    selectedId = n.id;
    graphState.inspectorOpen = true;
  }

  /// Click handler for link / backlink / tag-doc entries surfaced by
  /// the shared InspectorBody. The other surfaces (file browser,
  /// search) treat onNavigate as "open in the editor", but here the
  /// user is exploring the graph: route to a select-in-canvas instead
  /// so the inspector keeps following the user as they hop along
  /// references. The "Open in this pane" button (onOpen) is still
  /// the path to the editor.
  function selectByPath(path: string): void {
    const n = nodeById.get(path);
    if (n) selectFromList(n);
  }

  /// Build the dispatcher's selection from selectedNode. Returns null
  /// for ghost / missing file nodes: FileInfoBody looks up the entry
  /// in the file tree, and a missing path would render as an empty
  /// "click a file to inspect" placeholder, hiding the ghost. We
  /// render the ghost branch inline below instead.
  const inspectorSelection = $derived<InspectorSelection>(
    selectedNode === null
      ? null
      : selectedNode.kind === "file"
        ? isFileGhost
          ? null
          : { kind: "file", path: selectedNode.path }
        : selectedNode.kind === "tag" || selectedNode.kind === "mention"
          ? {
              kind: selectedNode.kind,
              nodeId: selectedNode.id,
              label: selectedNode.label,
            }
          : null,
  );

  // ---- presentation ------------------------------------------------------

  /// Cytoscape resolves --g-* via getComputedStyle at buildCytoscape
  /// time, so theme changes propagate next reload.
  const EDGE_COLORS: Record<RenderedEdgeKind, string> = {
    link: "var(--text-secondary)",
    tag: "var(--g-tag)",
    mention: "var(--warn-text)",
    contains: "var(--g-folder)",
    language: "var(--g-language)",
    // Group-scope edges read as the accent so they pop against the
    // document edges without looking like another link kind.
    group: "var(--accent)",
  };

  /// Per-chip dot color. Edge-kind chips reuse EDGE_COLORS; img is a
  /// node filter so it points at the image node color directly.
  const FILTER_COLORS: Record<FilterKind, string> = {
    link: EDGE_COLORS.link,
    tag: EDGE_COLORS.tag,
    mention: EDGE_COLORS.mention,
    language: EDGE_COLORS.language,
    img: "var(--g-img)",
    folder: "var(--g-folder)",
  };

  function isFsDirectory(node: FsGraphNode): boolean {
    return node.kind === "directory";
  }

  function stripDirectoryPrefix(id: string): string {
    if (id.startsWith("directory:")) return id.slice("directory:".length);
    if (id.startsWith("folder:")) return id.slice("folder:".length);
    return id;
  }

  // ---- canvas glue -------------------------------------------------------
  //
  // The graph is rendered by the GraphCanvas child component. This
  // panel owns the data fetch + the scope/depth derivations; the
  // canvas owns the d3-force simulation, painting, and pointer
  // interaction. Selection round-trips: GraphCanvas calls back into
  // `setSelected` on tap, and we re-emit `selectedId` so the
  // inspector + per-selection style updates fire as before.

  /// Files under a directory or repo-root prefix. Used by
  /// `focalIds` to seed the canvas with anchor nodes for dir /
  /// git_repo scopes.
  function filesUnder(prefix: string): string[] {
    const root = prefix.replace(/\/+$/, "");
    const withSlash = root + "/";
    return nodes
      .filter(
        (n) =>
          n.kind === "file" && (n.path === root || n.path.startsWith(withSlash)),
      )
      .map((n) => (n.kind === "file" ? n.path : ""))
      .filter((p) => p);
  }

  /// Node ids the canvas should pin at the world origin while the
  /// initial layout settles. Empty list = no anchor (drive / global
  /// scope); the canvas falls back to a free force-directed layout.
  const focalIds = $derived.by<string[]>(() => {
    if (!currentScope) return [];
    if (currentScope.kind === "group") return [SCOPE_HUB_ID];
    if (currentScope.kind === "tag") return [currentScope.nodeId];
    let seedPaths: string[];
    if (currentScope.kind === "file") seedPaths = [currentScope.path];
    else if (currentScope.kind === "dir") seedPaths = filesUnder(currentScope.path);
    else if (currentScope.kind === "git_repo")
      seedPaths = filesUnder(currentScope.root);
    else return [];
    const ids: string[] = [];
    for (const n of nodes) {
      if (n.kind === "file" && seedPaths.includes(n.path)) ids.push(n.id);
    }
    return ids;
  });

  /// Fetch the graph view and stash the rendered-kind subset
  /// (files + tags + mentions). Date nodes / edges are dropped:
  /// chan-drive's index has stopped emitting them (issue #17), but
  /// stale indexes may still contain them.
  async function load(): Promise<void> {
    loading = true;
    error = null;
    try {
      if (filesystemMode && currentScope) {
        languageMaxDepth = 0;
        const fsScope =
          currentScope.kind === "file" ? "file" : "directory";
        const fsPath =
          currentScope.kind === "dir" || currentScope.kind === "file"
            ? currentScope.path
            : "";
        const fs = await api.fsGraph({
          scope: fsScope,
          path: fsPath,
          depth: graphState.depth,
        });
        fsNodes = fs.nodes;
        fsTruncated = fs.truncated;
        nodes = mapFsNodes(fs);
        edges = mapFsEdges(fs);
        const pending = graphState.pendingSelectId;
        if (pending && fs.nodes.some((n) => n.id === pending)) {
          selectedId = pending;
          graphState.inspectorOpen = true;
        } else if (!selectedId || !fs.nodes.some((n) => n.id === selectedId)) {
          selectedId = fs.path;
        }
        graphState.pendingSelectId = null;
        return;
      }
      fsNodes = [];
      fsTruncated = false;
      if (languageMode) {
        const g: LanguageGraphResponse = await api.languageGraph({
          depth: graphState.depth,
        });
        languageMaxDepth = g.max_depth;
        nodes = mapLanguageNodes(g.nodes);
        edges = mapLanguageEdges(g.edges);
        selectedId = selectedId && g.nodes.some((n) => n.id === selectedId)
          ? selectedId
          : null;
        graphState.pendingSelectId = null;
        return;
      }
      languageMaxDepth = 0;
      const graphScope =
        currentScope?.kind === "file"
          ? { scope: "file" as const, path: currentScope.path }
          : currentScope?.kind === "dir"
            ? { scope: "directory" as const, path: currentScope.path }
            : { scope: "drive" as const, path: "" };
      const g = await api.graph({
        ...graphScope,
        depth: Math.max(graphState.depth, 1),
      });
      const seenIds = new Set<string>();
      const renderedNodes: RenderedNode[] = [];
      const renderedEdges: RenderedEdge[] = [];
      fsNodes = [];
      fsTruncated = false;
      for (const n of g.nodes) {
        const mapped = mapGraphNode(n);
        if (mapped && !seenIds.has(mapped.id)) {
          renderedNodes.push(mapped);
          seenIds.add(mapped.id);
        }
      }
      for (const e of g.edges) {
        if (
          e.kind === "link" ||
          e.kind === "tag" ||
          e.kind === "mention" ||
          e.kind === "contains" ||
          e.kind === "language"
        ) {
          renderedEdges.push(e as RenderedEdge);
        }
      }
      nodes = renderedNodes;
      edges = renderedEdges;
      // Honour any selection openGraphAtNode pre-loaded into the
      // overlay state so the inspector opens on the right node.
      const pending = graphState.pendingSelectId;
      if (pending !== null && renderedNodes.some((n) => n.id === pending)) {
        selectedId = pending;
        graphState.inspectorOpen = true;
      }
      graphState.pendingSelectId = null;
    } catch (e) {
      error = (e as Error).message;
    } finally {
      loading = false;
    }
  }

  function mapLanguageNodes(input: LanguageGraphResponse["nodes"]): RenderedNode[] {
    return input.map((n): RenderedNode => {
      if (n.kind === "directory") {
        return { ...n, kind: "folder" };
      }
      return n;
    });
  }

  function mapLanguageEdges(input: LanguageGraphResponse["edges"]): RenderedEdge[] {
    return input.map((e) => ({
      source: e.source,
      target: stripDirectoryPrefix(e.target),
      kind: "language",
    }));
  }

  function mapFsNodes(fs: FsGraphResponse): RenderedNode[] {
    return fs.nodes.map((n): RenderedNode => {
      if (n.kind === "file") {
        return {
          kind: "file",
          id: n.id,
          label: n.name || n.path || "(drive)",
          path: n.path,
          missing: Boolean(n.broken),
        };
      }
      if (n.kind === "ghost") {
        return {
          kind: "file",
          id: n.id,
          label: n.name || n.target || n.id,
          path: n.path || n.id,
          missing: true,
        };
      }
      if (isFsDirectory(n)) {
        return {
          kind: "folder",
          id: n.id,
          label: `${n.name || "drive"}/`,
          path: n.path,
          files: 0,
          code: 0,
        };
      }
      return {
        kind: "mention",
        id: n.id,
        label: `${n.name}${n.broken ? " (broken)" : ""}`,
      };
    });
  }

  function mapFsEdges(fs: FsGraphResponse): RenderedEdge[] {
    return fs.edges.map((e): RenderedEdge => ({
      source: e.source,
      target: e.target,
      kind:
        e.kind === "contains"
          ? "contains"
          : e.kind === "symlink"
            ? "tag"
            : "mention",
      broken:
        e.kind === "symlink" &&
        Boolean(fs.nodes.find((n) => n.id === e.target)?.broken),
    }));
  }

  function mapGraphNode(n: GraphViewNode): RenderedNode | null {
    if (n.kind === "file" || n.kind === "tag" || n.kind === "mention" || n.kind === "language") {
      return n as RenderedNode;
    }
    if (n.kind === "media") {
      return {
        kind: "file",
        id: n.id,
        label: n.label,
        path: n.path,
        missing: n.missing,
      };
    }
    if (n.kind === "directory") {
      return {
        kind: "folder",
        id: n.id,
        label: `${n.label || "drive"}/`,
        path: n.path,
        files: n.files,
        code: n.code,
      };
    }
    return null;
  }

  /// Refetch the graph whenever the overlay opens, plus once on
  /// mount so the first open after a window reload has data ready.
  /// Idle overlays don't pay for an /api/graph round-trip.
  $effect(() => {
    if (visible) void load();
  });

  $effect(() => {
    if (!visible) driveDepthProbe = null;
  });

  $effect(() => {
    if (!visible) return;
    if (currentScope?.kind !== "drive" && currentScope?.kind !== "global") return;
    if (driveDepthProbe || driveDepthProbeLoading) return;
    void loadDriveDepthProbe();
  });

  $effect(() => {
    if (languageMode) return;
    const max = depthCap;
    if (graphState.depth < 1) {
      graphState.depth = 1;
    } else if (graphState.depth > max) {
      graphState.depth = max;
    }
  });

  $effect(() => {
    const nonce = graphReloadSignal.nonce;
    if (!visible) {
      seenGraphReloadNonce = nonce;
      return;
    }
    if (nonce === seenGraphReloadNonce) return;
    seenGraphReloadNonce = nonce;
    if (watchReloadTimer) clearTimeout(watchReloadTimer);
    watchReloadTimer = setTimeout(() => {
      watchReloadTimer = null;
      if (visible) {
        if (currentScope?.kind === "drive" || currentScope?.kind === "global") {
          driveDepthProbe = null;
          void loadDriveDepthProbe();
        }
        void load();
      }
    }, 250);
  });

  onDestroy(() => {
    if (watchReloadTimer) clearTimeout(watchReloadTimer);
  });

  /// Selection callback handed to GraphCanvas. Tapping a node
  /// flips the inspector open; background tap clears.
  function setSelected(id: string | null): void {
    selectedId = id;
    if (id !== null) graphState.inspectorOpen = true;
  }
</script>

{#if tab}
  {@render graphContent()}
{:else}
  <OverlayShell
    id="graph"
    open={visible}
    onClose={close}
    onBackdropContextMenu={onGraphContextMenu}
  >
    {@render graphContent()}
  </OverlayShell>
{/if}

{#snippet graphContent()}
  <div class="graph-tab" oncontextmenu={onGraphContextMenu} role="presentation">
  <div class="bar">
    <button
      type="button"
      class="chrome-btn"
      onclick={doToggleOverlayMaximized}
      title={overlayMaximized.on ? "Restore size" : "Maximize"}
      aria-label={overlayMaximized.on ? "Restore size" : "Maximize"}
    >
      {#if overlayMaximized.on}
        <Minimize2 size={14} strokeWidth={1.75} aria-hidden="true" />
      {:else}
        <Maximize2 size={14} strokeWidth={1.75} aria-hidden="true" />
      {/if}
    </button>
    <span class="scope-label">Scope</span>
    <select
      class="scope-select"
      value={graphState.scopeId}
      onchange={(e) =>
        (graphState.scopeId = (e.currentTarget as HTMLSelectElement).value)}
      title="graph scope"
    >
      {#each scopeOptions as opt (opt.id)}
        <option value={opt.id} disabled={opt.enabled === false}>
          {opt.label}
        </option>
      {/each}
    </select>
    <div class="filters">
      {#each ["link", "tag", "mention", "language", "img", "folder"] as const as kind (kind)}
        {@const driveLike =
          currentScope?.kind === "drive" || currentScope?.kind === "global"}
        {#if (!filesystemMode || (kind !== "img" && kind !== "language")) && (languageMode ? kind === "language" : kind !== "language" || driveLike) && (kind !== "folder" || filesystemMode || driveLike)}
          <label class="chip" class:on={show[kind]}>
            <input type="checkbox" bind:checked={show[kind]} />
            <span class="dot" style="background:{FILTER_COLORS[kind]}"></span>
            {#if filesystemMode}
              {kind === "link"
                ? "contains"
                : kind === "tag"
                  ? "symlink"
                  : kind === "mention"
                    ? "hardlink"
                    : "directory"}
            {:else}
              {kind === "mention" ? "contact" : kind === "img" ? "media" : kind}
            {/if}
            <span class="count">{counts[kind]}</span>
          </label>
        {/if}
      {/each}
    </div>
    <span class="bar-menu">
      <HamburgerMenu
        bind:this={menu}
        bind:open={menuOpen}
        width={POPOVER_WIDTH}
        height={POPOVER_HEIGHT}
      >
        {@render menuItems()}
      </HamburgerMenu>
    </span>
  </div>

  <div class="body">
  <div class="canvas">
    {#if loading}
      <div class="placeholder">loading graph…</div>
    {:else if error}
      <div class="placeholder error">{error}</div>
    {:else if nodes.length === 0}
      <div class="placeholder">
        {filesystemMode ? "no filesystem graph nodes for this scope" : languageMode ? "no language graph nodes for this drive yet" : "no markdown files in this drive yet"}
      </div>
    {/if}
    <div class="cy" class:dim={loading || !!error}>
      <GraphCanvas
        open={visible}
        {nodes}
        {edges}
        {visibleNodeIds}
        {visibleEdges}
        {focalIds}
        {selectedId}
        onSelect={setSelected}
        onContextMenu={onGraphContextMenu}
      />
    </div>
  </div>

  {#if graphState.inspectorOpen}
    <Inspector
      title="Details"
      bind:width={paneWidths.graph}
      onResize={persistPaneWidths}
      onClose={() => (graphState.inspectorOpen = false)}
    >
      {#if (selectedFsNode && isFsDirectory(selectedFsNode) && selectedFsNode.id === "") || (selectedNode?.kind === "folder" && selectedNode.id === "")}
        <!-- Drive root: same body the file browser hamburger
             menu's Directory row pops (DriveInfoBody) so the
             whole-drive config lives in one place across surfaces.
             Differentiated visually by GraphCanvas painting the
             "drive" sub-kind in a darker fill with the HardDrive
             glyph. -->
        <DriveInfoBody />
      {:else if selectedFsNode && (isFsDirectory(selectedFsNode) || selectedFsNode.kind === "file") && selectedFsNode.path !== undefined && !selectedFsNode.broken}
        <!-- Real fs-mode file or directory: render the same body as the
             file browser / editor inspector (counts, size, code
             report; tags / refs / backlinks for files) by routing
             through InspectorBody. FileInfoBody dispatches on
             entry.is_dir so the "file" selection variant covers both
             shapes. Both file and directory nodes can re-scope the
             filesystem graph from here; file keeps "Open in this pane"
             as the extra editor action. -->
        {@const fsPath = selectedFsNode.path}
        {@const fsKind = selectedFsNode.kind}
        <InspectorBody
          selection={{ kind: "file", path: fsPath }}
          showRefs
          onOpen={fsKind === "file"
            ? () => { void openInActivePane(fsPath); close(); }
            : undefined}
          onReveal={revealSelectedFsEntry}
          onSetAsScope={fsKind === "file" || isFsDirectory(selectedFsNode)
            ? () => {
                // Re-scope the current fs graph to this node's
                // neighbourhood. Depth resets to 1; the caller can
                // widen via the slider.
                scopeFsGraphFromHere(fsPath, isFsDirectory(selectedFsNode!));
                selectedId = selectedFsNode!.id;
              }
            : undefined}
          onNavigate={(p) => {
            const peer = fsNodes.find((n) => n.path === p);
            if (peer) {
              selectedId = peer.id;
              graphState.inspectorOpen = true;
            }
          }}
        />
      {:else if selectedFsNode}
        <div class="ghost-body">
          <header class="head">
            <KindChip
              kind={isFsDirectory(selectedFsNode) ? "folder" : selectedFsNode.kind === "file" ? "document" : "binary"}
              block
              ghost={selectedFsNode.kind === "ghost" || selectedFsNode.broken === true}
            />
          </header>
          <h3 class="title" title={selectedFsNode.path || selectedFsNode.target || selectedFsNode.id}>
            {selectedFsNode.name || selectedFsNode.path || selectedFsNode.id || "(drive)"}
          </h3>
          <div class="path mono">{selectedFsNode.path || selectedFsNode.target || selectedFsNode.id}</div>
          {#if selectedFsNode.target}
            <div class="missing">target: {selectedFsNode.target}</div>
          {/if}
          {#if selectedFsNode.outside}
            <div class="missing">target is outside this drive</div>
          {:else if selectedFsNode.broken}
            <div class="missing">missing or unreadable target</div>
          {/if}
          {#if selectedFsNode.kind === "file" && selectedFsNode.path}
            <button class="open-fs" onclick={() => { void openInActivePane(selectedFsNode!.path); close(); }}>
              Open in this pane
            </button>
          {/if}
        </div>
      {:else if selectedNode && selectedNode.kind === "file" && isFileGhost}
        <!-- Ghost: either an explicit broken-link target, or the
             graph claims the file exists but it's not in the current
             tree listing (stale search index, common after a bulk
             drive change). FileInfoBody can't render either; surface
             inline inside the shared Inspector header. -->
        {@const ghostKind = classifyFileKind(
          selectedNode.path,
          selectedNode.node_kind,
        ) as FileKind}
        {@const hint = selectedNode.missing
          ? "file does not exist (broken-link target)"
          : ghostIndexerHint ?? "not in the current file listing (try Reload / chan index)"}
        <div class="ghost-body">
          <header class="head">
            <KindChip kind={ghostKind} block ghost />
          </header>
          <h3 class="title" title={selectedNode.path}>{selectedNode.label}</h3>
          <div class="path mono">{selectedNode.path}</div>
          <div class="missing">{hint}</div>
        </div>
      {:else}
        <InspectorBody
          selection={inspectorSelection}
          onOpen={
            inspectorSelection?.kind === "file"
              ? openSelectedFile
              : inspectorSelection?.kind === "mention" && selectedContactPath
                ? () => {
                    // Mention/contact "Open in this pane": route the
                    // resolved contact file (looked up via
                    // tree.kind === "contact") through the active
                    // pane and close the graph.
                    void openInActivePane(selectedContactPath!);
                    close();
                  }
                : undefined
          }
          onReveal={revealSelectedFile}
          onNavigate={selectByPath}
          onContactNavigate={selectByPath}
          onSetAsScope={
            selectedNode?.kind === "tag" ||
            (selectedNode?.kind === "mention" && selectedContactPath) ||
            (selectedNode?.kind === "file" && !selectedNode.missing)
              ? () => {
                  // "Graph from here" inside the graph re-scopes the
                  // current graph. Tag: to the tag's neighbourhood.
                  // Mention: to the resolved contact's file so the
                  // user can use a contact as a graph anchor without
                  // leaving the overlay. File (incl. images): to
                  // that file's own neighbourhood, with the file
                  // pinned as the focal node. Depth resets to 1 so
                  // a freshly-scoped graph always starts tight; the
                  // user can widen it back via the slider.
                  graphState.depth = 1;
                  if (selectedNode?.kind === "tag") {
                    graphState.scopeId = `tag:${selectedNode.id}`;
                    graphState.pendingSelectId = selectedNode.id;
                  } else if (
                    selectedNode?.kind === "mention" &&
                    selectedContactPath
                  ) {
                    graphState.scopeId = `file:${selectedContactPath}`;
                    const fileNode = nodes.find(
                      (n) =>
                        n.kind === "file" &&
                        n.path === selectedContactPath,
                    );
                    if (fileNode) {
                      graphState.pendingSelectId = fileNode.id;
                      selectedId = fileNode.id;
                    }
                  } else if (
                    selectedNode?.kind === "file" &&
                    !selectedNode.missing
                  ) {
                    graphState.scopeId = `file:${selectedNode.path}`;
                    graphState.pendingSelectId = selectedNode.id;
                    selectedId = selectedNode.id;
                  }
                }
              : undefined
          }
          documentsOverride={selectionDocumentsInScope}
        />
      {/if}
    </Inspector>
  {/if}
  </div>
  <div class="statusbar">
    <span class="stat">
      {visibleNodeIds.size}/{nodes.length} nodes · {visibleEdges.length}/{edges.length} edges
      {#if filesystemMode && fsTruncated} · truncated{/if}
    </span>
    <span class="hint">
      {filesystemMode ? "filesystem graph" : languageMode ? "language graph" : "semantic graph"} · drag to pan · scroll to zoom · click to inspect
    </span>
  </div>
  </div>
{/snippet}

{#snippet menuItems()}
  <li>
    <button role="menuitem" onclick={toggleInspector}>
      {#if graphState.inspectorOpen}
        <ArrowRight size={16} strokeWidth={1.75} aria-hidden="true" />
      {:else}
        <ArrowLeft size={16} strokeWidth={1.75} aria-hidden="true" />
      {/if}
      <span class="menu-row-label">
        {graphState.inspectorOpen ? "Hide Details" : "Show Details"}
      </span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  <li class="sep" role="separator"></li>
  <!-- Depth slider is always in the menu so it doesn't disappear
       under the user when the scope toggles. Disabled on
       drive / global scopes (those always render everything
       regardless of hop count) so the affordance stays visible. -->
  {@const depthDisabled =
    !languageMode &&
    (!currentScope ||
      currentScope.kind === "drive" ||
      currentScope.kind === "global")}
  <li>
    <div class="menu-slider-row" class:disabled={depthDisabled}>
      <span class="menu-slider-label">Depth</span>
      <input
        type="range"
        min={languageMode ? "0" : "1"}
        max={depthCap}
        step="1"
        bind:value={graphState.depth}
        disabled={depthDisabled}
        onmousedown={(e) => e.stopPropagation()}
        aria-label="depth"
      />
      <span class="menu-slider-value">{languageMode && graphState.depth === 0 ? "max" : graphState.depth}</span>
    </div>
  </li>
  <li class="sep" role="separator"></li>
  <li>
    <button role="menuitem" onclick={reloadGraph}>
      <span class="glyph" aria-hidden="true">↻</span>
      <span class="menu-row-label">Reload</span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  <li class="sep" role="separator"></li>
  <li>
    <button role="menuitem" onclick={doOpenSettings}>
      <Settings size={14} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">Settings</span>
      <span class="menu-row-chord">{chordFor("app.settings.toggle") ?? ""}</span>
    </button>
  </li>
{/snippet}

<style>
  .graph-tab {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    min-width: 0;
    background: var(--bg);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.5rem;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
    font-size: 14px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }
  .scope-label {
    color: var(--text-secondary);
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .scope-select {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 6px;
    font: inherit;
    font-size: 14px;
    max-width: 280px;
    cursor: pointer;
  }
  .scope-select:focus { outline: none; border-color: var(--link); }
  /* Window-manager chrome: maximize/restore on the far left of the
     bar, close on the far right. */
  .chrome-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 24px;
    padding: 0;
    background: var(--bg);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    transition: color 0.15s ease, border-color 0.15s ease;
    flex-shrink: 0;
  }
  .chrome-btn:hover {
    color: var(--text);
    border-color: var(--btn-hover);
  }
  /* Slider row used inside the hamburger menu. Mirrors the file
     tab menu's page-width row so all in-menu sliders read alike. */
  :global(.menu-slider-row) {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    color: var(--text-secondary);
    font-size: 13px;
  }
  :global(.menu-slider-label) {
    color: var(--text);
    min-width: 4.5em;
  }
  :global(.menu-slider-row input[type="range"]) {
    flex: 1;
    accent-color: var(--link);
  }
  :global(.menu-slider-value) {
    font-variant-numeric: tabular-nums;
    color: var(--text);
    min-width: 2ch;
    text-align: right;
  }
  /* Disabled state: dim the row and disable pointer interactions
     on the range input. The native disabled attribute already
     blocks dragging; this is the visual cue. */
  :global(.menu-slider-row.disabled) {
    opacity: 0.4;
    cursor: not-allowed;
  }
  :global(.menu-slider-row.disabled input[type="range"]) {
    cursor: not-allowed;
  }
  .statusbar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.25rem 0.5rem;
    background: var(--bg-card);
    border-top: 1px solid var(--border);
    font-size: 13px;
    color: var(--text-secondary);
    flex-shrink: 0;
    min-height: 22px;
  }
  .stat {
    font-variant-numeric: tabular-nums;
    color: var(--text);
  }
  .hint {
    margin-left: auto;
    opacity: 0.8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .filters {
    display: flex;
    gap: 0.35rem;
    align-items: center;
    flex-wrap: wrap;
  }
  .bar-menu {
    margin-left: auto;
    display: flex;
    align-items: center;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 1px 6px;
    border: 1px solid var(--btn-border);
    border-radius: 12px;
    cursor: pointer;
    user-select: none;
    color: var(--text-secondary);
    background: var(--btn-bg);
  }
  .chip.on {
    color: var(--text);
    border-color: var(--btn-hover);
  }
  .chip input {
    display: none;
  }
  .chip .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .chip .count {
    font-variant-numeric: tabular-nums;
    opacity: 0.75;
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
    min-width: 0;
  }
  .canvas {
    flex: 1;
    min-height: 0;
    min-width: 0;
    position: relative;
    overflow: hidden;
  }
  /* Cytoscape mount: positioned absolute inside the relative
     .canvas parent. Cytoscape's example pattern; without explicit
     positioning its internal canvases can resolve their absolute
     positioning to the wrong ancestor and end up sized wrong. */
  .cy {
    position: absolute;
    inset: 0;
    /* Smooth fade when buildCytoscape clears the inline opacity:0
       it sets at the start of a rebuild; gives a gentler reveal
       than a hard pop into the d3-force animation. */
    transition: opacity 200ms ease-out;
  }
  .cy.dim {
    opacity: 0.4;
  }
  /* Inline ghost branch for nodes that exist in the graph but
     not in the tree (FileInfoBody can't render those). Mounted
     inside the shared `<Inspector>` wrapper, so we only style
     the body — the title bar / close × comes from Inspector. */
  .ghost-body {
    padding: 0.6rem 0.7rem 0.8rem 0.7rem;
    font-size: 12.5px;
  }
  .ghost-body .head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.4rem;
  }
  .ghost-body .title {
    margin: 0 0 0.15rem 0;
    font-size: 16px;
    font-weight: 600;
    word-break: break-word;
  }
  .ghost-body .path {
    color: var(--text-secondary);
    font-size: 13px;
    margin-bottom: 0.5rem;
    word-break: break-all;
  }
  .ghost-body .mono { font-family: ui-monospace, monospace; }
  .ghost-body .missing {
    color: var(--warn-text);
    font-style: italic;
    margin: 0.3rem 0 0.6rem 0;
    font-size: 11.5px;
  }
  .placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-style: italic;
    pointer-events: none;
  }
  .placeholder.error {
    color: #d33;
    font-style: normal;
  }
</style>
