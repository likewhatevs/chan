import { describe, expect, test } from "vitest";
import source from "./HybridEditorConfig.svelte?raw";
import shell from "./HybridSurfaceConfigShell.svelte?raw";

// Editor settings (theme, line spacing, date format, strip trailing
// whitespace) live in HybridEditorConfig. The dirty comparator and save
// are scoped to the editor fields so parallel saves from other surfaces
// do not trigger spurious PATCHes. Appearance and Screen Lock live in
// the Dashboard back-of-card, not here.

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
    // The background editor re-skins immediately without waiting for
    // the autosave round-trip.
    expect(source).toMatch(
      /setAttribute\([\s\S]*?"data-editor-theme"[\s\S]*?editing\.editor_theme/,
    );
  });

  test("keeps editorToolsPrefs.stripTrailingWhitespaceOnSave in sync", () => {
    // The editor's save() reads editorToolsPrefs synchronously, so the
    // toggle must propagate before the next save.
    expect(source).toMatch(
      /editorToolsPrefs\.stripTrailingWhitespaceOnSave\s*=\s*editing\.strip_trailing_whitespace_on_save/,
    );
  });

  test("save merges only editor fields through the serialized config write chain", () => {
    // Race safety: the shared updateGlobalConfigSerial chain re-reads the
    // latest config, overlays only the editor fields, and writes without
    // interleaving — so a parallel save from another surface (e.g.
    // HybridFileBrowserConfig's semantic-search, a theme override) can't be
    // clobbered.
    expect(source).toMatch(
      /import \{ updateGlobalConfigSerial, workspace \} from "\.\.\/state\/store\.svelte"/,
    );
    expect(source).toMatch(
      /await updateGlobalConfigSerial\(\(prefs\) => \(\{ \.\.\.prefs, \.\.\.slice \}\)\)/,
    );
    expect(source).toMatch(
      /const slice = \{[\s\S]*?editor_theme: editing\.editor_theme,[\s\S]*?strip_trailing_whitespace_on_save:[\s\S]*?editing\.strip_trailing_whitespace_on_save,/,
    );
    expect(source).not.toMatch(/const cfgBody: GlobalConfig/);
  });

  test("dirty check is scoped to the four editor-related fields", () => {
    // dirty() compares four fields: editor_theme + line_spacing +
    // date_format + strip_trailing_whitespace_on_save. The app theme
    // (editing.theme) is owned by the Dashboard back-of-card.
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
    // Migrates line_spacing "tight" -> "compact" and provides a catalog
    // fallback for retired date_format ids, keeping dirty() stable
    // across server re-fetches.
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

describe("post-save effect_update_depth_exceeded guard", () => {
  // The hydration $effect must not reassign `editing` to a
  // content-identical clone on every workspace.info change, which
  // replaces the $state proxy and re-fires the effect -> Svelte 5
  // trips effect_update_depth_exceeded (UI freeze). The fix tracks a
  // JSON snapshot and bails when the server editor slice is unchanged.
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
