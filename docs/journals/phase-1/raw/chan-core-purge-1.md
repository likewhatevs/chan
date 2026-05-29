# chan-core-purge-1: drop pre-v3 contacts email backfill from chan-drive

Owner: architect to dispatch (sibling repo `chan-writer/chan-core`).
Surfaced by: rustacean-1.

## What

`rustacean-1` removed the consumer side of the pre-v3 contact email
backfill from this repo:

- `crates/chan-server/src/indexer.rs::Indexer::spawn` no longer queues
  a one-shot rebuild when contact rows lack an `emails` column.
- `crates/chan/src/main.rs::cmd_status` no longer reads or prints
  `contacts_need_email_backfill`.

As of those commits, the producer side in chan-core has no in-repo
callers and is dead code with respect to chan's public release.

## Where in chan-core

- `chan-drive::graph.rs::contacts_need_email_backfill` — the bool
  helper that drove the consumer.
- Tests around lines ~2143-2210 of the same file exercise the
  backfill detection.

Coordinates per syseng-1's pre-phase survey
(`phase-1/syseng-1.md`, item 7) and the architect's
advisory (`architect-syseng-1.md`).

## Ask

Remove the helper, the graph-column / migration code that produces
the missing-emails state, and the associated tests. Confirm that
fresh-install behavior of `Drive::reindex` still populates contact
rows with their email vectors via the current parse path. No
schema-version bump or migration shim — this is the first canonical
release.

## Coordination

- `rustacean-1` is REVIEW in this repo. Its commit lands with the
  helper alive but unused.
- chan-server pulls chan-core as a path dep today; once the symbol
  is gone, the chan-server build in this repo will still compile
  unchanged. No revdep churn.
- syseng-1 is gated on rustacean-1/-2/-3; the chan-core purge can
  land in parallel with syseng's hardening pass.

## Done means

The helper and its tests are gone from chan-core, the version of
chan-core that chan-server picks up no longer exposes the symbol,
and `cargo build -p chan-server` in this repo still passes.

---

## 2026-05-16 Execution

Status: DONE. Picked up by rustacean since this is Rust work and
the context from rustacean-1 is hot. Architect can still review
before sealing Phase 1.

### Files changed (chan-core repo)

- `crates/chan-drive/src/drive.rs`
  - Removed the `Drive::contacts_need_email_backfill` wrapper.
- `crates/chan-drive/src/graph.rs`
  - Removed `GraphView::contacts_need_email_backfill`.
  - Removed the `migration_v3_adds_emails_column_and_marks_existing_
    contacts_for_backfill` test.
  - Trimmed the schema-header comment that referenced the
    "indexer triggers a full rebuild" path; the `emails` column is
    now documented as a forward-going invariant filled by the
    indexer on every walk.
  - Rewrote the v3 migration comment: drops the "chan-server
    indexer's initial-build trigger" wording, keeps the actual
    ALTER + version-bump (the column still has to exist on every
    DB).
- `crates/chan-drive/design.md`
  - Replaced the paragraph describing the backfill flag with one
    describing how `emails` is populated by the indexer on the
    drive's first walk. No more "operator intervention" language.

### What stayed

The v3 migration itself stays. It's idempotent and runs on every
fresh DB open (v0 → v6 in sequence); folding all migrations into a
single CREATE-from-scratch is a bigger refactor that needs an
architect/Alex decision. Filed as a follow-up risk below rather
than executing here.

### Verification

In `chan-core/`:

```
cargo fmt --all -- --check                # clean
cargo build -p chan-drive                 # ok
cargo clippy --all-targets -- -D warnings # clean
cargo test -p chan-drive                  # 401 + 8 + 1 + 2 + 8 + 3
                                          # + 4 + 1 = 428 passed
```

In `chan/`:

```
cargo build -p chan-server -p chan        # ok
cargo test -p chan-server                 # 78 passed
cargo test -p chan                        # 39 passed
```

### Residual risks / follow-ups

- The migration chain v0..v6 is still present. For a true
  first-canonical-version cleanup the architect could collapse it
  into a single CREATE TABLE block that emits the v6 final shape.
  Out of scope here; needs Alex / architect to weigh the readability
  win against the loss of recoverability for any in-flight dev DBs.
- chan-report and chan-tunnel-* crates were not audited for similar
  pre-release migration code in this pass. None showed up in the
  rustacean-1 grep, but a follow-up sweep before commit is cheap.

### Branch

The chan-core changes are uncommitted on `main`. rustacean did not
push or create a commit; per project process the architect
distributes commit responsibility.
