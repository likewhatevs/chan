<script lang="ts">
  // Editor settings on the Hybrid back-side mount point. Four
  // sections live here: Editor theme, Layout (line spacing), Date
  // pills (date format), On save (strip trailing whitespace).
  // Appearance lives on the Dashboard back-of-card as the global
  // default; this back side only offers the top-bar body theme switch
  // shared by all Hybrid Editor tabs.
  //
  // Same self-contained / merge-against-current-server save shape as
  // `HybridTerminalConfig.svelte`. The dirty comparator is scoped to
  // the editor-related preference fields so a
  // parallel save elsewhere (HybridFileBrowserConfig's
  // semantic-search etc.) doesn't trigger a spurious PATCH from
  // here, and vice versa.

  import { api } from "../api/client";
  import type {
    EditorTheme,
    GlobalConfig,
    LineSpacing,
    Preferences,
  } from "../api/types";
  import { workspace } from "../state/store.svelte";
  import { DATE_FORMATS } from "../editor/dateFormats";
  import { editorToolsPrefs } from "../state/editorTools.svelte";
  import HybridSurfaceConfigShell from "./HybridSurfaceConfigShell.svelte";

  let { onDone }: { onDone?: () => void } = $props();
  type SaveStatus = "idle" | "saving" | "saved" | { error: string };

  /// Local edit buffer for the editor-related preference slice.
  /// Mirrors `workspace.info.preferences` so the form can debounce
  /// edits into a single PATCH; a $effect re-syncs from the
  /// server whenever workspace.info changes and no local edit is
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
  /// Tracks the JSON snapshot of the server's editor-related
  /// preference slice the last time we re-synced `editing` from it.
  /// Without this guard the hydration effect would reassign `editing`
  /// to a content-identical clone on every workspace.info change
  /// (including the one triggered by our own save), producing a new
  /// $state proxy each pass and re-firing the effect on its own
  /// write, which Svelte 5 rejects with
  /// `effect_update_depth_exceeded`.
  let lastSyncedServerSnap: string | null = null;

  function clone(p: Preferences): Preferences {
    return JSON.parse(JSON.stringify(p));
  }

  function serverEditorSnapshot(p: Preferences | null | undefined): string {
    if (!p) return "null";
    return JSON.stringify({
      editor_theme: p.editor_theme,
      line_spacing: p.line_spacing,
      date_format: p.date_format,
      strip_trailing_whitespace_on_save: p.strip_trailing_whitespace_on_save,
    });
  }

  /// Normalize editor-related fields. Migrates "tight" → "compact"
  /// and falls back to "standard" for any unrecognized line_spacing
  /// value. date_format falls through to the catalog default when
  /// the persisted id is no longer in the known set. Keeps the
  /// dirty() comparison stable across a server re-fetch.
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

  /// This effect bails when the server's editor slice JSON hasn't
  /// actually changed, so the $state proxy identity stays stable
  /// post-save. Reassigning `editing = normalize(...)` on every fire
  /// would replace the proxy with a content-identical clone after a
  /// save (workspace.info changes -> effect re-fires -> editing
  /// reassigned -> effect re-fires on its own write -> ...) and trip
  /// Svelte 5's `effect_update_depth_exceeded` guard.
  $effect(() => {
    const info = workspace.info;
    if (!info) return;
    if (editing && editorSnapshot() !== lastSentSnapshot) {
      if (lastSentSnapshot === null) return;
    }
    const serverSnap = serverEditorSnapshot(info.preferences);
    if (editing && serverSnap === lastSyncedServerSnap) {
      // Server-side editor slice unchanged since the last
      // hydration; the live buffer already matches. Reassigning
      // would create an identical-content clone and re-trigger
      // this effect.
      return;
    }
    lastSyncedServerSnap = serverSnap;
    editing = normalizeEditor(clone(info.preferences));
  });

  /// Live-apply the editor-theme attribute on every change so the
  /// editor in the background re-skins instantly, without waiting
  /// for the 500 ms autosave + server round-trip.
  $effect(() => {
    if (!editing) return;
    document.documentElement.setAttribute(
      "data-editor-theme",
      editing.editor_theme,
    );
  });

  /// Keep the local editor-tools snapshot in sync with
  /// `editing.strip_trailing_whitespace_on_save`
  /// so save() in the editor (which reads editorToolsPrefs)
  /// observes the new value the moment the user toggles it here.
  $effect(() => {
    if (!editing) return;
    editorToolsPrefs.stripTrailingWhitespaceOnSave =
      editing.strip_trailing_whitespace_on_save;
  });

  /// Dirty check scoped to the editor-related preference fields.
  /// Comparing the whole preferences object would react to edits
  /// owned by other back-of-card surfaces (terminal config, FB
  /// semantic-search, etc.) and fire spurious PATCHes.
  function editorDirty(): boolean {
    if (!editing || !workspace.info) return false;
    const server = workspace.info.preferences;
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

  /// Save the editor-related slice. Fetches the latest GlobalConfig
  /// from the server first, overlays only the editor-related fields,
  /// then PATCHes. Parallel autosaves from other back-of-card surfaces
  /// (semantic-search, etc.) cannot be clobbered.
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
        workspaces: current.workspaces,
      };
      await api.updateConfig(cfgBody);
      const info = await api.workspace();
      workspace.info = info;
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
        Style of the markdown editor only - typography, headings,
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
     same theme-row / theme-opt pill shape used by the Dashboard
     back-of-card's Appearance section. Each back-side surface
     keeps a local copy so they don't depend on a sibling's CSS
     being mounted. */
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
  /* Trailing-whitespace toggle reuses the chip pill from the radio
     rows. The `strip-toggle` class name keeps the semantics tied to
     its content so it doesn't collide with semantic-search controls
     in HybridFileBrowserConfig. */
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
