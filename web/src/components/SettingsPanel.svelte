<script lang="ts">
  // Settings overlay. Per-device-global preferences form (editor
  // theme, editor density, date format, and theme).
  //
  // The drive display name is edited from the file-browser
  // hamburger, not here, so the settings overlay is purely
  // about device-wide preferences.
  //
  // Auto-saves on change (500 ms debounce).

  import { onDestroy, onMount } from "svelte";
  import { api } from "../api/client";
  import type {
    BuildInfo,
    GlobalConfig,
    Preferences,
    SemanticState,
  } from "../api/types";
  import { Maximize2, Minimize2, X } from "lucide-svelte";
  import {
    refreshDrive,
    settingsOverlay,
    drive,
  } from "../state/store.svelte";
  import {
    overlayMaximized,
    setOverlayMaximized,
  } from "../state/pageWidth.svelte";
  import OverlayShell from "./OverlayShell.svelte";

  // `fullstack-a-45` (Task B) moved the Terminal section
  // (scrollback MB + default TERM, originally `fullstack-b-11`) out
  // of this overlay into `HybridTerminalConfig.svelte`.
  //
  // `fullstack-a-46` (Task C) moves the Editor settings
  // (Editor theme, Appearance, Layout / line spacing, Date pills /
  // date format, On save / strip trailing whitespace) out of this
  // overlay into `HybridEditorConfig.svelte`. The relevant
  // imports, derived view helpers, $effects, and CSS scope all
  // live in that component now; this overlay no longer touches
  // the editor preference fields directly.

  function doToggleOverlayMaximized(): void {
    setOverlayMaximized(!overlayMaximized.on);
  }

  const visible = $derived(settingsOverlay.open);

  function close(): void {
    settingsOverlay.open = false;
  }

  let editing = $state<Preferences | null>(null);
  /// Cached global config. Populated on mount and after every
  /// global save. Settings are always per-device-global now (no
  /// per-drive override); we keep the cached payload here so
  /// dirty() can compare the form against the source of truth
  /// without re-fetching on every keystroke.
  let globalConfig = $state<GlobalConfig | null>(null);
  /// Auto-save status surfaced in the tab-bar. "saving…" while the
  /// PATCH is in flight; "saved" briefly after success; the error
  /// string sticks until the next change so a transient failure
  /// stays visible.
  type SaveStatus = "idle" | "saving" | "saved" | { error: string };
  let saveStatus = $state<SaveStatus>("idle");
  /// Build identity for the About footer. Loaded on mount; the
  /// version + embeddings feature flag are static for the running
  /// binary so a single fetch is enough.
  let buildInfo = $state<BuildInfo | null>(null);
  // When the upstream drive info changes (initial load, external
  // edit, server restart), reset the form to the server state.
  // We intentionally only sync into the form when there's no local
  // edit pending, otherwise the user's typing would get clobbered
  // by background polls.
  $effect(() => {
    const info = drive.info;
    if (!info) return;
    if (!editing) {
      editing = normalizePrefs(clone(info.preferences));
    }
  });

  /// Fill in optional preference fields older servers may omit.
  /// Applied to BOTH editing and globalConfig so dirty() doesn't
  /// see a permanent diff and trigger an autosave loop.
  /// `-a-45` moved Terminal normalization to
  /// `HybridTerminalConfig.svelte`; `-a-46` moved Editor
  /// normalization (line_spacing / date_format defaults) to
  /// `HybridEditorConfig.svelte`. This overlay passes both
  /// subtrees through untouched as part of the GlobalConfig
  /// round-trip; only the semantic-search and about sections
  /// remain.
  function normalizePrefs(p: Preferences): Preferences {
    return p;
  }

  function clone(p: Preferences): Preferences {
    return JSON.parse(JSON.stringify(p));
  }

  function snapshot(): string {
    return JSON.stringify({ editing });
  }

  /// True when the form differs from the last server payload. Drives
  /// the auto-save effect: identical-to-server means nothing to do.
  /// Compares against the global config (settings are always
  /// per-device-global now).
  function dirty(): boolean {
    if (!editing || !drive.info) return false;
    if (!globalConfig) return false;
    if (JSON.stringify(editing) !== JSON.stringify(globalConfig.preferences)) {
      return true;
    }
    return false;
  }

  /// Autosave debounce window. 500 ms is long enough to coalesce a
  /// burst of typing into one PATCH but short enough that a quick
  /// edit lands before the user looks away.
  const AUTOSAVE_DELAY_MS = 500;
  /// "saved" status flashes for this long after a successful PATCH
  /// before reverting to "idle" so the indicator doesn't stick.
  const SAVED_FLASH_MS = 1500;

  let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  let savedFlashTimer: ReturnType<typeof setTimeout> | null = null;
  let inflight = false;
  let failedSaveSnap: string | null = null;

  async function save(): Promise<void> {
    if (!editing || inflight) return;
    inflight = true;
    saveStatus = "saving";
    if (savedFlashTimer) {
      clearTimeout(savedFlashTimer);
      savedFlashTimer = null;
    }
    const sent = snapshot();
    try {
      // Prefs (global config) -> PATCH /api/config. Drive name lives
      // in the file-browser hamburger now and the default-root +
      // recent-drives list moved to the drive inspector; this overlay
      // only writes preferences. We round-trip the existing
      // default_drive_root + drives values so we don't clobber
      // anything the drive inspector wrote in parallel.
      const cfgBody: GlobalConfig = {
        preferences: editing,
        default_drive_root: globalConfig?.default_drive_root ?? null,
        drives: globalConfig?.drives,
      };
      await api.updateConfig(cfgBody);
      // Re-fetch authoritative state. Two reads (drive + global)
      // because the prefs save can echo back into drive.info via
      // the indexer / config bridge.
      const [info, cfg] = await Promise.all([api.drive(), api.config()]);
      drive.info = info;
      globalConfig = cfg;
      if (snapshot() === sent) {
        editing = normalizePrefs(clone(info.preferences));
      }
      if (globalConfig) normalizePrefs(globalConfig.preferences);
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
      // If the form went dirty again while saving, schedule another pass.
      if (dirty() && snapshot() !== failedSaveSnap) scheduleSave();
    }
  }

  /// Pull the global config. Used on mount and after global
  /// PATCHes so the form mirrors the persisted values.
  async function loadGlobalConfig(): Promise<void> {
    try {
      globalConfig = await api.config();
      normalizePrefs(globalConfig.preferences);
    } catch {
      globalConfig = null;
    }
  }

  function scheduleSave(): void {
    if (autosaveTimer) clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => {
      autosaveTimer = null;
      void save();
    }, AUTOSAVE_DELAY_MS);
  }

  // Auto-save effect. Watches the editable fields; every change
  // schedules a debounced PATCH. The dirty() guard avoids saving
  // identity-equal state (e.g. right after the post-save re-clone).
  $effect(() => {
    // Read-track every editable field.
    if (!editing) return;
    const snap = snapshot();
    if (!dirty()) return;
    if (snap === failedSaveSnap) return;
    scheduleSave();
  });

  // `fullstack-a-46` Task C moved the editor-theme attribute and
  // editorToolsPrefs side-effects into `HybridEditorConfig.svelte`
  // alongside the matching editor settings. SettingsPanel only
  // owns semantic-search + about now, neither of which needs
  // the editor-theme DOM attribute or the strip-trailing
  // editor-tools snapshot.

  async function loadBuildInfo(): Promise<void> {
    try {
      buildInfo = await api.buildInfo();
    } catch {
      // Non-fatal: footer falls back to "n/a".
      buildInfo = null;
    }
  }

  /// `fullstack-a-21` semantic-search opt-in. Snapshot of the
  /// server's `systacean-7` state plus a downloading flag the UI
  /// owns (we don't store it server-side; it lives in the
  /// in-flight POST). Error sticks until the next user action.
  let semanticState = $state<SemanticState | null>(null);
  let semanticDownloading = $state(false);
  let semanticEnabling = $state(false);
  let semanticError = $state<string | null>(null);
  /// Polling handle used during downloads. The download endpoint
  /// is synchronous in v1 (no per-byte progress events), so we
  /// poll `/api/index/semantic/state` every few seconds during
  /// the wait to detect the `model_present` transition out-of-band
  /// from the awaited POST. Cleared on success / failure / unmount.
  let semanticPollTimer: ReturnType<typeof setInterval> | null = null;
  const SEMANTIC_POLL_INTERVAL_MS = 3000;

  async function loadSemanticState(): Promise<void> {
    try {
      semanticState = await api.semanticState();
    } catch {
      // Older servers without the endpoint surface as a no-op; the
      // section just renders the "not available" placeholder.
      semanticState = null;
    }
  }

  function stopSemanticPoll(): void {
    if (semanticPollTimer !== null) {
      clearInterval(semanticPollTimer);
      semanticPollTimer = null;
    }
  }

  async function semanticToggle(next: boolean): Promise<void> {
    if (!semanticState) return;
    semanticError = null;
    if (next) {
      if (semanticState.model_present) {
        // Model already on disk — just enable. No download wait.
        semanticEnabling = true;
        try {
          semanticState = await api.semanticEnable();
        } catch (err) {
          semanticError = (err as Error).message;
        } finally {
          semanticEnabling = false;
        }
        return;
      }
      // First-time download. Kick off the synchronous POST and
      // start polling state in parallel so the spinner reflects
      // the model_present transition even before the POST returns.
      semanticDownloading = true;
      stopSemanticPoll();
      semanticPollTimer = setInterval(() => {
        void loadSemanticState();
      }, SEMANTIC_POLL_INTERVAL_MS);
      try {
        semanticState = await api.semanticDownload();
        stopSemanticPoll();
        // Server returns the post-download state; auto-enable on
        // top so the toggle lands ON rather than leaving the user
        // a second click on a freshly-downloaded model.
        semanticEnabling = true;
        try {
          semanticState = await api.semanticEnable();
        } finally {
          semanticEnabling = false;
        }
      } catch (err) {
        stopSemanticPoll();
        semanticError = (err as Error).message;
        // Refresh state so the toggle reflects whatever the server
        // ended up with (the download may have partially landed).
        await loadSemanticState();
      } finally {
        semanticDownloading = false;
      }
    } else {
      try {
        semanticState = await api.semanticDisable();
      } catch (err) {
        semanticError = (err as Error).message;
      }
    }
  }

  function formatModelSize(bytes: number | null): string {
    if (bytes === null || bytes <= 0) return "size unknown";
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(1)} MB`;
  }

  onMount(() => {
    // Make sure we have the latest server state when the tab opens.
    void refreshDrive();
    void loadGlobalConfig();
    void loadBuildInfo();
    void loadSemanticState();
  });

  onDestroy(() => {
    stopSemanticPoll();
  });
</script>

<OverlayShell id="settings" open={visible} onClose={close}>
<div class="settings-tab">
  <div class="tab-bar">
    <button
      type="button"
      class="chrome-btn"
      onclick={doToggleOverlayMaximized}
      title={overlayMaximized.on ? "Restore size" : "Maximize"}
      aria-label={overlayMaximized.on ? "Restore size" : "Maximize"}
    >
      {#if overlayMaximized.on}
        <Minimize2 size={14} strokeWidth={1.75} aria-hidden="true" />
      {:else}
        <Maximize2 size={14} strokeWidth={1.75} aria-hidden="true" />
      {/if}
    </button>
    <span class="title">Settings</span>
    <span class="save-status" aria-live="polite">
      {#if saveStatus === "saving"}
        <span class="muted">saving…</span>
      {:else if saveStatus === "saved"}
        <span class="ok">saved</span>
      {:else if typeof saveStatus === "object"}
        <span class="err" title={saveStatus.error}>save failed</span>
      {/if}
    </span>
    <button
      type="button"
      class="chrome-btn close"
      onclick={close}
      title="Close"
      aria-label="Close"
    >
      <X size={14} strokeWidth={1.75} aria-hidden="true" />
    </button>
  </div>

  <div class="body">
{#if !editing || !drive.info}
  <div class="placeholder">loading settings…</div>
{:else}
  <div class="settings">
    <!-- `fullstack-a-45` (Task B) removed the Terminal section
         (scrollback MB + default TERM) — its UI lives in the
         Hybrid Terminal back now (`HybridTerminalConfig.svelte`).
         Terminal preferences still round-trip through the same
         GlobalConfig PATCH; only the mounting point changed.

         `fullstack-a-46` (Task C) removed the Editor sections
         (Editor theme, Appearance, Layout, Date pills, On save)
         for the same reason — they live in
         `HybridEditorConfig.svelte` now. -->


    <!-- `fullstack-a-21`: opt-in to Hybrid search (BM25 + dense
         vectors via BGE-small). The model is no longer bundled in
         the default binary (`systacean-6` cargo feature gate); the
         user downloads it on demand from this row. v1 uses a
         spinner + polling pattern rather than a per-byte progress
         bar because hf-hub doesn't expose progress callbacks
         (per @@Systacean's `systacean-7` constraint); the
         downloading endpoint is synchronous and the UI polls
         `/state` in parallel to surface the model_present
         transition. -->
    <section>
      <h3>Semantic search</h3>
      {#if buildInfo && !buildInfo.features.embeddings}
        <p class="hint">
          Semantic search isn't compiled into this binary. Rebuild
          with <code>--features embed-model</code> (or install a
          chan release that includes it) to enable Hybrid search.
        </p>
      {:else if semanticState === null}
        <p class="hint muted">Loading semantic-search state…</p>
      {:else}
        <p class="hint">
          Hybrid search blends BM25 keyword scoring with dense-vector
          similarity from
          <code>{semanticState.model_name}</code>
          ({formatModelSize(semanticState.model_size_bytes)}). The
          model file is shared across drives.
        </p>
        <label class="theme-opt semantic-toggle" class:on={semanticState.semantic_enabled}>
          <input
            type="checkbox"
            checked={semanticState.semantic_enabled}
            disabled={semanticDownloading || semanticEnabling}
            onchange={(e) =>
              void semanticToggle((e.currentTarget as HTMLInputElement).checked)}
          />
          <span>
            Enable semantic search (Hybrid mode)
          </span>
        </label>
        {#if semanticDownloading}
          <p class="hint muted">
            <span class="spinner" aria-hidden="true"></span>
            Downloading model… this may take a few minutes.
          </p>
        {:else if semanticEnabling}
          <p class="hint muted">Enabling…</p>
        {/if}
        <div class="grid semantic-info">
          <span class="k">Active</span>
          <span class="v">
            {#if semanticState.mode === "hybrid"}
              <span class="ok">Hybrid (BM25 + semantic)</span>
            {:else}
              <span class="muted">BM25</span>
            {/if}
          </span>
          <span class="k">Stored at</span>
          <span class="v mono" title="Shared across drives">{semanticState.model_path}</span>
        </div>
        {#if semanticError}
          <p class="hint err" role="alert">{semanticError}</p>
        {/if}
      {/if}
    </section>

    <section class="about">
      <h3>About</h3>
      <div class="grid">
        <span class="k">chan version</span>
        <span class="v mono">{buildInfo?.version ?? "n/a"}</span>
        <span class="k">embeddings</span>
        <span class="v">
          {#if buildInfo === null}
            n/a
          {:else if buildInfo.features.embeddings}
            <span class="ok">on</span>
            <span class="muted">(hybrid search available)</span>
          {:else}
            <span class="muted">off (BM25 only)</span>
          {/if}
        </span>
        <!-- `fullstack-b-12`: Source Code Pro attribution. Ships
             with chan under the SIL Open Font License 1.1; the OFL
             notice is at /static/fonts/OFL.txt next to the .woff2. -->
        <span class="k">terminal font</span>
        <span class="v">
          Source Code Pro Regular
          <span class="muted">
            (<a href="/static/fonts/OFL.txt" target="_blank" rel="noopener">SIL OFL 1.1</a>)
          </span>
        </span>
      </div>
    </section>

  </div>
{/if}

  </div>
</div>
</OverlayShell>

<style>
  /* Outer container: vertical stack with the top bar above and the
     body row below. Same recipe as FileBrowserTab. */
  .settings-tab {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    min-width: 0;
    background: var(--bg);
    color: var(--text);
  }
  .tab-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.5rem;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
    font-size: 14px;
    color: var(--text-secondary);
    flex-shrink: 0;
    min-height: 28px;
  }
  .tab-bar .title { flex: 1; font-weight: 600; color: var(--text); }
  /* Window-manager chrome: maximize/restore on the far left of the
     tab-bar, close on the far right. Matches the affordance used
     by every other overlay header. */
  .chrome-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 24px;
    padding: 0;
    background: var(--bg);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    transition: color 0.15s ease, border-color 0.15s ease;
    flex-shrink: 0;
  }
  .chrome-btn:hover {
    color: var(--text);
    border-color: var(--btn-hover);
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
    min-width: 0;
  }
  .settings {
    flex: 1;
    overflow-y: auto;
    padding: 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    background: var(--bg);
    color: var(--text);
  }
  .placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-style: italic;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border);
  }
  section:last-of-type { border-bottom: 0; }
  /* `fullstack-a-46` removed the two-column .section-row layout
     that paired (Editor theme + Appearance) and (Layout + Date
     pills). Both pairs migrated to `HybridEditorConfig.svelte`;
     the remaining sections (Semantic search, About) stack
     single-column. */
  h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  label {
    display: grid;
    grid-template-columns: 7em 1fr;
    align-items: center;
    gap: 0.5rem;
    font-size: 15px;
  }
  label > span { color: var(--text-secondary); }
  /* `fullstack-a-46`: `select` rule dropped — no select element
     remains in this overlay after the Date pills migration. */
  input {
    background: var(--bg-card);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 7px;
    font: inherit;
    font-size: 15px;
    outline: none;
    width: 100%;
  }
  input:focus { border-color: var(--link); }
  .grid {
    display: grid;
    grid-template-columns: 7em 1fr;
    gap: 4px 0.5rem;
    font-size: 15px;
  }
  .grid .k { color: var(--text-secondary); }
  .grid .v { color: var(--text); }
  .mono { font-family: ui-monospace, monospace; }
  .muted { color: var(--text-secondary); font-style: italic; }
  .hint {
    color: var(--text-secondary);
    font-size: 11.5px;
    margin: 0 0 0.5rem 0;
  }
  .hint code {
    font-family: ui-monospace, monospace;
    font-size: 13px;
    background: var(--bg-card);
    padding: 0 4px;
    border-radius: 3px;
  }
  /* `.theme-opt` chip shape is reused by the semantic-search
     toggle (`<label class="theme-opt semantic-toggle">` at line
     415); the Editor / Layout / Date pills usages migrated to
     `HybridEditorConfig.svelte` with `-a-46`. The undo of the
     generic `label { display: grid }` and `input { width: 100% }`
     rules stays here because the semantic toggle still depends
     on it. */
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
  /* `fullstack-a-46`: `.theme-opt input[type="radio"]` rule
     dropped — semantic-toggle is the only remaining `.theme-opt`
     consumer here, and it's a checkbox. The checkbox-specific
     reset is just below in `.semantic-toggle input[type="checkbox"]`. */
  .theme-opt > span { color: var(--text); }
  .theme-opt:hover { border-color: var(--btn-hover); }
  .theme-opt.on {
    border-color: var(--link);
    background: var(--hover-bg);
  }
  .v .ok { color: var(--accent); }
  .hint.err { color: #d33; }
  /* `fullstack-a-21`: re-use the theme-opt chip shape for the
     semantic-search toggle so it visually matches the rest of
     the Settings chips. The checkbox is a distinct input shape
     (vs the radios theme-opt was built for), so a few resets
     undo the generic `input { width: 100% }` rule above. */
  .semantic-toggle {
    margin-bottom: 0.5rem;
  }
  .semantic-toggle input[type="checkbox"] {
    width: auto;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
  }
  .semantic-toggle input[type="checkbox"]:disabled,
  .semantic-toggle:has(input[type="checkbox"]:disabled) {
    cursor: not-allowed;
    opacity: 0.7;
  }
  .semantic-info {
    margin-top: 0.5rem;
    font-size: 13px;
  }
  .spinner {
    display: inline-block;
    width: 0.85em;
    height: 0.85em;
    margin-right: 0.25em;
    vertical-align: -0.1em;
    border: 2px solid var(--border);
    border-top-color: var(--link);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  @media (prefers-reduced-motion: reduce) {
    .spinner { animation: none; border-top-color: var(--border); }
  }
  /* Tab-bar autosave indicator. Sits between the title and the
     actions strip. Empty when idle (no extra padding). */
  .save-status { font-size: 14px; min-width: 60px; text-align: right; }
  .save-status .ok { color: var(--accent); }
  .save-status .err { color: #d33; }
  .save-status .muted { color: var(--text-secondary); }
</style>
