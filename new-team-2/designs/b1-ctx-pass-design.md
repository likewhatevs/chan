# B1 design — chan-server ctx-pass refactor

Author: @@CtxPass. Date: 2026-06-12. Status: awaiting @@Conductor
sign-off (no code edited).

Bar (binding, from task-Conductor-CtxPass-5): behavior preservation —
no logic changes, no error-shape changes, no renames beyond the new
ctx types. Wire formats untouched (ControlRequest serde shape is
frozen; see wave 4b).

## Verified counts vs the round-1 inventory

Every count below is from qualified `rg --text --no-ignore` sweeps
(binaries/target excluded) against HEAD e0ec0d3c, cross-checked by
reading each definition. The round-1 inventory numbers do NOT
reproduce — every family is 1–4 params smaller than recorded
(e.g. merge_* "11/9/9/8" vs actual max 7; restart "8" vs actual
6 + self). Same class of error as round 1's `handle_request`
overcount; the plan below uses only the verified numbers.

Two name collisions found and excluded the same way:

- `GraphView::replace_file` (graph.rs, in scope) vs
  `VectorStore::replace_file` (index/vectors.rs + facade.rs callers,
  4 params, NOT in scope).
- chan-server `routes/graph.rs` merge_* (in scope) vs nothing else —
  but `merge_filesystem_layer` exists only `#[cfg(test)]`.

| fn | params (now) | call sites (prod + test) |
|---|---|---|
| merge_directory_node | 7 | 2 + 0 (all graph.rs) |
| merge_tree_entry | 6 | 1 + 0 |
| ensure_directory_path | 5 | 5 + 0 (1 recursive) |
| merge_unified_tree_layer | 5 | 1 + 0 |
| merge_filesystem_layer_with_buckets | 5 | 2 + 0 |
| merge_tree_file_node | 4 | 1 + 0 |
| push_contains_edge | 4 | 2 + 0 |
| merge_language_layer | 4 | 1 + 2 |
| merge_filesystem_layer (cfg(test)) | 4 | 0 + 6 |
| spawn_coordinator | 8 (allow) | 1 + 0 |
| spawn_watcher_loop | 7 | 1 + 0 |
| Indexer::spawn | 5 | 1 + 9 |
| set_idle | 4 | 4 + 2 |
| reconcile_idle | 4 | 2 + 2 |
| build_fs_graph_paged | 6 | 1 + 9 (all fs_graph.rs) |
| create_followup_file | 6 | 1 + 6 (all survey.rs) |
| GraphView::replace_file | 9 + self | 1 + 18 |
| drafts::scan_entries | 8 (allow) | 2 + 0 (1 recursive) |
| drafts::promote | 5 | 1 + 7 |
| contacts slug_for | 5 | 2 + 17 |
| contacts/import run | 5 | 1 + 0 |
| TerminalRegistry::restart | 6 + self | 2 + 0 |
| handle_team | 9 (allow) | 1 + 8 |

## Wave 1 — graph.rs merge_* family

The real pattern: `build_graph_view` owns the accumulators
(`nodes: BTreeMap<String, GraphNodeView>`, `edges: Vec<GraphEdgeView>`)
and the tree layer threads them + a dedup `edge_set` + two read-only
refs (`workspace`, `report_buckets`) through 4 helpers.

Ctx struct (private to routes/graph.rs):

```rust
/// Accumulators + read-only inputs threaded through the unified
/// tree layer. `edge_set` is OWNED: it is built from the edges
/// accumulated so far at construction (exactly where today's
/// merge_unified_tree_layer builds it) and dies with the ctx, so
/// the contains-edge dedup keeps seeing the fs-layer edges pushed
/// before the tree pass.
struct TreeMergeCtx<'a> {
    workspace: &'a chan_workspace::Workspace,
    report_buckets: &'a HashMap<String, ReportFileBucket>,
    nodes: &'a mut BTreeMap<String, GraphNodeView>,
    edges: &'a mut Vec<GraphEdgeView>,
    edge_set: BTreeSet<(String, String, &'static str)>,
}
```

Helpers become `impl TreeMergeCtx` methods (names kept):
`merge_tree_entry(&mut self, entry)`, `ensure_directory_path(&mut
self, path)` (recursion → method recursion), `merge_tree_file_node
(&mut self, path)`, `push_contains_edge(&mut self, source, target)`.

