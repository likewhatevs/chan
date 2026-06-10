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
  import type { GlobalConfig } from "../../api/types";
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

  // Default workspace + recents config. Moved here from the front
  // Workspace slide (WorkspaceInfoBody); owns its own globalConfig load +
  // autosave, matching the reports/metadata sections above. The default
  // root writes through the global config endpoint, debounced.
  let globalConfig = $state<GlobalConfig | null>(null);
  let editedDefaultRoot = $state<string>("");
  let initialDefaultRoot = $state<string>("");
  let saveError = $state<string | null>(null);

  const AUTOSAVE_DELAY_MS = 500;
  let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  let inflight = false;

  async function loadGlobalConfig(): Promise<void> {
    try {
      globalConfig = await api.config();
      const cur = globalConfig.default_workspace_root ?? "";
      editedDefaultRoot = cur;
      initialDefaultRoot = cur;
    } catch {
      globalConfig = null;
    }
  }

  function dirty(): boolean {
    if (!globalConfig) return false;
    return editedDefaultRoot !== initialDefaultRoot;
  }

  async function saveDefaultRoot(): Promise<void> {
    if (!globalConfig || inflight) return;
    inflight = true;
    saveError = null;
    const sent = editedDefaultRoot;
    try {
      const trimmed = sent.trim();
      const body: GlobalConfig = {
        preferences: globalConfig.preferences,
        default_workspace_root: trimmed === "" ? null : trimmed,
        workspaces: globalConfig.workspaces,
      };
      const cfg = await api.updateConfig(body);
      globalConfig = cfg;
      // Don't clobber further edits the user typed while in flight.
      if (editedDefaultRoot === sent) {
        const echoed = cfg.default_workspace_root ?? "";
        editedDefaultRoot = echoed;
        initialDefaultRoot = echoed;
      } else {
        initialDefaultRoot = cfg.default_workspace_root ?? "";
      }
    } catch (e) {
      saveError = (e as Error).message;
    } finally {
      inflight = false;
      if (dirty()) scheduleDefaultRootSave();
    }
  }

  function scheduleDefaultRootSave(): void {
    if (autosaveTimer) clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => {
      autosaveTimer = null;
      void saveDefaultRoot();
    }, AUTOSAVE_DELAY_MS);
  }

  $effect(() => {
    void editedDefaultRoot;
    if (!dirty()) return;
    scheduleDefaultRootSave();
  });

  function displayPathLabel(path: string): string {
    const stripped = path.replace(/[/\\]+$/, "");
    if (!stripped) return path || "(root)";
    const slash = Math.max(stripped.lastIndexOf("/"), stripped.lastIndexOf("\\"));
    return slash < 0 ? stripped : stripped.slice(slash + 1);
  }

  function formatLastSeen(iso: string): string {
    try {
      const d = new Date(iso);
      const yyyy = d.getUTCFullYear();
      const mm = String(d.getUTCMonth() + 1).padStart(2, "0");
      const dd = String(d.getUTCDate()).padStart(2, "0");
      const hh = String(d.getUTCHours()).padStart(2, "0");
      const mi = String(d.getUTCMinutes()).padStart(2, "0");
      return `${yyyy}-${mm}-${dd} ${hh}:${mi} UTC`;
    } catch {
      return iso;
    }
  }

  onMount(() => {
    void loadReportsState();
    void loadGlobalConfig();
  });

  onDestroy(() => {
    // Reports + metadata are request/response; pending metadata reload
    // timeouts are harmless if the slot is switched away mid-flight.
    // Clear a pending default-root autosave so it can't fire post-unmount.
    if (autosaveTimer) clearTimeout(autosaveTimer);
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

<section class="divided">
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

<!-- Default workspace + recents. Moved here from the front Workspace
     slide (WorkspaceInfoBody): the global default-root config + the
     recent-workspaces list now live on this slot's flip-back, below a
     dashed divider matching the one between chan-reports and Metadata
     archive above. -->
<section class="divided">
  <h3>Workspaces</h3>
  <p class="hint">
    Your default workspace directory is where chan opens when launched
    without a specific one in mind. Leave empty to use the platform
    default (<code>~/Documents/Chan</code> on macOS,
    <code>$XDG_DATA_HOME/chan/default</code> on Linux).
  </p>
  <label class="field">
    <span>Default</span>
    <input
      bind:value={editedDefaultRoot}
      placeholder="(platform default)"
      spellcheck="false"
      autocomplete="off"
    />
  </label>
  {#if saveError}
    <div class="err-line">save failed: {saveError}</div>
  {/if}

  {#if globalConfig?.workspaces && globalConfig.workspaces.length > 0}
    <h5 class="recents-head">Recent</h5>
    <ul class="recents">
      {#each globalConfig.workspaces as u (u.path)}
        <li>
          <span class="recents-time">{formatLastSeen(u.last_seen_at)}</span>
          <span class="recents-name" title={u.path}>{displayPathLabel(u.path)}</span>
          <span class="recents-path mono" title={u.path}>{u.path}</span>
        </li>
      {/each}
    </ul>
    <p class="hint">
      Updated every time you open a directory. In-app open-from-list
      lands in a follow-up; for now use the menu's Open Directory.
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
  /* Dashed section divider, matching the workspace inspector idiom: a
     rule between chan-reports / Metadata archive / Workspaces. The
     shell's 1.25rem inter-section gap supplies the space above; the
     padding spaces the heading below the rule. */
  .divided {
    padding-top: 0.7rem;
    border-top: 1px dashed var(--border);
  }
  .hint code {
    font-family: ui-monospace, monospace;
    font-size: 12px;
    background: var(--bg-card);
    padding: 0 4px;
    border-radius: 3px;
  }
  .field {
    display: grid;
    grid-template-columns: 6.5em 1fr;
    gap: 0.5rem;
    align-items: center;
    margin: 0.25rem 0;
  }
  .field > span { color: var(--text-secondary); font-size: 14px; }
  .field input {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 4px 7px;
    font: inherit;
    font-size: 14px;
    outline: none;
    width: 100%;
  }
  .field input:focus { border-color: var(--link); }
  .err-line {
    color: var(--warn-text);
    font-size: 13px;
    margin: 0.25rem 0;
  }
  .recents-head {
    margin: 0.6rem 0 0.25rem 0;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
  }
  .recents {
    list-style: none;
    padding: 0;
    margin: 0 0 0.4rem 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .recents li {
    display: grid;
    grid-template-columns: 12em auto 1fr;
    gap: 0.6rem;
    font-size: 13px;
    color: var(--text);
    align-items: baseline;
  }
  .recents-time {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .recents-name { color: var(--text); font-weight: 500; }
  .recents-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
</style>
