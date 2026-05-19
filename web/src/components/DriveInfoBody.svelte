<script lang="ts">
  // Drive inspector body. Shown in the file browser's Inspector pane
  // when the user clicks the Directory row in the hamburger menu. Houses
  // the global Notes Directories config (default root + recent drives list).
  // Search index status lives in the Search Status overlay.

  import { onMount } from "svelte";
  import { api } from "../api/client";
  import type { GlobalConfig } from "../api/types";
  import { drive } from "../state/store.svelte";

  /// `fullstack-73`: optional "Graph from here" callback. Consumers
  /// that host this body alongside an existing inspector convention
  /// pass it; surfaces that don't (legacy callers) leave it unset
  /// and the button doesn't render. The action's semantic differs
  /// per consumer — FileBrowserSurface SPAWNS a new Graph tab while
  /// GraphPanel's own inspector RE-SCOPES the current tab — so the
  /// consumer wires the function and this body is callback-agnostic.
  let {
    onSetAsScope,
  }: { onSetAsScope?: () => void } = $props();

  let globalConfig = $state<GlobalConfig | null>(null);
  let editedDefaultRoot = $state<string>("");
  let initialDefaultRoot = $state<string>("");
  let saveError = $state<string | null>(null);

  const AUTOSAVE_DELAY_MS = 500;
  let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  let inflight = false;

  async function loadGlobalConfig(): Promise<void> {
    try {
      globalConfig = await api.config();
      const cur = globalConfig.default_drive_root ?? "";
      editedDefaultRoot = cur;
      initialDefaultRoot = cur;
    } catch {
      globalConfig = null;
    }
  }

  function dirty(): boolean {
    if (!globalConfig) return false;
    return editedDefaultRoot !== initialDefaultRoot;
  }

  async function save(): Promise<void> {
    if (!globalConfig || inflight) return;
    inflight = true;
    saveError = null;
    const sent = editedDefaultRoot;
    try {
      const trimmed = sent.trim();
      const body: GlobalConfig = {
        preferences: globalConfig.preferences,
        default_drive_root: trimmed === "" ? null : trimmed,
        drives: globalConfig.drives,
      };
      const cfg = await api.updateConfig(body);
      globalConfig = cfg;
      // Don't clobber further edits the user typed while in flight.
      if (editedDefaultRoot === sent) {
        const echoed = cfg.default_drive_root ?? "";
        editedDefaultRoot = echoed;
        initialDefaultRoot = echoed;
      } else {
        initialDefaultRoot = cfg.default_drive_root ?? "";
      }
    } catch (e) {
      saveError = (e as Error).message;
    } finally {
      inflight = false;
      if (dirty()) scheduleSave();
    }
  }

  function scheduleSave(): void {
    if (autosaveTimer) clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => {
      autosaveTimer = null;
      void save();
    }, AUTOSAVE_DELAY_MS);
  }

  $effect(() => {
    void editedDefaultRoot;
    if (!dirty()) return;
    scheduleSave();
  });

  function formatLastOpened(iso: string): string {
    try {
      const d = new Date(iso);
      const yyyy = d.getUTCFullYear();
      const mm = String(d.getUTCMonth() + 1).padStart(2, "0");
      const dd = String(d.getUTCDate()).padStart(2, "0");
      const hh = String(d.getUTCHours()).padStart(2, "0");
      const mi = String(d.getUTCMinutes()).padStart(2, "0");
      return `${yyyy}-${mm}-${dd} ${hh}:${mi} UTC`;
    } catch {
      return iso;
    }
  }

  onMount(() => {
    void loadGlobalConfig();
  });
</script>

