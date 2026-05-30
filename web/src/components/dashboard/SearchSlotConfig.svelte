<script lang="ts">
  // Search-slot body for the redesigned Dashboard flip-back. Renders
  // section content only: the outer shell (title band, theme toggle, OK
  // button, slot picker) is owned by DashboardSlotBack. Sections: Index
  // / Semantic search / Embedding model.
  //
  // Index is ported from the former SearchStatusOverlay's Index widget:
  // it shows the live indexer state (chunks/vectors/model when idle,
  // progress when building/reindexing) and offers an explicit rebuild.
  // The status poller is owned here and cleared in onDestroy, so polling
  // stops the moment the user switches slots (this body unmounts).
  //
  // Semantic search + Embedding model are ported from the former
  // HybridFileBrowserConfig back (stateful POST endpoints with a polled
  // download path; the SPA owns the spinner flags).

  import { onDestroy, onMount, untrack } from "svelte";
  import { api } from "../../api/client";
  import type {
    BuildInfo,
    SemanticModelRegistry,
    SemanticState,
  } from "../../api/types";
  import { indexStatus } from "../../state/store.svelte";

  // No external props. Index status lives in the shared `indexStatus`
  // store (mirrored from the same endpoint the search overlay used);
  // semantic state is owned locally and read straight from `api`.

  // Index status polling. `mounted` gates the poller so a late timer
  // callback after onDestroy cannot re-arm itself. In the old overlay
  // this gate was the `visible` flag; here the body only mounts when the
  // Search slot is selected, so mount lifetime is the gate.
  let mounted = $state(false);
  let indexResetting = $state(false);
  let indexResetError = $state<string | null>(null);
  let statusPollTimer: ReturnType<typeof setTimeout> | null = null;

  function fmt(n: number): string {
    return n.toLocaleString();
  }

  function stopStatusPoll(): void {
    if (statusPollTimer) {
      clearTimeout(statusPollTimer);
      statusPollTimer = null;
    }
  }

  function scheduleStatusPoll(delayMs = 1500): void {
    stopStatusPoll();
    statusPollTimer = setTimeout(() => {
      statusPollTimer = null;
      void refreshIndexStatus();
    }, delayMs);
  }

  async function refreshIndexStatus(): Promise<void> {
    if (!mounted) return;
    try {
      const s = await api.indexStatus();
      indexStatus.value = s;
      scheduleStatusPoll(s.state === "idle" ? 5000 : 1000);
    } catch {
      indexStatus.value = null;
      scheduleStatusPoll(5000);
    }
  }

  async function rebuildIndex(): Promise<void> {
    indexResetting = true;
    indexResetError = null;
    try {
      await api.indexRebuild();
      indexStatus.value = { state: "reindexing", file: "" };
      scheduleStatusPoll(250);
    } catch (e) {
      indexResetError = (e as Error).message;
    } finally {
      indexResetting = false;
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
    return parts.join(" - ");
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
    mounted = true;
    // untrack so the initial kick-off does not register reactive deps;
    // the poller re-arms itself off its own setTimeout chain.
    untrack(() => {
      void refreshIndexStatus();
      scheduleStatusPoll(0);
    });
    void loadBuildInfo();
    void refreshSemanticSearchState();
  });

  onDestroy(() => {
    mounted = false;
    stopStatusPoll();
    stopSemanticPoll();
  });
</script>

<!-- Index: live indexer state + explicit rebuild. Read-only except for
     the rebuild button. -->
<section>
  <h3>Index</h3>
  <div class="grid">
    <span class="k">state</span>
    <span class="v">{indexStatus.value?.state ?? "n/a"}</span>
    {#if indexStatus.value?.state === "idle"}
      <span class="k">chunks</span>
      <span class="v">{fmt(indexStatus.value.indexed_docs)}</span>
      <span class="k">vectors</span>
      <span class="v">{fmt(indexStatus.value.indexed_vectors)}</span>
      <span class="k">model</span>
      <span class="v mono">{indexStatus.value.model}</span>
    {:else if indexStatus.value?.state === "building"}
      <span class="k">progress</span>
      <span class="v">{fmt(indexStatus.value.current)} / {fmt(indexStatus.value.total)}</span>
      <span class="k">file</span>
      <span class="v mono path">{indexStatus.value.file}</span>
    {:else if indexStatus.value?.state === "reindexing"}
      <span class="k">file</span>
      <span class="v mono path">{indexStatus.value.file}</span>
    {:else if indexStatus.value?.state === "error"}
      <span class="k">error</span>
      <span class="v err">{indexStatus.value.message}</span>
    {/if}
  </div>
  <button class="action" onclick={() => void rebuildIndex()} disabled={indexResetting}>
    {indexResetting ? "Rebuilding..." : "Rebuild index"}
  </button>
  {#if indexResetError}
    <div class="err-line">{indexResetError}</div>
  {/if}
</section>

<!-- Semantic-search opt-in. -->
<section>
  <h3>Semantic search</h3>
  {#if buildInfo && !buildInfo.features.embeddings}
    <p class="hint">
      Semantic search isn't compiled into this binary. Rebuild with
      <code>--features embed-model</code> (or install a chan release that
      includes it) to enable Hybrid search.
    </p>
  {:else if semanticState === null}
    <p class="hint muted">Loading semantic-search state...</p>
  {:else}
    <p class="hint">
      Hybrid search blends BM25 keyword scoring with dense-vector
      similarity from
      <code>{semanticState.model_name}</code>
      ({formatModelSize(semanticState.model_size_bytes)}). The model file
      is shared across workspaces.
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
        Downloading model... this may take a few minutes.
      </p>
    {:else if semanticEnabling}
      <p class="hint muted">Enabling...</p>
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
    indexing. Changing it persists immediately; enabling Hybrid search
    downloads the selected model first when needed.
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
            {model.label} - {formatModelMeta(model)}
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

<style>
  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .hint.muted { color: var(--text-secondary); font-style: italic; }
  .hint.err { color: #d33; }
  .hint.sub-hint { font-size: 11.5px; margin: 0; }
  /* `.theme-opt` chip + `.strip-toggle` checkbox affordance so the
     semantic toggle matches the rest of the back chrome. */
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
  /* Shared key/value grid. Used by both the Index widget and the
     semantic-search Active/Stored-at rows. */
  .grid {
    display: grid;
    grid-template-columns: 8em minmax(0, 1fr);
    gap: 4px 10px;
    font-size: 14px;
  }
  .grid .k { color: var(--text-secondary); }
  .grid .v { color: var(--text); min-width: 0; }
  .v .ok { color: var(--accent); }
  .mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
  .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .muted { color: var(--text-secondary); font-style: italic; }
  .err, .err-line { color: var(--warn-text); }
  .err-line { margin-top: 8px; font-size: 13px; }
  .action {
    margin-top: 12px;
    padding: 5px 10px;
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    background: var(--btn-bg);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    align-self: flex-start;
  }
  .action:hover:not(:disabled) { border-color: var(--btn-hover); }
  .action:disabled { opacity: 0.55; cursor: default; }
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
