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
  import LanguageInfoBody from "./LanguageInfoBody.svelte";
  import type { GraphViewNode } from "../state/graphData.svelte";

  export type InspectorSelection =
    | { kind: "file"; path: string }
    | { kind: "directory"; path: string; label?: string }
    | {
        kind: "tag" | "mention" | "date";
        nodeId: string;
        label: string;
      }
    // Language bubbles get a dedicated body (name + file/code stats +
    // "Graph from here").
    | {
        kind: "language";
        language: string;
        label: string;
        files?: number;
        code?: number;
      }
    | null;

  let {
    selection,
    onOpen,
    onReveal,
    onClose,
    onNavigate,
    onContactNavigate,
    onSetAsScope,
    showRefs = true,
    documentsOverride,
  }: {
    selection: InspectorSelection;
    /// Forwarded to FileInfoBody as the "Open" handler.
    /// Tag bodies don't take an open action.
    onOpen?: () => void;
    /// Forwarded to FileInfoBody as the "Show in file browser"
    /// handler for image entries. Hosts that already live inside
    /// the file browser pass undefined.
    onReveal?: () => void;
    onClose?: () => void;
    onNavigate?: (path: string) => void;
    /// Forwarded to FileInfoBody. Graph overlay binds this so a
    /// contact pill clicked in the file inspector selects that
    /// contact's node on the canvas instead of opening a new
    /// graph scoped to the contact.
    onContactNavigate?: (path: string) => void;
    /// "Graph from here" handler. Forwarded to FileInfoBody (files /
    /// images) and TagInfoBody (tag / mention). GraphPanel uses it
    /// to re-scope the current graph to the clicked entity; search
    /// overlay closes itself and opens a tag-scoped graph (file
    /// path doesn't make sense to scope from search, so the search
    /// host leaves it unbound for file selections).
    onSetAsScope?: () => void;
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
    {onContactNavigate}
    {onSetAsScope}
    {showRefs}
  />
{:else if selection.kind === "directory"}
  <!-- Folder parity (inspector-spec.md I3): route directory selections
       through FileInfoBody (its is_dir branch) so the graph folder
       inspector renders the SAME body as the File Browser folder
       inspector. FileInfoBody looks the entry up from the tree (loading
       the parent dir if needed) and prefers the O(1) /api/report/dir
       cache the old DirectoryInfoBody used. `label` carries the graph
       node's display name. `onReveal` spawns/focuses a File Browser tab
       for the folder on non-browser surfaces. -->
  <FileInfoBody
    path={selection.path}
    label={selection.label}
    {onReveal}
    {onSetAsScope}
    {onClose}
    {onNavigate}
  />
{:else if selection.kind === "language"}
  <!-- Language bubble inspector. `onSetAsScope` is the "Graph from
       here" affordance (graph host re-scopes to the language
       lens). -->
  <LanguageInfoBody
    language={selection.language}
    label={selection.label}
    files={selection.files}
    code={selection.code}
    {onSetAsScope}
  />
{:else}
  <TagInfoBody
    nodeId={selection.nodeId}
    label={selection.label}
    kind={selection.kind}
    {onClose}
    {onNavigate}
    {onSetAsScope}
    {onOpen}
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
