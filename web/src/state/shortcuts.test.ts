import { describe, expect, test } from "vitest";
import { renderTable } from "./shortcuts";

describe("shortcut table", () => {
  // `fullstack-42` dropped the standalone Terminal chord in favour
  // of Pane Mode (Cmd+K 1). `fullstack-b-2` brought it back under
  // a distinct label ("New terminal") so power users can spawn a
  // terminal with one chord without entering Pane Mode first.
  // Cmd+T on native everywhere; Cmd+Alt+T on web (macOS only —
  // Ctrl+Alt+T on Win/Linux web is owned by tab.reopenClosed).
  test("advertises New terminal under Cmd+Alt+T (web mac)", () => {
    const table = renderTable("web", "mac");
    expect(table).toMatch(/^New terminal +Cmd\+Alt\+T\b/m);
  });

  test("advertises New terminal under Cmd+T (native mac)", () => {
    const table = renderTable("native", "mac");
    expect(table).toMatch(/^New terminal +Cmd\+T\b/m);
  });

  // Sibling label `Terminal rich prompt` shouldn't ever sit under
  // the bare-Terminal regex; the guards below anchor to that exact
  // word to catch an accidental rename or duplicate entry.
  test("does not advertise a bare 'Terminal' row (web)", () => {
    const table = renderTable("web", "mac");
    expect(table).not.toMatch(/^Terminal +Cmd\+Alt\+T\b/m);
    expect(table).not.toMatch(/^Terminal +Cmd\+`\b/m);
    expect(table).not.toMatch(/^Terminal +Cmd\+T\b/m);
  });

  test("does not advertise a bare 'Terminal' row (native)", () => {
    const table = renderTable("native", "mac");
    expect(table).not.toMatch(/^Terminal +Cmd\+T\b/m);
    expect(table).not.toMatch(/^Terminal +Cmd\+Alt\+T\b/m);
  });

  test("advertises Hybrid Nav (Cmd+.) as the canonical spawn surface", () => {
    // Chord swapped to Cmd+. per `fullstack-a-7` so Cmd+, can own
    // Settings (macOS convention). `fullstack-a-68 slice 1` +
    // `slice 1b` demoted the label from all-caps "NAV" to
    // title-case "Nav".
    const web = renderTable("web", "mac");
    const native = renderTable("native", "mac");
    expect(web).toMatch(/^Enter Hybrid Nav\s+Cmd\+\.$/m);
    expect(native).toMatch(/^Enter Hybrid Nav\s+Cmd\+\.$/m);
  });

  test("close-tab chord is Ctrl+D on both platforms (per fullstack-41)", () => {
    const web = renderTable("web", "mac");
    const native = renderTable("native", "mac");
    expect(web).toMatch(/^Close tab\s+Ctrl\+D/m);
    expect(native).toMatch(/^Close tab\s+Ctrl\+D/m);
  });

  // `phase-12 lane-e` (addendum-2 Q5): pane nav splits per platform.
  // The web build uses Alt+[/] (Cmd+[/] is browser back/forward there);
  // desktop-native keeps Cmd+[/].
  test("advertises pane nav: web Alt+[/], native Cmd+[/]", () => {
    const web = renderTable("web", "mac");
    const native = renderTable("native", "mac");
    expect(web).toMatch(/^Previous pane\s+Alt\+\[/m);
    expect(web).toMatch(/^Next pane\s+Alt\+\]/m);
    expect(native).toMatch(/^Previous pane\s+Cmd\+\[/m);
    expect(native).toMatch(/^Next pane\s+Cmd\+\]/m);
  });

  // `phase-12 lane-e` (addendum-2): Cmd+S = workspace-wide search,
  // reclaimed after fullstack-56 dropped it.
  test("advertises Cmd+S search on both platforms", () => {
    expect(renderTable("web", "mac")).toMatch(/^Search\s+Cmd\+S/m);
    expect(renderTable("native", "mac")).toMatch(/^Search\s+Cmd\+S/m);
  });

  // `phase-12 lane-e` (addendum-2 Q8): direct Cmd+I infographics chord
  // (in addition to Hybrid Nav `i`).
  test("advertises Cmd+I infographics on both platforms", () => {
    expect(renderTable("web", "mac")).toMatch(/^Infographics\s+Cmd\+I/m);
    expect(renderTable("native", "mac")).toMatch(/^Infographics\s+Cmd\+I/m);
  });

  // `phase-12 lane-e` (addendum-2): splits are desktop-native only
  // (web reaches them via Hybrid Nav `/` `\`), so they render in the
  // native table but not the web one.
  test("advertises splits on native only", () => {
    const web = renderTable("web", "mac");
    const native = renderTable("native", "mac");
    expect(native).toMatch(/^Split right\s+Cmd\+\//m);
    expect(native).toMatch(/^Split bottom\s+Cmd\+\\/m);
    expect(web).not.toMatch(/^Split right/m);
    expect(web).not.toMatch(/^Split bottom/m);
  });

  test("advertises Hybrid Nav close-all and kill-pane chords", () => {
    const table = renderTable("web", "mac");
    expect(table).toMatch(/^Close all tabs in pane\s+Cmd\+\. x/m);
    expect(table).toMatch(/^Kill pane\s+Cmd\+\. Backspace/m);
  });

  test("advertises screen lock only through Hybrid Nav", () => {
    const table = renderTable("web", "mac");
    expect(table).toMatch(/^Lock screen\s+Cmd\+\. L/m);
    expect(table).not.toMatch(/^Lock screen\s+Cmd\+L/m);
  });
});
