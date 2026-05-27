import { describe, expect, test } from "vitest";
import source from "./GraphCanvas.svelte?raw";

// `fullstack-a-49` (G2): filesystem-hierarchy as graph spine.
// Layout transform added to GraphCanvas's d3-force simulation so
// every plotted node sits ABOVE its ancestor-chain to the workspace
// root (GI-10: workspace root anchors the bottom, the spine grows up).
// Three load-bearing pieces:
//
// 1. `DNode` extended with `depth` + `parentId`.
// 2. `nodeHierarchy()` helper derives depth + parentId from
//    kind + path.
// 3. `buildSim()` wires a depth-aware `forceY` + a custom
//    `parentXForce` so files sit ABOVE their parent dir + siblings
//    cluster horizontally under the same parent.
//
// Tests pin the wiring shape so a future refactor can't silently
// drop the hierarchy backbone.

describe("fullstack-a-49: filesystem-hierarchy layout shape", () => {
  test("DNode carries depth + parentId fields", () => {
    expect(source).toMatch(
      /type DNode = \{[\s\S]*?depth: number;\s*\n\s*parentId: string \| null;/,
    );
  });

  test("FORCE config defines hierarchyYSpacing + hierarchyYStrength + parentXStrength", () => {
    expect(source).toMatch(/hierarchyYSpacing: \d+/);
    expect(source).toMatch(/hierarchyYStrength: 0\./);
    expect(source).toMatch(/parentXStrength: 0\./);
  });

  test("nodeHierarchy: tag / mention / language nodes get depth = -1", () => {
    // Non-hierarchical kinds float on the existing center force;
    // they don't anchor to a depth band.
    expect(source).toMatch(
      /if \(n\.kind === "tag" \|\| n\.kind === "mention" \|\| n\.kind === "language"\)/,
    );
    expect(source).toMatch(/return \{ depth: -1, parentId: null \}/);
  });

  test("nodeHierarchy: workspace root (folder id \"\") sits at depth 0, no parent", () => {
    expect(source).toMatch(/if \(n\.id === "" \|\| n\.path === ""\)/);
    expect(source).toMatch(/return \{ depth: 0, parentId: null \}/);
  });

  test("nodeHierarchy: folder depth + parentId derived from path segments", () => {
    expect(source).toMatch(
      /const segs = n\.path\.split\("\/"\)\.filter\(\(s\) => s\.length > 0\)/,
    );
    expect(source).toMatch(/const depth = segs\.length/);
    expect(source).toMatch(
      /const parentId = parentPath === ""[\s\S]*?\? ""[\s\S]*?: `directory:\$\{parentPath\}`/,
    );
  });

  test("nodeHierarchy: file depth = path segment count; parent is parent dir node id", () => {
    // File at "docs/foo.md" → depth 2, parent "directory:docs".
    // File at "README.md" → depth 1, parent "" (workspace root).
    // Pin the same derivation block for file/media kinds (no
    // separate branch needed because the helper falls through to
    // the path-based shape after the folder + non-hierarchical
    // early-returns).
    expect(source).toMatch(/const filePath = n\.path \?\? ""/);
  });

  test("rebuildWorkingSet populates depth + parentId on every DNode", () => {
    // Both the existing-node branch (mutate-in-place) and the
    // fresh-node branch (construct) must propagate depth +
    // parentId so the forces see the current shape.
    expect(source).toMatch(
      /const \{ depth, parentId \} = nodeHierarchy\(n\)/,
    );
    expect(source).toMatch(/existing\.depth = depth/);
    expect(source).toMatch(/existing\.parentId = parentId/);
    expect(source).toMatch(/const fresh: DNode = \{[\s\S]*?depth, parentId/);
  });

  test("buildSim wires depth-aware forceY for hierarchical nodes", () => {
    // The original `forceY<DNode>(0)` is replaced. Hierarchical
    // nodes (depth >= 0) target `-depth * hierarchyYSpacing` (GI-10:
    // negative so the spine grows UP from the workspace root at the
    // bottom); non-hierarchical (depth < 0) fall back to centerStrength.
    expect(source).toMatch(
      /forceY<DNode>\(\(d\) => \{[\s\S]*?return -d\.depth \* FORCE\.hierarchyYSpacing/,
    );
    expect(source).toMatch(
      /\.strength\(\(d\) =>[\s\S]*?d\.depth < 0 \? FORCE\.centerStrength : FORCE\.hierarchyYStrength/,
    );
  });

  test("buildSim registers the parentX force", () => {
    expect(source).toMatch(/\.force\("parentX", parentXForce\(FORCE\.parentXStrength\)\)/);
  });

  test("parentXForce pulls each hierarchical node toward its parent's X", () => {
    expect(source).toMatch(/function parentXForce\(strength: number\)/);
    // Skip non-hierarchical nodes (depth < 0).
    expect(source).toMatch(/if \(node\.depth < 0\) continue/);
    // Skip nodes with no parent.
    expect(source).toMatch(/if \(node\.parentId === null\) continue/);
    // Skip parent that's not in the working set (filtered out).
    expect(source).toMatch(/if \(!parent \|\| parent\.x == null\) continue/);
    // Velocity push proportional to alpha + strength * (parent.x - node.x).
    expect(source).toMatch(
      /node\.vx = \(node\.vx \?\? 0\) \+ dx \* strength \* alpha/,
    );
  });

  test("parentXForce.initialize wires the node array per d3-force convention", () => {
    expect(source).toMatch(/force\.initialize = \(n: DNode\[\]\) => \{[\s\S]*?initialized = n/);
  });
});
