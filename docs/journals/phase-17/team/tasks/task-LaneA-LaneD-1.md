# task-LaneA-LaneD-1: B11 + B10 (Wave 1)

From: @@LaneA  To: @@LaneD  Wave: 1 (isolated, start now)

Two isolated platform items. B5 (MCP env off) is Wave-2 and waits on a scope
decision I am surfacing to @@Alex now (global-off vs codex-only) - do NOT start
B5 yet. D1 docs is Wave-3 (verify-late, after B10 + launcher commands land).

================================================================================
## B11 - editable-text by content, not just extension
================================================================================

### Objective

The editable/binary gate is extension-only, so extensionless or odd text files
(.zshrc, *.service, Dockerfile, Makefile) are refused by the editor + file
browser. Add a content sniff so real text opens. @@Alex listed this; it is the
phase-15-deferred "content magic detection".

### Pre-decided approach (do NOT deviate)

Hand-rolled sniff: read first N bytes; if valid UTF-8 (allow a trailing partial
multibyte sequence at the N-byte cut) AND no NUL byte -> Text. NO new
dependency (single-binary line: do NOT add libmagic / infer / tree_magic).
See docs/journals/phase-15/round-4-wave-4.md "Round-5 (deferred)".

### Touch points (re-verify against HEAD)

- crates/chan-workspace/src/fs_ops.rs ~313-394: the editable/binary gate
  (classify ~342). Add the content sniff here.
- crates/chan-workspace/src/indexer.rs: the async hook that classifies files
  for the index.
- Wire kind surface: crates/chan-server/src/routes/{files.rs,graph.rs}.
- SPA mirror: web/src/state/{kinds.ts,fileTypes.ts} - add a "pending" kind
  (content not yet sniffed) per bootstrap.

### .md-vs-text rule (honor the phase-15 architect call)

- .md = document (participates in graph as a document node).
- Other sniffed text = editable + searchable, NOT a document node.
- Keep the Rust FileClass enum and the TS mirror (kinds.ts/fileTypes.ts) in
  LOCKSTEP - any new variant lands in both.

### Gate

- cargo fmt --check + cargo clippy -p chan-workspace -p chan-server
  --all-targets -D warnings + cargo test -p chan-workspace -p chan-server.
- Re-check --no-default-features build if you touch any feature gate.
- make web-check + svelte-check + npm run build for the kinds.ts/fileTypes.ts
  mirror.
- Empirical: open a .zshrc and a *.service file in the editor + see them in the
  file browser (test server; coordinate the server with @@LaneA - I'll tell you
  which client). Confirm a real binary (e.g. a PNG) is STILL refused (no false
  positive from the sniff).

================================================================================
## B10 - chan serve progress on a large tree
================================================================================

### Objective

`chan serve .` on a huge tree (shallow linux-kernel clone) runs silent for a
long time even with --verbose. Print concise progress (indexing phase + counts)
to stderr BEFORE the ready URL. @@Alex: "not too excessive ... about what chan
is doing there, before it spits out the url".

### Touch points (re-verify against HEAD)

- crates/chan/src/main.rs cmd_serve ~1093-1242 (the silent window).
- crates/chan-server/src/lib.rs build_app ~302-537 (indexer spawn ~367, "chan
  is ready" ~537) - the indexing happens here.
- chan-workspace boot/indexer + the existing progress.rs event stream is the
  progress source - reuse it, do not invent a parallel counter.

### Requirements

- Concise: indexing phase label + counts (files seen / indexed), not a
  per-file spew. Goes to stderr (the ready URL is also stderr).
- Honor --verbose: more detail with --verbose, a minimal one-liner without.
- Test on a real large tree: git clone --depth=1
  https://github.com/torvalds/linux /tmp/linux-shallow (per @@Alex), then
  chan serve /tmp/linux-shallow and confirm the user sees progress before the
  URL. Tear the throwaway down after.

### Gate

- cargo fmt --check + cargo clippy -p chan -p chan-server --all-targets
  -D warnings + cargo test -p chan -p chan-server.
- Empirical: the linux-shallow run above shows progress before the URL.

================================================================================
## Report
================================================================================

When BOTH B11 and B10 own-gate green, cut tasks/task-LaneD-LaneA-1.md (summary
per item + own-gate-green + pathspec shas) and poke @@LaneA. If one finishes
well before the other you may report them separately (task-...-1, -2) - your
call; keep me posted via the journal either way.

B5 (Wave-2) + D1 (Wave-3) come as separate tasks. Hold on both.
