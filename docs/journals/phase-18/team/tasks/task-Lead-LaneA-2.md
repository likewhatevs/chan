# task Lead -> LaneA (2): `[[` decision + cross-lane authorization

@@Alex answered the survey: **"Paths + existing link-targets (both)"** - ADDITIVE.
So `[[` keeps the existing filename/heading/block candidates AND gains
workspace-path candidates. This unblocks item 4. Authorization + boundaries:

## Where the backend change goes (recon already done by @@Lead)
- chan-workspace `link_targets` lives at `crates/chan-workspace/src/graph.rs:1308`
  (wrapped by workspace.rs:2796). The `LinkTargetKind` enum (File/Heading) is at
  graph.rs:1429. Extend `link_targets` to ALSO yield workspace-path candidates
  (add a `Path`/`Directory` LinkTargetKind variant if you need to distinguish
  them), keeping existing File/Heading rows. recent_files is at ~1462.
- The chan-server handler `api_link_targets` (crates/chan-server/src/routes/
  graph.rs:127) is a PURE PASSTHROUGH (`workspace.link_targets(q, limit)` ->
  `Json`). You should NOT need to touch routes/graph.rs at all. That file is
  @@LaneB's (graph wire-kinds). If you believe you must touch it, STOP and route
  through me - do not edit it.
- Editor client: bubbles/wiki.ts + the shared TS `LinkTarget` type +
  web/src/api/client.ts (linkTargets). If you add a LinkTargetKind variant/field,
  grep ALL literals (BOTH casings) + svelte-check + make web-check (the
  required-field rule: vitest strips types so a scoped vitest passes with stale
  fixtures). Tests for link_targets live at graph.rs ~2034-2160; extend + keep
  green.

## CONTENTION - chan-workspace/src/graph.rs is SHARED with @@LaneB (SEQUENCED)
That file is ALSO @@LaneB's (NodeKind / GraphNode / contact-stamp, ~80-210).
A `.rs` file is NOT interleave-safe (a mid-edit leaves a non-compiling window
that breaks the peer's `cargo check -p chan-workspace`). So:
- Do everything else NOW: items 1-3, the editor-CLIENT side of item 4
  (wiki.ts rendering + client type), and DESIGN your link_targets change +
  write the test - but HOLD your WRITE to crates/chan-workspace/src/graph.rs.
- @@LaneB's contact-stamp (a bug, already in flight) lands its chan-workspace
  graph.rs portion FIRST. I will poke you "chan-workspace graph.rs stable" once
  B's portion is green. THEN land your link_targets edit, re-`cargo check -p
  chan-workspace` + `cargo test -p chan-workspace` green.
- If B's stamp fix turns out NOT to touch graph.rs (lands in a different file),
  I'll release you immediately to edit graph.rs in parallel.

## Gate (unchanged)
Rust: cargo fmt --check + clippy --all-targets -D warnings + cargo test
(-p chan-workspace). Frontend: make web-check + svelte-check + npm run build,
browser-smoke the `[[` bubble showing path candidates.

## On completion: fold item 4 status into your task-LaneA-Lead-1.md (or a -2 if 1
already shipped). Poke me. Flag any other shared-file touch BEFORE landing.
