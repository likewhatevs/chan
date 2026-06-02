<script lang="ts">
  // File Browser back-side (Cmd+, on a File Browser). Hosts the
  // per-workspace directory blocklist (round-1 wave-3): names of
  // directories to skip when indexing + building the graph. The walk
  // skips `effective = union(defaults, additions)`; `defaults` is the
  // global machine-wide baseline (read-only here), and this surface
  // edits only the per-workspace additions.
  //
  // Editing model is GET-then-PUT-the-whole-set: load the current set,
  // add / remove client-side, then PUT the full additions list (the
  // server normalizes + queues one re-walk). Names only - exact
  // case-insensitive basenames matched at any depth, no paths/globs.
  import { onDestroy, onMount } from "svelte";
  import HybridSurfaceConfigShell from "./HybridSurfaceConfigShell.svelte";
  import { api } from "../api/client";
  import { tree } from "../state/store.svelte";
  import type { ExcludedDirsView } from "../api/types";

  type SaveStatus = "idle" | "saving" | "saved" | { error: string };

  let { onDone }: { onDone?: () => void } = $props();

  let view = $state<ExcludedDirsView | null>(null);
  let additions = $state<string[]>([]);
  let draft = $state("");
  let loadError = $state<string | null>(null);
  let saveStatus = $state<SaveStatus>("idle");

  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    try {
      const v = await api.excludedDirs();
      view = v;
      additions = [...v.workspace];
    } catch (e) {
      loadError = e instanceof Error ? e.message : String(e);
    }
  });

  onDestroy(() => {
    if (saveTimer) clearTimeout(saveTimer);
  });

  function basename(p: string): string {
    const parts = p.split("/").filter(Boolean);
    return parts.length ? parts[parts.length - 1] : p;
  }

  // Directory basenames from the loaded tree, minus what's already
  // excluded (additions or a baseline default), for the add-input's
  // autocomplete. Only currently-loaded dirs show up; the field still
  // accepts any typed name (the blocklist matches at any depth).
  const suggestions = $derived.by(() => {
    const have = new Set([...additions, ...(view?.defaults ?? [])]);
    const names = new Set<string>();
    for (const e of tree.entries) {
      if (!e.is_dir) continue;
      const b = basename(e.path).trim().toLowerCase();
      if (b && !have.has(b)) names.add(b);
    }
    return [...names].sort();
  });

  // Mirror the server's normalize(): trim, lower-case (matching is
  // case-insensitive), reject path separators (a name, not a path).
  // Returns null when the entry is empty or invalid.
  function normalizeName(raw: string): string | null {
    const name = raw.trim();
    if (!name) return null;
    if (name.includes("/") || name.includes("\\")) return null;
    return name.toLowerCase();
  }

  function addDraft(): void {
    const name = normalizeName(draft);
    // Keep an invalid entry (empty / contains a path separator) in the
    // field so the names-only rejection is visible next to the hint,
    // rather than silently clearing it.
    if (!name) return;
    draft = "";
    if (additions.includes(name) || (view?.defaults ?? []).includes(name)) return;
    additions = [...additions, name].sort();
    scheduleSave();
  }

  function remove(name: string): void {
    additions = additions.filter((d) => d !== name);
    scheduleSave();
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "Enter") {
      e.preventDefault();
      addDraft();
    }
  }

  // Debounce so rapid add/remove edits collapse into one PUT (and one
  // re-walk) rather than firing per keystroke.
  function scheduleSave(): void {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(save, 600);
  }

  async function save(): Promise<void> {
    saveTimer = null;
    saveStatus = "saving";
    try {
      const v = await api.setExcludedDirs(additions);
      view = v;
      // Re-sync to the server's normalized set (it already matches what
      // we sent, since we normalize client-side too).
      additions = [...v.workspace];
      saveStatus = "saved";
    } catch (e) {
      saveStatus = { error: e instanceof Error ? e.message : String(e) };
    }
  }
</script>

<HybridSurfaceConfigShell
  title="Hybrid File Browser"
  surface="browser"
  ariaLabel="File Browser settings"
  {saveStatus}
  {onDone}
>
  <section>
    <h3>Excluded directories</h3>
    <p class="hint">
      Directory names to skip when indexing and building the graph.
      Matched by exact name at any depth, case-insensitive. Names only,
      not paths.
    </p>

    {#if loadError}
      <p class="error">Couldn't load the blocklist: {loadError}</p>
    {:else}
      <div class="add-row">
        <input
          type="text"
          placeholder="Add a directory name..."
          list="excluded-dir-suggestions"
          bind:value={draft}
          onkeydown={onKeydown}
          aria-label="Add an excluded directory name"
        />
        <datalist id="excluded-dir-suggestions">
          {#each suggestions as s (s)}
            <option value={s}></option>
          {/each}
        </datalist>
        <button type="button" class="add-btn" onclick={addDraft} disabled={!draft.trim()}>
          Add
        </button>
      </div>

      {#if additions.length === 0}
        <p class="hint muted">No extra directories excluded for this workspace.</p>
      {:else}
        <ul class="chips" aria-label="Excluded directories for this workspace">
          {#each additions as name (name)}
            <li class="chip">
              <span class="chip-name">{name}</span>
              <button
                type="button"
                class="chip-x"
                onclick={() => remove(name)}
                aria-label={`Remove ${name}`}
                title={`Remove ${name}`}>×</button
              >
            </li>
          {/each}
        </ul>
      {/if}

      {#if view && view.defaults.length}
        <details class="defaults">
          <summary>Always excluded ({view.defaults.length})</summary>
          <p class="hint muted">
            These come from the machine-wide baseline and apply to every
            workspace. They can't be edited here.
          </p>
          <ul class="chips readonly">
            {#each view.defaults as name (name)}
              <li class="chip"><span class="chip-name">{name}</span></li>
            {/each}
          </ul>
        </details>
      {/if}
    {/if}
  </section>
</HybridSurfaceConfigShell>

<style>
  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.4;
  }
  .hint.muted {
    font-style: italic;
  }
  .error {
    margin: 0;
    color: #d33;
    font-size: 13px;
  }
  .add-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .add-row input {
    flex: 1;
    min-width: 0;
    background: var(--input-bg, var(--bg-card));
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 5px 8px;
    font: inherit;
  }
  .add-btn {
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 5px 12px;
    font: inherit;
    cursor: pointer;
  }
  .add-btn:hover:not(:disabled) {
    border-color: var(--btn-hover);
  }
  .add-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .chips {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: var(--bg-card, rgba(0, 0, 0, 0.04));
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 2px 4px 2px 10px;
    font-size: 13px;
    color: var(--text);
  }
  .chips.readonly .chip {
    padding: 2px 10px;
    color: var(--text-secondary);
  }
  .chip-name {
    font-family: var(--chan-editor-code-family, monospace);
  }
  .chip-x {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    padding: 0 4px;
    border-radius: 50%;
  }
  .chip-x:hover {
    color: var(--text);
    background: var(--border);
  }
  .defaults {
    margin-top: 4px;
  }
  .defaults summary {
    cursor: pointer;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .defaults p {
    margin: 6px 0;
  }
</style>
