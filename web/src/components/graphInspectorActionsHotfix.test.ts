import { describe, expect, test } from "vitest";
import panel from "./GraphPanel.svelte?raw";
import canvas from "./GraphCanvas.svelte?raw";

// Graph inspector action hotfix. The reload $effect read `currentScope`
// (a $derived), which was recomputed from the pane-derived scope list and
// returned an equal-but-new object whenever a layout change occurred.
// "Open" and "Show File" both triggered that churn and re-fired the
// reload. The fix anchors the reload effect on a stable loadKey value.

describe("graph reload is anchored on scope/depth/mode, not layout churn", () => {
  test("a stable loadKey derives from scopeId + depth + mode only", () => {
    // The loadKey must not include the layout-derived currentScope object,
    // whose identity changes when an editor tab opens or a reveal happens.
    expect(panel).toMatch(
      /const loadKey = \$derived\(\s*`\$\{graphState\.scopeId\}\|\$\{graphState\.depth\}\|\$\{graphState\.mode\}`,?\s*\)/,
    );
  });

  test("the reload effect tracks visible + loadKey and runs load() untracked", () => {
    // Reading visible + loadKey up front registers exactly those two as
    // dependencies. untrack() around load() prevents load()'s internal
    // reads (currentScope, filters...) from becoming reload triggers.
    // Round 2 (keep-alive) added lazy-first + dirty gating between the
    // trigger reads and the load() call, but the contract is unchanged:
    // those two are the only deps, and load() still runs untracked.
    expect(panel).toMatch(/const show = visible;\s*const key = loadKey;/);
    expect(panel).toMatch(/untrack\(\(\) => void load\(\)\);/);
  });

  test("untrack is imported from svelte", () => {
    expect(panel).toMatch(/import \{ onDestroy, untrack \} from "svelte";/);
  });
});

describe("Open routes to the editor, not a graph reload", () => {
  test("openSelectedFile opens the selected file in the active pane", () => {
    expect(panel).toMatch(
      /function openSelectedFile\(\): void \{[\s\S]*?openInActivePane\(selectedNode\.path\)/,
    );
  });

  test("the semantic-node inspector binds onOpen to openSelectedFile for file selections", () => {
    expect(panel).toMatch(
      /onOpen=\{[\s\S]*?inspectorSelection\?\.kind === "file"\s*\?\s*openSelectedFile/,
    );
  });

  test("the fs-mode inspector binds onOpen to openInActivePane for file nodes", () => {
    // No close() after openInActivePane: the graph is a tab and close()
    // would close the pane's new active tab instead of the graph.
    expect(panel).toMatch(
      /onOpen=\{fsKind === "file"\s*\?\s*\(\) => \{ void openInActivePane\(fsPath\); \}/,
    );
    expect(panel).not.toMatch(/void openInActivePane\(fsPath\); close\(\);/);
  });
});

describe("Show File reveals in the File Browser, not a graph reload", () => {
  test("revealSelectedFile reveals + selects the path in a browser tab", () => {
    // Reveal routes through revealPathInBrowserTab (openBrowserInActivePane),
    // not the overlay-era revealPathInBrowser + close().
    expect(panel).toMatch(
      /function revealSelectedFile\(\): void \{[\s\S]*?revealPathInBrowserTab\(selectedNode\.path, false\)/,
    );
  });

  test("revealSelectedFsEntry reveals the fs-node path in a browser tab", () => {
    // Directories pass isDir=true so the browser expands the dir itself;
    // files expand ancestors. Detailed branch pins are in
    // graphDirInspectorHotfix.test.ts.
    expect(panel).toMatch(
      /function revealSelectedFsEntry\(\): void \{[\s\S]*?revealPathInBrowserTab\(selectedFsNode\.path, isFsDirectory\(selectedFsNode\)\)/,
    );
  });

  test("inspector binds onReveal to the reveal helpers (not a reload)", () => {
    expect(panel).toMatch(/onReveal=\{revealSelectedFile\}/);
    expect(panel).toMatch(/onReveal=\{revealSelectedFsEntry\}/);
  });
});

describe("directory nodes are slightly bigger than leaf nodes", () => {
  test("RADIUS_DIR sits between the leaf base and the doc/workspace hub size", () => {
    expect(canvas).toMatch(/const RADIUS_BASE = 5;/);
    expect(canvas).toMatch(/const RADIUS_DIR = 6;/);
    expect(canvas).toMatch(/const RADIUS_DOC = 7;/);
  });

  test("renderRadius gives folder nodes the RADIUS_DIR base and workspace its own 1.5x size", () => {
    // Workspace root is sized 1.5x the worst-case hub-scaled dir
    // (RADIUS_DIR * RADIUS_HUB_SCALE * 1.5) so the gap holds even
    // when top-level dirs are hub-scaled. Workspace skips the backlink
    // ramp so its size is exactly RADIUS_WORKSPACE.
    expect(canvas).toMatch(
      /const RADIUS_WORKSPACE = RADIUS_DIR \* RADIUS_HUB_SCALE \* 1\.5;/,
    );
    expect(canvas).toMatch(
      /kind === "workspace"\s*\?\s*RADIUS_WORKSPACE\s*:\s*kind === "doc"\s*\?\s*RADIUS_DOC\s*:\s*kind === "folder"\s*\?\s*RADIUS_DIR\s*:\s*RADIUS_BASE/,
    );
    expect(canvas).toMatch(
      /if \(kind === "workspace"\) return base;[\s\S]{1,400}if \(maxBacklinks <= 0\) return base;/,
    );
  });
});