Stays a free fn: `merge_directory_node(nodes, id, label, path,
path_class, files, code)` — its second caller
(merge_filesystem_layer_with_buckets:1102) runs BEFORE any edge_set
exists, so it cannot be a ctx method without moving edge_set
construction earlier, which would un-dedup the fs-layer edges
(behavior change). Its 6 non-accumulator params are the directory
node's field payload — genuinely per-call data; loose.

Unchanged signatures: merge_unified_tree_layer (constructs the ctx),
merge_filesystem_layer_with_buckets, merge_language_layer,
merge_filesystem_layer (test-only). All ≤5 params, each a distinct
layer entry point consuming `&GraphParams` (params already struct-
ified). Zero test-call-site edits in this wave; the 6+2 test callers
hit the unchanged outer fns.

Borrow shape: methods take `&mut self`; intra-method field accesses
are disjoint (`self.workspace` shared + `self.nodes` mut). The loop
in merge_unified_tree_layer can keep calling
`path_class_for_graph(workspace, ...)` alongside the ctx because the
ctx holds a shared reborrow of workspace.

## Wave 2 — indexer spawn family

`IndexerShared { status, telemetry, bg_embed }` already exists
(indexer.rs:117, Clone) and is already what spawn_watcher_loop takes
— the design extends it to the whole family instead of inventing a
second ctx. Add the two remaining values shared by BOTH spawned
tasks:

```rust
#[derive(Clone)]
struct IndexerShared {
    status: Arc<Mutex<IndexStatus>>,
    telemetry: Arc<Mutex<IndexerTelemetry>>,
    bg_embed: BgEmbed,
    cancel: Arc<AtomicBool>,
    search_aggression: SearchAggression,
}
```

New signatures (all private, all in indexer.rs):

- `spawn_coordinator(workspace: Weak<Workspace>, shared:
  IndexerShared, rebuild_rx, progress_sink)` — 4 params; drops the
  `#[allow(clippy::too_many_arguments)]` AND its justifying comment
  ("bundling ... would churn the call sites for no clarity win",
  lines 304–307) — that comment is the recorded counter-position;
  this designed pass supersedes it, flagging per the task's
  redesign-vs-grouping rule.
- `spawn_watcher_loop(workspace: Weak<Workspace>, shared:
  IndexerShared, watch_events, rebuild_tx, watch_context)` — 5.
- `set_idle(workspace: &Workspace, shared: &IndexerShared)` — 2.
- `reconcile_idle(workspace: &Weak<Workspace>, shared:
  &IndexerShared)` — 2.

Loose params stay loose: the channel halves (`rebuild_rx`,
`watch_events`, `rebuild_tx`) are per-task endpoints, not shared
state; `progress_sink` is coordinator-only; `watch_context` is
watcher-only. `Indexer::spawn` (public, 5 params, 1 prod + 9 test
call sites) is genuinely per-call config — untouched, so all 10 of
its call sites stay untouched. The `Indexer` struct's own fields are
also untouched (snapshot/health_snapshot/cancel keep reading
`self.status` etc.).

Test edits: 4 call sites (set_idle ×2, reconcile_idle ×2) construct
the trio today; they'll construct IndexerShared with a fresh cancel
flag + default aggression. All edits confined to indexer.rs.

## Wave 3 — fs_graph / survey / drafts / contacts (+ workspace graph)

Mixed bag; three real ctx/record structs, one accumulator, one
allocator, two leave-loose calls.

### 3a. GraphView::replace_file — record struct (the big win)

9 params + self, 1 prod (workspace.rs:2573) + 18 test call sites
whose positional `None, None, Some(1), ...` runs are unreadable.
All per-file row data → a named record, pub in chan-workspace
graph.rs next to Edge/NodeKind:

```rust
pub struct FileRecord<'a> {
    pub rel: &'a str,
    pub title: Option<&'a str>,
    pub mtime: Option<i64>,
    pub size: Option<i64>,
    pub node_kind: NodeKind,
    pub outgoing: &'a [Edge],
    pub headings: &'a [markdown::Heading],
    pub emails: Option<&'a str>,
    pub aliases: Option<&'a str>,
}
```

