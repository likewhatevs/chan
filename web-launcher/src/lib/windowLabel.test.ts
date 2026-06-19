import { describe, it, expect } from "vitest";
import {
  basename,
  libraryIcon,
  librarySectionLabel,
  rowLabel,
  shortLibraryId,
} from "./windowLabel";

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

describe("library section labels", () => {
  it("marks the local library with a home icon", () => {
    expect(libraryIcon("local")).toBe("🏠");
    expect(librarySectionLabel("local", null)).toBe("🏠 Local");
  });
  it("marks a remote library with an arrow and the resolved name", () => {
    expect(libraryIcon("lib-abc")).toBe("↗");
    expect(librarySectionLabel("lib-abc", "prod")).toBe("↗ prod");
  });
  it("falls back to a short id when the remote name is unknown", () => {
    expect(librarySectionLabel("lib-7f3a9c21b40d8e65", null)).toBe("↗ 7f3a9c21...");
  });
});

describe("shortLibraryId", () => {
  it("strips the lib- prefix and truncates", () => {
    expect(shortLibraryId("lib-7f3a9c21b40d8e65")).toBe("7f3a9c21...");
  });
  it("leaves a short id intact", () => {
    expect(shortLibraryId("lib-abc123")).toBe("abc123");
  });
});
