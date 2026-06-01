# Phase-15 round-4 backlog (carryover from round-3)

Architect-collected. Items not finished to a tested state in round-3 land
here, per the bootstrap round-close rule. @@Architect consolidates the full
list at round-3 close; this file is seeded mid-round as deferrals are made.

## Theme-6: complete phase-8 docs cleanup (deferred from round-3, @@LaneB)

@@LaneB finished Theme-6 for phases 1-7, 9-14 (raw dropped, essence READMEs
with #hashtags, a930a96f). phase-8 was DEFERRED, not skipped, because it is
the one phase still cited from `docs/agents/` (out of B's lane) and it needs
nuanced handling rather than a mechanical repoint:

- `docs/agents/desktect.md` has 3 real content links into phase-8
  (`phase-9-desktop-native-vision.md`, `event-architect-desktect.md`,
  `process.md`). They ALREADY point at the pre-`raw/` layout, so they are
  broken today; deleting phase-8 raw would remove the content they aim at.
- `docs/agents/bootstrap.md` uses `phase-8` as a TEMPLATE EXAMPLE path
  throughout (it even says "phase-8 for now; update as we roll forward").
  Those are illustrative path patterns in a living process template, NOT
  live citations to preserve; a blanket repoint would be wrong.

Round-4 task: (1) synthesize the phase-8 essence README in the phases-1-13
shape; (2) repoint desktect.md's 3 links to that README (or a git-history
note); (3) decide bootstrap.md's template-phase handling (leave as an
example vs bump the example phase); (4) THEN delete phase-8 `raw/`. This is
destructive + touches coordination docs, so it is an @@Host-sequenced item
(cf. the "destructive cleanups coordinate with docs" rule), not a mechanical
cleanup. Not release-blocking: the citations are already broken, so leaving
phase-8 raw is no regression.

## Desktop: real AppImage `cs` re-exec verify (deferred from round-3, @@LaneD)

The `cs_install` unit tests pass on HEAD, but the Linux AppImage argv[0]
re-exec path could not be exercised in this environment (no AppImage build).
Verify on a built AppImage. The macOS desktop-as-`cs` path IS verified
(round-3, against the real desktop control socket).

## IDX: investigate the Metal hang + re-enable GPU as default (Theme-5, @@Host)

@@Host: "we had previously commented out the use of Metal on macOS because it
was hanging the indexing. Let's create a follow up item to investigate the
hang and re-enable." Status after round-3:

- The opt-in infra is DONE and works today: embeddings default to CPU, and
  `CHAN_ENABLE_GPU=1` selects Metal (macOS) / CUDA (Linux/Win) at runtime.
  Device selection: `crates/chan-workspace/src/index/embeddings.rs` ~365-417
  (`enable_gpu = CHAN_ENABLE_GPU set` -> `Device::new_metal(0)`). The `metal`
  feature is target-gated on the macOS chan/chan-server build (chan/Cargo.toml
  85-86), so the opt-in is live, not stubbed.
- The hang: candle's Metal backend blocks indefinitely in
  `[_MTLCommandBuffer waitUntilCompleted]` on at least one Apple machine
  during a large embed pass (embeddings.rs ~17-20, 351-358). CPU is the
  default until that is fixed.

Round-4 task: reproduce under `CHAN_ENABLE_GPU=1` on a large reindex, root-
cause the command-buffer submit/await (candle Metal internals; check whether
a candle version bump or a submit/commit/await-ordering correction resolves
it), and once it runs clean, flip the default back to GPU on macOS (drop the
opt-in gate, or invert it to `CHAN_DISABLE_GPU`). Needs a Mac with Metal +
a workspace large enough to trigger the multi-batch embed pass. The Theme-5
in-flush freeze fix (smaller batch, shipped round-3) helps CPU UX meanwhile.

## Product/scope question for @@Host (surfaced by the round-3 search PROBE)

Semantic (BGE) search is BUILT and stored on every reindex but NEVER queried:
every search path (HTTP route + `chan search` CLI) is BM25-only. We pay embed
compute for retrieval nothing reads. Decide a direction: flip hybrid on by
default, gate it behind the existing `semantic_enabled` opt-in and wire it
into the route, or drop the dense-vector build. Out of round-3's Theme-4
(mentions/paths) scope; raised for a deliberate call.
