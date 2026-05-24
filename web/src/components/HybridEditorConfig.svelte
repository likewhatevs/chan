<script lang="ts">
  // `fullstack-a-46` Task C: Editor settings migrated out of
  // `SettingsPanel.svelte` into the Hybrid back-side mount point
  // introduced by `-a-43` Task A. Four sections live here:
  // Editor theme, Layout (line spacing), Date pills (date format),
  // On save (strip trailing whitespace). Appearance stays in the
  // main Settings overlay as the global default. This back side
  // only offers the top-bar body theme switch shared by all
  // Hybrid Editor tabs.
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
  import { drive } from "../state/store.svelte";
  import { DATE_FORMATS } from "../editor/dateFormats";
  import { editorToolsPrefs } from "../state/editorTools.svelte";
  import HybridSurfaceConfigShell from "./HybridSurfaceConfigShell.svelte";

  let { onDone }: { onDone?: () => void } = $props();
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

<HybridSurfaceConfigShell
  title="Hybrid Editor"
  surface="editor"
  saveStatus={saveStatus}
  {onDone}
>
    <p class="hint warning">
      Most settings here apply to ALL editors on this device. The
      top-bar theme switch applies to ALL editor bodies.
    </p>

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
</HybridSurfaceConfigShell>

<style>
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
  /* Theme + Layout + Date pills + On save share the
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
