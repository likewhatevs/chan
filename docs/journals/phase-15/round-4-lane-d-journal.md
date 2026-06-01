# Phase-15 round-4 - @@LaneD journal

Lane: @@LaneD (semantic-search wiring + phase-8 docs cleanup).
Two disjoint workstreams. Wave 1 complete; Wave 2 (delete phase-8 raw/)
pending the refresh.

## Wave 1 - status: DONE, merged locally, gated-green

Commits (local, on main; not pushed - push is @@Host-only):

- `0a180ffd` feat(search): request hybrid when semantic_enabled + model
  present  (crates/chan-server/src/routes/search.rs, crates/chan/src/main.rs)
- `f8c8edec` docs(phase-8): clean essence README + repoint agent
  citations  (docs/journals/phase-8/README.md, docs/agents/desktect.md,
  docs/agents/bootstrap.md)

Both are pathspec commits (`git commit -F msg -- <paths>`) with a
pre-commit diff --stat audit and a post-commit show --stat audit. The
shared worktree had @@LaneC's WIP (chan-shell, control_socket,
team_config, web/ wikilink) and @@LaneB's scripts/dev/ present
throughout; neither commit swept any of it.

### Workstream 1 - semantic search behind `semantic_enabled`

The probe finding held: dense vectors are built + stored every reindex
(`indexed_vectors` was 2 on the live probe) but were NEVER queried - the
route + CLI hardcoded `SearchOpts { ..Default::default() }` = Mode::Bm25
and never read the per-workspace opt-in. The empty-query response also
hardcoded mode:"hybrid" (stale).

Change (decision point only - no facade/config/indexer touch, the infra
was already correct):

- New `resolve_search_mode(workspace)` in both the route and cmd_search.
  It mirrors `routes/index::build_state`: request Mode::Hybrid only when
  `semantic_enabled()` is on AND `resolve_model(semantic_model())` finds
  the model on disk, else Mode::Bm25. A flipped-on flag with no model
  still serves Bm25 (defensive; `enable` refuses that shape, but a model
  removed out from under us would otherwise mis-route).
- The route also factored out a pure `select_search_mode(enabled,
  present)` so the truth table is unit-testable without a model on disk.
- The model probe is behind `#[cfg(feature = "embeddings")]`; the
  no-default-features build compiles it out and always returns Bm25 (the
  facade collapses Hybrid to Bm25 there regardless). Verified with
  `cargo build --no-default-features`.
- Empty-query mode is now `mode.label()` (tracks the flag), not the
  stale hardcoded "hybrid".
- Updated the stale doc comments (module doc, ContentSearchResponse.mode
  doc, the "defaults to Hybrid via SearchOpts::default" comment).

Tests added (crates/chan-server/src/routes/search.rs):
- `select_search_mode_requires_flag_and_model` - Hybrid only for
  (true,true); Bm25 for the other three combinations.
- `content_search_reports_bm25_when_semantic_disabled` - real query on a
  default (semantic-off) workspace reports bm25.
- `content_search_empty_query_reports_flag_mode` - empty query reports
  bm25 + zero hits (catches the old hardcoded "hybrid").
All deterministic regardless of whether a model is cached on the host
(they exercise the semantic-OFF path, where the flag gates the result).

Live probe (EMPIRICALLY VERIFIED - the model BAAI/bge-small-en-v1.5 is
cached on this machine, so the hybrid path was exercised end to end):
- Served a throwaway drive (/tmp, renamed binary copy, scoped pkill).
- Route, semantic OFF (default): q=brew -> mode=bm25, empty q -> bm25.
- POST /api/index/semantic/enable -> /semantic/state flips to hybrid.
- Route, semantic ON (model present): q=brew -> mode=hybrid, empty q ->
  hybrid.
- POST /api/index/semantic/disable -> back to bm25.
- CLI parity: `chan search` prints BM25 scores (2.000/1.000) with
  semantic OFF and RRF-fused scores (0.033/0.032) with semantic ON. The
  CLI does not print the mode label, but the score shape proves which
  retrieval path ran.
- Test server + drive + registry entry + binary copy torn down.

### Workstream 2 - phase-8 docs cleanup (non-destructive, Wave 1 part)

phase-8 already had a README.md, but in the OLD shape (markdown links
into raw/, no Tags line, a bulleted Raw-material index). The
phases-1-7,9-14 cleanup (a930a96f) skipped it. Brought it into the
cleaned shape:

- phase-8/README.md: added `Tags: #bugfixes #signing #release
  #opensource #ci #desktop #docs`; de-linked all 11 `[..](raw/..)`
  markdown links to plain-text backtick references; rewrote Raw material
  to the canonical "preserved in git history ... removed in the phase-15
  docs cleanup" note. Only surviving link is `../../agents/README.md`
  (target persists). Widest line 74 cols.
