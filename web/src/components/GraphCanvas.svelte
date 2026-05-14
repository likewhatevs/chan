<script lang="ts">
  // Canvas + d3-force renderer for the chan graph. Replaces the
  // cytoscape view that used to live inside GraphPanel.svelte.
  //
  // Why: cytoscape's per-frame pipeline (style-selector resolution,
  // SVG-icon bitmap compositing, scenegraph hit-testing) sits above
  // the d3-force simulation it wraps, and chews enough budget that
  // a 90-node graph never matches the Observable d3-force example's
  // smoothness. Rendering straight to a canvas — one fill + ring +
  // icon blit per node per tick — closes that gap.
  //
  // What GraphCanvas owns:
  //   - HTML5 canvas + animation loop (requestAnimationFrame)
  //   - d3-force simulation (charge / link / collide / x / y) with
  //     "infinite: true"-style alphaTarget bookkeeping so dragging
  //     a node re-heats the cluster and releasing settles it
  //   - hover / click / drag / pan / zoom interaction
  //   - focal pinning during the initial layout
  //   - icon pre-rasterisation per kind (chan's existing
  //     lucide-style glyphs decoded once to HTMLImageElement and
  //     drawn via ctx.drawImage)
  //   - label visibility: hidden by default; opt-in for the
  //     selected node and every first-degree neighbour
  //
  // What stays in GraphPanel.svelte:
  //   - scope picker + BFS to compute visibleNodeIds / visibleEdges
  //   - filter chips (link / tag / mention / img)
  //   - depth slider, inspector, scope history, hamburger menu

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
  import type { GraphViewEdge, GraphViewNode } from "../api/types";

  type RenderedEdgeKind = "link" | "tag" | "mention" | "group";
  type RenderedEdge = GraphViewEdge & { kind: RenderedEdgeKind };
  type RenderedNode = Extract<
    GraphViewNode,
    { kind: "file" | "tag" | "mention" }
  >;

  // ---- props ------------------------------------------------------------

  type Props = {
    open: boolean;
    nodes: RenderedNode[];
    edges: RenderedEdge[];
    visibleNodeIds: Set<string>;
    visibleEdges: RenderedEdge[];
    focalIds: string[];
    selectedId: string | null;
    onSelect: (id: string | null) => void;
    onContextMenu?: (e: MouseEvent) => void;
  };
  let {
    open,
    nodes,
    edges,
    visibleNodeIds,
    visibleEdges,
    focalIds,
    selectedId,
    onSelect,
    onContextMenu,
  }: Props = $props();

  // ---- types: d3-shaped working copies ---------------------------------

  type DKind = "doc" | "img" | "contact" | "tag" | "mention";
  type DNode = {
    id: string;
    label: string;
    kind: DKind;
    missing: boolean;
    isFocal: boolean;
    radius: number;
    // d3-force mutates these in-place; declared optional so the
    // first tick can initialise them.
    index?: number;
    x?: number; y?: number; vx?: number; vy?: number;
    fx?: number | null; fy?: number | null;
  };
  type DEdge = {
    source: string | DNode;
    target: string | DNode;
    kind: RenderedEdgeKind;
    broken?: boolean;
  };

  // ---- constants --------------------------------------------------------

  /// Node rendering radii. Doc nodes are slightly larger so the file
  /// chrome reads as the load-bearing kind at a glance; tag / mention
  /// / image / contact sit a notch smaller. Backlink mapData scaling
  /// is applied on top in `renderRadius`.
  const RADIUS_BASE = 5;
  const RADIUS_DOC = 7;
  const RADIUS_HUB_SCALE = 1.4;

  /// Icon glyph occupies this fraction of the rendered diameter.
  /// Matches the cytoscape value so the visual mass of icons doesn't
  /// shift in the cutover.
  const ICON_FRACTION = 0.6;

  /// d3-force tuning. Values mirror the d3-compare demo defaults so
  /// the cutover lands in the same regime the user has already been
  /// looking at; tweak here, not in the per-call layout configs.
  const FORCE = {
    chargeStrength: -120,
    linkDistance: 70,
    linkDistanceTag: 50,
    linkStrength: 0.55,
    collidePad: 2,
    velocityDecay: 0.55,
    centerStrength: 0.04,
  };

  /// Stroke ring colour for non-missing nodes. Reads from the page
  /// background so touching nodes still separate visually.
  let strokeColor = "#1c1c1e";
  /// Body text + label outline (halo). Read from theme on each
  /// paint pass.
  let textColor = "#ebebf0";

  // ---- state ------------------------------------------------------------

  let canvas: HTMLCanvasElement | undefined = $state();
  let containerEl: HTMLDivElement | undefined = $state();
  let resizeObs: ResizeObserver | null = null;
  let sim: Simulation<DNode, DEdge> | null = null;
  let rafId: number | null = null;
  let dNodes: DNode[] = [];
  let dEdges: DEdge[] = [];
  let nodeById = new Map<string, DNode>();
  let adjacency = new Map<string, Set<string>>();
  let visibleEdgeRefs: DEdge[] = [];

  /// Pan + zoom transform. Same shape as d3-zoom's: viewportPx =
  /// worldCoord * k + (tx, ty). Updated by mouse wheel + drag-pan
  /// handlers; consumed once per paint pass.
  let transform = { x: 0, y: 0, k: 1 };

  /// Pointer interaction state. dragId: id of the node currently
  /// being pulled by the cursor (fixes its fx/fy); panStart: mouse
  /// origin + transform snapshot at the moment the user started a
  /// background-pan gesture. hoverId: node under the cursor for
  /// the hover ring + cursor change.
  let dragId: string | null = null;
  let panStart: { x: number; y: number; tx: number; ty: number } | null = null;
  let hoverId = $state<string | null>(null);
  /// Position of the mousedown that started the current gesture.
  /// Used at mouseup to decide whether the gesture was a tap
  /// (movement under a few pixels → onSelect) versus a drag.
  let downAt: { x: number; y: number } | null = null;

  // ---- icon rasterisation ----------------------------------------------

  /// Per-kind icon raster. Decoded from chan's existing
  /// lucide-style 24px SVG paths into HTMLImageElement so paint can
  /// blit each glyph with one ctx.drawImage call per node.
  /// Rebuilt whenever the theme background colour changes (the icon
  /// stroke is baked in at build time).
  const iconImages: Partial<Record<DKind, HTMLImageElement>> = {};

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

  function svgStrokeIcon(inner: string, stroke: string): string {
    const svg =
      `<svg xmlns='http://www.w3.org/2000/svg' width='48' height='48' ` +
      `viewBox='0 0 24 24' fill='none' stroke='${stroke}' stroke-width='2.4' ` +
      `stroke-linecap='round' stroke-linejoin='round'>${inner}</svg>`;
    return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
  }
  function svgTextIcon(glyph: string, fill: string): string {
    const svg =
      `<svg xmlns='http://www.w3.org/2000/svg' width='48' height='48' ` +
      `viewBox='0 0 24 24'>` +
      `<text x='12' y='18' text-anchor='middle' ` +
      `font-family='-apple-system, system-ui, sans-serif' ` +
      `font-size='20' font-weight='800' fill='${fill}'>${glyph}</text>` +
      `</svg>`;
    return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
  }

  function loadIcon(kind: DKind, dataUrl: string): void {
    const img = new Image();
    img.src = dataUrl;
    // The decode resolves asynchronously; until it lands, paint
    // skips this kind's icon. The disc + ring still render.
    img.decode?.().catch(() => { /* fall through; .complete remains true once loaded */ });
    iconImages[kind] = img;
  }

  function rebuildIcons(bg: string): void {
    loadIcon("doc", svgStrokeIcon(PATH_DOC, bg));
    loadIcon("img", svgStrokeIcon(PATH_IMG, bg));
    loadIcon("contact", svgStrokeIcon(PATH_CONTACT, bg));
    loadIcon("tag", svgTextIcon("#", bg));
    loadIcon("mention", svgTextIcon("@", bg));
  }

  // ---- theme ------------------------------------------------------------

  type ThemeColors = {
    bg: string;
    bgCard: string;
    text: string;
    textSec: string;
    doc: string;
    img: string;
    tag: string;
    mention: string;
    accent: string;
  };

  function readTheme(host: HTMLElement): ThemeColors {
    const cs = getComputedStyle(host);
    const v = (n: string, fb: string) => cs.getPropertyValue(n).trim() || fb;
    return {
      bg: v("--bg", "#1c1c1e"),
      bgCard: v("--bg-card", "#232325"),
      text: v("--text", "#ebebf0"),
      textSec: v("--text-secondary", "#8e8e93"),
      doc: v("--g-doc", "#ff8a3d"),
      img: v("--g-img", "#b07dff"),
      tag: v("--g-tag", "#6cd07a"),
      mention: v("--warn-text", "#e3b341"),
      accent: v("--accent", "#3fb950"),
    };
  }

  let theme: ThemeColors = $state({
    bg: "#1c1c1e", bgCard: "#232325", text: "#ebebf0", textSec: "#8e8e93",
    doc: "#ff8a3d", img: "#b07dff", tag: "#6cd07a", mention: "#e3b341",
    accent: "#3fb950",
  });

  function refreshTheme(): void {
    if (!containerEl) return;
    const next = readTheme(containerEl);
    theme = next;
    strokeColor = next.bg;
    textColor = next.text;
    rebuildIcons(next.bg);
  }

  // ---- node classification + data assembly -----------------------------

  function classifyFile(
    path: string,
    nodeKind: "contact" | undefined,
  ): "doc" | "img" | "contact" {
    if (/\.(png|jpe?g|gif|webp|svg|avif|bmp)$/i.test(path)) return "img";
    if (nodeKind === "contact") return "contact";
    return "doc";
  }

  /// Backlink counts per node — used to size hubs slightly larger.
  /// Pre-counted at assembly so the paint pass doesn't walk edges
  /// every frame.
  let backlinks: Map<string, number> = new Map();
  let maxBacklinks = 0;

  function rebuildAdjacency(): void {
    adjacency = new Map();
    backlinks = new Map();
    maxBacklinks = 0;
    for (const e of edges) {
      let a = adjacency.get(e.source);
      if (!a) { a = new Set(); adjacency.set(e.source, a); }
      a.add(e.target);
      let b = adjacency.get(e.target);
      if (!b) { b = new Set(); adjacency.set(e.target, b); }
      b.add(e.source);
      const c = (backlinks.get(e.target) ?? 0) + 1;
      backlinks.set(e.target, c);
      if (c > maxBacklinks) maxBacklinks = c;
    }
  }

  function renderRadius(kind: DKind, id: string): number {
    const base = kind === "doc" ? RADIUS_DOC : RADIUS_BASE;
    if (maxBacklinks <= 0) return base;
    const bl = backlinks.get(id) ?? 0;
    // Linear ramp from base to base*RADIUS_HUB_SCALE across the
    // observed backlink range. Anything heavily-linked gets a
    // visible bump; leaf nodes stay at the base size.
    const t = bl / maxBacklinks;
    return base * (1 + (RADIUS_HUB_SCALE - 1) * t);
  }

  /// Build the d3 working set from the latest props. Reuses existing
  /// DNodes when ids match (preserves position + velocity through a
  /// scope/depth tick); creates fresh ones for newly-visible ids and
  /// drops any whose id left the visible set.
  function rebuildWorkingSet(): { added: DNode[]; removed: DNode[] } {
    const newById = new Map<string, DNode>();
    const added: DNode[] = [];
    const removed: DNode[] = [];
    const focalSet = new Set(focalIds);
    for (const n of nodes) {
      if (!visibleNodeIds.has(n.id)) continue;
      const kind: DKind = n.kind === "file"
        ? classifyFile(n.path, n.node_kind)
        : n.kind === "tag" ? "tag" : "mention";
      const existing = nodeById.get(n.id);
      const missing = n.kind === "file" && Boolean(n.missing);
      const isFocal = focalSet.has(n.id);
      const radius = renderRadius(kind, n.id);
      if (existing) {
        existing.label = n.label;
        existing.kind = kind;
        existing.missing = missing;
        existing.isFocal = isFocal;
        existing.radius = radius;
        newById.set(n.id, existing);
      } else {
        const fresh: DNode = {
          id: n.id, label: n.label, kind, missing,
          isFocal, radius,
        };
        newById.set(n.id, fresh);
        added.push(fresh);
      }
    }
    for (const [id, n] of nodeById) {
      if (!newById.has(id)) removed.push(n);
    }
    // Seed fresh node positions near a visible neighbour so they
    // grow out of the cluster on a depth-bump rather than popping
    // in from the origin and being yanked across the canvas by
    // the spring forces.
    for (const a of added) {
      const adj = adjacency.get(a.id);
      let anchor: DNode | null = null;
      if (adj) {
        for (const nb of adj) {
          const ex = nodeById.get(nb);
          if (ex && ex.x != null && ex.y != null) {
            anchor = ex;
            break;
          }
        }
      }
      if (anchor) {
        a.x = anchor.x! + (Math.random() - 0.5) * 40;
        a.y = anchor.y! + (Math.random() - 0.5) * 40;
      } else {
        // No anchor available (first run or fully disconnected
        // component): drop into a 600x600 box around origin and
        // let the sim settle it.
        a.x = (Math.random() - 0.5) * 600;
        a.y = (Math.random() - 0.5) * 600;
      }
    }
    // Pin focal nodes at origin so the cluster anchors there
    // (matches the previous cytoscape behaviour). Non-focal nodes
    // keep whatever fx/fy state they had (typically null).
    for (const n of newById.values()) {
      if (n.isFocal) {
        n.fx = 0;
        n.fy = 0;
      } else if (n.fx === 0 && n.fy === 0) {
        // Was a focal; the new prop set demotes it. Release.
        n.fx = null;
        n.fy = null;
      }
    }
    nodeById = newById;
    dNodes = [...newById.values()];
    // Rebuild edges from `edges` (the full set), keyed by string
    // ids until d3-force's forceLink resolves them to objects on
    // its next tick.
    dEdges = [];
    visibleEdgeRefs = [];
    const visibleSet = new Set<string>();
    for (const e of visibleEdges) {
      visibleSet.add(`${e.source}|${e.target}|${e.kind}`);
    }
    for (const e of edges) {
      const key = `${e.source}|${e.target}|${e.kind}`;
      // Only feed the sim edges whose endpoints are visible. The
      // visibleEdges prop already encodes the chip-filter logic;
      // we cross-check the endpoint set here in case it lags by a
      // tick.
      if (!newById.has(e.source) || !newById.has(e.target)) continue;
      const de: DEdge = {
        source: e.source, target: e.target, kind: e.kind, broken: e.broken,
      };
      dEdges.push(de);
      if (visibleSet.has(key)) visibleEdgeRefs.push(de);
    }
    return { added, removed };
  }

  // ---- sim plumbing -----------------------------------------------------

  function buildSim(): void {
    if (!canvas) return;
    sim?.stop();
    sim = forceSimulation<DNode>(dNodes)
      .force(
        "link",
        forceLink<DNode, DEdge>(dEdges)
          .id((d) => d.id)
          .distance((d) =>
            d.kind === "link" ? FORCE.linkDistance : FORCE.linkDistanceTag,
          )
          .strength(FORCE.linkStrength),
      )
      .force("charge", forceManyBody<DNode>().strength(FORCE.chargeStrength))
      .force(
        "collide",
        forceCollide<DNode>().radius((d) => d.radius + FORCE.collidePad),
      )
      .force("x", forceX<DNode>(0).strength(FORCE.centerStrength))
      .force("y", forceY<DNode>(0).strength(FORCE.centerStrength))
      .velocityDecay(FORCE.velocityDecay)
      .alpha(1)
      .alphaTarget(0)
      // The animation loop drives painting independently; the sim
      // just needs to mutate positions. No-op on tick keeps us from
      // doing per-tick work twice (rAF + tick callback).
      .on("tick", () => {});
  }

  /// Re-warm with a fresh `dNodes` / `dEdges` set without
  /// reconstructing the simulation. d3-force keeps node positions
  /// across `nodes(...)` swaps when objects are reused, which is
  /// the entire point of preserving DNode identity in
  /// `rebuildWorkingSet`. Pass a higher alpha for full-rebuild
  /// gestures (scope change), a smaller one for incremental
  /// depth bumps.
  function rewarmSim(alpha: number): void {
    if (!sim) {
      buildSim();
      return;
    }
    sim.nodes(dNodes);
    const link = sim.force("link") as unknown as {
      links(ls: DEdge[]): unknown;
    } | undefined;
    if (link && typeof link.links === "function") link.links(dEdges);
    sim.alpha(alpha).restart();
  }

  // ---- canvas sizing + DPR ----------------------------------------------

  function resize(): void {
    if (!canvas || !containerEl) return;
    const r = containerEl.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(r.width * dpr));
    canvas.height = Math.max(1, Math.round(r.height * dpr));
    canvas.style.width = `${r.width}px`;
    canvas.style.height = `${r.height}px`;
    // setTransform inside paint() applies the DPR scaling along
    // with the user's pan/zoom, so we don't bake it here.
  }

  /// Translate a viewport-space pixel into model/world space using
  /// the current pan + zoom transform.
  function screenToWorld(px: number, py: number): { x: number; y: number } {
    return {
      x: (px - transform.x) / transform.k,
      y: (py - transform.y) / transform.k,
    };
  }

  function pickNode(px: number, py: number): DNode | null {
    const p = screenToWorld(px, py);
    let best: DNode | null = null;
    let bestD2 = Infinity;
    for (const n of dNodes) {
      if (n.x == null || n.y == null) continue;
      const dx = n.x - p.x;
      const dy = n.y - p.y;
      // Add ~4px slack around the node so small targets are still
      // clickable; closer hits win when several discs overlap.
      const r = n.radius + 4 / Math.max(0.5, transform.k);
      const d2 = dx * dx + dy * dy;
      if (d2 <= r * r && d2 < bestD2) {
        best = n;
        bestD2 = d2;
      }
    }
    return best;
  }

  // ---- paint ------------------------------------------------------------

  function paint(): void {
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    // Combined transform: DPR scaling × user pan/zoom. setTransform
    // re-applies from identity each frame so cumulative drift is
    // impossible.
    ctx.setTransform(
      dpr * transform.k, 0, 0, dpr * transform.k,
      dpr * transform.x, dpr * transform.y,
    );
    ctx.clearRect(
      -transform.x / transform.k,
      -transform.y / transform.k,
      canvas.width / (dpr * transform.k),
      canvas.height / (dpr * transform.k),
    );

    const adj = selectedId ? adjacency.get(selectedId) : null;

    // Edges first so nodes paint on top. Group by kind so we only
    // change strokeStyle once per kind.
    ctx.lineWidth = 1 / Math.max(0.5, transform.k);
    const edgesByKind: Record<RenderedEdgeKind, DEdge[]> = {
      link: [], tag: [], mention: [], group: [],
    };
    for (const e of visibleEdgeRefs) edgesByKind[e.kind].push(e);
    for (const kind of ["link", "tag", "mention", "group"] as const) {
      const list = edgesByKind[kind];
      if (list.length === 0) continue;
      ctx.globalAlpha = 0.18;
      ctx.strokeStyle =
        kind === "link" ? theme.text
        : kind === "tag" ? theme.tag
        : kind === "mention" ? theme.mention
        : theme.accent;
      ctx.beginPath();
      for (const e of list) {
        const s = e.source as DNode;
        const t = e.target as DNode;
        if (s.x == null || t.x == null) continue;
        if (e.broken) continue; // dashed pass below
        ctx.moveTo(s.x!, s.y!);
        ctx.lineTo(t.x!, t.y!);
      }
      ctx.stroke();
      // Broken edges: same colour, lower alpha, dashed.
      const broken = list.filter((e) => e.broken);
      if (broken.length > 0) {
        ctx.save();
        ctx.setLineDash([3 / Math.max(0.5, transform.k), 3 / Math.max(0.5, transform.k)]);
        ctx.globalAlpha = 0.12;
        ctx.beginPath();
        for (const e of broken) {
          const s = e.source as DNode;
          const t = e.target as DNode;
          if (s.x == null || t.x == null) continue;
          ctx.moveTo(s.x!, s.y!);
          ctx.lineTo(t.x!, t.y!);
        }
        ctx.stroke();
        ctx.restore();
      }
    }
    ctx.globalAlpha = 1;

    // Nodes: fill disc, stroke ring, blit icon, optional label.
    for (const n of dNodes) {
      if (n.x == null || n.y == null) continue;
      const isSel = n.id === selectedId;
      const isAdj = adj?.has(n.id) === true;
      const isHover = n.id === hoverId;
      // Fill colour follows the kind palette unless this is a
      // broken-link ghost, which sinks to the muted card colour
      // with a dashed ring.
      const fill = n.missing
        ? theme.bgCard
        : n.kind === "doc" ? theme.doc
        : n.kind === "img" ? theme.img
        : n.kind === "contact" ? theme.mention
        : n.kind === "tag" ? theme.tag
        : theme.mention;
      ctx.beginPath();
      ctx.arc(n.x, n.y, n.radius, 0, Math.PI * 2);
      ctx.fillStyle = fill;
      ctx.fill();
      // Stroke ring. Selected / hover get the same thickness as
      // the rest so the disc size stays stable; the colour change
      // is what signals state.
      ctx.lineWidth = n.missing
        ? 1 / Math.max(0.5, transform.k)
        : isSel ? 2 / Math.max(0.5, transform.k)
        : 1.2 / Math.max(0.5, transform.k);
      ctx.strokeStyle = n.missing
        ? theme.textSec
        : isSel ? theme.text
        : isHover ? theme.text
        : strokeColor;
      if (n.missing) {
        ctx.save();
        ctx.setLineDash([2 / Math.max(0.5, transform.k), 2 / Math.max(0.5, transform.k)]);
        ctx.stroke();
        ctx.restore();
      } else {
        ctx.stroke();
      }
      // Icon blit. Skips while the SVG is still decoding; the disc
      // alone still reads as a kind via colour.
      const icon = iconImages[n.kind];
      if (icon && icon.complete && icon.naturalWidth > 0) {
        const w = n.radius * 2 * ICON_FRACTION;
        ctx.globalAlpha = n.missing ? 0.4 : 1;
        ctx.drawImage(icon, n.x - w / 2, n.y - w / 2, w, w);
        ctx.globalAlpha = 1;
      }
      // Label: only for the selected node + first-degree
      // neighbours, so the canvas stays uncluttered at rest.
      if (isSel || isAdj) {
        const fontPx = 11 / Math.max(0.5, transform.k);
        ctx.font = `${fontPx}px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "bottom";
        const labelY = n.y - n.radius - 4 / Math.max(0.5, transform.k);
        // Halo: thick stroke in page-bg, then thin fill in text.
        ctx.lineWidth = 3 / Math.max(0.5, transform.k);
        ctx.strokeStyle = theme.bg;
        ctx.strokeText(n.label, n.x, labelY);
        ctx.fillStyle = textColor;
        ctx.fillText(n.label, n.x, labelY);
      }
    }
  }

  function loop(): void {
    paint();
    rafId = requestAnimationFrame(loop);
  }

  // ---- pointer handlers -------------------------------------------------

  function localCoords(e: MouseEvent): { x: number; y: number } {
    const r = canvas!.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  function onMouseDown(e: MouseEvent): void {
    if (!canvas) return;
    if (e.button !== 0) return; // right-click → context menu
    const p = localCoords(e);
    downAt = { x: p.x, y: p.y };
    const n = pickNode(p.x, p.y);
    if (n) {
      dragId = n.id;
      n.fx = n.x;
      n.fy = n.y;
      sim?.alphaTarget(0.3).restart();
    } else {
      panStart = { x: e.clientX, y: e.clientY, tx: transform.x, ty: transform.y };
    }
  }

  function onMouseMove(e: MouseEvent): void {
    if (!canvas) return;
    const p = localCoords(e);
    if (dragId) {
      const n = nodeById.get(dragId);
      if (!n) return;
      const w = screenToWorld(p.x, p.y);
      n.fx = w.x;
      n.fy = w.y;
      return;
    }
    if (panStart) {
      transform.x = panStart.tx + (e.clientX - panStart.x);
      transform.y = panStart.ty + (e.clientY - panStart.y);
      return;
    }
    // Cheap hover update. Picks against the same hit-test as click.
    const h = pickNode(p.x, p.y);
    hoverId = h?.id ?? null;
  }

  function onMouseUp(e: MouseEvent): void {
    const p = localCoords(e);
    const moved =
      downAt && (Math.abs(p.x - downAt.x) > 3 || Math.abs(p.y - downAt.y) > 3);
    if (dragId) {
      const n = nodeById.get(dragId);
      if (n && !n.isFocal) {
        // Release the node back to the simulation. Focal nodes
        // remain pinned at origin regardless.
        n.fx = null;
        n.fy = null;
      }
      sim?.alphaTarget(0);
      dragId = null;
      if (!moved) {
        // A tap on a node (no drag movement) selects it.
        const tapped = pickNode(p.x, p.y);
        onSelect(tapped ? tapped.id : null);
      }
    } else if (panStart) {
      panStart = null;
      if (!moved) {
        // Background tap clears selection.
        onSelect(null);
      }
    }
    downAt = null;
  }

  function onWheel(e: WheelEvent): void {
    if (!canvas) return;
    e.preventDefault();
    const p = localCoords(e);
    // Wheel deltas are wildly inconsistent across input devices:
    // mouse-wheel notches fire ~100, trackpads fire dozens of
    // small events (~3-15), `deltaMode` further changes units.
    // Map every delta through `exp(-d * SENSITIVITY)` so the
    // effective zoom factor is smooth and bounded regardless of
    // device, and tune SENSITIVITY low enough that a trackpad
    // pinch doesn't snap-zoom across the cluster.
    const SENSITIVITY = 0.0015;
    const factor = Math.exp(-e.deltaY * SENSITIVITY);
    const k2 = Math.min(6, Math.max(0.15, transform.k * factor));
    // Zoom toward the cursor: the world point under the cursor
    // must stay anchored across the transform. Solve for the new
    // (tx, ty) that holds wx, wy invariant.
    transform.x = p.x - ((p.x - transform.x) * k2) / transform.k;
    transform.y = p.y - ((p.y - transform.y) * k2) / transform.k;
    transform.k = k2;
  }

  function onContextMenuLocal(e: MouseEvent): void {
    // Delegate to the parent so the existing hamburger-menu
    // affordance still works on right-click.
    onContextMenu?.(e);
  }

  // ---- lifecycle --------------------------------------------------------

  /// First-open setup: pre-rasterise icons, kick the resize
  /// observer, build the working set + sim, start the rAF loop.
  function start(): void {
    if (!canvas || !containerEl) return;
    refreshTheme();
    resize();
    rebuildAdjacency();
    rebuildWorkingSet();
    // Centre the world origin in the viewport so the focal-pinned
    // cluster lands in the middle of the canvas on first paint.
    if (canvas) {
      transform = { x: canvas.clientWidth / 2, y: canvas.clientHeight / 2, k: 1 };
    }
    buildSim();
    if (rafId === null) rafId = requestAnimationFrame(loop);
  }

  function stop(): void {
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
    sim?.stop();
    sim = null;
    nodeById = new Map();
    dNodes = [];
    dEdges = [];
    visibleEdgeRefs = [];
  }

  onMount(() => {
    if (!canvas || !containerEl) return;
    resizeObs = new ResizeObserver(() => resize());
    resizeObs.observe(containerEl);
    // Theme tracker: re-read CSS variables when the document's
    // `data-theme` attribute flips so light/dark switches without
    // a remount.
    const themeObs = new MutationObserver(() => refreshTheme());
    themeObs.observe(document.documentElement, {
      attributes: true, attributeFilter: ["data-theme"],
    });
    if (open) start();
    return () => {
      themeObs.disconnect();
    };
  });

  onDestroy(() => {
    resizeObs?.disconnect();
    stop();
  });

  // ---- prop-reactive effects -------------------------------------------

  /// Open/close: start the renderer when the overlay becomes
  /// visible, tear it down when it closes. Matches the previous
  /// cytoscape lifecycle and keeps idle overlays from burning rAF
  /// budget.
  $effect(() => {
    if (open) {
      if (!sim) start();
    } else {
      stop();
    }
  });

  /// Nodes / edges arrays changed (new graph payload from the
  /// server). Full rebuild: regenerate adjacency, recreate the
  /// working set, restart the sim with alpha=1.
  $effect(() => {
    void nodes;
    void edges;
    if (!sim) return;
    rebuildAdjacency();
    rebuildWorkingSet();
    rewarmSim(1);
  });

  /// Visibility change without a full data swap: scope / depth /
  /// chip filters moved. Incremental rebuild — preserves existing
  /// node positions, seeds new arrivals near a visible neighbour,
  /// re-warms with a small alpha so the cluster relaxes around the
  /// edit instead of jumping.
  $effect(() => {
    void visibleNodeIds;
    void visibleEdges;
    void focalIds;
    if (!sim) return;
    const { added, removed } = rebuildWorkingSet();
    const alpha = added.length > 0 ? 0.35 : removed.length > 0 ? 0.2 : 0.1;
    rewarmSim(alpha);
  });

  /// Cursor: pointer over a node when nothing is being dragged.
  const cursor = $derived(
    dragId !== null ? "grabbing"
    : hoverId !== null ? "pointer"
    : "grab",
  );
</script>

<div
  bind:this={containerEl}
  class="canvas-host"
  oncontextmenu={onContextMenuLocal}
  role="presentation"
>
  <canvas
    bind:this={canvas}
    style:cursor={cursor}
    onmousedown={onMouseDown}
    onmousemove={onMouseMove}
    onmouseup={onMouseUp}
    onmouseleave={onMouseUp}
    onwheel={onWheel}
  ></canvas>
</div>

<style>
  .canvas-host {
    position: relative;
    width: 100%;
    height: 100%;
  }
  canvas {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
