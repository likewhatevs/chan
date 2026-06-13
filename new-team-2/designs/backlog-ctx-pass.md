# Backlog B1 — chan-server threaded-state ctx-pass refactor

Lane: @@CtxPass. This was DEFERRED from round 1 explicitly "for a
designed ctx pass" — the design doc comes first, then execution in
waves. Do not start editing before @@Conductor signs off on the
design.

## Inventory (from round 1)

Full inventory: `new-team-1/tasks/task-Chan-Lead-1.md` (the round-1
recon by @@Chan; param counts in parentheses):

- chan-server `graph.rs` merge_* family (11/9/9/8 params)
- chan-server `control_socket.rs` handle_team (11)
- indexer spawn family
- `terminal_sessions::restart` (8)
- fs_graph / survey / drafts / contacts entries

Round-1 context worth reading: @@Chan's param-struct refactor
01d0cba6 (ServeArgs/ControlSocketCtx) — accepted with zero defects
and reviewed field-by-field; THIS pass continues that work for the
clusters that were deferred because they thread mutable state through
call chains (a config-struct can't just absorb them — they need a
designed context type per family).

## Design doc requirements

For each family: the proposed ctx struct (name, fields, ownership/
borrow shape), which params stay loose (genuinely per-call data),
the call-site count (use QUALIFIED greps — round-1 lesson: a
different private `handle_request` inflated a count to 13; verify
names against the actual module), and the wave it lands in.
Behavior preservation is the bar: no logic changes ride along.

## Wave ordering (overlap-driven; binding)

1. `graph.rs` merge_* family — no contention, start immediately.
2. Indexer spawn family.
3. fs_graph / survey / drafts / contacts.
4. **LAST, gated:** `terminal_sessions::restart` (after @@PromptQueue's
   item-2 server half lands in terminal_sessions.rs) and
   `control_socket.rs` handle_team (after @@TeamFlow's item-5
   template change lands — different file but same crate; and
   handle_team is adjacent to the team plumbing they verify against).

Each wave: signature + ALL call sites in one burst,
`cargo check -p chan-server` green before pausing (three lanes share
this crate), scoped own-gate with RUSTFLAGS="-D warnings" on clippy
AND test, pathspec-atomic commit per family. Cross-review by
@@PromptQueue per wave (field-by-field call-site mapping, the round-1
standard).

## Out of scope

No behavior changes, no error-shape changes, no renames beyond the
new ctx types. If a family turns out to want a real redesign (not
just parameter grouping), write it up for @@Conductor and move on to
the next wave.
