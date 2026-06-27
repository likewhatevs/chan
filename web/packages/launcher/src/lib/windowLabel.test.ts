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

describe("rowLabel recompose from kind/ordinal/workspace_path", () => {
  it("names a terminal window by ordinal", () => {
    expect(rowLabel("terminal", 2, null)).toBe("Terminal Window 2");
  });
  it("names a workspace window by its folder base", () => {
    expect(rowLabel("workspace", 1, "/srv/api")).toBe("api Window 1");
  });
  it("falls back when a workspace path is missing", () => {
    expect(rowLabel("workspace", 3, null)).toBe("Workspace Window 3");
  });
});
