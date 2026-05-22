# systacean-35 — chan-server /api/mentions endpoint (mention-corpus prefix query; unblocks -a-70)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Add `GET /api/mentions?q=<prefix>&limit=<int>` endpoint
returning prefix-matched mention handles from the
chan-server's mention corpus. Unblocks @@FullStackA's
`-a-70` (editor mention completion gap).

## Reference

@@FullStackA's scope-poke at the tail of
[`../fullstack-a/fullstack-a-70.md`](../fullstack-a/fullstack-a-70.md):

Editor's mention completion today queries only the
contact list (frontmatter-marked files). Per
`systacean-22`'s contact-filter work, the broader
mention corpus (all `@@<Name>` references across
markdown) IS in the graph but not exposed via API.

## Scope

### Route

`GET /api/mentions?q=<prefix>&limit=<int>`:

* `q`: case-insensitive prefix to match against
  mention labels.
* `limit`: cap on returned entries; default 10
  (mirror `/api/contacts` shape).
* Returns: `Array<{label: string}>` JSON.

### Implementation

* Build the same `mention_set` aggregation the
  graph route uses (per `routes/graph.rs::merge_*`
  shape).
* Filter by case-insensitive prefix.
* Sort label-asc.
* Cap at `limit`.

### Performance note

Per @@FullStackA's audit: if per-call walk is too
slow on a large drive (chan-source seed has ~1973
files), lift mention-extraction into the indexer
boot pass + cache on the graph handle. Implementer's
call after first impl; profile if needed.

### Wiring

* New `crates/chan-server/src/routes/mentions.rs`
  (or extend existing route module).
* Route registration in `lib.rs::router()`.
* Re-export from `routes/mod.rs`.

## Acceptance

1. `/api/mentions?q=Arc&limit=10` returns
   `[{label: "@@Architect"}, ...]` for the chan
   repo seed (or whatever prefix matches).
2. `/api/mentions?q=Z&limit=10` returns `[]`
   when no match.
3. `/api/mentions?limit=5` returns top 5 sorted.
4. `limit` defaults to 10 when not provided.

### Tests

* Fixture insert with known mention tokens; assert
  prefix query returns them.
* Empty-result case.
* Limit cap behaviour.

### Gate

`cargo fmt / clippy / test`; smoke green.

## Coordination

* @@Systacean lane.
* @@FullStackA wires the SPA side
  (`api.mentions(q, limit)` client method +
  editor completion query merge) after this lands.

## Authorization

Yes for `crates/chan-server/src/routes/mentions.rs`
(new or extension) + lib.rs + routes/mod.rs + tests
+ task tail + outbound.

## Numbering

This is `-35`.

## Out of scope

* SPA-side completion query merge (`-a-70` lane).
* Mention-extraction performance optimization
  unless first-impl benchmark warrants.
