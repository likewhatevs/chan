<script lang="ts">
  // Standalone "raw d3-force + canvas" demo, loading the live
  // /api/graph payload from the running chan server through vite's
  // proxy (web/vite.config.ts → 127.0.0.1:8787). Renders the same
  // node + edge set the production GraphPanel does, but via the
  // Observable d3-force pattern: canvas 2d context, one fill +
  // stroke per element per frame, no cytoscape between us and the
  // pixels.
  //
  // The point is to make the smoothness gap concrete. Drag a node
  // and feel how the simulation reacts; compare to the cytoscape
  // GraphPanel against the same drive. The FPS readout in the
  // corner is wall-clock.
  //
  // Mounted from web/d3-compare.html. Reachable at
  // http://localhost:5173/d3-compare.html when vite dev is up.

  import { onDestroy, onMount } from "svelte";
  import {
    forceCenter,
    forceCollide,
    forceLink,
    forceManyBody,
    forceSimulation,
    forceX,
    forceY,
    type Simulation,
  } from "d3-force";

  // ---- types match /api/graph ------------------------------------------

  type ApiNode =
    | { kind: "file"; id: string; label: string; path: string;
        node_kind?: "contact"; missing?: boolean }
    | { kind: "tag"; id: string; label: string }
    | { kind: "mention"; id: string; label: string }
    | { kind: "date"; id: string; label: string };
  type ApiEdge = {
    kind: "link" | "tag" | "mention" | "date";
    source: string;
    target: string;
    broken?: boolean;
  };
  type ApiGraph = { nodes: ApiNode[]; edges: ApiEdge[] };

  // Working types after we've split files into doc / img / contact
  // and resolved string ids into node refs (d3 mutates the edge
  // source/target fields with the node objects).
  type Kind = "doc" | "img" | "contact" | "tag" | "mention";
  type Node = {
    id: string;
    label: string;
    kind: Kind;
    missing?: boolean;
    radius: number;
    x?: number; y?: number; vx?: number; vy?: number; fx?: number | null; fy?: number | null;
  };
  type Edge = {
    source: string | Node;
    target: string | Node;
    kind: "link" | "tag" | "mention" | "date";
    broken?: boolean;
  };

  // ---- palette (chan theme) --------------------------------------------

  const COLORS: Record<Kind, string> = {
    doc: "#ff8a3d",
    img: "#b07dff",
    contact: "#e3b341",
    tag: "#6cd07a",
    mention: "#e3b341",
  };

  const EDGE_COLOR = {
    link: "#ebebf0",   // chan --text in dark mode; faint over canvas
    tag: "#6cd07a",
    mention: "#e3b341",
    date: "#98989d",
  } as const;

  // ---- state ------------------------------------------------------------

  let canvas: HTMLCanvasElement | undefined = $state();
  let stats = $state({ nodes: 0, edges: 0, fps: 0 });
  let error: string | null = $state(null);
  let loading = $state(true);
  let nodes: Node[] = [];
  let edges: Edge[] = [];
  let sim: Simulation<Node, Edge> | null = null;
  let dragId: string | null = null;
  let transform = { x: 0, y: 0, k: 1 };
  let rafId: number | null = null;
  let lastFpsT = performance.now();
  let frames = 0;

  // ---- knobs ------------------------------------------------------------

  let tune = $state({
    chargeStrength: -120,
    linkDistance: 60,
    linkStrength: 0.6,
    collideExtra: 2,        // extra px on top of node radius for collide
    velocityDecay: 0.55,
    centerStrength: 0.04,
    nodeRadius: 6,
    docRadius: 8,
    edgeOpacity: 0.18,
    edgeWidth: 1,
    strokeOnNode: true,
    strokeColor: "#1c1c1e",
  });

  // ---- classify + fetch -------------------------------------------------

  function classifyFile(path: string, nodeKind?: "contact"): "doc" | "img" | "contact" {
    if (/\.(png|jpe?g|gif|webp|svg|avif|bmp)$/i.test(path)) return "img";
    if (nodeKind === "contact") return "contact";
    return "doc";
  }

  function buildWorkingSet(g: ApiGraph): { nodes: Node[]; edges: Edge[] } {
    // Drop date nodes/edges; the production graph does too.
    const keep = new Set<string>();
    const ns: Node[] = [];
    for (const n of g.nodes) {
      if (n.kind === "date") continue;
      keep.add(n.id);
      let k: Kind;
      let missing: boolean | undefined;
      if (n.kind === "file") {
        k = classifyFile(n.path, n.node_kind);
        missing = n.missing;
      } else if (n.kind === "tag") k = "tag";
      else k = "mention";
      const radius = k === "doc" ? tune.docRadius : tune.nodeRadius;
      ns.push({ id: n.id, label: n.label, kind: k, missing, radius });
    }
    const es: Edge[] = [];
    for (const e of g.edges) {
      if (e.kind === "date") continue;
      if (!keep.has(e.source) || !keep.has(e.target)) continue;
      es.push({ source: e.source, target: e.target, kind: e.kind, broken: e.broken });
    }
    return { nodes: ns, edges: es };
  }

  async function fetchGraph(): Promise<void> {
    loading = true;
    error = null;
    try {
      // /api goes through vite's proxy to the chan server.
      const res = await fetch("/api/graph");
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data: ApiGraph = await res.json();
      const ws = buildWorkingSet(data);
      nodes = ws.nodes;
      edges = ws.edges;
      stats.nodes = nodes.length;
      stats.edges = edges.length;
      restart();
    } catch (e) {
      error = (e as Error).message;
    } finally {
      loading = false;
    }
  }

  // ---- simulation -------------------------------------------------------

  function restart(): void {
    if (!canvas) return;
    sim?.stop();
    // Refresh per-node radii so tune.nodeRadius / tune.docRadius
    // sliders take effect without a full rebuild.
    for (const n of nodes) {
      n.radius = n.kind === "doc" ? tune.docRadius : tune.nodeRadius;
      // Drop any prior fix so a fresh simulation can lay them out.
      n.fx = null;
      n.fy = null;
    }
    sim = forceSimulation<Node>(nodes)
      .force(
        "link",
        forceLink<Node, Edge>(edges)
          .id((d) => d.id)
          .distance(tune.linkDistance)
          .strength(tune.linkStrength),
      )
      .force("charge", forceManyBody<Node>().strength(tune.chargeStrength))
      .force(
        "collide",
        forceCollide<Node>().radius((d) => d.radius + tune.collideExtra),
      )
      .force("center", forceCenter(canvas.width / 2, canvas.height / 2).strength(0.5))
      .force("x", forceX(canvas.width / 2).strength(tune.centerStrength))
      .force("y", forceY(canvas.height / 2).strength(tune.centerStrength))
      .velocityDecay(tune.velocityDecay)
      .alpha(1)
      .alphaTarget(0)
      .on("tick", () => {
        // No DOM work in the tick; the rAF loop is what paints.
      });
  }

  function paint(): void {
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const w = canvas.width, h = canvas.height;
    ctx.clearRect(0, 0, w, h);
    ctx.save();
    ctx.translate(transform.x, transform.y);
    ctx.scale(transform.k, transform.k);

    // Edges first so nodes paint on top.
    ctx.lineWidth = tune.edgeWidth;
    ctx.globalAlpha = tune.edgeOpacity;
    for (const e of edges) {
      const s = e.source as Node;
      const t = e.target as Node;
      if (s.x == null || t.x == null) continue;
      ctx.strokeStyle = EDGE_COLOR[e.kind] ?? EDGE_COLOR.link;
      ctx.beginPath();
      ctx.moveTo(s.x!, s.y!);
      ctx.lineTo(t.x!, t.y!);
      ctx.stroke();
    }
    ctx.globalAlpha = 1;

    // Nodes: filled disc, optional stroke ring.
    for (const n of nodes) {
      if (n.x == null) continue;
      ctx.beginPath();
      ctx.arc(n.x!, n.y!, n.radius, 0, Math.PI * 2);
      ctx.fillStyle = n.missing ? "#252525" : COLORS[n.kind];
      ctx.fill();
      if (tune.strokeOnNode) {
        ctx.strokeStyle = tune.strokeColor;
        ctx.lineWidth = 1.5;
        ctx.stroke();
      }
    }

    ctx.restore();

    // FPS tally (wall-clock, sliding 1s window).
    frames++;
    const now = performance.now();
    if (now - lastFpsT >= 1000) {
      stats.fps = Math.round((frames * 1000) / (now - lastFpsT));
      frames = 0;
      lastFpsT = now;
    }
  }

  function loop(): void {
    paint();
    rafId = requestAnimationFrame(loop);
  }

  // ---- canvas sizing + interaction --------------------------------------

  function resize(): void {
    if (!canvas) return;
    const r = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(r.width * dpr);
    canvas.height = Math.round(r.height * dpr);
    const ctx = canvas.getContext("2d");
    if (ctx) ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    // forceCenter targets a fixed centre — re-seat it on resize so
    // the cluster doesn't drift toward a stale midpoint.
    if (sim) {
      const cx = r.width / 2, cy = r.height / 2;
      sim.force("center", forceCenter(cx, cy).strength(0.5));
      sim.force("x", forceX(cx).strength(tune.centerStrength));
      sim.force("y", forceY(cy).strength(tune.centerStrength));
      sim.alpha(0.2).restart();
    }
  }

  function pickNode(x: number, y: number): Node | null {
    // Convert screen → world coords.
    const wx = (x - transform.x) / transform.k;
    const wy = (y - transform.y) / transform.k;
    let best: Node | null = null;
    let bestD2 = Infinity;
    for (const n of nodes) {
      if (n.x == null) continue;
      const dx = n.x! - wx, dy = n.y! - wy;
      const r = n.radius + 4;
      const d2 = dx * dx + dy * dy;
      if (d2 <= r * r && d2 < bestD2) {
        best = n; bestD2 = d2;
      }
    }
    return best;
  }

  function onMouseDown(e: MouseEvent): void {
    if (!canvas) return;
    const r = canvas.getBoundingClientRect();
    const n = pickNode(e.clientX - r.left, e.clientY - r.top);
    if (!n) {
      // Pan: hold the mouse and drag empty space.
      panStart = { x: e.clientX, y: e.clientY, tx: transform.x, ty: transform.y };
      return;
    }
    dragId = n.id;
    n.fx = n.x;
    n.fy = n.y;
    sim?.alphaTarget(0.3).restart();
  }
  let panStart: { x: number; y: number; tx: number; ty: number } | null = null;

  function onMouseMove(e: MouseEvent): void {
    if (!canvas) return;
    const r = canvas.getBoundingClientRect();
    if (dragId) {
      const n = nodes.find((x) => x.id === dragId);
      if (!n) return;
      n.fx = (e.clientX - r.left - transform.x) / transform.k;
      n.fy = (e.clientY - r.top - transform.y) / transform.k;
      return;
    }
    if (panStart) {
      transform.x = panStart.tx + (e.clientX - panStart.x);
      transform.y = panStart.ty + (e.clientY - panStart.y);
    }
  }

  function onMouseUp(): void {
    if (dragId) {
      const n = nodes.find((x) => x.id === dragId);
      if (n) {
        // Release back to the simulation; matches the Observable
        // example. To keep dropped nodes pinned, comment these out.
        n.fx = null; n.fy = null;
      }
      sim?.alphaTarget(0);
      dragId = null;
    }
    panStart = null;
  }

  function onWheel(e: WheelEvent): void {
    if (!canvas) return;
    e.preventDefault();
    const r = canvas.getBoundingClientRect();
    const cx = e.clientX - r.left, cy = e.clientY - r.top;
    const factor = e.deltaY > 0 ? 0.9 : 1.1;
    const k2 = Math.min(8, Math.max(0.2, transform.k * factor));
    // Zoom toward the cursor: world point under the cursor must stay
    // anchored. wx = (cx - tx) / k → solve for new (tx, ty) so wx
    // doesn't move.
    transform.x = cx - ((cx - transform.x) * k2) / transform.k;
    transform.y = cy - ((cy - transform.y) * k2) / transform.k;
    transform.k = k2;
  }

  onMount(() => {
    if (!canvas) return;
    resize();
    new ResizeObserver(() => resize()).observe(canvas);
    void fetchGraph();
    rafId = requestAnimationFrame(loop);
  });

  // Hot-patch the simulation forces when sliders move so the user
  // sees the effect without a full restart (which would scramble
  // positions).
  $effect(() => {
    void tune.chargeStrength;
    void tune.linkDistance;
    void tune.linkStrength;
    void tune.collideExtra;
    void tune.velocityDecay;
    void tune.centerStrength;
    if (!sim) return;
    sim
      .force("charge", forceManyBody<Node>().strength(tune.chargeStrength))
      .force(
        "link",
        forceLink<Node, Edge>(edges)
          .id((d) => d.id)
          .distance(tune.linkDistance)
          .strength(tune.linkStrength),
      )
      .force(
        "collide",
        forceCollide<Node>().radius((d) => d.radius + tune.collideExtra),
      );
    if (canvas) {
      const r = canvas.getBoundingClientRect();
      sim
        .force("x", forceX(r.width / 2).strength(tune.centerStrength))
        .force("y", forceY(r.height / 2).strength(tune.centerStrength));
    }
    sim.velocityDecay(tune.velocityDecay);
    sim.alpha(0.2).restart();
  });

  // Radius-only sliders: rewrite the radius field on every node so
  // the next tick uses the new value.
  $effect(() => {
    void tune.nodeRadius;
    void tune.docRadius;
    for (const n of nodes) {
      n.radius = n.kind === "doc" ? tune.docRadius : tune.nodeRadius;
    }
  });

  onDestroy(() => {
    if (rafId !== null) cancelAnimationFrame(rafId);
    sim?.stop();
  });
