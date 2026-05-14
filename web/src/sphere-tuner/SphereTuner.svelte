<script lang="ts">
  // Standalone sphere-shading tuner. Runs without the chan server.
  // Reachable in dev at http://localhost:5173/sphere-tuner.html.
  //
  // The tuner exposes every shading knob the production graph uses
  // (gradient stops, hotspot position + size, optional specular dot,
  // icon scale, border treatment) as live sliders against a graph
  // sized like the user's real drive (~90 nodes). Pick values here,
  // copy them back into `svgSphereIcon` in
  // `components/GraphPanel.svelte` when satisfied. The "copy values"
  // button at the bottom emits a ready-to-paste TS fragment with
  // the current settings.

  import { onDestroy, onMount } from "svelte";
  import cytoscape from "cytoscape";
  import type { Core, ElementDefinition, Layouts } from "cytoscape";
  // @ts-expect-error fcose ships no .d.ts
  import fcose from "cytoscape-fcose";
  // @ts-expect-error cytoscape-d3-force ships no .d.ts
  import d3Force from "cytoscape-d3-force";

  import type { GraphView } from "../api/types";
  import { makeFakeGraph, type GraphSpec } from "../graph-demo/fakeData";

  cytoscape.use(fcose);
  cytoscape.use(d3Force);

  // ---- shading state ----------------------------------------------------

  let shade = $state({
    /// Lightness added to the base colour at the gradient's
    /// brightest stop. Higher = more pop, but past ~0.85 the
    /// kind colour washes out into white.
    highlightDelta: 0.55,
    /// Lightness removed at the gradient's darkest stop.
    /// More negative = deeper shadow / more 3D volume.
    shadowDelta: -0.65,
    /// Position of the highlight inside the disc, expressed as
    /// fractions of the 24x24 viewBox.
    highlightCx: 0.32,
    highlightCy: 0.26,
    /// Radius of the gradient falloff. Smaller = tighter, brighter
    /// hotspot; larger = softer, more diffuse.
    gradientRadius: 0.95,
    /// Optional second white-ish stop right at the highlight to
    /// suggest a specular reflection. Off by default; turn on for
    /// a glossier "billiard ball" look.
    specularOn: true,
    /// Brightness of the specular stop, applied on top of the base
    /// colour. 1 = pure white, 0 = no specular.
    specularIntensity: 0.55,
    /// Radius of the specular stop as a fraction of the disc.
    /// Smaller = sharper pinpoint.
    specularSize: 0.12,
    /// Icon glyph occupies this fraction of the disc width.
    /// Smaller leaves more sphere surface visible so the shading
    /// reads through; the production default is 0.55.
    iconScale: 0.42,
    /// Border around the node. Three modes:
    ///   "page-bg": bg-colour ring (current production behaviour;
    ///              gives a halo against edges).
    ///   "shaded":  use the darkest gradient stop so the ring
    ///              continues the sphere shading.
    ///   "none":    no border.
    borderMode: "shaded" as "page-bg" | "shaded" | "none",
    borderWidth: 1.0,
    /// Base node diameter (px in model space). The largest node
    /// scales up by `nodeRatio` based on backlink count.
    nodeBase: 26,
    nodeRatio: 1.2,
    /// Toggle to compare the sphere against the prior flat-disc
    /// look at the same node size.
    useSphere: true,
    /// "Observable d3-force"-style preset: tiny flat circles, no
    /// icon, thin white-ish stroke. Drops the icon and the sphere
    /// gradient regardless of the other toggles when enabled, so
    /// the user can A/B this against the chan-style nodes without
    /// fiddling individual sliders.
    flatObservable: false,
    /// Render labels above each node. Off by default per the
    /// "remove labels, leave just the circles" direction.
    labels: false,
  });

  let theme = $state<"dark" | "light">("dark");

  let spec = $state<GraphSpec>({
    seed: 7,
    documents: 58,
    images: 4,
    tags: 14,
    mentions: 14,
    dates: 0,
    linkDensity: 0.07,
    imageRefDensity: 0.05,
    tagsPerDoc: 2.1,
    mentionsPerDoc: 0.9,
    datesPerDoc: 0,
  });

  let containerEl: HTMLDivElement | undefined = $state();
  let cy: Core | null = null;
  let resizeObs: ResizeObserver | null = null;
  let forceLayout: Layouts | null = null;
  let stats = $state({ nodes: 0, edges: 0 });

  // ---- colour math + sphere SVG (mirrored from GraphPanel) --------------

  function parseColor(s: string): [number, number, number] {
    const t = s.trim();
    if (t.startsWith("#")) {
      let h = t.slice(1);
      if (h.length === 3) h = h.split("").map((c) => c + c).join("");
      const n = parseInt(h, 16);
      return [(n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff];
    }
    const m = t.match(/^rgba?\(([^)]+)\)/i);
    if (m) {
      const p = m[1]!.split(",").map((x) => parseFloat(x.trim()));
      return [p[0] | 0, p[1] | 0, p[2] | 0];
    }
    return [128, 128, 128];
  }
  function shadeColor(s: string, delta: number): string {
    let [r, g, b] = parseColor(s);
    if (delta >= 0) {
      r = Math.round(r + (255 - r) * delta);
      g = Math.round(g + (255 - g) * delta);
      b = Math.round(b + (255 - b) * delta);
    } else {
      const k = 1 + delta;
      r = Math.round(r * k);
      g = Math.round(g * k);
      b = Math.round(b * k);
    }
    return `#${[r, g, b].map((x) => x.toString(16).padStart(2, "0")).join("")}`;
  }

  function svgSphereIcon(
    base: string,
    inner: string,
    mode: "stroke" | "text",
    iconColor: string,
  ): string {
    const highlight = shadeColor(base, shade.highlightDelta);
    const shadow = shadeColor(base, shade.shadowDelta);
    const specular = shade.specularOn
      ? shadeColor("#ffffff", -(1 - shade.specularIntensity))
      : null;
    const iconLayer =
      mode === "stroke"
        ? `<g fill='none' stroke='${iconColor}' stroke-width='2.4' ` +
          `stroke-linecap='round' stroke-linejoin='round'>${inner}</g>`
        : `<text x='12' y='18' text-anchor='middle' ` +
          `font-family='-apple-system, system-ui, sans-serif' ` +
          `font-size='20' font-weight='800' fill='${iconColor}'>` +
          `${inner}</text>`;
    const iconScale = shade.iconScale;
    const off = (12 * (1 - iconScale)).toFixed(2);
    const stops = shade.specularOn
      ? `<stop offset='0' stop-color='${specular}'/>` +
        `<stop offset='${shade.specularSize}' stop-color='${highlight}'/>` +
        `<stop offset='1' stop-color='${shadow}'/>`
      : `<stop offset='0' stop-color='${highlight}'/>` +
        `<stop offset='1' stop-color='${shadow}'/>`;
    // Observable-d3-force-style preset: a tiny solid circle, no
    // icon, no gradient. Renders here only so the tuner can A/B
    // the chan look against the reference; the stroke colour is
    // applied via the cytoscape border, not the SVG.
    if (shade.flatObservable) {
      const obs =
        `<svg xmlns='http://www.w3.org/2000/svg' width='48' height='48' ` +
        `viewBox='0 0 24 24'>` +
        `<circle cx='12' cy='12' r='12' fill='${base}'/>` +
        `</svg>`;
      return `data:image/svg+xml;utf8,${encodeURIComponent(obs)}`;
    }
    if (shade.useSphere) {
      const svg =
        `<svg xmlns='http://www.w3.org/2000/svg' width='48' height='48' ` +
        `viewBox='0 0 24 24'>` +
        `<defs>` +
        `<radialGradient id='s' cx='${shade.highlightCx}' cy='${shade.highlightCy}' ` +
        `r='${shade.gradientRadius}'>${stops}</radialGradient>` +
        `</defs>` +
        `<circle cx='12' cy='12' r='12' fill='url(#s)'/>` +
        `<g transform='translate(${off} ${off}) scale(${iconScale})'>${iconLayer}</g>` +
        `</svg>`;
      return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
    }
    // Flat comparison: solid base disc + icon, no gradient.
    const flat =
      `<svg xmlns='http://www.w3.org/2000/svg' width='48' height='48' ` +
      `viewBox='0 0 24 24'>` +
      `<circle cx='12' cy='12' r='12' fill='${base}'/>` +
      `<g transform='translate(${off} ${off}) scale(${iconScale})'>${iconLayer}</g>` +
      `</svg>`;
    return `data:image/svg+xml;utf8,${encodeURIComponent(flat)}`;
  }

  // ---- stylesheet (mirrors GraphPanel's sphere path) --------------------

  function readThemeColors(host: HTMLElement) {
    const cs = getComputedStyle(host);
    const v = (n: string, fb: string) => cs.getPropertyValue(n).trim() || fb;
    return {
      doc: v("--g-doc", "#ff8a3d"),
      img: v("--g-img", "#b07dff"),
      tag: v("--g-tag", "#6cd07a"),
      mention: v("--warn-text", "#e3b341"),
      text: v("--text", "#f0e6d8"),
      textSec: v("--text-secondary", "#8b7e6e"),
      bg: v("--bg", "#0e0e0e"),
      bgCard: v("--bg-card", "#181614"),
    };
  }

  function buildStylesheet(host: HTMLElement): cytoscape.StylesheetJson {
    const c = readThemeColors(host);
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
    const ICON_DOC = svgSphereIcon(c.doc, PATH_DOC, "stroke", c.bg);
    const ICON_IMG = svgSphereIcon(c.img, PATH_IMG, "stroke", c.bg);
    const ICON_TAG = svgSphereIcon(c.tag, "#", "text", c.bg);
    const ICON_MENTION = svgSphereIcon(c.mention, "@", "text", c.bg);
    const ICON_CONTACT = svgSphereIcon(c.mention, PATH_CONTACT, "stroke", c.bg);
    // The Observable d3 example uses a thin near-white stroke
    // around each disc. Pin the colour to the page text colour
    // when that preset is on so it tracks the theme; otherwise
    // fall through to the shade.borderMode selector below.
    const borderColor = shade.flatObservable
      ? c.text
      : shade.borderMode === "page-bg"
        ? c.bg
        : shade.borderMode === "shaded"
          ? shadeColor(c.doc, shade.shadowDelta)
          : "transparent";
    const borderWidth = shade.flatObservable
      ? 1.0
      : shade.borderMode === "none"
        ? 0
        : shade.borderWidth;
    return [
      {
        selector: "node",
        style: {
          shape: "ellipse",
          width: shade.nodeBase,
          height: shade.nodeBase,
          label: shade.labels ? "data(label)" : "",
          color: c.text,
          "font-family":
            '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
          "font-size": 7,
          "text-halign": "center",
          "text-valign": "top",
          "text-margin-y": -3,
          "text-wrap": "ellipsis",
          "text-max-width": "110px",
          "text-outline-color": c.bg,
          "text-outline-width": 1,
          "border-width": borderWidth,
          "border-color": borderColor,
          "min-zoomed-font-size": 6,
          "background-fit": "cover",
          "background-clip": "node",
          "background-width": "100%",
          "background-height": "100%",
          "background-position-x": "50%",
          "background-position-y": "50%",
          "background-color": c.bg,
        },
      },
      {
        selector: 'node[kind = "doc"]',
        style: {
          width: shade.nodeBase * shade.nodeRatio,
          height: shade.nodeBase * shade.nodeRatio,
          "background-image": ICON_DOC,
          "border-color":
            shade.borderMode === "shaded"
              ? shadeColor(c.doc, shade.shadowDelta)
              : borderColor,
        },
      },
      {
        selector: 'node[kind = "img"]',
        style: {
          "background-image": ICON_IMG,
          "border-color":
            shade.borderMode === "shaded"
              ? shadeColor(c.img, shade.shadowDelta)
              : borderColor,
        },
      },
      {
        selector: 'node[kind = "tag"]',
        style: {
          "background-image": ICON_TAG,
          "border-color":
            shade.borderMode === "shaded"
              ? shadeColor(c.tag, shade.shadowDelta)
              : borderColor,
        },
      },
      {
        selector: 'node[kind = "mention"]',
        style: {
          "background-image": ICON_MENTION,
          "border-color":
            shade.borderMode === "shaded"
              ? shadeColor(c.mention, shade.shadowDelta)
              : borderColor,
        },
      },
      {
        selector: 'node[kind = "contact"]',
        style: {
          "background-image": ICON_CONTACT,
          "border-color":
            shade.borderMode === "shaded"
              ? shadeColor(c.mention, shade.shadowDelta)
              : borderColor,
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
    ];
  }

  // ---- graph build + force run (light copy of GraphPanel) ---------------

  function classifyFile(path: string): "doc" | "img" {
    return /\.(png|jpe?g|gif|webp|svg|avif|bmp)$/i.test(path) ? "img" : "doc";
  }

  function buildElements(g: GraphView): ElementDefinition[] {
    const keepKinds = new Set(["file", "tag", "mention"]);
    const keep = new Set<string>();
    for (const n of g.nodes) if (keepKinds.has(n.kind)) keep.add(n.id);
    const els: ElementDefinition[] = [];
    for (const n of g.nodes) {
      if (!keep.has(n.id)) continue;
      const data: Record<string, unknown> = { id: n.id, label: n.label };
      if (n.kind === "file") {
        data.kind = classifyFile(n.path);
        data.path = n.path;
      } else if (n.kind === "mention") {
        // Treat mention-shaped fake nodes as contacts so the
        // tuner shows the person icon (closer to what the user
        // sees in their real drive than the bare @ glyph).
        data.kind = "contact";
      } else {
        data.kind = "tag";
      }
      els.push({ group: "nodes", data });
    }
    const seen = new Map<string, number>();
    for (const e of g.edges) {
      if (!keep.has(e.source) || !keep.has(e.target)) continue;
      if (e.kind !== "link" && e.kind !== "tag" && e.kind !== "mention") continue;
      const base = `${e.source}|${e.target}|${e.kind}`;
      const n = (seen.get(base) ?? 0) + 1;
      seen.set(base, n);
      const id = n === 1 ? base : `${base}#${n}`;
      els.push({
        group: "edges",
        data: { id, source: e.source, target: e.target, kind: e.kind },
      });
    }
    return els;
  }

  function rebuild(): void {
    if (!containerEl) return;
    forceLayout?.stop();
    forceLayout = null;
    cy?.destroy();

    const g = makeFakeGraph(spec);
    const elements = buildElements(g);
    stats = {
      nodes: elements.filter((e) => e.group === "nodes").length,
      edges: elements.filter((e) => e.group === "edges").length,
    };
    cy = cytoscape({
      container: containerEl,
      elements,
      style: buildStylesheet(containerEl),
      minZoom: 0.15,
      maxZoom: 6,
      boxSelectionEnabled: false,
      selectionType: "single",
    });
    const fc = cy.layout({
      name: "fcose",
      quality: "default",
      randomize: true,
      animate: false,
      fit: false,
      padding: 30,
      nodeSeparation: 140,
      idealEdgeLength: (e: cytoscape.EdgeSingular) =>
        e.data("kind") === "link" ? 70 : 40,
      edgeElasticity: 0.45,
      nestingFactor: 0.1,
      numIter: 2500,
      gravity: 0.25,
      packComponents: true,
    } as cytoscape.LayoutOptions);
    fc.one("layoutstop", () => {
      requestAnimationFrame(() => {
        if (!cy) return;
        cy.resize();
        const vis = cy.elements();
        if (vis.nonempty()) cy.fit(vis, 30);
        forceLayout = cy.layout({
          name: "d3-force",
          animate: true,
          fit: false,
          randomize: false,
          infinite: true,
          ungrabifyWhileSimulating: false,
          fixedAfterDragging: true,
          alpha: 1,
          alphaMin: 0.05,
          alphaDecay: 1 - Math.pow(0.05, 1 / 60),
          alphaTarget: 0,
          velocityDecay: 0.6,
          collideRadius: (n: { kind?: string }) => {
            if (n.kind === "doc") return 30;
            if (n.kind === "tag") return 20;
            return 18;
          },
          collideStrength: 0.95,
          collideIterations: 3,
          manyBodyStrength: -50,
          linkDistance: (e: { kind?: string }) =>
            e.kind === "link" ? 110 : 60,
          linkStrength: 0.55,
          linkId: (n: { id: string }) => n.id,
          xStrength: 0.05,
          yStrength: 0.05,
        } as unknown as cytoscape.LayoutOptions);
        forceLayout.run();
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const sim = (forceLayout as any).simulation;
        if (sim) sim.force("center", null);
        cy.nodes().on("free unlock", () => {
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          const s = (forceLayout as any)?.simulation;
          if (s) s.alphaTarget(0);
        });
      });
    });
    fc.run();
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

  // Shading sliders: hot-patch the stylesheet without rebuilding the
  // graph so positions are preserved. Touch every field so Svelte
  // tracks the effect against every slider individually.
  $effect(() => {
    void shade.highlightDelta;
    void shade.shadowDelta;
    void shade.highlightCx;
    void shade.highlightCy;
    void shade.gradientRadius;
    void shade.specularOn;
    void shade.specularIntensity;
    void shade.specularSize;
    void shade.iconScale;
    void shade.borderMode;
    void shade.borderWidth;
    void shade.nodeBase;
    void shade.nodeRatio;
    void shade.useSphere;
    void shade.flatObservable;
    void shade.labels;
    void theme;
    if (!cy || !containerEl) return;
    cy.style(buildStylesheet(containerEl));
  });

  $effect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  });

  onDestroy(() => {
    resizeObs?.disconnect();
    forceLayout?.stop();
    cy?.destroy();
    cy = null;
  });

  // ---- isolated single-sphere previews ---------------------------------
  //
  // Hand-built data URLs the right preview pane renders large so the
  // user can see the highlight / shadow / specular detail without
  // squinting at 26px graph nodes. Tracks the current shade.* state
  // through the buildStylesheet path.
  const PATH_DOC =
    `<path d='M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z'/>` +
    `<polyline points='14 3 14 8 19 8'/>`;
  const previewBase = $derived(theme === "dark" ? "#ff8a3d" : "#c25a1f");
  const previewBg = $derived(theme === "dark" ? "#0e0e0e" : "#ffffff");
  const previewSvg = $derived(svgSphereIcon(previewBase, PATH_DOC, "stroke", previewBg));

  // Snippet of paste-back code for GraphPanel.svelte. Mirrors the
  // exact field names so the user can search-replace in the
  // production svgSphereIcon body.
  function copySnippet(): void {
    const text =
      `// paste into svgSphereIcon in components/GraphPanel.svelte\n` +
      `const highlight = shadeColor(opts.base, ${shade.highlightDelta.toFixed(2)});\n` +
      `const shadow = shadeColor(opts.base, ${shade.shadowDelta.toFixed(2)});\n` +
      (shade.specularOn
        ? `const specular = shadeColor("#ffffff", -(1 - ${shade.specularIntensity.toFixed(2)}));\n`
        : `// specular: off\n`) +
      `const ICON_SCALE = ${shade.iconScale.toFixed(2)};\n` +
      `// radialGradient cx='${shade.highlightCx}' cy='${shade.highlightCy}' ` +
      `r='${shade.gradientRadius}'\n` +
      (shade.specularOn
        ? `// stops: 0:specular  ${shade.specularSize}:highlight  1:shadow\n`
        : `// stops: 0:highlight  1:shadow\n`) +
      `// border: ${shade.borderMode} ${shade.borderWidth}px\n`;
    navigator.clipboard.writeText(text).catch(() => {
      // Clipboard may be denied in some browser contexts; the
      // textarea below is the fallback (the snippet shows live
      // anyway so the user can copy by hand).
    });
  }
</script>

<div class="page">
  <aside class="controls">
    <h3>shade</h3>
    <label>highlight + <span class="v">{shade.highlightDelta.toFixed(2)}</span>
      <input type="range" min="0" max="1" step="0.01" bind:value={shade.highlightDelta} />
    </label>
    <label>shadow - <span class="v">{shade.shadowDelta.toFixed(2)}</span>
      <input type="range" min="-1" max="0" step="0.01" bind:value={shade.shadowDelta} />
    </label>
    <label>cx <span class="v">{shade.highlightCx.toFixed(2)}</span>
      <input type="range" min="0" max="1" step="0.01" bind:value={shade.highlightCx} />
    </label>
    <label>cy <span class="v">{shade.highlightCy.toFixed(2)}</span>
      <input type="range" min="0" max="1" step="0.01" bind:value={shade.highlightCy} />
    </label>
    <label>radius <span class="v">{shade.gradientRadius.toFixed(2)}</span>
      <input type="range" min="0.3" max="1.6" step="0.01" bind:value={shade.gradientRadius} />
    </label>

    <h3>specular</h3>
    <label class="row">
      <input type="checkbox" bind:checked={shade.specularOn} />
      add specular hotspot
    </label>
    <label>intensity <span class="v">{shade.specularIntensity.toFixed(2)}</span>
      <input type="range" min="0" max="1" step="0.01" bind:value={shade.specularIntensity}
        disabled={!shade.specularOn} />
    </label>
    <label>size <span class="v">{shade.specularSize.toFixed(2)}</span>
      <input type="range" min="0.02" max="0.4" step="0.01" bind:value={shade.specularSize}
        disabled={!shade.specularOn} />
    </label>

    <h3>icon + border</h3>
    <label>icon scale <span class="v">{shade.iconScale.toFixed(2)}</span>
      <input type="range" min="0" max="0.9" step="0.01" bind:value={shade.iconScale} />
    </label>
    <div class="row">
      <span>border:</span>
      <button class:on={shade.borderMode === "page-bg"}
        onclick={() => (shade.borderMode = "page-bg")}>page-bg</button>
      <button class:on={shade.borderMode === "shaded"}
        onclick={() => (shade.borderMode = "shaded")}>shaded</button>
      <button class:on={shade.borderMode === "none"}
        onclick={() => (shade.borderMode = "none")}>none</button>
    </div>
    <label>border width <span class="v">{shade.borderWidth.toFixed(1)}</span>
      <input type="range" min="0" max="3" step="0.1" bind:value={shade.borderWidth}
        disabled={shade.borderMode === "none"} />
    </label>

    <h3>node size</h3>
    <label>base <span class="v">{shade.nodeBase}</span>
      <input type="range" min="14" max="48" step="1" bind:value={shade.nodeBase} />
    </label>
    <label>doc ratio <span class="v">{shade.nodeRatio.toFixed(2)}</span>
      <input type="range" min="1.0" max="1.8" step="0.05" bind:value={shade.nodeRatio} />
    </label>

    <h3>compare</h3>
    <label class="row">
      <input type="checkbox" bind:checked={shade.useSphere}
        disabled={shade.flatObservable} />
      sphere shading (off = flat disc baseline)
    </label>
    <label class="row">
      <input type="checkbox" bind:checked={shade.flatObservable} />
      Observable d3-force preset (flat + thin white ring)
    </label>
    <label class="row">
      <input type="checkbox" bind:checked={shade.labels} />
      show labels
    </label>

    <h3>theme</h3>
    <div class="row">
      <button class:on={theme === "dark"} onclick={() => (theme = "dark")}>dark</button>
      <button class:on={theme === "light"} onclick={() => (theme = "light")}>light</button>
    </div>

    <h3>graph</h3>
    <label>seed <span class="v">{spec.seed}</span>
      <input type="number" bind:value={spec.seed} />
    </label>
    <button onclick={regen}>regen graph</button>
    <button onclick={() => rebuild()}>rebuild</button>

    <h3>preview</h3>
    <div class="preview">
      <div class="swatch" style="background: {previewBg};">
        <img src={previewSvg} alt="" width="160" height="160" />
      </div>
      <p class="hint">single sphere @ 160px — the same SVG the graph nodes render at ~26px.</p>
    </div>

    <button class="copy" onclick={copySnippet}>copy values</button>
  </aside>

  <main class="canvas-wrap">
    <div bind:this={containerEl} class="cy"></div>
    <div class="status">
      {stats.nodes} nodes · {stats.edges} edges · drag a node to nudge ·
      scroll to zoom
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
  :global(html[data-theme="dark"]) {
    --bg: #0e0e0e;
    --bg-card: #181614;
    --bg-elev: #1f1c19;
    --border: #2c2724;
    --text: #f0e6d8;
    --text-secondary: #8b7e6e;
    --warn-text: #e3b341;
    --g-doc: #ff8a3d;
    --g-img: #b07dff;
    --g-tag: #6cd07a;
  }
  :global(html[data-theme="light"]) {
    --bg: #ffffff;
    --bg-card: #f5f5f7;
    --bg-elev: #ffffff;
    --border: #d1d1d6;
    --text: #1c1c1e;
    --text-secondary: #6c6c70;
    --warn-text: #9a6700;
    --g-doc: #c25a1f;
    --g-img: #7a4cd8;
    --g-tag: #2f9444;
  }
  .page {
    display: flex;
    height: 100vh;
    width: 100vw;
  }
  .controls {
    width: 290px;
    flex: none;
    padding: 14px 14px 24px;
    background: var(--bg-card);
    border-right: 1px solid var(--border);
    overflow: auto;
    font-size: 12.5px;
  }
  .controls h3 {
    margin: 14px 0 6px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
  }
  .controls h3:first-of-type { margin-top: 0; }
  .controls label {
    display: block;
    margin: 4px 0 8px;
  }
  .controls .v {
    float: right;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
  }
  .controls input[type="range"] {
    width: 100%;
    margin: 2px 0 0;
  }
  .controls input[type="number"] {
    width: 70px;
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 4px;
  }
  .controls button {
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 10px;
    font: inherit;
    cursor: pointer;
    margin: 2px 4px 2px 0;
  }
  .controls button.on {
    background: var(--g-doc);
    color: #1c1c1e;
    border-color: var(--g-doc);
  }
  .controls .row {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    margin: 4px 0 8px;
  }
  .controls .copy {
    margin-top: 12px;
    width: 100%;
  }
  .preview .swatch {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .preview .hint {
    margin: 6px 0 0;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.4;
  }
  .canvas-wrap {
    flex: 1;
    position: relative;
  }
  .cy {
    position: absolute;
    inset: 0;
  }
  .status {
    position: absolute;
    left: 12px;
    bottom: 10px;
    color: var(--text-secondary);
    font-size: 11px;
    pointer-events: none;
  }
</style>
