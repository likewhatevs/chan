import { describe, expect, test } from "vitest";
import { renderTable } from "./shortcuts";

describe("shortcut table", () => {
  // `fullstack-42` dropped the standalone Terminal chord; Pane Mode
  // (Cmd+K 1) is the only way in. Guard the rendered table against
  // re-introducing either the web or native Terminal binding.
  // `Terminal rich prompt` is a sibling label, so the guards are
  // anchored to a row whose label is EXACTLY `Terminal` (followed
  // by any number of spaces and then a chord token) rather than
  // any line that happens to begin with the word.
  test("does not advertise a standalone Terminal chord (web)", () => {
    const table = renderTable("web", "mac");
    expect(table).not.toMatch(/^Terminal +Cmd\+Alt\+T\b/m);
    expect(table).not.toMatch(/^Terminal +Cmd\+`\b/m);
    expect(table).not.toMatch(/^Terminal +Cmd\+T\b/m);
  });

  test("does not advertise a standalone Terminal chord (native)", () => {
    const table = renderTable("native", "mac");
    expect(table).not.toMatch(/^Terminal +Cmd\+T\b/m);
    expect(table).not.toMatch(/^Terminal +Cmd\+Alt\+T\b/m);
  });

  test("advertises Pane Mode (Cmd+K) as the canonical spawn surface", () => {
    const web = renderTable("web", "mac");
    const native = renderTable("native", "mac");
    expect(web).toMatch(/^Enter Pane Mode\s+Cmd\+K$/m);
    expect(native).toMatch(/^Enter Pane Mode\s+Cmd\+K$/m);
  });

  test("close-tab chord is Ctrl+D on both platforms (per fullstack-41)", () => {
    const web = renderTable("web", "mac");
    const native = renderTable("native", "mac");
    expect(web).toMatch(/^Close tab\s+Ctrl\+D/m);
    expect(native).toMatch(/^Close tab\s+Ctrl\+D/m);
  });
});
