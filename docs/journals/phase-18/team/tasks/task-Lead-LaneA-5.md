# task Lead -> LaneA (5): cross-lane follow-up - graph-link click-to-open

Small in-scope editor hook to complete Graph item 5. @@LaneB built the copy half
(graph tab menu "Copy link to graph" -> a `chan://graph?...` link) + the open
function; you wire the editor click-handler so such a link, pasted into a
markdown file, OPENS the graph tab on click. @@Alex's spec requires both halves
this round ("copy these links into a markdown file and open the graph tab on
click").

## What's already there (B's done exports - verified by me, just import)
- `openGraphFromLink(link: string): boolean` - web/src/state/store.svelte.ts:2175.
  Returns TRUE when it handled a graph link (so you fall through to normal
  link handling when false).
- `GRAPH_LINK_PREFIX = "chan://graph?"` - web/src/state/tabs.svelte.ts:3568
  (parseGraphLink at :3595 does the actual parse; openGraphFromLink wraps it).

## The hook (your editor domain)
- web/src/editor/external_links.ts is chan's editor link-click handler (the
  chan:// / external-link path; it has external_links.test.ts). In the click
  path, detect an href starting with GRAPH_LINK_PREFIX and call
  `openGraphFromLink(href)`; if it returns true, short-circuit (preventDefault /
  do NOT treat it as an external navigation). Otherwise fall through to existing
  behavior unchanged.
- Make sure a `chan://graph?...` href RENDERS as a clickable link in the editor
  (if the renderer currently drops/ą blocks the custom scheme, allow it through
  for this prefix). Your call on the cleanest spot (external_links.ts vs the
  wikilink/widget path) - you own the editor link surface.
- Extend external_links.test.ts to pin: a chan://graph href is intercepted +
  routed to openGraphFromLink (and a non-graph href is NOT).

## Ownership / contention
external_links.ts (+ its test) is editor domain = yours (coherent-domain rule;
no other lane owns it). You only IMPORT B's exports; B is done and does NOT touch
editor files. No contention.

## Gate + smoke
make web-check + svelte-check + npm run build. Browser-smoke: copy a graph
link from a graph tab's "Copy link to graph", paste into a markdown note, click
it -> the graph tab opens with the serialized scope/depth/mode/filters/selection.

## On completion
Cut task-LaneA-Lead-3.md (pathspec/fingerprint for the hook + smoke result),
poke me. This is the LAST work item of the round - after it lands I converge
(merge shared files + clean server + smokes + survey batch).
