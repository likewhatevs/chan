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

  import { onDestroy, onMount } from "svelte";
  import {
    ArrowLeft,
    ArrowRight,
    Clock,
    Maximize2,
    Minimize2,
  } from "lucide-svelte";
  import {
    overlayMaximized,
    setOverlayMaximized,
  } from "../state/pageWidth.svelte";
  import cytoscape from "cytoscape";
  import type { Core, ElementDefinition, EventObject, Layouts } from "cytoscape";
  // @ts-expect-error fcose ships no .d.ts; the layout name is enough
  import fcose from "cytoscape-fcose";
  // @ts-expect-error cytoscape-d3-force ships no .d.ts
  import d3Force from "cytoscape-d3-force";

  import { api } from "../api/client";
  import type { GraphView, GraphViewEdge, GraphViewNode } from "../api/types";
  import { openInActivePane } from "../state/tabs.svelte";
  import { isImage } from "../state/fileTypes";
  import {
    availableGraphScopes,
    browserOverlay,
    graphOverlay,
    openBrowser,
    openScopeHistory,
    paneWidths,
    persistPaneWidths,
    revealAndSelect,
    tree,
  } from "../state/store.svelte";
  import { type ScopeOption, defaultScopeId } from "../state/scope.svelte";
  import ResizeHandle from "./ResizeHandle.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";
  import Inspector from "./Inspector.svelte";
  import OverlayShell from "./OverlayShell.svelte";
  import InspectorBody, { type InspectorSelection } from "./InspectorBody.svelte";

  // cytoscape.use is idempotent across module reloads.
  cytoscape.use(fcose);
  cytoscape.use(d3Force);

  // Visibility of the details aside lives on `graphOverlay.inspectorOpen`
  // (module state) so it round-trips through the URL hash.

  const visible = $derived(graphOverlay.open);

  /// Dropdown options derived from the live layout. Same shape as
  /// the assistant overlay; relabels "drive" as "Whole drive".
  const scopeOptions = $derived<ScopeOption[]>(availableGraphScopes());

  const currentScope = $derived<ScopeOption | null>(
    scopeOptions.find((o) => o.id === graphOverlay.scopeId) ?? null,
  );

  /// Snap to a sensible scope on open if the saved scopeId no longer
  /// resolves (file closed since last open, group set changed). Skip
  /// while the overlay is closed for the same reason as the assistant.
  $effect(() => {
    if (!visible) return;
    if (!currentScope) graphOverlay.scopeId = defaultScopeId();
  });

  function close(): void {
    graphOverlay.open = false;
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
  type RenderedEdgeKind = "link" | "tag" | "mention" | "group";
  /// Stable id for the synthetic scope hub node. Prefixed with `__`
  /// so it can't collide with a real file path.
  const SCOPE_HUB_ID = "__scope_hub__";
  type RenderedEdge = GraphViewEdge & { kind: RenderedEdgeKind };
  type RenderedNode = Extract<GraphViewNode, { kind: "file" | "tag" | "mention" }>;
  /// Chip toggles. `link`, `tag`, `mention` are edge-kind filters
  /// (the visual element they govern is the edge plus any node only
  /// reachable through edges of that kind). `img` is a node filter:
  /// flipping it off hides every file node whose path classifies as
  /// an image, along with any edge touching one.
  type FilterKind = "link" | "tag" | "mention" | "img";

  // ---- state -------------------------------------------------------------

  let containerEl: HTMLDivElement | undefined = $state();
  let cy: Core | null = null;
  let resizeObs: ResizeObserver | null = null;
  let forceLayout: Layouts | null = null;

  /// Half-width of the box that the d3-force `randomize` scatters
  /// visible nodes into at the start of every rebuild. 150 = a
  /// 300x300 model-space area centered on the focal at (0, 0).
  /// Kept small so the spring forces have room to wobble within
  /// the brief shake window without flinging satellites off-frame.
  const INITIAL_BBOX_HALF = 150;

  /// Camera zoom for the opening frame: the cluster appears as a
  /// tiny dot at canvas center, then animates out to the fitted
  /// size. Sits just above `minZoom` (0.15) so the boot frame is
  /// small but still legible enough to hint at structure during
  /// the grow-in.
  const BOOT_ZOOM = 0.25;

  /// Gates the currentScope $effect: scopeOptions re-derives any
  /// time the file tree updates, producing a fresh array (and thus
  /// a fresh currentScope object reference) even when the user
  /// hasn't actually changed scope. Without this gate every tree
  /// tick would kick off a relayout. Tracked by id, not by object
  /// identity, so the effect only acts on real transitions.
  let lastScopeId: string | null = null;

  let nodes: RenderedNode[] = $state([]);
  let edges: RenderedEdge[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);

  /// Chip toggles live on `graphOverlay.filters` (module state) so
  /// they round-trip through the URL hash. Local proxy aliases keep
  /// the existing read sites compact.
  const show = graphOverlay.filters;

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

  function toggleInspector(): void {
    graphOverlay.inspectorOpen = !graphOverlay.inspectorOpen;
    menu?.close();
  }

  function doToggleOverlayMaximized(): void {
    setOverlayMaximized(!overlayMaximized.on);
    menu?.close();
  }

  async function reloadGraph(): Promise<void> {
    menu?.close();
    await load();
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
  //       graphOverlay.depth hops. Drive = no filter.
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
      for (let i = 0; i < graphOverlay.depth; i++) {
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
    for (let i = 0; i < graphOverlay.depth; i++) {
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

  const visibleEdges = $derived(
    edges.filter(
      (e) =>
        show[e.kind] &&
        !hiddenImageIds.has(e.source) &&
        !hiddenImageIds.has(e.target) &&
        !hiddenContactIds.has(e.source) &&
        !hiddenContactIds.has(e.target) &&
        (scopedNodeIds === null ||
          (scopedNodeIds.has(e.source) && scopedNodeIds.has(e.target))),
    ),
  );
  const visibleNodeIds = $derived.by(() => {
    const ids = new Set<string>();
    for (const n of nodes) {
      if (scopedNodeIds !== null && !scopedNodeIds.has(n.id)) continue;
      if (n.kind === "file" && !hiddenImageIds.has(n.id) && !hiddenContactIds.has(n.id)) {
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
    const c: Record<FilterKind, number> = { link: 0, tag: 0, mention: 0, img: 0 };
    for (const e of edges) c[e.kind]++;
    for (const n of nodes) {
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
    selectedId ? (nodeById.get(selectedId) ?? null) : null,
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
      revealAndSelect(selectedNode.path);
      openBrowser();
      browserOverlay.inspectorOpen = true;
      close();
    }
  }

  function selectFromList(n: RenderedNode): void {
    selectedId = n.id;
    if (cy) {
      cy.$(":selected").unselect();
      const ele = cy.getElementById(n.id);
      if (ele.nonempty()) ele.select();
    }
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
        : {
            kind: selectedNode.kind,
            nodeId: selectedNode.id,
            label: selectedNode.label,
          },
  );

  // ---- presentation ------------------------------------------------------

  /// Cytoscape resolves --g-* via getComputedStyle at buildCytoscape
  /// time, so theme changes propagate next reload.
  const EDGE_COLORS: Record<RenderedEdgeKind, string> = {
    link: "var(--text-secondary)",
    tag: "var(--g-tag)",
    mention: "var(--warn-text)",
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
    img: "var(--g-img)",
  };

  // ---- cytoscape glue ----------------------------------------------------

  /// Stable edge id, used to address elements after add/remove. The
  /// graph is small enough that collisions are rare; the counter
  /// suffix makes it safe regardless.
  function makeEdgeIds(g: GraphView): Map<GraphViewEdge, string> {
    const map = new Map<GraphViewEdge, string>();
    const seen = new Map<string, number>();
    for (const e of g.edges) {
      const base = `${e.source}|${e.target}|${e.kind}`;
      const n = (seen.get(base) ?? 0) + 1;
      seen.set(base, n);
      map.set(e, n === 1 ? base : `${base}#${n}`);
    }
    return map;
  }

  function readThemeColors(host: HTMLElement) {
    const cs = getComputedStyle(host);
    const v = (n: string, fb: string) => cs.getPropertyValue(n).trim() || fb;
    return {
      // Node kinds in the graph: documents (doc), files classified as
      // images (img), contact-kind files + @@mentions, and hashtag
      // (tag) nodes. Every kind renders as a coloured circle with an
      // icon glyph inside; the colour identifies the kind, the icon
      // reinforces it, and circle size scales with the node's backlink
      // count. Dates are filtered upstream.
      doc: v("--g-doc", "#ff8a3d"),
      img: v("--g-img", "#b07dff"),
      tag: v("--g-tag", "#6cd07a"),
      mention: v("--warn-text", "#e3b341"),
      accent: v("--accent", "#ff7a3b"),
      text: v("--text", "#e8e8e8"),
      textSec: v("--text-secondary", "#9a9a9a"),
      bg: v("--bg", "#1e1e1e"),
      bgCard: v("--bg-card", "#252525"),
    };
  }

  /// Wrap an inner SVG fragment in a 24x24 viewBox and emit a
  /// data: URL suitable for cytoscape's `background-image`. The
  /// stroke colour is baked in at build time because cytoscape
  /// doesn't currentColor-resolve SVG backgrounds.
  function svgStrokeIcon(inner: string, stroke: string): string {
    // Explicit width/height on the SVG root so cytoscape's canvas
    // image cache treats the icon as a fixed-size raster; without
    // them browsers fall back to 300x150 intrinsic size which makes
    // `background-fit` math unpredictable.
    const svg =
      `<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' ` +
      `viewBox='0 0 24 24' fill='none' stroke='${stroke}' stroke-width='2.4' ` +
      `stroke-linecap='round' stroke-linejoin='round'>${inner}</svg>`;
    return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
  }
  function svgTextIcon(glyph: string, fill: string): string {
    const svg =
      `<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' ` +
      `viewBox='0 0 24 24'>` +
      `<text x='12' y='18' text-anchor='middle' ` +
      `font-family='-apple-system, system-ui, sans-serif' ` +
      `font-size='20' font-weight='800' fill='${fill}'>${glyph}</text>` +
      `</svg>`;
    return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
  }

  /// Base circle diameter; the largest (most-backlinked) node tops
  /// out at NODE_BASE_PX * NODE_SIZE_RATIO. 20 % spread is enough
  /// for a heavily-linked hub to stand out from leaf nodes without
  /// dominating the layout.
  const NODE_BASE_PX = 22;
  const NODE_SIZE_RATIO = 1.2;
  /// Icon glyph occupies this fraction of the node diameter. Leaves
  /// a coloured ring around the icon so the kind colour still reads
  /// at small zoom levels.
  const ICON_FRACTION_PCT = "55%";

  function buildStylesheet(
    host: HTMLElement,
    maxBacklinks: number,
  ): cytoscape.StylesheetJson {
    const c = readThemeColors(host);
    // mapData clamps to outMin when fieldMax == fieldMin, so the
    // floor on the denominator just keeps the formula well-defined
    // when no backlinks exist in the current view; the smallest
    // circle still ends up at NODE_BASE_PX.
    const maxBL = Math.max(1, maxBacklinks);
    const sizeMap = `mapData(backlinks, 0, ${maxBL}, ${NODE_BASE_PX}, ${
      NODE_BASE_PX * NODE_SIZE_RATIO
    })`;
    // Icons are stroked / filled in the page background colour so
    // they "knock out" the coloured node beneath them. Paths come
    // from a lucide-style 24x24 grid so they read uniformly.
    const PATH_DOC =
      `<path d='M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z'/>` +
      `<polyline points='14 3 14 8 19 8'/>`;
    const PATH_CONTACT =
      `<circle cx='12' cy='9' r='3.4'/>` +
      `<path d='M5 20c.6-3.6 3.6-6 7-6s6.4 2.4 7 6'/>`;
    const PATH_IMG =
      `<rect x='3.5' y='4' width='17' height='16' rx='2'/>` +
      `<circle cx='9' cy='10' r='1.6'/>` +
      `<polyline points='20 16 15 11 5 19'/>`;
    const ICON_DOC = svgStrokeIcon(PATH_DOC, c.bg);
    const ICON_CONTACT = svgStrokeIcon(PATH_CONTACT, c.bg);
    const ICON_IMG = svgStrokeIcon(PATH_IMG, c.bg);
    const ICON_TAG = svgTextIcon("#", c.bg);
    const ICON_MENTION = svgTextIcon("@", c.bg);
    return [
      {
        selector: "node",
        style: {
          shape: "ellipse",
          width: sizeMap as unknown as number,
          height: sizeMap as unknown as number,
          label: "data(label)",
          color: c.text,
          // Match the rest of the chrome (App.svelte body
          // font-family) so graph labels read as part of the same
          // UI rather than a separate widget.
          "font-family":
            '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
          "font-size": 7,
          // Labels above the node, ellipsised at a fixed width.
          // Right-of-node placement (the prior layout) bled long
          // file names into neighbouring nodes once fcose packed
          // anything close together.
          "text-halign": "center",
          "text-valign": "top",
          "text-margin-y": -3,
          "text-wrap": "ellipsis",
          "text-max-width": "110px",
          "text-outline-color": c.bg,
          // Outline at 1px reads as a subtle halo for legibility
          // over edges/nodes; 2px made labels look bolder than the
          // surrounding UI text.
          "text-outline-width": 1,
          "border-width": 1.5,
          "border-color": c.bg,
          "min-zoomed-font-size": 6,
          // Icon glyph centered inside the circle. `background-fit:
          // none` is the only mode where width/height percentages
          // are honoured literally; `contain` ignores them and sized
          // off the SVG's intrinsic 24px box, which overshot the
          // node bounds and gave us file-shaped blobs instead of
          // circles. `inside` containment clips anything that would
          // still overhang.
          "background-fit": "none",
          "background-image-containment": "inside",
          "background-clip": "node",
          "background-width": ICON_FRACTION_PCT,
          "background-height": ICON_FRACTION_PCT,
          "background-position-x": "50%",
          "background-position-y": "50%",
        },
      },
      {
        selector: 'node[kind = "doc"]',
        style: {
          "background-color": c.doc,
          "background-image": ICON_DOC,
        },
      },
      {
        selector: 'node[kind = "contact"]',
        style: {
          "background-color": c.mention,
          "background-image": ICON_CONTACT,
        },
      },
      {
        selector: 'node[kind = "img"]',
        style: {
          "background-color": c.img,
          "background-image": ICON_IMG,
        },
      },
      {
        selector: 'node[kind = "tag"]',
        style: {
          "background-color": c.tag,
          "background-image": ICON_TAG,
        },
      },
      {
        selector: 'node[kind = "mention"]',
        style: {
          "background-color": c.mention,
          "background-image": ICON_MENTION,
        },
      },
      {
        // Synthetic scope-hub node. No icon, no label, no border;
        // smaller than file circles so it reads as a marker rather
        // than a peer. Backlink-based `mapData` would clamp the size
        // to NODE_BASE_PX here (no incoming edges in `g.edges`), but
        // we set width/height explicitly to keep the hub visibly
        // distinct.
        selector: 'node[kind = "scope"]',
        style: {
          "background-color": c.textSec,
          "background-image": "none",
          "border-width": 0,
          width: 12,
          height: 12,
          label: "",
          opacity: 0.7,
        },
      },
      {
        selector: "node[?missing]",
        style: {
          "background-color": c.bgCard,
          "border-style": "dashed",
          "border-color": c.textSec,
          opacity: 0.6,
        },
      },
      {
        selector: "node.hover",
        style: {
          "border-color": c.text,
          "border-width": 1.5,
        },
      },
      {
        selector: "node.focal",
        style: {
          "border-color": c.accent,
          "border-width": 2,
        },
      },
      {
        selector: "node:selected",
        style: {
          "border-color": c.doc,
          "border-width": 3,
          "overlay-color": c.doc,
          "overlay-opacity": 0.15,
          "overlay-padding": 2,
        },
      },
      {
        selector: "edge",
        style: {
          "curve-style": "bezier",
          "line-cap": "round",
          opacity: 0.15,
        },
      },
      {
        selector: 'edge[kind = "link"]',
        style: { "line-color": c.text, width: 0.7 },
      },
      {
        selector: 'edge[kind = "tag"]',
        style: { "line-color": c.tag, width: 1.0 },
      },
      {
        selector: 'edge[kind = "mention"]',
        style: { "line-color": c.mention, width: 1.0 },
      },
      {
        // Group-scope synthetic edges. Dashed + slightly bolder than
        // a real link so the eye reads them as "this is a UI hint,
        // not a document edge"; the accent colour keeps them
        // distinguishable from `edge[?broken]` (broken doc links,
        // dashed greyed-out).
        selector: 'edge[kind = "group"]',
        style: {
          "line-color": c.accent,
          "line-style": "dashed",
          width: 1.4,
          opacity: 0.55,
        },
      },
      {
        selector: "edge[?broken]",
        style: { "line-style": "dashed", opacity: 0.11 },
      },
      {
        selector: ".hidden",
        style: { display: "none" },
      },
    ];
  }

  /// fcose options. Edge length differs by kind so the dense file
  /// cluster has room to breathe while satellites sit close to their
  /// owner; mirrors the d3-force `distance` configuration we used
  /// before.
  ///
  /// `focalIds` pins one or more file nodes at known model
  /// coordinates via fcose's `fixedNodeConstraint`. Pinning has to
  /// happen during the layout, not after: post-layout repositioning
  /// fights fcose's spring system and leaves neighbours stuck where
  /// the unconstrained run originally placed them, which is why the
  /// first version of this code drew empty canvases on first open.
  function fcoseOptions(opts: {
    randomize: boolean;
    animate: boolean;
    focalIds?: string[];
  }) {
    const constraints =
      opts.focalIds && opts.focalIds.length > 0
        ? opts.focalIds.length === 1
          ? [{ nodeId: opts.focalIds[0]!, position: { x: 0, y: 0 } }]
          : opts.focalIds.map((id, i, all) => {
              const angle = (i / all.length) * Math.PI * 2 - Math.PI / 2;
              const r = 120;
              return {
                nodeId: id,
                position: { x: Math.cos(angle) * r, y: Math.sin(angle) * r },
              };
            })
        : undefined;
    return {
      name: "fcose",
      quality: "default",
      randomize: opts.randomize,
      animate: opts.animate,
      animationDuration: opts.animate ? 350 : 0,
      fit: false, // we fit ourselves on layoutstop against the
      // actually-rendered container; fcose's built-in fit can
      // anchor on a too-small viewport when the panel is still
      // settling.
      padding: 30,
      // Wider separation now that labels render above nodes; 75
      // packed satellites tight enough that ellipsised labels still
      // overlapped on dense clusters.
      nodeSeparation: 140,
      idealEdgeLength: (e: cytoscape.EdgeSingular) =>
        e.data("kind") === "link" ? 70 : 40,
      edgeElasticity: 0.45,
      nestingFactor: 0.1,
      numIter: 2500,
      gravity: 0.25,
      packComponents: true,
      fixedNodeConstraint: constraints,
    } as cytoscape.LayoutOptions;
  }

  /// d3-force options. Runs AFTER fcose seeds positions, so
  /// `randomize: false` keeps the cluster layout fcose produced and
  /// just relaxes overlaps + adds the live "jostle" feel: drag a
  /// node, neighbours nudge out of the way, everything settles.
  /// d3-force tuning depends on graph size: the playground values
  /// (manyBody -90, weak centering 0.05) were dialed in against a
  /// ~45-node graph. Applied to a 7-node file-scope view, the
  /// repulsion overwhelms the spring constraints and pushes the
  /// satellites off the canvas. Scale repulsion down and centering
  /// up as the visible count drops.
  function d3ForceOptions(visibleCount: number) {
    // Smooth interpolation between sparse (≤8 nodes) and dense
    // (≥40 nodes) regimes.
    const t = Math.max(0, Math.min(1, (visibleCount - 8) / 32));
    const manyBody = -20 + (-90 - -20) * t;
    const xy = 0.25 - (0.25 - 0.05) * t;
    return {
      name: "d3-force",
      animate: true,
      fit: false,
      randomize: false,
      infinite: true,
      ungrabifyWhileSimulating: false,
      fixedAfterDragging: true,
      alpha: 1,
      alphaMin: 0.05,
      // ~90 ticks (~1.5s at 60fps) for the simulation to cool from
      // alpha=1 to alphaMin: a brief shake-then-settle rather than
      // a long simmer.
      alphaDecay: 1 - Math.pow(0.05, 1 / 90),
      alphaTarget: 0,
      // Velocity damping. Loose enough that the shake stays visible
      // across the 90-tick budget; tight enough that nodes stop on
      // a definite resting point.
      velocityDecay: 0.45,
      // Collide radius approximates the label's visual footprint,
      // not just the node circle: doc labels can be ~110px wide
      // (text-max-width) so we use ~55 as a half-width bubble that
      // keeps neighbouring file names from overlapping.
      collideRadius: (n: { kind?: string }) => {
        if (n.kind === "doc") return 55;
        if (n.kind === "tag") return 30;
        return 22;
      },
      collideStrength: 0.95,
      collideIterations: 2,
      manyBodyStrength: manyBody,
      linkDistance: (e: { kind?: string }) =>
        e.kind === "link" ? 120 : 70,
      linkStrength: 0.55,
      linkId: (n: { id: string }) => n.id,
      xStrength: xy,
      yStrength: xy,
    } as cytoscape.LayoutOptions;
  }

  /// Cap the camera zoom after a `cy.fit()` call so a small visible
  /// cluster (5-15 nodes packed tight) doesn't blow up labels at
  /// load. cytoscape's font-size is model-space pixels scaled by
  /// zoom; past ~2x, an 11px label becomes 22+px screen text. The
  /// global `maxZoom` is set much higher than this so the USER can
  /// still scroll-wheel in close — only the auto-fit is capped.
  /// Upper bound for the auto-fit zoom so small scopes don't blow
  /// up label sizes at load. The user-driven max (cy.maxZoom) is
  /// well above this; this cap only governs the boot animation.
  const AUTO_FIT_MAX_ZOOM = 2.2;
  /// Multiplier applied to the natural fit zoom so the cluster
  /// lands a touch tighter than a bare cy.fit() would. Capped by
  /// AUTO_FIT_MAX_ZOOM so tiny scopes don't run away.
  const AUTO_FIT_ZOOM_BOOST = 1.10;
  function clampAutoFitZoom(): void {
    if (!cy) return;
    if (cy.zoom() > AUTO_FIT_MAX_ZOOM) cy.zoom(AUTO_FIT_MAX_ZOOM);
  }

  /// Compute the clamped {zoom, pan} that would frame `eles` with
  /// `padding` px of breathing room. Returns explicit targets so
  /// the boot animation can feed them straight into a
  /// `cy.animate({ zoom, pan })` call — animating to explicit
  /// targets interpolates monotonically, with no overshoot then
  /// snap that cytoscape's built-in `fit:` target would produce.
  function computeFitTarget(
    eles: cytoscape.CollectionReturnValue,
    padding: number,
  ): { zoom: number; pan: { x: number; y: number } } {
    const bb = eles.boundingBox({});
    const cw = cy!.width();
    const ch = cy!.height();
    const zoomFit = Math.min(
      (cw - 2 * padding) / Math.max(bb.w, 1),
      (ch - 2 * padding) / Math.max(bb.h, 1),
    );
    const z = Math.min(zoomFit * AUTO_FIT_ZOOM_BOOST, AUTO_FIT_MAX_ZOOM);
    return { zoom: z, pan: panToCenter(bb, z) };
  }

  /// Pan vector that lands the bounding-box center at the viewport
  /// center for a given zoom. viewportPx = modelCoord * zoom + pan,
  /// so pan = viewportCenter - bbCenter * zoom.
  function panToCenter(
    bb: { x1: number; y1: number; x2: number; y2: number },
    zoom: number,
  ): { x: number; y: number } {
    if (!cy) return { x: 0, y: 0 };
    const cx = (bb.x1 + bb.x2) / 2;
    const cy_ = (bb.y1 + bb.y2) / 2;
    return { x: cy.width() / 2 - cx * zoom, y: cy.height() / 2 - cy_ * zoom };
  }

  /// File-node ids matching the current scope's seed path(s). Used
  /// to pin the focal-anchor inside the fcose run. Returns an empty
  /// array for drive / global scope (no anchor wanted).
  function computeFocalNodeIds(): string[] {
    if (!currentScope) return [];
    // Group scope: pin only the synthetic hub (added by
    // buildElements). fcose pulls every scope file toward it via the
    // group edges, which is what spreads the seed files around the
    // hub instead of stacking them all at the origin.
    if (currentScope.kind === "group") return [SCOPE_HUB_ID];
    // Tag scope: pin the tag node itself so fcose pulls the docs
    // referencing it into a halo around the tag (same shape file
    // scopes get around the file node).
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
  }

  /// Files split into "doc", "img", or "contact" at element-build
  /// time. Image classification is extension-based (chan-drive's
  /// indexer skips non-text files, so image nodes get synthesized
  /// from edge targets and don't carry node_kind). Contact
  /// classification comes from the wire `node_kind: "contact"` stamp,
  /// which the server populates from chan-drive's `Drive::contacts()`.
  /// Anything else is a doc.
  function classifyFile(
    path: string,
    nodeKind: "contact" | undefined,
  ): "doc" | "img" | "contact" {
    if (/\.(png|jpe?g|gif|webp|svg|avif|bmp)$/i.test(path)) return "img";
    if (nodeKind === "contact") return "contact";
    return "doc";
  }

  function buildElements(g: GraphView): {
    elements: ElementDefinition[];
    dropped: number;
    maxBacklinks: number;
  } {
    const nodeIds = new Set(g.nodes.map((n) => n.id));
    const edgeIds = makeEdgeIds(g);
    const els: ElementDefinition[] = [];
    // Pre-count incoming edges per node. The stylesheet's `mapData`
    // turns this into a circle diameter, so a heavily-referenced
    // note / tag / contact ends up visibly larger than a leaf node.
    // Only edges with both endpoints in the node set count, mirroring
    // the dangling-endpoint filter below.
    const backlinks = new Map<string, number>();
    for (const e of g.edges) {
      if (!nodeIds.has(e.source) || !nodeIds.has(e.target)) continue;
      backlinks.set(e.target, (backlinks.get(e.target) ?? 0) + 1);
    }
    let maxBacklinks = 0;
    for (const v of backlinks.values()) {
      if (v > maxBacklinks) maxBacklinks = v;
    }
    for (const n of g.nodes) {
      // Display labels: mentions arrive from chan-drive as `@@name`
      // (the source-text form). Strip the `@@` so the canvas reads
      // `alice` not `@@alice`; one palette and one display style for
      // contacts across all surfaces.
      const displayLabel =
        n.kind === "mention" ? n.label.replace(/^@@/, "") : n.label;
      const data: Record<string, unknown> = {
        id: n.id,
        kind:
          n.kind === "file" ? classifyFile(n.path, n.node_kind) : n.kind,
        label: displayLabel,
        backlinks: backlinks.get(n.id) ?? 0,
      };
      if (n.kind === "file") {
        data.path = n.path;
        if (n.missing) data.missing = true;
      }
      els.push({ group: "nodes", data });
    }
    let dropped = 0;
    for (const e of g.edges) {
      // forceLink-style preflight: drop edges whose endpoints aren't
      // in the node set so a single dangling row doesn't poison the
      // whole render.
      if (!nodeIds.has(e.source) || !nodeIds.has(e.target)) {
        dropped++;
        continue;
      }
      const data: Record<string, unknown> = {
        id: edgeIds.get(e)!,
        source: e.source,
        target: e.target,
        kind: e.kind,
      };
      if (e.broken) data.broken = true;
      els.push({ group: "edges", data });
    }
    // Synthetic "scope" node + star edges. When the current scope is
    // a multi-file group, drop a small grey hub node at the cluster
    // centre and link each scope file to it. The hub gives fcose a
    // single point to pull the seed files toward (instead of stacking
    // them all at the origin) and shows the user, at a glance, which
    // files the current scope binds together. Synthetic — not in
    // `g.edges`, doesn't count toward backlink sizing.
    if (currentScope && currentScope.kind === "group") {
      const paths = currentScope.paths.filter((p) => nodeIds.has(p));
      if (paths.length > 0) {
        els.push({
          group: "nodes",
          data: { id: SCOPE_HUB_ID, kind: "scope", label: "" },
        });
        for (const p of paths) {
          els.push({
            group: "edges",
            data: {
              id: `group:${SCOPE_HUB_ID}|${p}`,
              source: SCOPE_HUB_ID,
              target: p,
              kind: "group",
            },
          });
        }
      }
    }
    return { elements: els, dropped, maxBacklinks };
  }

  function buildCytoscape(g: GraphView): void {
    if (!containerEl) return;
    forceLayout?.stop();
    forceLayout = null;
    cy?.destroy();
    // Direct DOM mutation, not a Svelte class binding — needs to
    // be effective synchronously, before fcose's first render.
    // Cleared just before forceLayout.run() so the d3-force
    // animation IS the user-visible "drawing" of the graph.
    containerEl.style.opacity = "0";
    const { elements, dropped, maxBacklinks } = buildElements(g);
    if (dropped > 0) {
      console.warn(`graph: dropped ${dropped} edges with unknown endpoints`);
    }
    const focalIds = computeFocalNodeIds();
    // Build cytoscape WITHOUT a layout option: passing `layout` to
    // the constructor lets fcose start (and potentially fire
    // layoutstop) before any of our event handlers are registered,
    // which is exactly how the first-open empty-canvas bug was
    // sneaking past every fix attempt. Run the layout explicitly
    // below, after handlers are wired.
    cy = cytoscape({
      container: containerEl,
      elements,
      style: buildStylesheet(containerEl, maxBacklinks),
      minZoom: 0.15,
      // User-driven zoom (scroll wheel) wants headroom — capped too
      // low and the canvas hits an invisible wall while inspecting.
      // Auto-fit on load is clamped separately by clampAutoFitZoom()
      // so small scopes don't blow up labels at startup; this ceiling
      // only bounds the user's own scroll-wheel pinch.
      maxZoom: 6,
      boxSelectionEnabled: false,
      selectionType: "single",
    });

    cy.on("tap", "node", (ev: EventObject) => {
      const id = ev.target.id() as string;
      selectedId = id;
      graphOverlay.inspectorOpen = true;
    });
    cy.on("tap", (ev: EventObject) => {
      if (ev.target === cy) selectedId = null;
    });
    cy.on("mouseover", "node", (ev: EventObject) => {
      ev.target.addClass("hover");
    });
    cy.on("mouseout", "node", (ev: EventObject) => {
      ev.target.removeClass("hover");
    });

    // Run the layout BEFORE applying the visibility filter:
    // cytoscape layouts skip hidden nodes, so syncing visibility
    // first leaves the off-scope nodes stranded at (0,0). With
    // the focal also pinned at (0,0) the visible bounding box
    // collapses and the fit zooms to a single point — exactly the
    // "empty canvas" symptom we kept chasing. Position everything
    // first; hide afterwards.
    const layout = cy.layout(
      fcoseOptions({ randomize: true, animate: false, focalIds }),
    );
    layout.one("layoutstop", () => {
      syncVisibility();
      requestAnimationFrame(() => {
        if (!cy || !containerEl) return;
        cy.resize();
        const liveEles = cy.elements(":visible");

        // Lock the focal at origin so it visually anchors the
        // cluster. We also strip cytoscape-d3-force's built-in
        // forceCenter below — that auto-centers on the canvas
        // midpoint in world coords, fighting our locked-at-(0,0)
        // focal and stranding it on the canvas edge.
        const focalSet = new Set(focalIds);
        for (const id of focalIds) {
          const ele = cy.getElementById(id);
          if (ele.nonempty()) {
            ele.position({ x: 0, y: 0 });
            ele.addClass("focal");
            ele.lock();
          }
        }

        // Scatter visible non-focal nodes uniformly within a
        // 600x600 box around origin. d3-force will then pull
        // them inward under spring + center forces over the
        // alpha decay (~5s); that convergence IS the user-visible
        // "shake" on first draw / scope / depth / reload.
        liveEles.nodes().forEach((n) => {
          if (focalSet.has(n.id())) return;
          n.position({
            x: (Math.random() - 0.5) * INITIAL_BBOX_HALF * 2,
            y: (Math.random() - 0.5) * INITIAL_BBOX_HALF * 2,
          });
        });

        // Boot frame: cluster sits as a tiny dot at canvas center.
        // The post-settle animate-fit grows it from here to the
        // fitted size.
        cy.zoom(BOOT_ZOOM);
        cy.center();

        const liveNodeCount = liveEles.nodes().length;
        forceLayout = liveEles.layout(d3ForceOptions(liveNodeCount));

        forceLayout.one("layoutstop", () => {
          if (!cy) return;
          // Unlock the focal once d3-force has settled so the user
          // can drag it. The lock was only needed during layout to
          // keep the focal anchored at origin while the rest of the
          // cluster relaxed around it; past that, an immovable
          // central node is just an annoyance (especially in file
          // scope where the seed IS the file the user opened the
          // graph for).
          for (const id of focalIds) {
            const ele = cy.getElementById(id);
            if (ele.nonempty()) ele.unlock();
          }
          requestAnimationFrame(() => {
            if (!cy) return;
            const v = cy.elements(":visible");
            if (v.nonempty()) {
              // Two-stage animation: 900ms grow from BOOT_ZOOM to a
              // 0.5% overshoot of the target, then a 100ms ease back
              // to the exact target. Total 1s. The micro-overshoot
              // gives the reveal a tiny bounce so the cluster reads
              // as a soft landing instead of an abrupt halt.
              const bb = v.boundingBox({});
              const target = computeFitTarget(v, 30);
              const overshootZoom = target.zoom * 1.005;
              const overshootPan = panToCenter(bb, overshootZoom);
              cy.animate(
                { zoom: overshootZoom, pan: overshootPan },
                {
                  duration: 900,
                  easing: "ease-out",
                  complete: () => {
                    if (!cy) return;
                    cy.animate(
                      { zoom: target.zoom, pan: target.pan },
                      { duration: 100, easing: "ease-in-out" },
                    );
                  },
                },
              );
            }
          });
        });

        // Start d3-force ticking BEFORE revealing the canvas so
        // the first visible frame already has nodes in motion,
        // not a static scatter that briefly flashes before the
        // animation begins. ~180 ms (≈ 11 ticks at 60 fps) is
        // enough for visible drift; CSS opacity transition then
        // fades the canvas in over the still-running animation.
        forceLayout.run();

        // Strip cytoscape-d3-force's built-in forceCenter, which
        // shifts the centroid each tick toward (cy.width/2,
        // cy.height/2) in world coords (a fixed point unrelated
        // to our (0, 0) focal). With the focal locked at origin,
        // that constant pull dragged the rest of the cluster
        // away from the focal; with it removed, only our
        // xStrength/yStrength forces pull each node toward
        // (0, 0) — same target the focal is locked at, so they
        // align.
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const sim = (forceLayout as any).simulation;
        if (sim) sim.force("center", null);

        setTimeout(() => {
          if (!containerEl) return;
          containerEl.style.opacity = "";
        }, 180);

        cy.nodes().on("free unlock", () => {
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          const sim = (forceLayout as any)?.simulation;
          if (sim) sim.alphaTarget(0);
        });
      });
    });
    layout.run();

    // Suppress the first re-firing of the currentScope effect: the
    // current scope IS the one we just built for.
    lastScopeId = `${currentScope?.id ?? null}|${graphOverlay.depth}`;
  }

  /// Apply current scope + edge-kind filters by toggling the
  /// `.hidden` class on cytoscape elements. Layout positions are
  /// preserved, so re-enabling a filter snaps elements back to
  /// where they were.
  function syncVisibility(): void {
    if (!cy) return;
    // Local copy so the synthetic scope-hub admit below doesn't
    // leak into the derived Set (the stat counter reads its size).
    const visN = new Set(visibleNodeIds);
    const visE = new Set<string>();
    // Mirror makeEdgeIds: walk edges in order and key by base; the
    // visibleEdges array is a subset of `edges` in the same order,
    // so we recompute counters the same way.
    const counters = new Map<string, number>();
    for (const e of edges) {
      const base = `${e.source}|${e.target}|${e.kind}`;
      const n = (counters.get(base) ?? 0) + 1;
      counters.set(base, n);
      const id = n === 1 ? base : `${base}#${n}`;
      const wantVisible =
        show[e.kind] &&
        !hiddenImageIds.has(e.source) &&
        !hiddenImageIds.has(e.target) &&
        (scopedNodeIds === null ||
          (scopedNodeIds.has(e.source) && scopedNodeIds.has(e.target)));
      if (wantVisible) visE.add(id);
    }
    // Re-admit the synthetic scope-hub node and its star edges.
    // Neither is in `nodes` / `edges`, so the loops above would
    // let the cy passes below hide them. The hub is always visible
    // in group scope; an edge is visible whenever its file endpoint
    // is in `visN`.
    if (currentScope && currentScope.kind === "group") {
      visN.add(SCOPE_HUB_ID);
      for (const p of currentScope.paths) {
        if (visN.has(p)) visE.add(`group:${SCOPE_HUB_ID}|${p}`);
      }
    }
    cy.batch(() => {
      cy!.nodes().forEach((n) => {
        const id = n.id();
        if (visN.has(id)) n.removeClass("hidden");
        else n.addClass("hidden");
      });
      cy!.edges().forEach((e) => {
        const id = e.id();
        if (visE.has(id)) e.removeClass("hidden");
        else e.addClass("hidden");
      });
    });
  }


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


  // ---- mount: fetch + layout --------------------------------------------

  // GraphPanel itself mounts at app boot, but the canvas div lives
  // inside OverlayShell's {#if open} block, so containerEl doesn't
  // bind until the user actually opens the overlay. We can't drive
  // the load() from onMount — by then containerEl is undefined and
  // every build attempt bails. Drive it from a visibility effect
  // instead: build on open, tear down on close. Each open gets a
  // fresh container (Svelte mounts a new div each time), so we
  // rebuild from scratch each time, which also keeps the data
  // current.
  $effect(() => {
    if (!visible) {
      // Overlay just closed (or hasn't opened yet). Tear down so
      // the next open can build against the freshly-mounted DOM.
      resizeObs?.disconnect();
      resizeObs = null;
      forceLayout?.stop();
      forceLayout = null;
      cy?.destroy();
      cy = null;
      lastScopeId = null;
      return;
    }
    if (!containerEl) return; // wait for bind:this to fire
    if (cy) return; // already built for this open
    resizeObs = new ResizeObserver(() => {
      cy?.resize();
    });
    resizeObs.observe(containerEl);
    void load();
  });

  onDestroy(() => {
    resizeObs?.disconnect();
    forceLayout?.stop();
    forceLayout = null;
    cy?.destroy();
    cy = null;
  });

  async function load(): Promise<void> {
    loading = true;
    error = null;
    try {
      const g: GraphView = await api.graph();
      // Drop date nodes / edges only. Dates aren't authored as
      // graph endpoints in any UX surface today (issue #17), so a
      // stale row from an older index would render with no visual
      // affordance.
      const renderedNodes: RenderedNode[] = g.nodes.filter(
        (n): n is RenderedNode =>
          n.kind === "file" || n.kind === "tag" || n.kind === "mention",
      );
      const renderedEdges: RenderedEdge[] = g.edges.filter(
        (e): e is RenderedEdge =>
          e.kind === "link" || e.kind === "tag" || e.kind === "mention",
      );
      nodes = renderedNodes;
      edges = renderedEdges;
      await buildCytoscapeWhenSized({
        nodes: renderedNodes,
        edges: renderedEdges,
      });
      // Apply any pending selection handed in by openGraphAtNode.
      // Done after the cytoscape build so :selected can attach to
      // an actual element. Opening the side panel makes the
      // selection visible without a second click.
      const pending = graphOverlay.pendingSelectId;
      if (pending && nodes.some((n) => n.id === pending)) {
        selectedId = pending;
        graphOverlay.inspectorOpen = true;
      }
      graphOverlay.pendingSelectId = null;
    } catch (e) {
      error = (e as Error).message;
    } finally {
      loading = false;
    }
  }

  /// Defer buildCytoscape until containerEl actually has non-zero
  /// dimensions. The panel's flex sizing can still be settling on
  /// the first frame after mount; mounting cytoscape against a
  /// 0x0 container leaves its internal canvas sized wrong, and
  /// even cy.resize() afterwards can't always recover the first
  /// fit. Falls through after ~500ms regardless so we never hang.
  async function buildCytoscapeWhenSized(g: GraphView): Promise<void> {
    for (let i = 0; i < 30; i++) {
      if (!containerEl) return;
      const r = containerEl.getBoundingClientRect();
      if (r.width >= 80 && r.height >= 80) {
        buildCytoscape(g);
        return;
      }
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => resolve()),
      );
    }
    console.warn("[graph] container never reached usable size; building anyway");
    buildCytoscape(g);
  }

  // Scope or depth change → full rebuild. The visible-only fcose
  // pass that runs after the global layout sizes itself to whatever
  // node set is currently in scope; toggling depth grows that set,
  // so the new nodes need fresh positions or they show up at their
  // (now stale) global-fcose coordinates. Cheap at our scale (low
  // hundreds of nodes). Gated on the (id, depth) pair so the
  // tree-derived scopeOptions re-deriving doesn't kick a rebuild.
  $effect(() => {
    const id = currentScope?.id ?? null;
    const depth = graphOverlay.depth;
    if (!cy) return;
    const sig = `${id}|${depth}`;
    if (sig === lastScopeId) return;
    lastScopeId = sig;
    if (nodes.length > 0) void buildCytoscapeWhenSized({ nodes, edges });
  });

  // Re-apply visibility filters when scope/depth/edge-kind chips
  // change. Cheap; just toggles a class.
  $effect(() => {
    void visibleNodeIds;
    void visibleEdges;
    if (!cy) return;
    syncVisibility();
  });

  // Mirror external selection back into cytoscape so the
  // `:selected` style follows clicks made via the side panel.
  $effect(() => {
    if (!cy) return;
    const want = selectedId;
    cy.batch(() => {
      cy!.$(":selected").forEach((n) => {
        if (n.id() !== want) n.unselect();
      });
      if (want) {
        const ele = cy!.getElementById(want);
        if (ele.nonempty() && !ele.selected()) ele.select();
      }
    });
  });
</script>

<OverlayShell id="graph" open={visible} onClose={close}>
  <div class="graph-tab" oncontextmenu={onGraphContextMenu} role="presentation">
  <div class="bar">
    <span class="scope-label">Scope</span>
    <select
      class="scope-select"
      value={graphOverlay.scopeId}
      onchange={(e) =>
        (graphOverlay.scopeId = (e.currentTarget as HTMLSelectElement).value)}
      title="graph scope"
    >
      {#each scopeOptions as opt (opt.id)}
        <option value={opt.id} disabled={opt.enabled === false}>
          {opt.label}
        </option>
      {/each}
    </select>
    <button
      class="scope-history-btn"
      onclick={openScopeHistory}
      title="Open scope history"
      aria-label="Open scope history"
    >
      <Clock size={14} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <div class="filters">
      {#each ["link", "tag", "mention", "img"] as const as kind (kind)}
        <label class="chip" class:on={show[kind]}>
          <input type="checkbox" bind:checked={show[kind]} />
          <span class="dot" style="background:{FILTER_COLORS[kind]}"></span>
          {kind === "mention" ? "contact" : kind === "img" ? "media" : kind}
          <span class="count">{counts[kind]}</span>
        </label>
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
      <div class="placeholder">no markdown files in this drive yet</div>
    {/if}
    <div
      bind:this={containerEl}
      class="cy"
      class:dim={loading || !!error}
    ></div>
  </div>

  {#if graphOverlay.inspectorOpen}
    <Inspector
      title="Details"
      bind:width={paneWidths.graph}
      onResize={persistPaneWidths}
      onClose={() => (graphOverlay.inspectorOpen = false)}
    >
      {#if selectedNode && selectedNode.kind === "file" && isFileGhost}
        <!-- Ghost: either an explicit broken-link target, or the
             graph claims the file exists but it's not in the current
             tree listing (stale search index, common after a bulk
             drive change). FileInfoBody can't render either; surface
             inline inside the shared Inspector header. -->
        {@const ghostKind = isImage(selectedNode.path)
          ? "image"
          : selectedNode.node_kind === "contact"
            ? "contact"
            : "doc"}
        {@const hint = selectedNode.missing
          ? "file does not exist (broken-link target)"
          : "not in the current file listing (try Reload / chan index)"}
        <div class="ghost-body">
          <header class="head">
            <span class="kind-chip ghost {ghostKind}">{ghostKind}</span>
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
          onSetAsScope={
            selectedNode?.kind === "tag" ||
            (selectedNode?.kind === "mention" && selectedContactPath)
              ? () => {
                  // "Set as Scope" inside the graph re-scopes the
                  // current graph. Tag: to the tag's neighbourhood.
                  // Mention: to the resolved contact's file so the
                  // user can use a contact as a graph anchor without
                  // leaving the overlay.
                  if (selectedNode?.kind === "tag") {
                    graphOverlay.scopeId = `tag:${selectedNode.id}`;
                    graphOverlay.pendingSelectId = selectedNode.id;
                  } else if (
                    selectedNode?.kind === "mention" &&
                    selectedContactPath
                  ) {
                    graphOverlay.scopeId = `file:${selectedContactPath}`;
                    const fileNode = nodes.find(
                      (n) =>
                        n.kind === "file" &&
                        n.path === selectedContactPath,
                    );
                    if (fileNode) {
                      graphOverlay.pendingSelectId = fileNode.id;
                      selectedId = fileNode.id;
                    }
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
    <span class="stat">{visibleNodeIds.size}/{nodes.length} nodes · {visibleEdges.length}/{edges.length} edges</span>
    <span class="hint">drag to pan · scroll to zoom · drag a node to move · click to inspect</span>
  </div>
  </div>
</OverlayShell>

{#snippet menuItems()}
  <li>
    <button role="menuitem" onclick={toggleInspector}>
      {#if graphOverlay.inspectorOpen}
        <ArrowRight size={16} strokeWidth={1.75} aria-hidden="true" />
      {:else}
        <ArrowLeft size={16} strokeWidth={1.75} aria-hidden="true" />
      {/if}
      <span>{graphOverlay.inspectorOpen ? "Hide Details" : "Show Details"}</span>
    </button>
  </li>
  <li>
    <button role="menuitem" onclick={doToggleOverlayMaximized}>
      {#if overlayMaximized.on}
        <Minimize2 size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Restore size</span>
      {:else}
        <Maximize2 size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Maximize</span>
      {/if}
    </button>
  </li>
  <li class="sep" role="separator"></li>
  <!-- Depth slider is always in the menu so it doesn't disappear
       under the user when the scope toggles. Disabled on
       drive / global scopes (those always render everything
       regardless of hop count) so the affordance stays visible. -->
  {@const depthDisabled =
    !currentScope ||
    currentScope.kind === "drive" ||
    currentScope.kind === "global"}
  <li>
    <div class="menu-slider-row" class:disabled={depthDisabled}>
      <span class="menu-slider-label">Depth</span>
      <input
        type="range"
        min="1"
        max={DEPTH_MAX}
        step="1"
        bind:value={graphOverlay.depth}
        disabled={depthDisabled}
        onmousedown={(e) => e.stopPropagation()}
        aria-label="depth"
      />
      <span class="menu-slider-value">{graphOverlay.depth}</span>
    </div>
  </li>
  <li class="sep" role="separator"></li>
  <li>
    <button role="menuitem" onclick={reloadGraph}>
      <span class="glyph" aria-hidden="true">↻</span>
      <span>Reload</span>
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
  .scope-history-btn {
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
  .scope-history-btn:hover {
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
  .ghost-body .kind-chip {
    color: #fff;
    text-transform: uppercase;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 3px;
    flex: 1;
    text-align: center;
  }
  .ghost-body .kind-chip.ghost { background: var(--g-doc); opacity: 0.55; }
  .ghost-body .kind-chip.ghost.image { background: var(--g-img); }
  .ghost-body .kind-chip.ghost.contact { background: var(--warn-text); }
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
