<script lang="ts">
  // `fullstack-a-48` Task F (option B): Search / Indexing /
  // Reports settings migrated from `SettingsPanel.svelte`'s
  // Semantic search section into the Hybrid FB back-side mount
  // point introduced by `-a-43` Task A. Three toggles in v1:
  //
  // 1. Semantic search (moved verbatim from -a-21; same state
  //    machine, same polling cadence, same enable/download/
  //    disable flow).
  // 2. Multi-model picker placeholder (Round-3 Track 2; future
  //    slot, disabled until backend ships a model registry).
  // 3. chan-reports toggle (G1 regression fix — toggle was
  //    specced in round-2-plan §"Pre-flight feature toggles"
  //    but never landed in v1; option B routed by @@Architect
  //    lands the SPA wiring + default ON behaviourally matching
  //    today's unconditional chan-report. Backend gating + the
  //    destructive-on-disable modal + the default-flip to OFF
  //    are a follow-up task).
  //
  // Save shape mirrors `-a-45` / `-a-46`: self-contained editing
  // buffer, merge-against-current-server PATCH (re-fetches
  // GlobalConfig before overlaying just the reports subtree),
  // dirty comparator scoped to the reports field so a parallel
  // SettingsPanel autosave (residual fields) doesn't trigger
  // spurious PATCHes here, and vice versa.

  import { onDestroy, onMount } from "svelte";
  import { api } from "../api/client";
  import type {
    BuildInfo,
    GlobalConfig,
    Preferences,
    SemanticState,
  } from "../api/types";
  import { drive } from "../state/store.svelte";

  type SaveStatus = "idle" | "saving" | "saved" | { error: string };

  /// Local edit buffer for the reports slice. The semantic-search
  /// state machine owns its own (semanticState etc.) — those
  /// endpoints are stateful POSTs against the chan-server, not
  /// preferences.
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

  /// Normalize the reports subtree. Pre-`-a-48` servers don't ship
  /// the `reports` field; backfill with `{ enabled: true }` to
  /// match the option (B) default and keep dirty()-stable across
  /// a server re-fetch.
  function normalizeReports(p: Preferences): Preferences {
    if (!p.reports) p.reports = { enabled: true };
    return p;
  }

  function reportsSnapshot(): string {
    return JSON.stringify(editing?.reports ?? null);
  }

  $effect(() => {
    const info = drive.info;
    if (!info) return;
    if (editing && reportsSnapshot() !== lastSentSnapshot) {
      if (lastSentSnapshot === null) return;
    }
    editing = normalizeReports(clone(info.preferences));
  });

  const reportsEnabled = $derived(editing?.reports?.enabled ?? true);

  function setReportsEnabled(next: boolean): void {
    if (!editing) return;
    editing.reports = { enabled: next };
  }

  function reportsDirty(): boolean {
    if (!editing || !drive.info) return false;
    const server = drive.info.preferences.reports ?? { enabled: true };
    return (editing.reports?.enabled ?? true) !== server.enabled;
  }

  function scheduleSave(): void {
    if (autosaveTimer) clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => {
      autosaveTimer = null;
      void save();
    }, AUTOSAVE_DELAY_MS);
  }

  /// Save the reports slice via the merge-against-current-server
  /// pattern from `-a-45`. Re-fetches the latest GlobalConfig
  /// before PATCHing so a parallel SettingsPanel autosave (residual
  /// fields after `-a-46` trim) can't be clobbered.
  async function save(): Promise<void> {
    if (!editing || inflight) return;
    if (!reportsDirty()) return;
    inflight = true;
    saveStatus = "saving";
    if (savedFlashTimer) {
      clearTimeout(savedFlashTimer);
      savedFlashTimer = null;
    }
    const sent = reportsSnapshot();
    lastSentSnapshot = sent;
    try {
      const current = await api.config();
      const cfgBody: GlobalConfig = {
        preferences: {
          ...current.preferences,
          reports: editing.reports,
        },
        default_drive_root: current.default_drive_root,
        drives: current.drives,
      };
      await api.updateConfig(cfgBody);
      const info = await api.drive();
      drive.info = info;
      editing = normalizeReports(clone(info.preferences));
      lastSentSnapshot = reportsSnapshot();
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
      if (reportsDirty() && reportsSnapshot() !== failedSaveSnap) {
        scheduleSave();
      }
    }
  }

  $effect(() => {
    if (!editing) return;
    const snap = reportsSnapshot();
    if (!reportsDirty()) return;
    if (snap === failedSaveSnap) return;
    scheduleSave();
  });

  // Semantic search state — same shape as the SettingsPanel
  // `-a-21` original. Endpoints are stateful POSTs on the server;
  // the SPA owns the downloading + enabling spinners (we don't
  // round-trip those flags through preferences).
  let buildInfo = $state<BuildInfo | null>(null);
  let semanticState = $state<SemanticState | null>(null);
  let semanticDownloading = $state(false);
  let semanticEnabling = $state(false);
  let semanticError = $state<string | null>(null);
  let semanticPollTimer: ReturnType<typeof setInterval> | null = null;
  const SEMANTIC_POLL_INTERVAL_MS = 3000;

  async function loadBuildInfo(): Promise<void> {
    try {
      buildInfo = await api.buildInfo();
    } catch {
      buildInfo = null;
    }
  }

  async function loadSemanticState(): Promise<void> {
    try {
      semanticState = await api.semanticState();
    } catch {
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
      semanticDownloading = true;
      stopSemanticPoll();
      semanticPollTimer = setInterval(() => {
        void loadSemanticState();
      }, SEMANTIC_POLL_INTERVAL_MS);
      try {
        semanticState = await api.semanticDownload();
        stopSemanticPoll();
        semanticEnabling = true;
        try {
          semanticState = await api.semanticEnable();
        } finally {
          semanticEnabling = false;
        }
      } catch (err) {
        stopSemanticPoll();
        semanticError = (err as Error).message;
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
    void loadBuildInfo();
    void loadSemanticState();
  });

  onDestroy(() => {
    stopSemanticPoll();
  });
</script>

<section class="hybrid-config" aria-label="Hybrid File Browser configuration">
  <header class="config-header">
    <h2 class="config-title">Hybrid File Browser</h2>
    <div class="save-status" aria-live="polite">
      {#if saveStatus === "saving"}
        <span class="muted">saving…</span>
      {:else if saveStatus === "saved"}
        <span class="ok">saved</span>
      {:else if typeof saveStatus === "object"}
        <span class="err" title={saveStatus.error}>save failed</span>
      {/if}
    </div>
  </header>
  <div class="config-body">
    <p class="hint warning">
      These settings apply to ALL file-browser surfaces on this
      drive, not just this one.
    </p>

    <!-- `fullstack-a-21` semantic-search opt-in (migrated from
         SettingsPanel by `-a-48`). -->
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
        <label class="theme-opt strip-toggle" class:on={semanticState.semantic_enabled}>
          <input
            type="checkbox"
            checked={semanticState.semantic_enabled}
            disabled={semanticDownloading || semanticEnabling}
            onchange={(e) =>
              void semanticToggle((e.currentTarget as HTMLInputElement).checked)}
          />
          <span>Enable semantic search (Hybrid mode)</span>
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

    <!-- Round-3 Track 2: multi-model picker. Placeholder slot
         (disabled until the backend ships a model registry). The
         element is present here so the user can see the future
         capability + so the Round-3 task lands as a strict
         addition rather than a structural change. -->
    <section>
      <h3>Embedding model</h3>
      <p class="hint">
        Pick which embedding model the indexer uses for dense
        vectors. A curated list ships in Round-3; the default is
        <code>BAAI/bge-small-en-v1.5</code>.
      </p>
      <label class="font-row">
        <span>Model</span>
        <select class="family" disabled aria-label="Embedding model picker (placeholder)">
          <option>BAAI/bge-small-en-v1.5 (default)</option>
        </select>
      </label>
      <p class="hint muted sub-hint">
        Picker placeholder; lands with the Round-3 multi-model
        registry.
      </p>
    </section>

    <!-- `fullstack-a-48` chan-reports toggle (option B landing —
         user-visible toggle + Preferences wire-up; backend gating
         + default flip to OFF + destructive-on-disable modal
         deferred to a follow-up task). -->
    <section>
      <h3>chan-reports</h3>
      <p class="hint">
        Per-file SLOC + language rollups (powered by
        <code>chan-report</code>). Aggregated stats surface in the
        file inspector + the graph directory inspector.
      </p>
      {#if editing}
      <label class="theme-opt strip-toggle" class:on={reportsEnabled}>
        <input
          type="checkbox"
          checked={reportsEnabled}
          onchange={(e) =>
            setReportsEnabled((e.currentTarget as HTMLInputElement).checked)}
        />
        <span>Enable chan-reports indexing</span>
      </label>
      <p class="hint muted sub-hint">
        Toggle persists via <code>/api/config</code>; backend
        gating + the destructive-on-disable confirmation modal
        land in a follow-up task. Default is ON to match today's
        unconditional behaviour.
      </p>
      {/if}
    </section>
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
  .hint.muted { color: var(--text-secondary); font-style: italic; }
  .hint.err { color: #d33; }
  .hint.sub-hint { font-size: 11.5px; margin: 0; }
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
  /* `.theme-opt` chip + `.strip-toggle` checkbox affordance
     carried over from `-a-45` / `-a-46` so the toggles in this
     back-side surface match the rest of the Hybrid back chrome. */
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
  .theme-opt input[type="checkbox"] {
    width: auto;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
  }
  .theme-opt > span { color: var(--text); }
  .theme-opt:hover { border-color: var(--btn-hover); }
  .theme-opt.on { border-color: var(--link); background: var(--hover-bg); }
  .strip-toggle input[type="checkbox"]:disabled,
  .strip-toggle:has(input[type="checkbox"]:disabled) {
    cursor: not-allowed;
    opacity: 0.7;
  }
  /* Picker placeholder. Same shape as `-a-46`'s Date pills layout. */
  .font-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  .font-row > span { color: var(--text-secondary); font-size: 14px; min-width: 5em; }
  .font-row select.family { flex: 1; min-width: 12em; }
  /* Semantic-search info grid carried over verbatim from
     SettingsPanel so the Active / Stored-at rows render with the
     same affordances. */
  .grid {
    display: grid;
    grid-template-columns: 7em 1fr;
    gap: 4px 0.5rem;
    font-size: 14px;
  }
  .grid .k { color: var(--text-secondary); }
  .grid .v { color: var(--text); }
  .v .ok { color: var(--accent); }
  .mono { font-family: ui-monospace, monospace; }
  .muted { color: var(--text-secondary); font-style: italic; }
  /* Spinner during semantic-model download. Local copy keeps this
     surface independent of SettingsPanel CSS. */
  .spinner {
    display: inline-block;
    width: 12px;
    height: 12px;
    margin-right: 4px;
    border: 2px solid var(--border);
    border-top-color: var(--link);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
    vertical-align: -2px;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) {
    .spinner { animation: none; border-top-color: var(--border); }
  }
</style>
