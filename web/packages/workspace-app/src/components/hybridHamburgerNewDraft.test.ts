import { describe, expect, test } from "vitest";
import pane from "./Pane.svelte?raw";
import app from "../App.svelte?raw";

// Spawn actions now live in the command launcher. The pane hamburger
// stays small: Commands, separator, focus border colour.

describe("pane hamburger no longer owns spawnActions", () => {
  test("spawnActions data and New Draft row are absent from Pane.svelte", () => {
    expect(pane).not.toContain("FULL_SPAWN_ACTIONS");
    expect(pane).not.toContain("spawnActions");
    expect(pane).not.toMatch(/<span class="menu-row-label">New Draft<\/span>/);
  });

  test("FilePlus icon is not imported by the pane hamburger", () => {
    expect(pane).not.toMatch(
      /import \{[\s\S]*?\bFilePlus,[\s\S]*?\} from "lucide-svelte";/,
    );
  });

  test("source comment points spawn discovery at the launcher", () => {
    expect(pane).toMatch(
      /launcher is the discovery surface for spawn actions/i,
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
    // The pane flip is command-driven; a duplicate Settings row is not needed
    // in the pane menu.
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
