import { describe, expect, test } from "vitest";
import panel from "./GraphPanel.svelte?raw";

// Graph directory-node inspector hotfix. Directory-node actions (Show
// Directory, Graph from here) and the depth slider were broken. These
// source-level pins mirror graphInspectorActionsHotfix's ?raw pattern
// because the handlers live inside the Svelte component.

describe("Show Directory reveals + ENTERS the directory in the File Browser", () => {
  test("revealSelectedFsEntry opens a File Browser TAB, expanding the dir itself", () => {
    // revealSelectedFsEntry routes through revealPathInBrowserTab.
    // Directories pass isDir=true so the browser expands the directory
    // itself (upto = parts.length) and opens AT it; files expand ancestors.
    expect(panel).toMatch(
      /function revealSelectedFsEntry\(\): void \{[\s\S]*?revealPathInBrowserTab\(selectedFsNode\.path, isFsDirectory\(selectedFsNode\)\)/,
    );
    expect(panel).toMatch(
      /function revealPathInBrowserTab\(path: string, isDir: boolean\)[\s\S]*?const upto = isDir \? parts\.length : parts\.length - 1;/,
    );
  });

  test("the dir-node inspector still binds onReveal to revealSelectedFsEntry", () => {
    expect(panel).toMatch(/onReveal=\{revealSelectedFsEntry\}/);
  });
});

describe("Graph from here on a directory re-roots at the dir itself + keeps it selected", () => {
  test("graphFromHere takes an isDir flag and re-roots dirs to the dir itself, files to the parent", () => {
    // Directory: scope to `dir:<path>` (not its parent). The old always-
    // parent rule made re-rooting to the current parent a no-op and left
    // the inspector blank. File: parent-folder rule is kept.
    expect(panel).toMatch(/function graphFromHere\(path: string, isDir: boolean\): void \{/);
    expect(panel).toMatch(/if \(isDir\) \{\s*scopeId = path \? `dir:\$\{path\}` : "workspace";/);
    expect(panel).toMatch(
      /\} else \{\s*const slash = path\.lastIndexOf\("\/"\);\s*const parent = slash > 0 \? path\.slice\(0, slash\) : "";\s*scopeId = parent \? `dir:\$\{parent\}` : "workspace";/,
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

describe("depth slider holds its dragged value via a full-depth dir probe", () => {
  test("a dirDepthProbe state tracks the dir at full depth", () => {
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
    // The cap uses the deep probe so dragging to 2/3 is not clamped back
    // to 1. Math.max with graphState.depth prevents snapping below what
    // is on screen before the probe lands.
    expect(panel).toMatch(/if \(filesystemMode && currentScope\?\.kind === "dir"\) \{/);
    expect(panel).toMatch(
      /fsGraph: dirDepthProbe \?\? \{ nodes: fsNodes, truncated: fsTruncated \},/,
    );
    expect(panel).toMatch(/return Math\.max\(probeCap, graphState\.depth\);/);
  });
});
