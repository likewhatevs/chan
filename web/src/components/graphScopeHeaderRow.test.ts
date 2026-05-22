import { describe, expect, test } from "vitest";
import graph from "./GraphPanel.svelte?raw";

// `fullstack-a-67` slice (Graph surface only): right-click menu
// gets a path-header row showing the current scope's path + a
// kind-appropriate icon. Click wiring → inspector deferred to a
// follow-up slice; this commit lands the display-only row to
// match the @@Alex addendum spec.

describe("fullstack-a-67 (Graph slice): scope-header row", () => {
  test("Lucide icons imported (FileText / Folder / HardDrive / Hash)", () => {
    expect(graph).toMatch(
      /import \{ FileText, Folder, HardDrive, Hash \} from "lucide-svelte";/,
    );
  });

  test("header row renders the scope path + kind-appropriate icon", () => {
    // Drive scope shows "Drive" label; file/dir scopes show their path;
    // tag scope prefixes `#`; etc.
    expect(graph).toMatch(/class="mbtn graph-scope-row"/);
    expect(graph).toMatch(/class="mbtn-label graph-scope-path"/);
  });

  test("icon dispatch covers drive / dir / tag / file at minimum", () => {
    expect(graph).toMatch(/currentScope\.kind === "drive" \|\| currentScope\.kind === "global"[\s\S]*?<HardDrive/);
    expect(graph).toMatch(/currentScope\.kind === "dir"[\s\S]*?<Folder/);
    expect(graph).toMatch(/currentScope\.kind === "tag"[\s\S]*?<Hash/);
    // File / git_repo / group fall through to FileText / Folder.
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

describe("fullstack-a-67 (slice 1b): scope-header click → inspector", () => {
  test("row is a <button> with onclick handler", () => {
    expect(graph).toMatch(
      /<button[\s\S]*?class="mbtn graph-scope-row"[\s\S]*?onclick=\{openScopeHeaderInspector\}/,
    );
  });

  test("openScopeHeaderInspector maps drive → empty-string id (drive-root node)", () => {
    expect(graph).toMatch(
      /openScopeHeaderInspector[\s\S]*?currentScope\.kind === "drive"[\s\S]*?nodeId = "";/,
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

  test("openScopeHeaderInspector maps dir/git_repo → folder node lookup by path", () => {
    expect(graph).toMatch(
      /currentScope\.kind === "dir" \|\| currentScope\.kind === "git_repo" \|\| currentScope\.kind === "group"[\s\S]*?nodes\.find\(\s*\(n\) => n\.kind === "folder" && n\.path === path,?\s*\)/,
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
