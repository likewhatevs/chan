import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";
import terminal from "../components/TerminalTab.svelte?raw";
import shortcuts from "./shortcuts.ts?raw";

// `fullstack-a-90`: the legacy `Alt+Space` rich-prompt chord
// is removed. Cmd+P (native), Cmd+Alt+P (web Mac), and
// `Mod+. p` (Hybrid NAV) cover every entry point.

describe("fullstack-a-90: Alt+Space chord removed from App.svelte", () => {
  test("no `altKey + Space` keymap branch for rich-prompt spawn", () => {
    expect(app).not.toMatch(
      /if \(e\.altKey && !meta && !e\.shiftKey && e\.code === "Space"\) \{[\s\S]*?spawnRichPromptFromContext\(\);/,
    );
  });

  test("removal-rationale comment present", () => {
    expect(app).toMatch(
      /`fullstack-a-90`:[\s\S]*?removed the legacy `Alt\+Space` rich-prompt/i,
    );
  });

  test("keymap header doc-block no longer advertises Alt+Space", () => {
    expect(app).not.toMatch(/Alt\+Space\s+->\s+Rich Prompt/);
  });
});

describe("fullstack-a-90: Alt+Space chord removed from TerminalTab.svelte", () => {
  test("no secondary `altKey + Space` handler firing `openRichPrompt`", () => {
    expect(terminal).not.toMatch(
      /if \(e\.altKey && !e\.ctrlKey && !e\.metaKey && !e\.shiftKey && e\.code === "Space"\) \{[\s\S]*?openRichPrompt\(\);/,
    );
  });

  test("removal-rationale comment present", () => {
    expect(terminal).toMatch(
      /`fullstack-a-90`:[\s\S]*?removed the legacy `Alt\+Space` rich-prompt/i,
    );
  });

  test("hamburger-menu comment no longer references Alt+Space as live", () => {
    expect(terminal).not.toMatch(/Alt\+Space still works as the/i);
  });
});

describe("fullstack-a-90: shortcuts registry note updated", () => {
  test("registry note no longer mentions Alt+Space", () => {
    expect(shortcuts).not.toMatch(/legacy Alt\+Space alias still bound/);
  });

  test("`-a-90` retire comment present in registry block", () => {
    expect(shortcuts).toMatch(
      /`fullstack-a-90`[\s\S]*?retired[\s\S]*?Alt\+Space/i,
    );
  });
});
