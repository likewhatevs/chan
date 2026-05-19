import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";
import graph from "./GraphPanel.svelte?raw";
import fileBrowserSurface from "./FileBrowserSurface.svelte?raw";

describe("file-browser reveal actions", () => {
  test("terminal Show Dir reveals in a browser tab, not the legacy overlay", () => {
    expect(terminal).toContain("function showTerminalCwd()");
    expect(terminal).toContain("revealPathInBrowser(cwd, { inspectorOpen: true });");
    expect(terminal).not.toContain("browserOverlay.open = true");
  });

  test("graph inspector reveal buttons reveal in a browser tab", () => {
    expect(graph).toContain("function revealSelectedFile()");
    expect(graph).toContain("revealPathInBrowser(selectedNode.path, { inspectorOpen: true });");
    expect(graph).toContain("function revealSelectedFsEntry()");
    expect(graph).toContain("revealPathInBrowser(selectedFsNode.path, { inspectorOpen: true });");
    expect(graph).not.toContain("openBrowser().inspectorOpen");
  });
});

// The Graph and File Browser surfaces are now first-class tabs; closing
// happens via the tab strip's `×`, so neither surface should ship an
// inline close affordance in its own chrome.
describe("no inline close affordance on first-class surfaces", () => {
  test("GraphPanel chrome has no chrome-btn.close button", () => {
    expect(graph).not.toContain('class="chrome-btn close"');
  });

  test("FileBrowserSurface chrome has no chrome-btn.close button", () => {
    expect(fileBrowserSurface).not.toContain('class="chrome-btn close"');
  });
});
