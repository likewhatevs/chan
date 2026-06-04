# task LaneA -> Lead (2): item 4 DONE (client-side) + Source.svelte parallel fix

Editor lane complete. Item 4 built CLIENT-SIDE off api.list per your revised
task-3 (graph.rs sequencing moot, ZERO @@LaneB dependency, /api/link-targets
untouched). Source.svelte:461 parallel scroll fix applied. All in my owned
files. Items 1-3 unchanged from task-1 (already accepted).

## Scoped own-gate: GREEN
- svelte-check: 0 ERRORS (1 WARNING = pre-existing RichPrompt.svelte a11y,
  @@LaneE's file, not mine).
- vitest 54/54 across my 3 test files (blocks.test, list.test,
  wikiLinkTargets.test).
- npm run build: OK.
- Full-tree `make web-check` still has the @@LaneC fileTreeSelectionMenu.test.ts
  red (peer WIP) - unchanged, not mine.

## Pathspec (supersedes task-1's; 9 files, for your Wave-3 commit)
- base HEAD: d5f7dd38
- `git diff -- <9 files> | git hash-object --stdin` = 1f55ffc8ace0616373d8622b29cbd60eae9cffb3
- files:
  - items 1-3: web/src/editor/decorations/blocks.ts(+test),
    web/src/editor/commands/list.ts(+test), web/src/editor/Wysiwyg.svelte
  - item 4: web/src/editor/bubbles/wiki.ts, web/src/api/types.ts,
    web/src/editor/bubbles/wikiLinkTargets.test.ts
  - Source parallel fix: web/src/editor/Source.svelte
- NO shared-file touch. web/src/api/types.ts: I only ADDED "Path" to the
  LinkTarget.kind union (additive; only consumer that branches on kind is
  wiki.ts, which I own). Flagging it since types.ts is a shared type file -
  no other lane edits LinkTarget, so no contention, but your call on commit
  grouping.

## Item 4 - `[[` workspace-path autocomplete: DONE (client-side, BOTH)
- Design (your task-3 + my recon): keep /api/link-targets UNCHANGED; add
  workspace-PATH candidates CLIENT-SIDE off GET /api/files (api.list, the
  recursive listing the file browser already uses), merged in bubbles/wiki.ts.
  No chan-server route change, no chan-workspace/graph.rs change, no @@LaneB
  contention. (DISREGARDED the task-2 graph.rs/link_targets backend plan and
  the held write entirely, as you directed.)
- Mechanics: LinkTarget.kind gains a client-synthesized "Path"; computePathHits
  filters the cached file tree (rank-1 path-prefix so `[[docs` already surfaces
  `docs/...`; + contains once the query has a `/`); tree fetched at most once
  per `[[` session; merged after the link-target hits and deduped against
  same-path file rows; rendered with a "PATH" tag + full path; commit/open
  treat Path like File. No triggers.ts change needed (`/` is a legal `[[`
  query char already).
- Decision (flagging): files-only, NO directory rows. A file row shows its
  full path so the user drills by typing more of the path; committing a
  directory link would be unresolvable. If @@Alex wants explicit directory
  drill-down rows later, that's a bounded follow-up.
- Smoked (Chrome, throwaway drive): `[[docs/` -> 2 PATH rows; `[[carb` ->
  File + H1 Heading link-targets (existing half, no path-noise dupes);
  `[[docs/phases/ph`+Enter -> committed `[phase-17](../docs/phases/phase-17.md)`
  (relativized). BOTH halves work.

## Item 3 Source.svelte parallel: DONE
- Removed the identical `scroll-behavior: smooth` from `.md-source
  .cm-scroller` (Source.svelte) - same fix as Wysiwyg, for source-mode
  parity. Smoked: source-mode scrollBehavior now "auto", scrollTop applies
  instantly.

## Hand-smoke for @@Alex (I can't drive; Blink has no trackpad momentum)
- Item 3 definitive "no stall" on a REAL trackpad, BOTH Wysiwyg and now
  source mode (Chrome or chan-desktop).

Journal: docs/journals/phase-18/team/journals/journal-LaneA.md
All four Editor items + the Source parallel fix are own-gate-green and
smoked. Ready for your Wave-3 commit. No open blockers from my lane.
