# task-Conductor-CtxPass-5 — B1: chan-server ctx-pass refactor (design first)

From: @@Conductor. To: @@CtxPass. Cut: 2026-06-12.

## Scope

B1 per new-team-2/designs/backlog-ctx-pass.md (read fully). B1 was
deferred from round 1 explicitly "for a designed ctx pass": the
design doc comes FIRST, and you do NOT edit code before my sign-off.

## Step 1 — design doc (the only work authorized right now)

Write new-team-2/designs/b1-ctx-pass-design.md covering, per family
(graph.rs merge_*, indexer spawn, fs_graph/survey/drafts/contacts,
terminal_sessions::restart, control_socket.rs handle_team):

- proposed ctx struct: name, fields, ownership/borrow shape;
- which params stay loose (genuinely per-call data);
- call-site counts from QUALIFIED greps (`rg --text --no-ignore`;
  verify names against the actual module — round 1's unqualified
  `handle_request` grep inflated a count to 13);
- wave assignment (ordering below is binding).

Behavior preservation is the bar: no logic changes, no error-shape
changes, no renames beyond the new ctx types.

Context to read first:
- new-team-1/tasks/task-Chan-Lead-1.md (round-1 inventory, param
  counts);
- commit 01d0cba6 (@@Chan's ServeArgs/ControlSocketCtx param-struct
  refactor — the accepted precedent; this pass continues it for the
  clusters that thread mutable state).

Then cut new-team-2/tasks/task-CtxPass-Conductor-<n>.md + poke me.
I review and sign off (or amend) before wave 1 starts.

## Wave ordering (binding)

1. graph.rs merge_* family — no contention, starts on sign-off.
2. Indexer spawn family.
3. fs_graph / survey / drafts / contacts.
4. LAST and gated — wait for my explicit poke on each half:
   a. terminal_sessions::restart — after @@PromptQueue's item-2
      server half lands in terminal_sessions.rs;
   b. control_socket.rs handle_team — after @@TeamFlow's item-5
      template change lands.

## Per-wave discipline

- Signature + ALL call sites in one burst; `cargo check -p
  chan-server` green before pausing (three-lane-hot crate); announce
  bursts in your journal.
- Scoped own-gate: RUSTFLAGS="-D warnings" on clippy AND test.
  Re-run after the final edit of each wave.
- Pathspec-atomic commit per family: `git commit -F <msg-file> --
  <paths>`; staged-stat before, show-stat after.
- @@PromptQueue cross-reviews each wave field-by-field (I route on
  your per-wave poke).
- If a family turns out to want a real redesign (not parameter
  grouping), write it up for me and move on to the next wave.

## Review pairing (both directions)

You also review @@PromptQueue's item-2 chan-server half when I route
it to you.

## Completion

Per-wave 1-line poke with the sha after each wave commit (deliberate:
keeps review routing off your critical path). Final completion task
after wave 4. Journal: journals/journal-CtxPass.md, append-only.
