import { describe, expect, test } from "vitest";
import settings from "./SettingsPanel.svelte?raw";

// `fullstack-a-76` slice 2: Settings overlay's Features
// section pairs the per-drive chan-reports + BGE (semantic)
// toggles. Pre-existing `HybridFileBrowserConfig.svelte`
// keeps its richer per-feature controls; Settings is the
// single-screen quick-toggle surface.

describe("fullstack-a-76 slice 2: Features section state", () => {
  test("module imports SemanticState type", () => {
    expect(settings).toMatch(/SemanticState,/);
  });

  test("reports state + busy + error declared", () => {
    expect(settings).toMatch(/let reportsEnabled = \$state<boolean \| null>\(null\);/);
    expect(settings).toMatch(/let reportsBusy = \$state\(false\);/);
    expect(settings).toMatch(/let reportsError = \$state<string \| null>\(null\);/);
  });

  test("semantic state + busy + error declared", () => {
    expect(settings).toMatch(/let semanticState = \$state<SemanticState \| null>\(null\);/);
    expect(settings).toMatch(/let semanticBusy = \$state\(false\);/);
    expect(settings).toMatch(/let semanticError = \$state<string \| null>\(null\);/);
  });

  test("loadFeaturesState calls api.reportsState + api.semanticState", () => {
    expect(settings).toMatch(
      /async function loadFeaturesState\(\): Promise<void> \{[\s\S]*?const r = await api\.reportsState\(\);[\s\S]*?reportsEnabled = r\.enabled;[\s\S]*?semanticState = await api\.semanticState\(\);/,
    );
  });

  test("onMount invokes loadFeaturesState", () => {
    expect(settings).toMatch(
      /onMount\(\(\) => \{[\s\S]*?void loadFeaturesState\(\);[\s\S]*?\}\);/,
    );
  });
});

describe("fullstack-a-76 slice 2: toggle handlers", () => {
  test("toggleReports flips state via the right endpoint per direction", () => {
    expect(settings).toMatch(
      /async function toggleReports\(\): Promise<void> \{[\s\S]*?const target = !reportsEnabled;[\s\S]*?const r = target \? await api\.reportsEnable\(\) : await api\.reportsDisable\(\);[\s\S]*?reportsEnabled = r\.enabled;/,
    );
  });

  test("toggleSemantic guards on model_present before enabling", () => {
    expect(settings).toMatch(
      /async function toggleSemantic\(\): Promise<void> \{[\s\S]*?if \(semanticState\.semantic_enabled\) \{[\s\S]*?semanticState = await api\.semanticDisable\(\);[\s\S]*?\} else if \(semanticState\.model_present\) \{[\s\S]*?semanticState = await api\.semanticEnable\(\);/,
    );
  });

  test("semantic toggle defers model download to FB config when model absent", () => {
    expect(settings).toMatch(
      /BGE model not downloaded yet\. Open the Hybrid File Browser back-side to download\./,
    );
  });
});

describe("fullstack-a-76 slice 2: Features section markup", () => {
  test("`<section class=\"features\">` block present", () => {
    expect(settings).toMatch(/<section class="features">[\s\S]*?<h3>Features<\/h3>/);
  });

  test("reports row carries the right title + sub-description + checkbox handler", () => {
    expect(settings).toMatch(/<div class="feature-title">chan-reports<\/div>/);
    expect(settings).toMatch(
      /onchange=\{toggleReports\}/,
    );
  });

  test("semantic row carries BGE description + checkbox handler", () => {
    expect(settings).toMatch(/<div class="feature-title">BGE semantic search<\/div>/);
    expect(settings).toMatch(
      /onchange=\{toggleSemantic\}/,
    );
  });

  test("semantic row reads 'Model not downloaded' when model_present is false", () => {
    expect(settings).toMatch(/Model not downloaded/);
  });

  test("rationale comment cites pairing + the FB-config fallback", () => {
    expect(settings).toMatch(/`fullstack-a-76`/);
    expect(settings).toMatch(/per-drive feature toggles/i);
    expect(settings).toMatch(/HybridFileBrowserConfig\.svelte/);
    expect(settings).toMatch(/richer[\s\S]{1,40}per-feature controls/i);
  });
});
