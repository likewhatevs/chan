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
  //   2. Model (per-CLI dropdown where we have a stable shortlist,
  //      free-text for Codex so the CLI validates the final value).
  //
  // Writes round-trip through `api.updateConfig` so the value sticks
  // across reloads. The Settings panel reads from the same source,
  // so opening it later shows whatever the inspector last persisted.

  import { onMount } from "svelte";
  import { api } from "../api/client";
  import type {
    AssistantBackendKind,
    AssistantPrefs,
    CliDetectionView,
    GlobalConfig,
    Preferences,
  } from "../api/types";
  import { drive, recordAssistantSwitch } from "../state/store.svelte";

  /// Curated model shortlists for the local CLIs (`claude_cli` /
  /// `gemini_cli`). Same list the old Settings UI used. Codex CLI
  /// stays free-text here because chan-llm does not expose a stable
  /// shortlist for it through this inspector surface yet. Aliases
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
  const CODEX_CLI_MODELS = ["gpt-5.2-codex", "gpt-5.2", "gpt-5.1-codex"];

  /// Local editable mirror of `drive.info.preferences`. Mutations to
  /// the assistant subtree trigger a debounced PATCH (see save()
  /// below); outside changes (e.g. the Settings panel saving on the
  /// other side, or a fresh /api/drive after a server-side flip) re-
  /// sync into `editing` whenever the snapshot differs.
  let editing = $state<Preferences | null>(null);
  let lastSnap = "";
  let cliDetections = $state<CliDetectionView[]>([]);
  let cliDetectionLoading = $state(false);
  let cliDetectionError = $state<string | null>(null);

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

  async function loadCliDetection(): Promise<void> {
    if (cliDetectionLoading) return;
    cliDetectionLoading = true;
    cliDetectionError = null;
    try {
      cliDetections = (await api.llmCliDetection()).detections;
    } catch (e) {
      cliDetections = [];
      cliDetectionError = (e as Error).message;
    } finally {
      cliDetectionLoading = false;
    }
  }

  /// Enumerate ready providers for the dropdown. Settings configures
  /// every CLI; the overlay chooses which ready assistant to use for
  /// the next turn.
  const readyList = $derived<{ kind: AssistantBackendKind; label: string }[]>(
    !editing || cliDetections.length === 0
      ? []
      : (
          [
            { kind: "claude_cli", label: "Claude CLI" },
            { kind: "gemini_cli", label: "Gemini CLI" },
            { kind: "codex_cli", label: "Codex CLI" },
          ] as const
        )
          .filter((p) => cliDetections.some((d) => d.backend === p.kind && d.ready))
          .map((p) => ({
            kind: p.kind as AssistantBackendKind,
            label: modelOf(editing!.assistant, p.kind)
              ? `${p.label} · ${modelOf(editing!.assistant, p.kind)}`
              : p.label,
          })),
  );

  /// Active provider derived from the persisted default, but only if
  /// that backend is currently ready. Otherwise fall back to the first
  /// ready CLI so the overlay can switch to a usable assistant.
  const activeProvider = $derived<AssistantBackendKind | null>(
    readyList.some((p) => p.kind === editing?.assistant.default_backend)
      ? editing?.assistant.default_backend ?? null
      : readyList[0]?.kind ?? null,
  );

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
    const prev = editing.assistant.default_backend;
    const next = (e.currentTarget as HTMLSelectElement).value as AssistantBackendKind;
    editing.assistant.default_backend = next;
    editing.assistant[next].enabled = true;
    if (prev !== next) {
      recordAssistantSwitch(next, modelOf(editing.assistant, next));
    }
  }

  onMount(() => {
    syncFromServer();
    void loadCliDetection();
  });

  /// Helper: read the current model field for the active provider.
  /// Returns the empty string when no model is pinned (chan-llm falls
  /// back to the backend default in that case).
  const activeModel = $derived<string | null>(
    activeAssistantField((a) => modelOf(a, activeProvider)),
  );

  function modelOf(a: AssistantPrefs, kind: AssistantBackendKind | null): string | null {
    if (!kind) return null;
    if (kind === "claude_cli") return a.claude_cli.model ?? null;
    if (kind === "gemini_cli") return a.gemini_cli.model ?? null;
    if (kind === "codex_cli") return a.codex_cli.model ?? null;
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
  {:else if readyList.length === 0}
    <div class="placeholder muted">
      {#if cliDetectionLoading}
        checking assistant CLIs…
      {:else if cliDetectionError}
        assistant readiness failed: {cliDetectionError}
      {:else}
        No ready assistant CLIs. Open Settings (Cmd/Ctrl+,) to inspect
        each CLI's readiness and binary override.
      {/if}
    </div>
  {:else}
    <label class="field">
      <span>Active assistant</span>
      <select value={activeProvider ?? ""} onchange={onActiveProviderChange}>
        {#each readyList as p (p.kind)}
          <option value={p.kind}>{p.label}</option>
        {/each}
      </select>
    </label>

    {#if activeProvider === "claude_cli"}
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
    {:else if activeProvider === "codex_cli"}
      <label class="field">
        <span>Model</span>
        <select bind:value={editing.assistant.codex_cli.model}>
          <option value={null}>(use CLI default)</option>
          {#each CODEX_CLI_MODELS as name (name)}
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
  .field select {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 4px 6px;
    font: inherit;
    font-size: 14px;
    outline: none;
  }
  .field select:focus { border-color: var(--link); }
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
