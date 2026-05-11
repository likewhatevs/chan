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
  import cytoscape from "cytoscape";
  import type { Core, ElementDefinition, EventObject, Layouts } from "cytoscape";
  // @ts-expect-error fcose ships no .d.ts; the layout name is enough
  import fcose from "cytoscape-fcose";
  // @ts-expect-error cytoscape-d3-force ships no .d.ts
  import d3Force from "cytoscape-d3-force";

  import { api } from "../api/client";
  import type { GraphView, GraphViewEdge, GraphViewNode } from "../api/types";
  import { openInActivePane } from "../state/tabs.svelte";
  import {
    availableGraphScopes,
    graphOverlay,
    paneWidths,
    persistPaneWidths,
    tree,
  } from "../state/store.svelte";
  import { type ScopeOption, defaultScopeId } from "../state/scope.svelte";
  import ResizeHandle from "./ResizeHandle.svelte";
  import OverlayShell from "./OverlayShell.svelte";
  import InspectorBody, { type InspectorSelection } from "./InspectorBody.svelte";

  // cytoscape.use is idempotent across module reloads.
  cytoscape.use(fcose);
  cytoscape.use(d3Force);

  // Visibility of the details aside lives on the overlay; per-window
  // session, not persisted to disk. Defaults closed.
  let panelOpen = $state(false);

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
  type RenderedEdgeKind = "link" | "tag" | "mention";
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
  /// visible nodes into at the start of every rebuild. 300 = a
  /// 600x600 model-space area centered on the focal at (0, 0).
  /// The pre-fit camera frames this box, so the first visible
  /// frame shows the explosion before d3-force pulls things in.
  const INITIAL_BBOX_HALF = 300;

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

  let show = $state<Record<FilterKind, boolean>>({
    link: true,
    tag: true,
    mention: true,
    img: true,
  });

  // Currently inspected node, surfaced in the side details panel.
  // Tap a node to set this; tap empty space to clear it. Nodes never
  // auto-open on click; the panel's Open button is the only path to
  // opening a file from here.
  let selectedId = $state<string | null>(null);

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
      if (n.kind === "file" && classifyFile(n.path) === "img") ids.add(n.id);
    }
    return ids;
  });

  const visibleEdges = $derived(
    edges.filter(
      (e) =>
        show[e.kind] &&
        !hiddenImageIds.has(e.source) &&
        !hiddenImageIds.has(e.target) &&
        (scopedNodeIds === null ||
          (scopedNodeIds.has(e.source) && scopedNodeIds.has(e.target))),
    ),
  );
  const visibleNodeIds = $derived.by(() => {
    const ids = new Set<string>();
    for (const n of nodes) {
      if (scopedNodeIds !== null && !scopedNodeIds.has(n.id)) continue;
      if (n.kind === "file" && !hiddenImageIds.has(n.id)) ids.add(n.id);
    }
    for (const e of visibleEdges) {
      ids.add(e.source);
      ids.add(e.target);
    }
    return ids;
  });

  /// Chip counts. Edge-kind chips report the underlying edge count;
  /// the `img` chip reports image-file-node count since that's what
  /// it actually toggles.
  const counts = $derived.by(() => {
    const c: Record<FilterKind, number> = { link: 0, tag: 0, mention: 0, img: 0 };
    for (const e of edges) c[e.kind]++;
    for (const n of nodes) {
      if (n.kind === "file" && classifyFile(n.path) === "img") c.img++;
    }
    return c;
  });

  // ---- side-panel derived state ------------------------------------------

  const nodeById = $derived(new Map(nodes.map((n) => [n.id, n])));

  const selectedNode = $derived<RenderedNode | null>(
    selectedId ? (nodeById.get(selectedId) ?? null) : null,
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
        ? selectedNode.missing
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
      // Node kinds in the graph: documents (orange file rectangles),
      // images (purple circles), tags (green hashtag labels), and
      // mentions (warn-colored @@name labels). Dates are filtered
      // upstream.
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

  function buildStylesheet(host: HTMLElement): cytoscape.StylesheetJson {
    const c = readThemeColors(host);
    return [
      {
        selector: "node",
        style: {
          label: "data(label)",
          color: c.text,
          // Match the rest of the chrome (App.svelte body
          // font-family) so graph labels read as part of the same
          // UI rather than a separate widget.
          "font-family":
            '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
          "font-size": 11,
          // Labels above the node, ellipsised at a fixed width.
          // Right-of-node placement (the prior layout) bled long
          // file names into neighbouring nodes once fcose packed
          // anything close together.
          "text-halign": "center",
          "text-valign": "top",
          "text-margin-y": -4,
          "text-wrap": "ellipsis",
          "text-max-width": "110px",
          "text-outline-color": c.bg,
          // Outline at 1px reads as a subtle halo for legibility
          // over edges/nodes; 2px made labels look bolder than the
          // surrounding UI text.
          "text-outline-width": 1,
          "border-width": 1.5,
          "border-color": c.bg,
          "min-zoomed-font-size": 8,
        },
      },
      // Documents: small orange rounded rectangle, label below.
      {
        selector: 'node[kind = "doc"]',
        style: {
          shape: "round-rectangle",
          "background-color": c.doc,
          width: 14,
          height: 18,
        },
      },
      // Images: small purple circle, label below.
      {
        selector: 'node[kind = "img"]',
        style: {
          shape: "ellipse",
          "background-color": c.img,
          width: 10,
          height: 10,
        },
      },
      // Tags: green hashtag text only, no fill, label centered on
      // the node so the "#name" string IS the visual.
      {
        selector: 'node[kind = "tag"]',
        style: {
          shape: "rectangle",
          "background-opacity": 0,
          "border-width": 0,
          width: 22,
          height: 14,
          color: c.tag,
          "font-size": 12,
          "font-weight": 600,
          "text-valign": "center",
          "text-halign": "center",
          "text-margin-y": 0,
          "text-outline-color": c.bg,
          "text-outline-width": 1,
        },
      },
      // Mentions: same text-only treatment as tags but in the warn
      // color so @@name reads as a different kind at a glance.
      {
        selector: 'node[kind = "mention"]',
        style: {
          shape: "rectangle",
          "background-opacity": 0,
          "border-width": 0,
          width: 28,
          height: 14,
          color: c.mention,
          "font-size": 12,
          "font-weight": 600,
          "text-valign": "center",
          "text-halign": "center",
          "text-margin-y": 0,
          "text-outline-color": c.bg,
          "text-outline-width": 1,
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
      // ~300 ticks (~5s at 60fps) for the simulation to cool from
      // alpha=1 to alphaMin. Doubled from the previous 150-tick
      // setting so the initial "shake" — and the same shake on
      // depth/scope/reload rebuilds — is visible long enough to
      // read as motion rather than a single snap.
      alphaDecay: 1 - Math.pow(0.05, 1 / 300),
      alphaTarget: 0,
      velocityDecay: 0.55,
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

  /// File-node ids matching the current scope's seed path(s). Used
  /// to pin the focal-anchor inside the fcose run. Returns an empty
  /// array for drive / global scope (no anchor wanted).
  function computeFocalNodeIds(): string[] {
    if (!currentScope) return [];
    let seedPaths: string[];
    if (currentScope.kind === "file") seedPaths = [currentScope.path];
    else if (currentScope.kind === "group") seedPaths = [...currentScope.paths];
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

  /// Files split into "doc" vs "img" by extension at element-build
  /// time; the GraphView type only knows "file". Anything not a
  /// recognised image extension counts as a document.
  function classifyFile(path: string): "doc" | "img" {
    return /\.(png|jpe?g|gif|webp|svg|avif|bmp)$/i.test(path) ? "img" : "doc";
  }

  function buildElements(g: GraphView): {
    elements: ElementDefinition[];
    dropped: number;
  } {
    const nodeIds = new Set(g.nodes.map((n) => n.id));
    const edgeIds = makeEdgeIds(g);
    const els: ElementDefinition[] = [];
    for (const n of g.nodes) {
      const data: Record<string, unknown> = {
        id: n.id,
        kind: n.kind === "file" ? classifyFile(n.path) : n.kind,
        label: n.label,
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
    return { elements: els, dropped };
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
    const { elements, dropped } = buildElements(g);
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
      style: buildStylesheet(containerEl),
      minZoom: 0.15,
      // Capped at 2 so the post-fit zoom on small scopes (5-15
      // visible nodes packed tight) doesn't blow up the labels.
      // cytoscape's font-size is model-space pixels, scaled by
      // zoom — zoom=5 turned an 11px font into 55px screen text.
      maxZoom: 2,
      boxSelectionEnabled: false,
      selectionType: "single",
    });

    cy.on("tap", "node", (ev: EventObject) => {
      const id = ev.target.id() as string;
      selectedId = id;
      panelOpen = true;
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

        // Fit camera to the scattered positions BEFORE reveal
        // so the first visible frame shows the full explosion.
        cy.fit(liveEles, 30);

        const liveNodeCount = liveEles.nodes().length;
        forceLayout = liveEles.layout(d3ForceOptions(liveNodeCount));

        forceLayout.one("layoutstop", () => {
          if (!cy) return;
          requestAnimationFrame(() => {
            if (!cy) return;
            const v = cy.elements(":visible");
            if (v.nonempty()) {
              // Smooth animated zoom-in to the settled cluster.
              cy.animate(
                { fit: { eles: v, padding: 30 } },
                { duration: 600, easing: "ease-out" },
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
    const visN = visibleNodeIds;
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
        panelOpen = true;
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

<OverlayShell open={visible} onClose={close}>
  <div class="graph-tab">
  <div class="bar">
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
    {#if currentScope && currentScope.kind !== "drive" && currentScope.kind !== "global"}
      <!-- Depth slider only matters when the scope is anchored to
           specific files; the drive (and eventual global) scopes
           always show everything regardless of hop count. -->
      <label class="depth" title="hops to expand from the seed file(s)">
        <span>depth</span>
        <input
          type="range"
          min="1"
          max="5"
          step="1"
          bind:value={graphOverlay.depth}
        />
        <span class="depth-val">{graphOverlay.depth}</span>
      </label>
    {/if}
    <div class="filters">
      {#each ["link", "tag", "mention", "img"] as const as kind (kind)}
        <label class="chip" class:on={show[kind]}>
          <input type="checkbox" bind:checked={show[kind]} />
          <span class="dot" style="background:{FILTER_COLORS[kind]}"></span>
          {kind}
          <span class="count">{counts[kind]}</span>
        </label>
      {/each}
    </div>
    <span class="actions">
      <button class="reload" onclick={() => void load()} title="Reload graph">↻</button>
      <button
        class="reload"
        class:on={panelOpen}
        onclick={() => (panelOpen = !panelOpen)}
        title={panelOpen ? "Hide details panel" : "Show details panel"}
      >≡</button>
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

  {#if panelOpen}
  <ResizeHandle
    bind:width={paneWidths.graph}
    onChange={() => persistPaneWidths()}
  />
  <aside class="details" style="width: {paneWidths.graph}px">
    {#if selectedNode && selectedNode.kind === "file" && selectedNode.missing}
      <!-- Ghost / broken-link target. Not in the file tree, so
           FileInfoBody can't render it; surface it inline. -->
      <header class="head">
        <span class="kind-chip ghost">doc</span>
        <button class="close" onclick={() => (selectedId = null)}>×</button>
      </header>
      <h3 class="title" title={selectedNode.path}>{selectedNode.label}</h3>
      <div class="path mono">{selectedNode.path}</div>
      <div class="missing">file does not exist (broken-link target)</div>
    {:else}
      <InspectorBody
        selection={inspectorSelection}
        onClose={() => (selectedId = null)}
        onOpen={openSelectedFile}
        onNavigate={selectByPath}
        documentsOverride={selectionDocumentsInScope}
      />
    {/if}
  </aside>
  {/if}
  </div>
  <div class="statusbar">
    <span class="stat">{visibleNodeIds.size}/{nodes.length} nodes · {visibleEdges.length}/{edges.length} edges</span>
    <span class="hint">drag to pan · scroll to zoom · drag a node to move · click to inspect</span>
  </div>
  </div>
</OverlayShell>

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
  .depth {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .depth input[type="range"] {
    width: 80px;
    accent-color: var(--link);
  }
  .depth-val {
    font-variant-numeric: tabular-nums;
    color: var(--text);
    min-width: 1ch;
    text-align: center;
  }
  .reload {
    background: transparent;
    border: 1px solid var(--btn-border);
    color: var(--text-secondary);
    border-radius: 4px;
    width: 22px;
    height: 22px;
    cursor: pointer;
  }
  .reload:hover {
    color: var(--text);
    border-color: var(--btn-hover);
  }
  .reload.on {
    color: var(--text);
    border-color: var(--btn-hover);
    background: var(--hover-bg);
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
  .bar > .actions {
    margin-left: auto;
    display: flex;
    gap: 2px;
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
  .details {
    flex-shrink: 0;
    border-left: 1px solid var(--border);
    background: var(--bg-card);
    color: var(--text);
    overflow-y: auto;
    padding: 0.6rem 0.7rem 0.8rem 0.7rem;
    font-size: 12.5px;
  }
  /* Most of the inspector now renders inside `<InspectorBody>` (file
     / tag / mention / date kinds share its styles). The rules below
     only style the inline ghost branch (file does not exist) since
     FileInfoBody can't render a path that's missing from the tree. */
  .details .head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.4rem;
  }
  .kind-chip {
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
  .kind-chip.ghost { background: var(--g-doc); opacity: 0.55; }
  .details .close {
    background: transparent;
    border: 0;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    padding: 0 4px;
  }
  .details .close:hover { color: var(--text); }
  .details .title {
    margin: 0 0 0.15rem 0;
    font-size: 16px;
    font-weight: 600;
    word-break: break-word;
  }
  .details .path {
    color: var(--text-secondary);
    font-size: 13px;
    margin-bottom: 0.5rem;
    word-break: break-all;
  }
  .details .mono { font-family: ui-monospace, monospace; }
  .details .missing {
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
