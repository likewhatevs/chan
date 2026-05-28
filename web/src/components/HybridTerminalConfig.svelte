<script lang="ts">
  // `fullstack-a-45` Task B: Terminal settings migrated out of
  // the (since-retired) global Settings overlay into the Hybrid
  // back-side mount point introduced by `-a-43` Task A. Settings
  // storage shape is unchanged; only the UI mounting point
  // moves.

  import { api } from "../api/client";
  import type { GlobalConfig, Preferences } from "../api/types";
  import { workspace } from "../state/store.svelte";
  import {
    clampScrollbackMb,
    SCROLLBACK_MB_DEFAULT,
    SCROLLBACK_MB_MAX,
    SCROLLBACK_MB_MIN,
  } from "../terminal/scrollback";
  import HybridSurfaceConfigShell from "./HybridSurfaceConfigShell.svelte";

  let { onDone }: { onDone?: () => void } = $props();

  /// `fullstack-b-11` TERM dropdown set. Known terminfo entries
  /// most users either want by default or fall back to from a
  /// custom environment; the "Custom..." escape hatch toggles a
  /// free-text input for exotic values (alacritty-direct, kitty,
  /// vt100, etc.). Mirrors the constant set that lived on the
  /// retired global Settings overlay before `-a-45` moved this
  /// section.
  const KNOWN_TERM_VALUES = [
    "xterm-256color",
    "xterm",
    "tmux-256color",
    "screen-256color",
  ] as const;
  const DEFAULT_TERM = "xterm-256color";
  const CUSTOM_TERM_SENTINEL = "__custom__";

  type SaveStatus = "idle" | "saving" | "saved" | { error: string };

  /// Local edit buffer for the terminal slice. We mirror
  /// `workspace.info.preferences` into a private snapshot so the form
  /// can debounce a burst of changes into one PATCH without
  /// clobbering external edits (e.g. a parallel theme save from
  /// another back-of-card surface). Re-syncs from the server on
  /// every workspace.info refresh when no local edit is in
  /// flight.
  let editing = $state<Preferences | null>(null);
  let saveStatus = $state<SaveStatus>("idle");

  const AUTOSAVE_DELAY_MS = 500;
  const SAVED_FLASH_MS = 1500;
  let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  let savedFlashTimer: ReturnType<typeof setTimeout> | null = null;
  let inflight = false;
  let lastSentSnapshot: string | null = null;
  let failedSaveSnap: string | null = null;

  function clone(p: Preferences): Preferences {
    return JSON.parse(JSON.stringify(p));
  }

  function normalizeTerminal(p: Preferences): Preferences {
    if (p.terminal) {
      p.terminal.scrollback_mb = clampScrollbackMb(p.terminal.scrollback_mb);
      const term = (p.terminal.default_term ?? "").trim();
      p.terminal.default_term = term.length > 0 ? term : DEFAULT_TERM;
    }
    return p;
  }

  function terminalSnapshot(): string {
    return JSON.stringify(editing?.terminal ?? null);
  }

  /// Sync from workspace.info into the local edit buffer when there is
  /// no pending edit. The guard avoids overwriting a user's typing
  /// while a background workspace refresh races. After a successful
  /// save we deliberately re-sync so the form reflects the
  /// server's authoritative state.
  $effect(() => {
    const info = workspace.info;
    if (!info) return;
    if (editing && terminalSnapshot() !== lastSentSnapshot) {
      // User has unsaved local edits; do not clobber.
      if (lastSentSnapshot === null) return;
    }
    editing = normalizeTerminal(clone(info.preferences));
  });

  const scrollbackMb = $derived(
    clampScrollbackMb(editing?.terminal?.scrollback_mb),
  );
  /// Raw persisted value WITHOUT the empty → DEFAULT_TERM
  /// collapse. The previous derivation
  /// (`(default_term ?? DEFAULT_TERM).trim() || DEFAULT_TERM`)
  /// over-coerced empty values, which is what produced the
  /// `-a-45` custom-TERM PARTIAL surfaced by `webtest-a-4` —
  /// after the user picked "Custom..." (seeding default_term=""),
  /// the derivation snapped back to DEFAULT_TERM, isKnownTerm
  /// went true, and the custom input never appeared. `-a-53`
  /// fix: track "user picked Custom" in a separate
  /// `customMode` state so the input renders even when the
  /// persisted value is empty.
  const persistedTerm = $derived(
    (editing?.terminal?.default_term ?? "").trim(),
  );
  const currentTerm = $derived(persistedTerm || DEFAULT_TERM);
  const persistedIsKnown = $derived(
    persistedTerm.length > 0 &&
      (KNOWN_TERM_VALUES as readonly string[]).includes(persistedTerm),
  );
  let customMode = $state(false);
  let customModeInited = false;
  /// Initialise `customMode` from the persisted shape exactly
  /// once, after the first server load. Re-syncs are gated to
  /// avoid clobbering the user's in-progress dropdown choice on
  /// every workspace.info refresh.
  $effect(() => {
    if (customModeInited) return;
    if (!editing) return;
    if (persistedTerm.length === 0) return;
    customMode = !persistedIsKnown;
    customModeInited = true;
  });
  const termSelectValue = $derived(
    customMode ? CUSTOM_TERM_SENTINEL : (persistedIsKnown ? persistedTerm : DEFAULT_TERM),
  );
  const isKnownTerm = $derived(persistedIsKnown);

  function setScrollbackMb(raw: number): void {
    if (!editing) return;
    editing.terminal = {
      ...editing.terminal,
      scrollback_mb: clampScrollbackMb(raw),
    };
  }

  function setTermSelection(next: string): void {
    if (!editing) return;
    if (next === CUSTOM_TERM_SENTINEL) {
      customMode = true;
      // Don't clear the persisted value here — if the user
      // toggles Custom → known → Custom, their previous custom
      // string should still be in the input. We just flip the
      // UI mode; the input becomes editable.
      return;
    }
    customMode = false;
    editing.terminal = { ...editing.terminal, default_term: next };
  }

  function setCustomTerm(raw: string): void {
    if (!editing) return;
    editing.terminal = { ...editing.terminal, default_term: raw };
  }

  /// `fullstack-b-30` slice b: terminal-font preference. Default
  /// "os-default" leans on the per-OS native mono chain from
  /// slice a; "source-code-pro" opts the user into SCP. When the
  /// build doesn't ship the rust-embed bundle (`embed-font` off),
  /// flipping to SCP fires the download endpoint to fetch the
  /// woff2 + OFL.txt into `<user-config>/chan/fonts/`. Failures
  /// roll the preference back to `os-default` so the SPA never
  /// claims SCP is active while the user-config-dir file doesn't
  /// exist.
  let fontDownloading = $state(false);
  let fontStatusMessage = $state<string | null>(null);

  const fontChoice = $derived(
    (editing?.terminal?.font ?? "os-default") as
      | "os-default"
      | "source-code-pro",
  );

  async function setFontChoice(next: "os-default" | "source-code-pro"): Promise<void> {
    if (!editing) return;
    if (next === fontChoice) return;
    // Optimistic update: flip the local edit buffer first so the
    // dropdown reflects the choice immediately. Roll back on
    // download failure.
    editing.terminal = { ...editing.terminal, font: next };
    if (next === "source-code-pro") {
      fontDownloading = true;
      fontStatusMessage = "Downloading Source Code Pro…";
      try {
        await api.fontsSourceCodeProDownload();
        fontStatusMessage = "Source Code Pro ready.";
      } catch (err) {
        // Roll back the preference so the SPA's terminal font
        // matches what's actually available on disk.
        if (editing) {
          editing.terminal = { ...editing.terminal, font: "os-default" };
        }
        const msg = err instanceof Error ? err.message : String(err);
        fontStatusMessage = `Source Code Pro download failed: ${msg}`;
      } finally {
        fontDownloading = false;
      }
    } else {
      fontStatusMessage = null;
    }
  }

  /// Compare the local terminal slice against the server's most
  /// recent workspace.info.preferences. The dirty check is scoped to
  /// the terminal subtree so we never trigger a PATCH for theme /
  /// editor / date changes (those belong to other back-of-card
  /// surfaces).
  function terminalDirty(): boolean {
    if (!editing) return false;
    const server = workspace.info?.preferences?.terminal;
    return (
      JSON.stringify(editing.terminal ?? null) !==
      JSON.stringify(server ?? null)
    );
  }

  function scheduleSave(): void {
    if (autosaveTimer) clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => {
      autosaveTimer = null;
      void save();
    }, AUTOSAVE_DELAY_MS);
  }

  /// Save the terminal slice. We re-fetch the current global config
  /// before PATCHing so concurrent edits from other back-of-card
  /// surfaces (theme etc.) merge correctly: the new GlobalConfig
  /// payload starts from the server's latest state and overlays
  /// only this form's terminal subtree. The two-fetch re-sync
  /// pattern after the PATCH keeps workspace.info authoritative.
  async function save(): Promise<void> {
    if (!editing || inflight) return;
    if (!terminalDirty()) return;
    inflight = true;
    saveStatus = "saving";
    if (savedFlashTimer) {
      clearTimeout(savedFlashTimer);
      savedFlashTimer = null;
    }
    const sent = terminalSnapshot();
    lastSentSnapshot = sent;
    try {
      const current = await api.config();
      const cfgBody: GlobalConfig = {
        preferences: { ...current.preferences, terminal: editing.terminal },
        default_workspace_root: current.default_workspace_root,
        workspaces: current.workspaces,
      };
      await api.updateConfig(cfgBody);
      const info = await api.workspace();
      workspace.info = info;
      // Re-sync the local buffer to the server's authoritative
      // value (cheap; the only field that differs is the one we
      // just persisted, but normalize the result).
      editing = normalizeTerminal(clone(info.preferences));
      lastSentSnapshot = terminalSnapshot();
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
      // If another edit landed while saving, reschedule. Same
      // guard the other back-of-card configs use to avoid an
      // infinite loop on identical-to-server state.
      if (terminalDirty() && terminalSnapshot() !== failedSaveSnap) {
        scheduleSave();
      }
    }
  }

  $effect(() => {
    if (!editing) return;
    const snap = terminalSnapshot();
    if (!terminalDirty()) return;
    if (snap === failedSaveSnap) return;
    scheduleSave();
  });
