<script lang="ts">
  // Inspector body that renders metadata for a single file or folder.
  // Looks the entry up from the global tree by path; renders nothing
  // until a path is supplied (callers that want a placeholder pass
  // their own empty state outside this component, or pass `null`
  // and the host's body slot stays empty).
  //
  // Used by:
  //   - FileBrowserTab: shows the current selection (browserSelection.path)
  //     plus an Open / × pair so the panel doubles as the action
  //     surface for the tree.
  //   - FileEditorTab: shown inside a "show info" disclosure for the
  //     currently-edited file; no Open/Close buttons (the file is
  //     already open; the inspector itself is closed via the toggle).
  //
  // Folder mode walks the flat tree to compute aggregate counts +
  // size + most-recent mtime. The walk is O(N) in tree size and only
  // re-runs when the selected path changes ($derived dependency
  // tracking does the gating).

  import { isEditableText } from "../state/fileTypes";
  import { basename, formatMtime, formatSize } from "../state/format";
  import { tree } from "../state/store.svelte";

  let {
    path,
    onOpen,
    onClose,
  }: {
    path: string | null;
    onOpen?: () => void;
    onClose?: () => void;
  } = $props();

  const entryByPath = $derived(
    new Map(tree.entries.map((e) => [e.path, e])),
  );

  const entry = $derived(path ? (entryByPath.get(path) ?? null) : null);

  const dirStats = $derived.by(() => {
    if (!entry || !entry.is_dir) return null;
    const prefix = entry.path ? `${entry.path}/` : "";
    let files = 0;
    let dirs = 0;
    let bytes = 0;
    let latest: number | null = null;
    for (const e of tree.entries) {
      if (prefix && !e.path.startsWith(prefix)) continue;
      if (e.path === entry.path) continue;
      if (e.is_dir) dirs += 1;
      else {
        files += 1;
        bytes += e.size;
      }
      if (e.mtime !== null && (latest === null || e.mtime > latest)) {
        latest = e.mtime;
      }
    }
    return { files, dirs, bytes, latest };
  });
</script>

{#if !entry}
  <div class="empty">
    <div class="empty-title">Details</div>
    <div class="empty-hint">click a file or folder to inspect</div>
  </div>
{:else if entry.is_dir}
  <div class="info">
    <header class="head">
      <span class="kind-chip dir">folder</span>
      {#if onClose}
        <button class="close" onclick={onClose} aria-label="clear selection">×</button>
      {/if}
    </header>
    <h3 class="title">{basename(entry.path) || "(root)"}</h3>
    <div class="path mono">{entry.path || "/"}</div>
    {#if dirStats}
      <div class="meta-grid">
        <span class="k">files</span>
        <span class="v">{dirStats.files}</span>
        <span class="k">subfolders</span>
        <span class="v">{dirStats.dirs}</span>
        <span class="k">size</span>
        <span class="v">{formatSize(dirStats.bytes)}</span>
        <span class="k">last change</span>
        <span class="v">{formatMtime(dirStats.latest)}</span>
      </div>
    {/if}
  </div>
{:else}
  {@const editable = isEditableText(entry.path)}
  <div class="info">
    <header class="head">
      <span class="kind-chip file" class:view-only={!editable}>
        {editable ? "file" : "view-only"}
      </span>
      {#if onClose}
        <button class="close" onclick={onClose} aria-label="clear selection">×</button>
      {/if}
    </header>
    <h3 class="title">{basename(entry.path)}</h3>
    <div class="path mono">{entry.path}</div>
    <div class="meta-grid">
      <span class="k">size</span>
      <span class="v">{formatSize(entry.size)}</span>
      <span class="k">modified</span>
      <span class="v">{formatMtime(entry.mtime)}</span>
    </div>
    {#if onOpen}
      {#if editable}
        <button class="open" onclick={onOpen}>Open in this pane</button>
      {:else}
        <p class="view-only-hint">
          Not an editable text file. Only .md and .txt open in the editor.
        </p>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .info {
    padding: 0.6rem 0.7rem 0.8rem 0.7rem;
    font-size: 12.5px;
  }
  .empty {
    text-align: center;
    color: var(--text-secondary);
    padding: 1.2rem 0.7rem 0.8rem 0.7rem;
  }
  .empty-title {
    font-weight: 600;
    color: var(--text);
    margin-bottom: 0.25rem;
  }
  .empty-hint {
    font-style: italic;
    font-size: 12px;
    opacity: 0.85;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.4rem;
  }
  .kind-chip {
    color: #fff;
    text-transform: uppercase;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 3px;
    flex: 1;
    text-align: center;
  }
  .kind-chip.file { background: var(--link); }
  .kind-chip.file.view-only { background: var(--text-secondary); }
  .kind-chip.dir { background: var(--accent); }
  .view-only-hint {
    color: var(--text-secondary);
    font-size: 12px;
    font-style: italic;
    margin: .4rem 0 0 0;
  }
  .close {
    background: transparent;
    border: 0;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    padding: 0 4px;
  }
  .close:hover { color: var(--text); }
  .title {
    margin: 0 0 0.15rem 0;
    font-size: 14px;
    font-weight: 600;
    word-break: break-word;
  }
  .path {
    color: var(--text-secondary);
    font-size: 11px;
    margin-bottom: 0.5rem;
    word-break: break-all;
  }
  .mono { font-family: ui-monospace, monospace; }
  .meta-grid {
    display: grid;
    grid-template-columns: 6.5em 1fr;
    gap: 2px 0.5rem;
    margin: 0.4rem 0 0.6rem 0;
    font-size: 12px;
  }
  .meta-grid .k { color: var(--text-secondary); }
  .meta-grid .v {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .open {
    width: 100%;
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 5px 0;
    cursor: pointer;
    font: inherit;
  }
  .open:hover { border-color: var(--btn-hover); }
</style>
