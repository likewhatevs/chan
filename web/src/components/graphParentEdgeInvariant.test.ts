import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";

// Graph parent-edge invariant. The forward-only BFS from a seed file
// walks source -> target only, so a `parent -> file` contains edge
// never adds the parent to `scopedNodeIds`. A parent-pull pass
// (`pullContainsSpine`) iterates to a fixed point, adding
// `e.source` for every contains edge whose `e.target` is already in
// scope. The pass is a shared helper so the tag / contact /
// language lenses reuse it too (see graphLensSpine.test.ts).

describe("parent-edge invariant", () => {
  test("the shared pull walks to a fixed point adding `e.source` of contains edges", () => {
    // Iterate to a fixed point adding `e.source` for every
    // contains edge whose `e.target` is already in scope.
    expect(graph).toMatch(
      /function pullContainsSpine\(visited: Set<string>\): void \{[\s\S]*?let pulled = true;[\s\S]*?while \(pulled\) \{[\s\S]*?pulled = false;[\s\S]*?for \(const e of edges\) \{[\s\S]*?e\.kind === "contains"[\s\S]*?visited\.has\(e\.target\)[\s\S]*?!visited\.has\(e\.source\)[\s\S]*?visited\.add\(e\.source\);[\s\S]*?pulled = true;/,
    );
  });

  test("file scope pulls the spine AFTER the forward BFS (positional check)", () => {
    // Sitting after the depth-bounded BFS ensures ancestors pull at
    // any depth. Anchored on the BFS comment then the pull comment.
    const matchBfs = graph.search(/Forward-only BFS: the walk follows edges source -> target/);
    const matchPull = graph.search(
      /Parent-edge invariant: every in-scope file/,
    );
    expect(matchBfs).toBeGreaterThan(-1);
    expect(matchPull).toBeGreaterThan(matchBfs);
  });

  test("the pull gates on `contains` edge kind only (no reverse-walk on link/tag/mention)", () => {
    // Gating on `e.kind === "contains"` prevents a generic reverse-walk
    // from undoing the depth-slider semantics for other edge kinds.
    const pullBlock = graph.match(
      /function pullContainsSpine[\s\S]*?while \(pulled\) \{[\s\S]*?\}\s*\}\s*\}/,
    );
    expect(pullBlock).not.toBeNull();
    expect(pullBlock![0]).toMatch(/e\.kind === "contains"/);
  });

  test("the pull writes to the same visited set (not a separate accumulator)", () => {
    // Writing to `visited` directly lets the next iteration pull the
    // freshly-added ancestor's own parent.
    const pullBlock = graph.match(
      /function pullContainsSpine[\s\S]*?while \(pulled\) \{[\s\S]*?\}\s*\}\s*\}/,
    );
    expect(pullBlock).not.toBeNull();
    expect(pullBlock![0]).toMatch(/visited\.add\(e\.source\)/);
  });

  test("folder-filter hiding still kicks in via hiddenFolderIds (pull doesn't bypass)", () => {
    // The pull adds the parent dir to scope, but folder-filter OFF
    // still hides it via hiddenFolderIds in visibleNodeIds.
    expect(graph).toMatch(/if \(hiddenFolderIds\.has\(n\.id\)\) continue;/);
  });
});
