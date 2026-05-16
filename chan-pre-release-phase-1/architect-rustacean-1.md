# architect-rustacean-1

rustacean has marked all assigned tasks plus one adjacent pickup
REVIEW. See `rustacean-1.md`, `rustacean-2.md`, `rustacean-3.md`,
and `chan-core-purge-1.md` for changed paths, verification commands,
and residual risks.

## Summary

- `rustacean-1` REVIEW: pre-v3 contact email backfill purged from
  `chan-server::indexer.rs` and `chan::cmd_status`. Auth comment +
  preferences test name rewritten to a snapshot tone.
  `chan-core-purge-1.md` filed for the producer-side helper in
  `chan-drive::graph.rs`.
- `rustacean-2` REVIEW: new `GET /api/fs-graph` route in
  `crates/chan-server/src/routes/fs_graph.rs`. Wire shape, sample
  payload, node-id and edge-kind rules are frozen in
  `rustacean-2.md` so webdev-1/-2 can start without guessing.
- `rustacean-3` REVIEW: `chan config get|set` for the editor
  namespace, alongside the inherited `chan graph` / `chan status`
  from backend-1. Backend-1's status output had a stale
  `contacts_backfill` field; removed.

## Verification gate run by rustacean

```
cargo fmt --all -- --check                # clean
cargo build -p chan-server -p chan        # ok
cargo clippy --all-targets -- -D warnings # clean
cargo test -p chan-server                 # 78 passed
cargo test -p chan                        # 37 passed
```

Manual `target/debug/chan config {get|set}` probes documented in
`rustacean-3.md` under "Verification".

## What unblocks now

- `webdev-1` and `webdev-2` are unblocked: wire shape for
  `/api/fs-graph` is frozen in `rustacean-2.md`.
- `syseng-1` prep already had the checklist; now that the three
  rustacean tasks are REVIEW, the hardening pass can execute. The
  syseng fixture already covers symlinks, hardlinks, broken links,
  FIFOs, and `.git`/`node_modules` exclusion.

## Architect follow-up status

Superseded by later phase work:

- Commit ordering is captured in `summary.md` and `journal.md`; the sibling
  chan-core cleanup is tracked separately from the main chan phase commits.
- `syseng-1` reached REVIEW after the hardening pass, and the follow-up
  blockers were dispatched and resolved as `architect-syseng-2`,
  `rustacean-4`, `rustacean-5`, and `rustacean-6`.
