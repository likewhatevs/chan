import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";

// Graph right-click menu scope-header row: shows the current scope's
// path + a kind-appropriate icon. Click wiring opens the inspector.

describe("Graph menu scope-header row", () => {
  test("Lucide icons imported (FileText / Folder / HardDrive / Hash)", () => {
    expect(graph).toMatch(
      /import \{[\s\S]*?\bFileText,[\s\S]*?\bFolder,[\s\S]*?\bHardDrive,[\s\S]*?\bHash,[\s\S]*?\} from "lucide-svelte";/,
    );
  });

  test("header row renders the scope path + kind-appropriate icon", () => {
    // Workspace scope shows "Workspace" label; file/dir scopes show their path;
    // tag scope prefixes `#`; etc.
    expect(graph).toMatch(/class="mbtn graph-scope-row"/);
    expect(graph).toMatch(/class="mbtn-label graph-scope-path"/);
  });

  test("icon dispatch covers workspace / dir / tag / file at minimum", () => {
    expect(graph).toMatch(/currentScope\.kind === "workspace"[\s\S]*?<HardDrive/);
    expect(graph).toMatch(/currentScope\.kind === "dir"[\s\S]*?<Folder/);
    expect(graph).toMatch(/currentScope\.kind === "tag"[\s\S]*?<Hash/);
    // The remaining live scope kind (file) falls through to FileText.
    expect(graph).toMatch(/<FileText/);
  });

  test("path label fades at right edge for long paths (mask-image)", () => {
    expect(graph).toMatch(
      /\.graph-scope-row \.graph-scope-path \{[\s\S]*?mask-image: linear-gradient\(to right, black calc\(100% - 1\.25rem\), transparent\);/,
    );
  });

  test("separator follows the scope row before the depth slider", () => {
    expect(graph).toMatch(
      /class="mbtn graph-scope-row"[\s\S]*?<div class="msep" role="separator"><\/div>\s*\{\/if\}\s*<div\s*class="mbtn depth-row"/,
    );
  });
});

describe("scope-header click → inspector", () => {
  test("row is a <button> with onclick handler", () => {
    expect(graph).toMatch(
      /<button[\s\S]*?class="mbtn graph-scope-row"[\s\S]*?onclick=\{openScopeHeaderInspector\}/,
    );
  });

  test("openScopeHeaderInspector maps workspace → empty-string id (workspace-root node)", () => {
    expect(graph).toMatch(
      /openScopeHeaderInspector[\s\S]*?currentScope\.kind === "workspace"[\s\S]*?nodeId = "";/,
    );
  });

  test("openScopeHeaderInspector maps tag → currentScope.nodeId", () => {
    expect(graph).toMatch(
      /currentScope\.kind === "tag"[\s\S]*?nodeId = currentScope\.nodeId;/,
    );
  });

  test("openScopeHeaderInspector maps file → node lookup by path", () => {
    expect(graph).toMatch(
      /currentScope\.kind === "file"[\s\S]*?nodes\.find\(\s*\(n\) => n\.kind === "file" && n\.path === currentScope\.path,?\s*\)/,
    );
  });

  test("openScopeHeaderInspector maps dir → folder node lookup by path", () => {
    expect(graph).toMatch(
      /currentScope\.kind === "dir"[\s\S]*?nodes\.find\(\s*\(n\) => n\.kind === "folder" && n\.path === currentScope\.path,?\s*\)/,
    );
  });

  test("openScopeHeaderInspector sets graphState.inspectorOpen = true + closes tab menu", () => {
    expect(graph).toMatch(
      /selectedId = nodeId;[\s\S]*?graphState\.inspectorOpen = true;[\s\S]*?closeTabMenu\(\);/,
    );
  });

  test("CSS hover state lifts the path label color to var(--text)", () => {
    expect(graph).toMatch(
      /\.graph-scope-row:hover \.graph-scope-path \{[\s\S]*?color: var\(--text\);/,
    );
  });
});
