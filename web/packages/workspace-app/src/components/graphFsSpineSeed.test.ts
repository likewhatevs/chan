import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";

// Semantic-mode graphs seed the always-available filesystem `contains`
// spine BEFORE the index-derived stream, so the directory / file
// skeleton renders immediately (even mid-index) instead of showing the
// "temporarily unavailable while indexing" copy. The stream then layers
// its richer nodes / edges on top of the same maps. The crux is the
// node-id reconciliation: fs directory ids are bare paths, but the
// semantic graph keys directories under `directory:<path>`, so the fs
// spine's directory ids (and the dir endpoints of its edges) are
// normalized to that form so the two sources collapse onto one id.
//
// Source-level pins mirror graphFsSpineCompleteness / graphLoadingState.

describe("graph: fs spine seed under the semantic graph", () => {
  test("a directoryNodeId helper mirrors the server (root stays empty)", () => {
    expect(graph).toMatch(
      /function directoryNodeId\(path: string\): string \{\s*return path === "" \? "" : `directory:\$\{path\}`;\s*\}/,
    );
  });

  test("the fs spine is fetched and seeded BEFORE the graphStream call", () => {
    const seedFetch = graph.search(
      /fs = await api\.fsGraph\(\{\s*scope: fsScope,/,
    );
    const stream = graph.search(/await api\.graphStream\(/);
    expect(seedFetch).toBeGreaterThan(-1);
    expect(stream).toBeGreaterThan(seedFetch);
    // The seed only runs for the tree-bearing scopes; the tag / mention
    // / contact / language lenses have no directory spine.
    expect(graph).toMatch(
      /currentScope\.kind === "file" \|\|\s*currentScope\.kind === "dir" \|\|\s*currentScope\.kind === "workspace"/,
    );
  });

  test("the seed is cursor-paged with abort checks and a frame yield", () => {
    expect(graph).toMatch(/limit: GRAPH_BATCH_NODES,\s*cursor,/);
    expect(graph).toMatch(/if \(seq !== graphLoadSeq\) return;/);
    expect(graph).toMatch(
      /if \(!fs\.done && cursor\) \{\s*await yieldToFrame\(\);/,
    );
    expect(graph).toMatch(/\} while \(!fs\.done && cursor\);/);
  });

  test("fs directory node ids + dir edge endpoints are normalized; root untouched", () => {
    // Only fs DIRECTORY ids get the `directory:` prefix; file / ghost /
    // symlink-leaf endpoints (bare path) are left alone. The dir-id set
    // comes from isFsDirectory.
    expect(graph).toMatch(/if \(isFsDirectory\(n\)\) fsDirIds\.add\(n\.id\);/);
    expect(graph).toMatch(
      /const normalizeId = \(id: string\): string =>\s*fsDirIds\.has\(id\) \? directoryNodeId\(id\) : id;/,
    );
    // Node ids AND both edge endpoints route through normalizeId.
    expect(graph).toMatch(
      /for \(const mapped of mapFsNodes\(fs\)\) \{\s*const id = normalizeId\(mapped\.id\);\s*renderedNodesById\.set\(id, \{ \.\.\.mapped, id \}\);/,
    );
    expect(graph).toMatch(
      /const source = normalizeId\(mapped\.source\);\s*const target = normalizeId\(mapped\.target\);/,
    );
  });

  test("the seeded nodes go into renderedNodesById, NOT fsNodes (inspector stays correct)", () => {
    // fsNodes is held empty in semantic mode so selectedFsNode (gated on
    // filesystemMode) stays null and the inspector reads selectedNode.
    expect(graph).toMatch(/renderedNodesById\.set\(id, \{ \.\.\.mapped, id \}\)/);
    expect(graph).not.toMatch(/fsNodes = mapFsNodes\(fs\)/);
  });

  test("the semantic branch does not blank nodes/edges across the await (no re-load flicker)", () => {
    // The fs spine is seeded then published, so the index-settle re-load
    // keeps the existing graph on screen. The old `nodes = []; edges = [];`
    // blanking that sat just before `const publish` is gone.
    expect(graph).not.toMatch(
      /fsTruncated = false;\s*nodes = \[\];\s*edges = \[\];\s*const publish = \(\): void =>/,
    );
  });
});

// The index-settle re-load must fire even when the graph is non-empty,
// because the fs spine keeps `nodes` populated. The old empty-only guard
// would never re-layer the index-derived edges.
describe("graph: re-layer fires on index-complete despite a seeded spine", () => {
  test("the edge reload drops the nodes.length === 0 guard and gates on semantic mode", () => {
    expect(graph).toMatch(
      /if \(visible && !filesystemMode && !languageMode\) void reloadGraph\(\)/,
    );
    expect(graph).not.toMatch(
      /if \(visible && nodes\.length === 0\) void reloadGraph\(\)/,
    );
  });
});
