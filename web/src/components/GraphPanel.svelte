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
          "text-halign": "right",
          "text-valign": "center",
          "text-margin-x": 5,
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
  function fcoseOptions(opts: { randomize: boolean; animate: boolean }) {
    return {
      name: "fcose",
      quality: "default",
      randomize: opts.randomize,
      animate: opts.animate,
      animationDuration: opts.animate ? 350 : 0,
      fit: opts.randomize,
      padding: 30,
      nodeSeparation: 75,
      idealEdgeLength: (e: cytoscape.EdgeSingular) =>
        e.data("kind") === "link" ? 70 : 40,
      edgeElasticity: 0.45,
      nestingFactor: 0.1,
      numIter: 2500,
      gravity: 0.25,
      packComponents: true,
      // fcose honours node.locked() — focal pins stick across runs.
    } as cytoscape.LayoutOptions;
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
    cy = cytoscape({
      container: containerEl,
      elements,
      style: buildStylesheet(containerEl),
      layout: fcoseOptions({ randomize: true, animate: false }),
      wheelSensitivity: 0.2,
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

    // Initial filter pass: scope/edge-kind chips might be non-defaults.
    syncVisibility();
    pinFocalNodes(false);
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

  /// Pin the focal-scope file node(s) at fixed positions so the
  /// "selected file" sits in the middle and its neighbours orbit.
  /// Multiple seed paths (group / dir / git_repo scope) fan out
  /// evenly on a circle around the centre. Drive / global scope
  /// leaves all nodes free to move; pin state is cleared on the way
  /// out.
  function pinFocalNodes(relax: boolean): void {
    if (!cy) return;
    cy.nodes().unlock();
    cy.nodes().removeClass("focal");
    let seedPaths: string[] | null = null;
    if (currentScope) {
      if (currentScope.kind === "file") seedPaths = [currentScope.path];
      else if (currentScope.kind === "group") seedPaths = [...currentScope.paths];
      else if (currentScope.kind === "dir") seedPaths = filesUnder(currentScope.path);
      else if (currentScope.kind === "git_repo")
        seedPaths = filesUnder(currentScope.root);
    }
    if (!seedPaths || seedPaths.length === 0) return;
    const seeds = cy.nodes().filter((n) => {
      if (n.data("kind") !== "file") return false;
      const p = n.data("path") as string | undefined;
      return p != null && seedPaths!.includes(p);
    });
    if (seeds.length === 0) return;
    const ext = cy.extent();
    const cx = (ext.x1 + ext.x2) / 2;
    const cyc = (ext.y1 + ext.y2) / 2;
    if (seeds.length === 1) {
      const n = seeds[0]!;
      n.position({ x: cx, y: cyc });
      n.lock();
      n.addClass("focal");
    } else {
      const r = Math.min(ext.x2 - ext.x1, ext.y2 - ext.y1) * 0.18;
      seeds.forEach((n, i) => {
        const angle = (i / seeds.length) * Math.PI * 2 - Math.PI / 2;
        n.position({ x: cx + Math.cos(angle) * r, y: cyc + Math.sin(angle) * r });
        n.lock();
        n.addClass("focal");
      });
    }
    if (relax) {
      cy.layout(fcoseOptions({ randomize: false, animate: true })).run();
    }
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

  function resetView(): void {
    if (!cy) return;
    cy.fit(undefined, 30);
  }

  // ---- mount: fetch + layout --------------------------------------------

  onMount(async () => {
    if (containerEl) {
      resizeObs = new ResizeObserver(() => {
        cy?.resize();
      });
      resizeObs.observe(containerEl);
    }
    // Defer one frame so the OverlayShell's growth-to-fullscreen
    // transition has settled before we measure the canvas and seed
    // the layout. Without this the first measurement can land
    // mid-animation and the layout settles around an off-centre
    // point.
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => resolve()),
    );
    await load();
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
      buildCytoscape(g);
    } catch (e) {
      error = (e as Error).message;
    } finally {
      loading = false;
    }
  }

  // Re-pin when scope changes (focal node set or pin policy may
  // shift). Skip while cy isn't built yet; load() does the first
  // pass.
  $effect(() => {
    void currentScope;
    if (!cy) return;
    pinFocalNodes(true);
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
  /* Cytoscape mount: fills the canvas area. The library handles
     pan / zoom / node drag internally; we just give it a sized
     box and listen for ResizeObserver to call cy.resize(). */
  .cy {
    width: 100%;
    height: 100%;
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
  }
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
