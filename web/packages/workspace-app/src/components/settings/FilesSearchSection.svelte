<script lang="ts">
  // Files and search settings: the server-config `search_aggression`
  // and `attachments_dir` fields. The attachments path is workspace-
  // relative and rejected empty server-side, so a cleared field is left
  // uncommitted rather than PATCHed as empty.

  import type { Preferences, SearchAggression } from "../../api/types";
  import type { CommitFn } from "./commit";
  import SettingField from "./SettingField.svelte";
  import PillRadio from "./PillRadio.svelte";

  let { prefs, commit }: { prefs: Preferences; commit: CommitFn } = $props();

  const AGGRESSION = [
    { value: "conservative", label: "Conservative" },
    { value: "balanced", label: "Balanced" },
    { value: "aggressive", label: "Aggressive" },
  ] as const;

  // Controlled field (value from the buffer); the debounce reads the
  // live input value so a per-keystroke PATCH is avoided.
  let attTimer: ReturnType<typeof setTimeout> | null = null;
  function onAttInput(raw: string): void {
    if (attTimer) clearTimeout(attTimer);
    const value = raw.trim();
    attTimer = setTimeout(() => {
      attTimer = null;
      if (!value) return; // empty is rejected server-side; keep the stored path
      commit((p) => ({ ...p, attachments_dir: value }));
    }, 400);
  }
</script>

<SettingField
  label="Search indexing"
  hint="Resource profile for the search indexer. Aggressive indexes more eagerly at a higher cost."
>
  <PillRadio
    name="settings-search"
    ariaLabel="Search indexing profile"
    value={prefs.search_aggression}
    options={AGGRESSION}
    onselect={(v) =>
      commit((p) => ({ ...p, search_aggression: v as SearchAggression }))}
  />
</SettingField>

<SettingField
  label="Attachments folder"
  hint="Workspace-relative folder where pasted and uploaded images are saved."
>
  <input
    type="text"
    value={prefs.attachments_dir}
    oninput={(e) => onAttInput(e.currentTarget.value)}
    placeholder="attachments"
    spellcheck={false}
    aria-label="Attachments folder"
  />
</SettingField>
