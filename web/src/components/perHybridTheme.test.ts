import { describe, expect, test } from "vitest";
import pane from "./Pane.svelte?raw";
import app from "../App.svelte?raw";

// fullstack-59: wire per-Hybrid `node.theme` into render. The model
// + URL hash layer already round-trips `ht` / `hb` (fullstack-48
// phase A); this test pins the render wiring so a future refactor
// can't silently unhook it.
describe("fullstack-59: per-Hybrid theme render wiring", () => {
  test("Pane root carries data-theme bound to pane.theme", () => {
    expect(pane).toContain("data-theme={pane.theme}");
  });

  test("Pane hamburger no longer renders the old theme-toggle entry", () => {
    // `fullstack-a-98`: addendum-a routes Settings through the
    // pane footer; the stale Light/Dark hamburger row is gone.
    expect(pane).not.toContain("togglePaneTheme");
    expect(pane).not.toContain("paneThemeTooltip");
    expect(pane).not.toContain("Light mode");
    expect(pane).not.toContain("Dark mode");
  });

  test("Settings remains the pane footer action", () => {
    expect(pane).toContain('dispatchCommand("app.settings.toggle")');
    expect(pane).toMatch(
      /dispatchCommand\("app\.settings\.toggle"\)[\s\S]*?<span class="menu-row-label">Settings<\/span>/,
    );
  });

  test("CSS cascade re-applies token blocks at pane scope", () => {
    // The `:global(.pane[data-theme="dark"])` and matching light
    // selector are what take the data-theme attribute and apply the
    // token palette at pane scope. Without these the attribute is
    // inert and the global theme keeps winning.
    expect(app).toContain(":global(.pane[data-theme=\"dark\"])");
    expect(app).toContain(":global(.pane[data-theme=\"light\"])");
  });
});
