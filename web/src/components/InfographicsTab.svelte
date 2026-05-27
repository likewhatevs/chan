<script lang="ts">
  // `fullstack-a-75b`: Infographics tab body. Per @@Alex's
  // `d4a3fc8` route on the slice-1 walk, the rotating carousel
  // moves OUT of the welcome surface (which becomes a static
  // spawn grid via EmptyPaneWelcome.svelte) and lives only
  // INSIDE this tab. The full carousel widget (rotation +
  // play/pause + pagination + 3 slides: Shortcuts / Workspace
  // metadata / Indexing graph) renders here.
  //
  // Earlier slice (-a-75 slice 1) shipped this tab as a static
  // ASCII shortcut table; that table is now slide 1 of the
  // carousel below.

  import { Download, Settings2, Upload } from "lucide-svelte";
  import { api } from "../api/client";
  import { formatSize } from "../state/format";
  import { surfaceThemeOverride } from "../state/store.svelte";
  import EmptyPaneCarousel from "./EmptyPaneCarousel.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";
  import HybridSurfaceConfigShell from "./HybridSurfaceConfigShell.svelte";

  let menu: HamburgerMenu | undefined = $state();
  let menuOpen = $state(false);
  let settingsOpen = $state(false);
  let metadataBusy = $state(false);
  let metadataStatus = $state<string | null>(null);
  let metadataError = $state<string | null>(null);
  let importInput: HTMLInputElement | undefined = $state();
  let metadataImportFile = $state<File | null>(null);
  let metadataImportBusy = $state(false);
  let metadataImportRescan = $state(true);
  let metadataImportForceScm = $state(false);

  function onContextMenu(e: MouseEvent): void {
    e.preventDefault();
    menu?.openAtCursor(e.clientX, e.clientY);
  }

  function openSettings(): void {
    menu?.close();
    settingsOpen = true;
  }

  function closeSettings(): void {
    settingsOpen = false;
  }

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
</script>

<div
  class="infographics"
  aria-label="Infographics"
  data-theme={surfaceThemeOverride("infographics")}
  oncontextmenu={onContextMenu}
  role="region"
>
  <HamburgerMenu
    bind:this={menu}
    bind:open={menuOpen}
    showTrigger={false}
    width={220}
    height={58}
  >
    <li>
      <button role="menuitem" onclick={openSettings}>
        <Settings2 size={16} strokeWidth={1.75} aria-hidden="true" />
        <span class="menu-row-label">Settings</span>
        <span class="menu-row-chord"></span>
      </button>
    </li>
  </HamburgerMenu>

  {#if settingsOpen}
    <HybridSurfaceConfigShell
      title="Infographics"
      surface="infographics"
      ariaLabel="Infographics settings"
      onDone={closeSettings}
    >
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
                Import replaces index, graph, report, sessions, and drafts metadata.
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
    </HybridSurfaceConfigShell>
  {:else}
    <EmptyPaneCarousel />
  {/if}
</div>

<style>
  .infographics {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--text);
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
