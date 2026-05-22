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
});