<div class="info">
  <header class="head">
    <span class="kind-chip drive">drive</span>
  </header>
  <h3 class="title" title={drive.info?.root}>
    {drive.info?.name ?? "(unnamed)"}
  </h3>
  <div class="meta-grid">
    <span class="k">directory</span>
    <span class="v mono path" title={drive.info?.root}>{drive.info?.root ?? ""}</span>
  </div>

  {#if onSetAsScope}
    <!-- `fullstack-73`: parity with FileInfoBody so every inspector
         surface offers the same "Graph from here" affordance. The
         hosting surface decides whether the click spawns a new
         Graph tab (file browser) or re-scopes the current one
         (Graph inspector). -->
    <button class="open" onclick={onSetAsScope}>Graph from here</button>
  {/if}

  <section class="refs">
    <h4>Notes directories</h4>
    <p class="hint">
      Your default notes directory is where chan opens when launched
      without a specific one in mind. Leave empty to use the platform
      default (<code>~/Documents/Chan</code> on macOS,
      <code>$XDG_DATA_HOME/chan/default</code> on Linux,
      <code>%USERPROFILE%\Documents\Chan</code> on Windows).
    </p>
    <label class="field">
      <span>Default</span>
      <input
        bind:value={editedDefaultRoot}
        placeholder="(platform default)"
        spellcheck="false"
        autocomplete="off"
      />
    </label>
    {#if saveError}
      <div class="err-line">save failed: {saveError}</div>
    {/if}

    {#if globalConfig?.drives && globalConfig.drives.length > 0}
      <h5 class="recents-head">Recent</h5>
      <ul class="recents">
        {#each globalConfig.drives as u (u.path)}
          <li>
            <span class="recents-time">{formatLastOpened(u.last_opened)}</span>
            {#if u.name}
              <span class="recents-name">{u.name}</span>
            {/if}
            <span class="recents-path mono" title={u.path}>{u.path}</span>
          </li>
        {/each}
      </ul>
      <p class="hint">
        Updated every time you open a directory. In-app open-from-list
        lands in a follow-up; for now use the menu's Open Directory.
      </p>
    {/if}
  </section>
</div>

<style>
  .info {
    padding: 0.6rem 0.7rem 0.8rem 0.7rem;
    font-size: 12.5px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.4rem;
  }
  /* Drive kind chip: black bg, white text. Sits alongside the
     existing doc / image / contact / view-only chips defined in
     FileInfoBody so the inspector's per-kind cue is consistent
     across selections. */
  .kind-chip {
    color: #fff;
    text-transform: uppercase;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 3px;
    flex: 1;
    text-align: center;
  }
  .kind-chip.drive {
    background: #000;
    color: #fff;
  }
  .title {
    margin: 0 0 0.5rem 0;
    font-size: 16px;
    font-weight: 600;
    word-break: break-word;
  }
  .meta-grid {
    display: grid;
    grid-template-columns: 6.5em 1fr;
    gap: 2px 0.5rem;
    margin: 0.4rem 0 0.6rem 0;
    font-size: 14px;
  }
  .meta-grid .k { color: var(--text-secondary); }
  .meta-grid .v {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .meta-grid .v.path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
  /* `fullstack-73`: mirrors the FileInfoBody `.open` styling so the
     "Graph from here" affordance reads consistently across every
     inspector body. */
  .open {
    width: 100%;
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 5px 0;
    cursor: pointer;
    font: inherit;
    margin-top: 0.6rem;
  }
  .open:hover { border-color: var(--btn-hover); }
  .refs { margin: 0.8rem 0 0 0; }
  .refs h4 {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    margin: 0 0 0.25rem 0;
  }
  .hint {
    color: var(--text-secondary);
    font-size: 11.5px;
    margin: 0 0 0.5rem 0;
  }
  .hint code {
    font-family: ui-monospace, monospace;
    font-size: 12px;
    background: var(--bg-card);
    padding: 0 4px;
    border-radius: 3px;
  }
  .field {
    display: grid;
    grid-template-columns: 6.5em 1fr;
    gap: 0.5rem;
    align-items: center;
    margin: 0.25rem 0;
  }
  .field > span { color: var(--text-secondary); font-size: 14px; }
  .field input {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 4px 7px;
    font: inherit;
    font-size: 14px;
    outline: none;
    width: 100%;
  }
  .field input:focus { border-color: var(--link); }
  .err-line {
    color: var(--warn-text);
    font-size: 13px;
    margin: 0.25rem 0;
  }
  .recents-head {
    margin: 0.6rem 0 0.25rem 0;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
  }
  .recents {
    list-style: none;
    padding: 0;
    margin: 0 0 0.4rem 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .recents li {
    display: grid;
    grid-template-columns: 12em auto 1fr;
    gap: 0.6rem;
    font-size: 13px;
    color: var(--text);
    align-items: baseline;
  }
  .recents-time {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .recents-name { color: var(--text); font-weight: 500; }
  .recents-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
