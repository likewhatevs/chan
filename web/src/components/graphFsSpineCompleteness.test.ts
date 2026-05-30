import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";

// Phase-11 GI-9 (next-round-backlog.md): the filesystem graph omitted
// most subdirectories at depth. The root cause was that `scopedNodeIds`
// applied the SEMANTIC scope BFS (which seeds only from `kind === "file"`
// nodes) to filesystem mode, where a directory's shallow children are
// DIRECTORIES, not files. The seed set came up empty and a pure-directory
// fs-graph rendered 0/N. The invariant that prevents regression: in
// filesystem mode `scopedNodeIds` must run its OWN branch and never fall
// through to the file-seed BFS.
//
// The directory-spine visibility is now driven by the per-instance
// expanded set (double-click a directory to reveal its next degree; File
// Browser parity). A node renders only when every ancestor directory up to
// the scope root is expanded, and the depth slider seeds that set to depth
// N - so the default view is the full spine to depth N, collapsible per
// directory. File scope stays a focused single-file view (no tree to
// expand) and keeps the unfiltered spine.
//
// These pins live at the source level because scopedNodeIds is a
// component-internal $derived (not a pure function), mirroring the
// existing graphParentEdgeInvariant / graphDirInspectorHotfix `?raw`
// pins. The behavioural side is verified in-browser per the journal.

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

  test("filesystem mode never reaches the semantic file-seed BFS (the 0/N bug)", () => {
    // The fs-mode branch must precede the file-seed path so a
    // directory-only fs scope can never hit the empty-seed shape.
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

  test("(F1): workspace + dir semantic scope use find -d N depth filter", () => {
    // Workspace + dir scope in SEMANTIC mode filter nodes by filesystem
    // depth relative to the scope root (find -d N semantics). depth >=
    // depthCap lifts the filter; the workspace-root anchor is always kept
    // so the spine has a root to hang off.
    expect(graph).toMatch(
      /currentScope\.kind === "workspace" \|\| currentScope\.kind === "dir"/,
    );
    expect(graph).toMatch(/if \(graphState\.depth >= depthCap\) return null;/);
    expect(graph).toMatch(
      /relativeDepth\(rootPath, nodePath\) <= graphState\.depth/,
    );
    // The pre-F1 dir-scope file-only seed is gone; the only file-seed path
    // remaining is file scope.
    expect(graph).not.toMatch(/n\.path === root \|\| n\.path\.startsWith\(prefix\)/);
  });
});
