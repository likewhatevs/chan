<script lang="ts">
  // Settings overlay. Per-device-global preferences form (editor
  // theme, assistant, attachments_dir, default-drive path) plus the
  // keychain controls for the assistant API key.
  //
  // The drive display name is edited from the file-browser
  // hamburger, not here, so the settings overlay is purely
  // about device-wide preferences.
  //
  // Auto-saves on change (500 ms debounce). Keychain writes are a
  // separate flow with their own button because the keychain
  // backend is OS-specific and the operation must surface its own
  // pass/fail.

  import { onMount } from "svelte";
  import { api } from "../api/client";
  import type {
    AssistantBackendKind,
    BuildInfo,
    EditorTheme,
    GlobalConfig,
    LlmKeysStatus,
    LlmStatus,
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
  // LLM backend status (key set / not set, ready, etc.). Refreshed
  // on mount + after every save so changing the default backend
  // re-reads readiness. Reports for the currently-default backend;
  // the keychain UI below targets that same backend.
  let llmStatus = $state<LlmStatus | null>(null);

  /// Build identity for the About footer. Loaded on mount; the
  /// version + embeddings feature flag are static for the running
  /// binary so a single fetch is enough.
  let buildInfo = $state<BuildInfo | null>(null);

  // Keychain integration. Available on any machine where the OS
  // keychain backend is reachable: macOS Keychain, Windows
  // Credential Manager, Linux with a running Secret Service
  // daemon (gnome-keyring / KWallet). The /api/llm/status payload
  // reports `keychain_available` so we hide the UI on headless
  // boxes (a `chan serve` over SSH on a server with no GUI
  // session) where the user still has env / file as fallbacks.
  /// Per-row keychain UI state. Each provider has its own input,
  /// busy flag, and error string so two rows can be edited in parallel
  /// without crosstalk (e.g. typing into Claude's input shouldn't
  /// clear or block Gemini's). `keysStatus` is the source of truth
  /// for "is a key stored" rendered as a pill per row; it's a
  /// separate fetch from `llmStatus` so non-default rows can show
  /// status without needing to be the active backend.
  type KeychainProvider = "anthropic" | "gemini";

  let keychainInput = $state<Record<KeychainProvider, string>>({
    anthropic: "",
    gemini: "",
  });
  let keychainBusy = $state<Record<KeychainProvider, boolean>>({
    anthropic: false,
    gemini: false,
  });
  let keychainError = $state<Record<KeychainProvider, string | null>>({
    anthropic: null,
    gemini: null,
  });
  let keysStatus = $state<LlmKeysStatus | null>(null);
  // `keychain_available` lives on the active-backend status payload
  // because chan-llm doesn't (yet) probe the keychain backend
  // separately. Headless boxes (no Secret Service / DBus session)
  // surface that here so the row-level UI hides itself on the same
  // signal the old single-block UI used.
  const keychainAvailable = $derived(llmStatus?.key.keychain_available === true);

  async function loadKeysStatus(): Promise<void> {
    try {
      keysStatus = await api.llmKeysStatus();
    } catch {
      keysStatus = null;
    }
  }

  function keychainHasFor(p: KeychainProvider): boolean {
    return keysStatus?.[p].source === "keychain";
  }

  function envOverrideFor(p: KeychainProvider): boolean {
    return keysStatus?.[p].source === "env";
  }

  async function saveKeychain(provider: KeychainProvider): Promise<void> {
    const v = keychainInput[provider].trim();
    if (!v || keychainBusy[provider]) return;
    keychainBusy[provider] = true;
    keychainError[provider] = null;
    try {
      // Saving a key is an implicit "I want to use this provider":
      // enable the matching row and (if no default is set yet) claim
      // the default pointer too. We don't yank the default away from
      // another provider though — the user already picked it.
      if (editing) {
        if (provider === "anthropic") {
          editing.assistant.claude.enabled = true;
        } else {
          editing.assistant.gemini.enabled = true;
        }
        if (editing.assistant.default_backend === null) {
          editing.assistant.default_backend = provider === "anthropic" ? "claude" : "gemini";
        }
      }
      if (autosaveTimer) {
        clearTimeout(autosaveTimer);
        autosaveTimer = null;
      }
      // Flush the enable / default change before saving the key so a
      // status refetch immediately after picks up the matching state.
      await save();
      // Server verifies the round trip (write then read-back) before
      // returning 204; a read-back failure surfaces in the catch arm.
      // The input only clears on a verified-good save so a stuck
      // "saving…" never happens.
      if (provider === "anthropic") {
        await api.setAnthropicKey(v);
      } else {
        await api.setGeminiKey(v);
      }
      keychainInput[provider] = "";
      await Promise.all([loadLlmStatus(), loadKeysStatus()]);
    } catch (e) {
      keychainError[provider] = (e as Error).message ?? String(e);
    } finally {
      keychainBusy[provider] = false;
    }
  }

  async function removeKeychain(provider: KeychainProvider): Promise<void> {
    if (keychainBusy[provider]) return;
    keychainBusy[provider] = true;
    keychainError[provider] = null;
    try {
      if (provider === "anthropic") {
        await api.clearAnthropicKey();
      } else {
        await api.clearGeminiKey();
      }
      await Promise.all([loadLlmStatus(), loadKeysStatus()]);
    } catch (e) {
      keychainError[provider] = (e as Error).message ?? String(e);
    } finally {
      keychainBusy[provider] = false;
    }
  }

  async function loadLlmStatus(): Promise<void> {
    try {
      llmStatus = await api.llmStatus();
    } catch {
      llmStatus = null;
    }
  }

  /// Provider enable-toggle handler. Toggling on a row claims the
  /// default pointer when no default is currently set, so the
  /// "enabled but no default" half-state can't occur from the UI.
  /// Toggling off clears the default if this row was holding it,
  /// nudging the user to pick another (or accept "off") instead of
  /// leaving the assistant in an inert "default disabled" state.
  /// Also wipes the keychain input so a key typed for one provider
  /// can't accidentally land in another's slot on the next save.
  function onProviderToggle(kind: AssistantBackendKind): void {
    if (!editing) return;
    keychainInput = { anthropic: "", gemini: "" };
    keychainError = { anthropic: null, gemini: null };
    const a = editing.assistant;
    const row = providerEnabledField(a, kind);
    if (row === null) return;
    if (row) {
      if (a.default_backend === null) {
        a.default_backend = kind;
      }
    } else if (a.default_backend === kind) {
      a.default_backend = null;
    }
    void loadLlmStatus();
  }

  /// Default-radio handler. Auto-enables the row if the user picks a
  /// disabled provider as default; otherwise the choice would be
  /// inert (the resolver gates on `enabled[default]`).
  function onDefaultChange(kind: AssistantBackendKind): void {
    if (!editing) return;
    keychainInput = { anthropic: "", gemini: "" };
    keychainError = { anthropic: null, gemini: null };
    const a = editing.assistant;
    a.default_backend = kind;
    setProviderEnabled(a, kind, true);
    void loadLlmStatus();
  }

  /// Read the per-provider `enabled` flag without a five-way switch
  /// at every call site. Returns null for unknown / placeholder
  /// kinds ("embedded") so toggles on those are a no-op.
  function providerEnabledField(
    a: Preferences["assistant"],
    kind: AssistantBackendKind,
  ): boolean | null {
    switch (kind) {
      case "claude":
        return a.claude.enabled;
      case "gemini":
        return a.gemini.enabled;
      case "ollama":
        return a.ollama.enabled;
      case "claude_cli":
        return a.claude_cli.enabled;
      case "gemini_cli":
        return a.gemini_cli.enabled;
      default:
        return null;
    }
  }

  function setProviderEnabled(
    a: Preferences["assistant"],
    kind: AssistantBackendKind,
    value: boolean,
  ): void {
    switch (kind) {
      case "claude":
        a.claude.enabled = value;
        return;
      case "gemini":
        a.gemini.enabled = value;
        return;
      case "ollama":
        a.ollama.enabled = value;
        return;
      case "claude_cli":
        a.claude_cli.enabled = value;
        return;
      case "gemini_cli":
        a.gemini_cli.enabled = value;
        return;
    }
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
    if (a.claude_cli === undefined) a.claude_cli = { model: null };
    if (a.gemini_cli === undefined) a.gemini_cli = { model: null };
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
      // Backend / model may have flipped; re-check readiness for the
      // current default AND refresh the per-row key statuses so each
      // row's pill matches the new server state.
      void loadLlmStatus();
      void loadKeysStatus();
      saveStatus = "saved";
      savedFlashTimer = setTimeout(() => {
        if (saveStatus === "saved") saveStatus = "idle";
        savedFlashTimer = null;
      }, SAVED_FLASH_MS);
    } catch (e) {
      saveStatus = { error: (e as Error).message };
    } finally {
      inflight = false;
      // If the form went dirty again while saving, schedule another pass.
      if (dirty()) scheduleSave();
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
    JSON.stringify(editing);
    if (!dirty()) return;
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
    void loadLlmStatus();
    void loadKeysStatus();
    void loadBuildInfo();
  });

  /// Friendly labels used in the provider list. Centralized so the
  /// dropdown ordering and the row ordering stay consistent across
  /// the markup below.
  const PROVIDER_ROWS: { kind: AssistantBackendKind; label: string; hint: string }[] = [
    { kind: "claude", label: "Claude", hint: "Anthropic API" },
    { kind: "gemini", label: "Gemini", hint: "Google API" },
    { kind: "ollama", label: "Ollama", hint: "local server" },
    { kind: "claude_cli", label: "Claude CLI", hint: "local `claude` shell-executor" },
    { kind: "gemini_cli", label: "Gemini CLI", hint: "local `gemini` shell-executor" },
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
      <h3>Assistant</h3>
      <p class="hint">
        Configure one or more assistants below. Toggle <strong>enabled</strong>
        per row to keep credentials around while picking which providers
        are usable; pick a <strong>default</strong> for new scopes. Model,
        max tokens, and other per-turn knobs live in the assistant
        overlay's inspector — open the assistant (Cmd+I) and click the
        inspector toggle.
      </p>

      {#if editing.assistant.default_backend === null}
        <div class="muted gate-hint">
          no default assistant picked yet: enable a row below and mark
          it as default to surface the assistant button + Cmd+I.
        </div>
      {:else if !editing.assistant.effective_enabled}
        <div class="muted gate-hint">
          default assistant is currently disabled: enable its row or
          pick a different default to make the assistant usable.
        </div>
      {/if}

      <div class="assistant-list">
        {#each PROVIDER_ROWS as row (row.kind)}
          {@const enabledNow = providerEnabledField(editing.assistant, row.kind) === true}
          {@const isDefault = editing.assistant.default_backend === row.kind}
          {@const isKeyProvider = row.kind === "claude" || row.kind === "gemini"}
          {@const keyProvider = (row.kind === "claude"
            ? "anthropic"
            : "gemini") as KeychainProvider}
          <div class="assistant-row" class:on={enabledNow}>
            <div class="row-head">
              <label class="row-toggle">
                <input
                  type="checkbox"
                  checked={enabledNow}
                  onchange={(e) => {
                    if (!editing) return;
                    const next = (e.currentTarget as HTMLInputElement).checked;
                    setProviderEnabled(editing.assistant, row.kind, next);
                    onProviderToggle(row.kind);
                  }}
                />
                <span class="provider-name">{row.label}</span>
                <span class="provider-hint">{row.hint}</span>
              </label>
              {#if enabledNow}
                <label class="default-radio">
                  <input
                    type="radio"
                    name="default-backend"
                    value={row.kind}
                    checked={isDefault}
                    onchange={() => onDefaultChange(row.kind)}
                  />
                  <span>default</span>
                </label>
              {/if}
            </div>

            {#if enabledNow}
              <div class="row-body">
                {#if row.kind === "ollama"}
                  <label class="row-field">
                    <span>URL</span>
                    <input
                      bind:value={editing.assistant.ollama.url}
                      placeholder="http://localhost:11434"
                    />
                  </label>
                {/if}

                {#if isKeyProvider && keychainAvailable && !envOverrideFor(keyProvider)}
                  <div class="keychain">
                    <div class="keychain-label">
                      {#if keychainHasFor(keyProvider)}
                        <span class="ok">●</span> key stored in this
                        machine's keychain
                      {:else if keysStatus?.[keyProvider].source === "file"}
                        <span class="ok">●</span> key stored in
                        <code class="mono">~/.config/chan/api-keys.toml</code>
                      {:else}
                        <span class="muted">no key configured</span>
                      {/if}
                    </div>
                    {#if keychainHasFor(keyProvider)}
                      <button
                        type="button"
                        onclick={() => void removeKeychain(keyProvider)}
                        disabled={keychainBusy[keyProvider]}
                        title="remove the stored key"
                      >{keychainBusy[keyProvider]
                        ? "removing…"
                        : "remove from keychain"}</button>
                    {:else}
                      <input
                        type="password"
                        placeholder={keyProvider === "anthropic"
                          ? "sk-ant-..."
                          : "AIza..."}
                        bind:value={keychainInput[keyProvider]}
                        onkeydown={(e) => {
                          if (e.key === "Enter") {
                            e.preventDefault();
                            void saveKeychain(keyProvider);
                          }
                        }}
                        spellcheck="false"
                        autocomplete="off"
                      />
                      <button
                        type="button"
                        onclick={() => void saveKeychain(keyProvider)}
                        disabled={keychainBusy[keyProvider] ||
                          !keychainInput[keyProvider].trim()}
                      >{keychainBusy[keyProvider]
                        ? "saving…"
                        : "save in keychain"}</button>
                    {/if}
                    {#if keychainError[keyProvider]}
                      <span class="err keychain-err">
                        {keychainError[keyProvider]}
                      </span>
                    {/if}
                  </div>
                {:else if isKeyProvider && envOverrideFor(keyProvider)}
                  <div class="muted err-line">
                    {keyProvider === "anthropic"
                      ? "ANTHROPIC_API_KEY"
                      : "GEMINI_API_KEY"} is set in env and always
                    wins; unset it to manage the key via keychain
                    instead.
                  </div>
                {:else if isKeyProvider && !keychainAvailable}
                  <div class="muted err-line">
                    export <code class="mono">
                      {keyProvider === "anthropic"
                        ? "ANTHROPIC_API_KEY"
                        : "GEMINI_API_KEY"}
                    </code> or write
                    <code class="mono">{llmStatus?.key.path ?? "~/.config/chan/api-keys.toml"}</code>
                    with <code class="mono">[{keyProvider}] api_key = "..."</code>
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      </div>

      <!-- Status block at the bottom: ready/not-ready pill for the
           current default + tools capability. Mirrors the old
           single-block status; sources from llmStatus (which
           dispatches on the active default). -->
      <div class="grid">
        <span class="k">status</span>
        <span class="v">
          {#if !editing.assistant.default_backend}
            <span class="muted">no default selected</span>
          {:else if !editing.assistant.effective_enabled}
            <span class="muted">default provider is disabled</span>
          {:else if llmStatus?.ready}
            <span class="ok">ready</span>
          {:else}
            <span class="err">
              not ready{llmStatus?.reason ? `: ${llmStatus.reason}` : ""}
            </span>
          {/if}
        </span>
        <span class="k">tools</span>
        <span class="v">
          {#if !llmStatus?.ready || !editing.assistant.effective_enabled}
            <span class="muted">—</span>
          {:else if llmStatus?.supports_tools}
            <span class="ok">supported</span>
          {:else}
            <span class="err">not supported</span>
            <div class="hint-text">
              the current model can chat but can't read other files,
              search the drive, or propose file edits; pick a
              tool-capable model from the assistant inspector
            </div>
          {/if}
        </span>
      </div>
    </section>

    <div class="section-row">
    <section>
      <h3>Editor theme</h3>
      <p class="hint">
        Style of the markdown editor only — typography, headings,
        code blocks, links, tables. Light and dark variants are
        picked from the appearance setting below.
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
        Tight matches Google Docs spacing for paragraphs and lists;
        standard keeps the older roomier layout. Line height drops
        too in tight mode so prose and bullets share the same cadence.
      </p>
      <!-- Reuses .theme-row / .theme-opt pill styles so this radio
           visually matches the Theme picker above. -->
      <div class="theme-row" role="radiogroup" aria-label="Line spacing">
        {#each [
          { value: "tight", label: "Tight" },
          { value: "standard", label: "Standard" },
        ] as opt (opt.value)}
          <label class="theme-opt" class:on={editing.line_spacing === opt.value}>
            <input
              type="radio"
              name="line-spacing"
              value={opt.value}
              checked={editing.line_spacing === opt.value}
              onchange={() => {
                editing!.line_spacing = opt.value as "tight" | "standard";
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
  .v .err { color: var(--warn-text); }
  /* Toggle row: checkbox + short label on one line, dimmed hint
     on the line below indented under the label. Replaces the old
     "auto-apply" layout where the helper text floated in the
     wide right column and read as misaligned. */
  .toggle-row {
    display: grid;
    grid-template-columns: 1.5em 1fr;
    grid-template-rows: auto auto;
    align-items: baseline;
    gap: 2px 0.5rem;
    margin: 0.25rem 0;
  }
  .toggle-row > input[type="checkbox"] {
    grid-row: 1;
    grid-column: 1;
  }
  .toggle-row > span:first-of-type {
    grid-row: 1;
    grid-column: 2;
    color: var(--text);
  }
  .toggle-row .hint-text {
    grid-row: 2;
    grid-column: 2;
    color: var(--text-secondary);
    font-size: 11.5px;
  }
  /* Inline cue under the master switch when the assistant is off, so
     the disabled provider/model rows below read as intentionally
     gated rather than broken. */
  .gate-hint {
    margin: 0 0 0.25rem 1.95em;
    font-size: 11.5px;
  }
  /* Dim a whole label row when the master switch gates it. The
     <select>/<input> inside is already `disabled`; without this the
     label text stays full-strength and the row reads as live. */
  label.dim > span { color: var(--text-secondary); opacity: 0.55; }
  label.dim input,
  label.dim select { opacity: 0.6; }
  /* Model dropdown + refresh button on one row. The button is
     inline-square with a rotation glyph; same look as the file
     browser's small action buttons. */
  .model-row { display: flex; gap: 4px; align-items: center; }
  .model-row select { flex: 1; }
  .model-row .refresh {
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    width: 28px;
    height: 28px;
    cursor: pointer;
    font-size: 15px;
  }
  .model-row .refresh:hover:not(:disabled) { border-color: var(--btn-hover); }
  .model-row .refresh:disabled { opacity: 0.55; cursor: default; }
  .err-line {
    color: var(--warn-text);
    font-size: 13px;
    margin: 0.25rem 0;
  }
  /* Assistant provider list. Each row is a card with a header (enable
     toggle + provider name + default radio) and a body that surfaces
     only when the row is enabled (URL field for Ollama, keychain UI
     for Claude/Gemini). The card boundary makes it visually clear
     that the per-row controls are scoped to that provider. */
  .assistant-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin: 0.4rem 0;
  }
  .assistant-row {
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.5rem 0.6rem;
    background: var(--bg);
  }
  .assistant-row.on {
    /* Faint accent border so enabled rows stand out from the disabled
       (configured but off) ones without being noisy. */
    border-color: var(--btn-border);
  }
  .row-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    justify-content: space-between;
  }
  .row-toggle {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    flex: 1;
    min-width: 0;
    cursor: pointer;
  }
  .row-toggle .provider-name {
    font-weight: 600;
  }
  .row-toggle .provider-hint {
    color: var(--text-secondary);
    font-size: 13px;
  }
  .default-radio {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--text-secondary);
    font-size: 13px;
    cursor: pointer;
  }
  .default-radio input[type="radio"] {
    margin: 0;
  }
  .row-body {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin-top: 0.5rem;
  }
  .row-field {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 14px;
  }
  .row-field > span {
    min-width: 60px;
    color: var(--text-secondary);
  }
  .row-field input[type="text"],
  .row-field input:not([type]) {
    flex: 1;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 4px 6px;
    font: inherit;
    font-size: 14px;
    outline: none;
  }
  .row-field input:focus { border-color: var(--link); }

  /* Keychain row: status label, password input (or remove button),
     primary action button. Wraps on narrow widths so the row doesn't
     overflow the form. */
  .keychain {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin: 0.4rem 0;
    font-size: 14px;
  }
  .keychain-label { flex-basis: 100%; color: var(--text-secondary); }
  .keychain-label .ok { color: var(--accent); }
  .keychain input[type="password"] {
    flex: 1;
    min-width: 200px;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 4px 6px;
    font: inherit;
    font-size: 14px;
    outline: none;
  }
  .keychain input[type="password"]:focus { border-color: var(--link); }
  .keychain button {
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 4px 10px;
    cursor: pointer;
    font: inherit;
    font-size: 14px;
  }
  .keychain button:hover:not(:disabled) { border-color: var(--btn-hover); }
  .keychain button:disabled { opacity: 0.55; cursor: default; }
  .keychain-err { flex-basis: 100%; color: var(--warn-text); font-size: 13px; }
  /* Tab-bar autosave indicator. Sits between the title and the
     actions strip. Empty when idle (no extra padding). */
  .save-status { font-size: 14px; min-width: 60px; text-align: right; }
  .save-status .ok { color: var(--accent); }
  .save-status .err { color: #d33; }
  .save-status .muted { color: var(--text-secondary); }
  section button {
    align-self: flex-start;
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 4px 12px;
    cursor: pointer;
    font: inherit;
  }
</style>