</script>

<HybridSurfaceConfigShell
  title="Hybrid Terminal"
  surface="terminal"
  saveStatus={saveStatus}
  {onDone}
>
    <!-- `fullstack-a-45`: warning copy carried over from the
         round-2-plan Hybrid back-side scope note. These settings
         are device-wide, not per-pane; every terminal in the
         workspace picks them up on next spawn. The top-bar body theme
         applies to every terminal body on this device. -->
    <p class="hint warning">
      Scrollback, TERM, and font apply to ALL terminals on this
      device. The top-bar theme switch applies to ALL terminal
      bodies. Existing terminals keep their current scrollback,
      <code>TERM</code>, and font until the chan session restarts.
    </p>

    <div class="terminal-field">
      <label class="terminal-label" for="hybrid-terminal-scrollback-mb">
        <span>Scrollback (MB)</span>
      </label>
      <div class="terminal-control scrollback-control">
        <input
          id="hybrid-terminal-scrollback-mb"
          type="range"
          min={SCROLLBACK_MB_MIN}
          max={SCROLLBACK_MB_MAX}
          step="5"
          value={scrollbackMb}
          oninput={(e) => setScrollbackMb(Number((e.currentTarget as HTMLInputElement).value))}
          aria-describedby="hybrid-terminal-scrollback-hint"
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
      <p id="hybrid-terminal-scrollback-hint" class="hint sub-hint">
        Per-terminal cap on the in-memory scrollback buffer.
        Default {SCROLLBACK_MB_DEFAULT} MB; range
        {SCROLLBACK_MB_MIN}-{SCROLLBACK_MB_MAX} MB. Higher caps let
        agent-driven terminals retain more redraw history at the
        cost of more browser memory per open tab.
      </p>
    </div>

    <div class="terminal-field">
      <label class="terminal-label" for="hybrid-terminal-default-term">
        <span>Default TERM</span>
      </label>
      <div class="terminal-control">
        <select
          id="hybrid-terminal-default-term"
          class="config-select family"
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
        Sets <code>TERM</code> on newly spawned shells. Pick one of
        the common terminfo entries or supply a custom value if
        your environment expects something exotic.
      </p>
    </div>

    <div class="terminal-field">
      <label class="terminal-label" for="hybrid-terminal-font">
        <span>Terminal font</span>
      </label>
      <div class="terminal-control">
        <select
          id="hybrid-terminal-font"
          class="config-select family"
          value={fontChoice}
          disabled={fontDownloading}
          onchange={(e) =>
            void setFontChoice(
              (e.currentTarget as HTMLSelectElement).value as
                | "os-default"
                | "source-code-pro",
            )}
        >
          <option value="os-default">OS default (mono)</option>
          <option value="source-code-pro">Source Code Pro</option>
        </select>
      </div>
      <p class="hint sub-hint">
        Default uses your OS's native monospace font (SF Mono on
        macOS, Cascadia on Windows, DejaVu on Linux). Choosing
        Source Code Pro downloads ~80 KB into your user config
        dir on first enable; subsequent enables are instant.
        Spawn-time-only: existing terminals keep their current
        font until restart.
      </p>
      {#if fontStatusMessage}
        <p class="hint sub-hint" role="status">{fontStatusMessage}</p>
      {/if}
    </div>
</HybridSurfaceConfigShell>

<style>
  /* `fullstack-a-45`: warning copy that distinguishes this surface
     from per-pane settings. The warning class adds a subtle
     border-left + tinted background so the all-terminals scope
     reads at a glance. */
  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .hint.warning {
    border-left: 3px solid var(--accent, #f97316);
    padding: 0.5rem 0.75rem;
    background: color-mix(in srgb, var(--accent, #f97316) 6%, transparent);
    border-radius: 4px;
  }
  /* `fullstack-b-11` Terminal field layout carried over from the
     retired global Settings overlay; the .terminal-* scope
     keeps the styles local to this component now that the old
     section is gone. */
  .terminal-field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .terminal-label {
    display: block;
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
  .config-select {
    min-width: 16em;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 5px 7px;
    font: inherit;
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
  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
</style>
