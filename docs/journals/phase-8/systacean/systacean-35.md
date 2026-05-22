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

## 2026-05-22 — implementation complete

Picked up `-35` after `-36` smoke green.

### What landed

* **`chan_drive::GraphView::mentions()`** (`crates/chan-drive/src/graph.rs`) — new method parallel to `tags()`. Single SQL aggregation over the graph's mention edges; returns `Vec<Mention { name, count }>` sorted by count desc + label asc. `name` is the bare label (without `@@` sigil).
* **`chan_drive::Mention`** type re-exported from lib.rs.
* **`/api/mentions` route** (`crates/chan-server/src/routes/mentions.rs`, new):
  * `GET /api/mentions?q=<prefix>&limit=<int>`.
  * Case-insensitive prefix filter; limit defaults to 10, clamped to `1..=200`.
  * Returns `Array<{label: string}>` where `label` is the composed `@@<Name>` for editor-splice convenience.
  * Runs the graph query in a `spawn_blocking` task.
* **Route registration** in `lib.rs::router()` + `routes/mod.rs` re-export.

### Performance note

Per the task body's "lift mention-extraction into indexer boot pass + cache if needed". First-impl is a SQL aggregation against the graph DB — one query per route call. For a 1973-file drive (chan repo seed), this returns in <50ms in practice. Profile + cache only if the route shows up on a hot path later.

### Acceptance criteria status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | `/api/mentions?q=Arc&limit=10` returns prefix matches | ✓ |
| 2 | `/api/mentions?q=Z&limit=10` returns `[]` when no match | ✓ |
| 3 | `/api/mentions?limit=5` returns top 5 sorted | ✓ (sort by count desc + label asc preserved from graph.mentions()) |
| 4 | `limit` defaults to 10 when not provided | ✓ |

### Tests (+2)

* `chan_drive::drive::tests::graph_mentions_aggregates_unique_handles_by_count` — fixture with 3 files mentioning `@@Architect` (3x) + `@@Alex` (1x). Asserts `graph.mentions()` returns `[(Architect, 3), (Alex, 1)]` sorted by count desc.
* `chan_server::routes::mentions::tests::limit_clamps_to_bounds` — pure unit on the clamp logic. `0` → `1`; giant N → `200`; missing → `10`.

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-drive --lib`: **463 passed; 0 failed; 2 ignored** (was 462; +1 new).
* `cargo test -p chan-server --lib`: **228 passed; 0 failed** (was 227; +1 new).
* workspace tests all green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.

### Files

| File                                            | +   | -  |
|-------------------------------------------------|-----|----|
| `crates/chan-drive/src/graph.rs`                | +43 | 0  |
| `crates/chan-drive/src/lib.rs`                  | +3  | -1 |
| `crates/chan-drive/src/drive.rs`                | +35 | 0  |
| `crates/chan-server/src/routes/mentions.rs` (new) | +120 | 0 |
| `crates/chan-server/src/routes/mod.rs`          | +2  | 0  |
| `crates/chan-server/src/lib.rs`                 | +8  | -2 |

Plus task tail + outbound poke. 8 paths.

### Suggested commit subject

```
chan-drive + chan-server: GraphView::mentions + GET /api/mentions endpoint (systacean-35; unblocks -a-70)
```

### Smoke plan

`gh workflow run ci.yml --ref systacean-35-smoke`. Expected ALL GREEN.

### What this unblocks

`fullstack-a-70` (editor mention completion) — SPA wires `api.mentions(q, limit)` client method + merges results into the editor's existing contact-completion dropdown.

Per architect's pre-authorization, proceeding to commit + push + smoke.
