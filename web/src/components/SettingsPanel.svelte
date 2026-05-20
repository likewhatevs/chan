<script lang="ts">
  // Settings overlay. Per-device-global preferences form (editor
  // theme, editor density, date format, and theme).
  //
  // The drive display name is edited from the file-browser
  // hamburger, not here, so the settings overlay is purely
  // about device-wide preferences.
  //
  // Auto-saves on change (500 ms debounce).

  import { onDestroy, onMount } from "svelte";
  import { api } from "../api/client";
  import type {
    BuildInfo,
    EditorTheme,
    GlobalConfig,
    LineSpacing,
    Preferences,
    SemanticState,
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
  import { editorToolsPrefs } from "../state/editorTools.svelte";
  import {
    clampScrollbackMb,
    SCROLLBACK_MB_DEFAULT,
    SCROLLBACK_MB_MAX,
    SCROLLBACK_MB_MIN,
  } from "../terminal/scrollback";
  import OverlayShell from "./OverlayShell.svelte";

  // `fullstack-b-11`: TERM dropdown options. Known terminfo entries
  // most users either want by default or fall back to from a custom
  // environment; the "Custom..." escape hatch toggles a free-text
  // input for exotic values (alacritty-direct, kitty, vt100, etc.).
  const KNOWN_TERM_VALUES = [
    "xterm-256color",
    "xterm",
    "tmux-256color",
    "screen-256color",
  ] as const;
  const DEFAULT_TERM = "xterm-256color";
  const CUSTOM_TERM_SENTINEL = "__custom__";

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

  /// Fill in optional preference fields older servers may omit.
  /// Applied to BOTH editing and globalConfig so dirty() doesn't
  /// see a permanent diff and trigger an autosave loop.
  function normalizePrefs(p: Preferences): Preferences {
    if (p.line_spacing === "tight") p.line_spacing = "compact";
    if (p.line_spacing !== "compact" && p.line_spacing !== "standard") {
      p.line_spacing = "standard";
    }
    // `fullstack-b-11`: older servers ship the `terminal` subtree
    // without scrollback / TERM. Snap to the defaults here so the
    // dirty() comparison after a server refetch doesn't trigger an
    // autosave loop.
    if (p.terminal) {
      p.terminal.scrollback_mb = clampScrollbackMb(p.terminal.scrollback_mb);
      const term = (p.terminal.default_term ?? "").trim();
      p.terminal.default_term = term.length > 0 ? term : DEFAULT_TERM;
    }
    return p;
  }

  // `fullstack-b-11`: derived view of the Terminal section so the UI
  // bindings read concrete numbers / strings instead of nullables.
  // The dropdown shape collapses to "custom" when the persisted value
  // isn't one of the four shipped terminfo entries.
  const scrollbackMb = $derived(
    clampScrollbackMb(editing?.terminal?.scrollback_mb),
  );
  const currentTerm = $derived(
    (editing?.terminal?.default_term ?? DEFAULT_TERM).trim() || DEFAULT_TERM,
  );
  const isKnownTerm = $derived(
    (KNOWN_TERM_VALUES as readonly string[]).includes(currentTerm),
  );
  // `bind:value` on the select wants a plain accessor; we keep the
  // sentinel branch flat to avoid an extra binding state slot.
  const termSelectValue = $derived(
    isKnownTerm ? currentTerm : CUSTOM_TERM_SENTINEL,
  );

  function setScrollbackMb(raw: number): void {
    if (!editing) return;
    // Range input emits strings via the bind:value coercion; the
    // clamp helper guards against an over-eager browser that hands
    // back a value outside the rendered min/max.
    editing.terminal = {
      ...editing.terminal,
      scrollback_mb: clampScrollbackMb(raw),
    };
  }

  function setTermSelection(next: string): void {
    if (!editing) return;
    if (next === CUSTOM_TERM_SENTINEL) {
      // Switching to custom keeps the existing string (whatever the
      // server last persisted) so the textbox isn't blanked out the
      // moment the user opens it; if the existing value is already
      // a known entry, swap to an empty string so the input prompts.
      const seed = isKnownTerm ? "" : currentTerm;
      editing.terminal = { ...editing.terminal, default_term: seed };
      return;
    }
    editing.terminal = { ...editing.terminal, default_term: next };
  }

  function setCustomTerm(raw: string): void {
    if (!editing) return;
    editing.terminal = { ...editing.terminal, default_term: raw };
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

  // `fullstack-a-25`: keep the local editor-tools snapshot in sync
  // with the in-flight `editing.strip_trailing_whitespace_on_save`
  // value so save() (which checks editorToolsPrefs before stripping)
  // sees the new value the moment the user toggles in Settings,
  // without waiting for the autosave PATCH + the next /api/config
  // round-trip to refresh the snapshot. The autosave still
  // persists the value to the server in the background.
  $effect(() => {
    if (!editing) return;
    editorToolsPrefs.stripTrailingWhitespaceOnSave =
      editing.strip_trailing_whitespace_on_save;
  });

  async function loadBuildInfo(): Promise<void> {
    try {
      buildInfo = await api.buildInfo();
    } catch {
      // Non-fatal: footer falls back to "n/a".
      buildInfo = null;
    }
  }

  /// `fullstack-a-21` semantic-search opt-in. Snapshot of the
  /// server's `systacean-7` state plus a downloading flag the UI
  /// owns (we don't store it server-side; it lives in the
  /// in-flight POST). Error sticks until the next user action.
  let semanticState = $state<SemanticState | null>(null);
  let semanticDownloading = $state(false);
  let semanticEnabling = $state(false);
  let semanticError = $state<string | null>(null);
  /// Polling handle used during downloads. The download endpoint
  /// is synchronous in v1 (no per-byte progress events), so we
  /// poll `/api/index/semantic/state` every few seconds during
  /// the wait to detect the `model_present` transition out-of-band
  /// from the awaited POST. Cleared on success / failure / unmount.
  let semanticPollTimer: ReturnType<typeof setInterval> | null = null;
  const SEMANTIC_POLL_INTERVAL_MS = 3000;

  async function loadSemanticState(): Promise<void> {
    try {
      semanticState = await api.semanticState();
    } catch {
      // Older servers without the endpoint surface as a no-op; the
      // section just renders the "not available" placeholder.
      semanticState = null;
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
      if (semanticState.model_present) {
        // Model already on disk — just enable. No download wait.
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
      // First-time download. Kick off the synchronous POST and
      // start polling state in parallel so the spinner reflects
      // the model_present transition even before the POST returns.
      semanticDownloading = true;
      stopSemanticPoll();
      semanticPollTimer = setInterval(() => {
        void loadSemanticState();
      }, SEMANTIC_POLL_INTERVAL_MS);
      try {
        semanticState = await api.semanticDownload();
        stopSemanticPoll();
        // Server returns the post-download state; auto-enable on
        // top so the toggle lands ON rather than leaving the user
        // a second click on a freshly-downloaded model.
        semanticEnabling = true;
        try {
          semanticState = await api.semanticEnable();
        } finally {
          semanticEnabling = false;
        }
      } catch (err) {
        stopSemanticPoll();
        semanticError = (err as Error).message;
        // Refresh state so the toggle reflects whatever the server
        // ended up with (the download may have partially landed).
        await loadSemanticState();
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

  function formatModelSize(bytes: number | null): string {
    if (bytes === null || bytes <= 0) return "size unknown";
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(1)} MB`;
  }

  onMount(() => {
    // Make sure we have the latest server state when the tab opens.
    void refreshDrive();
    void loadGlobalConfig();
    void loadBuildInfo();
    void loadSemanticState();
  });

  onDestroy(() => {
    stopSemanticPoll();
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

    <!-- `fullstack-a-25`: trailing-whitespace toggle moved from
         the editor's right-click / hamburger menu (where it was
         a checkbox `Run automatically on save / auto-save`) to
         Settings, where editor preferences belong. Binding goes
         through `editing.strip_trailing_whitespace_on_save`
         (autosave handles persistence) + a sibling $effect that
         keeps `editorToolsPrefs.stripTrailingWhitespaceOnSave`
         in sync so save() sees the new value immediately. -->
    <section>
      <h3>On save</h3>
      <p class="hint">
        Strip trailing whitespace from each line when the file is
        saved. Affects every editable text buffer (.md, .txt, source
        files). The one-shot "Remove trailing whitespace" action in
        the editor menu still works for manual cleanup.
      </p>
      <label class="theme-opt semantic-toggle" class:on={editing.strip_trailing_whitespace_on_save}>
        <input
          type="checkbox"
          bind:checked={editing.strip_trailing_whitespace_on_save}
        />
        <span>Strip trailing whitespace on save</span>
      </label>
    </section>
    </div>

    <!-- `fullstack-b-11`: per-terminal scrollback budget (MB) and
         default TERM env var. Settings apply to NEWLY spawned
         terminals; existing terminals keep their current values
         until the session restarts. The hint text under each
         control names this contract explicitly so users don't
         expect retroactive resize. -->
    <section class="terminal-section">
      <h3>Terminal</h3>
      <p class="hint">
        Applies to terminals spawned after this setting changes.
        Existing terminals keep their current scrollback and
        <code>TERM</code> value until the chan session restarts.
      </p>

      <div class="terminal-field">
        <label class="terminal-label" for="terminal-scrollback-mb">
          <span>Scrollback (MB)</span>
        </label>
        <div class="terminal-control scrollback-control">
          <input
            id="terminal-scrollback-mb"
            type="range"
            min={SCROLLBACK_MB_MIN}
            max={SCROLLBACK_MB_MAX}
            step="5"
            value={scrollbackMb}
            oninput={(e) => setScrollbackMb(Number((e.currentTarget as HTMLInputElement).value))}
            aria-describedby="terminal-scrollback-hint"
          />
          <input
            class="scrollback-number"
            type="number"
            min={SCROLLBACK_MB_MIN}
            max={SCROLLBACK_MB_MAX}
            step="1"
            value={scrollbackMb}
            oninput={(e) => setScrollbackMb(Number((e.currentTarget as HTMLInputElement).value))}
            aria-label="Scrollback buffer in megabytes"
          />
          <span class="terminal-unit">MB</span>
        </div>
        <p id="terminal-scrollback-hint" class="hint sub-hint">
          Per-terminal cap on the in-memory scrollback buffer.
          Default {SCROLLBACK_MB_DEFAULT} MB; range
          {SCROLLBACK_MB_MIN}-{SCROLLBACK_MB_MAX} MB. Higher caps
          let agent-driven terminals retain more redraw history at
          the cost of more browser memory per open tab.
        </p>
      </div>

      <div class="terminal-field">
        <label class="terminal-label" for="terminal-default-term">
          <span>Default TERM</span>
        </label>
        <div class="terminal-control">
          <select
            id="terminal-default-term"
            class="family"
            value={termSelectValue}
            onchange={(e) =>
              setTermSelection((e.currentTarget as HTMLSelectElement).value)}
          >
            {#each KNOWN_TERM_VALUES as value (value)}
              <option {value}>{value}</option>
            {/each}
            <option value={CUSTOM_TERM_SENTINEL}>Custom...</option>
          </select>
        </div>
        {#if termSelectValue === CUSTOM_TERM_SENTINEL}
          <div class="terminal-control">
            <input
              type="text"
              class="custom-term"
              placeholder="alacritty-direct"
              value={currentTerm}
              oninput={(e) =>
                setCustomTerm((e.currentTarget as HTMLInputElement).value)}
              aria-label="Custom TERM value"
            />
          </div>
        {/if}
        <p class="hint sub-hint">
          Sets <code>TERM</code> on newly spawned shells. Pick one
          of the common terminfo entries or supply a custom value
          if your environment expects something exotic.
        </p>
      </div>
    </section>

    <!-- `fullstack-a-21`: opt-in to Hybrid search (BM25 + dense
         vectors via BGE-small). The model is no longer bundled in
         the default binary (`systacean-6` cargo feature gate); the
         user downloads it on demand from this row. v1 uses a
         spinner + polling pattern rather than a per-byte progress
         bar because hf-hub doesn't expose progress callbacks
         (per @@Systacean's `systacean-7` constraint); the
         downloading endpoint is synchronous and the UI polls
         `/state` in parallel to surface the model_present
         transition. -->
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
          model file is shared across drives.
        </p>
        <label class="theme-opt semantic-toggle" class:on={semanticState.semantic_enabled}>
          <input
            type="checkbox"
            checked={semanticState.semantic_enabled}
            disabled={semanticDownloading || semanticEnabling}
            onchange={(e) =>
              void semanticToggle((e.currentTarget as HTMLInputElement).checked)}
          />
          <span>
            Enable semantic search (Hybrid mode)
          </span>
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
          <span class="v mono" title="Shared across drives">{semanticState.model_path}</span>
        </div>
        {#if semanticError}
          <p class="hint err" role="alert">{semanticError}</p>
        {/if}
      {/if}
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
  .v .ok { color: var(--accent); }
  .hint.err { color: #d33; }
  /* `fullstack-a-21`: re-use the theme-opt chip shape for the
     semantic-search toggle so it visually matches the rest of
     the Settings chips. The checkbox is a distinct input shape
     (vs the radios theme-opt was built for), so a few resets
     undo the generic `input { width: 100% }` rule above. */
  .semantic-toggle {
    margin-bottom: 0.5rem;
  }
  .semantic-toggle input[type="checkbox"] {
    width: auto;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
  }
  .semantic-toggle input[type="checkbox"]:disabled,
  .semantic-toggle:has(input[type="checkbox"]:disabled) {
    cursor: not-allowed;
    opacity: 0.7;
  }
  .semantic-info {
    margin-top: 0.5rem;
    font-size: 13px;
  }
  .spinner {
    display: inline-block;
    width: 0.85em;
    height: 0.85em;
    margin-right: 0.25em;
    vertical-align: -0.1em;
    border: 2px solid var(--border);
    border-top-color: var(--link);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  @media (prefers-reduced-motion: reduce) {
    .spinner { animation: none; border-top-color: var(--border); }
  }
  /* `fullstack-b-11`: Terminal section layout. The generic
     `label { display: grid }` rule above forces a two-column grid;
     undo it for the terminal-label so the field label sits flush
     above its control row, and let the range + number input share
     a single inline row. */
  .terminal-section { gap: 0.75rem; }
  .terminal-field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .terminal-field + .terminal-field { margin-top: 0.5rem; }
  .terminal-label {
    display: block;
    grid-template-columns: none;
    font-size: 14px;
    color: var(--text-secondary);
  }
  .terminal-control {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
  }
  .terminal-control select,
  .terminal-control input[type="text"] {
    width: auto;
    min-width: 16em;
  }
  .scrollback-control input[type="range"] {
    flex: 1;
    width: auto;
    padding: 0;
    border: 0;
    background: transparent;
  }
  .scrollback-control .scrollback-number {
    width: 5em;
    text-align: right;
  }
  .terminal-unit {
    color: var(--text-secondary);
    font-size: 13px;
    min-width: 1.8em;
  }
  .terminal-field .sub-hint {
    margin: 0;
    font-size: 11.5px;
  }
  /* Tab-bar autosave indicator. Sits between the title and the
     actions strip. Empty when idle (no extra padding). */
  .save-status { font-size: 14px; min-width: 60px; text-align: right; }
  .save-status .ok { color: var(--accent); }
  .save-status .err { color: #d33; }
  .save-status .muted { color: var(--text-secondary); }
  @media (max-width: 760px) {
    .section-row {
      grid-template-columns: 1fr;
    }
  }
</style>
