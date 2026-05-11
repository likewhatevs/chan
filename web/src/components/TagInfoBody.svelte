<script lang="ts">
  // Inspector body for non-file graph nodes (tags, mentions, dates).
  // Lifted from the inline panel that GraphPanel renders for the same
  // node kinds, so search and graph hosts can share one component.
  //
  // Reads the document list from the global graph store (loaded
  // lazily via ensureGraphLoaded). Caller supplies the node id and a
  // navigate callback used when the user clicks a referencing
  // document.

  import {
    documentsReferencing,
    ensureGraphLoaded,
    graphData,
    type GraphViewNode,
  } from "../state/graphData.svelte";

  let {
    nodeId,
    label,
    kind,
    onClose,
    onNavigate,
    documentsOverride,
  }: {
    nodeId: string;
    label: string;
    kind: "tag" | "mention" | "date";
    onClose?: () => void;
    /// Click handler for a referencing document. Receives the doc's
    /// path. Hosts decide whether to open it in the active pane and
    /// close themselves; absent = entries render as non-clickable.
    onNavigate?: (path: string) => void;
    /// Optional scope-filtered document list. When provided, this
    /// replaces the full-graph `documentsReferencing(nodeId)` lookup.
    /// GraphPanel passes its scope-filtered selectionEdges.documents
    /// so the tag inspector only lists docs visible in the rendered
    /// subgraph; search overlay leaves this unset for the full list.
    documentsOverride?: GraphViewNode[];
  } = $props();

  // Make sure the graph is loaded before we try to look up
  // referencing documents. ensureGraphLoaded is idempotent and
  // shared with GraphPanel / FileInfoBody, so this is essentially a
  // no-op once the user has touched any graph-aware surface.
  $effect(() => {
    void ensureGraphLoaded();
  });

  const documents = $derived<GraphViewNode[]>(
    documentsOverride ?? documentsReferencing(nodeId),
  );

  /// Background color for the kind chip. Mirrors the graph palette
  /// (--g-tag etc.) so search and graph chips are visually identical.
  const chipColor = $derived(
    kind === "tag"
      ? "var(--g-tag)"
      : kind === "mention"
        ? "var(--warn-text)"
        : "var(--info-text)",
  );

  function navigate(node: GraphViewNode): void {
    if (node.kind !== "file" || !onNavigate) return;
    onNavigate(node.path);
  }
</script>

<div class="info">
  <header class="head">
    <span class="kind-chip" style="background: {chipColor}">{kind}</span>
    {#if onClose}
      <button class="close" onclick={onClose} aria-label="clear selection">×</button>
    {/if}
  </header>
  <h3 class="title">{label}</h3>
  <div class="meta-grid">
    <span class="k">documents</span>
    <span class="v">{documents.length}</span>
  </div>

  {#if !graphData.view && graphData.loading}
    <div class="muted">loading references…</div>
  {:else if graphData.error}
    <div class="err">references unavailable: {graphData.error}</div>
  {:else if documents.length === 0}
    <div class="muted">no documents reference this</div>
  {:else}
    <section class="refs">
      <h4>Documents</h4>
      <ul>
        {#each documents as d (d.id)}
          <li>
            {#if d.kind === "file" && !d.missing && onNavigate}
              <button class="ref" onclick={() => navigate(d)}>{d.label}</button>
            {:else}
              <span class="ref" class:missing={d.kind === "file" && d.missing}>{d.label}</span>
            {/if}
          </li>
        {/each}
      </ul>
    </section>
  {/if}
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
  .meta-grid .v { color: var(--text); font-variant-numeric: tabular-nums; }
  .muted {
    color: var(--text-secondary);
    font-size: 13px;
    margin-top: 0.4rem;
    font-style: italic;
  }
  .err {
    color: var(--warn-text);
    font-size: 13px;
    margin-top: 0.4rem;
  }
  .refs { margin: 0.6rem 0 0 0; }
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
  button.ref { cursor: pointer; }
  button.ref:hover {
    border-color: var(--btn-hover);
    background: var(--hover-bg);
  }
  .ref.missing { color: var(--text-secondary); font-style: italic; }
</style>
