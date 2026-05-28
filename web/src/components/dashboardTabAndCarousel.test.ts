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
    expect(carousel).toMatch(
      /<div class="slide slide-workspace" aria-label="Workspace info">[\s\S]{1,400}<WorkspaceInfoBody \/>/,
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

describe("Wave 4: Dashboard settings", () => {
  test("right-click Settings menu uses the shared HamburgerMenu primitive", () => {
    expect(dashboard).toMatch(/import HamburgerMenu from "\.\/HamburgerMenu\.svelte";/);
    expect(dashboard).toMatch(/function onContextMenu\(e: MouseEvent\): void/);
    expect(dashboard).toMatch(/menu\?\.openAtCursor\(e\.clientX, e\.clientY\)/);
    expect(dashboard).toMatch(/<Settings2 size=\{16\}/);
    expect(dashboard).toMatch(/<span class="menu-row-label">Settings<\/span>/);
  });

  test("settings view uses the shared surface theme shell and OK button", () => {
    // `phase-13 lane-b` slice 3c: surfaceThemeOverride is now
    // imported alongside the global Appearance helpers
    // (setThemeChoice + ThemeChoice + ui) from store.svelte, so
    // the assertion matches the import inside a multi-import
    // block rather than requiring a dedicated import line.
    expect(dashboard).toMatch(
      /import \{[\s\S]{1,400}surfaceThemeOverride,?[\s\S]{0,400}\} from "\.\.\/state\/store\.svelte";/,
    );
    expect(dashboard).toMatch(/data-theme=\{surfaceThemeOverride\("dashboard"\)\}/);
    expect(dashboard).toMatch(/ariaLabel="Dashboard settings"/);
    expect(dashboard).toMatch(
      /<HybridSurfaceConfigShell[\s\S]{1,220}title="Dashboard"[\s\S]{1,120}surface="dashboard"[\s\S]{1,160}onDone=\{closeSettings\}/,
    );
    expect(dashboard).not.toMatch(/type DashboardAppearance/);
    // Slice 3c added a GLOBAL Appearance radio group to this
    // back-of-card; the radio `name` deliberately uses
    // `app-appearance` (not `dashboard-appearance`) so the
    // earlier rejected per-tab DashboardAppearance enum can't
    // sneak back via the same name.
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
