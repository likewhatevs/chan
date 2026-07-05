<script lang="ts">
  // Terminal settings: the server-config `terminal` slice. All fields
  // are spawn-time, so a change applies to newly spawned terminals. The
  // scrollback slider and the free-text TERM field debounce their
  // writes. Terminal font selection (with its font download) stays on
  // the terminal back-of-pane card.

  import type { Preferences } from "../../api/types";
  import type { CommitFn } from "./commit";
  import SettingField from "./SettingField.svelte";
  import PillToggle from "./PillToggle.svelte";

  let { prefs, commit }: { prefs: Preferences; commit: CommitFn } = $props();

  const SCROLLBACK_MIN = 10;
  const SCROLLBACK_MAX = 500;
  const SCROLLBACK_STEP = 10;
  function clampScrollback(v: number | undefined): number {
    return Math.min(SCROLLBACK_MAX, Math.max(SCROLLBACK_MIN, v ?? 50));
  }

  // Local slider position so the thumb tracks the drag; the effect
  // seeds it from the buffer and resyncs when the stored value changes.
  let scrollback = $state(50);
  $effect(() => {
    scrollback = clampScrollback(prefs.terminal.scrollback_mb);
  });
  let scrollbackTimer: ReturnType<typeof setTimeout> | null = null;
  function onScrollbackInput(): void {
    if (scrollbackTimer) clearTimeout(scrollbackTimer);
    const mb = scrollback;
    scrollbackTimer = setTimeout(() => {
      scrollbackTimer = null;
      commit((p) => ({ ...p, terminal: { ...p.terminal, scrollback_mb: mb } }));
    }, 200);
  }

  // TERM is a controlled field (value from the buffer); the debounce
  // reads the live input value so a per-keystroke PATCH is avoided
  // while an in-flight edit is preserved (the buffer is stable between
  // commits).
  let termTimer: ReturnType<typeof setTimeout> | null = null;
  function onTermInput(value: string): void {
    if (termTimer) clearTimeout(termTimer);
    termTimer = setTimeout(() => {
      termTimer = null;
      commit((p) => ({ ...p, terminal: { ...p.terminal, default_term: value } }));
    }, 400);
  }
</script>

<SettingField
  label="Scrollback"
  hint="Per-terminal scrollback budget. New terminals only; existing ones keep their scrollback until they restart."
>
  <input
    type="range"
    min={SCROLLBACK_MIN}
    max={SCROLLBACK_MAX}
    step={SCROLLBACK_STEP}
    bind:value={scrollback}
    oninput={onScrollbackInput}
    aria-label="Terminal scrollback megabytes"
  />
  <span class="value">{scrollback} MB</span>
</SettingField>

<SettingField
  label="TERM"
  hint="TERM environment variable for new terminals. Blank falls back to xterm-256color."
>
  <input
    type="text"
    value={prefs.terminal.default_term ?? ""}
    oninput={(e) => onTermInput(e.currentTarget.value)}
    placeholder="xterm-256color"
    spellcheck={false}
    aria-label="Terminal TERM value"
  />
</SettingField>

<SettingField
  label="MCP discovery"
  hint="Expose the chan MCP env vars (CHAN_MCP_*) to new terminals so agent CLIs can find the MCP server."
>
  <PillToggle
    label="Enable in new terminals"
    checked={prefs.terminal.mcp_env ?? false}
    ontoggle={(on) =>
      commit((p) => ({ ...p, terminal: { ...p.terminal, mcp_env: on } }))}
  />
</SettingField>

<style>
  .value {
    color: var(--text-secondary);
    font-size: 13px;
    min-width: 4.5em;
    text-align: right;
  }
</style>
