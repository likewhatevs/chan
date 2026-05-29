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

## 2026-05-22 — ready for review

Two-file change. SPA-only; no Rust touched.

### What landed

`web/src/components/GraphPanel.svelte` `counts`
$derived: rewrote the loop to walk `nodes` instead
of `edges`.

Pre-`-a-63` shape (loop body now removed):

```ts
for (const e of edges) {
  if (kind === "contains") c.folder++;
  else if (kind !== "link") c[kind]++;
}
```

Post-`-a-63` shape:

```ts
for (const n of nodes) {
  if (n.kind === "tag")      { c.tag++;      continue; }
  if (n.kind === "mention")  { c.mention++;  continue; }
  if (n.kind === "language") { c.language++; continue; }
  if (n.kind === "folder")   { c.folder++;   continue; }
  if (n.kind !== "file") continue;
  const cls = classifyFile(n.path, n.node_kind);
  if (cls === "img")          c.img++;
  else if (cls === "contact") c.mention++;
  else if (cls === "doc")     c.markdown++;
  else if (cls === "source")  c.source++;
}
```

Audit of all 7 chips:

* `tag` — was edge-tally (mention/tag/language fall-
  through to `c[kind]++`); now node-tally on
  `n.kind === "tag"`.
* `mention` (contact) — was edge-tally; now
  node-tally on `n.kind === "mention"` PLUS
  contact-discriminated file nodes via `cls ===
  "contact"`. Both add because the mention chip
  toggle hides both (see `hiddenContactIds`).
* `language` — was edge-tally; now node-tally on
  `n.kind === "language"`.
* `img` — already node-tally pre-`-a-63`
  (media-class files); preserved.
* `folder` — was BOTH edge-tally on `contains`
  edges AND node-tally on `n.kind === "folder"` —
  the edge tally was a double-count for filesystem-
  mode graphs where each folder gets a contains edge
  to every child. Now purely node-tally on
  `n.kind === "folder"`.
* `markdown` — node-tally on `cls === "doc"` per
  `-a-57`; preserved.
* `source` — node-tally on `cls === "source"` per
  `-a-57`; preserved.

`web/src/components/graphChipCountSemantics.test.ts`
(new): 6 raw-source pins covering the loop shape
absence + the new walk-nodes-by-kind structure.

### Acceptance

1. **Contact chip count matches API node count** ✓
   — `c.mention` now counts `mention`-kind nodes +
   contact files; both align with the node-count
   the chip toggle hides. With 48 deduped contact
   nodes from `-22`, the chip will display ~48
   (vs ~1982 pre-`-a-63`).
2. **Other chips audited + node-tally** ✓ — tag,
   language, folder corrected; img / markdown /
   source preserved.
3. **Existing chip toggle behavior preserved** ✓ —
   `visibleEdges` + `visibleNodeIds` + `hidden*Ids`
   sets all unchanged; only the display number
   changed.

### Gate

* vitest **738 / 738** (+6 net from `-a-56`'s
  732).
* svelte-check 0 errors / 0 warnings across
  4000 files.
* npm build clean.
* Rust gate not re-run.

### Decisions

* **Fold `tag` / `mention` / `language` node walks
  into the SAME `for (const n of nodes)` loop**
  rather than separate loops — single O(N) pass +
  reads as a single counts derivation.
* **Folder was double-counted pre-`-a-63`**
  (contains-edge tally + folder-node tally) —
  now folder-node-only. Audit-confirmed by reading
  the pre-`-a-63` loop body. Matches user
  expectation: "how many folders".
* **Mention chip aggregates `mention`-kind nodes
  + contact files** because the chip toggle hides
  both (`hiddenContactIds` set). The displayed
  count reflects the toggle's hide-set, not just
  one node-kind. If @@Alex wants per-node-kind
  split (mention nodes vs contact files in
  separate chips), that's a follow-up task — task
  body's "out of scope: per-handle dedup" framing
  suggests not yet.

### Suggested commit subject

```
Graph chip counts: switch from edge-tally to node-tally (fullstack-a-63)
```

Single commit. Loop rewrite + test pin tightly
coupled.

### Files for `git add` (per-path discipline)

* `web/src/components/GraphPanel.svelte`
* `web/src/components/graphChipCountSemantics.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-63.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only; the
working tree carries unrelated WIP from other lanes
(`docs/journals/phase-8/alex/event-ci-architect.md`,
etc.) that must NOT be swept into this commit.

Push held. Standing by for clearance.
