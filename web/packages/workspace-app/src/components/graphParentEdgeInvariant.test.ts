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

  test("folder-filter hiding still gates visibleNodeIds via hiddenFolderIds", () => {
    // The hiddenFolderIds gate stays in visibleNodeIds; what changed is its
    // CONTENTS -- it now excludes spine directories (spineFolderIds), so folder
    // OFF hides only non-spine directory clutter, not the file→parent spine.
    expect(graph).toMatch(/if \(hiddenFolderIds\.has\(n\.id\)\) continue;/);
  });

  test("contains edges are NOT gated by the folder chip (spine survives folder OFF)", () => {
    // The file→parent contains spine renders whenever both endpoints are
    // visible, regardless of show.folder; gating it on the folder chip dropped
    // the whole spine when folder was off and rendered files loose (the bug).
    expect(graph).toMatch(/if \(kind === "contains"\) return true;/);
    const fn = graph.match(/function edgeVisibleByChip[\s\S]*?\n {2}\}/);
    expect(fn).not.toBeNull();
    expect(fn![0]).not.toMatch(/kind === "contains"\) return show\.folder/);
  });

  test("the folder chip hides only NON-spine directory clutter", () => {
    // spineFolderIds = directories anchoring an in-scope file (walked up the
    // contains edges); hiddenFolderIds (folder OFF) excludes them, so spine
    // directories stay visible and files keep their containment anchor.
    expect(graph).toMatch(
      /const spineFolderIds = \$derived\.by\(\(\) => \{[\s\S]*?e\.kind === "contains"[\s\S]*?onSpine\.has\(e\.target\)[\s\S]*?onSpine\.add\(e\.source\)/,
    );
    expect(graph).toMatch(/n\.kind === "folder" && !spineFolderIds\.has\(n\.id\)/);
  });
});
