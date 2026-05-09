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
  import type { Core, ElementDefinition, EventObject } from "cytoscape";
  // @ts-expect-error fcose ships no .d.ts; the layout name is enough
  import fcose from "cytoscape-fcose";

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

  // cytoscape.use is idempotent across module reloads.
  cytoscape.use(fcose);

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

  type EdgeKind = GraphViewEdge["kind"];

  // ---- state -------------------------------------------------------------

  let containerEl: HTMLDivElement | undefined = $state();
  let cy: Core | null = null;
  let resizeObs: ResizeObserver | null = null;

  /// Gates the currentScope $effect: scopeOptions re-derives any
  /// time the file tree updates, producing a fresh array (and thus
  /// a fresh currentScope object reference) even when the user
  /// hasn't actually changed scope. Without this gate every tree
  /// tick would kick off a relayout. Tracked by id, not by object
  /// identity, so the effect only acts on real transitions.
  let lastScopeId: string | null = null;

  let nodes: GraphViewNode[] = $state([]);
  let edges: GraphViewEdge[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);

  let show = $state<Record<EdgeKind, boolean>>({
    link: true,
    tag: true,
    mention: true,
    date: true,
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

  const visibleEdges = $derived(
    edges.filter(
      (e) =>
        show[e.kind] &&
        (scopedNodeIds === null ||
          (scopedNodeIds.has(e.source) && scopedNodeIds.has(e.target))),
    ),
  );
  const visibleNodeIds = $derived.by(() => {
    const ids = new Set<string>();
    for (const n of nodes) {
      if (scopedNodeIds !== null && !scopedNodeIds.has(n.id)) continue;
      if (n.kind === "file") ids.add(n.id);
    }
    for (const e of visibleEdges) {
      ids.add(e.source);
      ids.add(e.target);
    }
    return ids;
  });

  const counts = $derived.by(() => {
    const c: Record<EdgeKind, number> = { link: 0, tag: 0, mention: 0, date: 0 };
    for (const e of edges) c[e.kind]++;
    return c;
  });

  // ---- side-panel derived state ------------------------------------------

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
      documents: [] as Extract<GraphViewNode, { kind: "file" }>[],
    };
    if (!selectedId) return out;
    const sel = nodeById.get(selectedId);
    if (!sel) return out;
    for (const e of edges) {
      if (sel.kind === "file" && e.source === selectedId) {
        const target = nodeById.get(e.target);
        if (!target) continue;
        if (e.kind === "tag") out.tags.push(target);
        else if (e.kind === "mention") out.mentions.push(target);
        else if (e.kind === "date") out.dates.push(target);
        else if (e.kind === "link") out.links.push(target);
      } else if (sel.kind !== "file" && e.target === selectedId) {
        const source = nodeById.get(e.source);
        if (source && source.kind === "file") out.documents.push(source);
      }
    }
    return out;
  });

  // Compact byte-count formatting tuned for the dense graph aside.
  function formatSizeCompact(bytes: number): string {
    if (bytes < 1024) return `${bytes}`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}K`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)}M`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(1)}G`;
  }

  function openSelectedFile(): void {
    if (selectedNode && selectedNode.kind === "file" && !selectedNode.missing) {
      void openInActivePane(selectedNode.path);
      close();
    }
  }

  function selectFromList(n: GraphViewNode): void {
    selectedId = n.id;
    if (cy) {
      cy.$(":selected").unselect();
      const ele = cy.getElementById(n.id);
      if (ele.nonempty()) ele.select();
    }
  }

  // ---- presentation ------------------------------------------------------

  /// Used by the inspector's kind chip background. Cytoscape itself
  /// resolves these via getComputedStyle at buildCytoscape time, so
  /// theme changes propagate next reload.
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
      link: v("--link", "#4a90e2"),
      accent: v("--accent", "#9b6dff"),
      warn: v("--warn-text", "#e0a93b"),
      info: v("--info-text", "#7eaecc"),
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
          "text-outline-width": 2,
          "border-width": 1.5,
          "border-color": c.bg,
          "min-zoomed-font-size": 8,
        },
      },
      {
        selector: 'node[kind = "file"]',
        style: {
          "background-color": c.link,
          width: 14,
          height: 14,
        },
      },
      {
        selector: 'node[kind = "tag"]',
        style: {
          "background-color": c.accent,
          width: 8,
          height: 8,
          "font-size": 10,
        },
      },
      {
        selector: 'node[kind = "mention"]',
        style: {
          "background-color": c.warn,
          width: 8,
          height: 8,
          "font-size": 10,
        },
      },
      {
        selector: 'node[kind = "date"]',
        style: {
          "background-color": c.info,
          width: 8,
          height: 8,
          "font-size": 10,
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
          "border-color": c.accent,
          "border-width": 3,
          "overlay-color": c.accent,
          "overlay-opacity": 0.12,
          "overlay-padding": 2,
        },
      },
      {
        selector: "edge",
        style: {
          "curve-style": "bezier",
          "line-cap": "round",
          opacity: 0.7,
        },
      },
      {
        selector: 'edge[kind = "link"]',
        style: { "line-color": c.textSec, width: 1.6 },
      },
      {
        selector: 'edge[kind = "tag"]',
        style: { "line-color": c.accent, width: 1.1 },
      },
      {
        selector: 'edge[kind = "mention"]',
        style: { "line-color": c.warn, width: 1.1 },
      },
      {
        selector: 'edge[kind = "date"]',
        style: { "line-color": c.info, width: 1.1 },
      },
      {
        selector: "edge[?broken]",
        style: { "line-style": "dashed", opacity: 0.45 },
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
        kind: n.kind,
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
    cy?.destroy();
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
      maxZoom: 4,
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
      // Apply the visibility filter now that every node has a real
      // position; hidden ones won't pull the bounding box around.
      syncVisibility();
      requestAnimationFrame(() => {
        if (!cy) return;
        cy.resize();
        const vis = cy.elements(":visible");
        if (vis.nonempty()) cy.fit(vis, 30);
        for (const id of focalIds) {
          const ele = cy.getElementById(id);
          if (ele.nonempty()) {
            ele.addClass("focal");
            ele.lock();
          }
        }
      });
    });
    layout.run();

    // Suppress the first re-firing of the currentScope effect: the
    // current scope IS the one we just built for.
    lastScopeId = currentScope?.id ?? null;
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
    cy?.destroy();
    cy = null;
  });

  async function load(): Promise<void> {
    loading = true;
    error = null;
    try {
      const g: GraphView = await api.graph();
      nodes = g.nodes;
      edges = g.edges;
      await buildCytoscapeWhenSized(g);
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

  // Scope change → full rebuild. fcose's `fixedNodeConstraint`
  // only takes effect at layout time; soft repositioning after the
  // fact would leave neighbours where the previous layout placed
  // them, around the *old* anchor. A rebuild is cheap at our scale
  // (low hundreds of nodes) and keeps the focal node visually
  // centred. Gated on the scope id so the tree-derived scopeOptions
  // array re-deriving doesn't trigger a rebuild when the user
  // hasn't actually changed scope.
  $effect(() => {
    const id = currentScope?.id ?? null;
    if (!cy) return;
    if (id === lastScopeId) return;
    lastScopeId = id;
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
      {#each ["link", "tag", "mention", "date"] as const as kind (kind)}
        <label class="chip" class:on={show[kind]}>
          <input type="checkbox" bind:checked={show[kind]} />
          <span class="dot" style="background:{EDGE_COLORS[kind]}"></span>
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
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .details ul li { margin: 0; }
  /* Chip-styled refs, stacked one per line. Mirrors FileInfoBody's
     reference list so the file-browser inspector and the graph
     inspector share the same look. */
  .details .ref {
    display: block;
    width: 100%;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 2px 6px;
    text-align: left;
    cursor: pointer;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    line-height: 1.5;
    word-break: break-word;
  }
  .details .ref:hover {
    border-color: var(--btn-hover);
    background: var(--hover-bg);
  }
  .details .ref.tag { color: var(--accent); }
  .details .ref.mention { color: var(--warn-text); }
  .details .ref.date { color: var(--info-text); }
  .details .ref.file { color: var(--link); }
  .details li.doc-row {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .details li.doc-row .ref { flex: 1; }
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
