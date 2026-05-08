// Shared lazy-loaded graph view, used by both GraphPanel (canvas)
// and FileInfoBody (browser inspector references). One network call
// per session-or-watch-event; both consumers read the same payload.
//
// The graph is small per chan (a single drive's wiki/tag/mention
// edges), so caching the whole thing is cheaper than per-file
// metadata round-trips. Watcher events invalidate the cache so a
// freshly-saved tag shows up in the inspector without a manual
// reload.

import { api } from "../api/client";
import type { GraphView, GraphViewEdge, GraphViewNode } from "../api/types";

type GraphState = {
  view: GraphView | null;
  loading: boolean;
  error: string | null;
};

export const graphData = $state<GraphState>({
  view: null,
  loading: false,
  error: null,
});

let inflight: Promise<void> | null = null;

/// Fetch the graph if we don't already have it. Idempotent;
/// concurrent callers share a single network round-trip. After a
/// successful fetch (or a cached hit) the promise resolves; on
/// failure `graphData.error` is set and the promise still resolves
/// (consumers should branch on `graphData.view`).
export function ensureGraphLoaded(): Promise<void> {
  if (graphData.view && !graphData.error) return Promise.resolve();
  if (inflight) return inflight;
  graphData.loading = true;
  graphData.error = null;
  inflight = (async () => {
    try {
      graphData.view = await api.graph();
    } catch (e) {
      graphData.error = (e as Error).message;
    } finally {
      graphData.loading = false;
      inflight = null;
    }
  })();
  return inflight;
}

/// Drop the cached graph so the next `ensureGraphLoaded` (or
/// `reloadGraph`) re-fetches. Called from the watcher on filesystem
/// events.
export function invalidateGraph(): void {
  graphData.view = null;
  graphData.error = null;
}

/// Re-fetch unconditionally. Useful when we know the payload is
/// stale (after `invalidateGraph`) and a consumer is currently
/// looking at it.
export function reloadGraph(): Promise<void> {
  invalidateGraph();
  return ensureGraphLoaded();
}

/// Outgoing-edge groupings for a file path. Mirrors the inline
/// derivation that previously lived in GraphPanel.svelte. Returns
/// empty arrays if the graph hasn't loaded yet or the file has no
/// node in the graph (e.g. non-markdown files).
export function selectionEdgesFor(path: string): {
  tags: GraphViewNode[];
  mentions: GraphViewNode[];
  dates: GraphViewNode[];
  links: GraphViewNode[];
} {
  const out = {
    tags: [] as GraphViewNode[],
    mentions: [] as GraphViewNode[],
    dates: [] as GraphViewNode[],
    links: [] as GraphViewNode[],
  };
  const view = graphData.view;
  if (!view) return out;
  const fileNode = view.nodes.find(
    (n) => n.kind === "file" && n.path === path,
  );
  if (!fileNode) return out;
  const nodeById = new Map(view.nodes.map((n) => [n.id, n]));
  for (const e of view.edges) {
    if (e.source !== fileNode.id) continue;
    const target = nodeById.get(e.target);
    if (!target) continue;
    if (e.kind === "tag") out.tags.push(target);
    else if (e.kind === "mention") out.mentions.push(target);
    else if (e.kind === "date") out.dates.push(target);
    else if (e.kind === "link") out.links.push(target);
  }
  return out;
}

/// Re-export so consumers don't need a second import.
export type { GraphViewEdge, GraphViewNode };
