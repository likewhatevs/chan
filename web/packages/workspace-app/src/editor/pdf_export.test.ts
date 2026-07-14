// @vitest-environment jsdom

import { PDFDocument } from "pdf-lib";
import { afterEach, describe, expect, test, vi } from "vitest";
import { DECK_PAGE_BOX_PX, DOC_CONTENT_WIDTH_PX, docPageGeometry } from "./pdf_pages";
import { type PageBoxPx, type PageSnapshot } from "./pdf_snapshot";
import { cssColorToRgb01, exportMarkdownToPdf, pdfFilenameFor } from "./pdf_export";

vi.mock("./mermaid_render", () => ({
  renderMermaid: vi.fn(async () => ({ ok: true, svg: "<svg></svg>" })),
}));
vi.mock("./excalidraw_render", () => ({
  renderExcalidraw: vi.fn(async () => ({ ok: true, svg: "<svg></svg>" })),
  renderExcalidrawFile: vi.fn(async () => ({ ok: true, svg: "<svg></svg>" })),
}));

// A valid 1x1 PNG so pdf-lib's embedPng accepts the fake raster.
const TINY_PNG = Uint8Array.from(
  atob(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
  ),
  (c) => c.charCodeAt(0),
);

function fakeRasterizer(calls: PageBoxPx[]) {
  return async (_root: HTMLElement, box: PageBoxPx): Promise<PageSnapshot> => {
    calls.push(box);
    return { png: TINY_PNG, widthPx: 2, heightPx: 2 };
  };
}

const DECK = `---
chan:
  kind: slides
  slides:
    aspect_ratio: "16:9"
---

# One

<hr class="chan-page-break">

# Two

<hr class="chan-page-break">

# Three
`;

afterEach(() => {
  document.body.innerHTML = "";
});

describe("pdfFilenameFor", () => {
  test("swaps the extension for .pdf", () => {
    expect(pdfFilenameFor("notes/report.md")).toBe("report.pdf");
    expect(pdfFilenameFor("deck.slides.md")).toBe("deck.slides.pdf");
  });

  test("appends .pdf when the basename has no extension", () => {
    expect(pdfFilenameFor("dir/readme")).toBe("readme.pdf");
  });

  test("falls back on empty input", () => {
    expect(pdfFilenameFor("")).toBe("document.pdf");
    expect(pdfFilenameFor("dir/")).toBe("document.pdf");
  });
});

describe("cssColorToRgb01", () => {
  test("parses computed rgb()/rgba() strings", () => {
    expect(cssColorToRgb01("rgb(255, 0, 0)")).toEqual({ r: 1, g: 0, b: 0 });
    expect(cssColorToRgb01("rgba(0, 128, 255, 0.5)")).toEqual({
      r: 0,
      g: 128 / 255,
      b: 1,
    });
  });

  test("parses hex literals and falls back to white", () => {
    expect(cssColorToRgb01("#fff")).toEqual({ r: 1, g: 1, b: 1 });
    expect(cssColorToRgb01("#111827").r).toBeCloseTo(0x11 / 255, 6);
    expect(cssColorToRgb01("nonsense")).toEqual({ r: 1, g: 1, b: 1 });
  });
});

describe("exportMarkdownToPdf", () => {
  test("a deck exports one A4 landscape page per slide", async () => {
    const calls: PageBoxPx[] = [];
    const bytes = await exportMarkdownToPdf(
      { path: "deck.md", markdown: DECK, theme: "dark" },
      { rasterize: fakeRasterizer(calls) },
    );

    const pdf = await PDFDocument.load(bytes);
    expect(pdf.getPageCount()).toBe(3);
    for (const page of pdf.getPages()) {
      expect(page.getWidth()).toBeCloseTo(841.89, 2);
      expect(page.getHeight()).toBeCloseTo(595.28, 2);
    }
    expect(calls).toHaveLength(3);
    expect(calls[0]!.widthPx).toBeCloseTo(DECK_PAGE_BOX_PX.widthPx, 4);
    expect(calls[0]!.heightPx).toBeCloseTo(DECK_PAGE_BOX_PX.heightPx, 4);
  });

  test("a document exports paginated A4 portrait pages", async () => {
    const calls: PageBoxPx[] = [];
    const bytes = await exportMarkdownToPdf(
      {
        path: "notes/doc.md",
        markdown: "# Title\n\nbody\n",
        theme: "light",
      },
      { rasterize: fakeRasterizer(calls) },
    );

    const pdf = await PDFDocument.load(bytes);
    expect(pdf.getPageCount()).toBe(1);
    const page = pdf.getPage(0);
    expect(page.getWidth()).toBeCloseTo(595.28, 2);
    expect(page.getHeight()).toBeCloseTo(841.89, 2);
    expect(calls).toHaveLength(1);
    expect(calls[0]!.widthPx).toBe(DOC_CONTENT_WIDTH_PX);
    expect(calls[0]!.heightPx).toBeCloseTo(
      docPageGeometry().pageContentHeightPx,
      4,
    );
    // The offscreen measuring host is cleaned up.
    expect(document.body.children).toHaveLength(0);
  });

  test("a rasterizer failure propagates instead of hanging", async () => {
    await expect(
      exportMarkdownToPdf(
        { path: "doc.md", markdown: "text\n", theme: "light" },
        {
          rasterize: async () => {
            throw new Error("raster blew up");
          },
        },
      ),
    ).rejects.toThrow("raster blew up");
    expect(document.body.children).toHaveLength(0);
  });
});