- desktect.md: repointed its 3 broken phase-8 citations (the pre-raw
  vision doc, the inbound event channel, the process spec) to the
  phase-8 essence README + a git-history backtick pointer to the
  original raw file. All links in the card now resolve to live targets
  (except a pre-existing issue, below).
- bootstrap.md: DECISION - left the template placeholder paths as
  illustrative examples (phase-8 is the example phase; "update as we
  roll forward" is the documented maintenance). Did NOT blanket-repoint.
  BUT the lane doc's "NOT live cites" framing missed 2 concrete markdown
  links in the "Phase-8 standing permissions" table
  (event-fullstack-b-alex.md, event-webtest-b-alex.md, pre-raw paths).
  Those are real dead links that Wave-2 raw deletion makes permanent, so
  I de-linked just those 2 to git-history references (same corrective
  treatment as the README/desktect.md; not a blanket repoint). Flagging
  it here since it diverges slightly from the "leave entirely" reading.

## For @@Architect

- Wave-2 carryover (mine): delete phase-8 `raw/`. The citation repoint
  is now merged (f8c8edec), so the destructive step is unblocked. After
  deletion, verify the chan-source graph shows no phase-8 ghost nodes
  and desktect.md's links resolve.
- PRE-EXISTING, OUT-OF-SCOPE observation (not fixed): `skills/architect.md`
  is a broken link in BOTH desktect.md (line 27) and architect.md (line
  15); docs/agents/skills/ is empty. Not a phase-8 citation, so I left
  it. Worth a separate cleanup task if the skills dir is meant to exist.
- Gate: Rust side green on the current tree (fmt - my files clean, the
  fmt failures are @@LaneC's in-flight control_socket/team_config/cli/
  wire; clippy default + no-default; cargo test -p chan-server -p chan =
  358 pass incl. my 3 new; build --no-default-features). I touched zero
  web files, so svelte-check + npm build are unaffected (not run here;
  the full pre-push gate runs them on @@Host's push).

## Wave 2 - status: DONE, merged locally, verified

Commit (local, on main; push is @@Host-only):

- `e747f1d2` docs(phase-8): drop raw/ (essence README + git history
  retained)  (283 files, 138036 deletions)

Pathspec commit (`git commit -F msg -- docs/journals/phase-8/raw/`) on a
pre-clean index (nothing else staged). The shared worktree's other-lane
state (round-4 coordination docs, .claude/, pub-site-release/,
new-team-1/) was present but untouched; the post-commit `git show --stat`
showed exactly the 283 raw/ deletions.

### What was deleted

All of `docs/journals/phase-8/raw/` (283 tracked files): the per-agent
journals + task files (alex, architect, ci, desktacean, desktect,
desktest, fullstack-a/b, rich-prompt, systacean, webtest-a/b), the
alex/ event channels, and the top-level phase-8-bugs.md / process.md /
request.md. The phase-8 essence `README.md` (Wave-1, f8c8edec) persists
as the summary; the raw material stays recoverable in git history.

### Verification (no ghost graph nodes + links resolve)

- raw/ gone from disk + 0 tracked paths remain under it
  (`git ls-files docs/journals/phase-8/raw/` = 0); README.md persists.
- ZERO clickable references into the deleted tree, both link forms:
  - markdown `](...phase-8/raw...)` -> none anywhere in the tree.
  - wikilink `[[...phase-8/raw...]]` -> none anywhere in the tree.
  This is the substantive proof of "no ghost nodes": chan's graph only
  manufactures a placeholder node from a DANGLING link target; with zero
  links into raw/, deleting the real files just drops them from the
  file-node set - nothing can resolve to a phantom phase-8/raw node. (A
  live graph serve would only re-confirm this static fact; skipped to
  avoid contending the shared test-server cores mid-Wave-2.)
- desktect.md's clickable markdown links all resolve: `../journals/
  phase-8/README.md`, `architect.md`, `bootstrap.md` all OK. The ONLY
  broken link is `skills/architect.md` - the PRE-EXISTING, out-of-scope
  item (empty docs/agents/skills/) already flagged in Wave-1 + the status
  doc as a round-5/phase-16 backlog item; NOT a phase-8 citation.
- The surviving `phase-8/raw/<path>` mentions in desktect.md (lines 23,
  49, 54) + bootstrap.md (lines 417, 418) are intentional backtick
  "preserved in git history at ..." annotations, NOT links - they stay
  accurate after deletion since the content remains in git history.

### Gate

Docs-only deletion: no Rust/web compile surface touched (rust-embed
bundles web/dist, not docs/), so fmt/clippy/test/build + svelte-check/
npm build are all unaffected. No gate run needed for this commit.

WAVE-2 COMPLETE for @@LaneD. No round-5 carryover from my lanes except
the pre-existing skills/architect.md broken link (NOT mine; flagged for
a separate cleanup task).

## Wave 4 - indexing/graph cluster (3 fixes) - status: DONE, merged, gated-green

Recycled into Wave 4 (round-4-wave-4.md): 3 small indexing/graph fixes for
v0.23.0. Each gated (fmt + clippy --all-targets -D warnings + cargo test +
build --no-default-features; web svelte-check + vitest + npm build for the
web-touching fix) and committed via race-proof pathspec.

Commits (local, on main; push is @@Host-only):

- `ce9c286e` fix(server): pulse the indexing spine through the background
  embed sweep  (crates/chan-server/src/routes/search.rs)
- `fcf06679` fix(cli): silence tokei "Unknown extension" log spam
  (crates/chan/src/main.rs)
- `a5c95545` fix(graph): only Markdown is a graph document; .txt stays
  searchable text  (workspace.rs, files.rs, inspector.rs, 2 integration
  tests, fileTypes.ts + new fileTypes.test.ts)

### Fix 1 - spine pulse (search.rs), browser-smoked

Root cause confirmed as triaged: the frontend + ancestor-spine propagation
were already correct. The bug was the SIGNAL. Once the first EmbedBatch
fires, the indexer commits BM25 and flips to `Idle { embedding: Some(..) }`
with NO per-file label, so `current_index_file()` is `None` for the whole
(minutes-long) embed window -> `build_indexing_state` marked nothing. The
old `broad_sweep` rescue keyed off an `"embedding"` `Building.file` sentinel
that PRODUCTION NEVER EMITS (dead code; only a synthetic test passed it).

Change (Option A, the smallest): added `is_embedding_sweep(&IndexStatus)`
(true for `Idle { embedding: Some(..) }`), computed it alongside
`current_file` in `api_indexing_state`, threaded it into
`build_indexing_state` as an explicit `embedding_sweep` flag, and OR'd it
into `broad_sweep`. So every dir with indexable content pulses Indexing for
the duration of the embed pass. Did NOT reintroduce a sentinel.

Tests: re-pointed the orphaned sentinel test at the real `Idle.embedding`
signal (`None, true` instead of `Some("embedding")`); added a unit test
pinning the `is_embedding_sweep` variant mapping; the other 3 callers pass
`false`.

LIVE BROWSER SMOKE (EMPIRICALLY VERIFIED, navigate re-allowed): fetched the
BGE model (`make models` -> resources/models.tar.zst, gitignored), built
`chan --features embed-model`, served a throwaway 360-file .md drive
(/tmp/chan-laned-spine, renamed binary copy /tmp/chan-laned-smoke, scoped
port 8831, pkill scoped to the drive path). Enabled semantic -> the embed
sweep ran (`/api/index/status` = `Idle { embedding: { done: 359, total:
360 } }`, `indexed_vectors: 0`). DURING that real window:
- `/api/indexing/state` returned ALL 4 dirs (root, docs, journal, notes)
  as `state: "indexing"` (the backend signal - the exact fix).
- The Dashboard indexing slide rendered all 3 dir nodes in ORANGE
  (theme.doc) connected to the central workspace node, legend "indexed /
  indexing", embed chip "embedding 359/360" active. This is the runtime-
  reactive proof the static gates miss. (FILE KINDS on the workspace slide
  also read "document 360" - the all-.md corpus, confirming Fix 3's
  .md->document mapping live.)
- Server + tab + drive + registry entry + binary copy all torn down; no
  other lane's `chan serve` touched.

### Fix 2 - tokei log spam (main.rs)

tokei (transitive via chan-report's language-count lens) logs `Unknown
extension: <ext>` at WARN via its own `LanguageType::from_path` for any
file it can't classify. chan-report is default-off, so on a source tree
with reports enabled it was pure console noise (no downstream effect; the
graph language lens already degrades when a bucket is absent).

Change: extracted `fallback_filter(level)` from `init_tracing`, adding a
`tokei=error` directive on the FALLBACK EnvFilter only (RUST_LOG parses
first via `try_from_default_env`, so RUST_LOG users keep full control).
Classification unchanged. Chose `tokei=error` over the module-specific
`tokei::language::language_type=off` for robustness (tokei's noisy file is
generated `.tera.rs`; the module path could shift, a crate-level cap can't).
Test: `fallback_filter_caps_tokei_for_every_level` pins that the static
directive parses for every verbosity level and survives in the filter
(a malformed directive would panic the binary at launch).

### Fix 3 - .txt is not a graph document (md/txt split)

@@Host wants only `.md` as graphed + wikilinked documents. `.txt` was
`FileClass::EditableText`, so it became a graph document node AND the
`"document"` wire kind AND was wikilink/heading/token-parsed. Architect
call (relayed in the bootstrap, @@Host-confirmed): keep `.txt` EDITABLE +
BM25-SEARCHABLE but NOT a graph document and NOT the `"document"` kind.

The codebase already had the right primitive: `fs_ops::is_markdown_file`
(.md only, already used for tag/mention token stripping + inspector kind).
I narrowed the GRAPH ingest to it while leaving the SEARCH index on the
wider `is_indexable_text`:
- `rebuild_graph`: the 3 walk gates (total count, resume live-map, skip)
  -> `is_markdown_file`. Stages only Markdown nodes.
- `index_file_inner` / `index_draft_file`: create a graph node only for
  `.md`; for a non-Markdown indexable file (`.txt`) skip the parse + node
  upsert and `graph.forget_file(rel)` to evict any stale node (e.g. a `.md`
  renamed to `.txt`). The BM25 `index_one` still runs for both, so `.txt`
  stays searchable.
- `facade::list_indexable` (the BM25 search pass) UNCHANGED on
  `is_indexable_text` -> `.txt` stays in full-text search.
- Wire kind: `routes::files::project_kind` AND
  `routes::inspector::file_kind_label` now map `.md`->`document`,
  `.txt`->`text` (both via `is_markdown_file`, kept in lockstep with a
  cross-reference comment).
- TS: `fileTypes::classifyPath` mirrors it (`.md`->`document`,
  `.txt`->`text`). `kinds.ts` already had both wire kinds. `isMarkdown`
  intentionally LEFT covering `.md`+`.txt`: the editor still renders `.txt`
  as markdown + offers Export-to-PDF - an editor-only concern the architect
  call did not touch.

Tests: workspace `reindex_txt_searchable_but_not_a_graph_document` +
`index_file_txt_skips_graph_node` (graph has the .md node, never the .txt;
both BM25-searchable); `project_kind` + `file_kind_label` unit tables;
`classifyPath` vitest. The 2 integration tests that pinned the OLD policy
(file_types.rs, remove_cleanup.rs) were updated: `.txt` is no longer a
graph node, and its removal/restore is now verified via BM25 search instead
of graph-node presence.

## For @@Architect (Wave 4)

- ALL 3 FIXES GATED-GREEN on the merged tree: `cargo fmt --check` =0,
  `clippy --all-targets -D warnings` =0 (incl. chan-desktop), `cargo test
  --workspace --exclude chan-desktop` =0 (32 ok-blocks, 0 fail), `build
  --no-default-features` =0; web `npm run check` 0 errors, `vitest` 1622
  pass, `npm run build` OK. chan-desktop excluded from the TEST run only
  (no test changes from me, needs Tauri toolchain); clippy DID check it.
- Fix 1 is EMPIRICALLY VERIFIED end-to-end in the browser (orange spine
  during a real embed sweep), not just unit-tested. Fixes 2 + 3 are
  unit/integration-tested; Fix 3's .md->document is also visible in the
  live Dashboard ("document 360").
- SCOPE NOTE (Fix 3, flagged for your review): I did NOT narrow the
  rename-link-rewrite gate (`workspace.rs` ~1939, `is_indexable_text`) to
  markdown. A `.txt` is still EditableText + the editor still renders it as
  markdown (isMarkdown), so keeping its links maintained on a rename is
  consistent with ".txt stays editable". Narrowing it was tempting for
  symmetry but would silently break a `.txt`'s relative links on an
  unrelated rename - out of the 3 enumerated items. Left as-is.
- MINOR DUPLICATION (not refactored): `project_kind` (files.rs) and
  `file_kind_label` (inspector.rs) are two copies of the same classify ->
  wire-kind mapping on different surfaces; I updated both identically with
  a lockstep cross-reference comment. A future consolidation into a shared
  `chan_workspace` helper would remove the drift risk; not done this round
  to keep the fix small.
- Model artifact: `make models` left `crates/chan-server/resources/
  models.tar.zst` (63 MB, gitignored, verified via git check-ignore) for
  the embed-model smoke build. Not committed.

WAVE-4 COMPLETE for @@LaneD. No round-5 carryover from these 3 fixes. The
deferred item (content "magic" detection + a "pending" state) stays
round-5 per round-4-wave-4.md.