`replace_file(&self, record: FileRecord<'_>)`. 19 call-site edits,
18 mechanical test rewrites to named fields. NOT touched:
`VectorStore::replace_file` (vectors.rs; facade.rs callers are the
vector one). Doc sync rider: chan-workspace/design.md:1041 lists a
stale 5-param signature — update in the same commit.

### 3b. drafts::scan_entries — accumulator struct

5 `&mut` accumulators (entries/file_count/dir_count/total_size/
has_draft_md) mirror DraftInspection's fields; carries an
`#[allow(too_many_arguments)]` today.

```rust
#[derive(Default)]
struct DraftScanAccum {
    entries: Vec<DraftEntry>,
    file_count: usize,
    dir_count: usize,
    total_size: u64,
    has_draft_md: bool,
}
```

`scan_entries(name, root, rel_dir, acc: &mut DraftScanAccum)` — 4
params, allow dropped. 2 call sites (scan_draft + recursion), zero
test edits (tests enter via promote/public API).

### 3c. contacts slug_for — allocator struct (threaded-state poster child)

`taken: &mut HashSet<String>` + `unnamed_counter: &mut usize` are
mutable state threaded through a loop at both prod call sites
(contacts/import.rs:63, chan/src/main.rs:2667), each with a fixed
`dir` + `on_disk` closure:

```rust
pub struct SlugAllocator<'a> {
    dir: &'a str,
    on_disk: &'a dyn Fn(&str) -> bool,
    taken: HashSet<String>,
    unnamed: usize,
}
impl<'a> SlugAllocator<'a> {
    pub fn new(dir: &'a str, on_disk: &'a dyn Fn(&str) -> bool) -> Self;
    pub fn slug_for(&mut self, c: &Contact) -> String;  // name kept
}
```

Both prod sites start with empty `taken` / zero counter (verified;
the import.rs pre-seed comment moves to the constructor). 2 prod +
17 test call-site edits (slug.rs tests, mechanical). Doc sync rider:
chan-workspace/design.md:1187 documents the old signature.

### 3d. build_fs_graph_paged — params struct, mirrors GraphParams

All 5 non-workspace params are per-request query data; the sibling
graph route already models exactly this as `GraphParams`. For idiom
symmetry:

```rust
pub struct FsGraphParams<'a> {
    pub scope: FsGraphScope,
    pub path: &'a str,
    pub depth: usize,
    pub cursor: Option<&'a str>,
    pub limit: Option<usize>,
}
```

`build_fs_graph_paged(workspace, p: FsGraphParams<'_>)`. The
non-paged `build_fs_graph` wrapper (4 params, used by graph.rs:1094)
keeps its loose signature and forwards with `cursor: None, limit:
None`. 1 prod + 9 test edits, all inside fs_graph.rs.

> [CORRECTION, post-review — @@Conductor, 2026-06-13] The "forwards
> with cursor: None, limit: None" sentence above was never true:
> `build_fs_graph` is an independent whole-scope walk and calls no
> paged builder (caught by @@PromptQueue's wave-3 review,
> task-PromptQueue-Conductor-28; the 8f070e36 commit message has it
> right). As landed, `build_fs_graph` keeps its loose signature and
> is untouched by wave 3d. See also the ratified amendment note: the
> struct is the module's pre-existing query type, not a new <'a>
> struct.

### 3e. survey create_followup_file — request struct

`(dir, from, to, title, body)` is the followup message payload; the
adjacent positional `("team", "@@A", "@@B", ...)` test calls make
from/to swaps invisible:

```rust
pub struct FollowupSpec<'a> {
    pub dir: &'a str,
    pub from: &'a str,
    pub to: &'a str,
    pub title: Option<&'a str>,
    pub body: &'a str,
}
```

`create_followup_file(workspace, spec: FollowupSpec<'_>)`. 1 prod +
6 test edits, all inside survey.rs.

### 3f. Leave loose (recommendation, with rationale)

- `drafts::promote(drafts_dir, workspace_root, workspace_root_canon,
  name, target_rel)` — 5 params, zero threaded mutable state, single
  prod caller (Workspace::promote_draft) that has the three paths as
  locals. A DraftPaths struct would be a struct for one fn.
