import { describe, expect, test } from "vitest";
import panel from "./GraphPanel.svelte?raw";

// Phase-11 graph/inspector hotfix (graph-inspector-bugs.md, GI-5/6/7).
//
// GI-1/GI-2 fixed the FILE-node Open / Show File actions. The
// DIRECTORY-node actions (Show Directory, Graph from here) plus the
// depth slider were still broken. These tests lock the directory + depth
// behaviour at the source level, mirroring graphInspectorActionsHotfix's
// `?raw` pins (the handlers live inside the Svelte component and the bugs
// were reactive wiring, not pure functions, so the pin is on the wiring).

describe("GI-5: Show Directory reveals + ENTERS the directory in the File Browser", () => {
  test("revealSelectedFsEntry passes enter:true for directories so the FB opens AT the dir", () => {
    // The no-op was revealAndSelect only expanding the ANCESTOR chain.
    // Directories now revealAndEnterDirectory (enter:true) which expands
    // the directory itself and lazy-loads its children; files stay
    // select-in-place (enter:false).
    expect(panel).toMatch(
      /function revealSelectedFsEntry\(\): void \{[\s\S]*?revealPathInBrowser\(selectedFsNode\.path, \{\s*enter: isFsDirectory\(selectedFsNode\),\s*inspectorOpen: true,\s*\}\)/,
    );
  });

  test("the dir-node inspector still binds onReveal to revealSelectedFsEntry", () => {
    expect(panel).toMatch(/onReveal=\{revealSelectedFsEntry\}/);
  });
});

describe("GI-6: Graph from here on a directory re-roots at the dir itself + keeps it selected", () => {
  test("graphFromHere takes an isDir flag and re-roots dirs to the dir itself, files to the parent", () => {
    // Directory: scope to `dir:<path>` (the dir itself), not its parent.
    // The old parent rule made re-rooting a child folder onto its already-
    // current parent a no-op (scopeId unchanged -> no reload) which left
    // the inspector blank. File: keep the parent-folder rule.
    expect(panel).toMatch(/function graphFromHere\(path: string, isDir: boolean\): void \{/);
    expect(panel).toMatch(/if \(isDir\) \{\s*scopeId = path \? `dir:\$\{path\}` : "drive";/);
    expect(panel).toMatch(
      /\} else \{\s*const slash = path\.lastIndexOf\("\/"\);\s*const parent = slash > 0 \? path\.slice\(0, slash\) : "";\s*scopeId = parent \? `dir:\$\{parent\}` : "drive";/,
    );
  });

  test("graphFromHere pins + selects the node so the inspector stays populated", () => {
    expect(panel).toMatch(
      /graphState\.scopeId = scopeId;\s*graphState\.depth = 1;[\s\S]*?graphState\.pendingSelectId = path;\s*selectedId = path;/,
    );
  });

  test("the fs-node inspector binds onSetAsScope with the directory flag", () => {
    expect(panel).toMatch(/\{@const fsIsDir = isFsDirectory\(selectedFsNode\)\}/);
    expect(panel).toMatch(/onSetAsScope=\{\(\) => graphFromHere\(fsPath, fsIsDir\)\}/);
  });

  test("the semantic inspector passes isDir from the selection kind", () => {
    expect(panel).toMatch(
      /graphFromHere\(\s*inspectorSelection\.path,\s*inspectorSelection\.kind === "directory",\s*\)/,
    );
  });
});

describe("GI-7: depth slider holds its dragged value via a full-depth dir probe", () => {
  test("a dirDepthProbe state mirrors the driveDepthProbe pattern", () => {
    expect(panel).toMatch(/let dirDepthProbe: FsGraphResponse \| null = \$state\(null\);/);
    expect(panel).toMatch(/let dirDepthProbeLoading = \$state\(false\);/);
    expect(panel).toMatch(/let dirDepthProbePath: string \| null = \$state\(null\);/);
  });

  test("loadDirDepthProbe fetches the dir at FS_GRAPH_DEPTH_MAX, guarded by path", () => {
    expect(panel).toMatch(
      /async function loadDirDepthProbe\(path: string\): Promise<void> \{[\s\S]*?depth: FS_GRAPH_DEPTH_MAX,[\s\S]*?if \(dirDepthProbePath === path\) dirDepthProbe = probe;/,
    );
  });

  test("an effect (re)runs the dir probe when the dir scope path changes", () => {
    expect(panel).toMatch(
      /if \(!visible \|\| currentScope\?\.kind !== "dir"\) \{\s*dirDepthProbe = null;\s*dirDepthProbePath = null;\s*return;\s*\}/,
    );
    expect(panel).toMatch(/untrack\(\(\) => void loadDirDepthProbe\(path\)\);/);
  });

  test("depthCap prefers the full-depth dir probe and never caps below the loaded depth", () => {
    // Feeding graphDepthCap the deep probe (not the shallow loaded slice)
    // is the fix: the cap reflects the dir's REACHABLE depth, so dragging
    // the slider to 2/3 is no longer clamped back to 1. The Math.max with
    // graphState.depth keeps the cap from snapping below what's on screen
    // before the probe lands.
    expect(panel).toMatch(/if \(filesystemMode && currentScope\?\.kind === "dir"\) \{/);
    expect(panel).toMatch(
      /fsGraph: dirDepthProbe \?\? \{ nodes: fsNodes, truncated: fsTruncated \},/,
    );
    expect(panel).toMatch(/return Math\.max\(probeCap, graphState\.depth\);/);
  });
});
