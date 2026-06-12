import { describe, expect, test } from "vitest";
import source from "./HybridFileBrowserConfig.svelte?raw";
import shell from "./HybridSurfaceConfigShell.svelte?raw";

// The File Browser back-side (Cmd+, on a File Browser) hosts the
// per-workspace directory blocklist editor: GET-then-
// PUT the excluded-dirs set, with a datalist autocomplete and names-only
// validation. Search, the embedding-model picker and chan-reports live
// on the Dashboard's Search + Workspace slot backs - they
// must NOT reappear on this surface.

describe("HybridFileBrowserConfig hosts the blocklist editor", () => {
  test("onDone prop is accepted and the shared shell owns OK", () => {
    expect(source).toMatch(
      /let \{ onDone \}: \{ onDone\?: \(\) => void \} = \$props\(\)/,
    );
    expect(source).toMatch(
      /<HybridSurfaceConfigShell[\s\S]{1,200}title="Hybrid File Browser"[\s\S]{1,200}surface="browser"[\s\S]*?\{onDone\}/,
    );
    expect(shell).toMatch(
      /<button type="button" class="config-ok" onclick=\{\(\) => onDone\?\.\(\)\}>OK<\/button>/,
    );
  });

  test("renders the excluded-directories editor wired to the GET/PUT API", () => {
    expect(source).toMatch(/<h3>Excluded directories<\/h3>/);
    // GET-then-PUT-the-whole-set against the merged blocklist route.
    expect(source).toMatch(/api\.excludedDirs\(\)/);
    expect(source).toMatch(/api\.setExcludedDirs\(/);
    // Autocomplete from the loaded tree's directory basenames.
    expect(source).toMatch(/list="excluded-dir-suggestions"/);
    expect(source).toMatch(/<datalist id="excluded-dir-suggestions">/);
    // Names-only normalization (reject path separators) before save.
    expect(source).toMatch(/function normalizeName/);
    // Save status flows to the shared shell.
    expect(source).toMatch(/\{saveStatus\}/);
  });

  test("none of the moved Dashboard settings live here", () => {
    // These live in the Dashboard Search + Workspace slot backs now.
    expect(source).not.toMatch(/<h3>Semantic search<\/h3>/);
    expect(source).not.toMatch(/<h3>Embedding model<\/h3>/);
    expect(source).not.toMatch(/<h3>chan-reports<\/h3>/);
    expect(source).not.toMatch(/api\.semanticEnable\(\)/);
    expect(source).not.toMatch(/api\.reportsEnable\(\)/);
  });
});
