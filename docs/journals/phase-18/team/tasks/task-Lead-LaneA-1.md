# task Lead -> LaneA (1): Editor

You are @@LaneA - Editor lane. Round-1, Wave 1. START NOW.

## Read first (context lives here, not in this poke)
- Process: docs/journals/phase-18/team/bootstrap.md
- Plan + your lane section + gate/quality bar + shared-file table:
  docs/journals/phase-18/team/round-1-plan.md  (section "@@LaneA - Editor")
- Verbatim spec: docs/journals/phase-18/round-1/draft.md  (section "### Editor")
- Re-verify all line anchors against HEAD; they came from a recon pass and drift.

## Wave 1 scope (4 items, all in your owned files)
1. Bullet + hyphen list cursor/indent/click parity with ENUMERATED lists
   (arrow-down lands cursor BEFORE the glyph today).
2. Restore distinct HYPHEN (`-`) lists (phase-17 regressed them into bullets;
   the google-docs change was bullets-only).
3. Trackpad free-scroll hang (reproduce first, then fix).
4. `[[` workspace-PATH autocomplete.

## Owned files (edit ONLY these)
web/src/editor/decorations/blocks.ts, web/src/editor/commands/list.ts,
web/src/editor/bubbles/{triggers.ts,wiki.ts}, web/src/editor/widgets/image.ts,
web/src/editor/Wysiwyg.svelte.

## HOLD on item 4 cross-lane touch
`[[` currently queries /api/link-targets (chan-server). @@Alex is being surveyed
NOW on whether `[[` returns workspace PATHS only / existing link-targets / both.
That decides if a chan-server route change is needed (cross-lane -> route through
@@Lead, do NOT edit any route yourself). Do items 1-3 first; recon item 4's
client side; HOLD the route decision for my survey answer.

## Gate before any "done" report (plan "Gate + quality bar")
make web-check (vitest) + svelte-check + npm run build. Browser-smoke any
Svelte-5 $state/$derived reactivity change (static gates miss runtime errors).

## On completion
Cut docs/journals/phase-18/team/tasks/task-LaneA-Lead-1.md (scoped own-gate-green
+ a pathspec sha for your changes + a 1-line per-item status), then poke me:
  cs terminal write --tab-name=@@Lead --submit=claude \
    $'poke from @@LaneA: done; read docs/journals/phase-18/team/tasks/task-LaneA-Lead-1.md'
Keep a running log in docs/journals/phase-18/team/journals/journal-LaneA.md.
Flag ANY shared-file touch to @@Lead BEFORE landing.
