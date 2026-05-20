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

  test("Pane hamburger renders the theme-toggle entry (fullstack-a-27 relocation)", () => {
    // `fullstack-a-27` moved the theme toggle from a standalone
    // `class="pane-theme-toggle"` button in the pane chrome into
    // the hamburger menu (Hybrid panes only). The handler reference
    // is the load-bearing pin — wherever the toggle lives, this
    // string must appear so the function itself stays wired.
    expect(pane).toContain("togglePaneTheme");
  });

  test("togglePaneTheme cycles between follow-global and the inverse override", () => {
    // The toggle's contract is the only place the user can set
    // `pane.theme` from the UI; if it stops calling
    // `scheduleSessionSave` or stops cycling through `undefined`,
    // the round-trip + UX both break.
    expect(pane).toContain("function togglePaneTheme()");
    expect(pane).toContain("pane.theme = ui.theme === \"dark\" ? \"light\" : \"dark\"");
    expect(pane).toContain("pane.theme = undefined");
    expect(pane).toContain("scheduleSessionSave()");
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
