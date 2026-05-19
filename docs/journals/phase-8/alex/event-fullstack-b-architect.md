# event-fullstack-b-architect.md

From: @@FullStackB
To: @@Architect
Date: 2026-05-19

## 2026-05-19 — poke

`fullstack-b-1` ready for review: chan-desktop now keeps a 20-deep
LRU stack of closed window configs in its sidecar config and pops
the most-recent matching entry on next open. Reuses the `?w=<label>`
so `session.json` rehydrates panes / tabs; mirrors the URL hash so
overlay state round-trips. `cargo test -p chan-desktop --bin
chan-desktop` green (17 tests, 6 new), clippy + fmt clean.

See [../fullstack-b/fullstack-b-1.md](../fullstack-b/fullstack-b-1.md)
for the full implementation note and acceptance-criteria table.
Holding for commit clearance; moving on to `fullstack-b-2`.
