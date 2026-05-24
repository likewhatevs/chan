<script lang="ts">
  // `fullstack-a-75b`: Infographics tab body. Per @@Alex's
  // `d4a3fc8` route on the slice-1 walk, the rotating carousel
  // moves OUT of the welcome surface (which becomes a static
  // spawn grid via EmptyPaneWelcome.svelte) and lives only
  // INSIDE this tab. The full carousel widget (rotation +
  // play/pause + pagination + 3 slides: Shortcuts / Drive
  // metadata / Indexing graph) renders here.
  //
  // Earlier slice (-a-75 slice 1) shipped this tab as a static
  // ASCII shortcut table; that table is now slide 1 of the
  // carousel below.

  import { Download, Settings2 } from "lucide-svelte";
  import { api } from "../api/client";
  import { formatSize } from "../state/format";
  import EmptyPaneCarousel from "./EmptyPaneCarousel.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";

  type InfographicsAppearance = "inherit" | "light" | "dark";

  let menu: HamburgerMenu | undefined = $state();
  let menuOpen = $state(false);
  let settingsOpen = $state(false);
  let appearance = $state<InfographicsAppearance>("inherit");
  let metadataBusy = $state(false);
  let metadataStatus = $state<string | null>(null);
  let metadataError = $state<string | null>(null);

  const effectiveTheme = $derived(
    appearance === "inherit" ? undefined : appearance,
  );

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
</script>

<div
  class="infographics"
  aria-label="Infographics"
  data-theme={effectiveTheme}
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
    <section class="infographics-settings" aria-label="Infographics settings">
      <header class="config-header">
        <h2 class="config-title">Infographics</h2>
        <button type="button" class="config-ok" onclick={closeSettings}>OK</button>
      </header>
      <div class="config-body">
        <section>
          <h3>Appearance</h3>
          <div class="theme-row" role="radiogroup" aria-label="Infographics appearance">
            {#each [
              { value: "inherit" as const, label: "Inherit" },
              { value: "light" as const, label: "Light" },
              { value: "dark" as const, label: "Dark" },
            ] as opt (opt.value)}
              <label class="theme-opt" class:on={appearance === opt.value}>
                <input
                  type="radio"
                  name="infographics-appearance"
                  value={opt.value}
                  checked={appearance === opt.value}
                  onchange={() => {
                    appearance = opt.value;
                  }}
                />
                <span>{opt.label}</span>
              </label>
            {/each}
          </div>
        </section>
        <section>
          <h3>Metadata archive</h3>
          <div class="metadata-row">
            <button
              type="button"
              class="metadata-action"
              onclick={exportMetadataArchive}
              disabled={metadataBusy}
            >
              <Download size={16} strokeWidth={1.75} aria-hidden="true" />
              <span>{metadataBusy ? "Exporting..." : "Export metadata archive"}</span>
            </button>
          </div>
          {#if metadataStatus}
            <p class="metadata-status ok">{metadataStatus}</p>
          {/if}
          {#if metadataError}
            <p class="metadata-status error">{metadataError}</p>
          {/if}
        </section>
      </div>
    </section>
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
  .infographics-settings {
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
  .theme-row {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
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
  .theme-opt input[type="radio"] {
    width: auto;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
  }
  .theme-opt > span {
    color: var(--text);
  }
  .theme-opt:hover {
    border-color: var(--btn-hover);
  }
  .theme-opt.on {
    border-color: var(--link);
    background: var(--hover-bg);
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
