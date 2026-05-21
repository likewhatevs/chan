import { describe, expect, test } from "vitest";
import source from "./HybridEditorConfig.svelte?raw";
import panel from "./SettingsPanel.svelte?raw";

// `fullstack-a-46` Task C: Editor settings UI migrated out of
// SettingsPanel into HybridEditorConfig. Five sections move:
// Editor theme, Appearance, Layout (line spacing), Date pills
// (date format), On save (strip trailing whitespace). Settings
// storage shape unchanged; both surfaces still PATCH the same
// `GlobalConfig.preferences`. The dirty comparator + save are
// scoped to the editor-related fields so SettingsPanel-owned
// edits (semantic-search) don't trigger spurious PATCHes.

describe("fullstack-a-46: HybridEditorConfig wiring", () => {
  test("warning copy distinguishes device-wide scope from per-Hybrid override", () => {
    expect(source).toMatch(
      /These settings apply to ALL editors, not just this one\./,
    );
    expect(source).toMatch(/class="hint warning"/);
  });

  test("Editor theme radios cover the three shipped themes + bind editor_theme", () => {
    expect(source).toContain('"github"');
    expect(source).toContain('"google_docs"');
    expect(source).toContain('"word"');
    expect(source).toMatch(
      /name="hybrid-editor-theme"[\s\S]*?editing!\.editor_theme = opt\.value as EditorTheme/,
    );
  });

  test("Appearance radios drive setThemeChoice + sync editing.theme", () => {
    expect(source).toContain('"system"');
    expect(source).toContain('"light"');
    expect(source).toContain('"dark"');
    expect(source).toMatch(/setThemeChoice\(v\)/);
    expect(source).toMatch(/if \(editing\) editing\.theme = v/);
  });

  test("Layout radios cover standard + compact + bind line_spacing", () => {
    expect(source).toMatch(/name="hybrid-line-spacing"/);
    expect(source).toContain('"standard"');
    expect(source).toContain('"compact"');
    expect(source).toMatch(
      /editing!\.line_spacing = opt\.value as LineSpacing/,
    );
  });

  test("Date pills select binds date_format + iterates DATE_FORMATS", () => {
    expect(source).toMatch(/bind:value=\{editing\.date_format\}/);
    expect(source).toMatch(/\{#each DATE_FORMATS as f \(f\.id\)\}/);
  });

  test("On save checkbox binds strip_trailing_whitespace_on_save", () => {
    expect(source).toMatch(
      /bind:checked=\{editing\.strip_trailing_whitespace_on_save\}/,
    );
  });

  test("live-applies the data-editor-theme attribute on every editor_theme change", () => {
    // Carry-over from SettingsPanel: the editor in the background
    // re-skins without waiting for the autosave round-trip.
    expect(source).toMatch(
      /setAttribute\([\s\S]*?"data-editor-theme"[\s\S]*?editing\.editor_theme/,
    );
  });

  test("keeps editorToolsPrefs.stripTrailingWhitespaceOnSave in sync", () => {
    // `fullstack-a-25` carry-over: the editor's save() reads
    // editorToolsPrefs synchronously; the toggle must propagate
    // before the next save without waiting for the autosave +
    // server round-trip.
    expect(source).toMatch(
      /editorToolsPrefs\.stripTrailingWhitespaceOnSave\s*=\s*editing\.strip_trailing_whitespace_on_save/,
    );
  });

  test("save merges only editor fields onto the server's current GlobalConfig", () => {
    // Race safety: re-fetch the latest config, overlay editor
    // fields, then PATCH. SettingsPanel's semantic-search save
    // can't be clobbered by a parallel HybridEditorConfig save.
    expect(source).toMatch(/const current = await api\.config\(\)/);
    expect(source).toMatch(
      /preferences:\s*\{[\s\S]*?\.\.\.current\.preferences[\s\S]*?editor_theme:\s*editing\.editor_theme[\s\S]*?strip_trailing_whitespace_on_save:[\s\S]*?editing\.strip_trailing_whitespace_on_save/,
    );
    expect(source).toMatch(/await api\.updateConfig\(cfgBody\)/);
  });

  test("dirty check is scoped to the five editor-related fields", () => {
    // Comparing the whole Preferences would react to terminal /
    // semantic-search / about edits owned by other surfaces and
    // fire spurious PATCHes (worse: a PATCH from here could
    // clobber state SettingsPanel hadn't yet flushed).
    expect(source).toMatch(/function editorDirty\(\): boolean/);
    expect(source).toMatch(/editing\.editor_theme !== server\.editor_theme/);
    expect(source).toMatch(/editing\.theme !== server\.theme/);
    expect(source).toMatch(/editing\.line_spacing !== server\.line_spacing/);
    expect(source).toMatch(/editing\.date_format !== server\.date_format/);
    expect(source).toMatch(
      /editing\.strip_trailing_whitespace_on_save !==[\s\S]*?server\.strip_trailing_whitespace_on_save/,
    );
  });

  test("normalizeEditor backfills line_spacing + date_format defaults", () => {
    // line_spacing "tight" → "compact" migration + the catalog
    // default fallback for retired date_format ids carry over
    // from SettingsPanel's normalizePrefs. Keeps the dirty()
    // comparison stable across a server re-fetch.
    expect(source).toMatch(
      /if \(p\.line_spacing === "tight"\) p\.line_spacing = "compact"/,
    );
    expect(source).toMatch(/DATE_FORMATS\[0\]!\.id/);
  });
});

describe("fullstack-a-46: Editor section removed from SettingsPanel", () => {
  test("section headers for the migrated sections are gone", () => {
    expect(panel).not.toMatch(/<h3>Editor theme<\/h3>/);
    expect(panel).not.toMatch(/<h3>Appearance<\/h3>/);
    expect(panel).not.toMatch(/<h3>Layout<\/h3>/);
    expect(panel).not.toMatch(/<h3>Date pills<\/h3>/);
    expect(panel).not.toMatch(/<h3>On save<\/h3>/);
  });

  test("editor-only imports are gone from SettingsPanel", () => {
    // Assert against import statements specifically; "carry-over"
    // comments that mention the symbols are benign.
    expect(panel).not.toMatch(/import\s+\{[^}]*\bEditorTheme\b/);
    expect(panel).not.toMatch(/import\s+\{[^}]*\bLineSpacing\b/);
    expect(panel).not.toMatch(/import\s+\{[^}]*\bThemeChoice\b/);
    expect(panel).not.toMatch(/import\s+\{[^}]*\bsetThemeChoice\b/);
    expect(panel).not.toMatch(/import\s+\{[^}]*\bDATE_FORMATS\b/);
    expect(panel).not.toMatch(/import\s+\{[^}]*\beditorToolsPrefs\b/);
  });

  test("editor-side $effects no longer present in SettingsPanel", () => {
    expect(panel).not.toMatch(/setAttribute\([\s\S]*?"data-editor-theme"/);
    expect(panel).not.toMatch(/editorToolsPrefs\.stripTrailingWhitespaceOnSave/);
  });

  test("editor preference field accesses are gone from SettingsPanel", () => {
    expect(panel).not.toMatch(/editing\.editor_theme/);
    expect(panel).not.toMatch(/editing\.line_spacing/);
    expect(panel).not.toMatch(/editing\.date_format/);
    expect(panel).not.toMatch(
      /editing\.strip_trailing_whitespace_on_save/,
    );
  });
});
