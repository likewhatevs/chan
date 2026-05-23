<script lang="ts">
  // Settings overlay. Per-device-global preferences form (editor
  // theme, editor density, date format, and theme).
  //
  // The drive display name is edited from the file-browser
  // hamburger, not here, so the settings overlay is purely
  // about device-wide preferences.
  //
  // Auto-saves on change (500 ms debounce).

  import { onMount } from "svelte";
  import { api } from "../api/client";
  import {
    hashPin,
    SCREENSAVER_MAX_TIMEOUT_SECS,
    SCREENSAVER_MIN_TIMEOUT_SECS,
    type ScreensaverTheme,
  } from "../state/screensaver";
  import {
    loadScreensaverState,
    lockNow,
    pauseScreensaverTimer,
    screensaver,
  } from "../state/screensaver.svelte";
  import type {
    BuildInfo,
    GlobalConfig,
    Preferences,
  } from "../api/types";
  import { Maximize2, Minimize2, X } from "lucide-svelte";
  import {
    refreshDrive,
    settingsOverlay,
    setThemeChoice,
    type ThemeChoice,
    ui,
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
  // `fullstack-a-46` (Task C) moved the Editor settings
  // (Editor theme, Appearance, Layout / line spacing, Date pills /
  // date format, On save / strip trailing whitespace) out of this
  // overlay into `HybridEditorConfig.svelte`.
  //
  // `fullstack-a-48` (Task F option B) moved the Semantic search
  // section out of this overlay into `HybridFileBrowserConfig.svelte`
  // alongside report indexing + the future multi-model picker
  // placeholder.
  //
  // `fullstack-a-53` reverts the Appearance section from
  // `HybridEditorConfig.svelte` back here. Appearance is a GLOBAL
  // default; the per-Hybrid override toggle (Inherit / Light /
  // Dark) lives in BOTH Hybrid Editor + Hybrid Terminal back-sides
  // and writes to `pane.theme` (the existing override slot from
  // `-b-5`/`-a-47`). Render resolution: pane.theme wins if set;
  // else this overlay's `ui.themeChoice`.

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
  /// round-trip.
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
  // alongside the matching editor settings.

  async function loadBuildInfo(): Promise<void> {
    try {
      buildInfo = await api.buildInfo();
    } catch {
      // Non-fatal: footer falls back to "n/a".
      buildInfo = null;
    }
  }

  // Settings owns the screen-lock controls. Search/report feature
  // toggles live in `HybridFileBrowserConfig.svelte`, where their
  // file-browser ownership and richer download/reporting flows are
  // visible.
  let screensaverEnabled = $state<boolean | null>(null);
  let screensaverTimeoutSecs = $state<number>(300);
  let screensaverTheme = $state<ScreensaverTheme>("plain");
  let screensaverPinSet = $state(false);
  let screensaverBusy = $state(false);
  let screensaverError = $state<string | null>(null);
  /// PIN edit buffer. `null` when not showing the dialog;
  /// otherwise carries the pin1/pin2 confirm pair.
  let pinDialog = $state<{ pin1: string; pin2: string } | null>(null);

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

  // `fullstack-a-77` slice 3: screensaver toggle handlers +
  // PIN setup flow. Each handler calls the matching api
  // method + refreshes the screensaver singleton via
  // `loadScreensaverState()` so the App-root tracker
  // re-arms with the new state.

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
      const salt = drive.info?.root ?? "";
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

  /// `fullstack-a-77` slice 3: pause the screensaver
  /// inactivity timer while Settings is open so a long
  /// configuration session doesn't trigger the lock
  /// mid-edit. Released on close.
  let screensaverPauseRelease: (() => void) | null = null;

  onMount(() => {
    // Make sure we have the latest server state when the tab opens.
    void refreshDrive();
    void loadGlobalConfig();
    void loadBuildInfo();
    void loadScreenLockState();
    screensaverPauseRelease = pauseScreensaverTimer();
    return () => {
      screensaverPauseRelease?.();
      screensaverPauseRelease = null;
    };
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
    <!-- `fullstack-a-45` Terminal, `-a-46` Editor settings,
         `-a-48` Semantic search migrations: see component-level
         comments in `HybridTerminalConfig` / `HybridEditorConfig`
         / `HybridFileBrowserConfig`. Only the Appearance (global
         theme), Screen lock, and About sections live here after
         the wave.

         `fullstack-a-53` Appearance revert: this section MOVED
         briefly to HybridEditorConfig in `-a-46`; @@Alex's design
         correction (2026-05-21) restored it here. The global
         default lives in Settings; per-Hybrid Inherit / Light /
         Dark overrides live in HybridEditor + HybridTerminal
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
              name="settings-appearance"
              value={opt.value}
              checked={ui.themeChoice === opt.value}
              onchange={() => {
                const v = opt.value as ThemeChoice;
                setThemeChoice(v);
                // Keep the autosave form in sync so the next
                // PATCH ships the new theme value (otherwise the
                // merge would revert the choice).
                if (editing) editing.theme = v;
                if (globalConfig) globalConfig.preferences.theme = v;
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
            Auto-lock the drive view after inactivity.
            Local-only PIN protection (Mod+L locks now).
          </div>
          {#if screensaverError}
            <div class="err" role="alert">{screensaverError}</div>
          {/if}
          {#if screensaverEnabled === true}
            <div class="screensaver-config">
              <label class="screensaver-theme">
                <span>Theme:</span>
                <select
                  bind:value={screensaverTheme}
                  onchange={commitScreensaverTheme}
                  disabled={screensaverBusy}
                >
                  <option value="plain">Plain</option>
                  <option value="matrix">Matrix</option>
                  <option value="castaway">Castaway</option>
                </select>
              </label>
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
     pills). The remaining sections stack single-column. */
  h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  /* About uses a compact read-only grid; screen-lock controls own
     their narrower form styles below. */
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
  .v .ok { color: var(--accent); }
  /* `fullstack-a-48` (Task F option B) swept most of the
     semantic-search CSS scope. `fullstack-a-53` brings back the
     `.theme-row` + `.theme-opt` chip styles for the restored
     Appearance section (the Inherit / Light / Dark per-Hybrid
     override toggles in HybridEditorConfig + HybridTerminalConfig
     own their own copies of these). The Appearance section needs
     the chip layout to render the system / light / dark radios. */
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
  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
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
  /* Tab-bar autosave indicator. Sits between the title and the
     actions strip. Empty when idle (no extra padding). */
  .save-status { font-size: 14px; min-width: 60px; text-align: right; }
  .save-status .ok { color: var(--accent); }
  .save-status .err { color: #d33; }
  .save-status .muted { color: var(--text-secondary); }
</style>
