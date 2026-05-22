# fullstack-a-63 — Graph chip count semantics: switch contact chip from edge-tally to node-tally (PARTIAL fix from webtest-a-8)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Fix the chip-count UX gap surfaced by `webtest-a-8`'s
PARTIAL verdict on `-22`: the contact chip displays
`1982` (mention-edge count) instead of `48` (mention-
node count after `-22`'s dedup-and-filter landing).

User experience: a user looking at the contact chip
would conclude "nothing changed" even though the
underlying graph composition is now ~40x cleaner.

## Reference

* `webtest-a-8` verdict (`7ecd18e`):
  [`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md)
  under "2026-05-22 — bundled walk: fullstack-a-62 +
  systacean-22" → "#5 Contact count drops" PARTIAL.
* Backing fix: `systacean-22` (`6443b98`) chan-server
  contact-file filter — empirically lands 48 mention
  nodes (vs ~1973 pre-fix) at the API level.

## Audit-confirmed root cause

`GraphPanel.svelte:550` `counts` Record builds per-
chip counts. The contact chip currently tallies
edges-touching-mention-nodes (or some variant
producing 1982 in the chan-source seed), not the
distinct mention-node count.

The "right" semantic for each chip:
* `tag` — count of distinct tag nodes (not tag edges).
* `mention/contact` — count of distinct contact
  nodes (not mention-edge fan-in).
* `language` — count of distinct language nodes.
* `media/img` — count of distinct media file nodes.
* `folder` — count of distinct folder nodes.

Many-to-one fan-in: 1982 mention edges across 48
contact nodes means the user expects "48 distinct
contacts in the graph" to show as `48`. Current `1982`
is the edge population.

## Fix shape

Audit + update the `counts` Record at
`GraphPanel.svelte:550-...`. Switch the contact chip
from whatever-it-tallies-today to:

```ts
counts.mention = uniqueMentionNodes.size;
```

where `uniqueMentionNodes` is the set of node ids
whose `kind === "mention"` in the response. Mirror
the pattern for any other chip currently doing an
edge-tally; markdown/source/etc. should already be
node-tallies since `-a-57` (audit-confirm at pickup).

## Scope

Bounded — audit the count loop + switch the offending
chip(s) from edge-tally to node-tally. Should be
~5-10 LOC. Plus test pin updates if existing chip-count
tests assert against edge-count semantics.

## Acceptance

1. **Contact chip count matches API node count**:
   on chan-source seed (no imported contacts), chip
   reads ~48 (down from 1982). Architect's prediction
   was ~49; @@WebtestA observed 48 at the API.
2. **Other chips audited**: tag/language/media/folder
   chips consistent with node-count semantics. If any
   is already node-tally, no change. If any is edge-
   tally, fix.
3. **Existing chip behavior preserved**: toggling a
   chip OFF still hides the right nodes. The count
   is purely display; toggle wiring unchanged.

### Tests

Vitest pin: `GraphFilters` count loop returns
node-count not edge-count. Fixture mention graph with
multiple edges per node; assert chip count = node
count, not edge count.

### Gate

* `npm test -- --run` green.
* `npm run check` 0e/0w.
* `npm run build` clean.

## Coordination

* @@FullStackA lane. SPA-only.
* Atomic-audit-commit discipline.

## Authorization

**Yes** for `web/src/components/GraphPanel.svelte` +
related test files + task tail + outbound.

## Numbering

Highest committed `fullstack-a-N` is `-62`. This is
`-63`.

## Out of scope

* Re-architecting the chip filter system (separate
  task `-a-57`).
* Sub-language picker (deferred from `-a-57`).
* Per-handle dedup for contact node `id` shape (the
  parser currently emits separate nodes for
  `@@Alex` / `@@Alex-driven` / `@@Alex-side`; that's
  separate parsing-strictness territory, not
  count-semantics). Flag for a future task if @@Alex
  wants per-canonical-handle dedup.
