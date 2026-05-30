<script lang="ts">
  // Standalone graph-force playground. Runs without the chan server:
  // just synthetic data, the same cytoscape styling we use in the
  // real GraphPanel, plus live sliders for the d3-force knobs so we
  // can dial in collide / charge / link tension by eye.
  //
  // Mounted from web/graph-demo.html; reachable in dev at
  // http://localhost:5173/graph-demo.html.

  import { onDestroy, onMount } from "svelte";
  import cytoscape from "cytoscape";
  import type {
    Core, ElementDefinition, EventObject, Layouts,
  } from "cytoscape";
  // @ts-expect-error fcose ships no .d.ts
  import fcose from "cytoscape-fcose";
  // @ts-expect-error cytoscape-d3-force ships no .d.ts
  import d3Force from "cytoscape-d3-force";

  import type {
    GraphView, GraphViewEdge, GraphViewNode,
  } from "../api/types";
  import { defaultSpec, makeFakeGraph, type GraphSpec } from "./fakeData";

  cytoscape.use(fcose);
  cytoscape.use(d3Force);

  // ---- tuning state -----------------------------------------------------

  let spec = $state<GraphSpec>({ ...defaultSpec });
  let force = $state({
    collideFile: 26,
    collideOther: 14,
    collideStrength: 0.95,
    manyBody: -90,
    linkDist: 120,
    attachDist: 70,
    linkStrength: 0.55,
    velocityDecay: 0.55,
    alphaMin: 0.05,
    settleTicks: 180,
    centerStrength: 0.05,
    fixedAfterDragging: true,
  });

  let style = $state({
    edgeOpacity: 0.15,
    brokenOpacity: 0.11,
    edgeWidth: 0.7,
    attachWidth: 1.0,
  });

  let theme = $state<"dark" | "light">("dark");

  let containerEl: HTMLDivElement | undefined = $state();
  let cy: Core | null = null;
  let resizeObs: ResizeObserver | null = null;
  let forceLayout: Layouts | null = null;
  let stats = $state({ nodes: 0, edges: 0 });

  // ---- styling (mirrors GraphPanel) -------------------------------------

  function readThemeColors(host: HTMLElement) {
    const cs = getComputedStyle(host);
    const v = (n: string, fb: string) => cs.getPropertyValue(n).trim() || fb;
    return {
      // Three node kinds in the graph: documents (orange file
      // rectangles), images (purple circles), tags (green
      // hashtag labels). Mentions and dates are intentionally
      // not plotted.
      doc: v("--g-doc", "#ff8a3d"),
      img: v("--g-img", "#b07dff"),
      tag: v("--g-tag", "#6cd07a"),
      text: v("--text", "#f0e6d8"),
      textSec: v("--text-secondary", "#8b7e6e"),
      bg: v("--bg", "#0e0e0e"),
      bgCard: v("--bg-card", "#181614"),
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
      // Documents: small orange rounded rectangle, label below.
      { selector: 'node[kind = "doc"]',
        style: {
          shape: "round-rectangle",
          "background-color": c.doc,
          width: 14,
          height: 18,
        } },
      // Images: small purple circle, label below.
      { selector: 'node[kind = "img"]',
        style: {
          shape: "ellipse",
          "background-color": c.img,
          width: 10,
          height: 10,
        } },
      // Tags: green hashtag text only - no fill, label centered
      // on the node so the "#name" string IS the visual.
      { selector: 'node[kind = "tag"]',
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
          "text-outline-width": 2,
        } },
      { selector: "node[?missing]",
        style: { "background-color": c.bgCard, "border-style": "dashed",
          "border-color": c.textSec, opacity: 0.6 } },
      { selector: "node.hover",
        style: { "border-color": c.text, "border-width": 1.5 } },
      { selector: "node:selected",
        style: { "border-color": c.doc, "border-width": 3,
          "overlay-color": c.doc, "overlay-opacity": 0.15,
          "overlay-padding": 2 } },
      { selector: "edge",
        style: { "curve-style": "bezier", "line-cap": "round",
          opacity: style.edgeOpacity } },
      { selector: 'edge[kind = "link"]',
        style: { "line-color": c.text, width: style.edgeWidth } },
      { selector: 'edge[kind = "tag"]',
        style: { "line-color": c.tag, width: style.attachWidth } },
      { selector: "edge[?broken]",
        style: { "line-style": "dashed", opacity: style.brokenOpacity } },
    ];
  }

  // ---- graph build ------------------------------------------------------

  /// Documents vs images: the GraphView types only know "file",
  /// so split by extension here. Anything not a recognised image
  /// extension counts as a document.
  function classifyFile(path: string): "doc" | "img" {
    return /\.(png|jpe?g|gif|webp|svg|avif|bmp)$/i.test(path) ? "img" : "doc";
  }

  function buildElements(g: GraphView): ElementDefinition[] {
    const keepKinds = new Set(["file", "tag"]);
    const keepNodeIds = new Set<string>();
    for (const n of g.nodes) {
      if (keepKinds.has(n.kind)) keepNodeIds.add(n.id);
    }
    const els: ElementDefinition[] = [];
    for (const n of g.nodes) {
      if (!keepNodeIds.has(n.id)) continue;
      const data: Record<string, unknown> = {
        id: n.id, label: n.label,
      };
      if (n.kind === "file") {
        data.kind = classifyFile(n.path);
        data.path = n.path;
        if (n.missing) data.missing = true;
      } else {
        data.kind = "tag";
      }
      els.push({ group: "nodes", data });
    }
    const seen = new Map<string, number>();
    for (const e of g.edges) {
      if (e.kind !== "link" && e.kind !== "tag") continue;
      if (!keepNodeIds.has(e.source) || !keepNodeIds.has(e.target)) continue;
      const base = `${e.source}|${e.target}|${e.kind}`;
      const n = (seen.get(base) ?? 0) + 1;
      seen.set(base, n);
      const id = n === 1 ? base : `${base}#${n}`;
      const data: Record<string, unknown> = {
        id, source: e.source, target: e.target, kind: e.kind,
      };
      if (e.broken) data.broken = true;
      els.push({ group: "edges", data });
    }
    return els;
  }

  function fcoseOptions() {
    return {
      name: "fcose",
      quality: "default",
      randomize: true,
      animate: false,
      fit: false,
      padding: 30,
      nodeSeparation: 140,
      idealEdgeLength: (e: cytoscape.EdgeSingular) =>
        e.data("kind") === "link" ? force.linkDist : force.attachDist,
      edgeElasticity: 0.45,
      nestingFactor: 0.1,
      numIter: 2500,
      gravity: 0.25,
      packComponents: true,
    } as cytoscape.LayoutOptions;
  }

  // cytoscape-d3-force merges each cy element's `data()` into the
  // d3-force node/edge object (see node_modules/cytoscape-d3-force,
  // `assign(getScratch(n), n.data())`). Accessors therefore receive
  // a plain object whose fields are the data keys directly - no
  // `.data()` method, no `.kind` getter. Read kind/id off the bare
  // object.
  type D3Node = { kind: string; id: string; index?: number };
  type D3Edge = { kind: string; source: D3Node; target: D3Node };

  function d3ForceOptions() {
    return {
      name: "d3-force",
      animate: true,
      fit: false,
      randomize: false,
      // infinite=true keeps the simulation + cytoscape grab/free
      // handlers alive after the initial settle. Without it, the
      // layout calls end() once progress hits 1 and tears its event
      // handlers down - drags after that point do nothing.
      infinite: true,
      ungrabifyWhileSimulating: false,
      fixedAfterDragging: force.fixedAfterDragging,
      alpha: 1,
      alphaMin: force.alphaMin,
      alphaDecay: 1 - Math.pow(force.alphaMin, 1 / Math.max(20, force.settleTicks)),
      alphaTarget: 0,
      velocityDecay: force.velocityDecay,
      collideRadius: (n: D3Node) => {
        if (n.kind === "doc") return force.collideFile;
        if (n.kind === "tag") return force.collideOther + 4;
        return force.collideOther;
      },
      collideStrength: force.collideStrength,
      collideIterations: 2,
      manyBodyStrength: force.manyBody,
      // d3-force resolves source/target to the node objects after
      // linkId; before that they're string ids. Defensive: handle
      // both shapes.
      linkDistance: (e: D3Edge) =>
        e.kind === "link" ? force.linkDist : force.attachDist,
      linkStrength: force.linkStrength,
      linkId: (n: D3Node) => n.id,
      xStrength: force.centerStrength,
      yStrength: force.centerStrength,
    } as cytoscape.LayoutOptions;
  }

  function rebuild(): void {
    if (!containerEl) return;
    forceLayout?.stop();
    forceLayout = null;
    cy?.destroy();

    const g = makeFakeGraph(spec);
    stats = { nodes: g.nodes.length, edges: g.edges.length };
    const elements = buildElements(g);
    cy = cytoscape({
      container: containerEl,
      elements,
      style: buildStylesheet(containerEl),
      minZoom: 0.15,
      maxZoom: 4,
      boxSelectionEnabled: false,
      selectionType: "single",
    });
    cy.on("mouseover", "node", (ev: EventObject) => ev.target.addClass("hover"));
    cy.on("mouseout", "node", (ev: EventObject) => ev.target.removeClass("hover"));

    const layout = cy.layout(fcoseOptions());
    layout.one("layoutstop", () => {
      requestAnimationFrame(() => {
        if (!cy) return;
        cy.resize();
        const vis = cy.elements();
        if (vis.nonempty()) cy.fit(vis, 30);
        forceLayout = cy.layout(d3ForceOptions());
        forceLayout.run();

        // cytoscape-d3-force's built-in grab/free handlers bump
        // alphaTarget to ~0.33 and never restore it to 0 - so
        // after a single drag the simulation runs forever. Add
        // our own free/unlock handler (registered after the
        // built-in's, so it executes second) that drops the
        // target back, letting alpha decay to alphaMin and the
        // graph actually settle.
        cy.nodes().on("free unlock", () => {
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          const sim = (forceLayout as any)?.simulation;
          if (sim) sim.alphaTarget(0);
        });
      });
    });
    layout.run();
  }

  function shake(): void {
    if (!cy) return;
    forceLayout?.stop();
    forceLayout = cy.layout(d3ForceOptions());
    forceLayout.run();
  }

  function relayout(): void {
    rebuild();
  }

  function regen(): void {
    spec = { ...spec, seed: spec.seed + 1 };
    rebuild();
  }

  onMount(() => {
    if (!containerEl) return;
    resizeObs = new ResizeObserver(() => cy?.resize());
    resizeObs.observe(containerEl);
    rebuild();
  });

  // Style sliders: re-apply the stylesheet in place. Cytoscape
  // diff-applies, positions are kept, simulation isn't disturbed.
  $effect(() => {
    void style.edgeOpacity;
    void style.brokenOpacity;
    void style.edgeWidth;
    void style.attachWidth;
    void theme;
    if (!cy || !containerEl) return;
    cy.style(buildStylesheet(containerEl));
  });

  // Theme toggle: flip the data-theme attribute on <html>; the
  // CSS vars below resolve from it, and the stylesheet effect
  // above re-reads them via getComputedStyle.
  $effect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  });

  onDestroy(() => {
    resizeObs?.disconnect();
    forceLayout?.stop();
    cy?.destroy();
    cy = null;
  });
