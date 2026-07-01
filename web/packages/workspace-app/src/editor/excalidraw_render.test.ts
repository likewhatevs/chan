// @vitest-environment jsdom

import { describe, expect, test, vi } from "vitest";
import { renderExcalidraw } from "./excalidraw_render";

// Mock the two heavy libraries so the render path is exercised without
// loading React / excalidraw. parseMermaidToExcalidraw throws on sentinel
// sources so both error branches are covered (a genuine parse error and the
// WebKit-only "SubGraph element not found"); exportToSvg returns a real
// (jsdom) SVG element so `.outerHTML` produces an <svg> string.
vi.mock("@excalidraw/mermaid-to-excalidraw", () => ({
  parseMermaidToExcalidraw: async (def: string) => {
    if (def.includes("BOOM")) {
      throw new Error("Parse error on line 2, column 3: unexpected token");
    }
    if (def.includes("SUBGRAPH")) {
      throw new Error("SubGraph element not found");
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
// Mock the fallback renderer so the excalidraw error path never loads real
// mermaid. It "renders" any source that is valid mermaid and fails only on
// the BOOM sentinel, mirroring how the plain mermaid renderer draws a valid
// flowchart-with-subgraph but rejects genuinely bad source.
vi.mock("./mermaid_render", () => ({
  renderMermaid: async (source: string) => {
    if (source.includes("BOOM")) {
      return { ok: false, error: "mermaid parse failed" };
    }
    return { ok: true, svg: '<svg data-mermaid="1"></svg>' };
  },
}));

describe("renderExcalidraw", () => {
  test("returns the exported SVG string on success", async () => {
    const res = await renderExcalidraw("flowchart TD\n  A --> B", false);
    expect(res.ok).toBe(true);
    expect(res.svg).toContain("<svg");
    expect(res.svg).toContain('data-rendered="1"');
  });

  test("degrades to the mermaid renderer when the excalidraw conversion fails", async () => {
    // WebKit throws "SubGraph element not found" here; the source is valid
    // mermaid, so the block still shows a diagram via the mermaid fallback.
    const res = await renderExcalidraw("flowchart LR\n  subgraph SUBGRAPH\n    A --> B\n  end", false);
    expect(res.ok).toBe(true);
    expect(res.svg).toContain('data-mermaid="1"');
  });

  test("resolves to a structured error when mermaid also fails (never throws)", async () => {
    const res = await renderExcalidraw("BOOM", false);
    expect(res.ok).toBe(false);
    expect(res.error).toContain("Parse error");
    // mermaid-to-excalidraw parses mermaid underneath, so its "line N,
    // column M" is surfaced for the failing-source accent.
    expect(res.errorLine).toBe(2);
    expect(res.errorCol).toBe(3);
  });
});
