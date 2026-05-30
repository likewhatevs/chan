import { describe, expect, test } from "vitest";
import tabs from "../state/tabs.svelte.ts?raw";
import pane from "./Pane.svelte?raw";
import carousel from "./EmptyPaneCarousel.svelte?raw";
import dashboard from "./DashboardTab.svelte?raw";
import app from "../App.svelte?raw";
import shell from "./HybridSurfaceConfigShell.svelte?raw";
import dashboardBack from "./dashboard/DashboardSlotBack.svelte?raw";
import aboutSlot from "./dashboard/AboutSlotConfig.svelte?raw";
import workspaceSlot from "./dashboard/WorkspaceSlotConfig.svelte?raw";

// Dashboard tab kind and carousel coverage. Pins the tab type and
// helpers, the Pane.svelte render branch, the carousel slide set,
// and the surface unification across the pane hamburger, empty-pane
// right-click, and carousel.

describe("DashboardTab type + helpers", () => {
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

describe("Pane.svelte render branch + import", () => {
  test("DashboardTab imported", () => {
    expect(pane).toMatch(
      /import DashboardTab from "\.\/DashboardTab\.svelte";/,
    );
  });

  test("render branch matches active?.kind === \"dashboard\" and passes the live tab", () => {
    // The live DashboardTab proxy is threaded through so the carousel
    // slide cursor can round-trip into the session serializer.
    // `frontActive={!pane.showingBack}` force-pauses the carousel while
    // the two-face card is flipped to its config back.
    expect(pane).toMatch(
      /\{:else if active\?\.kind === "dashboard"\}[\s\S]{1,400}<DashboardTab tab=\{active\} frontActive=\{!pane\.showingBack\} \/>/,
    );
  });
});

describe("Dashboard command + spawnActions wiring", () => {
  test("app.dashboard.open command routed to openDashboardInActivePane", () => {
    expect(app).toMatch(
      /case "app\.dashboard\.open":[\s\S]{1,400}openDashboardInActivePane\(\);/,
    );
  });

  test("spawnActions carries the Dashboard entry after Graph + Search", () => {
    // One spawnActions list backs both the pane hamburger and the
    // empty-pane right-click menu: ..., Graph, Search, Dashboard.
    expect(pane).toMatch(
      /const spawnActions:[\s\S]{1,2000}label: "Graph",[\s\S]{1,400}command: "app\.graph\.toggle",[\s\S]{1,400}label: "Search",[\s\S]{1,400}command: "app\.search\.toggle",[\s\S]{1,400}label: "Dashboard",[\s\S]{1,400}command: "app\.dashboard\.open",/,
    );
    expect(pane).not.toMatch(/const emptyPaneExtraActions:/);
  });
});

describe("carousel slide 1", () => {
  // Spawn entries + secondary band live in EmptyPaneWelcome.svelte,
  // not the carousel. The carousel is a pure rotating widget hosted
  // inside the Dashboard tab.
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

describe("carousel slides", () => {
  test("slide 0 is the About widget", () => {
    expect(carousel).toMatch(
      /<div class="slide slide-about" aria-label="About">/,
    );
    expect(carousel).toMatch(/chan version/);
    expect(carousel).toMatch(/embeddings/);
    expect(carousel).toMatch(/Source Code Pro Regular/);
    expect(carousel).toMatch(/dcragusa\/MatrixScreensaver/);
    // License links point to canonical upstream URLs rather than
    // embedded /static/ paths (which resolve to 127.0.0.1 under the
    // desktop non-root mount).
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
    // withTokenQuery wraps the QR image path so the bearer token and
    // prefix rewrite apply under non-root mounts; a bare path would
    // render broken.
    expect(carousel).toMatch(/src=\{withTokenQuery\("\/qr-donate\.png"\)\}/);
    expect(carousel).toMatch(
      /import \{[\s\S]{1,200}withTokenQuery[\s\S]{1,200}\} from "\.\.\/api\/transport"/,
    );
    expect(carousel).toMatch(/Fund the work/);
    // "Share the love, cheers!" tail on the Fund-the-work copy.
    expect(carousel).toMatch(
      /Chan is independent software\. Small tips help cover time[\s\S]{1,40}spent on releases, packaging, and documentation\.[\s\S]{1,40}Share the love, cheers!/,
    );
  });

  test("About widget licenses block sits after the QR + the separator", () => {
    // License rows live in `.about-licenses`, separated from the
    // Fund-the-work surface by `.about-sep`. Chan's own Apache 2.0
    // joins so all three runtime licenses are together.
    expect(carousel).toMatch(
      /<div class="about-fund">[\s\S]{1,2000}<div class="about-sep"[\s\S]{1,200}<div class="about-licenses">[\s\S]{1,3000}<a href="https:\/\/github\.com\/fiorix\/chan\/blob\/main\/LICENSE"[\s\S]{1,200}Apache 2\.0[\s\S]{1,400}Source Code Pro Regular[\s\S]{1,1200}dcragusa\/MatrixScreensaver/,
    );
    expect(carousel).toMatch(/\.about-licenses \{[\s\S]{1,400}grid-template-columns: max-content 1fr/);
    expect(carousel).toMatch(/\.about-sep \{[\s\S]{1,400}background: var\(--border\)/);
    // The terminal-font + matrix-screen-lock rows appear EXACTLY
    // ONCE in the source (inside `.about-licenses`). Asserting a
    // single occurrence prevents a dual-render (also inside
    // `.about-grid`) from sneaking in.
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
    // variant="dashboard" so the Notes-directories config renders
    // (the inspector variant drops it).
    expect(carousel).toMatch(
      /<div class="slide slide-workspace" aria-label="Workspace info">[\s\S]{1,400}<WorkspaceInfoBody[\s\S]{1,200}variant="dashboard"/,
    );
  });

  test("Shortcuts slide + workspace-metadata slide are removed", () => {
    expect(carousel).not.toMatch(/class="slide slide-shortcuts"/);
    expect(carousel).not.toMatch(/class="slide slide-metadata"/);
    expect(carousel).not.toMatch(/<pre class="shortcuts-table">/);
    expect(carousel).not.toMatch(/renderTable\(platform, os\)/);
    expect(carousel).not.toMatch(/from "\.\.\/state\/shortcuts"/);
  });

  test("slide 2 is the read-only, spine-only indexing graph", () => {
    expect(carousel).toMatch(/class="slide slide-indexing"/);
    // No inspector / scope picker / depth slider / filter chips; the
    // slide is a pure status read-out.
    expect(carousel).toMatch(/aria-label="Indexing graph"/);
  });

  test("indexing slide maximises to the tab width/height with a 10px border", () => {
    // About + Workspace read well in the 720px column; the indexing
    // graph needs the full tab area so the spine does not compress.
    // slide-stage-wide drops the max-width cap; carousel-wide tightens
    // padding to ~10px so the canvas reads edge-to-edge.
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

  test("indexing slide tracks a selectedIndexId so GraphCanvas labels selection + 1-hop", () => {
    // Clicks update selectedIndexId which feeds into GraphCanvas.selectedId,
    // labelling the selected node and its 1-hop neighbours.
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

describe("DashboardTab mounts the carousel", () => {
  test("DashboardTab imports + mounts EmptyPaneCarousel + threads tab.carouselSlide", () => {
    expect(dashboard).toMatch(
      /import EmptyPaneCarousel from "\.\/EmptyPaneCarousel\.svelte";/,
    );
    // The persisted slide cursor + write-back callback survive a reload.
    // The carousel is controlled now (`slide` prop, not a one-shot
    // `initialSlide` snapshot) so the front dots and the flip-back slot
    // picker share tab.carouselSlide as the single source of truth.
    expect(dashboard).toMatch(
      /<EmptyPaneCarousel[\s\S]{1,400}slide=\{tab\.carouselSlide \?\? 0\}[\s\S]{1,200}onSlideChange=\{onCarouselSlideChange\}/,
    );
    expect(dashboard).toMatch(
      /import \{[\s\S]{1,400}scheduleSessionSave[\s\S]{1,200}\} from "\.\.\/state\/store\.svelte"/,
    );
    expect(dashboard).toMatch(/type DashboardTab/);
    expect(dashboard).toMatch(
      /function onCarouselSlideChange\(i: number\): void \{[\s\S]{1,400}tab\.carouselSlide = i;[\s\S]{1,200}scheduleSessionSave\(\);/,
    );
  });

  test("static ASCII pre + Shortcuts header dropped", () => {
    expect(dashboard).not.toMatch(/<pre class="info-shortcuts">/);
    expect(dashboard).not.toMatch(/renderTable\(platform, os\)/);
  });

  test("body wraps the carousel in a labeled region", () => {
    expect(dashboard).toMatch(
      /class="dashboard"[\s\S]{1,120}aria-label="Dashboard"[\s\S]{1,120}role="region"/,
    );
  });
});

describe("Dashboard back-of-card is per-slot (DashboardSlotBack)", () => {
  test("DashboardTab right-click menu carries only Reload (no Settings entry)", () => {
    // Pane.svelte mounts HybridDashboardConfig via the `dashboard` arm;
    // Cmd+, is the canonical flip. The right-click menu keeps a Reload
    // row so it is discoverable from the body.
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
    // No Settings entry + no supporting state.
    expect(dashboard).not.toMatch(/<span class="menu-row-label">Settings<\/span>/);
    expect(dashboard).not.toMatch(/let settingsOpen = \$state/);
    expect(dashboard).not.toMatch(/function openSettings\b/);
    expect(dashboard).not.toMatch(/function closeSettings\b/);
    expect(dashboard).not.toMatch(/import HybridSurfaceConfigShell/);
  });

  test("DashboardSlotBack wraps the shared shell + dispatches one body per slot", () => {
    // Same shell every other Hybrid back uses, titled by the active
    // slot, mounting one of the three slot bodies off tab.carouselSlide.
    expect(dashboardBack).toMatch(
      /<HybridSurfaceConfigShell[\s\S]{1,200}title=\{SLOTS\[slot\]\}[\s\S]{1,120}surface="dashboard"[\s\S]{1,200}ariaLabel="Dashboard settings"[\s\S]{1,120}\{onDone\}/,
    );
    expect(dashboardBack).toMatch(
      /const SLOTS = \["About", "Workspace", "Search"\] as const;/,
    );
    expect(dashboardBack).toMatch(
      /\{#if slot === 0\}[\s\S]{1,80}<AboutSlotConfig \/>[\s\S]{1,160}<WorkspaceSlotConfig \/>[\s\S]{1,120}<SearchSlotConfig \/>/,
    );
    // Picking a slot moves the shared carousel cursor so the front
    // carousel lands on the same slot on flip-back.
    expect(dashboardBack).toMatch(/tab\.carouselSlide = i;/);
    // The shared shell still owns the OK button.
    expect(shell).toMatch(
      /<button type="button" class="config-ok" onclick=\{\(\) => onDone\?\.\(\)\}>OK<\/button>/,
    );
  });

  test("About slot owns Appearance + Screen lock; Workspace slot owns chan-reports + Metadata archive", () => {
    // The monolithic HybridDashboardConfig split per slot: Appearance +
    // Screen lock to the About body, Metadata archive (plus chan-reports
    // lifted from the former File Browser config) to the Workspace body.
    expect(aboutSlot).toMatch(/<h3>Appearance<\/h3>/);
    expect(aboutSlot).toMatch(/<h3>Screen lock<\/h3>/);
    expect(aboutSlot).toMatch(/name="app-appearance"/);
    expect(aboutSlot).toMatch(
      /onMount\(\(\) => \{[\s\S]{1,200}void loadScreenLockState\(\);/,
    );
    expect(workspaceSlot).toMatch(/<h3>chan-reports<\/h3>/);
    expect(workspaceSlot).toMatch(/<h3>Metadata archive<\/h3>/);
    expect(workspaceSlot).toMatch(/await api\.metadataExport\(\)/);
    expect(workspaceSlot).toMatch(/await api\.metadataImport\(metadataImportFile/);
    expect(workspaceSlot).toContain("Export metadata archive");
    expect(workspaceSlot).toContain("Import metadata archive");
    expect(workspaceSlot).toMatch(/bind:checked=\{metadataImportRescan\}/);
    expect(workspaceSlot).toMatch(/bind:checked=\{metadataImportForceScm\}/);
  });

  test("Pane.svelte back-side switch mounts DashboardSlotBack on the dashboard arm", () => {
    expect(pane).toMatch(
      /import DashboardSlotBack from "\.\/dashboard\/DashboardSlotBack\.svelte";/,
    );
    expect(pane).toMatch(
      /\{:else if active\?\.kind === "dashboard"\}[\s\S]{1,600}<DashboardSlotBack[\s\S]{1,160}tab=\{active\}[\s\S]{1,160}onDone=\{\(\) => flipHybrid\(pane\.id\)\}/,
    );
  });
});

describe("EmptyPaneWelcome static spawn surface", () => {
  test("EmptyPaneWelcome.svelte renders the 5-tile spawn grid + Dashboard tile", async () => {
    const welcome = (await import("./EmptyPaneWelcome.svelte?raw"))
      .default as string;
    expect(welcome).toMatch(
      /const spawnEntries: SpawnRow\[\] = \[[\s\S]{1,200}label: "New Draft",[\s\S]{1,1000}label: "Terminal",[\s\S]{1,800}label: "File Browser",[\s\S]{1,800}label: "Team Work",[\s\S]{1,800}label: "Graph",/,
    );
    // Secondary tile row: Search + Dashboard, both with chord hints
    // via chordLabel(row.chordId), in a 2-column grid.
    expect(welcome).toMatch(
      /const secondaryEntries: SpawnRow\[\] = \[[\s\S]{1,800}label: "Search",[\s\S]{1,200}command: "app\.search\.toggle",[\s\S]{1,800}label: "Dashboard",[\s\S]{1,200}command: "app\.dashboard\.open",/,
    );
    expect(welcome).toMatch(
      /import \{[\s\S]{1,200}\bSearch\b[\s\S]{1,200}\} from "lucide-svelte"/,
    );
    expect(welcome).toMatch(
      /spawn-row spawn-row-secondary[\s\S]{1,1000}<span class="spawn-chord">\{chordLabel\(row\.chordId\)\}<\/span>/,
    );
    expect(welcome).not.toMatch(/<span class="spawn-chord"><\/span>/);
    expect(welcome).toMatch(
      /\.spawn-row-secondary \{[\s\S]{1,400}grid-template-columns: repeat\(2,/,
    );
    // No "scope for Graph" hint in the welcome surface.
    expect(welcome).not.toMatch(/class="welcome-hint"/);
    expect(welcome).not.toMatch(/Each pane's visible tab is part of the scope/);
    // No `<p>` paragraph rendering the hint in the markup.
    expect(welcome).not.toMatch(/<p[\s\S]{0,200}scope[\s\S]{0,40}for Graph/);
  });

  test("Pane.svelte mounts EmptyPaneWelcome (not EmptyPaneCarousel) on lone-pane empty case", async () => {
    const pane = (await import("./Pane.svelte?raw")).default as string;
    expect(pane).toMatch(
      /import EmptyPaneWelcome from "\.\/EmptyPaneWelcome\.svelte";/,
    );
    // EmptyPaneWelcome does not forward oncontextmenu because there
    // is no empty-pane right-click menu.
    expect(pane).toMatch(
      /\{#if !multiPane\}[\s\S]{1,800}<EmptyPaneWelcome \/>/,
    );
    expect(pane).not.toMatch(/<EmptyPaneWelcome oncontextmenu=/);
    // EmptyPaneCarousel is owned by DashboardTab.svelte, not Pane.svelte.
    expect(pane).not.toMatch(
      /import EmptyPaneCarousel from "\.\/EmptyPaneCarousel\.svelte";/,
    );
  });
});
