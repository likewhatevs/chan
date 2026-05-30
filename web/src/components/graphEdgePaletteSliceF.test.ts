import { describe, expect, test } from "vitest";
import canvas from "./GraphCanvas.svelte?raw";
import panel from "./GraphPanel.svelte?raw";

// Graph edge palette:
//   * directory->directory and directory->file containment edges stay
//     GREY (the `contains` kind, stroked in theme.folder),
//   * every OTHER edge matches its document type honouring the Graph
//     settings palette: tag/mention/language keep their own hue, and the
//     `link` (wiki/markdown reference) edge is coloured by the SOURCE
//     document's kind (markdown orange --g-doc, source royalblue
//     --g-source, etc.).
// The canvas paint context isn't unit-testable directly, so these are
// source-pinning assertions on the colour-resolution structure.

describe("contains (dir->dir / dir->file) edges stay grey", () => {
  test("contains maps to theme.folder (grey)", () => {
    expect(canvas).toMatch(/kind === "contains" \? theme\.folder/);
  });

  test("contains rides the single-stroke-per-kind loop, not the link pass", () => {
    expect(canvas).toMatch(
      /\["tag", "mention", "contains", "language", "group", "drafts_link"\] as const/,
    );
  });
});

describe("link edges coloured by source document type", () => {
  test("fileKindColor resolves the palette hue per node kind", () => {
    expect(canvas).toMatch(/function fileKindColor\(kind: DKind\): string/);
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

describe("non-document edges keep their palette hue", () => {
  test("tag green / mention / language retain their own colours", () => {
    expect(canvas).toMatch(/kind === "tag" \? theme\.tag/);
    expect(canvas).toMatch(/kind === "mention" \? theme\.mention/);
    expect(canvas).toMatch(/kind === "language" \? theme\.language/);
  });
});

describe("Graph reuses the FB per-directory pub/sub", () => {
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
