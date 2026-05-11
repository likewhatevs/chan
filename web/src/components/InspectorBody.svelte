<script lang="ts">
  // Dispatcher that renders the correct inspector body for whatever
  // the user picked. Replaces ad-hoc per-overlay dispatching so the
  // file browser, search overlay, and graph all share one component.
  //
  // Caller hands in a discriminated `selection` describing what was
  // clicked. Files (text + image) route to FileInfoBody — it already
  // branches internally on extension, so the search "image" hit and
  // the graph "image-ish file" node both get the same preview. Tag
  // / mention / date nodes route to TagInfoBody.

  import FileInfoBody from "./FileInfoBody.svelte";
  import TagInfoBody from "./TagInfoBody.svelte";
  import type { GraphViewNode } from "../state/graphData.svelte";

  export type InspectorSelection =
    | { kind: "file"; path: string }
    | {
        kind: "tag" | "mention" | "date";
        nodeId: string;
        label: string;
      }
    | null;

  let {
    selection,
    onOpen,
    onReveal,
    onClose,
    onNavigate,
    showRefs = true,
    documentsOverride,
  }: {
    selection: InspectorSelection;
    /// Forwarded to FileInfoBody as the "Open in this pane" handler.
    /// Tag bodies don't take an open action.
    onOpen?: () => void;
    /// Forwarded to FileInfoBody as the "Show in file browser"
    /// handler for image entries. Hosts that already live inside
    /// the file browser pass undefined.
    onReveal?: () => void;
    onClose?: () => void;
    onNavigate?: (path: string) => void;
    showRefs?: boolean;
    /// Forwarded to TagInfoBody. GraphPanel uses this to keep the
    /// tag inspector scoped to docs visible in the rendered subgraph.
    documentsOverride?: GraphViewNode[];
  } = $props();
</script>

{#if !selection}
  <div class="empty">
    <div class="empty-title">Details</div>
    <div class="empty-hint">click a result to inspect</div>
  </div>
{:else if selection.kind === "file"}
  <FileInfoBody
    path={selection.path}
    {onOpen}
    {onReveal}
    {onClose}
    {onNavigate}
    {showRefs}
  />
{:else}
  <TagInfoBody
    nodeId={selection.nodeId}
    label={selection.label}
    kind={selection.kind}
    {onClose}
    {onNavigate}
    {documentsOverride}
  />
{/if}

<style>
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
</style>
