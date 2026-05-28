import { describe, expect, test } from "vitest";
import panel from "./GraphPanel.svelte?raw";
import canvas from "./GraphCanvas.svelte?raw";

// Phase-11 graph/inspector hotfix (graph-inspector-bugs.md, GI-1..4).
//
// These lock the ACTUAL button BEHAVIOR that regressed in live testing.
// The prior inspector tests passed but did not catch the wiring bugs
// because they asserted the handlers were bound, not that the graph
// would NOT reload as a side effect of invoking them. The root cause
// was reactive, not a mis-bound onclick: the graph reload $effect read
// the `currentScope` $derived (through load()'s synchronous prelude),
// and `currentScope` was recomputed from the pane-derived scope list
// (`availableGraphScopes`, removed in the phase-12 scope-concept wipe;
// currentScope now derives from the tab's own scopeId via
// `synthesizeScope`). So "Open" (opens an editor tab) and "Show File"
// (reveals in the File Browser) both shifted the layout, churned
// `currentScope` to an equal-but-new object, and re-fired the reload.
// The fix anchors the reload effect on a stable value key.

describe("GI-1/GI-2: graph reload is anchored on scope/depth/mode, not layout churn", () => {
  test("a stable loadKey derives from scopeId + depth + mode only", () => {
    // The key the reload effect tracks. It must NOT include the
    // layout-derived `currentScope` object, whose identity changes
    // when an editor tab opens or a File Browser reveal happens.
    expect(panel).toMatch(
      /const loadKey = \$derived\(\s*`\$\{graphState\.scopeId\}\|\$\{graphState\.depth\}\|\$\{graphState\.mode\}`,?\s*\)/,
    );
  });

  test("the reload effect tracks visible + loadKey and runs load() untracked", () => {
    // Reading `visible` and `loadKey` up front registers exactly those
    // two as the effect's dependencies; wrapping load() in untrack()
    // stops load()'s internal reads (currentScope, filters, ...) from
    // becoming reload triggers. Without untrack, the currentScope
    // recompute on a layout change re-fires this effect.
    expect(panel).toMatch(
      /const show = visible;\s*void loadKey;\s*if \(show\) untrack\(\(\) => void load\(\)\);/,
    );
  });

  test("untrack is imported from svelte", () => {
    expect(panel).toMatch(/import \{ onDestroy, untrack \} from "svelte";/);
  });
});

describe("GI-1: Open routes to the editor, not a graph reload", () => {
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
    // No close() after openInActivePane: the graph is a tab now, so
    // close() -> onClose closes the pane's active tab, which by then is
    // the just-opened file tab. Open leaves the graph tab in place
    // (File Browser inspector "Open" parity).
    expect(panel).toMatch(
      /onOpen=\{fsKind === "file"\s*\?\s*\(\) => \{ void openInActivePane\(fsPath\); \}/,
    );
    expect(panel).not.toMatch(/void openInActivePane\(fsPath\); close\(\);/);
  });
});

describe("GI-2: Show File reveals in the File Browser, not a graph reload", () => {
  test("revealSelectedFile reveals + selects the path in a browser tab", () => {
    // GI-8: reveal now routes through the tab-world revealPathInBrowserTab
    // (opens a File Browser tab via openBrowserInActivePane), not the
    // overlay-era revealPathInBrowser + close().
    expect(panel).toMatch(
      /function revealSelectedFile\(\): void \{[\s\S]*?revealPathInBrowserTab\(selectedNode\.path, false\)/,
    );
  });

  test("revealSelectedFsEntry reveals the fs-node path in a browser tab", () => {
    // GI-5 + GI-8: directories pass isDir=true so revealPathInBrowserTab
    // expands the directory ITSELF and the File Browser tab opens AT it;
    // files expand ancestors only. The reveal-not-reload behaviour this
    // test guards is unchanged. Detailed dir/file branch pins live in
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

describe("GI-4: directory nodes are slightly bigger than leaf nodes", () => {
  test("RADIUS_DIR sits between the leaf base and the doc/workspace hub size", () => {
    expect(canvas).toMatch(/const RADIUS_BASE = 5;/);
    expect(canvas).toMatch(/const RADIUS_DIR = 6;/);
    expect(canvas).toMatch(/const RADIUS_DOC = 7;/);
  });

  test("renderRadius gives folder nodes the RADIUS_DIR base", () => {
    expect(canvas).toMatch(
      /kind === "doc" \|\| kind === "workspace"\s*\?\s*RADIUS_DOC\s*:\s*kind === "folder"\s*\?\s*RADIUS_DIR\s*:\s*RADIUS_BASE/,
    );
  });
});
