<script lang="ts">
  // Right-side inspector pane for the assistant overlay. Mirrors the
  // FileInfo / Graph / Search inspector pattern: shows the per-turn
  // knobs that don't belong in Settings (they're picked per-task in
  // the conversation, not once per machine).
  //
  // Contents:
  //   1. Active assistant (dropdown of enabled providers from
  //      drive.info.preferences.assistant). Picking switches the
  //      default backend.
  //   2. Model (dropdown sourced from the live catalog for the
  //      picked provider; refresh button always re-queries).
  //   3. Max output tokens (numeric input; empty falls back to the
  //      backend default).
  //
  // Writes round-trip through `api.updateConfig` so the value sticks
  // across reloads. The Settings panel reads from the same source,
  // so opening it later shows whatever the inspector last persisted.

  import { onMount, untrack } from "svelte";
  import { api } from "../api/client";
  import type {
    AnthropicModelsResponse,
    AssistantBackendKind,
    AssistantPrefs,
    GeminiModelsResponse,
    GlobalConfig,
    LlmModelEntry,
    Preferences,
  } from "../api/types";
  import { drive } from "../state/store.svelte";

  /// Curated model shortlists for the local CLIs (`claude_cli` /
  /// `gemini_cli`). Same list the old Settings UI used. Aliases
  /// (`opus` / `sonnet` / `haiku`) sit at the top — they survive a
  /// model bump without a config edit. Pinned full names follow for
  /// users who want to lock to a generation.
  const CLAUDE_CLI_MODELS = [
    "opus",
    "sonnet",
    "haiku",
    "claude-opus-4-7",
    "claude-sonnet-4-6",
    "claude-haiku-4-5",
  ];
  const GEMINI_CLI_MODELS = ["gemini-2.5-pro", "gemini-2.5-flash"];

  /// Local editable mirror of `drive.info.preferences`. Mutations to
  /// the assistant subtree trigger a debounced PATCH (see save()
  /// below); outside changes (e.g. the Settings panel saving on the
  /// other side, or a fresh /api/drive after a server-side flip) re-
  /// sync into `editing` whenever the snapshot differs.
  let editing = $state<Preferences | null>(null);
  let lastSnap = "";

  /// Per-provider model catalogs. Loaded on demand: switching to a
  /// provider that hasn't been queried yet refreshes its catalog
  /// once. Stays in memory for the lifetime of the inspector mount.
  let anthropicModels = $state<LlmModelEntry[]>([]);
  let anthropicSource = $state<"live" | "curated" | "fallback" | null>(null);
  let anthropicLoading = $state(false);
  let anthropicError = $state<string | null>(null);

  let geminiModels = $state<LlmModelEntry[]>([]);
  let geminiSource = $state<"live" | "curated" | "fallback" | null>(null);
  let geminiLoading = $state(false);
  let geminiError = $state<string | null>(null);

  let ollamaModels = $state<LlmModelEntry[]>([]);
  let ollamaLoading = $state(false);
  let ollamaError = $state<string | null>(null);

  /// Save status surfaced under the inspector header. Mirrors
  /// SettingsPanel's status shape so the user sees the same idiom.
  type SaveStatus = "idle" | "saving" | "saved" | { error: string };
  let saveStatus = $state<SaveStatus>("idle");

  const AUTOSAVE_DELAY_MS = 350;
  let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  let savedFlashTimer: ReturnType<typeof setTimeout> | null = null;
  let inflight = false;

  /// Snapshot helper for the dirty-check + sync gate. Stringifying
  /// is good enough for these small payloads and avoids hand-rolling
  /// a structural compare per field.
  function snapOf(p: Preferences | null): string {
    if (!p) return "";
    // Only the assistant subtree is editable here; ignore the rest
    // so unrelated theme / pane-width updates don't trigger churn.
    return JSON.stringify(p.assistant);
  }

  /// Pull the latest preferences from drive.info into `editing`.
  /// Skipped when the form has unsaved local edits to avoid clobbering
  /// the user's typing.
  function syncFromServer(): void {
    const cur = drive.info?.preferences;
    if (!cur) return;
    const snap = snapOf(cur);
    if (snap === lastSnap) return;
    // Only resync when the form isn't dirty against the previous
    // server snapshot; otherwise the user's pending edits would be
    // wiped by a /api/drive refresh that crossed the save in flight.
    if (editing && snapOf(editing) !== lastSnap) return;
    editing = JSON.parse(JSON.stringify(cur));
    lastSnap = snap;
  }

  $effect(() => {
    // Read drive.info reactively; on any update, re-sync.
    void drive.info;
    syncFromServer();
  });

  /// Active provider derived from the persisted default. Falls back
  /// to the first enabled provider so a fresh scope (no default yet)
  /// still has the dropdown land on something usable.
  const activeProvider = $derived<AssistantBackendKind | null>(
    editing?.assistant.default_backend ?? firstEnabledProvider(),
  );

  function firstEnabledProvider(): AssistantBackendKind | null {
    const a = editing?.assistant;
    if (!a) return null;
    if (a.claude.enabled) return "claude";
    if (a.gemini.enabled) return "gemini";
    if (a.ollama.enabled) return "ollama";
    if (a.claude_cli.enabled) return "claude_cli";
    if (a.gemini_cli.enabled) return "gemini_cli";
    return null;
  }

  /// Enumerate enabled providers for the dropdown. Disabled
  /// providers stay out of the picker entirely (the row in Settings
  /// is the place to enable them); listing them here with a
  /// disabled attribute would surface a confusing "I can't pick this"
  /// option.
  const enabledList = $derived<{ kind: AssistantBackendKind; label: string }[]>(
    !editing
      ? []
      : (
          [
            { kind: "claude", label: "Claude", on: editing.assistant.claude.enabled },
            { kind: "gemini", label: "Gemini", on: editing.assistant.gemini.enabled },
            { kind: "ollama", label: "Ollama", on: editing.assistant.ollama.enabled },
            { kind: "claude_cli", label: "Claude CLI", on: editing.assistant.claude_cli.enabled },
            { kind: "gemini_cli", label: "Gemini CLI", on: editing.assistant.gemini_cli.enabled },
          ] as const
        )
          .filter((p) => p.on)
          .map((p) => ({ kind: p.kind as AssistantBackendKind, label: p.label })),
  );

  /// Pull the Anthropic / Gemini / Ollama catalogs on demand. Each
  /// refresh button hits the same endpoint the Settings UI used
  /// before its model picker moved here; "fallback" sources stamp
  /// an inline note so the user knows why a curated list is showing.
  async function refreshAnthropicModels(): Promise<void> {
    if (anthropicLoading) return;
    anthropicLoading = true;
    anthropicError = null;
    try {
      const resp: AnthropicModelsResponse = await api.anthropicModels();
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

  async function refreshGeminiModels(): Promise<void> {
    if (geminiLoading) return;
    geminiLoading = true;
    geminiError = null;
    try {
      const resp: GeminiModelsResponse = await api.geminiModels();
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

  async function refreshOllamaModels(): Promise<void> {
    if (!editing || ollamaLoading) return;
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

  /// When a catalog refresh lands, fill the matching model field
  /// with the first available entry if the user doesn't already
  /// have a valid pick. Same footgun-avoidance the old Settings UI
  /// had; keeps the dropdown from rendering blank when the saved
  /// model isn't in the catalog the API returned this session.
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

  /// Lazy catalog load: when the active provider switches, fetch its
  /// catalog if we haven't already. Untracked so the load doesn't
  /// retrigger on every catalog field mutation.
  $effect(() => {
    const kind = activeProvider;
    if (!kind) return;
    untrack(() => {
      if (kind === "claude" && anthropicModels.length === 0) {
        void refreshAnthropicModels();
      } else if (kind === "gemini" && geminiModels.length === 0) {
        void refreshGeminiModels();
      } else if (kind === "ollama" && ollamaModels.length === 0) {
        void refreshOllamaModels();
      }
    });
  });

  /// Parse a max-output-tokens text input into Option<u32>. Empty
  /// string / non-positive numbers clear the override.
  function parseMaxTokens(raw: string): number | null {
    const t = raw.trim();
    if (t === "") return null;
    const n = Number(t);
    if (!Number.isFinite(n) || n <= 0) return null;
    const i = Math.floor(n);
    return i > 0xffff_ffff ? 0xffff_ffff : i;
  }

  function scheduleSave(): void {
    if (autosaveTimer) clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => {
      autosaveTimer = null;
      void save();
    }, AUTOSAVE_DELAY_MS);
  }

  async function save(): Promise<void> {
    if (!editing || inflight) return;
    inflight = true;
    saveStatus = "saving";
    if (savedFlashTimer) {
      clearTimeout(savedFlashTimer);
      savedFlashTimer = null;
    }
    try {
      const body: GlobalConfig = {
        preferences: editing,
        // Pass-through fields: the inspector doesn't manage these but
        // the PATCH expects the full payload back. Read from the
        // server's last-known snapshot (via drive.info) so the inspector
        // never accidentally clobbers settings written elsewhere.
        default_drive_root: undefined,
        drives: undefined,
      };
      await api.updateConfig(body);
      const info = await api.drive();
      drive.info = info;
      // Re-snap so the next dirty-check compares against what the
      // server canonicalized (e.g. trim / unset-clear semantics).
      const fresh = snapOf(info.preferences);
      lastSnap = fresh;
      saveStatus = "saved";
      savedFlashTimer = setTimeout(() => {
        if (saveStatus === "saved") saveStatus = "idle";
        savedFlashTimer = null;
      }, 1500);
    } catch (e) {
      saveStatus = { error: (e as Error).message };
    } finally {
      inflight = false;
      // If another edit landed during the save, kick off a follow-up.
      if (editing && snapOf(editing) !== lastSnap) scheduleSave();
    }
  }

  /// Edit hook: any mutation to `editing.assistant` schedules a save.
  $effect(() => {
    if (!editing) return;
    const snap = snapOf(editing);
    if (snap === lastSnap) return;
    scheduleSave();
  });

  function onActiveProviderChange(e: Event): void {
    if (!editing) return;
    const next = (e.currentTarget as HTMLSelectElement).value as AssistantBackendKind;
    editing.assistant.default_backend = next;
    // Lazy-load catalog for the new provider so the model dropdown
    // populates immediately instead of waiting for the next render
    // cycle's $effect to catch up.
    if (next === "claude" && anthropicModels.length === 0) void refreshAnthropicModels();
    else if (next === "gemini" && geminiModels.length === 0) void refreshGeminiModels();
    else if (next === "ollama" && ollamaModels.length === 0) void refreshOllamaModels();
  }

  onMount(() => {
    syncFromServer();
  });

  /// Helper: read the current model field for the active provider.
  /// Returns the empty string when no model is pinned (chan-llm falls
  /// back to the backend default in that case).
  const activeModel = $derived<string | null>(
    activeAssistantField((a) => modelOf(a, activeProvider)),
  );

  function modelOf(a: AssistantPrefs, kind: AssistantBackendKind | null): string | null {
    if (!kind) return null;
    if (kind === "claude") return a.claude.model ?? null;
    if (kind === "gemini") return a.gemini.model ?? null;
    if (kind === "ollama") return a.ollama.model ?? null;
    if (kind === "claude_cli") return a.claude_cli.model ?? null;
    if (kind === "gemini_cli") return a.gemini_cli.model ?? null;
    return null;
  }

  function activeAssistantField<T>(f: (a: AssistantPrefs) => T): T | null {
    if (!editing) return null;
    return f(editing.assistant);
  }
</script>

<div class="assist-inspector">
  {#if !editing || !drive.info}
    <div class="placeholder">loading…</div>
  {:else if enabledList.length === 0}
    <div class="placeholder muted">
      No assistants enabled yet. Open Settings (Cmd/Ctrl+,) and enable a
      provider to use the assistant on this scope.
    </div>
  {:else}
    <label class="field">
      <span>Active assistant</span>
      <select value={activeProvider ?? ""} onchange={onActiveProviderChange}>
        {#each enabledList as p (p.kind)}
          <option value={p.kind}>{p.label}</option>
        {/each}
      </select>
    </label>

    {#if activeProvider === "claude"}
      <label class="field">
        <span>Model</span>
        <span class="model-row">
          <select bind:value={editing.assistant.claude.model}>
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
            disabled={anthropicLoading}
            title="re-query Anthropic for the model list"
          >{anthropicLoading ? "…" : "↻"}</button>
        </span>
      </label>
      <label class="field">
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
        />
      </label>
      {#if anthropicSource === "fallback" && anthropicError}
        <div class="muted small">
          Anthropic: {anthropicError} (showing curated list)
        </div>
      {/if}
    {:else if activeProvider === "gemini"}
      <label class="field">
        <span>Model</span>
        <span class="model-row">
          <select bind:value={editing.assistant.gemini.model}>
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
            disabled={geminiLoading}
            title="re-query Google for the model list"
          >{geminiLoading ? "…" : "↻"}</button>
        </span>
      </label>
      <label class="field">
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
        />
      </label>
      {#if geminiSource === "fallback" && geminiError}
        <div class="muted small">
          Gemini: {geminiError} (showing curated list)
        </div>
      {/if}
    {:else if activeProvider === "ollama"}
      <label class="field">
        <span>Model</span>
        <span class="model-row">
          <select bind:value={editing.assistant.ollama.model} disabled={ollamaLoading}>
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
            disabled={ollamaLoading}
            title="re-query Ollama for installed models"
          >{ollamaLoading ? "…" : "↻"}</button>
        </span>
      </label>
      <label class="field">
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
        />
      </label>
      {#if ollamaError}
        <div class="muted small">Ollama: {ollamaError}</div>
      {/if}
    {:else if activeProvider === "claude_cli"}
      <label class="field">
        <span>Model</span>
        <select bind:value={editing.assistant.claude_cli.model}>
          <option value={null}>(use CLI default)</option>
          {#each CLAUDE_CLI_MODELS as name (name)}
            <option value={name}>{name}</option>
          {/each}
        </select>
      </label>
    {:else if activeProvider === "gemini_cli"}
      <label class="field">
        <span>Model</span>
        <select bind:value={editing.assistant.gemini_cli.model}>
          <option value={null}>(use CLI default)</option>
          {#each GEMINI_CLI_MODELS as name (name)}
            <option value={name}>{name}</option>
          {/each}
        </select>
      </label>
    {/if}

    <div class="footer">
      <span class="status">
        {#if saveStatus === "saving"}
          <span class="muted">saving…</span>
        {:else if saveStatus === "saved"}
          <span class="ok">saved</span>
        {:else if typeof saveStatus === "object"}
          <span class="err" title={saveStatus.error}>save failed</span>
        {/if}
      </span>
      {#if activeModel}
        <span class="active-model mono" title={activeModel}>{activeModel}</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .assist-inspector {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0.6rem;
    font-size: 14px;
  }
  .placeholder { color: var(--text-secondary); font-size: 14px; }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .field > span {
    color: var(--text-secondary);
    font-size: 13px;
  }
  .field select,
  .field input {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 4px 6px;
    font: inherit;
    font-size: 14px;
    outline: none;
  }
  .field select:focus,
  .field input:focus { border-color: var(--link); }
  .model-row {
    display: flex;
    align-items: stretch;
    gap: 4px;
  }
  .model-row select { flex: 1; }
  .model-row .refresh {
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 3px;
    padding: 0 8px;
    font: inherit;
    cursor: pointer;
  }
  .model-row .refresh:hover:not(:disabled) { border-color: var(--btn-hover); }
  .model-row .refresh:disabled { opacity: 0.55; cursor: default; }
  .small { font-size: 12px; }
  .muted { color: var(--text-secondary); }
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    margin-top: 0.3rem;
    font-size: 12px;
  }
  .status .ok { color: var(--accent); }
  .status .err { color: var(--warn-text); }
  .active-model {
    color: var(--text-secondary);
    text-overflow: ellipsis;
    overflow: hidden;
    white-space: nowrap;
    max-width: 60%;
  }
  .mono { font-family: var(--mono-font, ui-monospace, monospace); }
</style>
