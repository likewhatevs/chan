<script lang="ts">
  // Graph view overlay: 2D force-directed (Obsidian-style) layout.
  //
  // d3-force simulates charge / link / center / collision forces;
  // we read the resulting (x, y) on each tick and re-render. When
  // the scope is "file" (or "group"), the seed file(s) are pinned
  // (fx/fy) to the canvas center so the user's note sits in the
  // middle and its neighbours orbit around it. When the scope is
  // "drive" or "global", no node is pinned and the whole graph
  // floats.
  //
  // Interaction:
  //   - drag empty space to pan (translate the canvas).
  //   - scroll wheel to zoom (uniform scale around the cursor).
  //   - drag a node to reposition it; release pins it back to the
  //     simulation. Hold while moving for a fluid drag.
  //   - click (no drag) selects: the side panel's Open button
  //     routes via openInActivePane and closes the overlay.
  //   - per-edge-kind filter chips toggle which edges (and the
  //     non-file nodes attached only to filtered edges) are drawn.

  import { onDestroy, onMount } from "svelte";
  import {
    forceCenter,
    forceCollide,
    forceLink,
    forceManyBody,
    forceSimulation,
    type Simulation,
    type SimulationLinkDatum,
    type SimulationNodeDatum,
  } from "d3-force";

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
  import { formatMtime } from "../state/format";
  import ResizeHandle from "./ResizeHandle.svelte";
  import OverlayShell from "./OverlayShell.svelte";

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

  /// One simulation node. Carries the original GraphViewNode fields
  /// plus the d3-force position fields (`x`, `y`, `vx`, `vy`,
  /// `fx`, `fy`). d3-force mutates these in place every tick.
  type LayoutNode = GraphViewNode &
    SimulationNodeDatum & {
      /// True when this node is currently pinned (fx/fy set). Used
      /// to apply a slightly different visual (no fade) so the user
      /// can spot the focal node at a glance.
      pinned?: boolean;
    };

  /// Edge with `source` / `target` re-typed as the live LayoutNode
  /// references d3-force populates after `forceLink` resolves them
  /// from string IDs. Keep `kind` and `broken` from the wire shape.
  type LayoutEdge = SimulationLinkDatum<LayoutNode> & {
    source: string | LayoutNode;
    target: string | LayoutNode;
    kind: GraphViewEdge["kind"];
    broken: boolean;
  };

  type EdgeKind = GraphViewEdge["kind"];

  // ---- state -------------------------------------------------------------

  let svgEl: SVGSVGElement | undefined = $state();
  let width = $state(800);
  let height = $state(600);

  // Pan / zoom on the canvas. `tx`, `ty` are translation in screen
  // pixels (relative to top-left of the SVG); `k` is the uniform
  // scale around the cursor. mousedown on empty space drags pan;
  // wheel adjusts zoom anchored on the cursor position.
  let tx = $state(0);
  let ty = $state(0);
  let k = $state(1);

  let nodes: LayoutNode[] = $state([]);
  let edges: LayoutEdge[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);

  /// Tick counter bumped on every simulation step. Reading it inside
  /// derivations (drawNodes / drawEdges) keeps the SVG in sync with
  /// d3-force's in-place position mutations without the bookkeeping
  /// of replacing the array on every frame.
  let tick = $state(0);

  /// d3-force handle. Recreated on every `load()`; stopped on the
  /// component destroy hook so the rAF loop doesn't outlive the
  /// overlay.
  let sim: Simulation<LayoutNode, LayoutEdge> | null = null;

  let show = $state<Record<EdgeKind, boolean>>({
    link: true,
    tag: true,
    mention: true,
    date: true,
  });

  let hoverId = $state<string | null>(null);
  // Visibility of the details aside lives on the tab struct so it
  // round-trips through session.json. Defaults closed for new tabs;
  // a user who left it open in this tab gets it back next launch.
  // Currently inspected node, surfaced in the side details panel.
  // Click a node to set this; click empty space to clear it; nodes
  // never auto-open on click any more (the panel's Open button is
  // the only path to opening a file from here).
  let selectedId = $state<string | null>(null);
  let resizeObs: ResizeObserver | null = null;

  // ---- derived: scope-filtered render set --------------------------------
  //
  // Two filters compose to decide what's drawn:
  //
  //   (1) the SCOPE picker in the header (file / group / drive).
  //       For file and group, BFS out from the seed paths up to
  //       graphOverlay.depth hops. Drive = no filter.
  //   (2) the per-edge-kind chips (link / tag / mention / date).
  //       Edges whose kind is filtered out are dropped, and any
  //       non-file node attached only via filtered edges drops too.
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
    // git_repo and dir scopes: seed BFS from every file node whose
    // path lives under the prefix. Both walk the same `depth`
    // neighbourhood as file/group scopes, just with a wider seed
    // set. Empty dir path collapses to the drive root and would
    // match every file, so currentScope.kind === "drive" already
    // handled that branch above.
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
        const s = edgeSourceId(e);
        const t = edgeTargetId(e);
        if (frontier.has(s) && !visited.has(t)) {
          next.add(t);
          visited.add(t);
        } else if (frontier.has(t) && !visited.has(s)) {
          next.add(s);
          visited.add(s);
        }
      }
      if (next.size === 0) break;
      frontier = next;
    }
    return visited;
  });

  /// d3-force replaces edge.source / edge.target with live node
  /// references after the first tick. Until then they're the
  /// string ids we passed in. Both branches surface as plain
  /// strings here.
  function edgeSourceId(e: LayoutEdge): string {
    return typeof e.source === "string" ? e.source : e.source.id;
  }
  function edgeTargetId(e: LayoutEdge): string {
    return typeof e.target === "string" ? e.target : e.target.id;
  }

  const visibleEdges = $derived(
    edges.filter(
      (e) =>
        show[e.kind] &&
        (scopedNodeIds === null ||
          (scopedNodeIds.has(edgeSourceId(e)) &&
            scopedNodeIds.has(edgeTargetId(e)))),
    ),
  );
  const visibleNodeIds = $derived.by(() => {
    const ids = new Set<string>();
    for (const n of nodes) {
      if (scopedNodeIds !== null && !scopedNodeIds.has(n.id)) continue;
      if (n.kind === "file") ids.add(n.id);
    }
    for (const e of visibleEdges) {
      ids.add(edgeSourceId(e));
      ids.add(edgeTargetId(e));
    }
    return ids;
  });

  const counts = $derived.by(() => {
    const c: Record<EdgeKind, number> = { link: 0, tag: 0, mention: 0, date: 0 };
    for (const e of edges) c[e.kind]++;
    return c;
  });

  // ---- side-panel derived state ------------------------------------------
  //
  // Looking up nodes by id is O(1) via this Map; same for mtime/size of
  // a file's tree entry. Both rebuild only when the underlying source
  // changes (graph payload or file tree refresh), not per render.

  const nodeById = $derived(new Map(nodes.map((n) => [n.id, n])));

  const fileEntryByPath = $derived.by(() => {
    const m = new Map<string, { mtime: number | null; size: number }>();
    for (const e of tree.entries) {
      if (!e.is_dir) m.set(e.path, { mtime: e.mtime, size: e.size });
    }
    return m;
  });

  const selectedNode = $derived<GraphViewNode | null>(
    selectedId ? (nodeById.get(selectedId) ?? null) : null,
  );

  /// Edges where `selectedId` is an endpoint, grouped for the side
  /// panel. For a file node this gives us its outgoing references
  /// per kind; for a tag/mention/date node, `documents` lists every
  /// file that references it.
  const selectionEdges = $derived.by(() => {
    const out = {
      tags: [] as GraphViewNode[],
      mentions: [] as GraphViewNode[],
      dates: [] as GraphViewNode[],
      links: [] as GraphViewNode[],
      // Documents are always file nodes (we filter on push), so
      // typing the array narrowly lets the template access
      // `path` / `missing` without re-narrowing every read.
      documents: [] as Extract<GraphViewNode, { kind: "file" }>[],
    };
    if (!selectedId) return out;
    const sel = nodeById.get(selectedId);
    if (!sel) return out;
    for (const e of edges) {
      const s = edgeSourceId(e);
      const t = edgeTargetId(e);
      if (sel.kind === "file" && s === selectedId) {
        const target = nodeById.get(t);
        if (!target) continue;
        if (e.kind === "tag") out.tags.push(target);
        else if (e.kind === "mention") out.mentions.push(target);
        else if (e.kind === "date") out.dates.push(target);
        else if (e.kind === "link") out.links.push(target);
      } else if (sel.kind !== "file" && t === selectedId) {
        const source = nodeById.get(s);
        if (source && source.kind === "file") out.documents.push(source);
      }
    }
    return out;
  });

  // Compact byte-count formatting tuned for the dense graph aside;
  // strips the unit letter so a 12.3K row stays narrow next to a row
  // labeled `size`. The verbose `formatSize` from `state/format` is
  // used everywhere else.
  function formatSizeCompact(bytes: number): string {
    if (bytes < 1024) return `${bytes}`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}K`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)}M`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(1)}G`;
  }

  function openSelectedFile(): void {
    if (selectedNode && selectedNode.kind === "file" && !selectedNode.missing) {
      void openInActivePane(selectedNode.path);
      // Close the overlay so the workspace pane gets the focus
      // immediately rather than the user clicking through a dim
      // backdrop to start editing.
      close();
    }
  }

  function selectFromList(n: GraphViewNode): void {
    selectedId = n.id;
  }

  // ---- d3-force layout helpers ------------------------------------------

  /// Visible nodes alone (the simulation runs on the full graph;
  /// rendering only shows what scope/chip filters allow). Reading
  /// `tick` ties the renderer to the simulation's per-frame state
  /// mutations so the SVG stays in sync without the bookkeeping
  /// of replacing the array on every tick.
  const drawNodes = $derived.by<LayoutNode[]>(() => {
    void tick;
    return nodes.filter((n) => visibleNodeIds.has(n.id));
  });

  /// Visible edges with `source` / `target` resolved to live nodes
  /// for the SVG <line>'s x1/y1/x2/y2 coords. d3-force overwrites
  /// these from string ids on first tick; until then we resolve
  /// against `nodes` ourselves.
  const drawEdges = $derived.by(() => {
    void tick;
    const idIndex = new Map(nodes.map((n) => [n.id, n]));
    return visibleEdges
      .map((e) => {
        const a = typeof e.source === "string" ? idIndex.get(e.source) : e.source;
        const b = typeof e.target === "string" ? idIndex.get(e.target) : e.target;
        if (!a || !b) return null;
        return { e, a, b };
      })
      .filter((x): x is { e: LayoutEdge; a: LayoutNode; b: LayoutNode } => !!x);
  });

  // Re-pin the focal node(s) whenever the scope changes. For "file"
  // / "group" / "git_repo" / "dir" scope, the seed file(s) get
  // pinned to the canvas center (or fanned out evenly when there
  // are several). For "drive" / "global", no pinning — the whole
  // graph floats in the force field.
  $effect(() => {
    void currentScope;
    void width;
    void height;
    if (!sim) return;
    pinFocalNodes();
    sim.alpha(0.6).restart();
  });

  // ---- mount: fetch + layout --------------------------------------------

  onMount(async () => {
    if (svgEl) {
      const r = svgEl.getBoundingClientRect();
      width = Math.max(200, r.width);
      height = Math.max(200, r.height);
      resizeObs = new ResizeObserver((entries) => {
        for (const ent of entries) {
          width = Math.max(200, ent.contentRect.width);
          height = Math.max(200, ent.contentRect.height);
          if (sim) {
            sim.force("center", forceCenter(width / 2, height / 2));
            // Re-pin so the focal node stays centered when the
            // viewport changes.
            pinFocalNodes();
            sim.alpha(0.4).restart();
          }
        }
      });
      resizeObs.observe(svgEl);
    }
    await load();
  });

  onDestroy(() => {
    resizeObs?.disconnect();
    sim?.stop();
    sim = null;
  });

  async function load(): Promise<void> {
    loading = true;
    error = null;
    try {
      const g: GraphView = await api.graph();
      buildSimulation(g);
    } catch (e) {
      error = (e as Error).message;
    } finally {
      loading = false;
    }
  }

  /// Build (or rebuild) the d3-force simulation over `g`. Stops the
  /// previous simulation if any. Initial node positions are seeded
  /// in a small jittered cluster around the canvas center so the
  /// first frame doesn't show a starburst from (0, 0); the forces
  /// then settle them into place over a couple of seconds.
  function buildSimulation(g: GraphView): void {
    sim?.stop();
    const cx = width / 2;
    const cy = height / 2;
    const layoutNodes: LayoutNode[] = g.nodes.map((n) => ({
      ...n,
      x: cx + (Math.random() - 0.5) * 80,
      y: cy + (Math.random() - 0.5) * 80,
    }));
    // d3-force's `forceLink` throws "node not found" when an edge
    // references an id that isn't in the node set. The graph DB
    // can hand us dangling edges (e.g. a `## glutenfree` heading
    // mis-tokenised as the tag `##glutenfree` whose node never
    // got created), and we don't want a single bad row to take
    // down the whole panel. Drop edges with unknown endpoints
    // before handing the array to the simulation; the rest of
    // the graph still renders.
    const nodeIds = new Set(layoutNodes.map((n) => n.id));
    const layoutEdges: LayoutEdge[] = [];
    let dropped = 0;
    for (const e of g.edges) {
      if (!nodeIds.has(e.source) || !nodeIds.has(e.target)) {
        dropped += 1;
        continue;
      }
      layoutEdges.push({
        source: e.source,
        target: e.target,
        kind: e.kind,
        broken: e.broken ?? false,
      });
    }
    if (dropped > 0) {
      console.warn(`graph: dropped ${dropped} edges with unknown endpoints`);
    }
    nodes = layoutNodes;
    edges = layoutEdges;

    sim = forceSimulation<LayoutNode>(layoutNodes)
      .force(
        "link",
        forceLink<LayoutNode, LayoutEdge>(layoutEdges)
          .id((n) => n.id)
          // Slightly longer links for file<->file (the dominant
          // edge kind) so the dense recipe-cluster still has room
          // to breathe; tighter for tag/mention/date so satellites
          // sit close to their owner.
          .distance((e) => (e.kind === "link" ? 70 : 40))
          .strength(0.6),
      )
      .force("charge", forceManyBody<LayoutNode>().strength(-180))
      .force("center", forceCenter(cx, cy))
      .force("collide", forceCollide<LayoutNode>().radius(18).strength(0.7))
      .alphaDecay(0.03)
      .on("tick", () => {
        // Bumping `tick` is what tells the reactive renderer to
        // re-read the in-place-mutated x/y on each node. Cheap;
        // d3-force ticks ~30 times before settling.
        tick = (tick + 1) | 0;
      });

    pinFocalNodes();
  }

  /// Pin the focal-scope file node(s) at fixed positions so the
  /// "selected file" sits in the middle and its neighbours orbit.
  /// Multiple seed paths (group / dir scope) fan out evenly on a
  /// circle around the center. Drive / global scope leaves all
  /// nodes free to move; pin state is cleared on the way out.
  function pinFocalNodes(): void {
    const cx = width / 2;
    const cy = height / 2;
    // Identify seed paths from the current scope.
    let seedPaths: string[] | null = null;
    if (currentScope) {
      if (currentScope.kind === "file") seedPaths = [currentScope.path];
      else if (currentScope.kind === "group") seedPaths = [...currentScope.paths];
      else if (currentScope.kind === "dir") seedPaths = filesUnder(currentScope.path);
      else if (currentScope.kind === "git_repo")
        seedPaths = filesUnder(currentScope.root);
    }
    // Clear any prior pinning first so swapping scopes never leaves
    // stale fixed positions behind.
    for (const n of nodes) {
      n.fx = null;
      n.fy = null;
      n.pinned = false;
    }
    if (!seedPaths || seedPaths.length === 0) return;
    const seedNodes = nodes.filter(
      (n) => n.kind === "file" && seedPaths!.includes(n.path),
    );
    if (seedNodes.length === 1) {
      const n = seedNodes[0]!;
      n.fx = cx;
      n.fy = cy;
      n.pinned = true;
    } else {
      const r = Math.min(width, height) * 0.18;
      seedNodes.forEach((n, i) => {
        const angle = (i / seedNodes.length) * Math.PI * 2 - Math.PI / 2;
        n.fx = cx + Math.cos(angle) * r;
        n.fy = cy + Math.sin(angle) * r;
        n.pinned = true;
      });
    }
  }

  /// Helper used by `pinFocalNodes` for the dir / git_repo scopes:
  /// every file node whose path lives under the prefix.
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

  // ---- interaction: pan + zoom + node drag ------------------------------

  /// Pan drag on empty SVG: translates the canvas. Anchored to
  /// the cursor so the user "grabs" the graph at the click point.
  let panStart: { mx: number; my: number; tx: number; ty: number } | null = null;
  /// Node drag: temporarily pins the dragged node so the simulation
  /// follows the cursor without yanking the rest of the graph.
  /// `moved` discriminates click vs drag for selection semantics.
  const DRAG_THRESHOLD = 4;
  let nodeDown:
    | { node: LayoutNode; mx: number; my: number; moved: boolean }
    | null = null;

  function onSvgMouseDown(e: MouseEvent): void {
    panStart = { mx: e.clientX, my: e.clientY, tx, ty };
    window.addEventListener("mousemove", onPanMove);
    window.addEventListener("mouseup", onPanUp);
  }

  function onPanMove(e: MouseEvent): void {
    if (!panStart) return;
    tx = panStart.tx + (e.clientX - panStart.mx);
    ty = panStart.ty + (e.clientY - panStart.my);
  }

  function onPanUp(): void {
    panStart = null;
    window.removeEventListener("mousemove", onPanMove);
    window.removeEventListener("mouseup", onPanUp);
  }

  function onWheel(e: WheelEvent): void {
    e.preventDefault();
    const factor = Math.exp(-e.deltaY * 0.0015);
    const next = Math.max(0.25, Math.min(4, k * factor));
    if (next === k) return;
    // Zoom anchored on the cursor: keep the world-point under
    // the pointer fixed by adjusting tx/ty alongside k.
    const rect = svgEl?.getBoundingClientRect();
    if (!rect) {
      k = next;
      return;
    }
    const px = e.clientX - rect.left;
    const py = e.clientY - rect.top;
    tx = px - ((px - tx) * next) / k;
    ty = py - ((py - ty) * next) / k;
    k = next;
  }

  function onNodeMouseDown(e: MouseEvent, n: LayoutNode): void {
    e.stopPropagation();
    nodeDown = { node: n, mx: e.clientX, my: e.clientY, moved: false };
    window.addEventListener("mousemove", onNodeMove);
    window.addEventListener("mouseup", onNodeUp);
  }

  function onNodeMove(e: MouseEvent): void {
    if (!nodeDown || !sim || !svgEl) return;
    const dx = e.clientX - nodeDown.mx;
    const dy = e.clientY - nodeDown.my;
    if (!nodeDown.moved && Math.hypot(dx, dy) >= DRAG_THRESHOLD) {
      nodeDown.moved = true;
      sim.alphaTarget(0.3).restart();
    }
    if (!nodeDown.moved) return;
    const rect = svgEl.getBoundingClientRect();
    // Convert client coords -> SVG coords by undoing the world
    // transform (translate(tx, ty) scale(k)).
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;
    nodeDown.node.fx = (sx - tx) / k;
    nodeDown.node.fy = (sy - ty) / k;
  }

  function onNodeUp(): void {
    if (!nodeDown) return;
    if (!nodeDown.moved) {
      selectedId = nodeDown.node.id;
    } else {
      sim?.alphaTarget(0);
      // Release the drag pin UNLESS this node is the focal pin
      // (kept pinned by `pinFocalNodes`); the `pinned` flag
      // discriminates.
      if (!nodeDown.node.pinned) {
        nodeDown.node.fx = null;
        nodeDown.node.fy = null;
      }
    }
    nodeDown = null;
    window.removeEventListener("mousemove", onNodeMove);
    window.removeEventListener("mouseup", onNodeUp);
  }

  function resetView(): void {
    tx = 0;
    ty = 0;
    k = 1;
    if (sim) {
      pinFocalNodes();
      sim.alpha(0.6).restart();
    }
  }

  // ---- presentation ------------------------------------------------------

  const NODE_COLORS: Record<GraphViewNode["kind"], string> = {
    file: "var(--link)",
    tag: "var(--accent)",
    mention: "var(--warn-text)",
    date: "var(--info-text)",
  };

  const EDGE_COLORS: Record<EdgeKind, string> = {
    link: "var(--text-secondary)",
    tag: "var(--accent)",
    mention: "var(--warn-text)",
    date: "var(--info-text)",
  };

  /// Visual radius. Files read larger than tag/mention/date
  /// satellites so the dominant content type is unmistakable;
  /// the focal-pinned node gets a small extra so the user sees
  /// the "you are here" anchor at a glance.
  function nodeRadius(n: LayoutNode): number {
    const base = n.kind === "file" ? 7 : 4;
    return n.pinned ? base + 2 : base;
  }
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
           always show everything
           regardless of hop count. -->
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
      {#each ["link", "tag", "mention", "date"] as const as kind (kind)}
        <label class="chip" class:on={show[kind]}>
          <input type="checkbox" bind:checked={show[kind]} />
          <span class="dot" style="background:{EDGE_COLORS[kind]}"></span>
          {kind}
          <span class="count">{counts[kind]}</span>
        </label>
      {/each}
    </div>
    <!-- Right-aligned actions, mirrors the other overlay headers. -->
    <span class="actions">
      <button class="reload" onclick={() => void load()} title="Reload graph">↻</button>
      <button class="reload" onclick={resetView} title="Reset view">⌖</button>
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
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <svg
      bind:this={svgEl}
      class:dim={loading || !!error}
      onmousedown={onSvgMouseDown}
      onwheel={onWheel}
    >
      <!-- World group: a single translate(tx, ty) scale(k) parents
           every node + edge so the canvas pans and zooms uniformly
           without re-running the simulation. -->
      <g transform={`translate(${tx}, ${ty}) scale(${k})`}>
        {#each drawEdges as { e, a, b } (`${a.id}->${b.id}-${e.kind}`)}
          <line
            x1={a.x ?? 0}
            y1={a.y ?? 0}
            x2={b.x ?? 0}
            y2={b.y ?? 0}
            stroke={EDGE_COLORS[e.kind]}
            stroke-opacity={e.broken ? 0.45 : 0.7}
            stroke-dasharray={e.broken ? "3 3" : undefined}
            stroke-width={e.kind === "link" ? 1.6 : 1.1}
            stroke-linecap="round"
          />
        {/each}

        {#each drawNodes as n (n.id)}
          <g
            class="node"
            class:file={n.kind === "file"}
            class:missing={n.kind === "file" && n.missing}
            class:focal={n.pinned}
            transform={`translate(${n.x ?? 0}, ${n.y ?? 0})`}
            onmousedown={(ev) => onNodeMouseDown(ev, n)}
            onmouseenter={() => (hoverId = n.id)}
            onmouseleave={() => (hoverId = hoverId === n.id ? null : hoverId)}
            role="button"
            tabindex="0"
          >
            {#if selectedId === n.id}
              <!-- Selection ring: draws underneath the node body so
                   the colored fill stays visible. -->
              <circle
                r={nodeRadius(n) + 4}
                fill="none"
                stroke="var(--accent)"
                stroke-width="2"
              />
            {/if}
            <circle r={nodeRadius(n)} fill={NODE_COLORS[n.kind]} />
            <text
              class="label-bg"
              x={nodeRadius(n) + 5}
              y={3}
              font-size={n.kind === "file" ? 11 : 10}
              pointer-events="none"
            >{n.label}</text>
            <text
              class="label"
              x={nodeRadius(n) + 5}
              y={3}
              font-size={n.kind === "file" ? 11 : 10}
              pointer-events="none"
            >{n.label}</text>
          </g>
        {/each}
      </g>
    </svg>
  </div>

  {#if panelOpen}
  <ResizeHandle
    bind:width={paneWidths.graph}
    onChange={() => persistPaneWidths()}
  />
  <aside class="details" style="width: {paneWidths.graph}px">
    {#if !selectedNode}
      <div class="empty">
        <div class="empty-title">Details</div>
        <div class="empty-hint">click a node to inspect it</div>
      </div>
    {:else if selectedNode.kind === "file"}
      <header class="head">
        <span class="kind-chip" style="background: {NODE_COLORS.file}">file</span>
        <button class="close" onclick={() => (selectedId = null)}>×</button>
      </header>
      <h3 class="title" title={selectedNode.path}>{selectedNode.label}</h3>
      <div class="path mono">{selectedNode.path}</div>

      {#if selectedNode.missing}
        <div class="missing">file does not exist (broken-link target)</div>
      {:else}
        {@const meta = fileEntryByPath.get(selectedNode.path)}
        <div class="meta-grid">
          <span class="k">size</span>
          <span class="v">{meta ? formatSizeCompact(meta.size) : "?"}</span>
          <span class="k">modified</span>
          <span class="v">{meta ? formatMtime(meta.mtime) : "?"}</span>
          <span class="k">tags</span>
          <span class="v">{selectionEdges.tags.length}</span>
          <span class="k">mentions</span>
          <span class="v">{selectionEdges.mentions.length}</span>
          <span class="k">dates</span>
          <span class="v">{selectionEdges.dates.length}</span>
          <span class="k">links out</span>
          <span class="v">{selectionEdges.links.length}</span>
        </div>
        <button class="open" onclick={openSelectedFile}>Open in this pane</button>
      {/if}

      {#if selectionEdges.tags.length > 0}
        <section>
          <h4>Tags</h4>
          <ul>
            {#each selectionEdges.tags as t (t.id)}
              <li><button class="ref tag" onclick={() => selectFromList(t)}>{t.label}</button></li>
            {/each}
          </ul>
        </section>
      {/if}
      {#if selectionEdges.mentions.length > 0}
        <section>
          <h4>Mentions</h4>
          <ul>
            {#each selectionEdges.mentions as m (m.id)}
              <li><button class="ref mention" onclick={() => selectFromList(m)}>{m.label}</button></li>
            {/each}
          </ul>
        </section>
      {/if}
      {#if selectionEdges.dates.length > 0}
        <section>
          <h4>Dates</h4>
          <ul>
            {#each selectionEdges.dates as d (d.id)}
              <li><button class="ref date" onclick={() => selectFromList(d)}>{d.label}</button></li>
            {/each}
          </ul>
        </section>
      {/if}
      {#if selectionEdges.links.length > 0}
        <section>
          <h4>Links to</h4>
          <ul>
            {#each selectionEdges.links as l (l.id)}
              <li><button class="ref file" onclick={() => selectFromList(l)}>{l.label}</button></li>
            {/each}
          </ul>
        </section>
      {/if}
    {:else}
      <header class="head">
        <span class="kind-chip" style="background: {NODE_COLORS[selectedNode.kind]}">{selectedNode.kind}</span>
        <button class="close" onclick={() => (selectedId = null)}>×</button>
      </header>
      <h3 class="title">{selectedNode.label}</h3>
      <div class="meta-grid">
        <span class="k">documents</span>
        <span class="v">{selectionEdges.documents.length}</span>
      </div>
      {#if selectionEdges.documents.length === 0}
        <div class="empty-hint">no documents reference this</div>
      {:else}
        <section>
          <h4>Documents</h4>
          <ul>
            {#each selectionEdges.documents as f (f.id)}
              <li class="doc-row">
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <span
                  class="ref file"
                  onclick={() => selectFromList(f)}
                  role="button"
                  tabindex="0"
                  title="select"
                >{f.label}</span>
                {#if !f.missing}
                  <button
                    class="row-open"
                    onclick={() => {
                      void openInActivePane(f.path);
                      close();
                    }}
                    title="open in the active pane"
                  >open</button>
                {/if}
              </li>
            {/each}
          </ul>
        </section>
      {/if}
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
  /* Scope picker mirrors the assistant overlay's context-select so
     the two surfaces feel like siblings. */
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
  /* Bottom status bar. Carries the node/edge counts (left) and the
     interaction hint (right) so they don't compete with scope and
     filters in the top bar. Mirrors the .bar treatment so the panel
     reads as toolbar + canvas + statusbar. */
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
  /* Push the action buttons (reload, reset-view, panel-toggle) to
     the right edge of the bar, matching the other tab kinds. */
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
  /* Row containing the globe canvas (flex:1) and the details aside
     (fixed width). Sits below the toolbar and above any future
     status row. */
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
  /* Details panel: equivalent to a file tab's inspector. Holds
     selection metadata and edge lists; clicks inside the panel
     either re-select another node (chains exploration) or open a
     file via the explicit button. */
  .details {
    /* width is set inline by the parent (paneWidths.graph) so the
       resize handle updates apply without a CSS rule rewrite. */
    flex-shrink: 0;
    border-left: 1px solid var(--border);
    background: var(--bg-card);
    color: var(--text);
    overflow-y: auto;
    padding: 0.6rem 0.7rem 0.8rem 0.7rem;
    font-size: 12.5px;
  }
  .details .empty {
    text-align: center;
    color: var(--text-secondary);
    padding-top: 1.2rem;
  }
  .details .empty-title {
    font-weight: 600;
    color: var(--text);
    margin-bottom: 0.25rem;
  }
  .details .empty-hint {
    font-style: italic;
    font-size: 14px;
    opacity: 0.85;
  }
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
  .details .meta-grid {
    display: grid;
    grid-template-columns: 6.5em 1fr;
    gap: 2px 0.5rem;
    margin: 0.4rem 0 0.6rem 0;
    font-size: 14px;
  }
  .details .meta-grid .k { color: var(--text-secondary); }
  .details .meta-grid .v {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .details .open {
    width: 100%;
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 5px 0;
    cursor: pointer;
    font: inherit;
    margin-bottom: 0.5rem;
  }
  .details .open:hover { border-color: var(--btn-hover); }
  .details section { margin-top: 0.55rem; }
  .details h4 {
    margin: 0 0 0.2rem 0;
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
  }
  .details ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  /* Reference rows: a list item containing a single button. The
     button reset matches the file tree's `.name` style so visited
     rows look identical regardless of which surface they live in. */
  .details button.ref {
    width: 100%;
    background: none;
    border: 0;
    text-align: left;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 3px;
    color: var(--text);
    font: inherit;
  }
  .details button.ref:hover { background: var(--hover-bg); }
  .details button.ref.tag { color: var(--accent); }
  .details button.ref.mention { color: var(--warn-text); }
  .details button.ref.date { color: var(--accent); }
  .details li.doc-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 1px 0;
  }
  .details li.doc-row .ref {
    flex: 1;
    padding: 2px 4px;
    border-radius: 3px;
    cursor: pointer;
  }
  .details li.doc-row .ref:hover { background: var(--hover-bg); }
  .details .row-open {
    background: transparent;
    border: 1px solid var(--btn-border);
    color: var(--text-secondary);
    border-radius: 3px;
    padding: 1px 6px;
    font-size: 12px;
    cursor: pointer;
  }
  .details .row-open:hover {
    color: var(--text);
    border-color: var(--btn-hover);
  }
  svg {
    width: 100%;
    height: 100%;
    display: block;
    cursor: grab;
    user-select: none;
  }
  svg.dim {
    opacity: 0.4;
  }
  svg:active {
    cursor: grabbing;
  }
  .node {
    cursor: pointer;
  }
  /* Focal node: subtly brighter halo so the user can spot the
     "you are here" anchor amid an otherwise uniform colour set.
     Drawn as a stroke on the inner circle (not the selection
     ring, which is reserved for click-selection). */
  .node.focal circle:last-of-type {
    stroke: var(--accent);
    stroke-width: 1.5;
  }
  .node.file circle {
    stroke: var(--bg);
    stroke-width: 1.5;
  }
  .node.missing circle {
    fill: var(--bg-card);
    stroke: var(--text-secondary);
    stroke-dasharray: 2 2;
  }
  .node.missing {
    cursor: not-allowed;
    opacity: 0.6;
  }
  .node:hover circle {
    stroke: var(--text);
    stroke-width: 1.5;
  }
  .label {
    fill: var(--text);
  }
  .label-bg {
    fill: var(--bg);
    stroke: var(--bg);
    stroke-width: 3;
    paint-order: stroke fill;
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
