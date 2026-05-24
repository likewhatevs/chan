<script lang="ts">
  // `fullstack-a-46` Task C: Editor settings migrated out of
  // `SettingsPanel.svelte` into the Hybrid back-side mount point
  // introduced by `-a-43` Task A. Four sections live here:
  // Editor theme, Layout (line spacing), Date pills (date format),
  // On save (strip trailing whitespace).
  //
  // `fullstack-a-53` reverted the Appearance section back to
  // SettingsPanel — Appearance is a GLOBAL default with per-Hybrid
  // OVERRIDES (added below as a new 3-option toggle), not a
  // per-Hybrid-only setting. The 3-option Inherit / Light / Dark
  // override toggle writes to `pane.theme` (the existing per-
  // Hybrid override slot from `-b-5`/`-a-47`). Resolution at
  // render: `pane.theme` wins if set; else the global
  // `ui.themeChoice` from Settings.
  //
  // Same self-contained / merge-against-current-server save shape
  // as `HybridTerminalConfig.svelte` (-a-45). The dirty comparator
  // is scoped to the editor-related preference fields so a
  // SettingsPanel edit elsewhere doesn't trigger a spurious PATCH
  // from here, and vice versa.

  import { api } from "../api/client";
  import type {
    EditorTheme,
    GlobalConfig,
    LineSpacing,
    Preferences,
  } from "../api/types";
  import { drive, ui } from "../state/store.svelte";
  import type { HybridTheme, LeafNode } from "../state/tabs.svelte";

  /// `fullstack-a-53` per-Hybrid theme override toggle.
  /// `pane` is the Hybrid pane this back-side surface belongs to.
  /// The override radios (Inherit / Light / Dark) write to
  /// `pane.theme` (the existing per-Hybrid override slot from
  /// `-b-5`/`-a-47`). Resolution at render time:
  /// `pane.theme ?? ui.theme`.
  let { pane, onDone }: { pane: LeafNode; onDone?: () => void } = $props();

  type OverrideChoice = "inherit" | HybridTheme;
  const overrideValue = $derived<OverrideChoice>(
    pane.theme ?? "inherit",
  );

  function setOverrideChoice(next: OverrideChoice): void {
    if (next === "inherit") {
      pane.theme = undefined;
    } else {
      pane.theme = next;
    }
  }
  import { DATE_FORMATS } from "../editor/dateFormats";
  import { editorToolsPrefs } from "../state/editorTools.svelte";

  type SaveStatus = "idle" | "saving" | "saved" | { error: string };

  /// Local edit buffer for the editor-related preference slice.
  /// Mirrors `drive.info.preferences` so the form can debounce
  /// edits into a single PATCH; a $effect re-syncs from the
  /// server whenever drive.info changes and no local edit is
  /// in flight.
  let editing = $state<Preferences | null>(null);
  let saveStatus = $state<SaveStatus>("idle");

  const AUTOSAVE_DELAY_MS = 500;
  const SAVED_FLASH_MS = 1500;
  let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  let savedFlashTimer: ReturnType<typeof setTimeout> | null = null;
  let inflight = false;
  let lastSentSnapshot: string | null = null;
  let failedSaveSnap: string | null = null;

  function clone(p: Preferences): Preferences {
    return JSON.parse(JSON.stringify(p));
  }

  /// Normalize editor-related fields. The line_spacing migration
  /// from "tight" → "compact" + the fallback to "standard" carry
  /// over from SettingsPanel's `normalizePrefs`; date_format
  /// falls through to the catalog default when the persisted id
  /// has been retired. Keeps the dirty() comparison stable across
  /// a server re-fetch.
  function normalizeEditor(p: Preferences): Preferences {
    if (p.line_spacing === "tight") p.line_spacing = "compact";
    if (p.line_spacing !== "compact" && p.line_spacing !== "standard") {
      p.line_spacing = "standard";
    }
    const knownIds = new Set(DATE_FORMATS.map((f) => f.id));
    if (!knownIds.has(p.date_format as never)) {
      p.date_format = DATE_FORMATS[0]!.id;
    }
    return p;
  }

  function editorSnapshot(): string {
    if (!editing) return "null";
    return JSON.stringify({
      editor_theme: editing.editor_theme,
      line_spacing: editing.line_spacing,
      date_format: editing.date_format,
      strip_trailing_whitespace_on_save:
        editing.strip_trailing_whitespace_on_save,
    });
  }

  $effect(() => {
    const info = drive.info;
    if (!info) return;
    if (editing && editorSnapshot() !== lastSentSnapshot) {
      if (lastSentSnapshot === null) return;
    }
    editing = normalizeEditor(clone(info.preferences));
  });

  /// Live-apply the editor-theme attribute on every change so the
  /// editor in the background re-skins instantly, without waiting
  /// for the 500 ms autosave + server round-trip. Carry-over from
  /// SettingsPanel.
  $effect(() => {
    if (!editing) return;
    document.documentElement.setAttribute(
      "data-editor-theme",
      editing.editor_theme,
    );
  });

  /// `fullstack-a-25` carry-over: keep the local editor-tools
  /// snapshot in sync with `editing.strip_trailing_whitespace_on_save`
  /// so save() in the editor (which reads editorToolsPrefs)
  /// observes the new value the moment the user toggles it here.
  $effect(() => {
    if (!editing) return;
    editorToolsPrefs.stripTrailingWhitespaceOnSave =
      editing.strip_trailing_whitespace_on_save;
  });

  /// Dirty check scoped to the editor-related preference fields.
  /// Comparing the whole preferences object would react to
  /// SettingsPanel-owned edits (terminal moved to its own back,
  /// semantic-search, etc.) and fire spurious PATCHes.
  function editorDirty(): boolean {
    if (!editing || !drive.info) return false;
    const server = drive.info.preferences;
    return (
      editing.editor_theme !== server.editor_theme ||
      editing.line_spacing !== server.line_spacing ||
      editing.date_format !== server.date_format ||
      editing.strip_trailing_whitespace_on_save !==
        server.strip_trailing_whitespace_on_save
    );
  }

  function scheduleSave(): void {
    if (autosaveTimer) clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => {
      autosaveTimer = null;
      void save();
    }, AUTOSAVE_DELAY_MS);
  }

  /// Save the editor-related slice. Mirrors `-a-45`'s
  /// merge-against-current-server pattern: fetch the latest
  /// GlobalConfig from the server first, overlay only the
  /// editor-related fields, then PATCH. SettingsPanel's parallel
  /// autosave (semantic-search, etc.) can not be clobbered.
  async function save(): Promise<void> {
    if (!editing || inflight) return;
    if (!editorDirty()) return;
    inflight = true;
    saveStatus = "saving";
    if (savedFlashTimer) {
      clearTimeout(savedFlashTimer);
      savedFlashTimer = null;
    }
    const sent = editorSnapshot();
    lastSentSnapshot = sent;
    try {
      const current = await api.config();
      const cfgBody: GlobalConfig = {
        preferences: {
          ...current.preferences,
          editor_theme: editing.editor_theme,
          line_spacing: editing.line_spacing,
          date_format: editing.date_format,
          strip_trailing_whitespace_on_save:
            editing.strip_trailing_whitespace_on_save,
        },
        default_drive_root: current.default_drive_root,
        drives: current.drives,
      };
      await api.updateConfig(cfgBody);
      const info = await api.drive();
      drive.info = info;
      editing = normalizeEditor(clone(info.preferences));
      lastSentSnapshot = editorSnapshot();
      failedSaveSnap = null;
      saveStatus = "saved";
      savedFlashTimer = setTimeout(() => {
        if (saveStatus === "saved") saveStatus = "idle";
        savedFlashTimer = null;
      }, SAVED_FLASH_MS);
    } catch (e) {
      const message = (e as Error).message;
      failedSaveSnap = sent;
      saveStatus = { error: message };
    } finally {
      inflight = false;
      if (editorDirty() && editorSnapshot() !== failedSaveSnap) {
        scheduleSave();
      }
    }
  }

  $effect(() => {
    if (!editing) return;
    const snap = editorSnapshot();
    if (!editorDirty()) return;
    if (snap === failedSaveSnap) return;
    scheduleSave();
  });