</script>

<div class="page">
  <aside class="controls">
    <h3>data</h3>
    <p class="hint">
      live from <code>/api/graph</code> via vite proxy →
      <code>127.0.0.1:8787</code>.
    </p>
    <p class="stat">
      {#if loading}loading…{:else if error}<span class="err">{error}</span>
      {:else}{stats.nodes} nodes · {stats.edges} edges{/if}
    </p>
    <button onclick={fetchGraph}>refetch</button>
    <button onclick={restart}>restart sim</button>

    <h3>forces</h3>
    <label>charge <span class="v">{tune.chargeStrength}</span>
      <input type="range" min="-400" max="-10" step="5" bind:value={tune.chargeStrength} />
    </label>
    <label>link distance <span class="v">{tune.linkDistance}</span>
      <input type="range" min="10" max="200" step="2" bind:value={tune.linkDistance} />
    </label>
    <label>link strength <span class="v">{tune.linkStrength.toFixed(2)}</span>
      <input type="range" min="0" max="2" step="0.05" bind:value={tune.linkStrength} />
    </label>
    <label>collide pad <span class="v">{tune.collideExtra}</span>
      <input type="range" min="0" max="20" step="1" bind:value={tune.collideExtra} />
    </label>
    <label>velocityDecay <span class="v">{tune.velocityDecay.toFixed(2)}</span>
      <input type="range" min="0.1" max="0.9" step="0.05" bind:value={tune.velocityDecay} />
    </label>
    <label>center pull <span class="v">{tune.centerStrength.toFixed(2)}</span>
      <input type="range" min="0" max="0.3" step="0.01" bind:value={tune.centerStrength} />
    </label>

    <h3>nodes</h3>
    <label>radius (non-doc) <span class="v">{tune.nodeRadius}</span>
      <input type="range" min="3" max="20" step="1" bind:value={tune.nodeRadius} />
    </label>
    <label>radius (doc) <span class="v">{tune.docRadius}</span>
      <input type="range" min="3" max="20" step="1" bind:value={tune.docRadius} />
    </label>
    <label class="row">
      <input type="checkbox" bind:checked={tune.strokeOnNode} />
      stroke ring
    </label>

    <h3>edges</h3>
    <label>opacity <span class="v">{tune.edgeOpacity.toFixed(2)}</span>
      <input type="range" min="0.05" max="1" step="0.01" bind:value={tune.edgeOpacity} />
    </label>
    <label>width <span class="v">{tune.edgeWidth.toFixed(1)}</span>
      <input type="range" min="0.5" max="3" step="0.1" bind:value={tune.edgeWidth} />
    </label>
  </aside>

  <main class="canvas-wrap">
    <canvas
      bind:this={canvas}
      onmousedown={onMouseDown}
      onmousemove={onMouseMove}
      onmouseup={onMouseUp}
      onmouseleave={onMouseUp}
      onwheel={onWheel}
    ></canvas>
    <div class="hud">
      {stats.fps} fps · drag a node to nudge · drag empty space to pan ·
      scroll to zoom
    </div>
  </main>
</div>

<style>
  :global(html), :global(body) {
    margin: 0;
    padding: 0;
    height: 100%;
    background: #0e0e0e;
    color: #f0e6d8;
    font-family: ui-sans-serif, system-ui, sans-serif;
  }
  .page {
    display: flex;
    height: 100vh;
    width: 100vw;
  }
  .controls {
    width: 270px;
    flex: none;
    padding: 14px;
    background: #181614;
    border-right: 1px solid #2c2724;
    overflow: auto;
    font-size: 12.5px;
  }
  .controls h3 {
    margin: 14px 0 6px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #8b7e6e;
  }
  .controls h3:first-of-type { margin-top: 0; }
  .controls label { display: block; margin: 4px 0 8px; }
  .controls .v {
    float: right;
    font-variant-numeric: tabular-nums;
    color: #8b7e6e;
  }
  .controls input[type="range"] { width: 100%; }
  .controls button {
    background: #1f1c19;
    color: #f0e6d8;
    border: 1px solid #2c2724;
    border-radius: 4px;
    padding: 4px 10px;
    font: inherit;
    cursor: pointer;
    margin: 2px 4px 2px 0;
  }
  .controls .row { display: flex; align-items: center; gap: 6px; }
  .controls .hint, .controls .stat {
    margin: 4px 0 6px;
    color: #8b7e6e;
    font-size: 11.5px;
    line-height: 1.4;
  }
  .controls .err { color: #f85149; }
  .controls code {
    background: #2a2a2c;
    padding: 1px 4px;
    border-radius: 3px;
    font-size: 11px;
  }
  .canvas-wrap {
    flex: 1;
    position: relative;
    overflow: hidden;
  }
  canvas {
    display: block;
    width: 100%;
    height: 100%;
    cursor: grab;
  }
  canvas:active { cursor: grabbing; }
  .hud {
    position: absolute;
    left: 12px;
    bottom: 10px;
    color: #8b7e6e;
    font-size: 11px;
    pointer-events: none;
  }
</style>
