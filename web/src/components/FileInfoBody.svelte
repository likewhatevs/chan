<script lang="ts">
  // Inspector body that renders metadata for a single file or folder.
  // Looks the entry up from the global tree by path; renders nothing
  // until a path is supplied (callers that want a placeholder pass
  // their own empty state outside this component, or pass `null`
  // and the host's body slot stays empty).
  //
  // Used by:
  //   - FileBrowserOverlay: shows the current selection
  //     (browserSelection.path) plus an Open / × pair so the panel
  //     doubles as the action surface for the tree. References
  //     section (tags / mentions / dates / links / backlinks) is
  //     enabled here via showRefs.
  //   - FileEditorTab: shown inside a "show info" disclosure for the
  //     currently-edited file; lean layout (no Open/Close, no refs).
  //
  // Folder mode walks the flat tree to compute aggregate counts +
  // size + most-recent mtime. The walk is O(N) in tree size and only
  // re-runs when the selected path changes ($derived dependency
  // tracking does the gating).

  import { api, withTokenQuery } from "../api/client";
  import type { GraphEdge } from "../api/types";
  import { isEditableText, isImage } from "../state/fileTypes";
  import { basename, formatMtime, formatSize } from "../state/format";
  import {
    ensureGraphLoaded,
    graphData,
    selectionEdgesFor,
  } from "../state/graphData.svelte";
  import { openImageZoom } from "../state/imageZoom";
  import { openGraphAtNode, tree } from "../state/store.svelte";

  /// Visual / behavioural kind for a file reference. Images route to
  /// the fullscreen zoom overlay (editor's "Zoom" button shares the
  /// same helper); contacts are markdown notes under `Contacts/` and
  /// open in the editor like other docs but get their own chip color
  /// so a glance distinguishes them. Anything else is a doc.
  type RefKind = "doc" | "image" | "contact";
  function classifyRef(path: string): RefKind {
    if (isImage(path)) return "image";
    if (path === "Contacts" || path.startsWith("Contacts/")) return "contact";
    return "doc";
  }

  let {
    path,
    onOpen,
    onReveal,
    onClose,
    showRefs = false,
    onNavigate,
  }: {
    path: string | null;
    onOpen?: () => void;
    /// Image-inspector counterpart to `onOpen`. Renders a
    /// "Show in file browser" button on image entries; the host
    /// reveals the path in its tree and closes itself. Absent = no
    /// button (e.g. when the inspector already lives inside the
    /// file browser).
    onReveal?: () => void;
    onClose?: () => void;
    /// When true, fetch + render tags / mentions / dates / links /
    /// backlinks for files. Off by default so the file editor's
    /// inline disclosure stays compact.
    showRefs?: boolean;
    /// Click handler for a link / backlink target. Receives the
    /// peer file path. Hosts decide whether to open it in the active
    /// pane and close themselves; absent = entries render as
    /// non-clickable.
    onNavigate?: (path: string) => void;
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

  /// Outgoing edges (tags / mentions / dates / links) come straight
  /// out of the shared graph store; null while the graph hasn't
  /// loaded yet so the template can render a "loading" line.
  const refs = $derived.by(() => {
    if (!showRefs || !entry || entry.is_dir) return null;
    if (!graphData.view) return null;
    return selectionEdgesFor(entry.path);
  });

  /// Backlinks (incoming `link` edges) live behind a separate
  /// endpoint (/api/backlinks/{path}) since the graph snapshot only
  /// records outgoing edges per file node. Refetched whenever the
  /// selected path changes; stale responses are dropped via the
  /// request-id guard.
  let backlinks = $state<GraphEdge[]>([]);
  let backlinksLoading = $state(false);
  let backlinksError = $state<string | null>(null);
  let backlinkReq = 0;

  $effect(() => {
    if (!showRefs || !entry || entry.is_dir) {
      backlinks = [];
      backlinksLoading = false;
      backlinksError = null;
      return;
    }
    void ensureGraphLoaded();
    const req = ++backlinkReq;
    const target = entry.path;
    backlinksLoading = true;
    backlinksError = null;
    void api
      .backlinks(target)
      .then((edges) => {
        if (req !== backlinkReq) return;
        backlinks = edges.filter((e) => e.kind === "link");
        backlinksLoading = false;
      })
      .catch((err: unknown) => {
        if (req !== backlinkReq) return;
        backlinks = [];
        backlinksLoading = false;
        backlinksError = (err as Error).message;
      });
  });

  function navigate(targetPath: string): void {
    if (!targetPath) return;
    onNavigate?.(targetPath);
  }
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
    <h3 class="title" title={entry.path || "/"}>{basename(entry.path) || "(root)"}</h3>
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
  {@const image = isImage(entry.path)}
  <div class="info">
    <header class="head">
      <span class="kind-chip file" class:image class:view-only={!editable && !image}>
        {image ? "image" : editable ? "file" : "view-only"}
      </span>
      {#if onClose}
        <button class="close" onclick={onClose} aria-label="clear selection">×</button>
      {/if}
    </header>
    <h3 class="title" title={entry.path}>{basename(entry.path)}</h3>
    {#if image}
      <!-- Inline preview. Bytes come from /api/files with the
           per-launch bearer token appended as a query param so the
           browser's <img> can fetch without a custom Authorization
           header. Object-fit contains so portrait + landscape both
           sit cleanly in the fixed-height frame.
           Wrapped in a button so a click on the preview opens the
           shared fullscreen zoom overlay (matches the editor's
           image action overlay). -->
      <button
        type="button"
        class="image-preview"
        title="Zoom"
        onclick={() => openImageZoom(entry.path)}
      >
        <img
          src={withTokenQuery(`/api/files/${encodeURIComponent(entry.path).replace(/%2F/g, "/")}`)}
          alt={basename(entry.path)}
          loading="lazy"
        />
      </button>
    {/if}
    <div class="meta-grid">
      <span class="k">size</span>
      <span class="v">{formatSize(entry.size)}</span>
      <span class="k">modified</span>
      <span class="v">{formatMtime(entry.mtime)}</span>
      {#if showRefs && !image}
        <span class="k">tags</span>
        <span class="v">{refs ? refs.tags.length : "…"}</span>
        <span class="k">mentions</span>
        <span class="v">{refs ? refs.mentions.length : "…"}</span>
        <span class="k">dates</span>
        <span class="v">{refs ? refs.dates.length : "…"}</span>
        <span class="k">links out</span>
        <span class="v">{refs ? refs.links.length : "…"}</span>
        <span class="k">backlinks</span>
        <span class="v">{backlinksLoading ? "…" : backlinks.length}</span>
      {:else if showRefs && image}
        <span class="k">linked from</span>
        <span class="v">{backlinksLoading ? "…" : backlinks.length}</span>
      {/if}
    </div>
    {#if image}
      {#if onReveal}
        <button class="open" onclick={onReveal}>Show in file browser</button>
      {/if}
    {:else if onOpen}
      {#if editable}
        <button class="open" onclick={onOpen}>Open in this pane</button>
      {:else}
        <p class="view-only-hint">
          Not an editable text file. Only .md and .txt open in the editor.
        </p>
      {/if}
    {/if}
    {#if showRefs}
      {#if !graphData.view && graphData.loading}
        <div class="refs-loading">loading references…</div>
      {:else if graphData.error}
        <div class="refs-error">references unavailable: {graphData.error}</div>
      {:else if refs}
        {#if refs.tags.length > 0}
          <section class="refs">
            <h4>Tags</h4>
            <ul>
              {#each refs.tags as t (t.id)}
                <li>
                  <button
                    class="ref tag"
                    onclick={() => openGraphAtNode(t.id)}
                    title="open in graph"
                  >{t.label}</button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}
        {#if refs.mentions.length > 0}
          <section class="refs">
            <h4>Mentions</h4>
            <ul>
              {#each refs.mentions as m (m.id)}
                <li>
                  <button
                    class="ref mention"
                    onclick={() => openGraphAtNode(m.id)}
                    title="open in graph"
                  >{m.label}</button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}
        {#if refs.dates.length > 0}
          <section class="refs">
            <h4>Dates</h4>
            <ul>
              {#each refs.dates as d (d.id)}
                <li>
                  <button
                    class="ref date"
                    onclick={() => openGraphAtNode(d.id)}
                    title="open in graph"
                  >{d.label}</button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}
        {#if refs.links.length > 0}
          <section class="refs">
            <h4>Links to</h4>
            <ul>
              {#each refs.links as l (l.id)}
                <li>
                  {#if l.kind !== "file"}
                    <span class="ref file">{l.label}</span>
                  {:else if l.missing}
                    <span class="ref file missing" data-refkind={classifyRef(l.path)}>{l.label}</span>
                  {:else if classifyRef(l.path) === "image"}
                    <button
                      class="ref file"
                      data-refkind="image"
                      title="Zoom"
                      onclick={() => openImageZoom(l.path)}
                    >{l.label}</button>
                  {:else if onNavigate}
                    <button
                      class="ref file"
                      data-refkind={classifyRef(l.path)}
                      onclick={() => navigate(l.path)}
                    >{l.label}</button>
                  {:else}
                    <span class="ref file" data-refkind={classifyRef(l.path)}>{l.label}</span>
                  {/if}
                </li>
              {/each}
            </ul>
          </section>
        {/if}
        {#if backlinks.length > 0}
          <section class="refs">
            <h4>Backlinks</h4>
            <ul>
              {#each backlinks as b (b.src)}
                <li>
                  {#if classifyRef(b.src) === "image"}
                    <button
                      class="ref file"
                      data-refkind="image"
                      title="Zoom"
                      onclick={() => openImageZoom(b.src)}
                    >{b.src}</button>
                  {:else if onNavigate}
                    <button
                      class="ref file"
                      data-refkind={classifyRef(b.src)}
                      onclick={() => navigate(b.src)}
                    >{b.src}</button>
                  {:else}
                    <span class="ref file" data-refkind={classifyRef(b.src)}>{b.src}</span>
                  {/if}
                </li>
              {/each}
            </ul>
          </section>
        {:else if backlinksError}
          <div class="refs-error">backlinks unavailable: {backlinksError}</div>
        {/if}
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
    font-size: 14px;
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
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 3px;
    flex: 1;
    text-align: center;
  }
  .kind-chip.file { background: var(--link); }
  .kind-chip.file.view-only { background: var(--text-secondary); }
  .kind-chip.file.image { background: var(--g-img); }
  .kind-chip.dir { background: var(--accent); }
  /* Image preview frame: fixed max height, checkered fallback bg
     (visible while bytes are loading or for images with alpha so
     the panel doesn't show empty space). object-fit contain keeps
     the natural aspect ratio so portraits and landscapes both fit. */
  /* Image preview frame: fixed max height, checkered fallback bg
     (visible while bytes are loading or for images with alpha so
     the panel doesn't show empty space). object-fit contain keeps
     the natural aspect ratio so portraits and landscapes both fit.
     Rendered as a <button> so a click opens the fullscreen zoom
     overlay; default button chrome is stripped. */
  .image-preview {
    margin: 0 0 0.6rem 0;
    padding: 4px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    max-height: 220px;
    overflow: hidden;
    width: 100%;
    cursor: zoom-in;
    font: inherit;
    color: inherit;
  }
  .image-preview:hover { border-color: var(--btn-hover); }
  .image-preview img {
    max-width: 100%;
    max-height: 210px;
    object-fit: contain;
    display: block;
    pointer-events: none;
  }
  .view-only-hint {
    color: var(--text-secondary);
    font-size: 14px;
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
  /* Reference sections (tags / mentions / dates / links / backlinks).
     Visual style mirrors the graph panel's aside so the two
     inspectors feel like one feature. */
  .refs {
    margin: 0.6rem 0 0 0;
  }
  .refs h4 {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    margin: 0 0 0.25rem 0;
  }
  .refs ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .refs li { margin: 0; }
  .ref {
    display: block;
    width: 100%;
    text-align: left;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 2px 6px;
    font-size: 13px;
    color: var(--text);
    cursor: default;
    font: inherit;
    line-height: 1.5;
    word-break: break-word;
  }
  button.ref {
    cursor: pointer;
  }
  button.ref:hover {
    border-color: var(--btn-hover);
    background: var(--hover-bg);
  }
  .ref.tag { color: var(--accent); }
  .ref.mention { color: var(--warn-text); }
  .ref.date { color: var(--info-text); }
  /* Backlinks / link targets: use the standard text color rather
     than --link. The chip already reads as a clickable button
     thanks to the surface + hover treatment, and the doc/file
     name is the actual content, not a call-to-action.
     Per-kind left accent (doc / image / contact) lets the reader
     scan the list and pick out images vs contacts vs plain docs
     without a per-row icon. Padding is fixed so swapping accent
     widths doesn't shift the label. */
  .ref.file {
    color: var(--text);
    word-break: break-all;
    border-left-width: 3px;
    padding-left: 6px;
  }
  .ref.file[data-refkind="doc"] { border-left-color: var(--g-doc); }
  .ref.file[data-refkind="image"] { border-left-color: var(--g-img); }
  .ref.file[data-refkind="contact"] { border-left-color: var(--info-text); }
  .ref.file.missing {
    color: var(--text-secondary);
    font-style: italic;
  }
  .refs-loading,
  .refs-error {
    color: var(--text-secondary);
    font-size: 13px;
    margin-top: 0.6rem;
    font-style: italic;
  }
  .refs-error { color: var(--warn-text); font-style: normal; }
</style>
