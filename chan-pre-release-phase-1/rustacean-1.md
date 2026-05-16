# rustacean-1: Remove pre-release migration paths and stale schema history

Owner: rustacean. Depends on: nothing. Unblocks: rustacean-3,
syseng-1.

## Goal

Make the codebase behave as Chan's first canonical public version. Remove
development-era migration code and docs that imply users may already have
old Chan production data.

## Background

The request is explicit: schema migrations and other migration code from
internal iterative versions are invalid for this release. Keep fresh
install initialization, current config loading, and defensive corruption
handling. Delete only old-version migration behavior.

## Files to inspect

- `crates/chan-server/src/config.rs`
- `crates/chan-server/src/preferences.rs`
- `crates/chan-server/src/store.rs`
- `crates/chan-server/src/indexer.rs`
- `crates/chan-server/src/routes/storage.rs`
- `crates/chan/src/main.rs`
- sibling chan-core surfaces only for identifying where the old behavior
  lives; do not edit chan-core from this repo task unless Alex/architect
  explicitly splits that work.

## Acceptance criteria

1. No `migrate*`, old `schema_version`, or "legacy Chan version" path
   remains unless it is still required for a current on-disk format.
2. Fresh install, existing current install, index rebuild, and storage
   reset still work.
3. Comments/docs describe the current decision, not the history of how
   development got here.
4. Any required chan-core cleanup is documented back to architect as a
   new task, not silently worked around here.

## Verification

- `rg -n "migrat|schema_version|legacy|old version|compat" crates web`
  with relevant findings reported.
- `cargo test -p chan-server`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`

## Done means

Update this file with changed paths, deleted migration behavior, tests
run, residual risks, and mark `rustacean-1` REVIEW in `journal.md`.

---

## 2026-05-16 Execution

Status: DONE.

### Audit

Ran `rg -n "migrat|schema_version|legacy|old version|compat|backward|
deprecated|v[0-9]+|pre-release|canonical" README.md design.md CLAUDE.md
crates web/src -g '!*.lock'`.

Classification of hits in this repo (chan-core hits filed separately,
see follow-up below):

- `crates/chan-server/src/indexer.rs:136-156` — real one-shot pre-v3
  contact email backfill consumer. REMOVED.
- `crates/chan/src/main.rs` (uncommitted draft of `chan status`) —
  surfaced `contacts_need_email_backfill` in CLI output. REMOVED so the
  CLI doesn't carry a migration flag past this release.
- `crates/chan-server/src/auth.rs:59` — comment framed an existing
  defensive corruption-heal as "pre-release build / manual fiddling".
  Code path is still valid (handles a stray regular file at the tokens
  dir). REWORDED to drop release-history language; kept behavior.
- `crates/chan-server/src/preferences.rs:204` — test name
  `pane_widths_legacy_file_fills_search_default` framed current
  partial-config resilience as legacy migration. RENAMED to
  `pane_widths_partial_fills_missing_defaults` with a snapshot-style
  doc comment. Behavior unchanged.
- `crates/chan-server/src/routes/{files,llm,graph,contacts}.rs` and
  `crates/chan-server/src/lib.rs` "legacy"/"compat"/"forward-compat" —
  describe current public API contracts (e.g. `/api/search` alias,
  basename-stem contact resolver, optional fields). Not pre-release
  migration; left alone per architect-1 audit.
- `crates/chan/src/main.rs` `chan-error-v2` marker, `chan-llm` v2
  comments, `BAAI/bge-small-en-v1.5` model name — external version
  identifiers, not internal migration. Left alone.
- Web `legacy`/`v1`/`v2` hits — editor-rewrite component compatibility
  and scope-naming (per architect-1 audit). Out of scope.

### Behavior removed

In `crates/chan-server/src/indexer.rs::Indexer::spawn`, the
`emails_need_backfill` branch and its companion log line are gone. The
on-boot rebuild trigger is now exactly `stats.indexed_docs == 0 ||
graph_empty`. The CLI side in `crates/chan/src/main.rs::cmd_status`
no longer queries `Drive::contacts_need_email_backfill` and no longer
emits a `contacts_backfill` field in JSON or text output.

Fresh-install behavior is unchanged: cold drives still trigger the
initial rebuild via the BM25-empty / graph-empty path. Existing
installs that already passed the backfill keep working (the on-disk
graph contact rows already carry the emails; we just stop checking).
Existing installs that had NEVER booted the previous binary would
miss the backfill, but this release is the first canonical version,
so by definition no such installs exist in the wild.

### Producer-side cleanup (chan-core)

The producer of the `contacts_need_email_backfill` signal lives in
the sibling `chan-core` workspace at
`chan-drive::graph.rs::contacts_need_email_backfill` (plus tests in
that file around lines 2143-2210, per syseng-1's survey). After this
task lands, that helper has no in-repo callers. Per the rustacean-1
brief, this cleanup is filed as a separate task to architect rather
than crossing the repo boundary in this commit; see
`chan-core-purge-1.md`.

### Files changed

- `crates/chan-server/src/indexer.rs`
- `crates/chan-server/src/auth.rs`
- `crates/chan-server/src/preferences.rs`
- `crates/chan/src/main.rs`

### Verification

```
cargo fmt --all -- --check                # clean
cargo build -p chan-server -p chan        # ok
cargo clippy --all-targets -- -D warnings # clean
cargo test -p chan-server                 # 67 passed
cargo test -p chan                        # 28 passed
```

### Residual risks

- The chan-core `Drive::contacts_need_email_backfill` API stays alive
  until the follow-up lands. The architect should hand that task off
  before declaring Phase 1 sealed; the symbol is dead code in this
  repo as of this commit.
- syseng-1 fixture probe for "fresh install" (empty tempdir + `chan
  serve`) is still gated on syseng-1 itself; no new risk introduced
  here.
