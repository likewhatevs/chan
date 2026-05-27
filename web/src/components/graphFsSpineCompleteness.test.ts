import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";

// Phase-11 GI-9 (next-round-backlog.md): the filesystem graph omitted
// most subdirectories at depth. @@Alex saw scope=agents/ depth-2 plot
// only the link-related branch while the File Browser showed ~10
// siblings; the status bar read "27/47 nodes" so the dropped nodes were
// present in the data but not rendered.
//
// ROOT CAUSE (empirically confirmed on a fresh binary + a seeded /tmp
// drive, 2026-05-27): `scopedNodeIds` seeds the scope BFS ONLY from
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
// drive / global already do. The fs-graph endpoint already returns the
// in-scope, depth-limited containment spine (the depth slider re-fetches
// at the new depth via load()), so there is no larger graph to narrow.
// Returning null renders the full backend spine for ALL branches; the
// per-kind chip filters still apply via the hidden*Ids sets. The scope
// BFS stays for the SEMANTIC modes, where /api/graph returns the drive's
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

  test("the fs-mode null guard sits BEFORE the file-only seeding branch", () => {
    // Ordering is load-bearing: the guard must short-circuit before the
    // dir / file branches that seed only from `kind === "file"` nodes
    // (the empty-seed bug). Anchor on the drive null (scopedNodeIds),
    // then the fs-mode null, then the dir-branch file-only seed.
    const driveNull = graph.search(
      /currentScope\.kind === "drive"\) \{\s*return null;/,
    );
    const fsNull = graph.search(/if \(filesystemMode\) return null;/);
    const fileSeed = graph.search(
      /n\.kind === "file" &&\s*\(n\.path === root \|\| n\.path\.startsWith\(prefix\)\)/,
    );
    expect(driveNull).toBeGreaterThan(-1);
    expect(fsNull).toBeGreaterThan(driveNull);
    expect(fileSeed).toBeGreaterThan(fsNull);
  });

  test("the dir-scope seed is still file-only (documents WHY fs-mode must bypass it)", () => {
    // This is the line that produced the empty seed for directory
    // children. If a future change makes the dir branch seed from
    // folder nodes too, revisit whether the fs-mode null guard is still
    // the right shape.
    expect(graph).toMatch(/\.filter\(\s*\(n\) =>\s*n\.kind === "file" &&/);
  });
});
