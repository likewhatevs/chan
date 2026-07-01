import { describe, expect, test } from "vitest";
import { pullMetaNeighbours, type LensEdge, type LensNode } from "./lensClosure";

// Nested @@mention fixture: three .md files across a 3-level tree, each
// mentioning @@handles and cross-linking. @@Alice is the shared handle;
// @@Bob and @@Carol are each referenced by one document.
const nodes: LensNode[] = [
  { id: "top.md", kind: "file" },
  { id: "l1/mid.md", kind: "file" },
  { id: "l1/l2/deep.md", kind: "file" },
  { id: "@@Alice", kind: "mention" },
  { id: "@@Bob", kind: "mention" },
  { id: "@@Carol", kind: "mention" },
];

const edges: LensEdge[] = [
  // mention edges (file -> @@handle)
  { source: "top.md", target: "@@Alice" },
  { source: "l1/mid.md", target: "@@Alice" },
  { source: "l1/mid.md", target: "@@Bob" },
  { source: "l1/l2/deep.md", target: "@@Alice" },
  { source: "l1/l2/deep.md", target: "@@Carol" },
  // cross links (file -> file)
  { source: "top.md", target: "l1/mid.md" },
  { source: "l1/mid.md", target: "l1/l2/deep.md" },
];

// The mention lens's both-endpoints edge filter (GraphPanel visibleEdges):
// an edge renders only when both endpoints are in the visible set.
function surviving(visited: Set<string>): LensEdge[] {
  return edges.filter((e) => visited.has(e.source) && visited.has(e.target));
}

// Visible set after the @@Alice mention lens's depth-1 bidirectional BFS:
// the handle plus every document that references it. (Mirrors the BFS the
// mention arm runs before the closure.)
function postBfsAlice(): Set<string> {
  return new Set(["@@Alice", "top.md", "l1/mid.md", "l1/l2/deep.md"]);
}

describe("mention lens meta-neighbour closure", () => {
  test("before closure: surfaced docs' other @@mention edges are dropped", () => {
    const visited = postBfsAlice();
    // @@Bob / @@Carol sit one hop past the BFS frontier, so their edges
    // fail the both-endpoints filter and vanish.
    expect(visited.has("@@Bob")).toBe(false);
    expect(visited.has("@@Carol")).toBe(false);
    const survivingMentionEdges = surviving(visited).filter((e) =>
      e.target.startsWith("@@"),
    );
    // Only the three @@Alice edges survive; mid->@@Bob and deep->@@Carol drop.
    expect(survivingMentionEdges).toHaveLength(3);
  });

  test("after closure: every surfaced doc renders its full mention edge set", () => {
    const visited = postBfsAlice();
    pullMetaNeighbours(visited, nodes, edges);
    expect(visited.has("@@Bob")).toBe(true);
    expect(visited.has("@@Carol")).toBe(true);
    // All five mention edges now survive the both-endpoints filter.
    const survivingMentionEdges = surviving(visited).filter((e) =>
      e.target.startsWith("@@"),
    );
    expect(survivingMentionEdges).toHaveLength(5);
  });

  test("closure is bounded: it never pulls in another document", () => {
    // Only top.md surfaced. Its @@Alice edge is already in scope; the
    // closure must not drag in mid.md / deep.md via any shared handle.
    const visited = new Set(["@@Alice", "top.md"]);
    pullMetaNeighbours(visited, nodes, edges);
    const files = [...visited].filter((id) => id.endsWith(".md"));
    expect(files).toEqual(["top.md"]);
  });

  test("also closes tag and language meta-nodes off a surfaced file", () => {
    const withMeta: LensNode[] = [
      { id: "a.md", kind: "file" },
      { id: "#infra", kind: "tag" },
      { id: "language:rust", kind: "language" },
    ];
    const metaEdges: LensEdge[] = [
      { source: "a.md", target: "#infra" },
      { source: "language:rust", target: "a.md" },
    ];
    const visited = new Set(["a.md"]);
    pullMetaNeighbours(visited, withMeta, metaEdges);
    expect(visited.has("#infra")).toBe(true);
    expect(visited.has("language:rust")).toBe(true);
  });
});
