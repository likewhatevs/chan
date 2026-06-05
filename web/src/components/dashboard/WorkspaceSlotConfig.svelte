<script lang="ts">
  // Workspace-slot body for the redesigned Dashboard flip-back. Renders
  // section content only: the outer shell (title band, theme toggle, OK
  // button, slot picker) is owned by DashboardSlotBack. Sections:
  // chan-reports / Metadata archive.
  //
  // chan-reports is ported from the former HybridFileBrowserConfig back
  // (per-workspace reports endpoints; IndexConfig.reports_enabled is the
  // source of truth). Metadata archive is ported from the former
  // HybridDashboardConfig back (export/import the per-workspace metadata
  // tarball). Both keep their original immediate-write behavior.

  import { onDestroy, onMount } from "svelte";
  import { Download, Upload } from "lucide-svelte";
  import { api } from "../../api/client";
  import { formatSize } from "../../state/format";

  // No external props. Reports + metadata state are owned here and read
  // straight from `api`, matching the originals.

  // chan-reports toggle. Reports writes are immediate per-workspace
  // endpoint calls; reportsState mirrors IndexConfig.reports_enabled.
  let reportsState = $state<{ enabled: boolean } | null>(null);
  let reportsBusy = $state(false);
  let reportsError = $state<string | null>(null);

  const reportsEnabled = $derived(reportsState?.enabled ?? false);

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
    try {
      reportsState = next ? await api.reportsEnable() : await api.reportsDisable();
    } catch (err) {
      const message = (err as Error).message;
      reportsError = message;
      try {
        reportsState = await api.reportsState();
      } catch {
        // Keep the original write error visible.
      }
    } finally {
      reportsBusy = false;
    }
  }

  // Metadata archive export/import. Import replaces the per-workspace
  // metadata (index, graph, report, sessions) and reloads.
  let metadataBusy = $state(false);
  let metadataStatus = $state<string | null>(null);
  let metadataError = $state<string | null>(null);
  let importInput: HTMLInputElement | undefined = $state();
  let metadataImportFile = $state<File | null>(null);
  let metadataImportBusy = $state(false);
  let metadataImportRescan = $state(true);
  let metadataImportForceScm = $state(false);

  async function exportMetadataArchive(): Promise<void> {
    if (metadataBusy) return;
    metadataBusy = true;
    metadataStatus = null;
    metadataError = null;
    try {
      const download = await api.metadataExport();
      const href = URL.createObjectURL(download.blob);
      const a = document.createElement("a");
      a.href = href;
      a.download = download.filename;
      a.rel = "noopener";
      document.body.appendChild(a);
      a.click();
      a.remove();
      window.setTimeout(() => URL.revokeObjectURL(href), 0);

      const details: string[] = [];
      if (download.files !== null) {
        details.push(`${download.files} ${download.files === 1 ? "file" : "files"}`);
      }
      if (download.bytes !== null) {
        details.push(formatSize(download.bytes));
      }
      metadataStatus =
        details.length > 0 ? `Exported ${details.join(", ")}` : "Archive exported";
    } catch (e) {
      metadataError = e instanceof Error ? e.message : String(e);
    } finally {
      metadataBusy = false;
    }
  }

  function chooseMetadataImportFile(): void {
    metadataError = null;
    importInput?.click();
  }

  function onMetadataImportFileChange(e: Event): void {
    const input = e.currentTarget as HTMLInputElement;
    metadataImportFile = input.files?.[0] ?? null;
    metadataStatus = null;
    metadataError = null;
  }

  function clearMetadataImport(): void {
    metadataImportFile = null;
    metadataImportForceScm = false;
    metadataImportRescan = true;
    if (importInput) importInput.value = "";
  }

  async function importMetadataArchive(): Promise<void> {
    if (!metadataImportFile || metadataImportBusy) return;
    metadataImportBusy = true;
    metadataStatus = null;
    metadataError = null;
    try {
      const report = await api.metadataImport(metadataImportFile, {
        rescan: metadataImportRescan,
        forceScm: metadataImportForceScm,
      });
      const details: string[] = [`${report.files} ${report.files === 1 ? "file" : "files"}`];
      details.push(formatSize(report.bytes));
      if (report.rescanned) details.push("rescanned");
      metadataStatus = `Imported ${details.join(", ")}; reloading...`;
      clearMetadataImport();
      window.setTimeout(() => window.location.reload(), 700);
    } catch (e) {
      metadataError = e instanceof Error ? e.message : String(e);
    } finally {
      metadataImportBusy = false;
    }
  }

  onMount(() => {
    void loadReportsState();
  });

  onDestroy(() => {
    // No long-lived timers here: reports + metadata are request/response.
    // Pending reload timeouts are harmless if the slot is switched away
    // mid-flight, so nothing to clear.
  });
