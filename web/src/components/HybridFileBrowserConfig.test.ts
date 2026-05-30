import { describe, expect, test } from "vitest";
import source from "./HybridFileBrowserConfig.svelte?raw";
import shell from "./HybridSurfaceConfigShell.svelte?raw";

// `fullstack-a-48` Task F (option B): Search / Indexing / Reports
// settings UI migrated out of the (since-retired) global Settings
// overlay into HybridFileBrowserConfig. Three toggles ship in v1:
// Semantic search (moved verbatim from `-a-21`), multi-model
// picker placeholder (Round-3 Track 2 future slot), chan-reports
// through the per-workspace reports endpoints.
//
// `phase-13 lane-b` slice 3c: the global Settings overlay was
// retired; the migration-direction assertions that used to pin
// "X is gone from the old overlay" are dropped (the file no
// longer exists).

describe("HybridFileBrowserConfig wiring", () => {
  test("warning copy distinguishes workspace-wide scope from per-FB-pane", () => {
    expect(source).toMatch(
      /These settings apply to ALL file-browser surfaces/,
    );
    expect(source).toMatch(/class="hint warning"/);
  });

  test("Semantic search section ships the same state machine as -a-21", () => {
    expect(source).toMatch(/let semanticState = \$state<SemanticState \| null>/);
    expect(source).toMatch(/async function semanticToggle\(next: boolean\)/);
    expect(source).toMatch(/api\.semanticEnable\(\)/);
    expect(source).toMatch(/api\.semanticDownload\(\)/);
    expect(source).toMatch(/api\.semanticDisable\(\)/);
    expect(source).toMatch(/api\.semanticState\(\)/);
    // Polling cadence preserved verbatim from the retired global Settings overlay.
    expect(source).toMatch(/SEMANTIC_POLL_INTERVAL_MS\s*=\s*3000/);
  });

  test("Semantic search guards the feature flag from BuildInfo", () => {
    expect(source).toMatch(/buildInfo && !buildInfo\.features\.embeddings/);
    expect(source).toContain("--features embed-model");
  });

  test("Semantic search toggle disables during downloading + enabling", () => {
    expect(source).toMatch(
      /disabled=\{semanticDownloading \|\| semanticEnabling\}/,
    );
  });

  test("formatModelSize helper carries over from the retired global Settings overlay", () => {
    expect(source).toMatch(
      /function formatModelSize\(bytes: number \| null \| undefined\)/,
    );
    expect(source).toMatch(/bytes == null \|\| !Number\.isFinite\(bytes\)/);
    expect(source).toMatch(/\.toFixed\(1\)/);
  });

  test("Multi-model picker renders the loaded registry as an enabled workspace-wide picker", () => {
    expect(source).toMatch(/<h3>Embedding model<\/h3>/);
    expect(source).toMatch(/let semanticModels = \$state<SemanticModelRegistry \| null>\(null\)/);
    expect(source).toMatch(/api\.semanticModels\(\)/);
    expect(source).toMatch(/api\.semanticModelPatch\(model\)/);
    expect(source).toMatch(
      /<select[\s\S]{1,160}class="config-select family"[\s\S]{1,200}disabled=\{semanticModels === null \|\| semanticModelBusy \|\| semanticDownloading \|\| semanticEnabling\}[\s\S]{1,200}value=\{semanticModels\?\.current_model \?\? ""\}[\s\S]{1,160}onchange=\{changeSemanticModel\}[\s\S]{1,120}aria-label="Embedding model picker"/,
    );
    expect(source).toMatch(/\{#each semanticModels\.models as model \(model\.id\)\}/);
    expect(source).toMatch(/formatModelMeta\(model\)/);
    expect(source).not.toMatch(/Picker placeholder/);
    expect(source).not.toMatch(/backend ships a model registry/);
  });

  test("chan-reports toggle uses per-workspace reports endpoints", () => {
    expect(source).toMatch(/<h3>chan-reports<\/h3>/);
    expect(source).toMatch(/function setReportsEnabled\(next: boolean\)/);
    expect(source).toMatch(/api\.reportsEnable\(\)/);
    expect(source).toMatch(/api\.reportsDisable\(\)/);
    expect(source).toMatch(/checked=\{reportsEnabled\}/);
  });

  test("chan-reports state loads independently from /api/config", () => {
    expect(source).toMatch(/let reportsState = \$state<\{ enabled: boolean \} \| null>/);
    expect(source).toMatch(/async function loadReportsState\(\)/);
    expect(source).toMatch(/reportsState = await api\.reportsState\(\)/);
    expect(source).not.toMatch(/reportsDirty/);
    expect(source).not.toMatch(/api\.updateConfig\(cfgBody\)/);
  });

  test("polling timer is cleaned up on destroy", () => {
    expect(source).toMatch(/onDestroy\(\(\) => \{\s*stopSemanticPoll\(\)/);
  });

  test("enable flow refreshes model registry after download", () => {
    expect(source).toMatch(/semanticState = await api\.semanticDownload\(\)/);
    expect(source).toMatch(/await refreshSemanticSearchState\(\)/);
    expect(source).toMatch(/semanticState = await api\.semanticEnable\(\)/);
  });

  test("model label metadata includes dimensions, size, and download state", () => {
    expect(source).toMatch(/function formatModelMeta\(model: SemanticModelRegistry\["models"\]\[number\]\): string/);
    expect(source).toMatch(/`\$\{model\.dim\}d`/);
    expect(source).toMatch(/model\.size_label/);
    expect(source).toMatch(/model\.downloaded \? "downloaded" : "not downloaded"/);
  });
});

describe("Wave 4: File Browser back-side controls", () => {
  test("onDone prop is accepted and OK button routes through it", () => {
    expect(source).toMatch(/let \{ onDone \}: \{ onDone\?: \(\) => void \} = \$props\(\)/);
    expect(source).toMatch(
      /<HybridSurfaceConfigShell[\s\S]{1,180}title="Hybrid File Browser"[\s\S]{1,120}surface="browser"[\s\S]*?\{onDone\}/,
    );
    expect(shell).toMatch(
      /<button type="button" class="config-ok" onclick=\{\(\) => onDone\?\.\(\)\}>OK<\/button>/,
    );
  });

  test("model dropdown uses the polished config-select style", () => {
    expect(source).toMatch(/class="config-select family"/);
    expect(source).toMatch(/\.config-select \{[\s\S]{1,300}border: 1px solid var\(--border\)/);
  });
});
