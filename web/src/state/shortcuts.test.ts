import { describe, expect, test } from "vitest";
import { renderTable } from "./shortcuts";

describe("shortcut table", () => {
  test("shows web terminal binding on Cmd+Alt+T", () => {
    const table = renderTable("web", "mac");

    expect(table).toMatch(/^Terminal\s+Cmd\+Alt\+T$/m);
    expect(table).not.toMatch(/^Terminal\s+Cmd\+`$/m);
  });

  test("shows native terminal binding on Cmd+T", () => {
    const table = renderTable("native", "mac");

    expect(table).toMatch(/^Terminal\s+Cmd\+T$/m);
  });
});