</script>

<div class="page">
  <aside class="controls">
    <h3>graph spec</h3>
    <label>seed
      <input type="number" bind:value={spec.seed} />
    </label>
    <label>documents <span class="v">{spec.documents}</span>
      <input type="range" min="2" max="120" step="1" bind:value={spec.documents} />
    </label>
    <label>images <span class="v">{spec.images}</span>
      <input type="range" min="0" max="40" step="1" bind:value={spec.images} />
    </label>
    <label>tags <span class="v">{spec.tags}</span>
      <input type="range" min="0" max="40" step="1" bind:value={spec.tags} />
    </label>
    <label>doc-doc link p <span class="v">{spec.linkDensity.toFixed(2)}</span>
      <input type="range" min="0" max="0.4" step="0.01" bind:value={spec.linkDensity} />
    </label>
    <label>doc-image p <span class="v">{spec.imageRefDensity.toFixed(2)}</span>
      <input type="range" min="0" max="0.4" step="0.01" bind:value={spec.imageRefDensity} />
    </label>
    <label>tags / doc <span class="v">{spec.tagsPerDoc.toFixed(1)}</span>
      <input type="range" min="0" max="6" step="0.1" bind:value={spec.tagsPerDoc} />
    </label>
    <button onclick={regen}>regen graph</button>
    <button onclick={relayout}>relayout (fcose + d3)</button>

    <h3>force tuning</h3>
    <label>collide r (file) <span class="v">{force.collideFile}</span>
      <input type="range" min="6" max="60" step="1" bind:value={force.collideFile} />
    </label>
    <label>collide r (other) <span class="v">{force.collideOther}</span>
      <input type="range" min="4" max="40" step="1" bind:value={force.collideOther} />
    </label>
    <label>collide strength <span class="v">{force.collideStrength.toFixed(2)}</span>
      <input type="range" min="0" max="1" step="0.05" bind:value={force.collideStrength} />
    </label>
    <label>manyBody <span class="v">{force.manyBody}</span>
      <input type="range" min="-200" max="0" step="1" bind:value={force.manyBody} />
    </label>
    <label>link dist (link) <span class="v">{force.linkDist}</span>
      <input type="range" min="20" max="200" step="1" bind:value={force.linkDist} />
    </label>
    <label>link dist (attach) <span class="v">{force.attachDist}</span>
      <input type="range" min="10" max="200" step="1" bind:value={force.attachDist} />
    </label>
    <label>link strength <span class="v">{force.linkStrength.toFixed(2)}</span>
      <input type="range" min="0" max="2" step="0.05" bind:value={force.linkStrength} />
    </label>
    <label>velocityDecay <span class="v">{force.velocityDecay.toFixed(2)}</span>
      <input type="range" min="0.1" max="0.95" step="0.05" bind:value={force.velocityDecay} />
    </label>
    <label>alphaMin <span class="v">{force.alphaMin.toFixed(3)}</span>
      <input type="range" min="0.001" max="0.2" step="0.001" bind:value={force.alphaMin} />
    </label>
    <label>settle ticks <span class="v">{force.settleTicks}</span>
      <input type="range" min="20" max="600" step="10" bind:value={force.settleTicks} />
    </label>
    <label>center pull <span class="v">{force.centerStrength.toFixed(2)}</span>
      <input type="range" min="0" max="0.3" step="0.01" bind:value={force.centerStrength} />
    </label>
    <label class="row">
      <input type="checkbox" bind:checked={force.fixedAfterDragging} />
      pin nodes after drag
    </label>
    <button onclick={shake}>shake (re-heat)</button>

    <h3>theme</h3>
    <div class="theme-row">
      <button
        class:on={theme === "dark"}
        onclick={() => (theme = "dark")}
      >dark</button>
      <button
        class:on={theme === "light"}
        onclick={() => (theme = "light")}
      >light</button>
    </div>

    <h3>edge style</h3>
    <label>edge opacity <span class="v">{style.edgeOpacity.toFixed(2)}</span>
      <input type="range" min="0.05" max="1" step="0.01" bind:value={style.edgeOpacity} />
    </label>
    <label>broken-edge opacity <span class="v">{style.brokenOpacity.toFixed(2)}</span>
      <input type="range" min="0.05" max="1" step="0.01" bind:value={style.brokenOpacity} />
    </label>
    <label>link width <span class="v">{style.edgeWidth.toFixed(1)}</span>
      <input type="range" min="0.5" max="4" step="0.1" bind:value={style.edgeWidth} />
    </label>
    <label>attach width <span class="v">{style.attachWidth.toFixed(1)}</span>
      <input type="range" min="0.5" max="4" step="0.1" bind:value={style.attachWidth} />
    </label>

    <div class="legend">
      <div><span class="dot doc"></span> doc</div>
      <div><span class="dot img"></span> image</div>
      <div><span class="dot tag"></span> tag</div>
    </div>
  </aside>

  <main class="canvas-wrap">
    <div bind:this={containerEl} class="cy"></div>
    <div class="status">
      {stats.nodes} nodes - {stats.edges} edges -       drag a node to nudge neighbours - scroll to zoom
    </div>
  </main>
