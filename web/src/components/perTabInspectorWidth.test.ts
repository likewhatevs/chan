import { describe, expect, test } from "vitest";
import fileBrowserSurface from "./FileBrowserSurface.svelte?raw";
import graphPanel from "./GraphPanel.svelte?raw";
import fileEditorTab from "./FileEditorTab.svelte?raw";

// fullstack-84: per-tab inspector width on BrowserTab / GraphTab /
// FileTab. The two-way bind uses Svelte 5's function-pair syntax
// so the getter falls back to `paneWidths.<kind>` while the setter
// writes the tab record. Tabs land with the singleton's current
// value as default; once the user resizes one tab, that tab carries
// its own width and others stay put.

describe("fullstack-84: FileBrowserSurface binds inspector width per-tab", () => {
  test("inspector width getter/setter pair routes through tab state", () => {
    expect(fileBrowserSurface).toContain(
      "() => browserState.inspectorWidth ?? paneWidths.browser",
    );
    expect(fileBrowserSurface).toContain(
      "(v) => (browserState.inspectorWidth = v)",
    );
  });

  test("paneWidths.browser still appears (fallback path) but not as direct bind target", () => {
    expect(fileBrowserSurface).toContain("paneWidths.browser");
    expect(fileBrowserSurface).not.toContain("bind:width={paneWidths.browser}");
  });

  test("tab expansion changes refresh the URL hash for reload restore", () => {
    expect(fileBrowserSurface).toMatch(
      /captured\.expanded = expanded\.length > 0 \? expanded : undefined;[\s\S]*?persistLayoutToHash\(\);/,
    );
  });
});

describe("fullstack-84: GraphPanel binds inspector width per-tab", () => {
  test("graph inspector width getter/setter pair routes through tab state", () => {
    expect(graphPanel).toContain(
      "() => graphState.inspectorWidth ?? paneWidths.graph",
    );
    expect(graphPanel).toContain(
      "(v) => (graphState.inspectorWidth = v)",
    );
  });

  test("paneWidths.graph still appears (fallback path) but not as direct bind target", () => {
    expect(graphPanel).toContain("paneWidths.graph");
    expect(graphPanel).not.toContain("bind:width={paneWidths.graph}");
  });
});

describe("fullstack-84: FileEditorTab binds inspector + outline widths per-tab", () => {
  test("inspector width pair routes through tab state", () => {
    expect(fileEditorTab).toContain(
      "() => tab.inspectorWidth ?? paneWidths.inspector",
    );
    expect(fileEditorTab).toContain(
      "(v) => (tab.inspectorWidth = v)",
    );
  });

  test("outline width pair routes through tab state", () => {
    expect(fileEditorTab).toContain(
      "() => tab.outlineWidth ?? paneWidths.outline",
    );
    expect(fileEditorTab).toContain(
      "(v) => (tab.outlineWidth = v)",
    );
  });

  test("paneWidths singletons not bound directly", () => {
    expect(fileEditorTab).not.toContain("bind:width={paneWidths.inspector}");
    expect(fileEditorTab).not.toContain("bind:width={paneWidths.outline}");
  });
});
