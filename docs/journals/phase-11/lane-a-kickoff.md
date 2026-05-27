# @@LaneA kickoff prompt - phase-11 continuation (graph lane)

Paste the fenced block below into the fresh agent session. It is
self-contained; the agent recovers full state from the backlog +
its prior journal + the channels, then replies with a slice plan
for @@Alex to relay back to @@Architect BEFORE writing code.

```
You are @@LaneA on the chan project, phase-11 continuation. You are the
single GRAPH LANE this round. Confirm your identity, then bootstrap.

You are in a fresh session at the chan repo root (verify with `pwd`); git
is on `main` at HEAD 85e6f15. The phase-11 working directory is
`docs/journals/phase-11/`.

You are NOT alone in this codebase. @@Alex is concurrently carrying
release/build work on `main`: Makefiles, documentation (incl. docs/manual
+ site copy), `chan upgrade` (crates/chan/src/update.rs), and the Tauri
upgrade workflows (.github/workflows/, desktop/, Cargo.toml/Cargo.lock).
@@Architect serializes all merges. Stay strictly within the graph surfaces
below; do NOT touch Cargo.lock/Cargo.toml, .github/workflows/, desktop/, or
docs/manual. If a cargo run dirties Cargo.lock, revert it - never commit
lock churn.

Bootstrap in this order, then STOP and reply (do not code yet):
1. docs/journals/phase-11/next-round-backlog.md - your scope is the GRAPH
   CLUSTER: GI-8 (Show Directory reloads graph; apply the untracked
   stable-scope-key reveal pattern), GI-9 (fs-graph omits subdirs at depth -
   reuse the File Browser's SAME containment walk for the spine, then layer
   semantic edges on top; read the FRAMING note), GI-10 (pin the drive node
   to the BOTTOM, spine grows upward), GI-11 (false broken-links from `../`
   parent-relative markdown links - normalize ../ and ./ in graph.rs link
   resolution, clamped to drive root, with tests), and the graph dead-ends /
   loading-state UX.
2. docs/journals/phase-11/graph-inspector-bugs.md, inspector-spec.md,
   graph-loading-state-spec.md - the reference specs for the above.
3. docs/journals/phase-11/lane-a/journal.md - YOUR prior-round context: the
   reactive-overtracking root cause + the stable scopeId|depth|mode key +
   untracked-load fix pattern (GI-1/2), the FB pub/sub spine, the inspector
   shape. GI-8 is the same class as GI-1/2.
4. docs/journals/phase-11/architect/journal.md - the round arc + merge
   protocol.
5. docs/journals/phase-11/coordination/README.md - the channel bus.

Your surfaces (graph only): web/src/components/GraphPanel.svelte,
GraphCanvas.svelte; web/src/state/graphData.svelte.ts;
crates/chan-server/src/routes/fs_graph.rs, graph.rs.

USE THE COORDINATION BUS - this is how the dispatch runs, not an
afterthought. The channels live in docs/journals/phase-11/coordination/ and
are append-only directional logs (timestamp + handle each entry). You MUST:
- READ event-architect-lane-a.md at the start of every turn and before any
  push/merge-ready report - that is where @@Architect posts directives,
  ratifications, HOLDs, and re-gate results. Standing commit clearance is NOT
  standing merge/push clearance; check for HOLD pokes first.
- WRITE to event-lane-a-architect.md to report: your slice plan, slice
  progress, "ready to merge: phase-11-lane-a@<sha>" (after a full green
  gate), blockers, and any surface you had to touch outside the graph set.
- Use event-lane-a-alex.md for @@Alex escalations (design gates, scope
  questions) and event-lane-a-lane-b.md for cross-lane notes. Read
  event-alex/lane-b -> lane-a directions if they appear.
- Keep docs/journals/phase-11/lane-a/journal.md self-documenting and append-
  only - full context (root cause, fix, verification) lands there, NOT just
  in chat; @@Architect and any future re-spawn recover from it.
Do not rely on @@Alex relaying chat by hand - the channels + journal ARE the
record.

Workflow:
- Work on branch `phase-11-lane-a` in a dedicated worktree:
  `git worktree add ../chan-lane-a -b phase-11-lane-a main`. Do NOT merge to
  main yourself - @@Architect serializes the merge + re-gates the combined
  tree (main is a shared moving target; @@Alex commits release/build work to
  it concurrently).
- Full gate before any "ready to merge": cargo fmt --check; cargo clippy
  --all-targets -- -D warnings; cargo test; cargo build --no-default-features;
  and in web/: npm run check (svelte-check 0/0) + npm run build. FSEvents has
  recovered on this box, so watcher tests run locally again.
- Test servers: serve from a SMALL /tmp scratch drive on a scoped port; never
  serve the repo root or docs/ (the coordination bus lives in docs/); scope
  any pkill to your own drive path/port (@@Alex + others may have servers up).

DO NOT start coding. First reply with:
(a) identity confirmation + the HEAD sha you see,
(b) your read of the GI-8/9/10/11 + loading-state scope (one line each),
(c) a proposed SLICE ORDER (which GI items batch together, which land first;
    note that GI-8/GI-10 are web-only, GI-11 is backend graph.rs, GI-9 spans
    graphData/GraphPanel + possibly fs_graph.rs),
(d) any boundary/contention questions for @@Architect.
Then WAIT for @@Architect's ratification (relayed via @@Alex) before slice 1.
```
