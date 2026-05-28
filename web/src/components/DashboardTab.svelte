<script lang="ts">
  // `fullstack-a-75b`: Dashboard tab body. Per @@Alex's
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
  //
  // `phase-13 lane-b` slice 3c: the back-of-card config (the
  // `flipHybrid` view) is now the only home for global
  // Appearance + Screen Lock + Screensaver controls; the global
  // Settings overlay was retired in the same slice. Cmd+, on a
  // focused pane flips that pane to its back-side; Cmd+, again
  // flips back. The four sections below render INSIDE
  // HybridSurfaceConfigShell, which provides the title bar + the
  // OK button (-> closeSettings).

  import { onMount } from "svelte";
  import { Download, Settings2, Upload } from "lucide-svelte";
  import { api } from "../api/client";
  import { formatSize } from "../state/format";
  import {
    setThemeChoice,
    surfaceThemeOverride,
    type ThemeChoice,
    ui,
  } from "../state/store.svelte";
  import {
    hashPin,
    SCREENSAVER_MAX_TIMEOUT_SECS,
    SCREENSAVER_MIN_TIMEOUT_SECS,
    type ScreensaverTheme,
  } from "../state/screensaver";
  import {
    loadScreensaverState,
    lockNow,
    screensaver,
  } from "../state/screensaver.svelte";
  import { workspace } from "../state/store.svelte";
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

  // Screen-lock state, lifted verbatim from the retired global
  // Settings overlay. The state lives on the screensaver
  // singleton (loadScreensaverState mirrors it back so the App-
  // root tracker re-arms); local mirrors below back the form
  // controls. The overlay's test-then-restore dance is gone —
  // the back-of-card flip survives the screensaver cover (no
  // need to re-open anything after unlock).
  let screensaverEnabled = $state<boolean | null>(null);
  let screensaverTimeoutSecs = $state<number>(300);
  let screensaverTheme = $state<ScreensaverTheme>("plain");
  let screensaverPinSet = $state(false);
  let screensaverBusy = $state(false);
  let screensaverError = $state<string | null>(null);
  /// PIN edit buffer. `null` when not showing the dialog;
  /// otherwise carries the pin1/pin2 confirm pair.
  let pinDialog = $state<{ pin1: string; pin2: string } | null>(null);

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

  async function loadScreenLockState(): Promise<void> {
    try {
      const s = await api.screensaverState();
      screensaverEnabled = s.enabled;
      screensaverTimeoutSecs = s.timeout_secs;
      screensaverTheme = s.theme;
      screensaverPinSet = s.pin_set;
    } catch (err) {
      screensaverError = `screensaver: ${(err as Error).message ?? err}`;
    }
  }

  async function toggleScreensaverEnabled(): Promise<void> {
    if (screensaverEnabled === null || screensaverBusy) return;
    screensaverBusy = true;
    screensaverError = null;
    try {
      const target = !screensaverEnabled;
      const s = await api.screensaverPatch({ enabled: target });
      screensaverEnabled = s.enabled;
      screensaverTimeoutSecs = s.timeout_secs;
      screensaverTheme = s.theme;
      screensaverPinSet = s.pin_set;
      await loadScreensaverState();
    } catch (err) {
      screensaverError = `toggle failed: ${(err as Error).message ?? err}`;
    } finally {
      screensaverBusy = false;
    }
  }

  async function commitTimeout(): Promise<void> {
    if (screensaverBusy) return;
    screensaverError = null;
    if (screensaverTimeoutSecs < SCREENSAVER_MIN_TIMEOUT_SECS) {
      screensaverTimeoutSecs = SCREENSAVER_MIN_TIMEOUT_SECS;
      screensaverError = `Timeout must be at least ${SCREENSAVER_MIN_TIMEOUT_SECS}s`;
    }
    if (screensaverTimeoutSecs > SCREENSAVER_MAX_TIMEOUT_SECS) {
      screensaverTimeoutSecs = SCREENSAVER_MAX_TIMEOUT_SECS;
      screensaverError = `Timeout must be at most ${SCREENSAVER_MAX_TIMEOUT_SECS}s`;
    }
    screensaverBusy = true;
    const validationMessage = screensaverError;
    try {
      const s = await api.screensaverPatch({ timeout_secs: screensaverTimeoutSecs });
      screensaverEnabled = s.enabled;
      screensaverTimeoutSecs = s.timeout_secs;
      screensaverTheme = s.theme;
      screensaverPinSet = s.pin_set;
      screensaverError = validationMessage;
      await loadScreensaverState();
    } catch (err) {
      screensaverError = `timeout save failed: ${(err as Error).message ?? err}`;
    } finally {
      screensaverBusy = false;
    }
  }

  async function commitScreensaverTheme(e: Event): Promise<void> {
    if (screensaverBusy) return;
    const theme = (e.currentTarget as HTMLSelectElement).value as ScreensaverTheme;
    screensaverBusy = true;
    screensaverError = null;
    try {
      const s = await api.screensaverPatch({ theme });
      screensaverEnabled = s.enabled;
      screensaverTimeoutSecs = s.timeout_secs;
      screensaverTheme = s.theme;
      screensaverPinSet = s.pin_set;
      await loadScreensaverState();
    } catch (err) {
      screensaverError = `theme save failed: ${(err as Error).message ?? err}`;
    } finally {
      screensaverBusy = false;
    }
  }

  function openPinDialog(): void {
    pinDialog = { pin1: "", pin2: "" };
  }

  function cancelPinDialog(): void {
    pinDialog = null;
  }

  async function commitPin(): Promise<void> {
    if (!pinDialog || screensaverBusy) return;
    const { pin1, pin2 } = pinDialog;
    if (!pin1) {
      screensaverError = "Enter a PIN";
      return;
    }
    if (pin1 !== pin2) {
      screensaverError = "PINs don't match";
      return;
    }
    screensaverBusy = true;
    screensaverError = null;
    try {
      const salt = workspace.info?.root ?? "";
      const hash = await hashPin(pin1, salt);
      const s = await api.screensaverSetPin(hash);
      screensaverEnabled = s.enabled;
      screensaverTimeoutSecs = s.timeout_secs;
      screensaverTheme = s.theme;
      screensaverPinSet = s.pin_set;
      pinDialog = null;
      await loadScreensaverState();
    } catch (err) {
      screensaverError = `PIN save failed: ${(err as Error).message ?? err}`;
    } finally {
      screensaverBusy = false;
    }
  }

  async function clearPin(): Promise<void> {
    if (screensaverBusy) return;
    screensaverBusy = true;
    screensaverError = null;
    try {
      const s = await api.screensaverClearPin();
      screensaverEnabled = s.enabled;
      screensaverTimeoutSecs = s.timeout_secs;
      screensaverTheme = s.theme;
      screensaverPinSet = s.pin_set;
      await loadScreensaverState();
    } catch (err) {
      screensaverError = `PIN clear failed: ${(err as Error).message ?? err}`;
    } finally {
      screensaverBusy = false;
    }
  }

  /// Lock immediately. The Dashboard back-of-card stays mounted
  /// under the screensaver cover; unlocking returns the user to
  /// the same flipped view (no need for the old
  /// close-overlay-then-test-then-reopen dance the retired global
  /// Settings overlay used to perform).
  async function testScreenLock(): Promise<void> {
    if (screensaverBusy) return;
    screensaverError = null;
    await loadScreensaverState();
    if (!screensaver.loaded) {
      screensaverError = "screen lock state unavailable";
      return;
    }
    lockNow();
  }

  onMount(() => {
    void loadScreenLockState();
  });

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
  class="dashboard"
  aria-label="Infographics"
  data-theme={surfaceThemeOverride("dashboard")}
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
      surface="dashboard"
      ariaLabel="Infographics settings"
      onDone={closeSettings}
    >
        <!-- Appearance: device-wide chrome + editor body theme.
             Lives in browser storage; "System" follows the OS
             setting live. Per-Hybrid Inherit / Light / Dark
             overrides live in HybridEditor / HybridTerminal
             back-sides. -->
        <section>
          <h3>Appearance</h3>
          <p class="hint">
            Global default for chan's chrome and editor body. Per-
            device only; lives in browser storage. "System" follows
            your OS appearance setting live. Override per-Hybrid in
            the Hybrid Editor or Hybrid Terminal back-side
            (Inherit / Light / Dark).
          </p>
          <div class="theme-row" role="radiogroup" aria-label="Appearance">
            {#each [
              { value: "system", label: "System" },
              { value: "light", label: "Light" },
              { value: "dark", label: "Dark" },
            ] as opt (opt.value)}
              <label class="theme-opt" class:on={ui.themeChoice === opt.value}>
                <input
                  type="radio"
                  name="app-appearance"
                  value={opt.value}
                  checked={ui.themeChoice === opt.value}
                  onchange={() => {
                    const v = opt.value as ThemeChoice;
                    setThemeChoice(v);
                  }}
                />
                <span>{opt.label}</span>
              </label>
            {/each}
          </div>
        </section>

        <section class="screen-lock">
          <h3>Screen lock</h3>
          <div class="screen-lock-row">
            <div class="screen-lock-meta">
              <div class="screen-lock-sub">
                Auto-lock the workspace view after inactivity.
                Local-only PIN protection (Mod+L locks now).
              </div>
              {#if screensaverError}
                <div class="err" role="alert">{screensaverError}</div>
              {/if}
              {#if screensaverEnabled === true}
                <div class="screensaver-config">
                  <label class="screensaver-timeout">
                    <span>Inactivity timeout (seconds):</span>
                    <input
                      type="number"
                      bind:value={screensaverTimeoutSecs}
                      onchange={commitTimeout}
                      min={SCREENSAVER_MIN_TIMEOUT_SECS}
                      max={SCREENSAVER_MAX_TIMEOUT_SECS}
                      step="30"
                      disabled={screensaverBusy}
                    />
                  </label>
                  <div class="screensaver-pin-controls">
                    {#if pinDialog === null}
                      <button type="button" onclick={testScreenLock} disabled={screensaverBusy}>
                        Test
                      </button>
                      {#if screensaverPinSet}
                        <button type="button" onclick={openPinDialog} disabled={screensaverBusy}>
                          Change PIN
                        </button>
                        <button type="button" onclick={clearPin} disabled={screensaverBusy}>
                          Clear PIN
                        </button>
                      {:else}
                        <button type="button" onclick={openPinDialog} disabled={screensaverBusy}>
                          Set PIN
                        </button>
                        <span class="muted">No PIN yet; lockout informational only.</span>
                      {/if}
                    {:else}
                      <input
                        type="password"
                        bind:value={pinDialog.pin1}
                        placeholder="PIN"
                        autocomplete="off"
                        disabled={screensaverBusy}
                      />
                      <input
                        type="password"
                        bind:value={pinDialog.pin2}
                        placeholder="Confirm"
                        autocomplete="off"
                        disabled={screensaverBusy}
                      />
                      <button type="button" onclick={commitPin} disabled={screensaverBusy}>
                        Save
                      </button>
                      <button type="button" onclick={cancelPinDialog} disabled={screensaverBusy}>
                        Cancel
                      </button>
                    {/if}
                  </div>
                </div>
              {/if}
            </div>
            <label class="screen-lock-switch">
              <input
                type="checkbox"
                checked={screensaverEnabled === true}
                disabled={screensaverEnabled === null || screensaverBusy}
                onchange={toggleScreensaverEnabled}
              />
              <span>
                {#if screensaverEnabled === null}
                  loading…
                {:else if screensaverBusy}
                  flipping…
                {:else if screensaverEnabled}
                  On
                {:else}
                  Off
                {/if}
              </span>
            </label>
          </div>
        </section>

        <section class="screensaver">
          <h3>Screensaver</h3>
          <p class="hint">
            Theme rendered behind the lock cover when the workspace
            view auto-locks.
          </p>
          <label class="screensaver-theme">
            <span>Theme:</span>
            <select
              bind:value={screensaverTheme}
              onchange={commitScreensaverTheme}
              disabled={screensaverBusy || screensaverEnabled !== true}
            >
              <option value="plain">Plain</option>
              <option value="matrix">Matrix</option>
            </select>
          </label>
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
  .dashboard {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--text);
  }
  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
  /* Appearance chip row, lifted from the retired global Settings overlay.
     The chip palette stays in sync with HybridEditorConfig +
     HybridTerminalConfig's per-Hybrid override chips. */
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
  .theme-opt input[type="radio"] {
    width: auto;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
  }
  .theme-opt > span { color: var(--text); }
  .theme-opt:hover { border-color: var(--btn-hover); }
  .theme-opt.on { border-color: var(--link); background: var(--hover-bg); }
  .screen-lock {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .screen-lock-row {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    padding: 0.5rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
  }
  .screen-lock-meta {
    flex: 1;
    min-width: 0;
  }
  .screen-lock-sub {
    font-size: 12.5px;
    color: var(--text-secondary);
    line-height: 1.35;
  }
  .screen-lock-meta .err {
    margin-top: 0.3rem;
    font-size: 12.5px;
    color: var(--warn-text);
  }
  .screen-lock-switch {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 13px;
    color: var(--text-secondary);
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
  }
  .screen-lock-switch input[disabled] {
    cursor: wait;
  }
  .screensaver-config {
    display: grid;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }
  .screensaver-theme,
  .screensaver-timeout {
    display: grid;
    grid-template-columns: minmax(9rem, auto) minmax(8rem, 1fr);
    align-items: center;
    gap: 0.5rem;
    max-width: 28rem;
    font-size: 13px;
  }
  .screensaver-theme select,
  .screensaver-timeout input,
  .screensaver-pin-controls input {
    min-width: 0;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 5px 7px;
    font: inherit;
  }
  .screensaver-pin-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .screensaver-pin-controls button {
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 5px 9px;
    font: inherit;
    cursor: pointer;
  }
  .screensaver-pin-controls button:hover:not(:disabled) {
    border-color: var(--btn-hover);
  }
  .screensaver-pin-controls button:disabled {
    opacity: 0.6;
    cursor: wait;
  }
  .muted { color: var(--text-secondary); font-style: italic; }
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
