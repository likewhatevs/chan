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
// shortcut table dropped, Infographics secondary band), and the
// surface unification across the three menus (pane hamburger,
// empty-pane right-click, carousel).
//
// `phase-13 lane-b`: Infographics -> Dashboard rename. The string
// discriminator + helper names + component file all moved to
// "dashboard"; the user-visible label still reads "Infographics"
// until the dashboard widget rework lands in a later slice.

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

  test("emptyPaneExtraActions carries the Infographics entry", () => {
    expect(pane).toMatch(
      /const emptyPaneExtraActions:[\s\S]{1,800}label: "Infographics",[\s\S]{1,400}command: "app\.dashboard\.open",/,
    );
  });
});

describe("fullstack-a-75: carousel slide 1 redesign", () => {
  // `fullstack-a-75b`: spawn entries + secondary band moved
  // OUT of the carousel and into EmptyPaneWelcome.svelte. The
  // carousel is now a pure rotating widget hosted inside the
  // Dashboard tab; slide 1 carries the ASCII shortcut table.
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

  test("slide 1 is now the Shortcuts ASCII table (renderTable back inside carousel)", () => {
    expect(carousel).toMatch(
      /import \{[\s\S]{1,400}renderTable,[\s\S]{1,200}\} from "\.\.\/state\/shortcuts";/,
    );
    expect(carousel).toMatch(
      /const shortcutTable = renderTable\(platform, os\);/,
    );
    expect(carousel).toMatch(
      /<div class="slide slide-shortcuts" aria-label="Shortcuts">[\s\S]{1,800}<pre class="shortcuts-table">\{shortcutTable\}<\/pre>/,
    );
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
      /class="dashboard"[\s\S]{1,120}aria-label="Infographics"[\s\S]{1,120}role="region"/,
    );
  });
});

describe("Wave 4: Dashboard settings", () => {
  test("right-click Settings menu uses the shared HamburgerMenu primitive", () => {
    expect(dashboard).toMatch(/import HamburgerMenu from "\.\/HamburgerMenu\.svelte";/);
    expect(dashboard).toMatch(/function onContextMenu\(e: MouseEvent\): void/);
    expect(dashboard).toMatch(/menu\?\.openAtCursor\(e\.clientX, e\.clientY\)/);
    expect(dashboard).toMatch(/<Settings2 size=\{16\}/);
    expect(dashboard).toMatch(/<span class="menu-row-label">Settings<\/span>/);
  });

  test("settings view uses the shared surface theme shell and OK button", () => {
    expect(dashboard).toMatch(
      /import \{ surfaceThemeOverride \} from "\.\.\/state\/store\.svelte";/,
    );
    expect(dashboard).toMatch(/data-theme=\{surfaceThemeOverride\("dashboard"\)\}/);
    expect(dashboard).toMatch(/ariaLabel="Infographics settings"/);
    expect(dashboard).toMatch(
      /<HybridSurfaceConfigShell[\s\S]{1,220}title="Infographics"[\s\S]{1,120}surface="dashboard"[\s\S]{1,160}onDone=\{closeSettings\}/,
    );
    expect(dashboard).not.toMatch(/type DashboardAppearance/);
    expect(dashboard).not.toMatch(/name="dashboard-appearance"/);
    expect(shell).toMatch(
      /<button type="button" class="config-ok" onclick=\{\(\) => onDone\?\.\(\)\}>OK<\/button>/,
    );
  });

  test("settings view exposes metadata archive export through the typed API", () => {
    expect(dashboard).toMatch(/import \{ api \} from "\.\.\/api\/client";/);
    expect(dashboard).toMatch(/import \{ formatSize \} from "\.\.\/state\/format";/);
    expect(dashboard).toMatch(/async function exportMetadataArchive\(\): Promise<void>/);
    expect(dashboard).toMatch(/await api\.metadataExport\(\)/);
    expect(dashboard).toMatch(/await api\.metadataImport\(metadataImportFile/);
    expect(dashboard).toMatch(/URL\.createObjectURL\(download\.blob\)/);
    expect(dashboard).toContain("Metadata archive");
    expect(dashboard).toContain("Export metadata archive");
    expect(dashboard).toContain("Import metadata archive");
    expect(dashboard).toContain("Force SCM mismatch");
    expect(dashboard).toContain("Rescan after import");
  });
});

describe("fullstack-a-75b: EmptyPaneWelcome static spawn surface", () => {
  test("EmptyPaneWelcome.svelte renders the 5-tile spawn grid + Infographics tile (per -a-95: welcome-hint dropped)", async () => {
    const welcome = (await import("./EmptyPaneWelcome.svelte?raw"))
      .default as string;
    expect(welcome).toMatch(
      /const spawnEntries: SpawnRow\[\] = \[[\s\S]{1,200}label: "New Draft",[\s\S]{1,1000}label: "Terminal",[\s\S]{1,800}label: "File Browser",[\s\S]{1,800}label: "Rich Prompt",[\s\S]{1,800}label: "Graph",/,
    );
    expect(welcome).toMatch(
      /const secondaryEntries: SpawnRow\[\] = \[[\s\S]{1,400}label: "Infographics",[\s\S]{1,200}command: "app\.dashboard\.open",/,
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
