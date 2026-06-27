import { describe, it, expect } from "vitest";
import { basename, rowLabel } from "./windowLabel";

describe("basename", () => {
  it("returns the trailing component", () => {
    expect(basename("/Users/x/notes")).toBe("notes");
  });
  it("tolerates a trailing slash", () => {
    expect(basename("/Users/x/notes/")).toBe("notes");
  });
  it("handles empty and root", () => {
    expect(basename("")).toBe("");
    expect(basename("/")).toBe("");
  });
});

describe("rowLabel recompose from kind/ordinal", () => {
  it("names a terminal window by ordinal", () => {
    expect(rowLabel("terminal", 2)).toBe("Terminal Window 2");
  });
  it("names a workspace window as just Window N (its card names the workspace)", () => {
    expect(rowLabel("workspace", 1)).toBe("Window 1");
  });
});
