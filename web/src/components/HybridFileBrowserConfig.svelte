<script lang="ts">
  // Search / Indexing / Reports settings on the Hybrid FB back-side
  // mount point. Three toggles:
  //
  // 1. Semantic search (enable/download/disable flow with a polled
  //    state machine).
  // 2. Workspace-wide multi-model picker backed by the semantic model
  //    registry endpoints.
  // 3. chan-reports toggle backed by the per-workspace reports
  //    endpoints. IndexConfig.reports_enabled is the source of truth.
  //
  // Reports writes are immediate per-workspace endpoint calls; semantic
  // search and model selection keep their endpoint-owned state
  // machines.

  import { onDestroy, onMount } from "svelte";
  import { api } from "../api/client";
  import type {
    BuildInfo,
    SemanticModelRegistry,
    SemanticState,
  } from "../api/types";
  import HybridSurfaceConfigShell from "./HybridSurfaceConfigShell.svelte";

  let { onDone }: { onDone?: () => void } = $props();

  type SaveStatus = "idle" | "saving" | "saved" | { error: string };

  let saveStatus = $state<SaveStatus>("idle");

  const SAVED_FLASH_MS = 1500;
  let savedFlashTimer: ReturnType<typeof setTimeout> | null = null;
  let reportsState = $state<{ enabled: boolean } | null>(null);
  let reportsBusy = $state(false);
  let reportsError = $state<string | null>(null);

  const reportsEnabled = $derived(reportsState?.enabled ?? false);

  function markSaveComplete(): void {
    if (savedFlashTimer) {
      clearTimeout(savedFlashTimer);
      savedFlashTimer = null;
    }
    saveStatus = "saved";
    savedFlashTimer = setTimeout(() => {
      if (saveStatus === "saved") saveStatus = "idle";
      savedFlashTimer = null;
    }, SAVED_FLASH_MS);
  }

  async function loadReportsState(): Promise<void> {
    try {
      reportsState = await api.reportsState();
      reportsError = null;
    } catch (err) {
      reportsError = (err as Error).message;
    }
  }

  async function setReportsEnabled(next: boolean): Promise<void> {
    if (reportsBusy) return;
    reportsBusy = true;
    reportsError = null;
    saveStatus = "saving";
    try {
      reportsState = next ? await api.reportsEnable() : await api.reportsDisable();
      markSaveComplete();
    } catch (err) {
      const message = (err as Error).message;
      reportsError = message;
      saveStatus = { error: message };
      try {
        reportsState = await api.reportsState();
      } catch {
        // Keep the original write error visible.
      }
    } finally {
      reportsBusy = false;
    }
  }

  // Semantic search state. Endpoints are stateful POSTs on the server;
  // the SPA owns the downloading + enabling spinners (those flags are
  // not round-tripped through preferences).
  let buildInfo = $state<BuildInfo | null>(null);
  let semanticState = $state<SemanticState | null>(null);
  let semanticDownloading = $state(false);
  let semanticEnabling = $state(false);
  let semanticModelBusy = $state(false);
  let semanticModels = $state<SemanticModelRegistry | null>(null);
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

  async function loadSemanticModels(): Promise<void> {
    try {
      semanticModels = await api.semanticModels();
    } catch {
      semanticModels = null;
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
      const selected = selectedModel();
      if (selected?.downloaded || semanticState.model_present) {
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
        void refreshSemanticSearchState();
      }, SEMANTIC_POLL_INTERVAL_MS);
      try {
        semanticState = await api.semanticDownload();
        await refreshSemanticSearchState();
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
        await refreshSemanticSearchState();
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

  function selectedModel(): SemanticModelRegistry["models"][number] | undefined {
    return semanticModels?.models.find((model) => model.id === semanticModels?.current_model);
  }

  function formatModelSize(bytes: number | null | undefined): string {
    if (bytes == null || !Number.isFinite(bytes) || bytes <= 0) return "size unknown";
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(1)} MB`;
  }

  function formatModelMeta(model: SemanticModelRegistry["models"][number]): string {
    const parts: string[] = [];
    parts.push(`${model.dim}d`);
    parts.push(model.size_label);
    parts.push(model.downloaded ? "downloaded" : "not downloaded");
    return parts.join(" · ");
  }

  async function changeSemanticModel(e: Event): Promise<void> {
    const model = (e.currentTarget as HTMLSelectElement).value;
    if (!model || model === semanticModels?.current_model || semanticModelBusy) return;
    semanticModelBusy = true;
    semanticError = null;
    try {
      semanticState = await api.semanticModelPatch(model);
      await refreshSemanticSearchState();
    } catch (err) {
      semanticError = (err as Error).message;
      await loadSemanticModels();
    } finally {
      semanticModelBusy = false;
    }
  }

  async function refreshSemanticSearchState(): Promise<void> {
    await Promise.all([loadSemanticState(), loadSemanticModels()]);
  }

  onMount(() => {
    void loadBuildInfo();
    void refreshSemanticSearchState();
    void loadReportsState();
  });

  onDestroy(() => {
    stopSemanticPoll();
    if (savedFlashTimer) clearTimeout(savedFlashTimer);
  });
</script>

<HybridSurfaceConfigShell
  title="Hybrid File Browser"
  surface="browser"
  saveStatus={saveStatus}
  {onDone}
>
    <p class="hint warning">
      These settings apply to ALL file-browser surfaces on this
      workspace, not just this one. The top-bar theme switch applies
      to all Hybrid File Browser tab bodies.
    </p>

    <!-- Semantic-search opt-in. -->
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
          model file is shared across workspaces.
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
          <span class="v mono" title="Shared across workspaces">{semanticState.model_path}</span>
        </div>
        {#if semanticError}
          <p class="hint err" role="alert">{semanticError}</p>
        {/if}
      {/if}
    </section>

    <section>
      <h3>Embedding model</h3>
      <p class="hint">
        Pick the workspace-wide embedding model used for dense-vector
        indexing. Changing it persists immediately; enabling Hybrid
        search downloads the selected model first when needed.
      </p>
      <label class="font-row">
        <span>Model</span>
        <select
          class="config-select family"
          disabled={semanticModels === null || semanticModelBusy || semanticDownloading || semanticEnabling}
          value={semanticModels?.current_model ?? ""}
          onchange={changeSemanticModel}
          aria-label="Embedding model picker"
        >
          {#if semanticModels === null}
            <option value="">Loading models...</option>
          {:else}
            {#each semanticModels.models as model (model.id)}
              <option value={model.id}>
                {model.label} · {formatModelMeta(model)}
              </option>
            {/each}
          {/if}
        </select>
      </label>
      {#if semanticModels !== null && selectedModel()}
        <p class="hint muted sub-hint">
          Selected: <code>{selectedModel()!.id}</code>.
        </p>
      {/if}
    </section>

    <!-- chan-reports: per-workspace reports endpoints. -->
    <section>
      <h3>chan-reports</h3>
      <p class="hint">
        Per-file SLOC + language rollups (powered by
        <code>chan-report</code>). Aggregated stats surface in the
        file inspector + the graph directory inspector.
      </p>
      {#if reportsState === null}
        <p class="hint muted">Loading chan-reports state...</p>
      {:else}
        <label class="theme-opt strip-toggle" class:on={reportsEnabled}>
          <input
            type="checkbox"
            checked={reportsEnabled}
            disabled={reportsBusy}
            onchange={(e) =>
              void setReportsEnabled((e.currentTarget as HTMLInputElement).checked)}
          />
          <span>Enable chan-reports indexing</span>
        </label>
        <p class="hint muted sub-hint">
          Per-workspace setting. Disabling drops generated report data;
          re-enable to rebuild it.
        </p>
        {#if reportsBusy}
          <p class="hint muted">Updating...</p>
        {/if}
        {#if reportsError}
          <p class="hint err" role="alert">{reportsError}</p>
        {/if}
      {/if}
    </section>
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
  .hint.muted { color: var(--text-secondary); font-style: italic; }
  .hint.err { color: #d33; }
  .hint.sub-hint { font-size: 11.5px; margin: 0; }
  /* `.theme-opt` chip + `.strip-toggle` checkbox affordance so the
     toggles in this back-side surface match the rest of the Hybrid
     back chrome. */
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
  /* Model picker. Same layout shape as the Date pills row. */
  .font-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  .font-row > span { color: var(--text-secondary); font-size: 14px; min-width: 5em; }
  .font-row select.family { flex: 1; min-width: 12em; }
  .config-select {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 5px 7px;
    font: inherit;
  }
  /* Semantic-search info grid for the Active / Stored-at rows. */
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
     surface self-contained. */
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