</div>

<style>
  :global(html), :global(body) {
    margin: 0;
    padding: 0;
    height: 100%;
    background: var(--bg);
    color: var(--text);
    font-family: ui-sans-serif, system-ui, sans-serif;
  }
  /* Dark theme (chan brand): near-black warm background, warm
     orange primary, supporting hues with strong saturation so the
     graph nodes pop. */
  :global(html[data-theme="dark"]) {
    --bg: #0e0e0e;
    --bg-card: #181614;
    --bg-elev: #1f1c19;
    --border: #2c2724;
    --text: #f0e6d8;
    --text-secondary: #8b7e6e;
    --accent: #ff7a3b;
    --g-doc: #ff8a3d;
    --g-img: #b07dff;
    --g-tag: #6cd07a;
  }
  /* Light theme: cream paper background, deeper brand orange so it
     stays legible at lower brightness, slightly desaturated
     supporting hues so they don't glare against the cream. */
  /* "Light mode" of the graph isn't actually light - a graph
     canvas wants a dark backdrop for its colored nodes to pop.
     This variant just shifts to a neutral medium-dark grey
     (vs the warm near-black of the dark theme), keeping the
     same saturated palette. */
  :global(html[data-theme="light"]) {
    --bg: #2c2c2c;
    --bg-card: #232323;
    --bg-elev: #383838;
    --border: #4a4a4a;
    --text: #ececec;
    --text-secondary: #a0a0a0;
    --accent: #ff7a3b;
    --g-doc: #ff8a3d;
    --g-img: #b07dff;
    --g-tag: #6cd07a;
  }
  .page {
    display: flex;
    height: 100vh;
  }
  .controls {
    width: 240px;
    flex-shrink: 0;
    background: var(--bg-card);
    border-right: 1px solid var(--border);
    padding: 10px;
    overflow-y: auto;
    font-size: 12px;
  }
  .controls h3 {
    margin: 0.5rem 0 0.4rem 0;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
  }
  .controls h3:first-child { margin-top: 0; }
  .controls label {
    display: block;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }
  .controls label.row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .controls .v {
    color: var(--text);
    font-variant-numeric: tabular-nums;
    margin-left: 4px;
  }
  .controls input[type="range"] {
    width: 100%;
    accent-color: var(--accent);
  }
  .controls input[type="number"] {
    width: 100%;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 2px 4px;
    font: inherit;
  }
  .controls button {
    width: 100%;
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 4px;
    cursor: pointer;
    margin-bottom: 6px;
  }
  .controls button:hover { border-color: var(--accent); }
  .controls button.on {
    border-color: var(--accent);
    color: var(--accent);
  }
  .theme-row {
    display: flex;
    gap: 6px;
    margin-bottom: 6px;
  }
  .theme-row button { margin-bottom: 0; }
  .legend {
    margin-top: 12px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
    color: var(--text-secondary);
  }
  .legend .dot {
    display: inline-block;
    width: 8px; height: 8px;
    border-radius: 50%;
    margin-right: 4px;
    vertical-align: middle;
  }
  .legend .dot.doc { background: var(--g-doc); border-radius: 2px; }
  .legend .dot.img { background: var(--g-img); }
  .legend .dot.tag { background: var(--g-tag); }
  .canvas-wrap {
    flex: 1;
    position: relative;
    min-width: 0;
  }
  .cy {
    position: absolute;
    inset: 0;
  }
  .status {
    position: absolute;
    bottom: 8px;
    left: 12px;
    right: 12px;
    color: var(--text-secondary);
    font-size: 12px;
    pointer-events: none;
    text-shadow: 0 0 4px var(--bg);
  }
</style>
