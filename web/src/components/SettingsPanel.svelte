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
    BuildInfo,
    EditorTheme,
    GlobalConfig,
    LlmModelEntry,
    LlmStatus,
    Preferences,
  } from "../api/types";
  import { Maximize2, Minimize2 } from "lucide-svelte";
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
  import HamburgerMenu from "./HamburgerMenu.svelte";
  import OverlayShell from "./OverlayShell.svelte";

  let menu: HamburgerMenu | undefined = $state();
  let menuOpen = $state(false);
  const POPOVER_WIDTH = 200;
  const POPOVER_HEIGHT = 50;

  function doToggleOverlayMaximized(): void {
    setOverlayMaximized(!overlayMaximized.on);
    menu?.close();
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
  // on mount + after every save so changing the backend in the
  // form re-reads readiness.
  let llmStatus = $state<LlmStatus | null>(null);
  /// Curated model shortlists for the local CLIs (`claude_cli` /
  /// `gemini_cli`). Neither CLI exposes `--list-models`; both
  /// accept the API-namespace model names verbatim, and `claude`
  /// additionally accepts the short aliases `opus` / `sonnet` /
  /// `haiku` that always resolve to the current latest of each
  /// tier (per `claude --help`). The aliases are the recommended
  /// default — they survive a model bump without a config edit —
  /// so they sit at the top of the list; the pinned full names
  /// follow for users who want to lock to a specific generation.
  const CLAUDE_CLI_MODELS = [
    "opus",
    "sonnet",
    "haiku",
    "claude-opus-4-7",
    "claude-sonnet-4-6",
    "claude-haiku-4-5",
  ];
  const GEMINI_CLI_MODELS = ["gemini-2.5-pro", "gemini-2.5-flash"];

  // Per-provider model catalogs for the dropdowns. Each provider
  // tracks loading state so the refresh button can show a spinner
  // and an error string so a failed live fetch surfaces inline
  // (mostly for Ollama, but Anthropic now also reports when the
  // live `/v1/models` call falls back to the curated list).
  let anthropicModels = $state<LlmModelEntry[]>([]);
  let anthropicSource = $state<"live" | "curated" | "fallback" | null>(null);
  let anthropicError = $state<string | null>(null);
  let anthropicLoading = $state(false);
  let geminiModels = $state<LlmModelEntry[]>([]);
  let geminiSource = $state<"live" | "curated" | "fallback" | null>(null);
  let geminiError = $state<string | null>(null);
  let geminiLoading = $state(false);
  let ollamaModels = $state<LlmModelEntry[]>([]);
  let ollamaLoading = $state(false);
  let ollamaError = $state<string | null>(null);

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
  let keychainInput = $state("");
  let keychainBusy = $state(false);
  let keychainError = $state<string | null>(null);
  // Whether the active key currently comes from the keychain;
  // derived from `llmStatus.key.source`, so we don't need a
  // separate probe.
  const keychainHas = $derived(llmStatus?.key.source === "keychain");
  const keychainAvailable = $derived(llmStatus?.key.keychain_available === true);
  // Hide the keychain UI when an env-var override is in effect:
  // env always wins, so storing in the keychain wouldn't change
  // anything until the env is cleared; rather than show a
  // confusingly-inert button we just point the user at the env
  // override that's currently winning.
  const envOverride = $derived(llmStatus?.key.source === "env");

  /// Which provider's keychain entry the keychain UI block is
  /// targeting. Derived from the active backend; `null` for
  /// backends that don't use a hosted-API key (Ollama, Embedded).
  /// The save/remove functions dispatch on this so the same UI
  /// block serves both Claude and Gemini.
  type KeychainProvider = "anthropic" | "gemini";

  function keychainProviderForBackend(): KeychainProvider | null {
    if (!editing) return null;
    if (editing.assistant.backend === "claude") return "anthropic";
    if (editing.assistant.backend === "gemini") return "gemini";
    return null;
  }

  async function saveKeychain(): Promise<void> {
    const v = keychainInput.trim();
    if (!v || keychainBusy) return;
    const provider = keychainProviderForBackend();
    if (!provider) return;
    keychainBusy = true;
    keychainError = null;
    try {
      // Saving a key is an implicit "I want to use this backend".
      // Commit that intent BEFORE we hit the keychain so /api/llm/
      // status reports ready=true on the immediate refetch. The
      // previous version only flushed pending autosaves, which
      // didn't help when the form was clean (e.g. fresh Settings
      // open: backend dropdown shows "Claude" but cfg.backend on
      // the server is still None until the user touches anything).
      if (editing) {
        editing.assistant.enabled = true;
        editing.assistant.backend = provider === "anthropic" ? "claude" : "gemini";
      }
      if (autosaveTimer) {
        clearTimeout(autosaveTimer);
        autosaveTimer = null;
      }
      // Always save: the implicit edits above may not register as
      // dirty if the user previously saw the same backend selected
      // in editing (e.g. they're refreshing the key on an already-
      // configured backend). save() short-circuits cleanly when
      // there's nothing to send.
      await save();
      // Server verifies the round trip (write then read-back) before
      // returning 204; on a read-back failure it surfaces a precise
      // error here in the catch arm. So the input only clears on a
      // verified-good save, and a stuck "saving…" never happens.
      if (provider === "anthropic") {
        await api.setAnthropicKey(v);
      } else {
        await api.setGeminiKey(v);
      }
      keychainInput = "";
      // Re-pull status so the source flips to "keychain" and the
      // readiness pill picks up the new key.
      await loadLlmStatus();
    } catch (e) {
      keychainError = (e as Error).message ?? String(e);
    } finally {
      keychainBusy = false;
    }
  }

  async function removeKeychain(): Promise<void> {
    if (keychainBusy) return;
    const provider = keychainProviderForBackend();
    if (!provider) return;
    keychainBusy = true;
    keychainError = null;
    try {
      if (provider === "anthropic") {
        await api.clearAnthropicKey();
      } else {
        await api.clearGeminiKey();
      }
      await loadLlmStatus();
    } catch (e) {
      keychainError = (e as Error).message ?? String(e);
    } finally {
      keychainBusy = false;
    }
  }

  async function loadLlmStatus(): Promise<void> {
    try {
      llmStatus = await api.llmStatus();
    } catch {
      llmStatus = null;
    }
  }

  /// Pull the Anthropic model list. Server returns the live
  /// `/v1/models` catalog when an API key is configured and the
  /// curated fallback when it isn't (or when the live fetch
  /// fails). Refresh button always reloads.
  async function refreshAnthropicModels(): Promise<void> {
    anthropicLoading = true;
    anthropicError = null;
    try {
      const resp = await api.anthropicModels();
      anthropicModels = resp.models;
      anthropicSource = resp.source;
      if (resp.source === "fallback") {
        anthropicError = resp.error ?? "live fetch failed";
      }
      autoPickModel("claude");
    } catch (e) {
      anthropicError = (e as Error).message;
      anthropicModels = [];
      anthropicSource = null;
    } finally {
      anthropicLoading = false;
    }
  }

  /// Pull the Gemini model list. Mirrors refreshAnthropicModels:
  /// live `/v1beta/models` when a key is configured, curated
  /// fallback otherwise; the refresh button always reloads.
  async function refreshGeminiModels(): Promise<void> {
    geminiLoading = true;
    geminiError = null;
    try {
      const resp = await api.geminiModels();
      geminiModels = resp.models;
      geminiSource = resp.source;
      if (resp.source === "fallback") {
        geminiError = resp.error ?? "live fetch failed";
      }
      autoPickModel("gemini");
    } catch (e) {
      geminiError = (e as Error).message;
      geminiModels = [];
      geminiSource = null;
    } finally {
      geminiLoading = false;
    }
  }

  /// Pull the Ollama model list. Used both on first switch to the
  /// ollama backend AND from the refresh button so the user can
  /// re-query after `ollama pull`-ing a new model.
  async function refreshOllamaModels(): Promise<void> {
    if (!editing) return;
    ollamaLoading = true;
    ollamaError = null;
    try {
      ollamaModels = await api.ollamaModels(
        editing.assistant.ollama.url || undefined,
      );
      autoPickModel("ollama");
    } catch (e) {
      ollamaError = (e as Error).message;
      ollamaModels = [];
    } finally {
      ollamaLoading = false;
    }
  }

  /// When the model catalog refreshes, pick the first available model
  /// for the user if they don't already have a valid choice. Avoids
  /// the "dropdown shows blank because the saved model isn't in the
  /// returned catalog" footgun and gives first-launch users something
  /// usable to pick from. Triggers an autosave so the choice persists.
  function autoPickModel(provider: "claude" | "gemini" | "ollama"): void {
    if (!editing) return;
    if (provider === "claude") {
      if (anthropicModels.length === 0) return;
      const cur = editing.assistant.claude.model;
      if (!cur || !anthropicModels.some((m) => m.name === cur)) {
        editing.assistant.claude.model = anthropicModels[0].name;
      }
    } else if (provider === "gemini") {
      if (geminiModels.length === 0) return;
      const cur = editing.assistant.gemini.model;
      if (!cur || !geminiModels.some((m) => m.name === cur)) {
        editing.assistant.gemini.model = geminiModels[0].name;
      }
    } else {
      if (ollamaModels.length === 0) return;
      const cur = editing.assistant.ollama.model;
      if (!cur || !ollamaModels.some((m) => m.name === cur)) {
        editing.assistant.ollama.model = ollamaModels[0].name;
      }
    }
  }

  /// Backend-dropdown change handler. Lazy-loads the right model
  /// catalog so we don't hit Ollama until the user actually picks
  /// it (the call would fail with no Ollama running otherwise).
  /// Also wipes the keychain input so a key the user typed for
  /// Anthropic doesn't accidentally land in the Gemini slot (or
  /// vice versa) on a subsequent save.
  async function onBackendChange(): Promise<void> {
    if (!editing) return;
    keychainInput = "";
    keychainError = null;
    if (editing.assistant.backend === "claude") {
      if (anthropicModels.length === 0) await refreshAnthropicModels();
    } else if (editing.assistant.backend === "gemini") {
      if (geminiModels.length === 0) await refreshGeminiModels();
    } else if (editing.assistant.backend === "ollama") {
      if (ollamaModels.length === 0) await refreshOllamaModels();
    }
    // Server's /api/llm/status dispatches its `key` payload on the
    // active backend, so re-pull so the readiness/key pill matches
    // the new selection without waiting for the autosave round-trip.
    void loadLlmStatus();
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

  /// Parse a max-output-tokens text input into Option<u32>. Empty
  /// string or non-positive numbers clear the override (chan-llm
  /// then falls back to its per-backend default). Caps the value at
  /// u32::MAX so the autosave can't ship a number the server can't
  /// deserialize.
  function parseMaxTokens(raw: string): number | null {
    const t = raw.trim();
    if (t === "") return null;
    const n = Number(t);
    if (!Number.isFinite(n) || n <= 0) return null;
    const i = Math.floor(n);
    return i > 0xffff_ffff ? 0xffff_ffff : i;
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
      // Backend / model may have flipped; re-check readiness.
      void loadLlmStatus();
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
    void loadBuildInfo();
    // Catalog warmup happens via the effect below, gated on both
    // `editing` and the scope-source being loaded so that
    // autoPickModel's mutation reliably appears as dirty and the
    // autosave round-trip persists it.
  });

  // Pre-populate the model catalog matching the currently-selected
  // backend, but wait until both `editing` AND the source-of-truth
  // for the active scope are available. Without that gate, autoPick
  // racing ahead of loadGlobalConfig leaves dirty() returning false
  // (scopeSource is undefined; see dirty() above) and the autosave
  // never fires, so the dropdown briefly flashes a model and then
  // appears unset again on the next mount because it was never
  // persisted.
  let catalogWarmedUp = false;
  $effect(() => {
    if (catalogWarmedUp) return;
    if (!editing || !globalConfig) return;
    catalogWarmedUp = true;
    void onBackendChange();
  });
</script>

<OverlayShell id="settings" open={visible} onClose={close}>
<div class="settings-tab">
  <div class="tab-bar">
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
    <HamburgerMenu
      bind:this={menu}
      bind:open={menuOpen}
      width={POPOVER_WIDTH}
      height={POPOVER_HEIGHT}
    >
      {@render settingsMenuItems()}
    </HamburgerMenu>
  </div>

  <div class="body">
{#if !editing || !drive.info}
  <div class="placeholder">loading settings…</div>
{:else}
  <div class="settings">
    <section>
      <h3>Assistant</h3>

      <!-- 1. Master switch + tooltip. -->
      <label class="toggle-row">
        <input type="checkbox" bind:checked={editing.assistant.enabled} />
        <span>Enable assistant</span>
        <span class="hint-text">
          turn off to hide Cmd+P, the assistant button in the editor
          toolbar, and the search palette's "ask" tab
        </span>
      </label>

      {#if !editing.assistant.enabled}
        <!-- Visible cue that the disabled dropdowns below aren't a bug.
             Surfaces only when the master switch is off; otherwise the
             gate is invisible and reads as broken UI. -->
        <div class="muted gate-hint">enable above to configure provider, model, and key</div>
      {/if}

      <!-- 2. Provider. -->
      <label class:dim={!editing.assistant.enabled}>
        <span>Provider</span>
        <select
          bind:value={editing.assistant.backend}
          onchange={() => void onBackendChange()}
          disabled={!editing.assistant.enabled}
        >
          <option value="claude_cli">claude CLI (shell-executor)</option>
          <option value="gemini_cli">gemini CLI (shell-executor)</option>
          <option value="claude">claude (Anthropic API)</option>
          <option value="gemini">gemini (Google API)</option>
          <option value="embedded" disabled>local: coming soon (embedded model)</option>
          <option value="ollama">ollama (local)</option>
        </select>
      </label>

      <!-- For Ollama, surface the URL above the model picker
           because the URL drives which catalog the picker reads. -->
      {#if editing.assistant.backend === "ollama"}
        <label class:dim={!editing.assistant.enabled}>
          <span>URL</span>
          <input
            bind:value={editing.assistant.ollama.url}
            placeholder="http://localhost:11434"
            disabled={!editing.assistant.enabled}
          />
        </label>
      {/if}

      <!-- 3. Model + always-visible reload. The reload button
           greys out when it can't do anything useful (refreshing
           a curated list when no key is set, or while a previous
           refresh is in flight). Same .model-row shape for both
           providers so the layout doesn't shift on backend
           change. -->
      {#if editing.assistant.backend === "claude"}
        <label class:dim={!editing.assistant.enabled}>
          <span>Model</span>
          <span class="model-row">
            <select
              bind:value={editing.assistant.claude.model}
              disabled={!editing.assistant.enabled}
            >
              <!-- The null-valued "default" placeholder only renders
                   while the catalog is empty (initial mount, refresh
                   failed). Once the catalog has loaded, autoPickModel
                   has selected a real entry, and keeping the
                   placeholder around with the same display text as a
                   real catalog entry causes the select to render
                   blank when the user picks it (Svelte's bind:value
                   resolution between null and a colliding string
                   value gets confused). Dropping it post-load also
                   simplifies the menu to just real models. -->
              {#if anthropicModels.length === 0}
                <option value={null}>claude-haiku-4-5 (default)</option>
              {/if}
              {#each anthropicModels as m (m.name)}
                <option value={m.name}>
                  {m.name}{m.supports_tools ? "" : "  (no tools)"}
                </option>
              {/each}
            </select>
            <button
              type="button"
              class="refresh"
              onclick={() => void refreshAnthropicModels()}
              disabled={anthropicLoading || !editing.assistant.enabled || !llmStatus?.key.set}
              title={!llmStatus?.key.set
                ? "set ANTHROPIC_API_KEY to fetch the live model list"
                : "re-query Anthropic for the model list"}
            >{anthropicLoading ? "…" : "↻"}</button>
          </span>
        </label>
        <label class:dim={!editing.assistant.enabled}>
          <span>Max output tokens</span>
          <input
            type="number"
            min="1"
            step="1"
            placeholder="4096 (default)"
            value={editing.assistant.claude.max_tokens ?? ""}
            oninput={(e) => {
              if (!editing) return;
              editing.assistant.claude.max_tokens = parseMaxTokens(
                (e.currentTarget as HTMLInputElement).value,
              );
            }}
            disabled={!editing.assistant.enabled}
          />
        </label>
        {#if anthropicSource === "fallback" && anthropicError}
          <div class="muted err-line">Anthropic: {anthropicError} (showing curated list)</div>
        {/if}
        {#if !llmStatus?.key.set}
          <div class="muted err-line">
            {#if keychainAvailable}
              Save your Anthropic API key in the OS keychain below, or
              export <code class="mono">ANTHROPIC_API_KEY</code> before
              launching chan.
            {:else}
              export <code class="mono">ANTHROPIC_API_KEY</code> or write
              <code class="mono">{llmStatus?.key.path ?? "~/.config/chan/api-keys.toml"}</code>
              with <code class="mono">[anthropic] api_key = "..."</code>
            {/if}
          </div>
        {/if}
        {#if keychainAvailable && !envOverride}
          <!-- Store the key in the OS keychain (macOS Keychain,
               Windows Credential Manager, Linux Secret Service).
               Hidden when ANTHROPIC_API_KEY is set in env: an env
               override always wins, so the keychain controls
               would be silently inert. Hidden also on headless
               boxes where the keychain backend isn't reachable
               (typical for `chan serve` over SSH). -->
          <div class="keychain">
            <div class="keychain-label">
              {#if keychainHas}
                <span class="ok">●</span> stored in this machine's keychain
              {:else}
                <span class="muted">no key in this machine's keychain</span>
              {/if}
            </div>
            {#if keychainHas}
              <button
                type="button"
                onclick={() => void removeKeychain()}
                disabled={keychainBusy}
                title="remove the stored key"
              >{keychainBusy ? "removing…" : "remove from keychain"}</button>
            {:else}
              <input
                type="password"
                placeholder="sk-ant-..."
                bind:value={keychainInput}
                onkeydown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    void saveKeychain();
                  }
                }}
                spellcheck="false"
                autocomplete="off"
              />
              <button
                type="button"
                onclick={() => void saveKeychain()}
                disabled={keychainBusy || !keychainInput.trim()}
              >{keychainBusy ? "saving…" : "save in keychain"}</button>
            {/if}
            {#if keychainError}
              <span class="err keychain-err">{keychainError}</span>
            {/if}
          </div>
        {:else if envOverride}
          <div class="muted err-line">
            ANTHROPIC_API_KEY is set in env and always wins; unset it to
            manage the key via keychain instead.
          </div>
        {/if}
      {:else if editing.assistant.backend === "gemini"}
        <label class:dim={!editing.assistant.enabled}>
          <span>Model</span>
          <span class="model-row">
            <select
              bind:value={editing.assistant.gemini.model}
              disabled={!editing.assistant.enabled}
            >
              {#if geminiModels.length === 0}
                <option value={null}>gemini-2.5-flash (default)</option>
              {/if}
              {#each geminiModels as m (m.name)}
                <option value={m.name}>
                  {m.name}{m.supports_tools ? "" : "  (no tools)"}
                </option>
              {/each}
            </select>
            <button
              type="button"
              class="refresh"
              onclick={() => void refreshGeminiModels()}
              disabled={geminiLoading || !editing.assistant.enabled || !llmStatus?.key.set}
              title={!llmStatus?.key.set
                ? "set GEMINI_API_KEY to fetch the live model list"
                : "re-query Google for the model list"}
            >{geminiLoading ? "…" : "↻"}</button>
          </span>
        </label>
        <label class:dim={!editing.assistant.enabled}>
          <span>Max output tokens</span>
          <input
            type="number"
            min="1"
            step="1"
            placeholder="4096 (default)"
            value={editing.assistant.gemini.max_tokens ?? ""}
            oninput={(e) => {
              if (!editing) return;
              editing.assistant.gemini.max_tokens = parseMaxTokens(
                (e.currentTarget as HTMLInputElement).value,
              );
            }}
            disabled={!editing.assistant.enabled}
          />
        </label>
        {#if geminiSource === "fallback" && geminiError}
          <div class="muted err-line">Gemini: {geminiError} (showing curated list)</div>
        {/if}
        {#if !llmStatus?.key.set}
          <div class="muted err-line">
            {#if keychainAvailable}
              Save your Google AI Studio API key in the OS keychain
              below, or export
              <code class="mono">GEMINI_API_KEY</code> before launching
              chan.
            {:else}
              export <code class="mono">GEMINI_API_KEY</code> or write
              <code class="mono">{llmStatus?.key.path ?? "~/.config/chan/api-keys.toml"}</code>
              with <code class="mono">[gemini] api_key = "..."</code>
            {/if}
          </div>
        {/if}
        {#if keychainAvailable && !envOverride}
          <div class="keychain">
            <div class="keychain-label">
              {#if keychainHas}
                <span class="ok">●</span> stored in this machine's keychain
              {:else}
                <span class="muted">no key in this machine's keychain</span>
              {/if}
            </div>
            {#if keychainHas}
              <button
                type="button"
                onclick={() => void removeKeychain()}
                disabled={keychainBusy}
                title="remove the stored key"
              >{keychainBusy ? "removing…" : "remove from keychain"}</button>
            {:else}
              <input
                type="password"
                placeholder="AIza..."
                bind:value={keychainInput}
                onkeydown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    void saveKeychain();
                  }
                }}
                spellcheck="false"
                autocomplete="off"
              />
              <button
                type="button"
                onclick={() => void saveKeychain()}
                disabled={keychainBusy || !keychainInput.trim()}
              >{keychainBusy ? "saving…" : "save in keychain"}</button>
            {/if}
            {#if keychainError}
              <span class="err keychain-err">{keychainError}</span>
            {/if}
          </div>
        {:else if envOverride}
          <div class="muted err-line">
            GEMINI_API_KEY is set in env and always wins; unset it to
            manage the key via keychain instead.
          </div>
        {/if}
      {:else if editing.assistant.backend === "ollama"}
        <label class:dim={!editing.assistant.enabled}>
          <span>Model</span>
          <span class="model-row">
            <select
              bind:value={editing.assistant.ollama.model}
              disabled={ollamaLoading || !editing.assistant.enabled}
            >
              {#if ollamaModels.length === 0}
                <option value={null}>(refresh to load)</option>
              {/if}
              {#each ollamaModels as m (m.name)}
                <option value={m.name}>
                  {m.name}{m.supports_tools ? "" : "  (no tools)"}
                </option>
              {/each}
            </select>
            <button
              type="button"
              class="refresh"
              onclick={() => void refreshOllamaModels()}
              disabled={ollamaLoading || !editing.assistant.enabled}
              title="re-query Ollama for installed models"
            >{ollamaLoading ? "…" : "↻"}</button>
          </span>
        </label>
        <label class:dim={!editing.assistant.enabled}>
          <span>Max output tokens</span>
          <input
            type="number"
            min="1"
            step="1"
            placeholder="uncapped (default)"
            value={editing.assistant.ollama.max_tokens ?? ""}
            oninput={(e) => {
              if (!editing) return;
              editing.assistant.ollama.max_tokens = parseMaxTokens(
                (e.currentTarget as HTMLInputElement).value,
              );
            }}
            disabled={!editing.assistant.enabled}
          />
        </label>
        {#if ollamaError}
          <div class="muted err-line">Ollama: {ollamaError}</div>
        {/if}
      {:else if editing.assistant.backend === "claude_cli"}
        <!-- The local `claude` CLI doesn't expose a programmatic
             catalog and its accepted `--model` aliases are a stable,
             short set. Use a hardcoded curated list rather than the
             Anthropic API catalog: the live API list includes
             versioned aliases (e.g. claude-opus-3-5-20240620) that
             aren't always valid CLI inputs, and refreshing the API
             catalog shouldn't change CLI options. "(use CLI default)"
             clears the override so `claude config`'s pick wins. -->
        <label class:dim={!editing.assistant.enabled}>
          <span>Model</span>
          <select
            bind:value={editing.assistant.claude_cli.model}
            disabled={!editing.assistant.enabled}
          >
            <option value={null}>(use CLI default)</option>
            {#each CLAUDE_CLI_MODELS as name (name)}
              <option value={name}>{name}</option>
            {/each}
          </select>
        </label>
        <div class="muted err-line">
          Auth + max-tokens for the claude CLI are managed by the CLI
          itself (run <code class="mono">claude config</code>). The
          model picker above maps to <code class="mono">--model</code>
          on the CLI invocation; "(use CLI default)" passes no flag.
        </div>
      {:else if editing.assistant.backend === "gemini_cli"}
        <!-- Same shape as claude_cli, against a hardcoded Gemini-CLI
             shortlist. -->
        <label class:dim={!editing.assistant.enabled}>
          <span>Model</span>
          <select
            bind:value={editing.assistant.gemini_cli.model}
            disabled={!editing.assistant.enabled}
          >
            <option value={null}>(use CLI default)</option>
            {#each GEMINI_CLI_MODELS as name (name)}
              <option value={name}>{name}</option>
            {/each}
          </select>
        </label>
        <div class="muted err-line">
          Auth + max-tokens for the gemini CLI are managed by the CLI
          itself (run <code class="mono">gemini config</code>). The
          model picker above maps to <code class="mono">--model</code>;
          "(use CLI default)" passes no flag.
        </div>
      {/if}

      <!-- Auto-apply moved out of Settings: the toggle now lives in
           the chat composer (next to Send), so it can be flipped
           per turn without leaving the conversation. The persisted
           pref still rides on the backend's editor TOML as the
           default the toggle initializes from. -->

      <!-- 5 + 6. Status block at the bottom: ready/not-ready
           pill, then the tools row with a longer explanation when
           the current model can't use them. Sitting at the bottom
           keeps it close to the controls that affect it (toggling
           the master switch, picking a provider/model) so the user
           sees the immediate consequence. -->
      <div class="grid">
        <span class="k">status</span>
        <span class="v">
          {#if llmStatus?.ready}
            <span class="ok">ready</span>
          {:else}
            <span class="err">
              not ready{llmStatus?.reason ? `: ${llmStatus.reason}` : ""}
            </span>
          {/if}
        </span>
        <span class="k">tools</span>
        <span class="v">
          {#if !llmStatus?.ready}
            <!-- The status row above already names the readiness
                 problem. Don't restate the tool capability here:
                 it's a per-model property the user can't act on
                 until the assistant is ready. The em dash signals
                 "n/a" without taking the green/red color. -->
            <span class="muted">—</span>
          {:else if llmStatus?.supports_tools}
            <span class="ok">supported</span>
          {:else}
            <span class="err">not supported</span>
            <div class="hint-text">
              the current model can chat but can't read other files,
              search the drive, or propose file edits; pick a
              tool-capable model to enable those
            </div>
          {/if}
        </span>
      </div>
    </section>

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
              onchange={() => setThemeChoice(opt.value as ThemeChoice)}
            />
            <span>{opt.label}</span>
          </label>
        {/each}
      </div>
    </section>

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

    <section class="about">
      <h3>About</h3>
      <div class="grid">
        <span class="k">version</span>
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

{#snippet settingsMenuItems()}
  <li>
    <button role="menuitem" onclick={doToggleOverlayMaximized}>
      {#if overlayMaximized.on}
        <Minimize2 size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Restore size</span>
      {:else}
        <Maximize2 size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Maximize</span>
      {/if}
    </button>
  </li>
{/snippet}

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
