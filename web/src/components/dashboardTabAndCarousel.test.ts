import { describe, expect, test } from "vitest";
import tabs from "../state/tabs.svelte.ts?raw";
import pane from "./Pane.svelte?raw";
import carousel from "./EmptyPaneCarousel.svelte?raw";
import dashboard from "./DashboardTab.svelte?raw";
import app from "../App.svelte?raw";
import shell from "./HybridSurfaceConfigShell.svelte?raw";

// Dashboard tab kind + carousel coverage.
// Tests pin: the tab type + helpers, the Pane.svelte render
// branch, the carousel's spawn band (New Draft slot 0, no shortcut
// table, Dashboard secondary band), and the surface unification
// across the three menus (pane hamburger, empty-pane right-click,
// carousel). The "dashboard" string discriminator drives the
// helper names + component file + user-visible labels (menu entry,
// shortcut label, tab title, aria-label, settings shell title).

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
    // Pane.svelte threads the live DashboardTab proxy through so
    // the carousel slide cursor can round-trip back into
    // tabs.svelte.ts's session serializer.
    expect(pane).toMatch(
      /\{:else if active\?\.kind === "dashboard"\}[\s\S]{1,200}<DashboardTab tab=\{active\} \/>/,
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
    // A single `spawnActions` list backs both the pane top-bar
    // hamburger and the empty-pane right-click menu, so both render
    // the same 7-entry spawn set in the same order: ..., Graph,
    // Search, Dashboard.
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
  // Slide 0 is the About widget (version + embeddings flag +
  // attributions + donation QR + chan.app/source links). The
  // carousel About widget is the sole home for the
  // version/attribution surface.
  test("slide 0 is the About widget", () => {
    expect(carousel).toMatch(
      /<div class="slide slide-about" aria-label="About">/,
    );
    expect(carousel).toMatch(/chan version/);
    expect(carousel).toMatch(/embeddings/);
    expect(carousel).toMatch(/Source Code Pro Regular/);
    expect(carousel).toMatch(/dcragusa\/MatrixScreensaver/);
    // License links resolve to canonical upstream URLs instead of
    // embedded `/static/...` paths (which under chan-desktop's
    // non-root mount surface as 127.0.0.1 links). The font lives in
    // the adobe-fonts source-code-pro repo + the screen-lock in
    // dcragusa's repo.
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
    // src is wrapped in `withTokenQuery("/qr-donate.png")` so the
    // prefix rewrite + per-launch bearer token apply under
    // chan-desktop's non-root mount and the tunnel-mode prefix.
    // A raw `<img src="/qr-donate.png">` bypasses both and renders
    // a broken-image square. Match the helper wrapper, not the raw
    // path.
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
    // License rows live in a dedicated `.about-licenses` block,
    // separated from the Fund-the-work surface by `.about-sep`.
    // Chan's own Apache 2.0 license joins the section so the three
    // runtime licenses live together.
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
    // The Dashboard slide passes variant="dashboard" so the
    // workspace-root inspector keeps its Notes-directories config
    // (the inspector variant drops it).
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

  test("slide 2 is the read-only, spine-only indexing graph", () => {
    expect(carousel).toMatch(/class="slide slide-indexing"/);
    // No chrome (inspector / scope picker / depth slider / filter
    // chips); the slide is purely a status read-out fed from
    // `/api/indexing/state`.
    expect(carousel).toMatch(/aria-label="Indexing graph"/);
  });

  test("indexing slide maximises to the tab width/height with a 10px border", () => {
    // The About + Workspace slides are text-shaped and read better
    // in the centered 720px column; the indexing graph wants the
    // full tab area so the spine doesn't compress to a vertical
    // band. The wide-stage class is toggled only on slideIndex === 2
    // + drops the `max-width: 720px` cap, and the carousel-wide
    // variant tightens the outer padding to ~10px so the canvas
    // reads edge-to-edge with a reasonable breathing border.
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
    // Clicks on the indexing graph update a `selectedIndexId`
    // $state and feed it into GraphCanvas.selectedId. GraphCanvas
    // labels the selected node + 1-hop neighbours, so this wiring
    // surfaces the labels on the read-only spine.
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
    // DashboardTab passes the persisted slide cursor + a write-back
    // callback so the carousel position survives a window reload.
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

describe("Dashboard back-of-card lives in HybridDashboardConfig", () => {
  test("DashboardTab right-click menu carries only Reload (no Settings entry)", () => {
    // There is no local `settingsOpen` path; Pane.svelte's
    // back-side switch mounts HybridDashboardConfig directly via the
    // `active?.kind === "dashboard"` arm, and Cmd+, is the canonical
    // flip. The right-click menu keeps a Reload row so the
    // affordance is still discoverable from the body.
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
    // Three sections: Appearance / Screen lock / Metadata archive.
    // The Screensaver theme picker is folded INTO the Screen lock
    // enable gate, so it shares the lifecycle of the Screen lock
    // toggle and carries no standalone `<h3>Screensaver</h3>`.
    expect(cfg).toMatch(/<h3>Appearance<\/h3>/);
    expect(cfg).toMatch(/<h3>Screen lock<\/h3>/);
    expect(cfg).not.toMatch(/<h3>Screensaver<\/h3>/);
    expect(cfg).toMatch(/<h3>Metadata archive<\/h3>/);
    // Appearance is an app-wide setting, so the radio group uses
    // the `app-appearance` name, not a per-tab
    // `dashboard-appearance` name.
    expect(cfg).toMatch(/name="app-appearance"/);
    expect(cfg).not.toMatch(/name="dashboard-appearance"/);
    // Metadata archive surfaces the typed API + the labels + the
    // rescan / force-SCM checkboxes.
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
    // The shared shell owns the OK button.
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

describe("EmptyPaneWelcome static spawn surface", () => {
  test("EmptyPaneWelcome.svelte renders the 5-tile spawn grid + Dashboard tile (no welcome-hint)", async () => {
    const welcome = (await import("./EmptyPaneWelcome.svelte?raw"))
      .default as string;
    expect(welcome).toMatch(
      /const spawnEntries: SpawnRow\[\] = \[[\s\S]{1,200}label: "New Draft",[\s\S]{1,1000}label: "Terminal",[\s\S]{1,800}label: "File Browser",[\s\S]{1,800}label: "Team Work",[\s\S]{1,800}label: "Graph",/,
    );
    // The secondary tile row carries Search + Dashboard (Search
    // first), and both render their chord hints via
    // `chordLabel(row.chordId)` rather than a hardcoded empty
    // `<span class="spawn-chord"></span>`. The row CSS is a
    // 2-column grid.
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
    // No hardcoded empty chord span.
    expect(welcome).not.toMatch(/<span class="spawn-chord"><\/span>/);
    // 2-column grid for the secondary row.
    expect(welcome).toMatch(
      /\.spawn-row-secondary \{[\s\S]{1,400}grid-template-columns: repeat\(2,/,
    );
    // No per-tab "scope for Graph" hint; picker-driven scope is the
    // active mechanism, surfaced in the graph overlay's chrome.
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
    // The EmptyPaneWelcome mount does not forward `oncontextmenu`;
    // there is no empty-pane right-click menu, so the welcome
    // surface has no parent handler to forward to.
    expect(pane).toMatch(
      /\{#if !multiPane\}[\s\S]{1,800}<EmptyPaneWelcome \/>/,
    );
    expect(pane).not.toMatch(/<EmptyPaneWelcome oncontextmenu=/);
    // Pane.svelte does not import EmptyPaneCarousel directly
    // (it's owned by DashboardTab.svelte).
    expect(pane).not.toMatch(
      /import EmptyPaneCarousel from "\.\/EmptyPaneCarousel\.svelte";/,
    );
  });
});
