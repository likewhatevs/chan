export type SlideAspectRatio = "16:9" | "4:3";

export type SlidesSpec = {
  aspectRatio: SlideAspectRatio;
  zoomFactor: number;
};

export type SlideOutlinePage<T> = {
  number: number;
  headings: T[];
};

export type SlidePage = {
  number: number;
  startLine: number;
  endLine: number;
  markdown: string;
};

const SUPPORTED_ASPECT_RATIOS = new Set<string>(["16:9", "4:3"]);
const DEFAULT_SLIDE_ASPECT_RATIO: SlideAspectRatio = "16:9";
const DEFAULT_SLIDE_ZOOM_FACTOR = 2;
const PAGE_BREAK_RE =
  /^\s*(?:<hr\b(?=[^>]*\bclass=(["'])chan-page-break\1)[^>]*\/?>|@pagebreak)\s*$/i;

export function parseSlidesSpec(source: string): SlidesSpec | null {
  const frontmatter = frontmatterBody(source);
  if (frontmatter === null) return null;

  let inChan = false;
  let inSlides = false;
  let slidesIndent = -1;
  let kind: string | null = null;
  let aspectRatio = DEFAULT_SLIDE_ASPECT_RATIO;
  let hasInvalidAspectRatio = false;
  let zoomFactor = DEFAULT_SLIDE_ZOOM_FACTOR;

  for (const rawLine of frontmatter.split(/\r?\n/)) {
    const trimmed = rawLine.trim();
    if (trimmed.length === 0 || trimmed.startsWith("#")) continue;

    const indent = rawLine.match(/^\s*/)?.[0].length ?? 0;
    if (indent === 0) {
      inChan = trimmed === "chan:";
      inSlides = false;
      slidesIndent = -1;
      continue;
    }
    if (!inChan) continue;
    if (inSlides && indent <= slidesIndent) {
      inSlides = false;
      slidesIndent = -1;
    }

    const field = trimmed.match(/^([A-Za-z_][\w-]*):(?:\s*(.*))?$/);
    if (!field) continue;
    const key = field[1] ?? "";
    const value = unquote((field[2] ?? "").trim());

    if (key === "kind") {
      kind = value.toLowerCase();
    } else if (key === "slides" && value.length === 0) {
      inSlides = true;
      slidesIndent = indent;
    } else if (inSlides && key === "aspect_ratio") {
      if (isSlideAspectRatio(value)) {
        aspectRatio = value;
      } else {
        hasInvalidAspectRatio = true;
      }
    } else if (inSlides && key === "zoom_factor") {
      zoomFactor = parseZoomFactor(value) ?? zoomFactor;
    }
  }

  if (kind !== "slides" || hasInvalidAspectRatio) return null;
  return { aspectRatio, zoomFactor };
}

export function groupHeadingsBySlides<T extends { line: number }>(
  source: string,
  headings: T[],
): SlideOutlinePage<T>[] {
  const breakLines = slidePageBreakLines(source);
  const pages: SlideOutlinePage<T>[] = Array.from(
    { length: breakLines.length + 1 },
    (_, index) => ({ number: index + 1, headings: [] }),
  );
  let pageIndex = 0;

  for (const heading of headings) {
    while (pageIndex < breakLines.length && heading.line > breakLines[pageIndex]!) {
      pageIndex++;
    }
    pages[pageIndex]!.headings.push(heading);
  }

  return pages;
}

export function splitSlidePages(source: string): SlidePage[] {
  const lines = source.split(/\r?\n/);
  const bodyStart = frontmatterEndLine(lines);
  const pages: SlidePage[] = [];
  let startLine = bodyStart;
  let pageLines: string[] = [];

  for (let i = bodyStart; i < lines.length; i++) {
    const line = lines[i] ?? "";
    if (PAGE_BREAK_RE.test(line)) {
      pages.push(makeSlidePage(pages.length + 1, startLine, i - 1, pageLines));
      startLine = i + 1;
      pageLines = [];
      continue;
    }
    pageLines.push(line);
  }

  pages.push(
    makeSlidePage(pages.length + 1, startLine, lines.length - 1, pageLines),
  );
  return pages;
}

export function slideIndexForLine(
  pages: readonly Pick<SlidePage, "startLine">[],
  line: number | null,
): number {
  if (pages.length === 0 || line === null) return 0;
  let index = 0;

  for (let i = 0; i < pages.length; i++) {
    if (line >= pages[i]!.startLine) index = i;
    else break;
  }

  return index;
}

function frontmatterBody(source: string): string | null {
  const match = source.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);
  return match?.[1] ?? null;
}

function frontmatterEndLine(lines: string[]): number {
  if (!/^---\s*$/.test(lines[0] ?? "")) return 0;
  for (let i = 1; i < lines.length; i++) {
    if (/^---\s*$/.test(lines[i] ?? "")) return i + 1;
  }
  return 0;
}

function unquote(value: string): string {
  if (value.length < 2) return value;
  const first = value[0];
  const last = value[value.length - 1];
  if ((first === '"' && last === '"') || (first === "'" && last === "'")) {
    return value.slice(1, -1);
  }
  return value;
}

function isSlideAspectRatio(value: string): value is SlideAspectRatio {
  return SUPPORTED_ASPECT_RATIOS.has(value);
}

function parseZoomFactor(value: string): number | null {
  const trimmed = value.trim();
  const percent = trimmed.match(/^([0-9]+(?:\.[0-9]+)?)%$/);
  if (percent) return positiveNumber(Number(percent[1]) / 100);

  const multiplier = trimmed.match(/^([0-9]+(?:\.[0-9]+)?)$/);
  if (multiplier) return positiveNumber(Number(multiplier[1]));

  return null;
}

function positiveNumber(value: number): number | null {
  return Number.isFinite(value) && value > 0 ? value : null;
}

function slidePageBreakLines(source: string): number[] {
  const lines = source.split(/\r?\n/);
  const out: number[] = [];

  for (let i = 0; i < lines.length; i++) {
    if (PAGE_BREAK_RE.test(lines[i] ?? "")) out.push(i);
  }

  return out;
}

function makeSlidePage(
  number: number,
  startLine: number,
  endLine: number,
  lines: string[],
): SlidePage {
  return {
    number,
    startLine,
    endLine,
    markdown: lines.join("\n"),
  };
}
