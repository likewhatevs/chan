// @vitest-environment jsdom

import { describe, expect, test, vi } from "vitest";
import { renderExcalidraw } from "./excalidraw_render";

// Mock the two heavy libraries so the render path is exercised without
// loading React / excalidraw. parseMermaidToExcalidraw throws on a sentinel
// source so the error branch is covered; exportToSvg returns a real (jsdom)
// SVG element so `.outerHTML` produces an <svg> string.
vi.mock("@excalidraw/mermaid-to-excalidraw", () => ({
  parseMermaidToExcalidraw: async (def: string) => {
    if (def.includes("BOOM")) {
      throw new Error("Parse error on line 2, column 3: unexpected token");
    }
    return { elements: [{ type: "rectangle" }], files: {} };
  },
}));
vi.mock("@excalidraw/excalidraw", () => ({
  convertToExcalidrawElements: (els: unknown[]) => els,
  exportToSvg: async () => {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("data-rendered", "1");
    return svg;
  },
}));

describe("renderExcalidraw", () => {
  test("returns the exported SVG string on success", async () => {
    const res = await renderExcalidraw("flowchart TD\n  A --> B", false);
    expect(res.ok).toBe(true);
    expect(res.svg).toContain("<svg");
    expect(res.svg).toContain('data-rendered="1"');
  });

  test("resolves to a structured error on a parse failure (never throws)", async () => {
    const res = await renderExcalidraw("BOOM", false);
    expect(res.ok).toBe(false);
    expect(res.error).toContain("Parse error");
    // mermaid-to-excalidraw parses mermaid underneath, so its "line N,
    // column M" is surfaced for the failing-source accent.
    expect(res.errorLine).toBe(2);
    expect(res.errorCol).toBe(3);
  });
});
