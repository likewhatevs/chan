<script lang="ts">
  // Graph view overlay: spherical / globe layout.
  //
  // Nodes are distributed on a unit sphere via the Fibonacci spiral
  // (even-ish coverage independent of N), then rotated by the user
  // and orthographically projected to 2D SVG. Edges are straight
  // lines between projected endpoints; their opacity fades with the
  // average depth so back edges don't visually clutter the front.
  //
  // Interaction:
  //   - drag empty space to rotate the globe (yaw + pitch).
  //   - scroll wheel to zoom (changes the on-screen radius).
  //   - click a file node to inspect; the side panel's Open button
  //     routes via openInActivePane and closes the overlay so the
  //     workspace pane gets focus.
  //   - per-edge-type filter chips toggle which edges (and the
  //     non-file nodes attached only to filtered edges) are drawn.
  //
  // Scope picker (header) gates which nodes are even candidates for
  // rendering: file scope = the chosen file plus its `depth`-hop
  // neighborhood; group scope = the same applied to the union of
  // visible files; drive = no filter (the full graph). Filtering
  // happens client-side; /api/graph still returns the whole graph
  // for v1 — TODO: add a server-side scope/paths/depth param so
  // huge drives don't ship megabytes of edges to skip.

  import { onDestroy, onMount } from "svelte";

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

  /// Position on the unit sphere, computed once at load and never
  /// mutated thereafter. Rotation is applied in the projection step.
  type SphereNode = GraphViewNode & {
    /// Base (unrotated) position on the unit sphere.
    bx: number;
    by: number;
    bz: number;
  };

  type EdgeKind = GraphViewEdge["kind"];

  // ---- state -------------------------------------------------------------

  let svgEl: SVGSVGElement | undefined = $state();
  let width = $state(800);
  let height = $state(600);

  // Rotation in radians. rotY is yaw (mouse X drag), rotX is pitch
  // (mouse Y drag). We compose Y first then X so horizontal drags
  // spin the globe like a record and vertical drags tilt it.
  let rotY = $state(0);
  let rotX = $state(0);

  // Globe radius in screen pixels and pan offsets. Wheel adjusts
  // radius for zoom; mousedown on empty space rotates rather than
  // pans, so panning isn't user-controllable in this layout; we
  // rely on the SVG center.
  let radius = $state(220);

  let nodes: SphereNode[] = $state([]);
  let edges: GraphViewEdge[] = $state([]);
  let loading = $state(true);
  let error: string | null = $state(null);

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

  // ---- projection --------------------------------------------------------

  /// Focal length (in sphere radii) for the perspective division.
  /// Lower → more dramatic foreshortening; higher → closer to
  /// orthographic. 2.6 sells "globe" without the back hemisphere
  /// pinching to nothing.
  const FOCAL = 2.6;

  /// Rotate (bx, by, bz) by the current (rotY, rotX), apply
  /// perspective, and project to the SVG center. Returns screen
  /// coords plus the rotated z and the perspective scale (used for
  /// node size and label scaling).
  function project(n: { bx: number; by: number; bz: number }): {
    sx: number;
    sy: number;
    z: number;
    persp: number;
  } {
    const cosY = Math.cos(rotY);
    const sinY = Math.sin(rotY);
    const cosX = Math.cos(rotX);
    const sinX = Math.sin(rotX);
    // Yaw around the Y axis (mixes x & z).
    const x1 = n.bx * cosY + n.bz * sinY;
    const z1 = -n.bx * sinY + n.bz * cosY;
    // Pitch around the X axis (mixes y & z1).
    const y2 = n.by * cosX - z1 * sinX;
    const z2 = n.by * sinX + z1 * cosX;
    const persp = FOCAL / (FOCAL - z2);
    return {
      sx: width / 2 + x1 * radius * persp,
      sy: height / 2 + y2 * radius * persp,
      z: z2,
      persp,
    };
  }

  // Project + cache by node id, recomputed whenever rotation, radius,
  // or canvas size changes (those are all reactive $state reads).
  type Proj = { sx: number; sy: number; z: number; persp: number };
  const projected = $derived.by(() => {
    const m = new Map<string, Proj>();
    for (const n of nodes) m.set(n.id, project(n));
    return m;
  });

  // ---- great-circle arcs and wireframe ----------------------------------

  /// Sample a great-circle arc between two points on the unit sphere.
  /// Uses spherical linear interpolation (slerp); samples are
  /// uniform in arc-length so the resulting polyline curves smoothly
  /// regardless of how far apart the endpoints are.
  function arcPoints(
    a: { bx: number; by: number; bz: number },
    b: { bx: number; by: number; bz: number },
    segments: number,
  ): Proj[] {
    // Dot product on the unit sphere = cos(angle between them).
    const dot = Math.max(-1, Math.min(1, a.bx * b.bx + a.by * b.by + a.bz * b.bz));
    const omega = Math.acos(dot);
    const sinO = Math.sin(omega);
    const out: Proj[] = [];
    // Antipodal-ish endpoints have an undefined arc; fall back to a
    // straight chord to avoid divide-by-zero. (Practically unreachable
    // for our data but cheap to guard.)
    if (sinO < 1e-6) {
      out.push(project(a));
      out.push(project(b));
      return out;
    }
    for (let i = 0; i <= segments; i++) {
      const t = i / segments;
      const k1 = Math.sin((1 - t) * omega) / sinO;
      const k2 = Math.sin(t * omega) / sinO;
      out.push(
        project({
          bx: k1 * a.bx + k2 * b.bx,
          by: k1 * a.by + k2 * b.by,
          bz: k1 * a.bz + k2 * b.bz,
        }),
      );
    }
    return out;
  }

  /// SVG path for a list of projected points. Front-facing segments
  /// are emitted as `M`/`L` commands; gaps where the arc dips behind
  /// the sphere become `M` jumps so the back portion stays hidden.
  function arcPath(pts: Proj[], hideBack: boolean): string {
    let d = "";
    let inSegment = false;
    for (const p of pts) {
      if (hideBack && p.z < -0.05) {
        inSegment = false;
        continue;
      }
      d += inSegment ? `L${p.sx.toFixed(1)} ${p.sy.toFixed(1)}` : `M${p.sx.toFixed(1)} ${p.sy.toFixed(1)}`;
      inSegment = true;
    }
    return d;
  }

  /// Companion to `arcPath`: returns just the back-facing portion
  /// of the arc so it can render as a dashed / dimmed stroke. Pairs
  /// with `arcPath(pts, true)` for the visible front segments. Used
  /// for graph edges so connections that pass behind the globe stay
  /// visible (helps the user see that node A actually links to node
  /// B even when B is on the far side).
  function arcPathBack(pts: Proj[]): string {
    let d = "";
    let inSegment = false;
    for (const p of pts) {
      if (p.z >= -0.05) {
        inSegment = false;
        continue;
      }
      d += inSegment ? `L${p.sx.toFixed(1)} ${p.sy.toFixed(1)}` : `M${p.sx.toFixed(1)} ${p.sy.toFixed(1)}`;
      inSegment = true;
    }
    return d;
  }

  /// Background wireframe: a few latitude rings and meridians,
  /// projected through the same rotation. Computed once per
  /// rotation/radius change via $derived; back portions are hidden
  /// so the sphere reads as a solid (if translucent) globe.
  const wireframe = $derived.by(() => {
    void rotX;
    void rotY;
    void radius;
    void width;
    void height;
    const SEG = 64;
    const lats: string[] = [];
    // Latitudes at -60°, -30°, 0°, 30°, 60° (skip the poles).
    for (const latDeg of [-60, -30, 0, 30, 60]) {
      const lat = (latDeg * Math.PI) / 180;
      const r = Math.cos(lat);
      const y = Math.sin(lat);
      const pts: Proj[] = [];
      for (let i = 0; i <= SEG; i++) {
        const a = (i / SEG) * Math.PI * 2;
        pts.push(project({ bx: r * Math.cos(a), by: y, bz: r * Math.sin(a) }));
      }
      lats.push(arcPath(pts, true));
    }
    const merids: string[] = [];
    // Six meridians evenly around the globe.
    for (let m = 0; m < 6; m++) {
      const lon = (m / 6) * Math.PI * 2;
      const cosL = Math.cos(lon);
      const sinL = Math.sin(lon);
      const pts: Proj[] = [];
      for (let i = 0; i <= SEG; i++) {
        const lat = (i / SEG - 0.5) * Math.PI;
        const r = Math.cos(lat);
        pts.push(project({ bx: r * cosL, by: Math.sin(lat), bz: r * sinL }));
      }
      merids.push(arcPath(pts, true));
    }
    return { lats, merids };
  });

  /// Visible nodes in render order (back-to-front, so near nodes
  /// overlap far ones).
  const drawOrder = $derived.by(() => {
    const arr = nodes
      .filter((n) => visibleNodeIds.has(n.id))
      .map((n) => ({ n, p: projected.get(n.id)! }));
    arr.sort((a, b) => a.p.z - b.p.z);
    return arr;
  });

  /// Visible edges as great-circle arcs. The polyline follows the
  /// sphere surface so the rendering reads as a globe rather than a
  /// flat chord diagram. Back portions are hidden via gap-jumps in
  /// the path so the sphere visually occludes its own far side.
  const drawEdges = $derived.by(() => {
    void rotX;
    void rotY;
    void radius;
    void width;
    void height;
    const nodeById = new Map(nodes.map((n) => [n.id, n]));
    return visibleEdges
      .map((e) => {
        const a = nodeById.get(e.source);
        const b = nodeById.get(e.target);
        if (!a || !b) return null;
        const pts = arcPoints(a, b, 24);
        const avgZ = (pts[0]!.z + pts[pts.length - 1]!.z) / 2;
        const opacity = 0.18 + 0.62 * ((avgZ + 1) / 2);
        return {
          e,
          d: arcPath(pts, true),
          dBack: arcPathBack(pts),
          opacity,
        };
      })
      .filter(
        (
          x,
        ): x is {
          e: GraphViewEdge;
          d: string;
          dBack: string;
          opacity: number;
        } => !!x,
      );
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
          // Keep the globe inside the viewport when the pane shrinks.
          radius = Math.min(radius, Math.min(width, height) * 0.45);
        }
      });
      resizeObs.observe(svgEl);
    }
    await load();
  });

  onDestroy(() => {
    resizeObs?.disconnect();
  });

  async function load(): Promise<void> {
    loading = true;
    error = null;
    try {
      const g: GraphView = await api.graph();
      layoutFibonacci(g);
    } catch (e) {
      error = (e as Error).message;
    } finally {
      loading = false;
    }
  }

  /// Place every node on a unit sphere via the Fibonacci spiral.
  /// Sorting nodes by (kind, label) before assigning indices keeps
  /// same-kind nodes adjacent on the spiral; colors band visibly
  /// rather than scattering.
  function layoutFibonacci(g: GraphView): void {
    const KIND_ORDER: Record<GraphViewNode["kind"], number> = {
      file: 0,
      tag: 1,
      mention: 2,
      date: 3,
    };
    const sorted = [...g.nodes].sort((a, b) => {
      const k = KIND_ORDER[a.kind] - KIND_ORDER[b.kind];
      if (k !== 0) return k;
      return a.label.localeCompare(b.label);
    });

    const N = sorted.length;
    const phi = Math.PI * (3 - Math.sqrt(5)); // golden angle
    const out: SphereNode[] = sorted.map((n, i) => {
      // y in [-1, 1], radius_at_y = sqrt(1 - y*y), angle stepping
      // by the golden angle gives a uniform distribution.
      const y = N === 1 ? 0 : 1 - (i / (N - 1)) * 2;
      const r = Math.sqrt(Math.max(0, 1 - y * y));
      const theta = i * phi;
      return {
        ...n,
        bx: Math.cos(theta) * r,
        by: y,
        bz: Math.sin(theta) * r,
      };
    });
    nodes = out;
    edges = g.edges;
  }

  // ---- interaction: rotate + zoom + click -------------------------------

  let rotateStart: { x: number; y: number; rotX: number; rotY: number } | null = null;
  // mousedown bookkeeping for "click vs drag" on a node.
  const DRAG_THRESHOLD = 4;
  let nodeDown: { node: SphereNode; x: number; y: number; moved: boolean } | null = null;

  function onSvgMouseDown(e: MouseEvent): void {
    rotateStart = { x: e.clientX, y: e.clientY, rotX, rotY };
    window.addEventListener("mousemove", onRotateMove);
    window.addEventListener("mouseup", onRotateUp);
  }

  function onRotateMove(e: MouseEvent): void {
    if (!rotateStart) return;
    // 0.0085 rad/px ≈ a half-turn for a ~370 px drag, comfortable
    // on most laptop trackpads without overshooting.
    const k = 0.0085;
    rotY = rotateStart.rotY + (e.clientX - rotateStart.x) * k;
    // Clamp pitch so the globe never flips upside-down (avoids a
    // disorienting "label suddenly mirrored" jump).
    const nextX = rotateStart.rotX + (e.clientY - rotateStart.y) * k;
    rotX = Math.max(-Math.PI / 2 + 0.01, Math.min(Math.PI / 2 - 0.01, nextX));
  }

  function onRotateUp(): void {
    rotateStart = null;
    window.removeEventListener("mousemove", onRotateMove);
    window.removeEventListener("mouseup", onRotateUp);
  }

  function onWheel(e: WheelEvent): void {
    e.preventDefault();
    const factor = Math.exp(-e.deltaY * 0.0015);
    const minR = 60;
    const maxR = Math.min(width, height) * 0.48;
    radius = Math.max(minR, Math.min(maxR, radius * factor));
  }

  function onNodeMouseDown(e: MouseEvent, n: SphereNode): void {
    e.stopPropagation();
    nodeDown = { node: n, x: e.clientX, y: e.clientY, moved: false };
    window.addEventListener("mousemove", onNodeMove);
    window.addEventListener("mouseup", onNodeUp);
  }

  function onNodeMove(e: MouseEvent): void {
    if (!nodeDown) return;
    const dx = e.clientX - nodeDown.x;
    const dy = e.clientY - nodeDown.y;
    if (!nodeDown.moved && Math.hypot(dx, dy) >= DRAG_THRESHOLD) {
      nodeDown.moved = true;
      // Promote to a globe rotation drag: hand off to the rotate
      // handlers as if the user had pressed on empty space.
      rotateStart = { x: nodeDown.x, y: nodeDown.y, rotX, rotY };
      window.addEventListener("mousemove", onRotateMove);
      window.addEventListener("mouseup", onRotateUp);
    }
    if (nodeDown.moved) onRotateMove(e);
  }

  function onNodeUp(e: MouseEvent): void {
    if (nodeDown && !nodeDown.moved) {
      // Selection rather than open: file vs tag/mention/date all
      // route through the side panel. Clicking the same node twice
      // doesn't toggle (deselect) because that would feel
      // accidental during normal browsing; use the panel's close
      // affordance to clear instead.
      selectedId = nodeDown.node.id;
    }
    nodeDown = null;
    window.removeEventListener("mousemove", onNodeMove);
    window.removeEventListener("mouseup", onNodeUp);
    // If we promoted to a rotation, let onRotateUp run via the
    // window mouseup it registered; otherwise nothing to do.
    void e;
  }

  function resetView(): void {
    rotX = 0;
    rotY = 0;
    radius = Math.min(width, height) * 0.4;
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

  function nodeRadius(n: SphereNode, p: Proj): number {
    const base = n.kind === "file" ? 6 : 4;
    // Use the perspective scale directly so node size tracks the
    // same projection as position. Clamp so back nodes don't
    // disappear and front nodes don't blow up.
    return Math.max(2.5, Math.min(12, base * p.persp));
  }

  function nodeOpacity(p: Proj): number {
    // Stronger fade on the back so the sphere visually occludes
    // its own far side without fully hiding it (drag-to-rotate
    // discoverability).
    return 0.3 + 0.7 * ((p.z + 1) / 2);
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
      <!-- Globe silhouette: a faint disc behind the wireframe so
           the sphere reads as a translucent ball rather than a wire
           cage in vacuum. -->
      <circle
        class="globe-fill"
        cx={width / 2}
        cy={height / 2}
        r={radius}
      />
      <!-- Latitude / longitude wireframe. Drawn under the edges so
           reference grid stays out of the way of the data. -->
      <g class="wireframe">
        {#each wireframe.lats as d, i (`lat-${i}`)}
          <path d={d} fill="none" stroke="var(--border)" />
        {/each}
        {#each wireframe.merids as d, i (`mer-${i}`)}
          <path d={d} fill="none" stroke="var(--border)" />
        {/each}
      </g>

      {#each drawEdges as { e, d, dBack, opacity } (`${e.source}->${e.target}-${e.kind}`)}
        {#if dBack}
          <!-- Back-of-globe portion: dashed + dimmed so the user
               sees that the edge continues but the front of the
               sphere is still visually solid in front of it. -->
          <path
            d={dBack}
            fill="none"
            stroke={EDGE_COLORS[e.kind]}
            stroke-opacity={opacity * (e.broken ? 0.6 : 1) * 0.35}
            stroke-dasharray="2 4"
            stroke-width={e.kind === "link" ? 1.5 : 1}
            stroke-linecap="round"
          />
        {/if}
        <path
          d={d}
          fill="none"
          stroke={EDGE_COLORS[e.kind]}
          stroke-opacity={opacity * (e.broken ? 0.6 : 1)}
          stroke-dasharray={e.broken ? "3 3" : undefined}
          stroke-width={e.kind === "link" ? 1.5 : 1}
          stroke-linecap="round"
        />
      {/each}

      {#each drawOrder as { n, p } (n.id)}
        <g
          class="node"
          class:file={n.kind === "file"}
          class:missing={n.kind === "file" && n.missing}
          transform={`translate(${p.sx}, ${p.sy})`}
          opacity={nodeOpacity(p)}
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
              r={nodeRadius(n, p) + 4}
              fill="none"
              stroke="var(--accent)"
              stroke-width="2"
            />
          {/if}
          <circle r={nodeRadius(n, p)} fill={NODE_COLORS[n.kind]} />
          {#if p.z > -0.4}
            <text
              class="label-bg"
              x={nodeRadius(n, p) + 5}
              y={3}
              font-size={n.kind === "file" ? 11 : 10}
              pointer-events="none"
            >{n.label}</text>
            <text
              class="label"
              x={nodeRadius(n, p) + 5}
              y={3}
              font-size={n.kind === "file" ? 11 : 10}
              pointer-events="none"
            >{n.label}</text>
          {/if}
        </g>
      {/each}
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
    <span class="hint">drag to rotate · scroll to zoom · click a node to inspect</span>
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
    /* Force the OverlayShell panel to fill its declared maxHeight.
       The inner SVG canvas has no intrinsic height; without this
       min-height the panel collapses to just the top bar + status
       bar and the graph reads as a tiny strip. Slightly under the
       panel's max-height (92vh) so the cap still wins on short
       viewports. */
    min-height: 90vh;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.5rem;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
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
    font-size: 12px;
    max-width: 280px;
    cursor: pointer;
  }
  .scope-select:focus { outline: none; border-color: var(--link); }
  .depth {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--text-secondary);
    font-size: 11px;
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
    font-size: 11px;
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
    font-size: 12px;
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
    font-size: 10px;
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
    font-size: 14px;
    font-weight: 600;
    word-break: break-word;
  }
  .details .path {
    color: var(--text-secondary);
    font-size: 11px;
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
    font-size: 12px;
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
    font-size: 11px;
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
    font-size: 10px;
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
  .globe-fill {
    fill: var(--bg-card);
    opacity: 0.35;
  }
  .wireframe path {
    opacity: 0.45;
    stroke-width: 0.75;
  }
  .node {
    cursor: pointer;
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
