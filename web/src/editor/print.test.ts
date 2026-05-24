// @vitest-environment jsdom

import { describe, expect, test } from "vitest";
import {
  buildPrintDocumentHtml,
  renderPrintableMarkdown,
} from "./print";

describe("renderPrintableMarkdown", () => {
  test("keeps the page-break marker as printable HTML", () => {
    const html = renderPrintableMarkdown(
      'before\n\n<hr class="chan-page-break">\n\nafter',
      "notes/doc.md",
    );

    expect(html).toContain('class="chan-page-break"');
  });

  test("resolves local markdown images through the file endpoint", () => {
    const html = renderPrintableMarkdown("![alt](../img/pic.png#w=240)", "notes/doc.md");

    expect(html).toContain('src="/api/files/img/pic.png"');
    expect(html).toContain("width: 240px");
  });
});

describe("buildPrintDocumentHtml", () => {
  test("includes the page-break print rules", () => {
    const html = buildPrintDocumentHtml({
      title: "doc.md",
      path: "doc.md",
      markdown: '<hr class="chan-page-break">',
      pageWidthRatio: 0.75,
    });

    expect(html).toContain("break-after: page");
    expect(html).toContain("page-break-after: always");
    expect(html).toContain("max-width: 75%");
  });
});
