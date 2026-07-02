import { describe, expect, test } from "vitest";
import {
  groupHeadingsBySlides,
  parseSlidesSpec,
  slideIndexForLine,
  splitSlidePages,
} from "./slides";

type TestHeading = {
  line: number;
  text: string;
};

describe("parseSlidesSpec", () => {
  test("recognizes the slides frontmatter contract", () => {
    const source = `---
chan:
  kind: slides
  slides:
    aspect_ratio: "16:9"
---

# Title
`;

    expect(parseSlidesSpec(source)).toEqual({ aspectRatio: "16:9", zoomFactor: 2 });
  });

  test("accepts the standard 4:3 aspect ratio", () => {
    const source = `---
chan:
  kind: slides
  slides:
    aspect_ratio: '4:3'
---
`;

    expect(parseSlidesSpec(source)).toEqual({ aspectRatio: "4:3", zoomFactor: 2 });
  });

  test("defaults to 16:9 when the aspect ratio is omitted", () => {
    const source = `---
chan:
  kind: slides
  slides:
    zoom_factor: 200%
---
`;

    expect(parseSlidesSpec(source)).toEqual({ aspectRatio: "16:9", zoomFactor: 2 });
  });

  test("accepts a percentage zoom factor", () => {
    const source = `---
chan:
  kind: slides
  slides:
    aspect_ratio: "16:9"
    zoom_factor: 200%
---
`;

    expect(parseSlidesSpec(source)).toEqual({ aspectRatio: "16:9", zoomFactor: 2 });
  });

  test("keeps numeric zoom factors compatible with the earlier contract", () => {
    const source = `---
chan:
  kind: slides
  slides:
    aspect_ratio: "16:9"
    zoom_factor: 4
---
`;

    expect(parseSlidesSpec(source)).toEqual({ aspectRatio: "16:9", zoomFactor: 4 });
  });

  test("ignores non-slide frontmatter", () => {
    const source = `---
chan:
  kind: contact
---

# Alice
`;

    expect(parseSlidesSpec(source)).toBeNull();
  });

  test("requires a supported aspect ratio", () => {
    const source = `---
chan:
  kind: slides
  slides:
    aspect_ratio: "1:1"
---
`;

    expect(parseSlidesSpec(source)).toBeNull();
  });
});

describe("groupHeadingsBySlides", () => {
  test("groups headings under slide pages split by page breaks", () => {
    const source = `---
chan:
  kind: slides
  slides:
    aspect_ratio: "16:9"
---

# Slide 1
## Context
<hr class="chan-page-break">
# Slide 2
### Detail
@pagebreak
# Slide 3
`;

    expect(pageHeadingText(source)).toEqual([
      { number: 1, headings: ["Slide 1", "Context"] },
      { number: 2, headings: ["Slide 2", "Detail"] },
      { number: 3, headings: ["Slide 3"] },
    ]);
  });

  test("merges headings into the previous slide when a page break is removed", () => {
    const source = `---
chan:
  kind: slides
  slides:
    aspect_ratio: "16:9"
---

# Slide 1
## Context
# Slide 2
### Detail
<hr class="chan-page-break">
# Slide 3
`;

    expect(pageHeadingText(source)).toEqual([
      { number: 1, headings: ["Slide 1", "Context", "Slide 2", "Detail"] },
      { number: 2, headings: ["Slide 3"] },
    ]);
  });
});

describe("splitSlidePages", () => {
  test("splits slide markdown without rendering frontmatter or page breaks", () => {
    const source = `---
chan:
  kind: slides
  slides:
    aspect_ratio: "16:9"
---

# Slide 1

one

<hr class="chan-page-break">

# Slide 2

two

@pagebreak

# Slide 3
`;

    expect(splitSlidePages(source)).toEqual([
      { number: 1, startLine: 6, endLine: 10, markdown: "\n# Slide 1\n\none\n" },
      { number: 2, startLine: 12, endLine: 16, markdown: "\n# Slide 2\n\ntwo\n" },
      { number: 3, startLine: 18, endLine: 20, markdown: "\n# Slide 3\n" },
    ]);
  });

  test("preserves blank lines inside slide boundaries", () => {
    const source = `---
chan:
  kind: slides
  slides:
    aspect_ratio: "16:9"
---


# Lower title



body


<hr class="chan-page-break">

# Next
`;

    expect(splitSlidePages(source)[0]?.markdown).toBe(
      "\n\n# Lower title\n\n\n\nbody\n\n",
    );
  });

  test("finds the current slide from the caret line", () => {
    const pages = splitSlidePages(`# Slide 1
text
<hr class="chan-page-break">
# Slide 2
text
<hr class="chan-page-break">
# Slide 3
`);

    expect(slideIndexForLine(pages, 0)).toBe(0);
    expect(slideIndexForLine(pages, 2)).toBe(0);
    expect(slideIndexForLine(pages, 3)).toBe(1);
    expect(slideIndexForLine(pages, 6)).toBe(2);
    expect(slideIndexForLine(pages, null)).toBe(0);
  });
});

function pageHeadingText(source: string): Array<{ number: number; headings: string[] }> {
  return groupHeadingsBySlides(source, testHeadings(source)).map((page) => ({
    number: page.number,
    headings: page.headings.map((heading) => heading.text),
  }));
}

function testHeadings(source: string): TestHeading[] {
  const out: TestHeading[] = [];

  source.split(/\r?\n/).forEach((line, index) => {
    const match = line.match(/^(#{1,6})\s+(.+?)\s*#*\s*$/);
    if (match) out.push({ line: index, text: match[2]!.trim() });
  });

  return out;
}
