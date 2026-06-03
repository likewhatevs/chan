# task-LaneD-LaneA-1: B11 + B10 DONE (Wave-1)

From: @@LaneD  To: @@LaneA  Re: task-LaneA-LaneD-1

Both Wave-1 items own-gate green. Pathspec sha256(my diff) = b29ba5241fd8d224.
Files (9): chan-workspace/src/{fs_ops,workspace}.rs;
chan-server/src/{lib,host,routes/files}.rs; chan/src/main.rs;
web/src/{api/types,state/fileTypes,state/kinds}.ts.

## Own-gate (scoped)
- cargo fmt --check: clean
- cargo clippy -p chan-workspace -p chan-server -p chan --all-targets
  -D warnings: clean
- cargo test -p chan-workspace -p chan-server -p chan: green
  (ws 58 + server 398 + chan 537 + subcrates, 0 failed)
- make web-check (vitest) + svelte-check (0 errors) + npm build: green
- cargo build --no-default-features -p chan -p chan-server -p chan-workspace:
  green

## B11 - editable-text by content
Shipped editable-core: the editor opens + the file browser shows-as-text any
extensionless / odd-suffix UTF-8 file (.zshrc, *.service, Kconfig, ...), and
real binaries stay refused.
- fs_ops::looks_like_text (no NUL + valid UTF-8, tolerates a trailing partial
  multibyte at the sample cut) + TEXT_SNIFF_BYTES=8192. No new dep.
- workspace::sniff_is_text + private editable_text_gate; the 5 read/write gates
  (read_text/_with_stat/_chunked, write_text/_if_unchanged) sniff on the
  Other-class fallback. fs_ops::is_editable_text stays a pure path predicate
  (hot index walks keep it I/O-free).
- files.rs: project_kind Other->"pending"; per-directory listing resolves
  pending->text/binary with a bounded sniff (browser lists one dir at a time,
  so it's a handful of 8KiB reads, synchronous, no async refinement needed -
  the task's "pending refinement push" turned out unnecessary). read route
  falls back to binary only on NotEditableText.
- TS lockstep: "pending" added to the wire kind union (api/types.ts) + FileKind
  + display maps (kinds.ts). classifyPath stays binary-fallback (client can't
  sniff). .md stays the only document/graph node (graph.rs untouched).
- EMPIRICAL (API smoke): .zshrc + app.service -> kind "text" + read returns
  text(200); note.md -> "document"; PNG-bytes-no-ext -> "binary" +
  octet-stream. Real binary still refused, no false positive.

### Two B11 things for you
1. SCOPE still open (followup-LaneD-LaneA-1, non-blocking): "searchable" sniffed
   text = BM25-index it, which means making the PATH-based is_indexable_text
   gate content-aware across ~15 hot-path sites incl the index walk + polluting
   BM25 with .S/Kconfig/config noise. I shipped editable-core only and
   recommend deferring BM25-of-sniffed. If @@Alex wants it searchable too,
   that's an additive follow-up - say the word.
2. BROWSER-VISUAL smoke is the only B11 piece I can't self-verify (editor
   actually renders the .zshrc, tree icon). Backend is API-verified. Tell me
   which client (Chrome vs WKWebView) when you want the visual walk, or fold it
   into a webtest pass.

## B10 - chan serve progress (+ a finding that reframes the bug)
Empirical first: the URL prints at ~0.1s even on a 93k-file tree (open is lazy,
the indexer is a background spawn_blocking). So the silent window is NOT
pre-URL indexing as the task framed it.

TIMING-PROBED ROOT CAUSE: on a content-heavy FIRST serve the ~13s pre-URL stall
is `workspace.watch()` (notify recursive watch), NOT indexing
(num_indexed=35ms, indexer.spawn=5ms; 9000-md vault -> watch()=13.0s). Pure
source trees index sub-second (60k files/2400 md = 0.68s).

Shipped (low-risk, addresses "silent"):
- A cold-gated heads-up printed BEFORE watch() (lands at ~0.06s): "chan:
  preparing this workspace (first run): registering the file watcher + building
  the search index. ... the URL prints below when ready, and indexing continues
  in the background." So the pre-URL stall is no longer silent, whatever its
  cause. Warm restarts (index non-empty) stay quiet.
- A stderr tee of the existing ProgressEvent stream (throttled 750ms,
  self-gated to >800ms elapsed so fast builds print nothing, -v adds the current
  label). A content-heavy build streams "chan: building graph N/9000 (NN%),
  ~Ns left". Verified on a 9000-md vault.
- ServeConfig.verbose plumbed from cli.verbose; tunnel + host config set false.

ESCALATION / your call: eliminating the 13s itself = making watch() setup async
(URL immediate, watcher primes on a background thread). That's chan-workspace
watcher surgery with a small event-loss correctness window during setup
(mitigated by the initial full reindex capturing at-start state). I did NOT do
it under release pressure. Options: (a) ship the heads-up this round + spin the
async-watch fix as a follow-up bug, or (b) you want me to take the async-watch
fix now - I'll scope + flag the risk. Recommend (a).

Note: B11's gate enforcement lives in workspace.rs, which isn't in my explicit
fs_ops/indexer ownership list but is uncontended this round (you already
flagged my fs_ops/workspace.rs WIP as expected). Flagging for transparency.

Holding on B5 (Wave-2, pending your global-vs-codex MCP-env decision to @@Alex)
and D1 (Wave-3, verify-late after B10 + launcher commands land).
