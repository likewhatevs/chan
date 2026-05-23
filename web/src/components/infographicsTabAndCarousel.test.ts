import { describe, expect, test } from "vitest";
import tabs from "../state/tabs.svelte.ts?raw";
import pane from "./Pane.svelte?raw";
import carousel from "./EmptyPaneCarousel.svelte?raw";
import infographics from "./InfographicsTab.svelte?raw";
import app from "../App.svelte?raw";

// `fullstack-a-75`: Infographics tab kind + carousel redesign.
// Tests pin: new tab type + helpers, the Pane.svelte render
// branch, the carousel's spawn band changes (New Draft slot 0,
// shortcut table dropped, Infographics secondary band), and the
// surface unification across the three menus (pane hamburger,
// empty-pane right-click, carousel).

describe("fullstack-a-75: InfographicsTab type + helpers", () => {
  test("Tab union includes InfographicsTab", () => {
    expect(tabs).toMatch(
      /export type InfographicsTab = \{[\s\S]{1,400}kind: "infographics";[\s\S]{1,200}id: string;[\s\S]{1,200}title: string;/,
    );
    expect(tabs).toMatch(
      /export type Tab =\s*\n\s*\| FileTab[\s\S]{1,400}\| InfographicsTab;/,
    );
  });

  test("openInfographicsInPane appends a Infographics tab + activates it", () => {
    expect(tabs).toMatch(
      /export function openInfographicsInPane\(paneId: string\): void \{[\s\S]{1,800}kind: "infographics",[\s\S]{1,400}node\.tabs\.push\(tab\);[\s\S]{1,200}node\.activeTabId = tab\.id;/,
    );
  });

  test("openInfographicsInActivePane delegates to openInfographicsInPane(layout.activePaneId)", () => {
    expect(tabs).toMatch(
      /export function openInfographicsInActivePane\(\): void \{[\s\S]{1,200}openInfographicsInPane\(layout\.activePaneId\);/,
    );
  });

  test("tabLabel handles infographics kind", () => {
    expect(tabs).toMatch(
      /export function tabLabel\(t: Tab, ctx\?: BrowserLabelCtx\): string \{[\s\S]{1,800}if \(t\.kind === "infographics"\) return t\.title;/,
    );
  });

  test("serializer emits k:\"i\" for infographics tabs", () => {
    expect(tabs).toMatch(
      /if \(t\.kind === "infographics"\) \{[\s\S]{1,200}k: "i",/,
    );
  });

  test("SerTab kind discriminator includes \"i\"", () => {
    expect(tabs).toMatch(
      /k\?: "f" \| "b" \| "s" \| "g" \| "h" \| "t" \| "i";/,
    );
  });
});

describe("fullstack-a-75: Pane.svelte render branch + import", () => {
  test("InfographicsTab imported", () => {
    expect(pane).toMatch(
      /import InfographicsTab from "\.\/InfographicsTab\.svelte";/,
    );
  });

  test("render branch matches active?.kind === \"infographics\"", () => {
    expect(pane).toMatch(
      /\{:else if active\?\.kind === "infographics"\}[\s\S]{1,200}<InfographicsTab \/>/,
    );
  });
});

describe("fullstack-a-75: Infographics command + emptyPaneExtraActions wiring", () => {
  test("app.infographics.open command routed to openInfographicsInActivePane", () => {
    expect(app).toMatch(
      /case "app\.infographics\.open":[\s\S]{1,400}openInfographicsInActivePane\(\);/,
    );
  });

  test("emptyPaneExtraActions carries the Infographics entry", () => {
    expect(pane).toMatch(
      /const emptyPaneExtraActions:[\s\S]{1,800}label: "Infographics",[\s\S]{1,400}command: "app\.infographics\.open",/,
    );
  });
});

describe("fullstack-a-75: carousel slide 1 redesign", () => {
  // `fullstack-a-75b`: spawn entries + secondary band moved
  // OUT of the carousel and into EmptyPaneWelcome.svelte. The
  // carousel is now a pure rotating widget hosted inside the
  // Infographics tab; slide 1 carries the ASCII shortcut table.
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

describe("fullstack-a-75b: InfographicsTab mounts the carousel", () => {
  test("InfographicsTab imports + mounts EmptyPaneCarousel", () => {
    expect(infographics).toMatch(
      /import EmptyPaneCarousel from "\.\/EmptyPaneCarousel\.svelte";/,
    );
    expect(infographics).toMatch(/<EmptyPaneCarousel \/>/);
  });

  test("static ASCII pre + Shortcuts header dropped (carousel owns the shortcut surface now)", () => {
    expect(infographics).not.toMatch(/<pre class="info-shortcuts">/);
    expect(infographics).not.toMatch(/renderTable\(platform, os\)/);
  });

  test("body wraps the carousel in a labeled region", () => {
    expect(infographics).toMatch(
      /<div class="infographics" aria-label="Infographics">/,
    );
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
      /const secondaryEntries: SpawnRow\[\] = \[[\s\S]{1,400}label: "Infographics",[\s\S]{1,200}command: "app\.infographics\.open",/,
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
    // (it's owned by InfographicsTab.svelte now).
    expect(pane).not.toMatch(
      /import EmptyPaneCarousel from "\.\/EmptyPaneCarousel\.svelte";/,
    );
  });
});