</script>

<!-- chan-reports: per-workspace reports endpoints. -->
<section>
  <h3>chan-reports</h3>
  <p class="hint">
    Per-file SLOC + language rollups (powered by
    <code>chan-report</code>). Aggregated stats surface in the file
    inspector + the graph directory inspector.
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

<section>
  <h3>Metadata archive</h3>
  <div class="metadata-row">
    <button
      type="button"
      class="metadata-action"
      onclick={exportMetadataArchive}
      disabled={metadataBusy || metadataImportBusy}
    >
      <Download size={16} strokeWidth={1.75} aria-hidden="true" />
      <span>{metadataBusy ? "Exporting..." : "Export metadata archive"}</span>
    </button>
    <input
      bind:this={importInput}
      class="metadata-file-input"
      type="file"
      accept=".tar.zst,application/zstd"
      onchange={onMetadataImportFileChange}
    />
    <button
      type="button"
      class="metadata-action"
      onclick={chooseMetadataImportFile}
      disabled={metadataBusy || metadataImportBusy}
    >
      <Upload size={16} strokeWidth={1.75} aria-hidden="true" />
      <span>Import metadata archive</span>
    </button>
  </div>
  {#if metadataImportFile}
    <div class="metadata-import-panel">
      <div class="metadata-import-file">{metadataImportFile.name}</div>
      <p class="metadata-warning">
        Import replaces index, graph, report, and sessions metadata.
      </p>
      <label class="metadata-check">
        <input
          type="checkbox"
          bind:checked={metadataImportRescan}
          disabled={metadataImportBusy}
        />
        <span>Rescan after import</span>
      </label>
      <label class="metadata-check">
        <input
          type="checkbox"
          bind:checked={metadataImportForceScm}
          disabled={metadataImportBusy}
        />
        <span>Force SCM mismatch</span>
      </label>
      <div class="metadata-import-actions">
        <button
          type="button"
          class="metadata-action"
          onclick={importMetadataArchive}
          disabled={metadataImportBusy}
        >
          <Upload size={16} strokeWidth={1.75} aria-hidden="true" />
          <span>{metadataImportBusy ? "Importing..." : "Import"}</span>
        </button>
        <button
          type="button"
          class="metadata-action subtle"
          onclick={clearMetadataImport}
          disabled={metadataImportBusy}
        >
          Cancel
        </button>
      </div>
    </div>
  {/if}
  {#if metadataStatus}
    <p class="metadata-status ok">{metadataStatus}</p>
  {/if}
  {#if metadataError}
    <p class="metadata-status error">{metadataError}</p>
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
     reports toggle matches the rest of the back chrome. */
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
  .metadata-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .metadata-action {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-height: 30px;
    padding: 5px 10px;
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    background: var(--btn-bg);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  .metadata-action:hover:not(:disabled) {
    border-color: var(--btn-hover);
  }
  .metadata-action:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .metadata-action.subtle {
    background: transparent;
  }
  .metadata-file-input {
    display: none;
  }
  .metadata-import-panel {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 560px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--panel-bg, transparent);
  }
  .metadata-import-file {
    font-size: 13px;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .metadata-warning {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .metadata-check {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    width: fit-content;
    font-size: 13px;
    color: var(--text);
  }
  .metadata-check input[type="checkbox"] {
    width: auto;
    margin: 0;
  }
  .metadata-import-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .metadata-status {
    margin: 0;
    font-size: 12px;
  }
  .metadata-status.ok {
    color: var(--muted);
  }
  .metadata-status.error {
    color: var(--danger, #b42318);
  }
</style>
