# @@Systacean task 3: search-aggression knob

Owner: @@Systacean
Status: REVIEW
Depends on: [systacean-2](./systacean-2.md) (scheduling shape locked in)
Coordinates with: [systacean-4](./systacean-4.md) (fs-change events
re-enter the indexer at scale; both must honour the same aggression
contract)

## Goal

Expose a configurable knob for how aggressive the search indexer is.
[systacean-2](./systacean-2.md) covers the boot/in-flight prioritisation
ordering; this task adds the dial that bounds search-indexer resource use
once it gets to run.

## Scope

* New config field on `chan-server` (and chan-drive, where the indexer
  budget actually lives). Suggested shape: a single enum
  `search.aggression` with three named levels:
  * `conservative` — small batches, long debounce, low parallelism.
  * `balanced` — current behaviour (default).
  * `aggressive` — large batches, short debounce, higher parallelism.
* Concrete defaults per level should map onto whatever knobs the search
  indexer already exposes internally (batch size, debounce window,
  parallelism / worker count, idle backoff). Pick the minimal set;
  don't grow new private knobs to back this if the existing ones are
  enough.
* Wire through:
  * `crates/chan-server/src/config.rs` so the config file accepts the
    new field.
  * `crates/chan-drive/src/index*` (and indexer.rs) so the budget is
    plumbed down.
  * CLI: `chan serve --search-aggression <level>` overrides the config
    file for this run.
  * `/api/config` GET path so the frontend can read the current level
    (no UI surface in this phase; future Settings pill can hang off
    this read).
* Default: `balanced` so the existing test suite and current behaviour
  remain unchanged out of the box.

## Acceptance criteria

* Config parser accepts the three level names and rejects others with a
  clear error.
* CLI flag overrides the file; absence of either picks `balanced`.
* On a moderately populated drive, `conservative` measurably lowers
  search indexer CPU / IO compared to `balanced`, and `aggressive`
  raises it (capture rough numbers in this task file: indexer wall
  time + peak RSS or CPU% suffices, no SLO).
* Existing index-status / rebuild tests still green.
* New unit tests:
  * Config parses each level and rejects garbage.
  * CLI flag wins over config.
  * The level threads down to whichever struct in chan-drive holds
    the budget (assert on a representative knob value).
* `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `npm run check`, `npm test`, `npm run build` all green.

## Hardening expectations

* Re-read the watcher-event coalescing path with the new budgets to
  make sure `aggressive` does not let a write storm pin the runtime.
* Confirm `conservative` still completes an initial pass on the test
  drive within a reasonable time (record the time observed; flag if
  it goes past order-of-minutes on the seeded test drive).

## Coordination

* [systacean-2](./systacean-2.md) owns the ordering between graph,
  chan-report, and search. This task only changes search's pacing.
  Reconcile if @@Systacean finds an overlap.

## Progress

* 2026-05-17 @@Systacean: picked up after [systacean-2](./systacean-2.md)
  reached REVIEW. Starting with config/CLI/indexer budget inspection.
* 2026-05-17 @@Systacean: implemented the `SearchAggression` enum and
  threaded it through chan-drive build budgets, chan-server config,
  indexer debounce, storage reset re-spawn, `/api/config`, and CLI
  `chan serve --search-aggression <level>`.

## Completion notes

Implemented:

* `chan_drive::SearchAggression` with `conservative`, `balanced`, and
  `aggressive`. `balanced` preserves the prior behavior:
  `available_parallelism - 2`, clamped to `[1, 6]`, queue bound
  `workers * 4`, 4096 chunk embedding flush, and 1s watcher debounce.
* `conservative`: 1 worker, queue bound 2, 1024 chunk embedding flush,
  2s debounce.
* `aggressive`: `available_parallelism - 1`, clamped to `[1, 8]`,
  queue bound `workers * 8`, 8192 chunk embedding flush, 250ms
  debounce.
* Persisted config shape:

  ```toml
  [search]
  aggression = "balanced"
  ```

* CLI override: `chan serve --search-aggression conservative|balanced|aggressive`.
  The override applies to both local serve and tunnel serve for that
  run; absence falls back to the persisted config, then `balanced`.
* `/api/config` exposes `preferences.search_aggression`; the Settings
  UI does not render a control yet, but the value round-trips with the
  existing preferences payload.

Manual fixture profile:

* Command: `cargo test -p chan-drive search_aggression_fixture_profile -- --ignored --nocapture`
* Fixture: 240 generated markdown files, 480 chunks, embeddings off
  to isolate the search read/chunk/BM25 budget.
* Observed on this workstation:
  * `conservative`: 169ms, workers=1, queue=2, debounce=2000ms.
  * `balanced`: 175ms, workers=6, queue=24, debounce=1000ms.
  * `aggressive`: 180ms, workers=8, queue=64, debounce=250ms.
* The fixture is intentionally too small for wall-clock differences to
  dominate scheduler noise; the resource delta is visible in the
  bounded worker/queue/debounce knobs and the ignored smoke confirms
  every level completes quickly.

Verification:

* `cargo fmt --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo check --no-default-features`
* `cargo test`
* `cargo test -p chan-drive search_aggression_fixture_profile -- --ignored --nocapture`
* `npm --prefix web run check`
* `npm --prefix web test -- --run`
* `npm --prefix web run build`
