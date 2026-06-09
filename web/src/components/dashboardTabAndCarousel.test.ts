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
import fileInfo from "./FileInfoBody.svelte?raw";
import inspector from "./InspectorBody.svelte?raw";

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
      /const FULL_SPAWN_ACTIONS:[\s\S]{1,2000}label: "Graph",[\s\S]{1,400}command: "app\.graph\.toggle",[\s\S]{1,400}label: "Search",[\s\S]{1,400}command: "app\.search\.toggle",[\s\S]{1,400}label: "Dashboard",[\s\S]{1,400}command: "app\.dashboard\.open",/,
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
    // The embeddings / hybrid-search status row moved to the Search
    // dashboard slot (SearchSlotConfig); the About card no longer renders
    // it. Match the visible label text, not source comments that explain
    // the move.
    expect(carousel).not.toMatch(/>embeddings</);
    expect(carousel).not.toMatch(/features\.embeddings/);
    // The third-party font + screensaver attributions were dropped from
    // the About slide; only chan's own Apache 2.0 (on the version row)
    // and the website / source links remain.
    expect(carousel).not.toMatch(/Source Code Pro Regular/);
    expect(carousel).not.toMatch(/dcragusa\/MatrixScreensaver/);
    expect(carousel).not.toMatch(/about-licenses/);
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

  test("A6: chan's Apache 2.0 sits on the version row; the licenses block is gone", () => {
    // A6 moved chan's own Apache 2.0 link onto the version row
    // (`chan version {version} Apache 2.0`). The third-party font +
    // screensaver attributions and the `.about-licenses` block that held
    // them were later dropped from the About slide.
    expect(carousel).toMatch(
      /<span class="k">chan version<\/span>[\s\S]{1,260}class="version-license"[\s\S]{1,160}Apache 2\.0<\/a>/,
    );
    // No `.about-licenses` block (markup or CSS) and no attributions.
    expect(carousel).not.toMatch(/about-licenses/);
    expect(carousel).not.toMatch(/<span class="k">terminal font<\/span>/);
    expect(carousel).not.toMatch(/<span class="k">matrix screen lock<\/span>/);
    // The chan / Apache 2.0 row no longer renders inside a k/v block
    // ("chan version" on the grid does not match this exact-text span).
    expect(carousel).not.toMatch(/<span class="k">chan<\/span>/);
    // The LICENSE anchor appears exactly once now (only the version row).
    const apacheMatches = carousel.match(
      /href="https:\/\/github\.com\/fiorix\/chan\/blob\/main\/LICENSE"/g,
    );
    expect(apacheMatches?.length ?? 0).toBe(1);
    // The separator below the Fund-the-work QR stays.
    expect(carousel).toMatch(/\.about-sep \{[\s\S]{1,400}background: var\(--border\)/);
  });

  test("R2-1: About widget shows the free/open-source tagline below the separator", () => {
    // The credits block sits after the Fund-the-work QR + the
    // `.about-sep`, and is just the free/open-source tagline; the
    // dependency list was dropped per @@Alex 2026-06-03 and the
    // third-party attributions were later removed too.
    expect(carousel).toMatch(
      /<div class="about-fund">[\s\S]{1,2000}<div class="about-sep"[\s\S]{1,400}<div class="about-credits">/,
    );
    expect(carousel).toMatch(
      /Built on a strong open-source foundation\. Chan is free and[\s\S]{1,20}open-source software\./,
    );
    // The dependency list (and any mermaid mirror) is gone after the trim.
    expect(carousel).not.toContain('class="credits-list"');
    expect(carousel).not.toContain("mermaid-cjv.pages.dev");
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
  test("A3: DashboardTab right-click menu lists slot toggles + Settings + Reload", () => {
    // A3 reverses the round-1 "only Reload" lock-out: the body
    // right-click menu now carries a per-slot on/off checkbox row, a
    // separator, a Settings (Cmd+,) row that flips to the config back,
    // and Reload.
    expect(dashboard).toMatch(/import HamburgerMenu from "\.\/HamburgerMenu\.svelte";/);
    expect(dashboard).toMatch(/function onContextMenu\(e: MouseEvent\): void/);
    expect(dashboard).toMatch(/menu\?\.openAtCursor\(e\.clientX, e\.clientY\)/);
    expect(dashboard).toMatch(
      /import \{[^}]*\bRefreshCw\b[^}]*\} from "lucide-svelte"/,
    );
    // Slot helpers + flipHybrid come from tabs.svelte.
    expect(dashboard).toMatch(
      /import \{[\s\S]{1,400}dashboardSlotEnabled,[\s\S]{1,200}toggleDashboardSlot,[\s\S]{1,120}\} from "\.\.\/state\/tabs\.svelte"/,
    );
    expect(dashboard).toMatch(
      /import \{[\s\S]{1,400}\bflipHybrid\b[\s\S]{1,200}\} from "\.\.\/state\/tabs\.svelte"/,
    );
    // One checkbox row per carousel slide, driven by the tab helpers.
    expect(dashboard).toMatch(
      /const SLOTS = \["About", "Workspace", "Search"\] as const;/,
    );
    expect(dashboard).toMatch(
      /\{#each SLOTS as label, i\}[\s\S]{1,400}role="menuitemcheckbox"[\s\S]{1,160}aria-checked=\{dashboardSlotEnabled\(tab, i\)\}[\s\S]{1,160}onclick=\{\(\) => onSlotToggle\(i\)\}/,
    );
    expect(dashboard).toMatch(
      /function onSlotToggle\(i: number\): void \{[\s\S]{1,200}toggleDashboardSlot\(tab, i\);/,
    );
    // Settings flips the active pane via flipHybrid (the Cmd+, path).
    expect(dashboard).toMatch(
      /function doSettings\(\): void \{[\s\S]{1,200}flipHybrid\(layout\.activePaneId\);/,
    );
    expect(dashboard).toMatch(
      /onclick=\{doSettings\}[\s\S]{1,200}<span class="menu-row-label">Settings<\/span>[\s\S]{1,200}chordLabel\("app\.settings\.toggle"\)/,
    );
    // Reload stays.
    expect(dashboard).toMatch(
      /import \{\s*reloadWindow\s*\} from "\.\.\/api\/desktop";/,
    );
    expect(dashboard).toMatch(/async function doReload\(\): Promise<void>/);
    expect(dashboard).toMatch(
      /onclick=\{doReload\}[\s\S]{1,200}<RefreshCw[\s\S]{1,200}<span class="menu-row-label">Reload<\/span>[\s\S]{1,160}<span class="menu-row-chord">\{chordLabel\("app\.window\.reload"\)\}<\/span>/,
    );
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
      /const FULL_SPAWN_ENTRIES: SpawnRow\[\] = \[[\s\S]{1,200}label: "New Draft",[\s\S]{1,1000}label: "Terminal",[\s\S]{1,800}label: "File Browser",[\s\S]{1,800}label: "Team Work",[\s\S]{1,800}label: "Graph",/,
    );
    // Secondary tile row: Search + Dashboard, both with chord hints
    // via chordLabel(row.chordId), in a 2-column grid.
    expect(welcome).toMatch(
      /const FULL_SECONDARY_ENTRIES: SpawnRow\[\] = \[[\s\S]{1,800}label: "Search",[\s\S]{1,200}command: "app\.search\.toggle",[\s\S]{1,800}label: "Dashboard",[\s\S]{1,200}command: "app\.dashboard\.open",/,
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

describe("Dashboard slot on/off helpers + persistence (A3)", () => {
  test("DashboardTab carries an optional disabledSlots set", () => {
    expect(tabs).toMatch(/disabledSlots\?: number\[\];/);
  });

  test("DASHBOARD_SLOT_COUNT + slot helpers are exported", () => {
    expect(tabs).toMatch(/export const DASHBOARD_SLOT_COUNT = 3;/);
    expect(tabs).toMatch(
      /export function dashboardSlotEnabled\(tab: DashboardTab, i: number\): boolean \{[\s\S]{1,200}!\(tab\.disabledSlots \?\? \[\]\)\.includes\(i\)/,
    );
    expect(tabs).toMatch(
      /export function firstEnabledSlot\(tab: DashboardTab\): number/,
    );
    expect(tabs).toMatch(
      /export function nextEnabledSlot\(tab: DashboardTab, from: number\): number/,
    );
  });

  test("toggleDashboardSlot refuses the last enabled slot + clears when all-on", () => {
    expect(tabs).toMatch(
      /export function toggleDashboardSlot\(tab: DashboardTab, i: number\): void \{[\s\S]{1,400}if \(DASHBOARD_SLOT_COUNT - disabled\.size <= 1\) return;[\s\S]{1,160}disabled\.add\(i\);/,
    );
    expect(tabs).toMatch(
      /tab\.disabledSlots = next\.length > 0 \? next : undefined;/,
    );
  });

  test("serializer emits ds only when the disabled set is non-empty", () => {
    expect(tabs).toMatch(/ds\?: number\[\];/);
    expect(tabs).toMatch(
      /if \(t\.kind === "dashboard"\) \{[\s\S]{1,600}\.\.\.\(t\.disabledSlots && t\.disabledSlots\.length > 0[\s\S]{1,80}\? \{ ds: t\.disabledSlots \}/,
    );
  });

  test("restore reads ds + clamps carouselSlide off a disabled slot", () => {
    expect(tabs).toMatch(
      /if \(kind === "d"\) \{[\s\S]{1,1400}dashboardSlotEnabled\(tab, want\)[\s\S]{1,80}\? want[\s\S]{1,80}: firstEnabledSlot\(tab\)/,
    );
  });

  test("carousel skips disabled slots in auto-rotate + dots", () => {
    expect(carousel).toMatch(/disabledSlots = \[\],/);
    expect(carousel).toMatch(/function nextEnabled\(from: number\): number/);
    // Auto-rotate advances to the next ENABLED slot.
    expect(carousel).toMatch(
      /setInterval\(\(\) => \{[\s\S]{1,120}onSlideChange\?\.\(nextEnabled\(slideIndex\)\);/,
    );
    // Pagination dots iterate the enabled slot set, not a fixed range.
    expect(carousel).toMatch(/\{#each enabledSlots as i \(i\)\}/);
    // slideIndex clamps off a disabled slot to the first enabled one.
    expect(carousel).toMatch(
      /const slideIndex = \$derived\.by\(\(\) => \{[\s\S]{1,240}slotEnabled\(clamped\) \? clamped : firstEnabled\(\)/,
    );
    // DashboardTab threads the per-tab set into the carousel.
    expect(dashboard).toMatch(/disabledSlots=\{tab\.disabledSlots \?\? \[\]\}/);
  });
});

describe("Search-slot directory inspector actions (A4)", () => {
  test("FileInfoBody gates Upload on allowUpload + adds a directory New Terminal", () => {
    expect(fileInfo).toMatch(/onNewTerminal\?: \(\) => void;/);
    expect(fileInfo).toMatch(/allowUpload\?: boolean;/);
    expect(fileInfo).toMatch(/allowUpload = true,/);
    // Upload is a directory dropdown action, gated behind allowUpload.
    expect(fileInfo).toMatch(
      /if \(allowUpload\) \{[\s\S]{1,160}label: "Upload file here",[\s\S]{1,80}onClick: triggerUpload/,
    );
    // Download is always offered (tarball for dirs, "Download file" otherwise).
    expect(fileInfo).toMatch(/onClick: downloadSelection,/);
    // "New terminal here" prefers the host handler, else seeds via fromHere.
    expect(fileInfo).toMatch(
      /label: "New terminal here",[\s\S]{1,40}onClick: newTerminalHere,/,
    );
    expect(fileInfo).toMatch(
      /function newTerminalHere\(\): void \{[\s\S]{1,120}if \(onNewTerminal\) \{[\s\S]{1,60}onNewTerminal\(\);[\s\S]{1,160}terminalFromHereTarget\(entry\.path, entry\.is_dir\)/,
    );
  });

  test("InspectorBody forwards onNewTerminal + allowUpload to the directory body", () => {
    expect(inspector).toMatch(/onNewTerminal,/);
    expect(inspector).toMatch(/allowUpload = true,/);
    // Directory arm forwards both; file arm forwards allowUpload.
    expect(inspector).toMatch(
      /\{onSetAsScope\}\s*\n\s*\{onNewTerminal\}\s*\n\s*\{allowUpload\}/,
    );
    expect(inspector).toMatch(/\{onSetAsScope\}\s*\n\s*\{allowUpload\}\s*\n\s*\{showRefs\}/);
  });

  test("index-graph slide binds the dir helpers + suppresses Upload", () => {
    expect(carousel).toMatch(/allowUpload=\{false\}/);
    expect(carousel).toMatch(
      /onReveal=\{\(\) => \{[\s\S]{1,200}revealPathInBrowser\(selectedIndexPath, \{/,
    );
    expect(carousel).toMatch(
      /onSetAsScope=\{\(\) => \{[\s\S]{1,200}openFsGraphForDirectory\(selectedIndexPath\)/,
    );
    expect(carousel).toMatch(
      /onNewTerminal=\{\(\) => \{[\s\S]{1,260}terminalFromHereTarget\(selectedIndexPath, true\)/,
    );
    // The helpers are imported from their owning modules.
    expect(carousel).toMatch(
      /import \{ layout, openTerminalInPane \} from "\.\.\/state\/tabs\.svelte";/,
    );
    expect(carousel).toMatch(
      /import \{ terminalFromHereTarget \} from "\.\.\/terminal\/fromHere";/,
    );
  });
});

describe("About-back screensaver preview reacts to theme (A7)", () => {
  test("preview switches on screensaverTheme; hint tracks the theme", () => {
    expect(aboutSlot).toMatch(
      /import PlainScreensaverPreview from "\.\.\/screensaver\/PlainScreensaverPreview\.svelte";/,
    );
    expect(aboutSlot).toMatch(
      /\{#if screensaverTheme === "matrix"\}[\s\S]{1,160}<MatrixRainPreview[\s\S]{1,80}\{:else\}[\s\S]{1,160}<PlainScreensaverPreview/,
    );
    expect(aboutSlot).toMatch(
      /Preview of the \{screensaverTheme === "matrix"[\s\S]{1,80}\? "Matrix"[\s\S]{1,80}: "Default"\} lock[\s\S]{1,20}theme/,
    );
    // No longer hardcoded to a Matrix-only preview + hint.
    expect(aboutSlot).not.toMatch(/Static preview of the Matrix lock theme\./);
    // The preview now lives INSIDE the Screen lock box (a div with a
    // title), not as a separate standalone <section> below it.
    expect(aboutSlot).not.toMatch(/<section class="screensaver-preview">/);
    expect(aboutSlot).toMatch(/class="preview-title">Screensaver preview</);
    // ...and only renders while the screen lock is ON (gated inside the
    // screensaverEnabled === true block).
    expect(aboutSlot).toMatch(
      /\{#if screensaverEnabled === true\}[\s\S]*?class="screensaver-preview"[\s\S]*?\{\/if\}/,
    );
  });

  test("PlainScreensaverPreview renders the enso mark on a dark backdrop", async () => {
    const plain = (
      await import("./screensaver/PlainScreensaverPreview.svelte?raw")
    ).default as string;
    expect(plain).toMatch(/chan-mark\.png/);
    expect(plain).toMatch(/background: var\(--bg\)/);
  });
});

describe("Per-tab auto-rotate opt-out (CK-CAROUSEL)", () => {
  test("DashboardTab carries optional autoRotate; serializer round-trips it as ar", () => {
    expect(tabs).toMatch(/autoRotate\?: boolean;/);
    expect(tabs).toMatch(/ar\?: boolean;/);
    expect(tabs).toMatch(
      /\.\.\.\(t\.autoRotate === false \? \{ ar: false \} : \{\}\)/,
    );
    expect(tabs).toMatch(/if \(sertab\.ar === false\) tab\.autoRotate = false;/);
  });

  test("carousel pauses auto-advance when autoRotate is false", () => {
    expect(carousel).toMatch(/autoRotate = true,/);
    expect(carousel).toMatch(/!active \|\| !autoRotate/);
    expect(dashboard).toMatch(/autoRotate=\{tab\.autoRotate \?\? true\}/);
  });
});

describe("Dashboard slot menu reachable from the tab title (A3)", () => {
  test("DashboardTab opens its menu from the shared tabMenu state", () => {
    // Pane.svelte's tab-title right-click routes every kind through
    // openTabMenu; DashboardTab translates a request targeting its tab
    // into opening the same HamburgerMenu at the click point.
    expect(dashboard).toMatch(
      /import \{ closeTabMenu, tabMenu \} from "\.\.\/state\/tabMenu\.svelte";/,
    );
    expect(dashboard).toMatch(/\$effect\(\(\) => \{/);
    expect(dashboard).toMatch(/tabMenu\.openForTabId !== tab\.id/);
    expect(dashboard).toMatch(/closeTabMenu\(\);[\s\S]{1,80}menu\.openAtCursor\(left, top\)/);
  });
});
