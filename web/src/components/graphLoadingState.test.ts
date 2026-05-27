import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";

// graph-loading-state-spec slice 1: the graph must signal an in-flight
// index so a not-yet-complete semantic graph (dead-end "missing" nodes
// that may simply be unindexed) is not read as final. Slice 1 wires the
// drive-global `indexStatus` into the graph + shows an "indexing" cue in
// the status bar; the per-scope ghost pull-back + parent-dir pulse is the
// follow-up slice.

describe("graph loading-state slice 1: indexing cue", () => {
  test("GraphPanel imports indexStatus from the store", () => {
    expect(graph).toMatch(/import \{[\s\S]*?\bindexStatus\b[\s\S]*?\} from "\.\.\/state\/store\.svelte"/);
  });

  test("indexBuilding derives from the building / reindexing index states", () => {
    expect(graph).toMatch(
      /const indexBuilding = \$derived\(\s*indexStatus\.value\?\.state === "building" \|\|\s*indexStatus\.value\?\.state === "reindexing",?\s*\)/,
    );
  });

  test("the status bar renders the indexing cue gated on indexBuilding", () => {
    expect(graph).toMatch(/\{#if indexBuilding\}[\s\S]*?class="indexing"[\s\S]*?indexing…/);
  });

  test("the indexing cue pulses (and respects reduced-motion)", () => {
    expect(graph).toMatch(/\.indexing \{[\s\S]*?animation: graph-indexing-pulse/);
    expect(graph).toMatch(/@media \(prefers-reduced-motion: reduce\)[\s\S]*?\.indexing \{[\s\S]*?animation: none/);
  });
});

// graph-loading-state slice 2: while the index is building, dead-end
// ("missing") nodes may just be not-yet-indexed link targets, so they are
// pulled back (with their edges) until the index settles; once idle they
// render as real broken links (the established dashed-ghost styling).
describe("graph loading-state slice 2: pull back dead-ends while indexing", () => {
  test("hiddenMissingIds collects missing file nodes only while indexBuilding", () => {
    expect(graph).toMatch(
      /const hiddenMissingIds = \$derived\.by\(\(\) => \{[\s\S]*?if \(!indexBuilding\) return ids;[\s\S]*?n\.kind === "file" && n\.missing/,
    );
  });

  test("visibleNodeIds excludes pulled-back dead-end nodes", () => {
    expect(graph).toMatch(/!hiddenMissingIds\.has\(n\.id\)/);
  });

  test("visibleEdges drops edges touching pulled-back dead-end nodes", () => {
    expect(graph).toMatch(/!hiddenMissingIds\.has\(e\.source\)/);
    expect(graph).toMatch(/!hiddenMissingIds\.has\(e\.target\)/);
  });
});
