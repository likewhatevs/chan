import { describe, expect, test } from "vitest";
import pane from "./Pane.svelte?raw";
import app from "../App.svelte?raw";

// `fullstack-a-67` slice 2: Hybrid hamburger gets New Draft as
// its first spawn surface per addendum-a. Shared across the
// empty-pane right-click + the pane hamburger + the empty-pane
// carousel slide 1 so all three surfaces gain the same
// affordance.

describe("fullstack-a-67 slice 2: spawnActions includes New Draft first", () => {
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

  test("rationale comment cites the addendum-a Hybrid hamburger spec + the three shared surfaces", () => {
    expect(pane).toMatch(/`fullstack-a-67` slice 2/);
    expect(pane).toMatch(/addendum-a Hybrid hamburger spec/i);
    expect(pane).toMatch(/empty-pane right-click \+ the pane[\s\S]*?hamburger \+ the empty-pane carousel/i);
  });

  test("the existing 4 spawn entries are preserved in order (Terminal/FB/Team Work/Graph)", () => {
    // phase-13 r2: the Rich Prompt entry was relabelled "Team Work"
    // (chord id app.terminal.richPrompt stays stable).
    expect(pane).toMatch(
      /label: "Terminal",[\s\S]*?label: "File Browser",[\s\S]*?label: "Team Work",[\s\S]*?label: "Graph",/,
    );
  });
});

describe("fullstack-a-98: Hybrid hamburger removes stale theme/flip rows", () => {
  test("old Light mode / Flip pane handlers and labels are gone", () => {
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

  test("Settings footer retired from the Hybrid pane menu (phase-13 slice 3c)", () => {
    // The Cmd+, rebind moves Settings off the empty-pane menu and
    // onto the Dashboard back-of-card via flipHybrid. The pane
    // menu must no longer carry a Settings row that dispatches
    // app.settings.toggle.
    expect(pane).not.toMatch(
      /onclick=\{\(\) => dispatchCommand\("app\.settings\.toggle"\)\}[\s\S]*?<span class="menu-row-label">Settings<\/span>/,
    );
  });
});

describe("fullstack-a-67 slice 2: App.svelte runCommand routes app.draft.new", () => {
  test("runCommand switch dispatches `app.draft.new` to createDraftAndOpen", () => {
    expect(app).toMatch(
      /case "app\.draft\.new":[\s\S]*?void createDraftAndOpen\(\);[\s\S]*?return;/,
    );
  });

  test("rationale comment cites the chord + menu + native menu convergence", () => {
    expect(app).toMatch(/`fullstack-a-67` slice 2/);
    expect(app).toMatch(
      /menu \+ the Cmd\+N chord \+[\s\S]{1,80}native menu all converge/i,
    );
  });

  test("createDraftAndOpen helper is preserved (the routing target)", () => {
    expect(app).toMatch(
      /async function createDraftAndOpen\(\): Promise<void> \{[\s\S]*?const \{ path \} = await api\.createDraft\(\);[\s\S]*?await openInActivePane\(path, \{/,
    );
  });
});
