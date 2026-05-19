import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";
import graph from "./GraphPanel.svelte?raw";

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
