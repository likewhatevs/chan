// @vitest-environment jsdom

import { describe, expect, test } from "vitest";
import {
  DECK_PAGE_BOX_PX,
  DOC_CONTENT_WIDTH_PX,
  buildDocPageElements,
  docPageGeometry,
  normalizeDocPageBreaks,
  paginateDocBlocks,
  slideBoxFit,
  type DocBlockRect,
} from "./pdf_pages";

function block(
  top: number,
  bottom: number,
  opts: Partial<DocBlockRect> = {},
): DocBlockRect {
  return { top, bottom, heading: false, pageBreak: false, ...opts };
}

describe("docPageGeometry", () => {
  test("derives point/pixel geometry from A4 and the fixed content width", () => {
    const g = docPageGeometry();
    expect(g.printableWidthPt).toBeCloseTo(595.28 - 2 * 46.8, 2);
    expect(g.printableHeightPt).toBeCloseTo(841.89 - 2 * 46.8, 2);
    expect(g.ptPerPx).toBeCloseTo(g.printableWidthPt / DOC_CONTENT_WIDTH_PX, 6);
    expect(g.pageContentHeightPx).toBeCloseTo(
      g.printableHeightPt / g.ptPerPx,
      4,
    );
  });
});

describe("paginateDocBlocks", () => {
  const PAGE = 1000;

  test("everything fitting yields one window ending at the last bottom", () => {
    const windows = paginateDocBlocks(
      [block(0, 300), block(320, 700), block(720, 950)],
      PAGE,
    );
    expect(windows).toEqual([{ startPx: 0, endPx: 950 }]);
  });

  test("a block crossing the boundary moves whole to the next page", () => {
    const windows = paginateDocBlocks(
      [block(0, 600), block(620, 1200)],
      PAGE,
    );
    expect(windows).toEqual([
      { startPx: 0, endPx: 620 },
      { startPx: 620, endPx: 1200 },
    ]);
  });

  test("a cut shifts up past the headings directly above it", () => {
    const windows = paginateDocBlocks(
      [
        block(0, 700),
        block(720, 780, { heading: true }),
        block(800, 860, { heading: true }),
        block(880, 1400),
      ],
      PAGE,
    );
    // The cut before the overflowing block pulls both headings with it.
    expect(windows[0]).toEqual({ startPx: 0, endPx: 720 });
    expect(windows[1]).toEqual({ startPx: 720, endPx: 1400 });
  });

  test("a heading at the window start never shifts the cut to zero width", () => {
    const windows = paginateDocBlocks(
      [block(0, 80, { heading: true }), block(100, 1600)],
      PAGE,
    );
    expect(windows[0]).toEqual({ startPx: 0, endPx: 100 });
    expect(windows[1]).toEqual({ startPx: 100, endPx: 1100 });
    expect(windows[2]).toEqual({ startPx: 1100, endPx: 1600 });
  });

  test("an oversized single block hard-cuts at page height", () => {
    const windows = paginateDocBlocks([block(0, 2500)], PAGE);
    expect(windows).toEqual([
      { startPx: 0, endPx: 1000 },
      { startPx: 1000, endPx: 2000 },
      { startPx: 2000, endPx: 2500 },
    ]);
  });

  test("a page-break block forces a cut at its position", () => {
    const windows = paginateDocBlocks(
      [
        block(0, 200),
        block(210, 210, { pageBreak: true }),
        block(220, 500),
      ],
      PAGE,
    );
    expect(windows).toEqual([
      { startPx: 0, endPx: 210 },
      { startPx: 210, endPx: 500 },
    ]);
  });

  test("an empty document still yields one window", () => {
    expect(paginateDocBlocks([], PAGE)).toEqual([{ startPx: 0, endPx: 0 }]);
  });
});

describe("normalizeDocPageBreaks", () => {
  test("rewrites @pagebreak lines and keeps break hrs verbatim", () => {
    const out = normalizeDocPageBreaks(
      'a\n@pagebreak\nb\n<hr class="chan-page-break">\nc @pagebreak c\n',
    );
    expect(out.split("\n")).toEqual([
      "a",
      '<hr class="chan-page-break">',
      "b",
      '<hr class="chan-page-break">',
      "c @pagebreak c",
      "",
    ]);
  });
});

describe("slideBoxFit", () => {
  test("16:9 fills the landscape width and letterboxes vertically", () => {
    const fit = slideBoxFit("16:9", DECK_PAGE_BOX_PX);
    expect(fit.widthPx).toBeCloseTo(DECK_PAGE_BOX_PX.widthPx, 4);
    expect(fit.heightPx).toBeCloseTo(DECK_PAGE_BOX_PX.widthPx / (16 / 9), 4);
    expect(fit.leftPx).toBeCloseTo(0, 4);
    expect(fit.topPx).toBeCloseTo(
      (DECK_PAGE_BOX_PX.heightPx - fit.heightPx) / 2,
      4,
    );
  });

  test("4:3 fills the landscape height and pillarboxes horizontally", () => {
    const fit = slideBoxFit("4:3", DECK_PAGE_BOX_PX);
    expect(fit.heightPx).toBeCloseTo(DECK_PAGE_BOX_PX.heightPx, 4);
    expect(fit.widthPx).toBeCloseTo(DECK_PAGE_BOX_PX.heightPx * (4 / 3), 4);
    expect(fit.topPx).toBeCloseTo(0, 4);
    expect(fit.leftPx).toBeCloseTo(
      (DECK_PAGE_BOX_PX.widthPx - fit.widthPx) / 2,
      4,
    );
  });
});

describe("buildDocPageElements", () => {
  test("windows become root clones with shifted content", () => {
    const root = document.createElement("div");
    root.className = "chan-print-page";
    const content = document.createElement("div");
    content.className = "chan-print-content";
    content.innerHTML = "<p>one</p><p>two</p>";
    root.appendChild(content);

    const pages = buildDocPageElements(
      { root, content, completion: Promise.resolve() },
      [
        { startPx: 0, endPx: 900 },
        { startPx: 900, endPx: 1400 },
      ],
      900,
    );

    expect(pages).toHaveLength(2);
    for (const page of pages) {
      expect(page.style.height).toBe("900px");
      expect(page.style.overflow).toBe("hidden");
    }
    expect(
      pages[0]!.querySelector<HTMLElement>(".chan-print-content")?.style
        .marginTop,
    ).toBe("0px");
    expect(
      pages[1]!.querySelector<HTMLElement>(".chan-print-content")?.style
        .marginTop,
    ).toBe("-900px");
    // Clones are independent of the original.
    expect(root.style.height).toBe("");
  });
});
