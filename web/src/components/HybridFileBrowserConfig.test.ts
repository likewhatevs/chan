import { describe, expect, test } from "vitest";
import source from "./HybridFileBrowserConfig.svelte?raw";
import panel from "./SettingsPanel.svelte?raw";

// `fullstack-a-48` Task F (option B): Search / Indexing / Reports
// settings UI migrated out of SettingsPanel into
// HybridFileBrowserConfig. Three toggles ship in v1: Semantic
// search (moved verbatim from `-a-21`), multi-model picker
// placeholder (Round-3 Track 2 future slot), chan-reports
// (G1 regression-fix toggle wired to a new Preferences.reports
// shape — backend gating + default-flip-to-OFF + destructive-on-
// disable modal are a follow-up task per option (B) routing).

describe("fullstack-a-48: HybridFileBrowserConfig wiring", () => {
  test("warning copy distinguishes drive-wide scope from per-FB-pane", () => {
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
    // Polling cadence preserved verbatim from SettingsPanel.
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

  test("formatModelSize helper carries over from SettingsPanel", () => {
    expect(source).toMatch(/function formatModelSize\(bytes: number \| null\)/);
    expect(source).toMatch(/\.toFixed\(1\)/);
  });

  test("Multi-model picker renders as a disabled placeholder slot", () => {
    expect(source).toMatch(/<h3>Embedding model<\/h3>/);
    expect(source).toMatch(
      /<select[\s\S]{1,160}class="config-select family"[\s\S]{1,120}disabled[\s\S]{1,120}aria-label="Embedding model picker \(placeholder\)"/,
    );
    expect(source).toContain("BAAI/bge-small-en-v1.5");
  });

  test("chan-reports toggle writes through editing.reports.enabled", () => {
    expect(source).toMatch(/<h3>chan-reports<\/h3>/);
    expect(source).toMatch(/function setReportsEnabled\(next: boolean\)/);
    expect(source).toMatch(/editing\.reports = \{ enabled: next \}/);
    expect(source).toMatch(/checked=\{reportsEnabled\}/);
  });

  test("normalizeReports backfills default ON for pre-a-48 servers", () => {
    // Pre-`-a-48` servers don't ship the `reports` field. Backfill
    // with `{ enabled: true }` so dirty() doesn't trigger an
    // immediate spurious PATCH after the post-save re-clone.
    expect(source).toMatch(/if \(!p\.reports\) p\.reports = \{ enabled: true \}/);
  });

  test("save merges only reports field onto the server's current GlobalConfig", () => {
    // Race safety: parallel SettingsPanel autosave (residual fields
    // after `-a-46` trim) can't be clobbered by a HybridFileBrowser
    // autosave, and vice versa.
    expect(source).toMatch(/const current = await api\.config\(\)/);
    expect(source).toMatch(
      /preferences:\s*\{\s*\.\.\.current\.preferences,\s*reports: editing\.reports/,
    );
    expect(source).toMatch(/await api\.updateConfig\(cfgBody\)/);
  });

  test("dirty check is scoped to the reports.enabled field", () => {
    expect(source).toMatch(/function reportsDirty\(\): boolean/);
    expect(source).toMatch(
      /\(editing\.reports\?\.enabled \?\? true\) !== server\.enabled/,
    );
  });

  test("polling timer is cleaned up on destroy", () => {
    expect(source).toMatch(/onDestroy\(\(\) => \{\s*stopSemanticPoll\(\)/);
  });
});

describe("Wave 4: File Browser back-side controls", () => {
  test("onDone prop is accepted and OK button routes through it", () => {
    expect(source).toMatch(/let \{ onDone \}: \{ onDone\?: \(\) => void \} = \$props\(\)/);
    expect(source).toMatch(
      /<button type="button" class="config-ok" onclick=\{\(\) => onDone\?\.\(\)\}>OK<\/button>/,
    );
  });

  test("placeholder dropdown uses the polished config-select style", () => {
    expect(source).toMatch(/class="config-select family"/);
    expect(source).toMatch(/\.config-select \{[\s\S]{1,300}border: 1px solid var\(--border\)/);
  });
});

describe("fullstack-a-48: rich Semantic search state machine removed from SettingsPanel", () => {
  test("semantic search section stays gone from SettingsPanel", () => {
    expect(panel).not.toMatch(/<h3>Semantic search<\/h3>/);
    expect(panel).not.toMatch(/BGE semantic search/);
  });

  test("rich model-download state machine helpers stay gone", () => {
    // These helpers belong to HybridFileBrowserConfig, not the
    // global Settings overlay.
    expect(panel).not.toMatch(/async function toggleSemantic\b/);
    expect(panel).not.toMatch(/async function semanticToggle\b/);
    expect(panel).not.toMatch(/async function loadSemanticState/);
    expect(panel).not.toMatch(/function formatModelSize/);
    expect(panel).not.toMatch(/api\.semantic(State|Enable|Disable)\(\)/);
    expect(panel).not.toMatch(/api\.semanticDownload\(\)/);
  });

  test("download-state variables stay gone", () => {
    expect(panel).not.toMatch(/let\s+semanticState\s*=/);
    expect(panel).not.toMatch(/let\s+semanticBusy\s*=/);
    expect(panel).not.toMatch(/let\s+semanticError\s*=/);
    expect(panel).not.toMatch(/let\s+semanticDownloading\s*=/);
    expect(panel).not.toMatch(/let\s+semanticEnabling\s*=/);
    expect(panel).not.toMatch(/let\s+semanticPollTimer\s*=/);
  });
});

describe("Wave 2: reports controls removed from SettingsPanel", () => {
  test("chan-reports controls stay owned by HybridFileBrowserConfig", () => {
    expect(source).toMatch(/<h3>chan-reports<\/h3>/);
    expect(panel).not.toContain("chan-reports");
    expect(panel).not.toMatch(/api\.reports(State|Enable|Disable)\(\)/);
    expect(panel).not.toMatch(/toggleReports/);
    expect(panel).not.toMatch(/reportsEnabled|reportsBusy|reportsError/);
  });
});
