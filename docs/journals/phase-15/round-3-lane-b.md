# Phase-15 round-3 - @@LaneB (Editor + Search frontend)

You are @@LaneB. Read `round-3-bootstrap.md` (process) and `round-3-status.md`
(active wave) first; the technical source is `round-3-plan.md` (Themes 3, 4, 6).
You own the editor and the search frontend. You MAY spawn subagents within
scope.

## Your files (no other lane edits these)

- web/src/editor/** (bubbles/wiki.ts, links.ts, empty_state.ts, triggers.ts,
  widgets/image.ts, ...)
- web/src/components/SearchPanel.svelte
- web/src/state/graphData.svelte.ts (Wave 3 graph frontend)
- docs/journals/** for the Theme-6 data cleanup (Wave 3, GATED on @@Host)

Do NOT edit backend search/index (that is @@LaneA), team/survey components
(@@LaneC), or the CLI/desktop/submitMode files (@@LaneD).

## Your work scope, by wave

### Wave 1 - relative links on disk + the `[[` stuck bubble

- On-disk links must be relative markdown, not wiki-links: `[[` completion emits
  `[](./path.md#anchor)` via links.ts `wikiLinkToMarkdown` (wiki.ts commit
  ~358-419, `linkTargetRef` ~374-379). KEEP existing wiki-links ONLY when the
  file already contains `[[` (per-file mode detected on load). New files /
  images / docs always emit relative markdown.
- `[[` stuck on "Indexing...": invalidate the cached empty-state so the bubble
  resolves when indexing completes (bubbles/wiki.ts + empty_state.ts ~19-39).
  When `[[` is hit in an unindexed file, prioritise the current file's
  directory for suggestions.
- Browser-smoke: completion writes relative markdown ON DISK; the stuck bubble
  clears. Your Wave-1 relative-link rule is a prerequisite for Theme 6 (Wave 3),
  so land it clean.

### Wave 2 - heading/block links + click-to-cursor + search FE

- Heading `#` / block `^` section links: browser-smoke whether truly regressed
  (round-3-plan.md notes the wiki.ts logic looks wired ~257-336, contradicting
  one exploration pass - verify, do not trust). Ensure they round-trip on disk
  as relative anchors and that search resolves them.
- Click-to-place-cursor: clicking blank space after the text on a line, or
  anywhere on a row, must place the caret (past-EOL -> the end of that line's
  text). The blockers are the image atomic ranges + default `posAtCoords`
  (widgets/image.ts ~290-296, ~741); add a line-level click handler.
- Search FE (Theme 4): SearchPanel handles `@@mention` / `path/to/file` / `.md`
  per @@LaneA's Wave-2 search-API contract. If A's probe shows semantic already
  covers it, this is display-only.

### Wave 3 - docs cleanup + graph frontend (GATED)

- Theme 6 (GATED on @@Host's delete-raw vs keep vs defer call - wait for
  @@Architect's go): a dedicated cleanup of docs/journals - summarize each phase
  into essence docs, transcribe + delete images, delete raw round data (it is in
  git history), tag outcomes (#reliability #features #bugfixes). Runs AFTER your
  Wave-1 relative-link rule so the cleanup emits relative links. Spawn a subagent
  for the bulk pass.
- Graph hygiene frontend (graphData.svelte.ts), paired with @@LaneA's backend
  ghost-node fix: the chan-source graph shows no ghost nodes + less clutter.

## Touch points

- A<->B (Wave 2): the search-API contract for mentions/paths (A provides).
- A<->B (Wave 3): graph ghost-node (A backend, you frontend graphData).
- Your Theme-6 cleanup depends on your own Wave-1 relative-link rule landing
  first.

## Completion (each wave)

Gated-green + local merge + journal entry + poke @@Architect "wave N done".
