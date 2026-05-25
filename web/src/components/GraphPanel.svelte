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

  import { api } from "../api/client";
  import type {
    FsGraphNode,
    FsGraphResponse,
    GraphView,
    GraphViewEdge,
    GraphViewNode,
    LanguageGraphResponse,
  } from "../api/types";
  import {
    canReopenClosedTab,
    openInActivePane,
    reopenClosedTab,
    type GraphTab,
  } from "../state/tabs.svelte";
  import {
    availableGraphScopes,
    graphReloadSignal,
    graphOverlay,
    paneWidths,
    persistPaneWidths,
    revealPathInBrowser,
    surfaceThemeOverride,
    tree,
  } from "../state/store.svelte";
  import { onDestroy } from "svelte";
  import { type ScopeOption } from "../state/scope.svelte";
  import ResizeHandle from "./ResizeHandle.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";
  import { clampMenu } from "./menuClamp";
  import { portal } from "./portal";
  import { tabMenu, closeTabMenu } from "../state/tabMenu.svelte";
  import {
    FileText,
    Folder,
    HardDrive,
    Hash,
    History,
    Settings2,
    X,
  } from "lucide-svelte";
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
    onFlip,
  }: {
    tab?: GraphTab;
    onClose?: () => void;
    onFlip?: () => void;
  } = $props();

  const graphState = $derived(tab ?? graphOverlay);
  const visible = $derived(tab ? true : graphOverlay.open);

  /// `fullstack-64`: the scope-selector dropdown is gone (Cmd+K 3
  /// + "Graph from here" + inspector reveal are the canonical
  /// scope-setting paths). The `scopeOptions` listing is still
  /// useful here for its rich labels when the active scope is one
  /// the layout knows about, but a context-aware spawn can land
  /// the user on a `file:`/`dir:` scope that isn't in any tab.
  /// Synthesize a matching ScopeOption from `scopeId` in that
  /// case so the rest of the panel (`filesystemMode`,
  /// `seedIds`, BFS shape, etc.) still has a `currentScope.kind`
  /// to branch on.
  ///
  /// The synthesis also removes the `fullstack-57` snap-back bug —
  /// the old effect resetted `scopeId` to `defaultScopeId()`
  /// whenever the lookup missed, clobbering the spawn's
  /// `file:` scope before the user saw it.
  const scopeOptions = $derived<ScopeOption[]>(availableGraphScopes());

  const currentScope = $derived<ScopeOption | null>(
    scopeOptions.find((o) => o.id === graphState.scopeId)
      ?? synthesizeScope(graphState.scopeId),
  );

  function synthesizeScope(scopeId: string): ScopeOption | null {
    if (scopeId === "drive") return { id: "drive", kind: "drive", label: "drive" };
    if (scopeId === "global") return { id: "global", kind: "global", label: "global" };
    if (scopeId.startsWith("file:")) {
      const path = scopeId.slice("file:".length);
      if (!path) return null;
      return { id: scopeId, kind: "file", label: path, path };
    }
    if (scopeId.startsWith("dir:")) {
      const path = scopeId.slice("dir:".length);
      if (!path) return null;
      return { id: scopeId, kind: "dir", label: path, path };
    }
    if (scopeId.startsWith("tag:")) {
      const nodeId = scopeId.slice("tag:".length);
      if (!nodeId) return null;
      return { id: scopeId, kind: "tag", label: nodeId, nodeId };
    }
    if (scopeId.startsWith("git_repo:")) {
      const root = scopeId.slice("git_repo:".length);
      if (!root) return null;
      return { id: scopeId, kind: "git_repo", label: root, root };
    }
    return null;
  }

  /// `fullstack-a-33`: ancestor breadcrumb for the current scope. Each
  /// entry is one clickable hop in the path from the drive root down
  /// to the current scope's root. Click an ancestor → mutate
  /// `graphState.scopeId` in place (no new tab). The chain renders
  /// only for path-based scopes (`drive` / `dir:` / `file:`); tag /
  /// git_repo / global scopes return an empty list so the breadcrumb
  /// band is hidden for those modes.
  ///
  /// The list always starts with the drive root so the user can hop
  /// back up to drive scope from anywhere. The final entry is the
  /// CURRENT scope, rendered as the active step (not clickable).
  type Crumb = { label: string; scopeId: string; current: boolean };
  const scopeAncestors = $derived.by<Crumb[]>(() => {
    if (!currentScope) return [];
    if (currentScope.kind === "drive" || currentScope.kind === "global") {
      return [{ label: "drive", scopeId: "drive", current: true }];
    }
    if (currentScope.kind !== "file" && currentScope.kind !== "dir") {
      return [];
    }
    const path = currentScope.path;
    if (!path) {
      return [{ label: "drive", scopeId: "drive", current: true }];
    }
    const out: Crumb[] = [{ label: "drive", scopeId: "drive", current: false }];
    const segments = path.split("/");
    for (let i = 0; i < segments.length; i++) {
      const sub = segments.slice(0, i + 1).join("/");
      const isLast = i === segments.length - 1;
      // Intermediate hops are always directory scopes; the leaf
      // mirrors the current scope's kind so a file-scoped graph
      // ends on `file:`, a dir-scoped one on `dir:`.
      const scopeId =
        isLast && currentScope.kind === "file"
          ? `file:${sub}`
          : `dir:${sub}`;
      out.push({ label: segments[i], scopeId, current: isLast });
    }
    return out;
  });

  /// Re-scope the current graph in place. Mirrors the existing
  /// semantic-mode `onSetAsScope` handler: depth resets to 1 so a
  /// freshly-scoped graph starts tight; selection clears so the
  /// inspector lands on the new scope's body. Used by the
  /// breadcrumb's click handler.
  function rescopeFromHere(scopeId: string): void {
    if (scopeId === graphState.scopeId) return;
    graphState.scopeId = scopeId;
    graphState.depth = 1;
    graphState.pendingSelectId = null;
    selectedId = null;
  }

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
  /// `fullstack-a-52` G10: `link` removed from the user-facing
  /// FilterKind — link edges always render now (visibility is
  /// implicit via endpoint visibility). The `link` slot on
  /// `GraphFilters` (store.svelte.ts) stays for URL-hash
  /// back-compat but isn't consumed here.
  /// `fullstack-a-57`: `markdown` + `source` FileBucket toggles
  /// added; default ON. Consumes `-a-51`'s SPA-side
  /// `classifyFile` helper to dispatch file nodes into the
  /// markdown / source / binary buckets without a chan-server
  /// emit change (the audit found `GraphNodeView::File` doesn't
  /// carry the `bucket` field even though `systacean-16` added
  /// it to `ReportFileStats`; reusing the client-side classifier
  /// follows the `-a-51` precedent).
  type FilterKind =
    | "tag"
    | "mention"
    | "language"
    | "img"
    | "folder"
    | "markdown"
    | "source";

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
  let graphLoadAbort: AbortController | null = null;
  let graphLoadSeq = 0;

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

  /// `fullstack-a-56` shallow-scope cue: when the scope's
  /// `depthCap` is 1 (single-file graph with no further forward
  /// hops; tag scope with only direct neighbours; etc.) the
  /// slider can't meaningfully be dragged. Surface that via a
  /// `[max]` suffix + disable the slider so the user can see at
  /// a glance there's nothing more to reveal. Gates: only fires
  /// outside language mode (which has its own depth=0 "max"
  /// affordance) + only when the slider would otherwise be
  /// enabled (depthDisabled is the drive/global guard).
  const depthShallow = $derived.by(() => {
    if (languageMode) return false;
    const disabled =
      !currentScope ||
      currentScope.kind === "drive" ||
      currentScope.kind === "global";
    if (disabled) return false;
    return depthCap <= 1;
  });

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

  /// Hamburger menu used by the overlay variant's bar + by the
  /// in-canvas right-click context menu. The depth slider, reload,
  /// and details toggle all live inside it. The tab variant drops
  /// the bar entirely per `fullstack-68`; the right-click bubble
  /// re-uses the same `menuItems` snippet so the items stay one
  /// source of truth.
  let menu: HamburgerMenu | undefined = $state();
  let menuOpen = $state(false);
  /// Bigger than the other overlays because the menu carries the
  /// scope-conditional depth slider, filters, and tab footer rows.
  const POPOVER_HEIGHT = 340;
  const POPOVER_WIDTH = 260;

  /// `fullstack-68`: tab right-click bubble state. Open when the
  /// shared tab-menu state addresses THIS tab; positioned via the
  /// stored anchor through `clampMenu` so the bubble stays on-
  /// screen even when the tab sits near the viewport edge.
  const tabMenuOpen = $derived(tab !== undefined && tabMenu.openForTabId === tab.id);
  const tabMenuPos = $derived.by(() => {
    const a = tabMenu.anchor;
    if (!a) return { x: 0, y: 0 };
    return { x: Math.round(a.left), y: Math.round(a.bottom + 4) };
  });

  function onTabMenuKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape" && tabMenuOpen) {
      e.preventDefault();
      closeTabMenu();
    }
  }

  /// Dismiss when the click lands outside the bubble AND outside
  /// the tab strip (the tab row's own click handler toggles the
  /// state; without this guard the global handler races the row
  /// handler and closes the menu before the row can re-open it).
  function onTabMenuPointerDown(e: PointerEvent): void {
    if (!tabMenuOpen) return;
    const t = e.target as Node | null;
    if (!t) return;
    const bubble = document.querySelector(".tab-menu-bubble");
    if (bubble && bubble.contains(t)) return;
    const trigger = (t as Element).closest?.(".tab");
    if (trigger) return;
    closeTabMenu();
  }
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
  /// `fullstack-a-57` extension: file-class buckets (`doc` for
  /// markdown, `source` for code/config, `binary` for everything
  /// else not covered by img/contact) added so the new markdown +
  /// source filter chips can route file nodes into their buckets.
  /// Mirrors `GraphCanvas.svelte`'s helper of the same name, with
  /// the addition of `MARKDOWN_EXT_RE` + `SOURCE_EXT_RE` from
  /// `-a-51`. The two helpers stay separate copies for now —
  /// they're parallel SPA-side helpers with the same regex set; a
  /// future cleanup task could extract them into a shared module.
  const MEDIA_EXT_RE_FA57 = /\.(png|jpe?g|gif|webp|svg|avif|bmp)$/i;
  const MARKDOWN_EXT_RE_FA57 = /\.(md|txt)$/i;
  const SOURCE_EXT_RE_FA57 =
    /\.(rs|py|ts|tsx|js|jsx|mjs|cjs|go|c|cc|cpp|cxx|h|hh|hpp|java|kt|swift|rb|php|cs|sh|bash|zsh|fish|pl|lua|toml|yaml|yml|json|jsonc|ini|conf|cfg|env|xml|html|htm|css|scss|sass|less|vue|svelte|sql|graphql|gql|proto|elm|ex|exs|erl|hs|lhs|ml|mli|fs|fsx|clj|cljs|cljc|edn|jl|nim|d|dart|zig|odin|v|vhd|vhdl|sv|verilog|asm|s|f|f90|f95|tex|R|r)$/i;

  function classifyFile(
    path: string,
    nodeKind: "contact" | undefined,
  ): "doc" | "img" | "contact" | "source" | "binary" {
    if (MEDIA_EXT_RE_FA57.test(path)) return "img";
    if (nodeKind === "contact") return "contact";
    if (MARKDOWN_EXT_RE_FA57.test(path)) return "doc";
    if (SOURCE_EXT_RE_FA57.test(path)) return "source";
    return "binary";
  }

  // `fullstack-64`: the overlay-maximize toggle helper was removed
  // alongside its button. The maximize state machinery stays in
  // `pageWidth.svelte` for any future consumer; this panel is no
  // longer one.

  async function reloadGraph(): Promise<void> {
    menu?.close();
    if (currentScope?.kind === "drive" || currentScope?.kind === "global") {
      driveDepthProbe = null;
      await loadDriveDepthProbe();
    }
    await load();
  }

  function flipToSettings(): void {
    menu?.close();
    closeTabMenu();
    onFlip?.();
  }

  function doReopenClosedTab(): void {
    menu?.close();
    closeTabMenu();
    reopenClosedTab();
  }

  function closeFromMenu(): void {
    menu?.close();
    closeTabMenu();
    close();
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

  function onGraphContextMenu(e: MouseEvent): void {
    const t = e.target as HTMLElement | null;
    // Don't hijack right-click on inputs / native select / filter
    // chips so the browser's native UI fires there. The scope-select
    // is gone (`fullstack-64`); `select, input` covers any other
    // native control that lands in the bar later.
    if (t?.closest("select, input, .filters")) return;
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
      // `fullstack-a-52` G9: forward-only BFS (outgoing edges
      // only). See the second BFS site below for the rationale.
      for (let i = 0; i < graphState.depth; i++) {
        const next = new Set<string>();
        for (const e of edges) {
          if (frontier.has(e.source) && !visited.has(e.target)) {
            next.add(e.target);
            visited.add(e.target);
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
    // `fullstack-a-52` G9: forward-only BFS. Previously the
    // BFS followed edges in both directions
    // (`frontier.has(e.source)` OR `frontier.has(e.target)`), which
    // hid the "depth slider reveals forward nodes" semantic @@Alex
    // wanted. Restricting to outgoing edges only makes the slider
    // read as "expand from the root in the direction edges point"
    // — markdown links emanate from the root doc; contains edges
    // emanate from the root directory toward its children; etc.
    for (let i = 0; i < graphState.depth; i++) {
      const next = new Set<string>();
      for (const e of edges) {
        if (frontier.has(e.source) && !visited.has(e.target)) {
          next.add(e.target);
          visited.add(e.target);
        }
      }
      if (next.size === 0) break;
      frontier = next;
    }
    // `fullstack-a-58` parent-edge invariant: pull each in-scope
    // node's ancestor chain via `contains` edges. The forward-only
    // BFS above expands DOWN from the seed; contains edges point
    // parent→child, so the parent is UPSTREAM of the seed (the
    // file is the target end of the parent→file contains edge).
    // Without this pass, file-scope graphs render the file but
    // not its parent directory + the user can't click-up via the
    // graph. Per @@Alex's spec: "every node has an inbound
    // contains edge from a parent directory unless folder filter
    // is OFF" — folder filter hiding is handled later by
    // `hiddenFolderIds` so we always include the chain here.
    //
    // Implementation: iterate to a fixed point, adding `source`
    // of every contains edge whose `target` is already in scope.
    // The contains-edge subgraph is a forest (each file/dir has
    // at most one parent) so this terminates in at most O(depth)
    // iterations.
    let pulled = true;
    while (pulled) {
      pulled = false;
      for (const e of edges) {
        if (
          e.kind === "contains" &&
          visited.has(e.target) &&
          !visited.has(e.source)
        ) {
          visited.add(e.source);
          pulled = true;
        }
      }
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

  /// `fullstack-a-57`: file nodes hidden when the markdown chip is OFF.
  /// Bucket = `classifyFile === "doc"` (.md / .txt per `-a-51`'s
  /// SPA-side classifier). The `contact` discriminator is gated by
  /// the `mention` chip (existing hiddenContactIds) and image-class
  /// files by `img`; this hidden-set covers markdown specifically.
  const hiddenMarkdownIds = $derived.by(() => {
    const ids = new Set<string>();
    if (show.markdown) return ids;
    for (const n of nodes) {
      if (n.kind === "file" && classifyFile(n.path, n.node_kind) === "doc") {
        ids.add(n.id);
      }
    }
    return ids;
  });

  /// `fullstack-a-57`: file nodes hidden when the source chip is OFF.
  /// Bucket = `classifyFile === "source"` (recognised code / config
  /// extensions per `-a-51`).
  const hiddenSourceIds = $derived.by(() => {
    const ids = new Set<string>();
    if (show.source) return ids;
    for (const n of nodes) {
      if (n.kind === "file" && classifyFile(n.path, n.node_kind) === "source") {
        ids.add(n.id);
      }
    }
    return ids;
  });

  function edgeVisibleByChip(kind: RenderedEdgeKind): boolean {
    if (kind === "contains") return show.folder;
    if (kind === "group") return true;
    // `fullstack-a-52` G10: link edges always render. Per @@Alex's
    // framing, the link filter doesn't make sense — link visibility
    // is implicit (an edge renders iff both endpoints render under
    // the current node-type filters + depth). The `link` slot on
    // `GraphFilters` stays for wire-format / URL-hash back-compat
    // but is no longer consumed by the UI.
    if (kind === "link") return true;
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
        // `fullstack-a-57`: hide edges touching markdown / source
        // file nodes the user has filtered out. Symmetric with the
        // existing img / contact / folder gates.
        !hiddenMarkdownIds.has(e.source) &&
        !hiddenMarkdownIds.has(e.target) &&
        !hiddenSourceIds.has(e.source) &&
        !hiddenSourceIds.has(e.target) &&
        (scopedNodeIds === null ||
          (scopedNodeIds.has(e.source) && scopedNodeIds.has(e.target))),
    ),
  );
  const visibleNodeIds = $derived.by(() => {
    const ids = new Set<string>();
    for (const n of nodes) {
      if (scopedNodeIds !== null && !scopedNodeIds.has(n.id)) continue;
      if (hiddenFolderIds.has(n.id)) continue;
      // `fullstack-a-57`: skip file nodes that the markdown / source
      // chips have hidden. The img + contact gates still apply.
      if (
        n.kind === "file" &&
        !hiddenImageIds.has(n.id) &&
        !hiddenContactIds.has(n.id) &&
        !hiddenMarkdownIds.has(n.id) &&
        !hiddenSourceIds.has(n.id)
      ) {
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

  /// Chip counts.
  ///
  /// `fullstack-a-63` semantic correction: chip counts are NODE
  /// counts, not edge counts. Pre-`-a-63` shape tallied edges per
  /// kind (so mention chip showed 1973-2000 mention-edge fan-in
  /// across only ~48 distinct contact nodes — ~40x over-tally).
  /// User reads the chip as "how many of THIS thing is in the
  /// graph", which is the node count. Edge counts are the
  /// rendered-edge population, which is a different concept and
  /// not what the chip needs to expose.
  ///
  /// The contact / mention chip count adds:
  ///   * `mention`-kind nodes (handle nodes the parser emits).
  ///   * Contact-discriminated FILE nodes (`node_kind === "contact"`)
  ///     since the mention chip toggle hides BOTH (see
  ///     `hiddenContactIds`).
  ///
  /// The img chip count covers media-class file nodes (the chip
  /// toggle hides those). The folder chip count covers folder-kind
  /// nodes (the chip toggle hides those). Markdown / source chips
  /// added in `-a-57` already used node-tally semantics; preserved.
  const counts = $derived.by(() => {
    const c: Record<FilterKind, number> = {
      tag: 0,
      mention: 0,
      language: 0,
      img: 0,
      folder: 0,
      markdown: 0,
      source: 0,
    };
    for (const n of nodes) {
      if (n.kind === "tag") {
        c.tag++;
        continue;
      }
      if (n.kind === "mention") {
        c.mention++;
        continue;
      }
      if (n.kind === "language") {
        c.language++;
        continue;
      }
      if (n.kind === "folder") {
        c.folder++;
        continue;
      }
      if (n.kind !== "file") continue;
      const cls = classifyFile(n.path, n.node_kind);
      if (cls === "img") c.img++;
      else if (cls === "contact") c.mention++;
      else if (cls === "doc") c.markdown++;
      else if (cls === "source") c.source++;
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
  /// server's resolver couldn't find it on disk — i.e. a genuine
  /// broken-link / deleted-file ghost. The server is the source of
  /// truth: post-`systacean-2` its resolver covers all on-disk files
  /// (markdown + non-markdown), so the previous SPA-side fallback of
  /// also checking the lazy-loaded FB tree's `tree.entries` was
  /// flagging every real file under an un-expanded subtree as a
  /// false ghost. Drop the lazy-tree check; trust the server flag.
  const isFileGhost = $derived<boolean>(
    selectedNode != null &&
      selectedNode.kind === "file" &&
      selectedNode.missing === true,
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

  /// `fullstack-a-67` slice 1b: click on the scope-header row in the
  /// graph tab-menu opens the inspector for the current scope. Maps
  /// the scope kind to the matching node id in the current graph
  /// nodes list; drive root + tag have stable ids, file/dir need a
  /// path-based lookup. No-op when the scope doesn't have a
  /// corresponding node in this view (e.g. global scope, or a file
  /// scope whose file isn't in the response).
  function openScopeHeaderInspector(): void {
    if (!currentScope) return;
    let nodeId: string | null = null;
    if (currentScope.kind === "drive" || currentScope.kind === "global") {
      // Drive root node carries id="" in the filesystem-merged
      // layer. Global has no first-class node; fall through to
      // no-op (the selection would highlight nothing useful).
      if (currentScope.kind === "drive") {
        nodeId = "";
      }
    } else if (currentScope.kind === "tag") {
      nodeId = currentScope.nodeId;
    } else if (currentScope.kind === "file") {
      // File-kind nodes carry their path as the id when emitted
      // from the markdown layer + a synthesized id from the
      // filesystem layer. Lookup by path matches both shapes.
      const found = nodes.find(
        (n) => n.kind === "file" && n.path === currentScope.path,
      );
      if (found) nodeId = found.id;
    } else if (currentScope.kind === "dir" || currentScope.kind === "git_repo" || currentScope.kind === "group") {
      // Directory nodes' ids carry a `directory:` prefix in the
      // merged layer; the SPA normalises `kind` to `folder` at
      // load. Match by path against folder-kind nodes.
      const path =
        currentScope.kind === "git_repo"
          ? currentScope.root
          : currentScope.kind === "group"
            ? null
            : currentScope.path;
      if (path !== null) {
        const found = nodes.find(
          (n) => n.kind === "folder" && n.path === path,
        );
        if (found) nodeId = found.id;
      }
    }
    if (nodeId === null) return;
    selectedId = nodeId;
    graphState.inspectorOpen = true;
    closeTabMenu();
  }

  /// Click handler for link / backlink / tag-doc entries surfaced by
  /// the shared InspectorBody. The other surfaces (file browser,
  /// search) treat onNavigate as "open in the editor", but here the
  /// user is exploring the graph: route to a select-in-canvas instead
  /// so the inspector keeps following the user as they hop along
  /// references. The "Open" button (onOpen) is still
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
          : selectedNode.kind === "folder"
            ? {
                // `fullstack-a-50` G3: directory nodes route to
                // DirectoryInfoBody via the new "directory" kind on
                // InspectorSelection. Backend emits `directory` for
                // the main /api/graph filesystem layer; GraphPanel
                // normalises that to `folder` for `RenderedNode`
                // (see `kind: "folder"` mappings at the data load
                // step). Both surfaces map to the same inspector.
                kind: "directory",
                path: selectedNode.path,
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
    tag: EDGE_COLORS.tag,
    mention: EDGE_COLORS.mention,
    language: EDGE_COLORS.language,
    img: "var(--g-img)",
    folder: "var(--g-folder)",
    // `fullstack-a-57`: FileBucket chip swatch colours. Markdown
    // tracks `--g-doc` (orange) per `-a-51`'s G6 palette; source
    // tracks `--g-source` (royalblue). Binary nodes have no chip;
    // the `--g-binary` slot still drives their canvas fill but the
    // user can't toggle them on/off.
    markdown: "var(--g-doc)",
    source: "var(--g-source)",
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
  function renderableGraphEdge(e: GraphViewEdge): RenderedEdge | null {
    if (
      e.kind === "link" ||
      e.kind === "tag" ||
      e.kind === "mention" ||
      e.kind === "contains" ||
      e.kind === "language"
    ) {
      return e as RenderedEdge;
    }
    return null;
  }

  function graphEdgeKey(e: GraphViewEdge): string {
    return `${e.source}\u0000${e.target}\u0000${e.kind}\u0000${e.rank ?? ""}`;
  }

  async function load(): Promise<void> {
    const seq = ++graphLoadSeq;
    graphLoadAbort?.abort();
    const controller = new AbortController();
    graphLoadAbort = controller;
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
        if (seq !== graphLoadSeq) return;
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
        if (seq !== graphLoadSeq) return;
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
      const renderedNodesById = new Map<string, RenderedNode>();
      const renderedEdgesByKey = new Map<string, RenderedEdge>();
      fsNodes = [];
      fsTruncated = false;
      nodes = [];
      edges = [];
      const publish = (): void => {
        if (seq !== graphLoadSeq) return;
        nodes = [...renderedNodesById.values()];
        edges = [...renderedEdgesByKey.values()];
        const pending = graphState.pendingSelectId;
        if (pending !== null && renderedNodesById.has(pending)) {
          selectedId = pending;
          graphState.inspectorOpen = true;
          graphState.pendingSelectId = null;
        }
      };
      await api.graphStream(
        {
          ...graphScope,
          depth: Math.max(graphState.depth, 1),
        },
        {
          signal: controller.signal,
          onNodes(batch) {
            for (const n of batch) {
              const mapped = mapGraphNode(n);
              if (mapped) renderedNodesById.set(mapped.id, mapped);
            }
            publish();
          },
          onEdges(batch) {
            for (const e of batch) {
              const mapped = renderableGraphEdge(e);
              if (mapped) renderedEdgesByKey.set(graphEdgeKey(e), mapped);
            }
            publish();
          },
        },
      );
      if (seq !== graphLoadSeq) return;
      publish();
      // Honour any selection openGraphAtNode pre-loaded into the
      // overlay state so the inspector opens on the right node.
      const pending = graphState.pendingSelectId;
      if (pending !== null && renderedNodesById.has(pending)) {
        selectedId = pending;
        graphState.inspectorOpen = true;
      } else if (pending !== null) {
        graphState.pendingSelectId = null;
      }
    } catch (e) {
      if (seq === graphLoadSeq && (e as DOMException).name !== "AbortError") {
        error = (e as Error).message;
      }
    } finally {
      if (seq === graphLoadSeq) {
        loading = false;
        graphLoadAbort = null;
      }
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
    graphLoadAbort?.abort();
  });

  /// Selection callback handed to GraphCanvas. Tapping a node
  /// flips the inspector open; background tap clears.
  function setSelected(id: string | null): void {
    selectedId = id;
    if (id !== null) graphState.inspectorOpen = true;
    // `fullstack-81`: surface the selection to the tab so the
    // tab strip can derive the title from the selected node's
    // label. We cache the label too so the title renders before
    // the graph data finishes reloading (e.g. after a hard
    // reload that round-trips the selection via URL hash).
    if (tab) {
      tab.selectedNodeId = id;
      tab.selectedNodeLabel = id === null ? null : graphSelectionLabel(id);
    }
  }

  function graphSelectionLabel(id: string): string | null {
    // FsGraphNode carries `name` directly — drive root has
    // name="" (empty path), so fall through to the semantic node
    // lookup before declaring no label.
    const fs = fsNodeById.get(id);
    if (fs && fs.name) return fs.name;
    const node = nodeById.get(id);
    if (node) return node.label ?? null;
    return null;
  }
</script>

<svelte:window onkeydown={onTabMenuKeydown} onpointerdown={onTabMenuPointerDown} />

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
  <div
    class="graph-tab"
    data-theme={tab ? surfaceThemeOverride("graph") : undefined}
    oncontextmenu={onGraphContextMenu}
    role="presentation"
  >
  {#if !tab}
    <!-- Overlay variant keeps the bar — there's no tab-strip
         right-click affordance available in the overlay shell.
         `fullstack-68` removes it from the tab variant; chips +
         hamburger items relocate to the tab right-click bubble
         below. -->
    <div class="bar">
      {@render filterChips()}
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
  {/if}

  {#if tab && tabMenuOpen}
    <!-- `fullstack-68`: Graph-tab right-click bubble. Anchored to
         the tab-strip click position via clampMenu.
         `fullstack-75`: row shape aligned with the standard
         hamburger-menu pattern from other tabs (TerminalTab /
         FileEditorTab / FileBrowserSurface) — `<button class="mbtn">`
         rows with optional icon + label + chord on the right; filters
         render vertically, one row per kind, with the kind colour as
         a dot + on/off cue via the `.on` class. -->
    {@const depthDisabled =
      !languageMode &&
      (!currentScope ||
        currentScope.kind === "drive" ||
        currentScope.kind === "global")}
    <div
      class="tab-menu-bubble"
      role="menu"
      tabindex="-1"
      aria-label="graph tab menu"
      use:portal
      use:clampMenu={tabMenuPos}
      onmousedown={(e) => e.stopPropagation()}
    >
      <!-- `fullstack-a-67` Graph slice: header row showing the
           current scope path + a kind-appropriate icon. Mirrors
           the path-row pattern @@Alex's addendum specs for the
           Terminal / File Browser / Editor / Graph right-click
           menus. Click on the row routes through the existing
           inspector-open path so the user can hop from the menu
           to the scope's inspector view. -->
      {#if currentScope}
        {@const scopePath =
          currentScope.kind === "drive" ? ""
          : currentScope.kind === "global" ? ""
          : currentScope.kind === "file" ? currentScope.path
          : currentScope.kind === "dir" ? currentScope.path
          : currentScope.kind === "tag" ? `#${currentScope.label}`
          : currentScope.kind === "git_repo" ? currentScope.label
          : currentScope.kind === "group" ? currentScope.label
          : ""}
        {@const scopeKindLabel =
          currentScope.kind === "drive" ? "Drive"
          : currentScope.kind === "global" ? "Global"
          : currentScope.kind === "tag" ? "Hashtag"
          : currentScope.kind === "git_repo" ? "Git repo"
          : currentScope.kind === "group" ? "Group"
          : currentScope.kind === "file" ? "File"
          : currentScope.kind === "dir" ? "Directory"
          : "Scope"}
        <button
          type="button"
          class="mbtn graph-scope-row"
          role="menuitem"
          tabindex="-1"
          title={scopePath || scopeKindLabel}
          onclick={openScopeHeaderInspector}
        >
          <span class="mbtn-icon" aria-hidden="true">
            {#if currentScope.kind === "drive" || currentScope.kind === "global"}
              <HardDrive size={16} strokeWidth={1.75} />
            {:else if currentScope.kind === "dir"}
              <Folder size={16} strokeWidth={1.75} />
            {:else if currentScope.kind === "tag"}
              <Hash size={16} strokeWidth={1.75} />
            {:else if currentScope.kind === "git_repo"}
              <Folder size={16} strokeWidth={1.75} />
            {:else if currentScope.kind === "group"}
              <Folder size={16} strokeWidth={1.75} />
            {:else}
              <FileText size={16} strokeWidth={1.75} />
            {/if}
          </span>
          <span class="mbtn-label graph-scope-path">
            {scopePath || scopeKindLabel}
          </span>
        </button>
        <div class="msep" role="separator"></div>
      {/if}
      <div
        class="mbtn depth-row"
        class:disabled={depthDisabled}
        class:shallow={depthShallow}
        title={depthShallow
          ? "Scope is shallow — depth 1 already reveals everything forward-reachable"
          : null}
      >
        <span class="mbtn-icon" aria-hidden="true"></span>
        <span class="mbtn-label">Depth</span>
        <input
          type="range"
          min={languageMode ? "0" : "1"}
          max={depthCap}
          step="1"
          bind:value={graphState.depth}
          disabled={depthDisabled || depthShallow}
          onmousedown={(e) => e.stopPropagation()}
          aria-label="depth"
        />
        <span class="depth-value">
          {#if languageMode && graphState.depth === 0}
            max
          {:else if depthShallow}
            {graphState.depth} <span class="depth-cue">[max]</span>
          {:else}
            {graphState.depth}
          {/if}
        </span>
      </div>
      <div class="msep" role="separator"></div>
      <button class="mbtn" onclick={reloadGraph}>
        <span class="mbtn-icon" aria-hidden="true">↻</span>
        <span class="mbtn-label">Reload</span>
        <span class="mbtn-chord"></span>
      </button>
      <div class="msep" role="separator"></div>
      {#each ["tag", "mention", "language", "img", "folder", "markdown", "source"] as const as kind (kind)}
        {@const driveLike =
          currentScope?.kind === "drive" || currentScope?.kind === "global"}
        {#if (!filesystemMode || (kind !== "img" && kind !== "language")) && (languageMode ? kind === "language" : kind !== "language" || driveLike) && (kind !== "folder" || filesystemMode || driveLike)}
          <button
            type="button"
            class="mbtn filter-row"
            class:on={show[kind]}
            onclick={() => (show[kind] = !show[kind])}
            role="menuitemcheckbox"
            aria-checked={show[kind]}
          >
            <span
              class="filter-dot"
              class:filter-dot-off={!show[kind]}
              style="background:{show[kind] ? FILTER_COLORS[kind] : 'transparent'};border-color:{FILTER_COLORS[kind]}"
              aria-hidden="true"
            ></span>
            <span class="mbtn-label">
              {#if filesystemMode}
                {kind === "tag"
                  ? "symlink"
                  : kind === "mention"
                    ? "hardlink"
                    : "directory"}
              {:else}
                {kind === "mention" ? "contact" : kind === "img" ? "media" : kind}
              {/if}
            </span>
            <span class="filter-count">{counts[kind]}</span>
          </button>
        {/if}
      {/each}
      <div class="msep" role="separator"></div>
      <button class="mbtn" onclick={flipToSettings}>
        <span class="mbtn-icon">
          <Settings2 size={16} strokeWidth={1.75} aria-hidden="true" />
        </span>
        <span class="mbtn-label">Settings</span>
        <span class="mbtn-chord"></span>
      </button>
      <div class="msep" role="separator"></div>
      <button
        class="mbtn"
        disabled={!canReopenClosedTab()}
        onclick={doReopenClosedTab}
      >
        <span class="mbtn-icon">
          <History size={16} strokeWidth={1.75} aria-hidden="true" />
        </span>
        <span class="mbtn-label">Reopen Closed Tab</span>
        <span class="mbtn-chord">{chordFor("app.tab.reopenClosed") ?? ""}</span>
      </button>
      <button class="mbtn" onclick={closeFromMenu}>
        <span class="mbtn-icon">
          <X size={16} strokeWidth={1.75} aria-hidden="true" />
        </span>
        <span class="mbtn-label">Close</span>
        <span class="mbtn-chord">{chordFor("app.tab.close") ?? ""}</span>
      </button>
    </div>
  {/if}

  <div class="body">
  <div class="canvas">
    {#if loading && nodes.length === 0}
      <div class="placeholder">loading graph…</div>
    {:else if error}
      <div class="placeholder error">{error}</div>
    {:else if !loading && nodes.length === 0}
      <div class="placeholder">
        {filesystemMode ? "no filesystem graph nodes for this scope" : languageMode ? "no language graph nodes for this drive yet" : "no markdown files in this drive yet"}
      </div>
    {/if}
    {#if loading && nodes.length > 0}
      <div class="stream-status">loading graph… {nodes.length} nodes, {edges.length} edges</div>
    {/if}
    <div class="cy" class:dim={!!error}>
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
      bind:width={
        () => graphState.inspectorWidth ?? paneWidths.graph,
        (v) => (graphState.inspectorWidth = v)
      }
      onResize={persistPaneWidths}
      onClose={() => (graphState.inspectorOpen = false)}
    >
      {#if scopeAncestors.length > 0}
        <!-- `fullstack-a-33`: ancestor breadcrumb. Replaces the
             explicit "Graph from here" button that used to live on
             every inspector body. Default render mode is "from
             here", so navigating back up the path is the load-
             bearing affordance; the breadcrumb provides it for
             every path-based scope (drive / dir: / file:). Click
             a prior segment to re-scope in place. -->
        <nav class="scope-crumbs" aria-label="graph scope ancestors">
          {#each scopeAncestors as crumb, i (i + ":" + crumb.scopeId)}
            {#if i > 0}
              <span class="crumb-sep" aria-hidden="true">/</span>
            {/if}
            {#if crumb.current}
              <span class="crumb current" aria-current="true">{crumb.label}</span>
            {:else}
              <button
                type="button"
                class="crumb"
                onclick={() => rescopeFromHere(crumb.scopeId)}
              >{crumb.label}</button>
            {/if}
          {/each}
        </nav>
      {/if}
      {#if (selectedFsNode && isFsDirectory(selectedFsNode) && selectedFsNode.id === "") || (selectedNode?.kind === "folder" && selectedNode.id === "")}
        <!-- Drive root: same body the file browser hamburger
             menu's Directory row pops (DriveInfoBody) so the
             whole-drive config lives in one place across surfaces.
             Differentiated visually by GraphCanvas painting the
             "drive" sub-kind in a darker fill with the HardDrive
             glyph.
             `fullstack-a-33`: stop passing `onSetAsScope` from
             the graph. The breadcrumb above is the in-graph path
             to drive-root scope; the button on DriveInfoBody is
             still used by FileBrowserSurface (which spawns a new
             graph instead of re-scoping). -->
        <DriveInfoBody />
      {:else if selectedFsNode && (isFsDirectory(selectedFsNode) || selectedFsNode.kind === "file") && selectedFsNode.path !== undefined && !selectedFsNode.broken}
        <!-- Real fs-mode file or directory: render the same body as the
             file browser / editor inspector (counts, size, code
             report; tags / refs / backlinks for files) by routing
             through InspectorBody. FileInfoBody dispatches on
             entry.is_dir so the "file" selection variant covers both
             shapes. File keeps the "Open" extra editor action.
             `fullstack-a-33`: dropped `onSetAsScope` — the
             breadcrumb above handles upward navigation; new
             from-here graphs come from chord spawn (Cmd+Shift+M,
             wired in `fullstack-a-32`). -->
        {@const fsPath = selectedFsNode.path}
        {@const fsKind = selectedFsNode.kind}
        <InspectorBody
          selection={{ kind: "file", path: fsPath }}
          showRefs
          onOpen={fsKind === "file"
            ? () => { void openInActivePane(fsPath); close(); }
            : undefined}
          onReveal={revealSelectedFsEntry}
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
              Open
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
        <!-- `fullstack-a-33`: dropped `onSetAsScope` for the
             tag / mention / file paths — breadcrumb + chord
             spawn cover those.
             `fullstack-a-50` G3: directory nodes get
             `onSetAsScope` back so the "Graph from here"
             button in DirectoryInfoBody re-roots the current
             graph at that directory via the existing
             `rescopeFromHere` helper. Mirror's the breadcrumb
             button's semantic. -->
        <InspectorBody
          selection={inspectorSelection}
          onOpen={
            inspectorSelection?.kind === "file"
              ? openSelectedFile
              : inspectorSelection?.kind === "mention" && selectedContactPath
                ? () => {
                    // Mention/contact "Open": route the
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
            inspectorSelection?.kind === "directory"
              ? () => rescopeFromHere(`dir:${inspectorSelection.path}`)
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

{#snippet filterChips()}
  <div class="filters">
    {#each ["tag", "mention", "language", "img", "folder", "markdown", "source"] as const as kind (kind)}
      {@const driveLike =
        currentScope?.kind === "drive" || currentScope?.kind === "global"}
      {#if (!filesystemMode || (kind !== "img" && kind !== "language")) && (languageMode ? kind === "language" : kind !== "language" || driveLike) && (kind !== "folder" || filesystemMode || driveLike)}
        <label class="chip" class:on={show[kind]}>
          <input type="checkbox" bind:checked={show[kind]} />
          <span class="dot" style="background:{FILTER_COLORS[kind]}"></span>
          {#if filesystemMode}
            {kind === "tag"
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
{/snippet}

{#snippet menuItems()}
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
    <button role="menuitem" onclick={flipToSettings} disabled={!onFlip}>
      <Settings2 size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">Settings</span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  <li class="sep" role="separator"></li>
  <li>
    <button
      role="menuitem"
      disabled={!canReopenClosedTab()}
      onclick={doReopenClosedTab}
    >
      <History size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">Reopen Closed Tab</span>
      <span class="menu-row-chord">{chordFor("app.tab.reopenClosed") ?? ""}</span>
    </button>
  </li>
  <li>
    <button role="menuitem" onclick={closeFromMenu}>
      <X size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">Close</span>
      <span class="menu-row-chord">{chordFor("app.tab.close") ?? ""}</span>
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
  /* `fullstack-64`: `.scope-label`, `.scope-select`, and the
     `.chrome-btn` rules dropped alongside the scope-selector +
     maximize-button DOM elements. The bar's hamburger menu lives
     inside `<HamburgerMenu>` so it brings its own chrome. */
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
  /* `fullstack-a-33`: ancestor breadcrumb band. Sits at the top
     of the inspector body, always visible for path-based scopes.
     Wraps on narrow inspector widths; clickable hops dim until
     hover. The current segment renders as plain text (no button)
     since clicking it would be a no-op. */
  .scope-crumbs {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    padding: 0.5rem 0.7rem 0.5rem 0.7rem;
    border-bottom: 1px solid var(--border);
    font-size: 12.5px;
    font-family: ui-monospace, monospace;
    color: var(--text-secondary);
    background: var(--bg-card);
  }
  .scope-crumbs .crumb {
    background: transparent;
    border: 0;
    padding: 1px 4px;
    margin: 0;
    color: var(--link);
    cursor: pointer;
    border-radius: 4px;
    font: inherit;
    line-height: 1.3;
    text-decoration: none;
    word-break: break-all;
  }
  .scope-crumbs .crumb:hover {
    background: var(--btn-hover);
    color: var(--text);
  }
  .scope-crumbs .crumb.current {
    color: var(--text);
    cursor: default;
    font-weight: 600;
  }
  .scope-crumbs .crumb-sep {
    color: var(--text-secondary);
    opacity: 0.6;
    user-select: none;
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
  .stream-status {
    position: absolute;
    left: 12px;
    top: 12px;
    z-index: 2;
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-card);
    color: var(--text-secondary);
    font-size: 12px;
    pointer-events: none;
  }

  /* `fullstack-68`: tab right-click bubble.
     `fullstack-75`: rows align with the standard hamburger-menu
     shape (`.mbtn` + `.msep`) used by TerminalTab / FileEditorTab
     / FileBrowserSurface. Filter rows pick up the same row chrome
     with a kind-coloured dot on the left, label in the middle,
     count on the right. */
  .tab-menu-bubble {
    position: fixed;
    z-index: 25500;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.18);
    padding: 4px;
    min-width: 240px;
    max-width: calc(100vw - 16px);
    max-height: calc(100vh - 24px);
    overflow-y: auto;
    color: var(--text);
    font-size: 13px;
    display: flex;
    flex-direction: column;
    /* `fullstack-a-8`: easeOutBack bubble-pop matching every
       other tab-menu bubble (TerminalTab / FileEditorTab) and
       the rest of the chrome. The phase-7 right-click rework
       dropped the wobble here; @@Alex never asked for that. */
    transform-origin: top left;
    animation: graph-tab-menu-pop 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
    transition: transform 200ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .tab-menu-bubble:hover {
    transform: scale(1.015);
  }
  @keyframes graph-tab-menu-pop {
    0%   { opacity: 0; transform: scale(0.92); }
    100% { opacity: 1; transform: scale(1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .tab-menu-bubble {
      animation: none;
      transition: none;
    }
    .tab-menu-bubble:hover {
      transform: none;
    }
  }
  .tab-menu-bubble .mbtn {
    display: flex;
    align-items: center;
    gap: 8px;
    background: none;
    border: 0;
    border-radius: 4px;
    cursor: pointer;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    padding: 6px 8px;
    text-align: left;
    transform-origin: left center;
    transition:
      background 80ms ease,
      color 80ms ease,
      transform 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .tab-menu-bubble .mbtn:hover,
  .tab-menu-bubble .mbtn.on {
    background: var(--hover-bg);
  }
  .tab-menu-bubble .mbtn:hover:not(.disabled) {
    transform: scale(1.02);
  }
  @media (prefers-reduced-motion: reduce) {
    .tab-menu-bubble .mbtn {
      transition: background 80ms ease, color 80ms ease;
    }
    .tab-menu-bubble .mbtn:hover {
      transform: none;
    }
  }
  .tab-menu-bubble .mbtn.disabled {
    color: var(--text-secondary);
    cursor: not-allowed;
    opacity: 0.58;
  }
  .tab-menu-bubble .mbtn-icon {
    width: 18px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .tab-menu-bubble .mbtn-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tab-menu-bubble .mbtn-chord {
    margin-left: 1.5rem;
    color: var(--text-secondary);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 11.5px;
  }
  .tab-menu-bubble .msep {
    height: 1px;
    background: var(--separator, var(--border));
    margin: 4px 2px;
  }
  /* Depth row hosts a slider in the value slot; keep the row
     height in line with the action rows by leaning on `.mbtn`. */
  .tab-menu-bubble .depth-row input[type="range"] {
    flex-shrink: 0;
    width: 90px;
  }
  .tab-menu-bubble .depth-value {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
    width: 1.6em;
    text-align: right;
  }
  /* `fullstack-a-67`: Graph hamburger header row — kind icon +
     path label. Path fades at the right edge for long file
     paths so the menu width stays bounded; matches the
     Pane.svelte tab-name + FileTree.svelte (`-a-62`) fade
     pattern verbatim. The row is non-interactive in this
     slice (display-only); click-to-inspect wiring can land in
     a follow-up. */
  .tab-menu-bubble .graph-scope-row {
    /* `fullstack-a-67` slice 1b: click-to-inspector. Cursor:
       pointer matches the rest of the menu's clickable rows so
       the affordance reads at a glance. */
    cursor: pointer;
  }
  .tab-menu-bubble .graph-scope-row:hover .graph-scope-path {
    color: var(--text);
  }
  .tab-menu-bubble .graph-scope-row .graph-scope-path {
    flex: 1;
    min-width: 0;
    display: block;
    white-space: nowrap;
    overflow: hidden;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
    mask-image: linear-gradient(to right, black calc(100% - 1.25rem), transparent);
    -webkit-mask-image: linear-gradient(to right, black calc(100% - 1.25rem), transparent);
  }
  /* `fullstack-a-56` shallow-scope cue: when the scope's
     depth-cap is 1 (single-file graph, etc.) show a `[max]`
     suffix on the depth value so the user can see at a glance
     that the slider can't be dragged further. The `.shallow`
     class on `.depth-row` widens the value column to fit the
     suffix; `.depth-cue` is the smaller dimmer trailing chip. */
  .tab-menu-bubble .depth-row.shallow .depth-value {
    width: auto;
  }
  .tab-menu-bubble .depth-cue {
    color: var(--text-secondary);
    font-size: 0.85em;
    margin-left: 0.25rem;
    opacity: 0.7;
  }
  /* Filter rows: kind-coloured dot left, label middle, count
     right. On-state fills the dot; off-state shows a hollow ring
     so the on/off cue reads at a glance without relying on the
     row background hover. The `.mbtn.on` default background would
     fill the row whenever a filter is toggled on; override it
     here so multiple-on filters don't paint the whole bubble. */
  .tab-menu-bubble .filter-row.on {
    background: transparent;
  }
  .tab-menu-bubble .filter-row.on:hover {
    background: var(--hover-bg);
  }
  .tab-menu-bubble .filter-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 2px solid transparent;
    flex-shrink: 0;
  }
  .tab-menu-bubble .filter-count {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
</style>
