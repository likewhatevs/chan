import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";

// `fullstack-a-52` G9 + G10 (minimum cut):
//
// * G9: BFS converted from bidirectional to forward-only so the
//   depth slider reveals OUTGOING nodes from the root. Previously
//   `frontier.has(e.source) || frontier.has(e.target)` walked
//   both directions, which hid the "expand from the root" mental
//   model @@Alex flagged in the depth-slider bug.
// * G10: `link` filter dropped from the chip set. Link edges
//   always render now — visibility is implicit (edge visible iff
//   both endpoints visible under the node-type filters + depth).
//   The `link` slot on `GraphFilters` (store.svelte.ts) stays for
//   URL-hash back-compat but isn't consumed by the UI.
//
// Node-type-dependent depth semantic + filter toolbar UI
// restructure flagged as follow-up; this commit lands the
// load-bearing fix-the-broken-slider + drop-the-confusing-chip
// piece + leaves the dispatch shape ready for the per-root-type
// reveal logic.

describe("fullstack-a-52 G9: forward-only BFS", () => {
  test("BFS does NOT walk the reverse edge direction", () => {
    // Old shape:
    //   if (frontier.has(e.source) && !visited.has(e.target)) {
    //     ...
    //   } else if (frontier.has(e.target) && !visited.has(e.source)) {
    //     ...
    //   }
    // Forward-only collapse: only the source-direction branch
    // survives at both BFS sites (tag-scope + general-scope).
    // Strip line comments first so the historical-shape comment
    // block that documents the removal doesn't trip the guard.
    const stripped = graph
      .split("\n")
      .filter((line) => !line.trim().startsWith("//"))
      .join("\n");
    expect(stripped).not.toMatch(
      /else if \(frontier\.has\(e\.target\) && !visited\.has\(e\.source\)\)/,
    );
  });

  test("BFS still walks the forward edge direction", () => {
    // Forward source → target traversal is still the load-bearing
    // step. Two BFS sites (tag-scope + general-scope) share the
    // shape.
    const matches = graph.match(
      /if \(frontier\.has\(e\.source\) && !visited\.has\(e\.target\)\)/g,
    );
    expect(matches).not.toBeNull();
    expect(matches!.length).toBeGreaterThanOrEqual(2);
  });

  test("BFS comment documents the forward-only direction", () => {
    expect(graph).toMatch(/forward-only BFS|outgoing edges only|edges emanate from the root/i);
  });
});

describe("fullstack-a-52 G10: link filter dropped", () => {
  test("FilterKind union no longer includes 'link'", () => {
    // `-a-52` dropped link from FilterKind; subsequent tasks
    // (`-a-57`) extend FilterKind with new bucket kinds. Pin the
    // load-bearing absence (link) rather than the exact union
    // shape so growing the kind set doesn't trip this guard.
    expect(graph).toMatch(/type FilterKind =/);
    expect(graph).toMatch(/\| "tag"/);
    expect(graph).toMatch(/\| "mention"/);
    expect(graph).toMatch(/\| "language"/);
    expect(graph).toMatch(/\| "img"/);
    expect(graph).toMatch(/\| "folder"/);
    // No `link` arm in the FilterKind union; tolerate
    // surrounding whitespace + leading separator.
    expect(graph).not.toMatch(/type FilterKind =[\s\S]*?\| "link"/);
    expect(graph).not.toMatch(/type FilterKind = "link"/);
  });

  test("link is short-circuited to always visible in edgeVisibleByChip", () => {
    expect(graph).toMatch(/if \(kind === "link"\) return true/);
  });

  test("chip iteration no longer ships a 'link' entry", () => {
    // The scope-concept wipe (lane-a A1) removed the overlay-bar
    // `filterChips` snippet, so the tab-menu bubble is now the SINGLE
    // chip-iteration site. `-a-57` extended the array with additional
    // bucket kinds; pin the load-bearing absence (link) + the leading-
    // kind shape (starts with tag) so the guard tolerates future-
    // extension additions.
    const matches = graph.match(/\["tag", "mention"[^\]]*\] as const/g);
    expect(matches).not.toBeNull();
    expect(matches!.length).toBe(1);
    expect(graph).not.toMatch(/\["link",\s*"tag"/);
  });

  test("FILTER_COLORS no longer maps link", () => {
    // Pin the literal-mapping block; the `link` key shouldn't
    // appear inside FILTER_COLORS' object literal.
    expect(graph).toMatch(
      /FILTER_COLORS: Record<FilterKind, string> = \{[\s\S]*?tag: EDGE_COLORS\.tag/,
    );
    expect(graph).not.toMatch(
      /FILTER_COLORS: Record<FilterKind, string> = \{[\s\S]{0,40}link:/,
    );
  });

  test("filesystem-mode label dispatch no longer branches on 'link'", () => {
    // The filesystem-mode chip-label dispatch used to map link →
    // "contains". With link dropped from the iteration, that
    // branch is dead. Confirm it's gone from the chip-label
    // ladder.
    expect(graph).not.toMatch(
      /\{kind === "link"\s*\?\s*"contains"\s*:/,
    );
  });
});

describe("round-1 closing-2 (B7b): depth slider works in workspace path-scope", () => {
  test("depthDisabled no longer pins workspace path-scope to disabled", () => {
    // Pre-fix shape was:
    //   `!languageMode && (!currentScope || currentScope.kind === "workspace")`
    // The workspace branch made the default landing graph's slider
    // unmovable. Post-fix drops the workspace guard so the slider
    // tracks `workspaceDepthProbe`-driven `depthCap` the same way
    // the dir scope does.
    expect(graph).toMatch(
      /\{@const depthDisabled = !languageMode && !currentScope\}/,
    );
    expect(graph).not.toMatch(
      /depthDisabled =\s*\n?\s*!languageMode && \(!currentScope \|\| currentScope\.kind === "workspace"\)/,
    );
  });

  test("depthShallow falls through to depthCap <= 1 for workspace scope too", () => {
    expect(graph).toMatch(
      /const depthShallow = \$derived\.by\(\(\) => \{[\s\S]{1,1200}if \(!currentScope\) return false;[\s\S]{1,200}return depthCap <= 1;/,
    );
    expect(graph).not.toMatch(
      /const disabled = !currentScope \|\| currentScope\.kind === "workspace";\s*\n\s*if \(disabled\) return false;/,
    );
  });
});
