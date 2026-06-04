# task LaneA -> Lead (3): item 5 graph-link click-to-open DONE (Graph item 5 complete)

The round's last work item. Editor click-handler now opens `chan://graph?...`
links via @@LaneB's openGraphFromLink. In-lane (external_links.ts), import-only
of B's exports, no contention.

## Scoped own-gate: GREEN
- svelte-check: 0 ERRORS (1 WARNING = pre-existing RichPrompt a11y, @@LaneE's,
  not mine).
- vitest 64/64 across my 4 test files (blocks, list, wikiLinkTargets,
  external_links).
- npm run build: OK.

## Pathspec (supersedes task-2's; 11 files = prior 9 + external_links.ts/.test)
- base HEAD: d5f7dd38
- `git diff -- <11 files> | git hash-object --stdin` = 9fad907c151e8b07448e5bbdc17b39cdd2e43f9f
- new this task: web/src/editor/external_links.ts, external_links.test.ts
- No shared-file touch. Only IMPORT B's exports (openGraphFromLink from
  store.svelte:2175, GRAPH_LINK_PREFIX from tabs.svelte:3568). No import cycle
  (state/ does not import external_links; Wysiwyg already imports store).

## Item 5 - graph-link click-to-open: DONE
- external_links.ts click handler: before the external-URL path, the new
  linkUrlAtPos(state,pos) returns the raw URL of ANY scheme; if it starts with
  GRAPH_LINK_PREFIX and openGraphFromLink(raw) handles it, preventDefault +
  short-circuit. Otherwise unchanged.
- Refactored externalUrlForNode to reuse rawUrlForNode/rawUrlFromChild (DRY;
  identical behavior - the openable-scheme filter still gates externalUrlAtPos).
- No renderer change needed: handleLink (marks.ts) already classes any URL with
  a scheme as external -> applies `.cm-md-link`, so chan:// links are clickable.
- Tests: linkUrlAtPos pure tests (chan:// raw, external raw, image->null) +
  source-pins on the click routing. (external_links.test executes external_links
  -> store.svelte transitively; jsdom-safe, confirmed.)

## Smoked end-to-end (Chrome, linked-notes drive)
- Opened workspace Graph tab -> right-clicked the TAB -> "Copy link to graph"
  present, "Reload" GONE (B's removal verified) -> wrote the matching
  `[..](chan://graph?s=workspace&m=s&f=2ltmaifds)` into a note -> renders as a
  clickable link -> CLICK opened a NEW graph tab (path=workspace, semantic,
  7/7 nodes/13 edges, scope+mode+filters restored). @@Alex's spec ("copy these
  links into a markdown file and open the graph tab on click") met. External
  link path intact.

## Hand-smoke for @@Alex (unchanged, can't drive in Blink)
- Item 3 real-trackpad "no stall", BOTH Wysiwyg and source mode.

Journal: docs/journals/phase-18/team/journals/journal-LaneA.md
Editor lane fully done: items 1-4 + Source parallel fix + item 5 graph-link
hook. All own-gate-green + smoked. Ready for your convergence/Wave-3. No open
blockers from my lane.
