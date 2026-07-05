<script lang="ts">
  // Editor settings: the editor-prefs slice for text editing behaviour.
  // The date format is a plain select; the page-width slider debounces
  // so a drag does not fire a PATCH per tick.

  import type { Preferences } from "../../api/types";
  import type { CommitFn } from "./commit";
  import { DATE_FORMATS } from "../../editor/dateFormats";
  import {
    PAGE_WIDTH_MAX_PCT,
    PAGE_WIDTH_MIN_PCT,
    PAGE_WIDTH_STEP_PCT,
  } from "../../state/pageWidth.svelte";
  import SettingField from "./SettingField.svelte";
  import PillToggle from "./PillToggle.svelte";

  let { prefs, commit }: { prefs: Preferences; commit: CommitFn } = $props();

  function ratioToPct(r: number | undefined): number {
    const clamped = Math.min(1, Math.max(0.25, r ?? 0.8));
    return Math.round(clamped * 100);
  }

  // Local slider position so the thumb tracks the drag; the effect
  // seeds it from the buffer and resyncs when the stored ratio changes.
  let widthPct = $state(80);
  $effect(() => {
    widthPct = ratioToPct(prefs.page_width_ratio);
  });
  let widthTimer: ReturnType<typeof setTimeout> | null = null;
  function onWidthInput(): void {
    if (widthTimer) clearTimeout(widthTimer);
    const pct = widthPct;
    widthTimer = setTimeout(() => {
      widthTimer = null;
      commit((p) => ({ ...p, page_width_ratio: pct / 100 }));
    }, 200);
  }
</script>

<SettingField
  label="Date format"
  hint="Default used by @today and pre-selected in the @date picker."
>
  <select
    value={prefs.date_format}
    onchange={(e) =>
      commit((p) => ({ ...p, date_format: e.currentTarget.value }))}
  >
    {#each DATE_FORMATS as f (f.id)}
      <option value={f.id}>{f.label}</option>
    {/each}
  </select>
</SettingField>

<SettingField
  label="Strip trailing whitespace"
  hint="Remove trailing spaces and tabs from each line on save. Applies to every editable text buffer."
>
  <PillToggle
    label="Strip on save"
    checked={prefs.strip_trailing_whitespace_on_save}
    ontoggle={(on) =>
      commit((p) => ({ ...p, strip_trailing_whitespace_on_save: on }))}
  />
</SettingField>

<SettingField
  label="Editor page width"
  hint="Cap the editor text column as a share of the window width. 100% removes the cap."
>
  <input
    type="range"
    min={PAGE_WIDTH_MIN_PCT}
    max={PAGE_WIDTH_MAX_PCT}
    step={PAGE_WIDTH_STEP_PCT}
    bind:value={widthPct}
    oninput={onWidthInput}
    aria-label="Editor page width percent"
  />
  <span class="value">{widthPct}%</span>
</SettingField>

<SettingField
  label="Empty-pane carousel"
  hint="Auto-rotate the welcome carousel shown in an empty single pane."
>
  <PillToggle
    label="Auto-rotate"
    checked={prefs.empty_pane_carousel_cycling ?? true}
    ontoggle={(on) =>
      commit((p) => ({ ...p, empty_pane_carousel_cycling: on }))}
  />
</SettingField>

<style>
  .value {
    color: var(--text-secondary);
    font-size: 13px;
    min-width: 3.5em;
    text-align: right;
  }
</style>
