import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";
import store from "../state/store.svelte.ts?raw";
import tabs from "../state/tabs.svelte.ts?raw";

// `fullstack-a-57`: Graph filter chips extended with FileBucket
// toggles (markdown / source) so users can hide markdown to see
// source-code population (or vice versa). Wire data lives in
// chan-report's `FileBucket` from `systacean-16`, but the audit
// found `GraphNodeView::File` doesn't carry the bucket field —
// SPA reuses `-a-51`'s client-side `classifyFile` helper to
// dispatch file nodes into buckets (mirrors the chan-server's
// future emit shape).
//
// Tests pin:
// - GraphFilters shape (markdown + source bits) in BOTH
//   store.svelte.ts and tabs.svelte.ts (duplicate type kept in
//   lockstep).
// - URL-hash encoder: 8-bit format with a default-on guard.
// - Per-tab SerTab encoder: version-2 sentinel for legacy
//   default-on behaviour.
// - FilterKind union extension + FILTER_COLORS swatches.
// - Hidden-id derived sets + visibleNodeIds consumption.
// - Chip iteration sites carry the new kinds.
// - classifyFile dispatches doc / source / binary buckets.

describe("GraphFilters shape (both modules)", () => {
  test("store.svelte.ts GraphFilters has markdown + source bits", () => {
    expect(store).toMatch(
      /export type GraphFilters = \{[\s\S]*?markdown: boolean;[\s\S]*?source: boolean;/,
    );
  });

  test("store.svelte.ts DEFAULT_GRAPH_FILTERS defaults both ON", () => {
    expect(store).toMatch(
      /export const DEFAULT_GRAPH_FILTERS: GraphFilters = \{[\s\S]*?markdown: true,[\s\S]*?source: true,/,
    );
  });

  test("tabs.svelte.ts GraphFilters mirrors the shape (duplicate kept in lockstep)", () => {
    expect(tabs).toMatch(
      /export type GraphFilters = \{[\s\S]*?markdown: boolean;[\s\S]*?source: boolean;/,
    );
  });

  test("tabs.svelte.ts DEFAULT_GRAPH_FILTERS mirrors defaults", () => {
    expect(tabs).toMatch(
      /const DEFAULT_GRAPH_FILTERS: GraphFilters = \{[\s\S]*?markdown: true,[\s\S]*?source: true,/,
    );
  });
});

// The old store.svelte.ts URL-hash filter codec (encodeGraphFilters /
// decodeGraphFilters) was REMOVED by the scope-concept wipe (W5, lane-a A3):
// the `graph=` overlay hash is retired. The live filter codec is the
// layout-`s` graph-tab encoder in tabs.svelte.ts, locked by the block below.

describe("SerTab encoder version sentinel", () => {
  test("encoder prefixes payload with version sentinel '2'", () => {
    expect(tabs).toMatch(
      /function encodeGraphTabFilters\(f: GraphFilters\): string \{[\s\S]*?"2",[\s\S]*?f\.markdown \? "d" : "",[\s\S]*?f\.source \? "s" : "",/,
    );
  });

  test("decoder gates new bits behind the version sentinel", () => {
    expect(tabs).toMatch(
      /const isV2 = src\.startsWith\("2"\);[\s\S]*?markdown: isV2 \? src\.includes\("d"\) : true,[\s\S]*?source: isV2 \? src\.includes\("s"\) : true,/,
    );
  });

  test("decoder default-on string includes the new sentinel + bucket bits", () => {
    expect(tabs).toMatch(/src = s \?\? "2ltmaifds"/);
  });
});

describe("FilterKind + FILTER_COLORS extension", () => {
  test("FilterKind union includes markdown + source", () => {
    expect(graph).toMatch(/\| "markdown"/);
    expect(graph).toMatch(/\| "source"/);
  });

  test("FILTER_COLORS maps markdown -> var(--g-doc) and source -> var(--g-source)", () => {
    expect(graph).toMatch(/markdown: "var\(--g-doc\)"/);
    expect(graph).toMatch(/source: "var\(--g-source\)"/);
  });

  test("classifyFile dispatches doc / source / binary buckets", () => {
    expect(graph).toMatch(
      /function classifyFile\([\s\S]*?\): "doc" \| "img" \| "contact" \| "source" \| "binary"/,
    );
    expect(graph).toMatch(/if \(MARKDOWN_EXT_RE_FA57\.test\(path\)\) return "doc"/);
    expect(graph).toMatch(/if \(SOURCE_EXT_RE_FA57\.test\(path\)\) return "source"/);
    expect(graph).toMatch(/return "binary"/);
  });
});

describe("hidden-id derived sets + visibility", () => {
  test("hiddenMarkdownIds set scoped to doc-class file nodes when chip OFF", () => {
    expect(graph).toMatch(
      /const hiddenMarkdownIds = \$derived\.by\([\s\S]*?if \(show\.markdown\) return ids;[\s\S]*?classifyFile\(n\.path, n\.node_kind\) === "doc"/,
    );
  });

  test("hiddenSourceIds set scoped to source-class file nodes when chip OFF", () => {
    expect(graph).toMatch(
      /const hiddenSourceIds = \$derived\.by\([\s\S]*?if \(show\.source\) return ids;[\s\S]*?classifyFile\(n\.path, n\.node_kind\) === "source"/,
    );
  });

  test("visibleEdges filters edges touching hidden markdown / source nodes", () => {
    expect(graph).toMatch(/!hiddenMarkdownIds\.has\(e\.source\)/);
    expect(graph).toMatch(/!hiddenMarkdownIds\.has\(e\.target\)/);
    expect(graph).toMatch(/!hiddenSourceIds\.has\(e\.source\)/);
    expect(graph).toMatch(/!hiddenSourceIds\.has\(e\.target\)/);
  });

  test("visibleNodeIds skips file nodes hidden by markdown / source chips", () => {
    expect(graph).toMatch(/!hiddenMarkdownIds\.has\(n\.id\)/);
    expect(graph).toMatch(/!hiddenSourceIds\.has\(n\.id\)/);
  });
});

describe("chip iteration sites + counts", () => {
  test("the chip iteration site includes markdown + source", () => {
    // Scope-concept wipe (lane-a A1) removed the overlay-bar
    // `filterChips` snippet, leaving the tab-menu bubble as the SINGLE
    // chip-iteration site; it must still carry markdown + source.
    const matches = graph.match(
      /\["tag", "mention", "language", "img", "folder", "markdown", "source"\] as const/g,
    );
    expect(matches).not.toBeNull();
    expect(matches!.length).toBe(1);
  });

  test("counts dispatch increments markdown + source on file-class match", () => {
    expect(graph).toMatch(/else if \(cls === "doc"\) c\.markdown\+\+/);
    expect(graph).toMatch(/else if \(cls === "source"\) c\.source\+\+/);
  });

  test("counts Record initialiser includes markdown + source slots", () => {
    expect(graph).toMatch(
      /Record<FilterKind, number> = \{[\s\S]*?markdown: 0,[\s\S]*?source: 0,/,
    );
  });
});
