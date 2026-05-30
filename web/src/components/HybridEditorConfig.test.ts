import { describe, expect, test } from "vitest";
import source from "./HybridEditorConfig.svelte?raw";
import shell from "./HybridSurfaceConfigShell.svelte?raw";

// `fullstack-a-46` Task C: Editor settings UI migrated out of the
// (since-retired) global Settings overlay into HybridEditorConfig.
// Four sections move: Editor theme, Layout (line spacing), Date
// pills (date format), On save (strip trailing whitespace).
// Settings storage shape unchanged; both surfaces still PATCH the
// same `GlobalConfig.preferences`. The dirty comparator + save
// are scoped to the editor-related fields so parallel saves
// from other surfaces (e.g. HybridFileBrowserConfig's
// semantic-search) didn't trigger spurious PATCHes.
//
// `phase-13 lane-b` slice 3c: the global Settings overlay was
// retired (Appearance + Screen Lock + Screensaver moved to
// DashboardTab back-of-card). The migration-direction assertions
// that used to pin "X is gone from the old overlay" are dropped —
// the file no longer exists, so there's nothing left to regress
// against.

describe("HybridEditorConfig wiring", () => {
  test("warning copy distinguishes device-wide settings from body theme scope", () => {
    expect(source).toMatch(
      /Most settings here apply to ALL editors on this device/,
    );
    expect(source).toMatch(/top-bar theme switch applies to ALL editor bodies/);
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

  test("top-bar body theme is delegated to the shared surface shell", () => {
    expect(source).toMatch(
      /<HybridSurfaceConfigShell[\s\S]{1,160}title="Hybrid Editor"[\s\S]{1,120}surface="editor"/,
    );
    expect(source).not.toMatch(/<h3>Appearance<\/h3>/);
    expect(source).not.toMatch(/name="hybrid-editor-theme-override"/);
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
    // Carry-over from the retired global Settings overlay: the
    // editor in the background re-skins without waiting for the
    // autosave round-trip.
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
    // fields, then PATCH. Parallel saves from other surfaces
    // (e.g. HybridFileBrowserConfig's semantic-search save) can't
    // be clobbered by a HybridEditorConfig save.
    expect(source).toMatch(/const current = await api\.config\(\)/);
    expect(source).toMatch(
      /preferences:\s*\{[\s\S]*?\.\.\.current\.preferences[\s\S]*?editor_theme:\s*editing\.editor_theme[\s\S]*?strip_trailing_whitespace_on_save:[\s\S]*?editing\.strip_trailing_whitespace_on_save/,
    );
    expect(source).toMatch(/await api\.updateConfig\(cfgBody\)/);
  });

  test("dirty check is scoped to the four editor-related fields (-a-53)", () => {
    // `-a-53` reverted Appearance to the global Settings overlay
    // (since retired in phase-13 lane-b slice 3c, where Appearance
    // moved to DashboardTab back-of-card). The `editing.theme`
    // field is no longer touched here. dirty() compares 4 fields
    // now: editor_theme + line_spacing + date_format +
    // strip_trailing_whitespace_on_save.
    expect(source).toMatch(/function editorDirty\(\): boolean/);
    expect(source).toMatch(/editing\.editor_theme !== server\.editor_theme/);
    expect(source).not.toMatch(/editing\.theme !== server\.theme/);
    expect(source).toMatch(/editing\.line_spacing !== server\.line_spacing/);
    expect(source).toMatch(/editing\.date_format !== server\.date_format/);
    expect(source).toMatch(
      /editing\.strip_trailing_whitespace_on_save !==[\s\S]*?server\.strip_trailing_whitespace_on_save/,
    );
  });

  test("normalizeEditor backfills line_spacing + date_format defaults", () => {
    // line_spacing "tight" → "compact" migration + the catalog
    // default fallback for retired date_format ids carry over
    // from the retired global Settings overlay's normalizePrefs.
    // Keeps the dirty() comparison stable across a server
    // re-fetch.
    expect(source).toMatch(
      /if \(p\.line_spacing === "tight"\) p\.line_spacing = "compact"/,
    );
    expect(source).toMatch(/DATE_FORMATS\[0\]!\.id/);
  });
});

describe("Wave 4: Editor back-side controls", () => {
  test("onDone prop is accepted and OK button routes through it", () => {
    expect(source).toMatch(
      /let \{ onDone \}: \{ onDone\?: \(\) => void \} = \$props\(\)/,
    );
    expect(source).toMatch(/<HybridSurfaceConfigShell[\s\S]*?\{onDone\}/);
    expect(shell).toMatch(
      /<button type="button" class="config-ok" onclick=\{\(\) => onDone\?\.\(\)\}>OK<\/button>/,
    );
  });

  test("Date pills dropdown uses the polished config-select style", () => {
    expect(source).toMatch(
      /<select class="config-select family" bind:value=\{editing\.date_format\}>/,
    );
    expect(source).toMatch(/\.config-select \{[\s\S]{1,300}border: 1px solid var\(--border\)/);
  });
});

describe("(C1): post-save effect_update_depth_exceeded guard", () => {
  // The hydration $effect used to reassign `editing` to a
  // content-identical clone on every workspace.info change, which
  // replaced the $state proxy and re-fired the effect on its own
  // write -> Svelte 5 trips effect_update_depth_exceeded. Repro:
  // open a draft, flip the editor's Hybrid back, switch the
  // editor theme; save() reassigns workspace.info, the effect
  // cycles, the UI freezes.
  test("tracks lastSyncedServerSnap across workspace.info refreshes", () => {
    expect(source).toMatch(
      /let lastSyncedServerSnap: string \| null = null;/,
    );
  });

  test("serverEditorSnapshot mirrors the local editorSnapshot field set", () => {
    expect(source).toMatch(
      /function serverEditorSnapshot\(p: Preferences \| null \| undefined\): string \{[\s\S]{1,800}editor_theme: p\.editor_theme,[\s\S]{1,200}line_spacing: p\.line_spacing,[\s\S]{1,200}date_format: p\.date_format,[\s\S]{1,200}strip_trailing_whitespace_on_save: p\.strip_trailing_whitespace_on_save,/,
    );
  });

  test("hydration effect bails when the server editor slice hasn't changed", () => {
    expect(source).toMatch(
      /\$effect\(\(\) => \{[\s\S]{1,2000}const serverSnap = serverEditorSnapshot\(info\.preferences\);[\s\S]{1,400}if \(editing && serverSnap === lastSyncedServerSnap\) \{[\s\S]{1,800}return;[\s\S]{1,200}\}[\s\S]{1,200}lastSyncedServerSnap = serverSnap;[\s\S]{1,200}editing = normalizeEditor\(clone\(info\.preferences\)\);/,
    );
  });
});