</script>

<section class="hybrid-config" aria-label="Hybrid Editor configuration">
  <header class="config-header">
    <h2 class="config-title">Hybrid Editor</h2>
    <div class="save-status" aria-live="polite">
      {#if saveStatus === "saving"}
        <span class="muted">saving…</span>
      {:else if saveStatus === "saved"}
        <span class="ok">saved</span>
      {:else if typeof saveStatus === "object"}
        <span class="err" title={saveStatus.error}>save failed</span>
      {/if}
    </div>
    <button type="button" class="config-ok" onclick={() => onDone?.()}>OK</button>
  </header>
  <div class="config-body">
    <p class="hint warning">
      Most settings here apply to ALL editors on this device; the
      Appearance override below applies only to THIS Hybrid pane.
    </p>

    <!-- `fullstack-a-53` per-Hybrid Appearance override. The
         global Appearance default lives in the Settings overlay;
         this toggle layers an explicit Light / Dark per-Hybrid
         override on top, or falls through to Inherit. Render
         resolution: `pane.theme ?? ui.theme`. -->
    <section>
      <h3>Appearance (this Hybrid)</h3>
      <p class="hint">
        Override the global Appearance default for just this
        Hybrid pane. Inherit follows the global Settings choice
        (currently
        <strong>{ui.themeChoice}</strong>).
      </p>
      <div class="theme-row" role="radiogroup" aria-label="Per-Hybrid Appearance override">
        {#each [
          { value: "inherit" as const, label: "Inherit" },
          { value: "light" as const, label: "Light" },
          { value: "dark" as const, label: "Dark" },
        ] as opt (opt.value)}
          <label class="theme-opt" class:on={overrideValue === opt.value}>
            <input
              type="radio"
              name="hybrid-editor-theme-override"
              value={opt.value}
              checked={overrideValue === opt.value}
              onchange={() => setOverrideChoice(opt.value)}
            />
            <span>{opt.label}</span>
          </label>
        {/each}
      </div>
    </section>

    {#if editing}
    <section>
      <h3>Editor theme</h3>
      <p class="hint">
        Style of the markdown editor only — typography, headings,
        code blocks, links, tables.
      </p>
      <div class="theme-row" role="radiogroup" aria-label="Editor theme">
        {#each [
          { value: "github", label: "GitHub" },
          { value: "google_docs", label: "Google Docs" },
          { value: "word", label: "Microsoft Word" },
        ] as opt (opt.value)}
          <label
            class="theme-opt"
            class:on={editing.editor_theme === opt.value}
          >
            <input
              type="radio"
              name="hybrid-editor-theme"
              value={opt.value}
              checked={editing.editor_theme === opt.value}
              onchange={() => {
                editing!.editor_theme = opt.value as EditorTheme;
              }}
            />
            <span>{opt.label}</span>
          </label>
        {/each}
      </div>
    </section>

    <section>
      <h3>Layout</h3>
      <p class="hint">
        Standard is the default reading density; compact tightens
        paragraph and list spacing while keeping the editor readable.
      </p>
      <div class="theme-row" role="radiogroup" aria-label="Line spacing">
        {#each [
          { value: "standard", label: "Standard" },
          { value: "compact", label: "Compact" },
        ] as opt (opt.value)}
          <label class="theme-opt" class:on={editing.line_spacing === opt.value}>
            <input
              type="radio"
              name="hybrid-line-spacing"
              value={opt.value}
              checked={editing.line_spacing === opt.value}
              onchange={() => {
                editing!.line_spacing = opt.value as LineSpacing;
              }}
            />
            <span>{opt.label}</span>
          </label>
        {/each}
      </div>
    </section>

    <section>
      <h3>Date pills</h3>
      <p class="hint">
        Format used by <code>@today</code> and pre-selected in the
        <code>@date</code> picker. The editor still detects every
        format on this list when reading a file or watching you
        type, so old documents keep auto-pilling regardless of
        which one is the default here.
      </p>
      <label class="font-row">
        <span>Default</span>
        <select class="config-select family" bind:value={editing.date_format}>
          {#each DATE_FORMATS as f (f.id)}
            <option value={f.id}>{f.label}</option>
          {/each}
        </select>
      </label>
    </section>

    <section>
      <h3>On save</h3>
      <p class="hint">
        Strip trailing whitespace from each line when the file is
        saved. Affects every editable text buffer (.md, .txt, source
        files). The one-shot "Remove trailing whitespace" action in
        the editor menu still works for manual cleanup.
      </p>
      <label class="theme-opt strip-toggle" class:on={editing.strip_trailing_whitespace_on_save}>
        <input
          type="checkbox"
          bind:checked={editing.strip_trailing_whitespace_on_save}
        />
        <span>Strip trailing whitespace on save</span>
      </label>
    </section>
    {/if}
  </div>
</section>

<style>
  .hybrid-config {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  .config-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }
  .config-title {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--text);
  }
  .save-status { font-size: 14px; min-width: 60px; text-align: right; }
  .save-status .ok { color: var(--accent); }
  .save-status .err { color: #d33; }
  .save-status .muted { color: var(--text-secondary); }
  .config-ok {
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 5px 12px;
    font: inherit;
    cursor: pointer;
  }
  .config-ok:hover {
    border-color: var(--btn-hover);
  }
  .config-body {
    flex: 1;
    overflow: auto;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }
  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .hint.warning {
    border-left: 3px solid var(--accent, #f97316);
    padding: 0.5rem 0.75rem;
    background: color-mix(in srgb, var(--accent, #f97316) 6%, transparent);
    border-radius: 4px;
  }
  /* Per-section vertical grouping. Each <section> hosts a header,
     a hint paragraph, and one control row (radios or select). */
  .config-body :global(section) {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .config-body :global(section h3) {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }
  /* Theme + Layout + Appearance + Date pills + On save share the
     same theme-row / theme-opt pill shape from SettingsPanel.
     Copied locally so this back-side surface doesn't depend on
     SettingsPanel CSS being mounted. */
  .theme-row { display: flex; gap: 4px; flex-wrap: wrap; }
  .theme-opt {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    background: var(--btn-bg);
    cursor: pointer;
    font-size: 14px;
  }
  .theme-opt input[type="radio"],
  .theme-opt input[type="checkbox"] {
    width: auto;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
  }
  .theme-opt > span { color: var(--text); }
  .theme-opt:hover { border-color: var(--btn-hover); }
  .theme-opt.on {
    border-color: var(--link);
    background: var(--hover-bg);
  }
  /* `fullstack-a-25` carry-over: trailing-whitespace toggle reuses
     the chip pill from the radio rows. Renamed `strip-toggle` to
     keep the class semantics tied to its content (was
     `.semantic-toggle` in SettingsPanel; that name no longer
     fits since semantic-search stayed there). */
  .strip-toggle input[type="checkbox"]:disabled,
  .strip-toggle:has(input[type="checkbox"]:disabled) {
    cursor: not-allowed;
    opacity: 0.7;
  }
  .font-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  .font-row > span {
    color: var(--text-secondary);
    font-size: 14px;
    min-width: 5em;
  }
  .font-row select.family {
    flex: 1;
    min-width: 12em;
  }
  .config-select {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 5px 7px;
    font: inherit;
  }
</style>
