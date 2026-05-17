<script lang="ts">
  // Settings overlay. Per-device-global preferences form (editor
  // theme, assistant, attachments_dir, default-drive path) plus the
  // local CLI assistant backend picker.
  //
  // The drive display name is edited from the file-browser
  // hamburger, not here, so the settings overlay is purely
  // about device-wide preferences.
  //
  // Auto-saves on change (500 ms debounce).

  import { onMount } from "svelte";
  import { ApiError, api } from "../api/client";
  import type {
    AssistantBackendKind,
    BuildInfo,
    CliDetectionView,
    EditorTheme,
    GlobalConfig,
    LineSpacing,
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
  import { DATE_FORMATS } from "../editor/dateFormats";
  import OverlayShell from "./OverlayShell.svelte";

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
  // Per-CLI readiness from /api/llm/cli_detection. Refreshed on
  // mount, dropdown change, and after a successful override save.
  let cliDetections = $state<CliDetectionView[]>([]);
  let cliDetectionLoading = $state(false);
  let cliDetectionError = $state<string | null>(null);
  let assistantSaveError = $state<string | null>(null);
  let selectedCli = $state<CliBackendKind>("claude_cli");

  /// Build identity for the About footer. Loaded on mount; the
  /// version + embeddings feature flag are static for the running
  /// binary so a single fetch is enough.
  let buildInfo = $state<BuildInfo | null>(null);

  async function loadCliDetection(): Promise<void> {
    if (cliDetectionLoading) return;
    cliDetectionLoading = true;
    cliDetectionError = null;
    try {
      cliDetections = (await api.llmCliDetection()).detections;
    } catch (e) {
      cliDetections = [];
      cliDetectionError = cliDetectionErrorMessage(e);
    } finally {
      cliDetectionLoading = false;
    }
  }

  type CliBackendKind = Extract<
    AssistantBackendKind,
    "claude_cli" | "gemini_cli" | "codex_cli"
  >;

  function cliPrefs(
    a: Preferences["assistant"],
    kind: CliBackendKind,
  ): Preferences["assistant"][CliBackendKind] {
    return a[kind];
  }

  function activeCliKind(): CliBackendKind {
    return selectedCli;
  }

  function activeDetection(): CliDetectionView | null {
    const kind = activeCliKind();
    return cliDetections.find((d) => d.backend === kind) ?? null;
  }

  function cliDetectionErrorMessage(e: unknown): string {
    if (e instanceof ApiError) {
      if (e.status === 404) {
        return "CLI detection endpoint is unavailable; restart the chan backend so it serves the latest API.";
      }
      return `CLI detection failed (${e.status}): ${e.message}`;
    }
    return `CLI detection failed: ${(e as Error).message}`;
  }

  function activeDetectionReason(): string {
    const detection = activeDetection();
    if (detection?.reason) return detection.reason;
    if (cliDetectionError) return cliDetectionError;
    if (cliDetections.length > 0) {
      return `no detection record returned for ${activeCliKind()}`;
    }
    return "not ready";
  }

  function commandLabel(detection: CliDetectionView | null): string | null {
    if (!detection || detection.command.length === 0) return null;
    return detection.command.join(" ");
  }

  function onActiveCliChange(e: Event): void {
    selectedCli = (e.currentTarget as HTMLSelectElement).value as CliBackendKind;
    assistantSaveError = null;
    void loadCliDetection();
  }
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
      // Migrate dead format ids (e.g. the retired "short" no-year
      // variant) to the catalog's default so the <select> below
      // doesn't render a blank option. The settings auto-save will
      // persist the corrected value on the next dirty edit, or the
      // user can re-pick explicitly.
      const knownIds = new Set(DATE_FORMATS.map((f) => f.id));
      if (!knownIds.has(editing.date_format as never)) {
        editing.date_format = DATE_FORMATS[0]!.id;
      }
    }
  });

  /// Fill in optional sub-views the server only learned about
  /// recently. An older chan-server returns `assistant.claude_cli`
  /// / `assistant.gemini_cli` as undefined; the model <select>
  /// crashes on `.model` access. Applied to BOTH editing and
  /// globalConfig so dirty() doesn't see a permanent diff and
  /// trigger an autosave loop.
  function normalizePrefs(p: Preferences): Preferences {
    const a = p.assistant as { [k: string]: unknown };
    if (a.claude_cli === undefined) a.claude_cli = { enabled: false, model: null };
    if (a.gemini_cli === undefined) a.gemini_cli = { enabled: false, model: null };
    if (a.codex_cli === undefined) a.codex_cli = { enabled: false, model: null };
    if (p.line_spacing === "tight") p.line_spacing = "compact";
    if (p.line_spacing !== "compact" && p.line_spacing !== "standard") {
      p.line_spacing = "standard";
    }
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
    assistantSaveError = null;
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
      void loadCliDetection();
      saveStatus = "saved";
      savedFlashTimer = setTimeout(() => {
        if (saveStatus === "saved") saveStatus = "idle";
        savedFlashTimer = null;
      }, SAVED_FLASH_MS);
    } catch (e) {
      const message = (e as Error).message;
      failedSaveSnap = sent;
      assistantSaveError = message;
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

  // Live-apply the editor-theme attribute on every change so the
  // editor in the background re-skins instantly, without waiting
  // for the 500 ms autosave + server round-trip. The App.svelte
  // post-save $effect later reapplies from the authoritative
  // drive.info; both paths produce the same DOM attribute.
  $effect(() => {
    if (!editing) return;
    document.documentElement.setAttribute(
      "data-editor-theme",
      editing.editor_theme,
    );
  });

  async function loadBuildInfo(): Promise<void> {
    try {
      buildInfo = await api.buildInfo();
    } catch {
      // Non-fatal: footer falls back to "n/a".
      buildInfo = null;
    }
  }

  onMount(() => {
    // Make sure we have the latest server state when the tab opens.
    void refreshDrive();
    void loadGlobalConfig();
    void loadCliDetection();
    void loadBuildInfo();
  });

  /// Friendly labels used in the provider list. Centralized so the
  /// dropdown ordering and the row ordering stay consistent across
  /// the markup below.
  const PROVIDER_ROWS: { kind: CliBackendKind; label: string; hint: string }[] = [
    { kind: "claude_cli", label: "Claude CLI", hint: "local `claude` shell-executor" },
    { kind: "gemini_cli", label: "Gemini CLI", hint: "local `gemini` shell-executor" },
    { kind: "codex_cli", label: "Codex CLI", hint: "local `codex` shell-executor" },
  ];
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
    <section>
      <h3>Agent</h3>
      <p class="hint">
        Configure each local agent CLI and optionally override its
        binary lookup.
      </p>

      <div class="assistant-config">
        <div class="assistant-control">
          <label class="assistant-field">
            <span>Agent CLI</span>
            <select value={activeCliKind()} onchange={onActiveCliChange}>
              {#each PROVIDER_ROWS as row (row.kind)}
                <option value={row.kind}>{row.label}</option>
              {/each}
            </select>
          </label>

          <div class="cli-readiness" aria-live="polite">
            {#if cliDetectionLoading && !activeDetection()}
              <span class="status-dot muted-dot"></span>
              <span class="muted">checking…</span>
            {:else if activeDetection()?.ready}
              <span class="status-dot ok-dot"></span>
              <span class="ok">ready</span>
            {:else}
              <span class="status-dot err-dot"></span>
              <span class="err">{activeDetectionReason()}</span>
            {/if}
          </div>
        </div>

        <div class="assistant-control">
          <label class="assistant-field">
            <span>Binary path override</span>
            <input
              value={cliPrefs(editing.assistant, activeCliKind()).cmd_override ?? ""}
              placeholder={commandLabel(activeDetection()) ?? "use PATH"}
              spellcheck="false"
              autocomplete="off"
              oninput={(e) => {
                if (!editing) return;
                const value = (e.currentTarget as HTMLInputElement).value.trim();
                cliPrefs(editing.assistant, activeCliKind()).cmd_override =
                  value === "" ? null : value;
                assistantSaveError = null;
              }}
            />
          </label>
          <div class="hint-text">
            leave blank to resolve {activeCliKind().replace("_cli", "")} from PATH
          </div>
          {#if assistantSaveError}
            <div class="override-error">{assistantSaveError}</div>
          {/if}
        </div>
      </div>
    </section>

    <div class="section-row">
    <section>
      <h3>Editor theme</h3>
      <p class="hint">
        Style of the markdown editor only — typography, headings,
        code blocks, links, tables.
      </p>
      <div class="theme-row" role="radiogroup" aria-label="Editor theme">
        {#each [
          { value: "github", label: "GitHub" },
          { value: "google_docs", label: "Google Docs" },
          { value: "word", label: "Microsoft Word" },
        ] as opt (opt.value)}
          <label
            class="theme-opt"
            class:on={editing.editor_theme === opt.value}
          >
            <input
              type="radio"
              name="editor-theme"
              value={opt.value}
              checked={editing.editor_theme === opt.value}
              onchange={() => {
                editing!.editor_theme = opt.value as EditorTheme;
              }}
            />
            <span>{opt.label}</span>
          </label>
        {/each}
      </div>
    </section>

    <section>
      <h3>Appearance</h3>
      <p class="hint">
        Per-device only; lives in browser storage. "System" follows
        your OS appearance setting live.
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
              name="theme"
              value={opt.value}
              checked={ui.themeChoice === opt.value}
              onchange={() => {
                const v = opt.value as ThemeChoice;
                setThemeChoice(v);
                // Keep the autosave form in sync; otherwise the next
                // PATCH (any dirty edit, or a pending autosave) ships
                // `editing.theme` stale and reverts the choice.
                if (editing) editing.theme = v;
                if (globalConfig) globalConfig.preferences.theme = v;
              }}
            />
            <span>{opt.label}</span>
          </label>
        {/each}
      </div>
    </section>
    </div>

    <div class="section-row">
    <section>
      <h3>Layout</h3>
      <p class="hint">
        Standard is the default reading density; compact tightens paragraph
        and list spacing while keeping the editor readable.
      </p>
      <!-- Reuses .theme-row / .theme-opt pill styles so this radio
           visually matches the Theme picker above. -->
      <div class="theme-row" role="radiogroup" aria-label="Line spacing">
        {#each [
          { value: "standard", label: "Standard" },
          { value: "compact", label: "Compact" },
        ] as opt (opt.value)}
          <label class="theme-opt" class:on={editing.line_spacing === opt.value}>
            <input
              type="radio"
              name="line-spacing"
              value={opt.value}
              checked={editing.line_spacing === opt.value}
              onchange={() => {
                editing!.line_spacing = opt.value as LineSpacing;
              }}
            />
            <span>{opt.label}</span>
          </label>
        {/each}
      </div>
    </section>

    <section>
      <h3>Date pills</h3>
      <p class="hint">
        Format used by <code>@today</code> and pre-selected in the
        <code>@date</code> picker. The editor still detects every
        format on this list when reading a file or watching you
        type, so old documents keep auto-pilling regardless of
        which one is the default here.
      </p>
      <label class="font-row">
        <span>Default</span>
        <select class="family" bind:value={editing.date_format}>
          {#each DATE_FORMATS as f (f.id)}
            <option value={f.id}>{f.label}</option>
          {/each}
        </select>
      </label>
    </section>
    </div>

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
  /* Two-column section pairing (Editor theme + Appearance, Layout +
     Date pills). Each child section keeps its own header + content
     stack but loses its individual bottom border: the wrapper carries
     the divider for the whole row. */
  .section-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.2rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border);
  }
  .section-row > section {
    padding-bottom: 0;
    border-bottom: 0;
    min-width: 0;
  }
  h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  label {
    display: grid;
    grid-template-columns: 7em 1fr;
    align-items: center;
    gap: 0.5rem;
    font-size: 15px;
  }
  label > span { color: var(--text-secondary); }
  input, select {
    background: var(--bg-card);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 7px;
    font: inherit;
    font-size: 15px;
    outline: none;
    width: 100%;
  }
  input:focus, select:focus { border-color: var(--link); }
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
  .hint {
    color: var(--text-secondary);
    font-size: 11.5px;
    margin: 0 0 0.5rem 0;
  }
  .hint code {
    font-family: ui-monospace, monospace;
    font-size: 13px;
    background: var(--bg-card);
    padding: 0 4px;
    border-radius: 3px;
  }
  /* Theme picker: three radios laid out as segmented chips.
     The generic `label { display: grid }` and `input { width: 100% }`
     rules above target every form control in this tab; we have to
     undo both for the chips so the radio sits inline with its label
     inside one bordered box. */
  .theme-row { display: flex; gap: 4px; }
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
  .theme-opt.on {
    border-color: var(--link);
    background: var(--hover-bg);
  }
  /* Inline status colors for the Assistant section's key state. */
  .v .ok { color: var(--accent); }
  .cli-readiness .ok { color: var(--accent); }
  .cli-readiness .err { color: var(--warn-text); }
  .assistant-config {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.8rem;
    margin-top: 0.2rem;
  }
  .assistant-control {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-width: 0;
  }
  .assistant-field {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 4px;
    font-size: 14px;
  }
  .assistant-field > span {
    color: var(--text-secondary);
    font-size: 13px;
  }
  .assistant-field select,
  .assistant-field input {
    background: var(--bg-card);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 6px;
    font: inherit;
    font-size: 14px;
    outline: none;
    width: 100%;
  }
  .assistant-field select:focus,
  .assistant-field input:focus { border-color: var(--link); }
  .cli-readiness {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    color: var(--text-secondary);
    font-size: 13px;
    min-height: 1.4rem;
  }
  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .ok-dot { background: var(--accent); }
  .err-dot { background: var(--warn-text); }
  .muted-dot { background: var(--text-secondary); }
  .override-error {
    color: var(--warn-text);
    font-size: 12px;
    line-height: 1.35;
  }
  /* Tab-bar autosave indicator. Sits between the title and the
     actions strip. Empty when idle (no extra padding). */
  .save-status { font-size: 14px; min-width: 60px; text-align: right; }
  .save-status .ok { color: var(--accent); }
  .save-status .err { color: #d33; }
  .save-status .muted { color: var(--text-secondary); }
  @media (max-width: 760px) {
    .section-row,
    .assistant-config {
      grid-template-columns: 1fr;
    }
  }
</style>
