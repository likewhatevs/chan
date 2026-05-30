import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";

// Graph parent-edge invariant. The G9 forward-only BFS from a seed file
// walks source -> target only, so a `parent -> file` contains edge
// never adds the parent to `scopedNodeIds`. Fix: a parent-pull pass
// that iterates to a fixed point, adding `e.source` for every contains
// edge whose `e.target` is already in scope.

describe("parent-edge invariant", () => {
  test("scopedNodeIds derive includes a parent-pull pass on contains edges", () => {
    // Iterate to a fixed point adding `e.source` for every
    // contains edge whose `e.target` is already in scope.
    expect(graph).toMatch(
      /let pulled = true;[\s\S]*?while \(pulled\) \{[\s\S]*?pulled = false;[\s\S]*?for \(const e of edges\) \{[\s\S]*?e\.kind === "contains"[\s\S]*?visited\.has\(e\.target\)[\s\S]*?!visited\.has\(e\.source\)[\s\S]*?visited\.add\(e\.source\);[\s\S]*?pulled = true;/,
    );
  });

  test("parent-pull runs AFTER the forward BFS (positional check)", () => {
    // Sitting after the depth-bounded BFS ensures ancestors pull at
    // any depth. Anchored on the comment blocks.
    const matchBfs = graph.search(/`fullstack-a-52` G9: forward-only BFS/);
    const matchPull = graph.search(
      /`fullstack-a-58` parent-edge invariant: pull each in-scope/,
    );
    expect(matchBfs).toBeGreaterThan(-1);
    expect(matchPull).toBeGreaterThan(matchBfs);
  });

  test("parent-pull respects `contains` edge kind only (doesn't pull on link/tag/mention)", () => {
    // Gating on `e.kind === "contains"` prevents a generic reverse-walk
    // from undoing the depth-slider semantics for other edge kinds.
    const pullBlock = graph.match(
      /`fullstack-a-58` parent-edge invariant[\s\S]*?while \(pulled\) \{[\s\S]*?\}\s*\}/,
    );
    expect(pullBlock).not.toBeNull();
    expect(pullBlock![0]).toMatch(/e\.kind === "contains"/);
  });

  test("parent-pull writes to the same visited set (not a separate accumulator)", () => {
    // Writing to `visited` directly lets the next iteration pull the
    // freshly-added ancestor's own parent.
    const pullBlock = graph.match(
      /`fullstack-a-58` parent-edge invariant[\s\S]*?while \(pulled\) \{[\s\S]*?\}\s*\}/,
    );
    expect(pullBlock).not.toBeNull();
    expect(pullBlock![0]).toMatch(/visited\.add\(e\.source\)/);
  });

  test("folder-filter hiding still kicks in via hiddenFolderIds (parent pull doesn't bypass)", () => {
    // Parent-pull adds the parent dir to scope, but folder-filter OFF
    // still hides it via hiddenFolderIds in visibleNodeIds.
    expect(graph).toMatch(/if \(hiddenFolderIds\.has\(n\.id\)\) continue;/);
  });
});
