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
