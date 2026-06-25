import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";

// Filesystem graph spine visibility. The bug: `scopedNodeIds` ran the
// semantic BFS (seeds from `kind === "file"`) in filesystem mode, where
// a directory-only scope has no file seeds, so the graph rendered 0/N.
// Fix: filesystem mode gets its own branch that never falls through to the
// file-seed BFS.
//
// Spine visibility is driven by the per-instance expanded set
// (double-click a dir to reveal the next degree; File Browser parity).
// A node renders only when every ancestor up to the scope root is
// expanded. The depth slider seeds that set, so the default view is the
// full spine to depth N. File scope keeps the unfiltered spine.
//
// Source-level pins mirror graphParentEdgeInvariant / graphDirInspectorHotfix.

describe("filesystem-mode graph spine visibility", () => {
  test("filesystem mode filters by the expanded set, not the file-seed BFS", () => {
    // Its own branch: directory / workspace scope render via
    // `ancestorsExpanded`; file scope keeps the unfiltered spine.
    expect(graph).toMatch(
      /if \(filesystemMode\) \{\s*if \(currentScope\.kind === "file"\) return null;/,
    );
    expect(graph).toMatch(
      /ancestorsExpanded\(fsRoot, n\.path, expanded\)/,
    );
  });

  test("filesystem mode never reaches the semantic file-seed BFS", () => {
    // The fs-mode branch must precede the file-seed path so a
    // directory-only fs scope never hits the empty-seed shape.
    const fsBranch = graph.search(/if \(filesystemMode\) \{/);
    const fileSeed = graph.search(
      /currentScope\.kind === "file" \? \[currentScope\.path\] : \[\]/,
    );
    expect(fsBranch).toBeGreaterThan(-1);
    expect(fileSeed).toBeGreaterThan(fsBranch);
    // The semantic depth branch stays gated on !filesystemMode so fs-mode
    // never enters it.
    expect(graph).toMatch(
      /if \(\s*!filesystemMode &&\s*\(currentScope\.kind === "workspace" \|\| currentScope\.kind === "dir"\)\s*\)/,
    );
  });

  test("workspace + dir semantic scope use the expanded-ancestor tree model", () => {
    // The semantic branch uses the same expanded-ancestor tree model the
    // filesystem mode uses (not a flat `relativeDepth(root, path) <= depth`
    // filter), so directory nodes expand / collapse in the
    // rich semantic graph. A file / folder renders only when every
    // ancestor up to the scope root is expanded; the workspace-root
    // anchor is always kept; tag / mention / language meta-nodes always
    // pass through.
    expect(graph).toMatch(
      /currentScope\.kind === "workspace" \|\| currentScope\.kind === "dir"/,
    );
    // The semantic branch now gates file / folder visibility on
    // ancestorsExpanded(rootPath, ...), not a relativeDepth comparison.
    expect(graph).toMatch(
      /ancestorsExpanded\(rootPath, n\.path, expanded\)/,
    );
    // The flat depth filter and its lift-the-filter short-circuit are
    // gone from the semantic branch.
    expect(graph).not.toMatch(
      /relativeDepth\(rootPath, nodePath\) <= graphState\.depth/,
    );
    expect(graph).not.toMatch(/if \(graphState\.depth >= depthCap\) return null;/);
    // The only remaining file-seed path is file scope.
    expect(graph).not.toMatch(/n\.path === root \|\| n\.path\.startsWith\(prefix\)/);
  });
});
