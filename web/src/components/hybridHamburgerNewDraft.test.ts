import { describe, expect, test } from "vitest";
import pane from "./Pane.svelte?raw";
import app from "../App.svelte?raw";

// New Draft is the first spawn entry in the Hybrid hamburger, the
// empty-pane right-click menu, and the carousel, backed by one shared
// `spawnActions` list.

describe("spawnActions includes New Draft first", () => {
  test("`New Draft` entry sits at slot 0 of spawnActions", () => {
    expect(pane).toMatch(
      /const spawnActions: EmptyMenuRow\[\] = \[[\s\S]*?label: "New Draft",[\s\S]*?icon: FilePlus,[\s\S]*?command: "app\.draft\.new",[\s\S]*?chordId: "app\.draft\.new",/,
    );
  });

  test("FilePlus icon imported alongside the other spawn-surface icons", () => {
    expect(pane).toMatch(
      /import \{[\s\S]*?\bFilePlus,[\s\S]*?\} from "lucide-svelte";/,
    );
  });

  test("source comment cites the three shared spawn surfaces", () => {
    expect(pane).toMatch(
      /empty-pane right-click menu[\s\S]*?pane hamburger[\s\S]*?empty-pane carousel/i,
    );
    expect(pane).toMatch(/single `spawnActions` list backs[\s\S]{1,20}all three surfaces/i);
  });

  test("the existing 4 spawn entries are preserved in order (Terminal/FB/Team Work/Graph)", () => {
    // The Team Work entry is labelled "Team Work" with chord id
    // app.terminal.teamWork.
    expect(pane).toMatch(
      /label: "Terminal",[\s\S]*?label: "File Browser",[\s\S]*?label: "Team Work",[\s\S]*?label: "Graph",/,
    );
  });
});

describe("Hybrid hamburger carries no theme/flip rows", () => {
  test("Light mode / Flip pane handlers and labels are absent", () => {
    expect(pane).not.toContain("togglePaneTheme");
    expect(pane).not.toContain("paneThemeTooltip");
    expect(pane).not.toContain("paneEffectiveTheme");
    expect(pane).not.toContain("Light mode");
    expect(pane).not.toContain("Dark mode");
    expect(pane).not.toContain("Flip pane");
    expect(pane).not.toContain("FlipHorizontal2");
    expect(pane).not.toContain("Moon");
    expect(pane).not.toContain("Sun");
  });

  test("no Settings footer in the Hybrid pane menu", () => {
    // Cmd+, opens Settings via the Dashboard back-of-card (flipHybrid);
    // a duplicate Settings row is not needed in the pane menu.
    expect(pane).not.toMatch(
      /onclick=\{\(\) => dispatchCommand\("app\.settings\.toggle"\)\}[\s\S]*?<span class="menu-row-label">Settings<\/span>/,
    );
  });
});

describe("App.svelte runCommand routes app.draft.new", () => {
  test("runCommand switch dispatches `app.draft.new` to createDraftAndOpen", () => {
    expect(app).toMatch(
      /case "app\.draft\.new":[\s\S]*?void createDraftAndOpen\(\);[\s\S]*?return;/,
    );
  });

  test("createDraftAndOpen helper is preserved (the routing target)", () => {
    expect(app).toMatch(
      /async function createDraftAndOpen\(\): Promise<void> \{[\s\S]*?const \{ path \} = await api\.createDraft\(\);[\s\S]*?await openInActivePane\(path, \{/,
    );
  });
});
