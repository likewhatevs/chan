<script lang="ts">
  // Screen lock for the "This workspace" settings tab. Auto-locks the
  // workspace view after inactivity with an optional local PIN (salted by the
  // workspace root, so it is per-workspace). It owns enable/timeout/theme
  // patches, the PIN set/confirm/clear dialog, a Test action, and the theme
  // preview.

  import { onMount } from "svelte";
  import {
    hashPin,
    SCREENSAVER_MAX_TIMEOUT_SECS,
    SCREENSAVER_MIN_TIMEOUT_SECS,
    type ScreensaverTheme,
  } from "../../../state/screensaver";
  import {
    loadScreensaverState,
    lockNow,
    screensaver,
  } from "../../../state/screensaver.svelte";
  import { api } from "../../../api/client";
  import { workspace } from "../../../state/store.svelte";
  import MatrixRainPreview from "../../screensaver/MatrixRainPreview.svelte";
  import PlainScreensaverPreview from "../../screensaver/PlainScreensaverPreview.svelte";

  let screensaverEnabled = $state<boolean | null>(null);
  let screensaverTimeoutSecs = $state<number>(300);
  let screensaverTheme = $state<ScreensaverTheme>("plain");
  let screensaverPinSet = $state(false);
  let screensaverBusy = $state(false);
  let screensaverError = $state<string | null>(null);
  /// PIN edit buffer. `null` when not showing the dialog; otherwise carries
  /// the pin1/pin2 confirm pair.
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
</script>

<section class="screen-lock">
  <h3>Screen lock</h3>
  <div class="screen-lock-row">
    <div class="screen-lock-meta">
      <div class="screen-lock-sub">
        Auto-lock the workspace view after inactivity. Local-only PIN protection
        (Mod+L locks now).
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
                <span class="muted">No PIN set; lockout informational only.</span>
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
          <label class="screensaver-theme">
            <span>Theme:</span>
            <select
              bind:value={screensaverTheme}
              onchange={commitScreensaverTheme}
              disabled={screensaverBusy}
            >
              <option value="plain">Default</option>
              <option value="matrix">Matrix</option>
            </select>
          </label>
          <p class="hint">
            Theme rendered behind the lock cover when the workspace view
            auto-locks.
          </p>
        </div>
        <div class="screensaver-preview">
          <div class="preview-title">Screensaver preview</div>
          <div class="preview-box">
            {#if screensaverTheme === "matrix"}
              <MatrixRainPreview width={320} height={180} />
            {:else}
              <PlainScreensaverPreview width={320} height={180} />
            {/if}
          </div>
          <p class="hint">
            Preview of the {screensaverTheme === "matrix" ? "Matrix" : "Default"} lock
            theme.
          </p>
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
          loading...
        {:else if screensaverBusy}
          flipping...
        {:else if screensaverEnabled}
          On
        {:else}
          Off
        {/if}
      </span>
    </label>
  </div>
</section>

<style>
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
  .muted {
    color: var(--text-secondary);
    font-style: italic;
  }
  .screensaver-preview {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin-top: 0.75rem;
  }
  .preview-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .preview-box {
    width: 320px;
    height: 180px;
    max-width: 100%;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
</style>
