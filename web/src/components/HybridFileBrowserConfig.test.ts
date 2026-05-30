import { describe, expect, test } from "vitest";
import source from "./HybridFileBrowserConfig.svelte?raw";
import shell from "./HybridSurfaceConfigShell.svelte?raw";

// After the phase-15 Dashboard redesign the File Browser back-side keeps
// no settings of its own: Search, the embedding-model picker and
// chan-reports moved to the Dashboard's Search + Workspace slot backs
// (covered by dashboardTabAndCarousel.test.ts). This surface is now a
// placeholder so Cmd+, on a File Browser still lands on a config shell.

describe("HybridFileBrowserConfig is a placeholder", () => {
  test("onDone prop is accepted and the shared shell owns OK", () => {
    expect(source).toMatch(
      /let \{ onDone \}: \{ onDone\?: \(\) => void \} = \$props\(\)/,
    );
    expect(source).toMatch(
      /<HybridSurfaceConfigShell[\s\S]{1,180}title="Hybrid File Browser"[\s\S]{1,160}surface="browser"[\s\S]*?\{onDone\}/,
    );
    expect(shell).toMatch(
      /<button type="button" class="config-ok" onclick=\{\(\) => onDone\?\.\(\)\}>OK<\/button>/,
    );
  });

  test("carries the placeholder copy and none of the moved settings", () => {
    expect(source).toMatch(/No settings here, cheers\./);
    // The moved controls must be gone from this surface (they live in the
    // Dashboard Search + Workspace slot backs now).
    expect(source).not.toMatch(/<h3>Semantic search<\/h3>/);
    expect(source).not.toMatch(/<h3>Embedding model<\/h3>/);
    expect(source).not.toMatch(/<h3>chan-reports<\/h3>/);
    expect(source).not.toMatch(/api\.semanticEnable\(\)/);
    expect(source).not.toMatch(/api\.reportsEnable\(\)/);
  });
});
