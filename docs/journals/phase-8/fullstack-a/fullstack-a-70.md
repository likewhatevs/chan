# fullstack-a-70 — Editor mention/matching gap (@@<Name> not matched even when @@ exists elsewhere in repo)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Fix the mention-matching gap surfaced by @@Alex
in [`../alex/addendun-a.md`](../alex/addendun-a.md):

> Mentions and matching: we've got tons of tags to my
> name in this repo and yet when I'm on the text
> editor I get this: ![](./image.png#w=250)

Screenshot `addendun-a/image.png` shows the editor's
mention completion / matching feature failing on a
name that has many existing references in the repo.

## Reference

* `addendun-a.md` "## Bugs" — mentions item.
* Screenshot at `docs/journals/phase-8/alex/image.png`.

## Audit hooks

1. Inspect the editor's mention-completion path —
   most likely lives in the WYSIWYG / Source editor
   extensions (`web/src/editor/*`).
2. Trace the data source: does the matcher query
   chan-server (mention-index) or only local
   in-file state?
3. Compare expected behavior (auto-complete on `@@`
   from the corpus of known handles) vs observed
   (the screenshot shows no match).

Likely root cause: matcher only looks at a stale
local cache OR doesn't include the mention-index that
chan-server emits via the graph route.

## Acceptance

1. **Typing `@@<partial>` in the editor** brings up
   matches from the existing mention corpus (the same
   handles that contributed to the deduped contact
   nodes via `systacean-22`).
2. **Match list includes all 49+ unique handles** in
   the chan-source seed.
3. **Selection completes** the mention to the chosen
   handle.

### Tests

Vitest pin on the mention-completion query + match
list shape.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  green.

## Coordination

* @@FullStackA SPA primary.
* If audit reveals chan-server emit gap (e.g. mention
  corpus not exposed), fire scope poke + I route the
  chan-server piece.

## Authorization

Yes for editor + mention-completion SPA files + tests
+ task tail + outbound. Scope-poke if chan-server
changes needed.

## Numbering

This is `-a-70`.
