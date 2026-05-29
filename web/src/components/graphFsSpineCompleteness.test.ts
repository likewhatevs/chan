import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";

// Phase-11 GI-9 (next-round-backlog.md): the filesystem graph omitted
// most subdirectories at depth. @@Alex saw scope=agents/ depth-2 plot
// only the link-related branch while the File Browser showed ~10
// siblings; the status bar read "27/47 nodes" so the dropped nodes were
// present in the data but not rendered.
//
// ROOT CAUSE (empirically confirmed on a fresh binary + a seeded /tmp
// workspace, 2026-05-27): `scopedNodeIds` seeds the scope BFS ONLY from
// `kind === "file"` nodes under the directory (the dir-scope branch
// filters `n.kind === "file" && n.path.startsWith(prefix)`). In
// filesystem mode a directory's shallow children are DIRECTORIES
// (mapped to `kind: "folder"`), not files, so the seed set was empty
// and `if (seedIds.size === 0) return seedIds` returned a NON-NULL
// EMPTY set. visibleNodeIds skips every node not in scopedNodeIds and
// visibleEdges drops every edge whose endpoints aren't in it, so a
// pure-directory shallow fs-graph rendered 0/N (a `dir:agents` depth-1
// graph showed 0/7). At depth only the branches that reached a FILE got
// seeded, dropping sibling directories -> the "27/47" symptom.
//
// FIX: filesystem mode returns null (no scope filter), exactly like
// workspace / global already do. The fs-graph endpoint already returns the
// in-scope, depth-limited containment spine (the depth slider re-fetches
// at the new depth via load()), so there is no larger graph to narrow.
// Returning null renders the full backend spine for ALL branches; the
// per-kind chip filters still apply via the hidden*Ids sets. The scope
// BFS stays for the SEMANTIC modes, where /api/graph returns the workspace's
// relevant subgraph and the frontend trims it.
//
// These pins live at the source level because scopedNodeIds is a
// component-internal $derived (not a pure function), mirroring the
// existing graphParentEdgeInvariant / graphDirInspectorHotfix `?raw`
// pins. The behavioural side is verified in-browser per the journal.

describe("GI-9: filesystem-mode graph renders the full containment spine", () => {
  test("scopedNodeIds returns null in filesystem mode (no SPA-side scope filter)", () => {
    // The backend fs-graph is already scoped + depth-limited, so the
    // file-centric BFS must not narrow it (that was the 0/N bug).
    expect(graph).toMatch(/if \(filesystemMode\) return null;/);
  });

  test("the semantic-mode filesystem-depth branch is gated on !filesystemMode (fs-mode never enters it)", () => {
    // The pre-F1 GI-9 fix returned null for both workspace AND
    // filesystem mode, which avoided the file-only seed bug by short-
    // circuiting altogether. The closing-8 (F1) `find -d N` filter
    // narrows workspace + dir scope by node path depth, so it needs to
    // skip fs-mode explicitly - the fs-graph endpoint already returns
    // the depth-limited containment spine and a SPA-side filter on
    // node paths would re-introduce the empty-seed shape for
    // directory-only fs scopes. Both guards must coexist: the new
    // branch reads `!filesystemMode &&` and the standalone
    // `if (filesystemMode) return null;` stays as a defense for any
    // future semantic-mode work that follows.
    expect(graph).toMatch(
      /if \(\s*!filesystemMode &&\s*\(currentScope\.kind === "workspace" \|\| currentScope\.kind === "dir"\)\s*\)/,
    );
    expect(graph).toMatch(/if \(filesystemMode\) return null;/);
    // File-scope seed is the last semantic branch.
    const fsNull = graph.search(/if \(filesystemMode\) return null;/);
    const fileSeed = graph.search(
      /currentScope\.kind === "file" \? \[currentScope\.path\] : \[\]/,
    );
    expect(fileSeed).toBeGreaterThan(fsNull);
  });

  test("round-1 closing-8 (F1): workspace + dir semantic scope use find -d N depth filter", () => {
    // Workspace + dir scope in semantic mode now filter nodes by
    // filesystem depth relative to the scope root (find -d N
    // semantics) instead of returning null / running a hop-based
    // BFS. depth >= depthCap lifts the filter (show full graph);
    // the workspace-root anchor is always kept so the spine has a
    // root to hang off.
    expect(graph).toMatch(
      /currentScope\.kind === "workspace" \|\| currentScope\.kind === "dir"/,
    );
    expect(graph).toMatch(/if \(graphState\.depth >= depthCap\) return null;/);
    expect(graph).toMatch(
      /relativeDepth\(rootPath, nodePath\) <= graphState\.depth/,
    );
    // The pre-F1 dir-scope file-only seed is gone; the only file-seed
    // path remaining is file scope.
    expect(graph).not.toMatch(/n\.path === root \|\| n\.path\.startsWith\(prefix\)/);
  });
});