- `contacts/import::run(workspace, dir, contacts, opts, progress)` —
  5 params, opts already IS the grouped struct (ImportOpts); the
  rest are distinct per-call inputs. Single prod caller.

If @@Conductor wants these grouped anyway, both are <30-minute
mechanical additions to wave 3.

## Wave 4a (gated: @@PromptQueue item-2 server half) — restart

`TerminalRegistry::restart(&self, id, tab_name, tab_group, window_id,
command, env)` — 5 of 6 params are optional overrides applied onto
`old.restart_options()`:

```rust
#[derive(Default)]
pub struct RestartOverrides {
    pub tab_name: Option<String>,
    /// Outer None keeps the existing group; Some(None) sets the
    /// default group; Some(Some(g)) sets group g.  [doc moves here]
    pub tab_group: Option<Option<String>>,
    pub window_id: Option<String>,
    pub command: Option<String>,
    pub env: Option<BTreeMap<String, String>>,
}
```

`restart(&self, id: &str, overrides: RestartOverrides)`. 2 call
sites: routes/terminal.rs:359 builds the literal;
restart_matching:734 passes `RestartOverrides::default()` (replacing
five Nones). Field list re-verified against terminal_sessions.rs
AFTER item-2 lands — if item-2 adds restart inputs they become
fields, not extra params.

## Wave 4b (gated: @@TeamFlow item-5) — handle_team

9 params, allow(too_many_arguments) + a comment arguing against
bundling (control_socket.rs:530–533; superseded by this design, same
flag as wave 2). The split is clean: 4 shared connection handles —
all already fields of `ControlSocketCtx` (the 01d0cba6 precedent
type) — plus 5 request fields that are exactly the
`ControlRequest::TerminalTeam` variant's fields.

```rust
struct TeamRequest {
    dir: String,
    op: TeamOp,
    config_toml: Option<String>,
    script: bool,
    /// [window_id doc comment moves here]
    window_id: Option<String>,
}
```

`handle_team(req: TeamRequest, ctx: &ControlSocketCtx)` — mirroring
`handle_request(req, &ctx)`. The dispatch arm constructs TeamRequest
from the destructured variant; the **ControlRequest enum itself is
untouched** (serde wire shape frozen — no nested-struct or flatten
games on a socket format). handle_team resolves the registry
internally via `ctx.terminal_registry.get()` — still per-request
against the same set-once cell, observably identical to today's
caller-side resolve. tenant copy (`*ctx.tenant`-style) matches
handle_request's existing destructure.

8 test call sites: rewritten onto the existing `test_ctx(...)`
helper (already used by handle_request tests) + TeamRequest
literals. Field list re-verified after item-5 lands.

## Wave plan recap + discipline

| wave | scope | gate | files touched |
|---|---|---|---|
| 1 | TreeMergeCtx | none — starts on sign-off | routes/graph.rs |
| 2 | IndexerShared widen | after wave 1 | indexer.rs |
| 3 | FileRecord, DraftScanAccum, SlugAllocator, FsGraphParams, FollowupSpec (+2 leave-loose) | after wave 2 | graph.rs+design.md+workspace.rs, drafts.rs, contacts/slug.rs+import.rs+chan/main.rs, routes/fs_graph.rs, routes/survey.rs |
| 4a | RestartOverrides | @@Conductor poke after item-2 server half | terminal_sessions.rs, routes/terminal.rs |
| 4b | TeamRequest | @@Conductor poke after item-5 | control_socket.rs |

Wave 3 lands as per-family pathspec-atomic commits (replace_file /
drafts / contacts / fs_graph / survey are independent), each:
signature + ALL call sites in one burst, `cargo check -p
chan-server` (and `-p chan-workspace` / `-p chan` where touched)
green before pausing, scoped own-gate `RUSTFLAGS="-D warnings"`
clippy + test re-run AFTER the final edit. Wave-3c touches
crates/chan/src/main.rs — single-burst with the chan-workspace
signature change since chan depends on it (shared-tree compile
window). Per-wave 1-line poke with sha to @@Conductor for
@@PromptQueue's field-by-field cross-review.

Three `#[allow(clippy::too_many_arguments)]` markers retired
(spawn_coordinator, scan_entries, handle_team); none added.
