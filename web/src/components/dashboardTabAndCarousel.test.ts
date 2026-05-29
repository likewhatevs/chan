import { describe, expect, test } from "vitest";
import tabs from "../state/tabs.svelte.ts?raw";
import pane from "./Pane.svelte?raw";
import carousel from "./EmptyPaneCarousel.svelte?raw";
import dashboard from "./DashboardTab.svelte?raw";
import app from "../App.svelte?raw";
import shell from "./HybridSurfaceConfigShell.svelte?raw";

// `fullstack-a-75`: Dashboard tab kind + carousel redesign.
// Tests pin: new tab type + helpers, the Pane.svelte render
// branch, the carousel's spawn band changes (New Draft slot 0,
// shortcut table dropped, Dashboard secondary band), and the
// surface unification across the three menus (pane hamburger,
// empty-pane right-click, carousel).
//
// `phase-13 lane-b`: Infographics -> Dashboard rename. The string
// discriminator + helper names + component file all moved to
// "dashboard"; round-1 closing slice flipped the user-visible
// labels too (B5/B6: menu entry, shortcut label, tab title,
// aria-label, settings shell title).

describe("fullstack-a-75: DashboardTab type + helpers", () => {
  test("Tab union includes DashboardTab", () => {
    expect(tabs).toMatch(
      /export type DashboardTab = \{[\s\S]{1,400}kind: "dashboard";[\s\S]{1,200}id: string;[\s\S]{1,200}title: string;/,
    );
    expect(tabs).toMatch(
      /export type Tab =\s*\n\s*\| FileTab[\s\S]{1,400}\| DashboardTab;/,
    );
  });

  test("openDashboardInPane appends a Dashboard tab + activates it", () => {
    expect(tabs).toMatch(
      /export function openDashboardInPane\(paneId: string\): void \{[\s\S]{1,800}kind: "dashboard",[\s\S]{1,400}node\.tabs\.push\(tab\);[\s\S]{1,200}node\.activeTabId = tab\.id;/,
    );
  });

  test("openDashboardInActivePane delegates to openDashboardInPane(layout.activePaneId)", () => {
    expect(tabs).toMatch(
      /export function openDashboardInActivePane\(\): void \{[\s\S]{1,200}openDashboardInPane\(layout\.activePaneId\);/,
    );
  });

  test("tabLabel handles dashboard kind", () => {
    expect(tabs).toMatch(
      /export function tabLabel\(t: Tab, ctx\?: BrowserLabelCtx\): string \{[\s\S]{1,800}if \(t\.kind === "dashboard"\) return t\.title;/,
    );
  });

  test("serializer emits k:\"d\" for dashboard tabs", () => {
    expect(tabs).toMatch(
      /if \(t\.kind === "dashboard"\) \{[\s\S]{1,200}k: "d",/,
    );
  });

  test("SerTab kind discriminator includes \"d\"", () => {
    expect(tabs).toMatch(
      /k\?: "f" \| "b" \| "s" \| "g" \| "h" \| "t" \| "d";/,
    );
  });
});

describe("fullstack-a-75: Pane.svelte render branch + import", () => {
  test("DashboardTab imported", () => {
    expect(pane).toMatch(
      /import DashboardTab from "\.\/DashboardTab\.svelte";/,
    );
  });

  test("render branch matches active?.kind === \"dashboard\" and passes the live tab", () => {
    // Round-1 closing-10 (G3): Pane.svelte now threads the live
    // DashboardTab proxy through so the carousel slide cursor can
    // round-trip back into tabs.svelte.ts's session serializer.
    expect(pane).toMatch(
      /\{:else if active\?\.kind === "dashboard"\}[\s\S]{1,200}<DashboardTab tab=\{active\} \/>/,
    );
  });
});

describe("fullstack-a-75: Dashboard command + emptyPaneExtraActions wiring", () => {
  test("app.dashboard.open command routed to openDashboardInActivePane", () => {
    expect(app).toMatch(
      /case "app\.dashboard\.open":[\s\S]{1,400}openDashboardInActivePane\(\);/,
    );
  });

  test("spawnActions carries the Dashboard entry after Graph + Search (B5 + B8)", () => {
    // Round-1 closing-2 (B8) folded the separate
    // `emptyPaneExtraActions` list into `spawnActions` so the
    // pane top-bar hamburger and the empty-pane right-click
    // menu both render the same 7-entry spawn set in the same
    // order: ..., Graph, Search, Dashboard. The user's quoted
    // ask required Search + Dashboard to surface in the
    // hamburger menu too, which the prior split list blocked.
    expect(pane).toMatch(
      /const spawnActions:[\s\S]{1,2000}label: "Graph",[\s\S]{1,400}command: "app\.graph\.toggle",[\s\S]{1,400}label: "Search",[\s\S]{1,400}command: "app\.search\.toggle",[\s\S]{1,400}label: "Dashboard",[\s\S]{1,400}command: "app\.dashboard\.open",/,
    );
    expect(pane).not.toMatch(/const emptyPaneExtraActions:/);
  });
});

describe("fullstack-a-75: carousel slide 1 redesign", () => {
  // `fullstack-a-75b`: spawn entries + secondary band moved
  // OUT of the carousel and into EmptyPaneWelcome.svelte. The
  // carousel is now a pure rotating widget hosted inside the
  // Dashboard tab.
  test("spawn entries no longer surface in the carousel", () => {
    expect(carousel).not.toMatch(/const spawnEntries: SpawnRow\[\]/);
    expect(carousel).not.toMatch(/const secondaryEntries: SpawnRow\[\]/);
    expect(carousel).not.toMatch(/function dispatchSpawn\(/);
  });

  test("welcome chrome (logo / dashboard / spawn-row) dropped from carousel markup", () => {
    expect(carousel).not.toMatch(/class="placeholder-mark"/);
    expect(carousel).not.toMatch(/class="dashboard-header"/);
    expect(carousel).not.toMatch(/<div class="spawn-row"/);
  });
});

describe("phase-13 slice 3b-1: carousel slide rework", () => {
  // Slide 0 is now the About widget (version + embeddings flag +
  // attributions + donation QR + chan.app/source links). Slice 3c
  // retired the global Settings overlay; the carousel About widget
  // is the sole home for the version/attribution surface now.
  test("slide 0 is the About widget", () => {
    expect(carousel).toMatch(
      /<div class="slide slide-about" aria-label="About">/,
    );
    expect(carousel).toMatch(/chan version/);
    expect(carousel).toMatch(/embeddings/);
    expect(carousel).toMatch(/Source Code Pro Regular/);
    expect(carousel).toMatch(/dcragusa\/MatrixScreensaver/);
    // Round-1 closing-3 (C2): license links resolve to canonical
    // upstream URLs instead of the embedded `/static/...` paths,
    // which under chan-desktop's non-root mount surfaced as
    // 127.0.0.1 links. The font lives in the adobe-fonts
    // source-code-pro repo + the screen-lock in dcragusa's repo.
    expect(carousel).toMatch(
      /href="https:\/\/github\.com\/adobe-fonts\/source-code-pro\/blob\/release\/LICENSE\.md"/,
    );
    expect(carousel).toMatch(
      /href="https:\/\/github\.com\/dcragusa\/MatrixScreensaver\/blob\/master\/LICENSE"/,
    );
    expect(carousel).not.toMatch(/href="\/static\/fonts\/OFL\.txt"/);
    expect(carousel).not.toMatch(/href="\/static\/matrix\/LICENSE-MatrixScreensaver\.txt"/);
  });

  test("About widget loads buildInfo from the typed API", () => {
    expect(carousel).toMatch(
      /let buildInfo = \$state<BuildInfo \| null>\(null\)/,
    );
    expect(carousel).toMatch(/buildInfo = await api\.buildInfo\(\)/);
  });

  test("About widget embeds the donation QR + Fund-the-work copy", () => {
    // B4c: src is now wrapped in `withTokenQuery("/qr-donate.png")`
    // so the prefix rewrite + per-launch bearer token apply under
    // chan-desktop's non-root mount and the tunnel-mode prefix.
    // A raw `<img src="/qr-donate.png">` bypasses both and renders
    // a broken-image square. Match the helper wrapper, not the raw
    // path.
    expect(carousel).toMatch(/src=\{withTokenQuery\("\/qr-donate\.png"\)\}/);
    expect(carousel).toMatch(
      /import \{[\s\S]{1,200}withTokenQuery[\s\S]{1,200}\} from "\.\.\/api\/transport"/,
    );
    expect(carousel).toMatch(/Fund the work/);
    // Round-1 closing-3 (C3): "Share the love, cheers!" tail
    // appended to the Fund-the-work copy.
    expect(carousel).toMatch(
      /Chan is independent software\. Small tips help cover time[\s\S]{1,40}spent on releases, packaging, and documentation\.[\s\S]{1,40}Share the love, cheers!/,
    );
  });

  test("About widget licenses block sits after the QR + the separator (C2)", () => {
    // C2: license rows moved OUT of the top about-grid into a
    // dedicated `.about-licenses` block, separated from the
    // Fund-the-work surface by `.about-sep`. Chan's own Apache
    // 2.0 license joins the section so the three runtime
    // licenses live together.
    expect(carousel).toMatch(
      /<div class="about-fund">[\s\S]{1,2000}<div class="about-sep"[\s\S]{1,200}<div class="about-licenses">[\s\S]{1,3000}<a href="https:\/\/github\.com\/fiorix\/chan\/blob\/main\/LICENSE"[\s\S]{1,200}Apache 2\.0[\s\S]{1,400}Source Code Pro Regular[\s\S]{1,1200}dcragusa\/MatrixScreensaver/,
    );
    expect(carousel).toMatch(/\.about-licenses \{[\s\S]{1,400}grid-template-columns: max-content 1fr/);
    expect(carousel).toMatch(/\.about-sep \{[\s\S]{1,400}background: var\(--border\)/);
    // The terminal-font + matrix-screen-lock rows appear EXACTLY
    // ONCE in the source now (inside `.about-licenses`); pre-fix
    // they ALSO appeared inside `.about-grid`. Asserting a single
    // occurrence prevents the prior dual-render from sneaking
    // back.
    const fontMatches = carousel.match(/<span class="k">terminal font<\/span>/g);
    expect(fontMatches?.length ?? 0).toBe(1);
    const matrixMatches = carousel.match(/<span class="k">matrix screen lock<\/span>/g);
    expect(matrixMatches?.length ?? 0).toBe(1);
  });

  test("About widget renders icon-linked website + source links", () => {
    expect(carousel).toMatch(/href="https:\/\/chan\.app"/);
    expect(carousel).toMatch(
      /href="https:\/\/github\.com\/fiorix\/chan"/,
    );
    expect(carousel).toMatch(
      /import \{[\s\S]{1,300}Code2,[\s\S]{1,300}Globe,[\s\S]{1,200}\} from "lucide-svelte"/,
    );
  });

  test("slide 1 mounts WorkspaceInfoBody", () => {
    expect(carousel).toMatch(
      /import WorkspaceInfoBody from "\.\/WorkspaceInfoBody\.svelte";/,
    );
    // A1 (phase-13): the Dashboard slide passes variant="dashboard"
    // so the workspace-root inspector keeps its Notes-directories
    // config (the inspector variant drops it).
    expect(carousel).toMatch(
      /<div class="slide slide-workspace" aria-label="Workspace info">[\s\S]{1,400}<WorkspaceInfoBody[\s\S]{1,200}variant="dashboard"/,
    );
  });

  test("Shortcuts slide + workspace-metadata slide are retired", () => {
    expect(carousel).not.toMatch(/class="slide slide-shortcuts"/);
    expect(carousel).not.toMatch(/class="slide slide-metadata"/);
    expect(carousel).not.toMatch(/<pre class="shortcuts-table">/);
    expect(carousel).not.toMatch(/renderTable\(platform, os\)/);
    expect(carousel).not.toMatch(/from "\.\.\/state\/shortcuts"/);
  });

  test("slide 2 stays the indexing graph and flags the slice 3b-2 deferral", () => {
    expect(carousel).toMatch(/class="slide slide-indexing"/);
    expect(carousel).toMatch(/slice 3b-2/);
  });

  test("indexing slide maximises to the tab width/height with a 10px border (Bug 2)", () => {
    // Round-1 closing-3 (Bug 2): the About + Workspace slides
    // are text-shaped and read better in the centered 720px
    // column; the indexing graph wants the full tab area so the
    // spine doesn't compress to a vertical band. The wide-stage
    // class is toggled only on slideIndex === 2 + drops the
    // `max-width: 720px` cap, and the carousel-wide variant
    // tightens the outer padding to ~10px so the canvas reads
    // edge-to-edge with a reasonable breathing border.
    expect(carousel).toMatch(
      /class="slide-stage" class:slide-stage-wide=\{slideIndex === 2\}/,
    );
    expect(carousel).toMatch(
      /class="carousel"\s*\n\s*class:carousel-wide=\{slideIndex === 2\}/,
    );
    expect(carousel).toMatch(
      /\.slide-stage-wide \{[\s\S]{1,200}max-width: none;/,
    );
    expect(carousel).toMatch(
      /\.carousel-wide \{[\s\S]{1,200}padding: 10px;/,
    );
  });

  test("indexing slide tracks a selectedIndexId so GraphCanvas labels selection + 1-hop (B12)", () => {
    // Phase-13 round-1 closing: clicks on the indexing graph
    // now update a `selectedIndexId` $state and feed it into
    // GraphCanvas.selectedId. GraphCanvas already labels the
    // selected node + 1-hop neighbours, so this is the wiring
    // change that surfaces the labels on the read-only spine.
    expect(carousel).toMatch(
      /let selectedIndexId = \$state<string \| null>\(null\);/,
    );
    expect(carousel).toMatch(
      /function onIndexingSelect\(id: string \| null\): void \{[\s\S]{1,200}selectedIndexId = id;/,
    );
    expect(carousel).toMatch(
      /<GraphCanvas[\s\S]{1,800}selectedId=\{selectedIndexId\}[\s\S]{1,200}onSelect=\{onIndexingSelect\}/,
    );
  });

  test("slide-stage scroll lives at the slide level for carousel resize", () => {
    expect(carousel).toMatch(/\.slide\s*\{[\s\S]{1,500}overflow-y: auto/);
    expect(carousel).toMatch(/\.carousel\s*\{[\s\S]{1,400}min-height: 0/);
  });
});

describe("fullstack-a-75b: DashboardTab mounts the carousel", () => {
  test("DashboardTab imports + mounts EmptyPaneCarousel + threads tab.carouselSlide (G3)", () => {
    expect(dashboard).toMatch(
      /import EmptyPaneCarousel from "\.\/EmptyPaneCarousel\.svelte";/,
    );
    // Round-1 closing-10 (G3): DashboardTab passes the persisted
    // slide cursor + a write-back callback so the carousel
    // position survives a window reload.
    expect(dashboard).toMatch(
      /<EmptyPaneCarousel[\s\S]{1,400}initialSlide=\{tab\.carouselSlide \?\? 0\}[\s\S]{1,200}onSlideChange=\{onCarouselSlideChange\}/,
    );
    expect(dashboard).toMatch(
      /import \{[\s\S]{1,400}scheduleSessionSave[\s\S]{1,200}\} from "\.\.\/state\/store\.svelte"/,
    );
    expect(dashboard).toMatch(/type DashboardTab/);
    expect(dashboard).toMatch(
      /function onCarouselSlideChange\(i: number\): void \{[\s\S]{1,400}tab\.carouselSlide = i;[\s\S]{1,200}scheduleSessionSave\(\);/,
    );
  });

  test("static ASCII pre + Shortcuts header dropped (carousel owns the shortcut surface now)", () => {
    expect(dashboard).not.toMatch(/<pre class="info-shortcuts">/);
    expect(dashboard).not.toMatch(/renderTable\(platform, os\)/);
  });

  test("body wraps the carousel in a labeled region", () => {
    expect(dashboard).toMatch(
      /class="dashboard"[\s\S]{1,120}aria-label="Dashboard"[\s\S]{1,120}role="region"/,
    );
  });
});

describe("phase-13 round-1 closing B3: Dashboard back-of-card lives in HybridDashboardConfig", () => {
  test("DashboardTab right-click menu carries only Reload (Settings entry retired)", () => {
    // After B3 the redundant local `settingsOpen` path is gone;
    // Pane.svelte's back-side switch mounts HybridDashboardConfig
    // directly via the `active?.kind === "dashboard"` arm, and
    // Cmd+, is the canonical flip. The right-click menu keeps a
    // Reload row so the affordance is still discoverable from
    // the body.
    expect(dashboard).toMatch(/import HamburgerMenu from "\.\/HamburgerMenu\.svelte";/);
    expect(dashboard).toMatch(/function onContextMenu\(e: MouseEvent\): void/);
    expect(dashboard).toMatch(/menu\?\.openAtCursor\(e\.clientX, e\.clientY\)/);
    expect(dashboard).toMatch(
      /import \{[^}]*\bRefreshCw\b[^}]*\} from "lucide-svelte"/,
    );
    expect(dashboard).toMatch(
      /import \{\s*reloadWindow\s*\} from "\.\.\/api\/desktop";/,
    );
    expect(dashboard).toMatch(/async function doReload\(\): Promise<void>/);
    expect(dashboard).toMatch(/await reloadWindow\(\)/);
    expect(dashboard).toMatch(
      /onclick=\{doReload\}[\s\S]{1,200}<RefreshCw[\s\S]{1,200}<span class="menu-row-label">Reload<\/span>[\s\S]{1,160}<span class="menu-row-chord">\{chordLabel\("app\.window\.reload"\)\}<\/span>/,
    );
    // Settings entry + its supporting state retired in B3.
    expect(dashboard).not.toMatch(/<span class="menu-row-label">Settings<\/span>/);
    expect(dashboard).not.toMatch(/let settingsOpen = \$state/);
    expect(dashboard).not.toMatch(/function openSettings\b/);
    expect(dashboard).not.toMatch(/function closeSettings\b/);
    expect(dashboard).not.toMatch(/import HybridSurfaceConfigShell/);
  });

  test("HybridDashboardConfig mirrors the other Hybrid configs and lives at its own file", async () => {
    const cfg = (await import("./HybridDashboardConfig.svelte?raw"))
      .default as string;
    // Shell wrapper carries the Dashboard title + onDone prop
    // wiring + the surface=\"dashboard\" tag (same shape as the
    // Terminal / Editor / Graph / FB configs).
    expect(cfg).toMatch(
      /<HybridSurfaceConfigShell[\s\S]{1,400}title="Dashboard"[\s\S]{1,200}surface="dashboard"[\s\S]{1,400}ariaLabel="Dashboard settings"[\s\S]{1,200}\{onDone\}/,
    );
    expect(cfg).toMatch(
      /let \{ onDone \}: \{ onDone\?: \(\) => void \} = \$props\(\);/,
    );
    // Three sections after B3c: Appearance / Screen lock /
    // Metadata archive. The Screensaver section was collapsed
    // INTO the Screen lock enable gate, so its theme picker
    // shares the lifecycle of the Screen lock toggle and no
    // longer carries a standalone `<h3>Screensaver</h3>`.
    expect(cfg).toMatch(/<h3>Appearance<\/h3>/);
    expect(cfg).toMatch(/<h3>Screen lock<\/h3>/);
    expect(cfg).not.toMatch(/<h3>Screensaver<\/h3>/);
    expect(cfg).toMatch(/<h3>Metadata archive<\/h3>/);
    // App-wide appearance radio group keeps the `app-appearance`
    // name so the rejected per-tab DashboardAppearance enum
    // can't sneak back in via the same name.
    expect(cfg).toMatch(/name="app-appearance"/);
    expect(cfg).not.toMatch(/name="dashboard-appearance"/);
    // Metadata archive surfaces the typed API + the same
    // labels + the rescan / force-SCM checkboxes the retired
    // Settings overlay used to ship.
    expect(cfg).toMatch(/await api\.metadataExport\(\)/);
    expect(cfg).toMatch(/await api\.metadataImport\(metadataImportFile/);
    expect(cfg).toMatch(/URL\.createObjectURL\(download\.blob\)/);
    expect(cfg).toContain("Export metadata archive");
    expect(cfg).toContain("Import metadata archive");
    expect(cfg).toContain("Force SCM mismatch");
    expect(cfg).toContain("Rescan after import");
    // Screen-lock state hydration runs on mount so the back
    // surface reads the server's current screensaver config
    // each time the user flips into it.
    expect(cfg).toMatch(
      /onMount\(\(\) => \{[\s\S]{1,200}void loadScreenLockState\(\);/,
    );
    // The shared shell still owns the OK button.
    expect(shell).toMatch(
      /<button type="button" class="config-ok" onclick=\{\(\) => onDone\?\.\(\)\}>OK<\/button>/,
    );
  });

  test("Pane.svelte back-side switch mounts HybridDashboardConfig on the dashboard arm", () => {
    expect(pane).toMatch(
      /import HybridDashboardConfig from "\.\/HybridDashboardConfig\.svelte";/,
    );
    expect(pane).toMatch(
      /\{:else if active\?\.kind === "dashboard"\}[\s\S]{1,600}<HybridDashboardConfig onDone=\{\(\) => flipHybrid\(pane\.id\)\} \/>/,
    );
  });
});

describe("fullstack-a-75b: EmptyPaneWelcome static spawn surface", () => {
  test("EmptyPaneWelcome.svelte renders the 5-tile spawn grid + Dashboard tile (per -a-95: welcome-hint dropped)", async () => {
    const welcome = (await import("./EmptyPaneWelcome.svelte?raw"))
      .default as string;
    expect(welcome).toMatch(
      /const spawnEntries: SpawnRow\[\] = \[[\s\S]{1,200}label: "New Draft",[\s\S]{1,1000}label: "Terminal",[\s\S]{1,800}label: "File Browser",[\s\S]{1,800}label: "Team Work",[\s\S]{1,800}label: "Graph",/,
    );
    // B9: secondary tile row now carries Search + Dashboard
    // (Search first), and both render their chord hints via
    // `chordLabel(row.chordId)` rather than a hardcoded empty
    // `<span class="spawn-chord"></span>`. The row CSS expands
    // to a 2-column grid.
    expect(welcome).toMatch(
      /const secondaryEntries: SpawnRow\[\] = \[[\s\S]{1,800}label: "Search",[\s\S]{1,200}command: "app\.search\.toggle",[\s\S]{1,800}label: "Dashboard",[\s\S]{1,200}command: "app\.dashboard\.open",/,
    );
    expect(welcome).toMatch(
      /import \{[\s\S]{1,200}\bSearch\b[\s\S]{1,200}\} from "lucide-svelte"/,
    );
    // Secondary tile chord render mirrors the primary row.
    expect(welcome).toMatch(
      /spawn-row spawn-row-secondary[\s\S]{1,1000}<span class="spawn-chord">\{chordLabel\(row\.chordId\)\}<\/span>/,
    );
    // The retired hardcoded empty chord span is gone.
    expect(welcome).not.toMatch(/<span class="spawn-chord"><\/span>/);
    // 2-column grid for the secondary row.
    expect(welcome).toMatch(
      /\.spawn-row-secondary \{[\s\S]{1,400}grid-template-columns: repeat\(2,/,
    );
    // `fullstack-a-95`: stale per-tab "scope for Graph" hint
    // dropped. @@Alex flagged the concept as retired after the
    // FS-backbone graph transition; picker-driven scope is the
    // active mechanism, surfaced in the graph overlay's chrome.
    expect(welcome).not.toMatch(/class="welcome-hint"/);
    expect(welcome).not.toMatch(/Each pane's visible tab is part of the scope/);
    // No `<p>` paragraph rendering the retired hint in the
    // markup (matches the user-visible surface only; the
    // retirement comment in the source is allowed).
    expect(welcome).not.toMatch(/<p[\s\S]{0,200}scope[\s\S]{0,40}for Graph/);
  });

  test("Pane.svelte mounts EmptyPaneWelcome (not EmptyPaneCarousel) on lone-pane empty case", async () => {
    const pane = (await import("./Pane.svelte?raw")).default as string;
    expect(pane).toMatch(
      /import EmptyPaneWelcome from "\.\/EmptyPaneWelcome\.svelte";/,
    );
    // Round-1 closing-2 (lane-b-empty-pane-menu): the
    // EmptyPaneWelcome mount no longer forwards
    // `oncontextmenu` — the empty-pane right-click menu was
    // retired, so the welcome surface has no parent handler to
    // forward to.
    expect(pane).toMatch(
      /\{#if !multiPane\}[\s\S]{1,800}<EmptyPaneWelcome \/>/,
    );
    expect(pane).not.toMatch(/<EmptyPaneWelcome oncontextmenu=/);
    // Pane.svelte no longer imports EmptyPaneCarousel directly
    // (it's owned by DashboardTab.svelte now).
    expect(pane).not.toMatch(
      /import EmptyPaneCarousel from "\.\/EmptyPaneCarousel\.svelte";/,
    );
  });
});
