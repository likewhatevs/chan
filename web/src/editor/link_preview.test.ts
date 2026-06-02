import { describe, expect, test } from "vitest";
import { resolvePreviewTarget } from "./link_preview";

describe("resolvePreviewTarget", () => {
  test("bare wiki stem expands to a .md note path", () => {
    expect(resolvePreviewTarget("bullets")).toBe("bullets.md");
    expect(resolvePreviewTarget("Contacts/alice")).toBe("Contacts/alice.md");
  });

  test("an existing known extension is preserved", () => {
    expect(resolvePreviewTarget("notes.md")).toBe("notes.md");
    expect(resolvePreviewTarget("diagram.png")).toBe("diagram.png");
    expect(resolvePreviewTarget("readme.txt")).toBe("readme.txt");
  });

  test("a #heading anchor is dropped before resolving", () => {
    expect(resolvePreviewTarget("bullets#intro")).toBe("bullets.md");
    expect(resolvePreviewTarget("notes.md#top")).toBe("notes.md");
  });

  test("an unknown extension is treated as a stem (gets .md)", () => {
    // A dotted stem with no recognized doc extension still resolves to a
    // note rather than reading a bare, likely-missing path.
    expect(resolvePreviewTarget("v1.2-plan")).toBe("v1.2-plan.md");
  });
});
