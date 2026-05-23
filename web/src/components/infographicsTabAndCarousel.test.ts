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
  test("ASCII shortcut table dropped from carousel markup", () => {
    expect(carousel).not.toMatch(/class="placeholder-shortcuts"/);
    expect(carousel).not.toMatch(/renderTable\(platform, os\)/);
  });

  test("renderTable import dropped from carousel", () => {
    expect(carousel).not.toMatch(
      /import \{[\s\S]{1,400}renderTable,[\s\S]{1,200}\} from "\.\.\/state\/shortcuts";/,
    );
  });

  test("primary spawnEntries lists New Draft / Terminal / FB / RP / Graph in order", () => {
    expect(carousel).toMatch(
      /const spawnEntries: SpawnRow\[\] = \[[\s\S]{1,200}label: "New Draft",[\s\S]{1,1000}label: "Terminal",[\s\S]{1,800}label: "File Browser",[\s\S]{1,800}label: "Rich Prompt",[\s\S]{1,800}label: "Graph",/,
    );
  });

  test("secondaryEntries carries Infographics", () => {
    expect(carousel).toMatch(
      /const secondaryEntries: SpawnRow\[\] = \[[\s\S]{1,400}label: "Infographics",[\s\S]{1,200}command: "app\.infographics\.open",/,
    );
  });

  test("markup renders secondary band below the primary spawn-row + separator", () => {
    expect(carousel).toMatch(
      /<div class="spawn-row" aria-label="spawn">[\s\S]{1,4000}<div class="spawn-sep"[\s\S]{1,400}<div class="spawn-row spawn-row-secondary"/,
    );
  });
});

describe("fullstack-a-75: InfographicsTab body carries the shortcut table", () => {
  test("renderTable used + shortcutTable rendered as monospace pre", () => {
    expect(infographics).toMatch(
      /const shortcutTable = renderTable\(platform, os\);/,
    );
    expect(infographics).toMatch(
      /<pre class="info-shortcuts">\{shortcutTable\}<\/pre>/,
    );
  });

  test("body wraps the table in a labeled region", () => {
    expect(infographics).toMatch(
      /<div class="infographics" aria-label="Infographics">/,
    );
    expect(infographics).toMatch(/<h2>Shortcuts<\/h2>/);
  });
});
