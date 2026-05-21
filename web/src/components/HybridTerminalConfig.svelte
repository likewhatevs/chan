<script lang="ts">
  // `fullstack-a-45` Task B: Terminal settings migrated out of
  // `SettingsPanel.svelte` into the Hybrid back-side mount point
  // introduced by `-a-43` Task A. Settings storage shape is
  // unchanged; only the UI mounting point moves.

  import { api } from "../api/client";
  import type { GlobalConfig, Preferences } from "../api/types";
  import { drive, ui } from "../state/store.svelte";
  import type { HybridTheme, LeafNode } from "../state/tabs.svelte";

  /// `fullstack-a-53` per-Hybrid theme override toggle.
  /// `pane` is the Hybrid pane this back-side surface belongs to.
  /// The override radios (Inherit / Light / Dark) write to
  /// `pane.theme` (the existing per-Hybrid override slot from
  /// `-b-5`/`-a-47`). Resolution at render time:
  /// `pane.theme ?? ui.theme`.
  let { pane }: { pane: LeafNode } = $props();

  type OverrideChoice = "inherit" | HybridTheme;
  const overrideValue = $derived<OverrideChoice>(
    pane.theme ?? "inherit",
  );

  function setOverrideChoice(next: OverrideChoice): void {
    if (next === "inherit") {
      pane.theme = undefined;
    } else {
      pane.theme = next;
    }
  }
  import {
    clampScrollbackMb,
    SCROLLBACK_MB_DEFAULT,
    SCROLLBACK_MB_MAX,
    SCROLLBACK_MB_MIN,
  } from "../terminal/scrollback";

  /// `fullstack-b-11` TERM dropdown set. Known terminfo entries
  /// most users either want by default or fall back to from a
  /// custom environment; the "Custom..." escape hatch toggles a
  /// free-text input for exotic values (alacritty-direct, kitty,
  /// vt100, etc.). Mirrors the constant set previously in
  /// SettingsPanel before `-a-45` moved this section.
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
  /// `drive.info.preferences` into a private snapshot so the form
  /// can debounce a burst of changes into one PATCH without
  /// clobbering external edits (e.g. SettingsPanel saving theme
  /// changes in parallel). Re-syncs from the server on every
  /// drive.info refresh when no local edit is in flight.
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

  /// Sync from drive.info into the local edit buffer when there is
  /// no pending edit. The guard avoids overwriting a user's typing
  /// while a background drive refresh races. After a successful
  /// save we deliberately re-sync so the form reflects the
  /// server's authoritative state.
  $effect(() => {
    const info = drive.info;
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
  /// every drive.info refresh.
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

  /// Compare the local terminal slice against the server's most
  /// recent drive.info.preferences. The dirty check is scoped to
  /// the terminal subtree so we never trigger a PATCH for theme /
  /// editor / date changes (those belong to SettingsPanel).
  function terminalDirty(): boolean {
    if (!editing) return false;
    const server = drive.info?.preferences?.terminal;
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
  /// before PATCHing so concurrent edits from SettingsPanel (theme
  /// etc.) merge correctly: the new GlobalConfig payload starts
  /// from the server's latest state and overlays only this form's
  /// terminal subtree. Mirrors SettingsPanel's two-fetch
  /// re-sync pattern after the PATCH so drive.info stays
  /// authoritative.
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
        default_drive_root: current.default_drive_root,
        drives: current.drives,
      };
      await api.updateConfig(cfgBody);
      const info = await api.drive();
      drive.info = info;
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
      // guard SettingsPanel uses to avoid an infinite loop on
      // identical-to-server state.
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

<section class="hybrid-config" aria-label="Hybrid Terminal configuration">
  <header class="config-header">
    <h2 class="config-title">Hybrid Terminal</h2>
    <div class="save-status" aria-live="polite">
      {#if saveStatus === "saving"}
        <span class="muted">saving…</span>
      {:else if saveStatus === "saved"}
        <span class="ok">saved</span>
      {:else if typeof saveStatus === "object"}
        <span class="err" title={saveStatus.error}>save failed</span>
      {/if}
    </div>
  </header>
  <div class="config-body">
    <!-- `fullstack-a-45`: warning copy carried over from the
         round-2-plan Hybrid back-side scope note. These settings
         are device-wide, not per-pane; every terminal in the
         drive picks them up on next spawn. The Appearance
         override below is the only per-Hybrid setting on this
         surface (`-a-53`). -->
    <p class="hint warning">
      Scrollback and TERM apply to ALL terminals on this device.
      The Appearance override below applies only to THIS Hybrid
      pane. Existing terminals keep their current scrollback and
      <code>TERM</code> value until the chan session restarts.
    </p>

    <!-- `fullstack-a-53` per-Hybrid Appearance override. Layered
         on top of the global Settings Appearance default;
         resolution at render: `pane.theme ?? ui.theme`. -->
    <section class="terminal-field">
      <h3 class="terminal-label">
        <span>Appearance (this Hybrid)</span>
      </h3>
      <p class="hint sub-hint">
        Override the global Appearance default for just this
        Hybrid pane. Inherit follows the global Settings choice
        (currently <strong>{ui.themeChoice}</strong>).
      </p>
      <div class="theme-row" role="radiogroup" aria-label="Per-Hybrid Appearance override">
        {#each [
          { value: "inherit" as const, label: "Inherit" },
          { value: "light" as const, label: "Light" },
          { value: "dark" as const, label: "Dark" },
        ] as opt (opt.value)}
          <label class="theme-opt" class:on={overrideValue === opt.value}>
            <input
              type="radio"
              name="hybrid-terminal-theme-override"
              value={opt.value}
              checked={overrideValue === opt.value}
              onchange={() => setOverrideChoice(opt.value)}
            />
            <span>{opt.label}</span>
          </label>
        {/each}
      </div>
    </section>

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
        Sets <code>TERM</code> on newly spawned shells. Pick one of
        the common terminfo entries or supply a custom value if
        your environment expects something exotic.
      </p>
    </div>
  </div>
</section>

<style>
  .hybrid-config {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  .config-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }
  .config-title {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--text);
  }
  .save-status { font-size: 14px; min-width: 60px; text-align: right; }
  .save-status .ok { color: var(--accent); }
  .save-status .err { color: #d33; }
  .save-status .muted { color: var(--text-secondary); }
  .config-body {
    flex: 1;
    overflow: auto;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
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
  /* `fullstack-b-11` Terminal field layout carried over from
     SettingsPanel; the .terminal-* scope keeps the styles
     local to this component now that the SettingsPanel section
     is gone. */
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
  /* `fullstack-a-53` per-Hybrid Appearance override chips.
     Same shape as `HybridEditorConfig.svelte` (and SettingsPanel
     after the Appearance revert). The override section reuses
     the existing `.terminal-field` wrapper for vertical layout
     consistency; the .theme-row chip group lives inside. */
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
  /* Override-section header lives inside .terminal-field so the
     section heading aligns vertically with the surrounding
     Scrollback / Default TERM labels. `h3.terminal-label` keeps
     the typography consistent with the rest of the labels in
     this surface. */
  h3.terminal-label {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
</style>
