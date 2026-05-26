import { describe, expect, test } from "vitest";
import canvas from "./GraphCanvas.svelte?raw";
import panel from "./GraphPanel.svelte?raw";

// Phase-11 Slice F edge palette (round-1 spec):
//   * directory->directory and directory->file containment edges stay
//     GREY (the `contains` kind, stroked in theme.folder),
//   * every OTHER edge matches its document type honouring the Graph
//     settings palette: tag/mention/language keep their own hue, and the
//     `link` (wiki/markdown reference) edge is coloured by the SOURCE
//     document's kind (markdown orange --g-doc, source royalblue
//     --g-source, etc.).
// These are source-pinning assertions in the same style as the existing
// graphDraftsStyling tests; the canvas paint context isn't unit-testable
// directly, so we pin the colour-resolution structure.

describe("Slice F: contains (dir->dir / dir->file) edges stay grey", () => {
  test("contains maps to theme.folder (grey)", () => {
    expect(canvas).toMatch(/kind === "contains" \? theme\.folder/);
  });

  test("contains rides the single-stroke-per-kind loop, not the link pass", () => {
    expect(canvas).toMatch(
      /\["tag", "mention", "contains", "language", "group", "drafts_link"\] as const/,
    );
  });
});

describe("Slice F: link edges coloured by source document type", () => {
  test("fileKindColor resolves the palette hue per node kind", () => {
    expect(canvas).toMatch(/function fileKindColor\(kind: DKind\): string/);
    // Markdown documents are orange (--g-doc); the round-1 example.
    expect(canvas).toMatch(/case "doc":\s*\n\s*return theme\.doc;/);
    // Source files royalblue (--g-source).
    expect(canvas).toMatch(/case "source":\s*\n\s*return theme\.source;/);
  });

  test("link edges are sub-grouped by source node kind", () => {
    expect(canvas).toMatch(/const linkByKind = new Map<string, DEdge\[\]>\(\);/);
    expect(canvas).toMatch(/for \(const e of edgesByKind\.link\)/);
  });

  test("each link sub-group is stroked with its source-kind colour", () => {
    expect(canvas).toMatch(
      /strokePass\(list, fileKindColor\(kind as DKind\), 0\.18\);/,
    );
  });
});

describe("Slice F: non-document edges keep their palette hue", () => {
  test("tag green / mention / language retain their own colours", () => {
    expect(canvas).toMatch(/kind === "tag" \? theme\.tag/);
    expect(canvas).toMatch(/kind === "mention" \? theme\.mention/);
    expect(canvas).toMatch(/kind === "language" \? theme\.language/);
  });
});

describe("Slice F: Graph reuses the FB per-directory pub/sub", () => {
  test("GraphPanel imports the fbWatch manager", () => {
    expect(panel).toMatch(/fbWatchRegister/);
    expect(panel).toMatch(/fbWatchReconcile/);
    expect(panel).toMatch(/fbWatchDispose/);
  });

  test("GraphPanel registers + disposes a watcher-scope instance", () => {
    expect(panel).toMatch(/fbWatchRegister\(id\)/);
    expect(panel).toMatch(/fbWatchDispose\(id\)/);
  });

  test("GraphPanel reconciles subscriptions against the displayed dirs", () => {
    expect(panel).toMatch(/const displayedDirs = \$derived\.by<string\[\]>/);
    expect(panel).toMatch(/fbWatchReconcile\(id, dirs\)/);
  });
});
