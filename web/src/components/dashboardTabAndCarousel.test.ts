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

  test("render branch matches active?.kind === \"dashboard\"", () => {
    expect(pane).toMatch(
      /\{:else if active\?\.kind === "dashboard"\}[\s\S]{1,200}<DashboardTab \/>/,
    );
  });
});

describe("fullstack-a-75: Dashboard command + emptyPaneExtraActions wiring", () => {
  test("app.dashboard.open command routed to openDashboardInActivePane", () => {
    expect(app).toMatch(
      /case "app\.dashboard\.open":[\s\S]{1,400}openDashboardInActivePane\(\);/,
    );
  });

  test("emptyPaneExtraActions carries the Dashboard entry between Graph and Search (B5)", () => {
    expect(pane).toMatch(
      /const emptyPaneExtraActions:[\s\S]{1,800}label: "Dashboard",[\s\S]{1,400}command: "app\.dashboard\.open",[\s\S]{1,400}label: "Search",/,
    );
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
    expect(carousel).toMatch(
      /href="\/static\/fonts\/OFL\.txt"/,
    );
    expect(carousel).toMatch(
      /href="\/static\/matrix\/LICENSE-MatrixScreensaver\.txt"/,
    );
  });

  test("About widget loads buildInfo from the typed API", () => {
    expect(carousel).toMatch(
      /let buildInfo = \$state<BuildInfo \| null>\(null\)/,
    );
    expect(carousel).toMatch(/buildInfo = await api\.buildInfo\(\)/);
  });

  test("About widget embeds the donation QR + Fund-the-work copy", () => {
    expect(carousel).toMatch(/src="\/qr-donate\.png"/);
    expect(carousel).toMatch(/Fund the work/);
    expect(carousel).toMatch(
      /Chan is independent software\. Small tips help cover time[\s\S]{1,40}spent on releases, packaging, and documentation\./,
    );
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
      /<div class="slide slide-workspace" aria-label="Workspace info">[\s\S]{1,400}<WorkspaceInfoBody variant="dashboard" \/>/,
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
  test("DashboardTab imports + mounts EmptyPaneCarousel", () => {
    expect(dashboard).toMatch(
      /import EmptyPaneCarousel from "\.\/EmptyPaneCarousel\.svelte";/,
    );
    expect(dashboard).toMatch(/<EmptyPaneCarousel \/>/);
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
    // Four sections: Appearance / Screen lock / Screensaver /
    // Metadata archive.
    expect(cfg).toMatch(/<h3>Appearance<\/h3>/);
    expect(cfg).toMatch(/<h3>Screen lock<\/h3>/);
    expect(cfg).toMatch(/<h3>Screensaver<\/h3>/);
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
      /const spawnEntries: SpawnRow\[\] = \[[\s\S]{1,200}label: "New Draft",[\s\S]{1,1000}label: "Terminal",[\s\S]{1,800}label: "File Browser",[\s\S]{1,800}label: "Rich Prompt",[\s\S]{1,800}label: "Graph",/,
    );
    expect(welcome).toMatch(
      /const secondaryEntries: SpawnRow\[\] = \[[\s\S]{1,400}label: "Dashboard",[\s\S]{1,200}command: "app\.dashboard\.open",/,
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
    expect(pane).toMatch(
      /\{#if !multiPane\}[\s\S]{1,800}<EmptyPaneWelcome oncontextmenu=\{onEmptyPaneContextMenu\} \/>/,
    );
    // Pane.svelte no longer imports EmptyPaneCarousel directly
    // (it's owned by DashboardTab.svelte now).
    expect(pane).not.toMatch(
      /import EmptyPaneCarousel from "\.\/EmptyPaneCarousel\.svelte";/,
    );
  });
});
