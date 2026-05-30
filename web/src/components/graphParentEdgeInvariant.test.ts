import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";

// `fullstack-a-58` — graph parent-edge invariant audit-then-fix.
//
// Audit verdict: HYBRID
//
// * File-scope: SPA regression introduced by `fullstack-a-52`'s
//   G9 forward-only BFS. The contains edge `parent → file` points
//   into the seed file; forward-only BFS from the file walks
//   source → target only, so the parent is never added to
//   `scopedNodeIds`. Fix: parent-pull pass that walks REVERSE on
//   contains edges, adding `source` for every contains edge
//   whose `target` is already in scope. Iterated to a fixed
//   point so ancestor chains pull cleanly.
// * Workspace-scope orphan markdown: chan-server emit appears
//   structurally complete from source reading (`walk_directory`
//   emits contains edges recursively); workspace-scope sets
//   `scopedNodeIds = null` so the SPA doesn't filter. The bug
//   may be transitively fixed by the file-scope fix OR may be a
//   separate chan-server emit gap. Flagged for empirical
//   confirmation as a follow-up (couldn't run a test server
//   against the seed at task time).

describe("parent-edge invariant", () => {
  test("scopedNodeIds derive includes a parent-pull pass on contains edges", () => {
    // Iterate to a fixed point adding `e.source` for every
    // contains edge whose `e.target` is already in scope.
    expect(graph).toMatch(
      /let pulled = true;[\s\S]*?while \(pulled\) \{[\s\S]*?pulled = false;[\s\S]*?for \(const e of edges\) \{[\s\S]*?e\.kind === "contains"[\s\S]*?visited\.has\(e\.target\)[\s\S]*?!visited\.has\(e\.source\)[\s\S]*?visited\.add\(e\.source\);[\s\S]*?pulled = true;/,
    );
  });

  test("parent-pull runs AFTER the forward BFS (positional check)", () => {
    // The pass should sit after the depth-bounded forward BFS so
    // ancestors get pulled regardless of depth. Anchor on the
    // BFS comment block + the parent-pull comment block to
    // confirm the ordering.
    const matchBfs = graph.search(/`fullstack-a-52` G9: forward-only BFS/);
    const matchPull = graph.search(
      /`fullstack-a-58` parent-edge invariant: pull each in-scope/,
    );
    expect(matchBfs).toBeGreaterThan(-1);
    expect(matchPull).toBeGreaterThan(matchBfs);
  });

  test("parent-pull respects `contains` edge kind only (doesn't pull on link/tag/mention)", () => {
    // The pass must gate on `e.kind === "contains"` — other edge
    // kinds (link, tag, mention) carry different semantics + a
    // generic reverse-walk would undo `-a-52`'s forward-only
    // depth slider intent.
    const pullBlock = graph.match(
      /`fullstack-a-58` parent-edge invariant[\s\S]*?while \(pulled\) \{[\s\S]*?\}\s*\}/,
    );
    expect(pullBlock).not.toBeNull();
    expect(pullBlock![0]).toMatch(/e\.kind === "contains"/);
  });

  test("parent-pull writes to the same visited set (not a separate accumulator)", () => {
    // Add to `visited` directly so the next iteration sees the
    // freshly-pulled ancestor + can pull its own parent.
    const pullBlock = graph.match(
      /`fullstack-a-58` parent-edge invariant[\s\S]*?while \(pulled\) \{[\s\S]*?\}\s*\}/,
    );
    expect(pullBlock).not.toBeNull();
    expect(pullBlock![0]).toMatch(/visited\.add\(e\.source\)/);
  });

  test("folder-filter hiding still kicks in via hiddenFolderIds (parent pull doesn't bypass)", () => {
    // The parent-pull adds the parent dir to scope, but folder-
    // filter OFF still hides it via `hiddenFolderIds` in
    // `visibleNodeIds` — so the spec acceptance criterion 3
    // (folder filter OFF -> parent dirs don't render) stays
    // satisfied without parent-pull bypassing the folder gate.
    expect(graph).toMatch(/if \(hiddenFolderIds\.has\(n\.id\)\) continue;/);
  });
});
