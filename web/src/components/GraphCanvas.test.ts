import { describe, expect, test } from "vitest";
import source from "./GraphCanvas.svelte?raw";

// Filesystem-hierarchy as graph spine. GraphCanvas's d3-force simulation
// places every node above its ancestor chain to the workspace root
// (the root anchors at the bottom; the spine grows up). Load-bearing
// pieces: DNode extended with depth + parentId; nodeHierarchy() derives
// those from kind + path; buildSim() wires a depth-aware forceY +
// parentXForce so files sit above their parent dir and siblings cluster
// horizontally. Tests pin the wiring shape.

describe("filesystem-hierarchy layout shape", () => {
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
    // Non-hierarchical kinds float on the center force rather than
    // anchoring to a depth band.
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
    // File at "docs/foo.md" -> depth 2, parent "directory:docs".
    // File at "README.md" -> depth 1, parent "" (workspace root).
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
    // Hierarchical nodes (depth >= 0) target -depth * hierarchyYSpacing
    // so the spine grows up from the workspace root at the bottom.
    // Non-hierarchical nodes (depth < 0) fall back to centerStrength.
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

describe("dblclick on a node = graph from here", () => {
  test("Props expose an optional onSetAsScope callback", () => {
    expect(source).toMatch(
      /type Props = \{[\s\S]{1,2000}onSetAsScope\?: \(\) => void;/,
    );
    expect(source).toMatch(
      /let \{[\s\S]{1,800}onSetAsScope,\s*\}: Props = \$props\(\);/,
    );
  });

  test("canvas element binds ondblclick to onDoubleClick", () => {
    expect(source).toMatch(/ondblclick=\{onDoubleClick\}/);
  });

  test("onDoubleClick picks at click-slack + invokes onSetAsScope when a node sits under the cursor", () => {
    // Mirrors onMouseUp's tap path: localCoords + pickNode at the wider
    // click slack. Empty-space dblclicks must NOT rescope.
    expect(source).toMatch(
      /function onDoubleClick\(e: MouseEvent\): void \{[\s\S]{1,200}const p = localCoords\(e\);[\s\S]{1,200}const n = pickNode\(p\.x, p\.y, PICK_SLACK_CLICK_PX\);[\s\S]{1,200}if \(n && onSetAsScope\) onSetAsScope\(\);/,
    );
  });
});
