# Phase-15 round-4 Wave-4 - indexing/graph cluster (3 small fixes)

@@Host-approved (after Wave-3 code-complete): 3 small, low-risk indexing/graph
fixes ship in v0.23.0; the 4th item (content "magic" detection + a "pending"
state) is ROUND-5 (M-L feature touching the editable-text correctness gate +
a cross-crate/TS wire change). Proposed owner: @@LaneD (indexing/semantic
domain; knows search.rs from Wave-1). A coordinates + gates + runs the cut.

All three are grounded by the round-4 triage subagents (file:line below).
Browser-smoke the spine pulse (runtime-reactive; static gates miss it).
Commit each via race-proof pathspec. Gate: fmt + clippy -D warnings + test +
build --no-default-features; web svelte-check + npm build.

## Fix 1 - the indexing spine never pulses orange (backend-only, ~10-20 lines)

Root cause (NOT a missing feature): the frontend + ancestor-spine propagation
are fully built + unit-tested. `GraphCanvas.svelte:1049-1055` maps
`indexState:"indexing"` -> orange `theme.doc` (#ff8a3d) + an alpha pulse
(1081-1089). `build_indexing_state` (chan-server/src/routes/search.rs:390) +
`ancestor_dirs_for_file` (search.rs:477) correctly mark the whole ancestor
chain `indexing` when `current_index_file` matches. THE BUG: the indexer drops
the `Building{file}` status the instant the embed sweep starts - in
`StatusUpdater::on_progress` (chan-server/src/indexer.rs:892-926) the first
`EmbedBatch` latches `started` and flips status to `Idle{embedding:Some(..)}`
(indexer.rs:952-974), so for the whole (minutes-long, visible) embed sweep
`current_index_file()` is `None` -> nothing pulses. The `broad_sweep` rescue
(search.rs:451) keys off a `Some("embedding")` sentinel that PRODUCTION NEVER
EMITS (dead code; only the synthetic unit test passes it).
FIX (Option A, smallest): in `api_indexing_state` / `build_indexing_state`
(search.rs), treat `Idle{embedding:Some(..)}` as a broad sweep so every dir
with indexable text pulses `indexing` during the embed phase. Re-point the
orphaned `"embedding"`-sentinel unit test at the real `Idle.embedding` signal.
The 3s poll (EmptyPaneCarousel.svelte:432) is fine - the embed sweep is a long
stable window. SMOKE: serve a workspace big enough to trigger a real embed
sweep; watch ancestor dirs pulse orange on the Dashboard indexing slide.

## Fix 2 - tokei "Unknown extension" log spam (trivial, ~5 lines)

The warn is tokei's OWN logging: `tokei .../language_type.tera.rs:310`
`warn!("Unknown extension: {}")`, reached via chan-report
(crates/chan-report/src/count.rs:63 `LanguageType::from_path`). It surfaces
through the tracing-log bridge at the default `warn` level
(crates/chan/src/main.rs:~791-800). chan-report is DEFAULT-OFF
(IndexConfig::reports_enabled=false), so it is pure console noise for users who
enabled reports on a source tree - no downstream breakage (the graph language
lens already degrades when a bucket is absent).
FIX: in `init_tracing` (chan/src/main.rs:~796), add a fallback EnvFilter
directive silencing the noisy module, e.g.
`EnvFilter::new(level).add_directive("tokei::language::language_type=off")`
(or `tokei=error`). Add only to the FALLBACK (try_from_default_env first) so
`RUST_LOG` users keep control. Does NOT change classification. +1 test.

## Fix 3 - .txt treated as a graph "document" (small)

`classify()` (crates/chan-workspace/src/fs_ops.rs:342) maps
`"md" | "txt" => FileClass::EditableText`; `project_kind`
(chan-server/src/routes/files.rs:73) maps `EditableText => "document"`; index +
graph ingest is gated on `EditableText` (fs_ops.rs:73,90). So .txt becomes a
graphed, wikilinked "document" node. @@Host wants ONLY .md as documents
(graphed + linked).
ARCHITECT CALL (provisional, flag for @@Host override): keep .txt EDITABLE and
SEARCHABLE, but NOT a graph document and NOT the "document" wire kind. I.e.
decouple "is a markdown document" (.md only, via the existing `is_markdown`
helper at fs_ops.rs:83) from "is editable/indexable text" (md + txt). Concretely:
the GRAPH document-node + wikilink parse + `project_kind`'s "document" mapping
key off MARKDOWN (.md), while .txt stays EditableText for the editor + search
and surfaces as the "text" wire kind. This avoids silently dropping .txt from
search (the simple "move txt -> FileClass::Text" alternative would also remove
.txt from the search index - a more surprising regression). If @@Host instead
wants .txt fully non-indexed (editable-only), the simpler txt->Text move does
that. RESOLVE THIS with @@Host before implementing. Keep the Rust `FileClass`
and the mirrored TS classifier (web/src/state/fileTypes.ts:229, kinds.ts:22) in
lockstep; pre-release = no back-compat. SMOKE: a .txt file no longer appears as
a floating document node in the graph; .md still graphs + links.

## Round-5 (deferred): content "magic" detection + "pending" state

The (b)/(c)/(d) of @@Host's file-type report: async content sniffing for files
the extension can't type, with a transient "pending indexing" state. Investigation
recommends a hand-rolled "first N bytes valid UTF-8 + no NUL -> Text" sniff (no
new dep) over `infer`, since the real pain is text-ish files with odd extensions
shown as binary. Touches fs_ops.rs:313-394 (the editable/binary correctness
gate), the indexer async hook (indexer.rs:252/361/459), the wire kind
(files.rs/graph.rs), and the SPA (a new "pending" kind in kinds.ts/fileTypes.ts
+ a refinement push). The "phase 1.5 content sniffing" already foreshadowed in
fs_ops.rs:100. Real feature; not v0.23.0.
