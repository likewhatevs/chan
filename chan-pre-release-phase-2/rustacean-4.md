# @@Rustacean task 4: Phase-2 ride-along cleanup of rustacean-1 nits

Owner: @@Rustacean
Status: REVIEW complete; ready for @@Architect commit coordination
Phase fit: small ride-along on the phase-2 commit if blessed in
time; otherwise its own follow-up commit after the phase lands.

## Goal

Land the four non-blocker improvements I captured during the
phase-2 review pass in [[chan-pre-release-phase-2/rustacean-1.md]]
so the surface I APPROVED ships with the matching polish.

All items are scoped, additive, and behavior-preserving. No wire
shape changes. No new dependencies. No MSRV impact.

## Relevant links

- [[chan-pre-release-phase-2/rustacean-1.md]] (source of the nits)
- [[chan-pre-release-phase-2/backend-2.md]] (search collapse)
- [[chan-pre-release-phase-2/backend-3.md]] (FS-truth)
- [[chan-pre-release-phase-2/backend-4.md]] (language graph)
- [[chan-pre-release-phase-2/backend-1.md]] (markdown tag gate; optional chan-core comment)

## Proposed scope

### 1. `crates/chan-server/src/routes/search.rs`

a. Make `normalized_content_limit(0)` delegate to
   `default_content_limit()` so the two paths share a single
   default value (20).
b. Break `expanded_content_candidate_limit` into two named locals
   (`widened`, `cap`) so the "widen 8x, cap at max(limit, 200)"
   rule reads obviously without consulting the test.
c. Add one rustdoc line on `collapse_hits_by_file` stating
   "input iterator is assumed score-descending; verified upstream
   in `chan-drive::index::fusion`." Locks the invariant.
d. Drop the unnecessary `|existing: &ContentHit|` closure type
   annotation (inference handles it).

### 2. `crates/chan-server/src/routes/graph.rs`

a. Rustdoc on `indexed_file_exists` explaining the
   `symlink_metadata` choice: "regular files only; in-drive
   symlinks (even healthy) are treated as missing so the graph's
   display truth matches what `chan-drive` would re-index next
   pass under lstat semantics." Pins the design decision against
   a future "fix" to `metadata()`.
b. Three additional focused tests for `build_language_graph` in
   `routes::graph::tests`:
   * `language_graph_breaks_ties_by_code_then_path` — two folders
     with equal `files`, different `code` → code-desc wins; equal
     both → path-asc wins.
   * `language_graph_renders_root_folder_with_slash_label` — a
     file at the drive root produces `folder:` id, `/` label,
     and `path: ""`.
   * `language_graph_clamps_depth_to_max_depth` — request
     `depth=99` against a 3-folder language returns 3 ranked
     edges, not 99.
c. Defensive `u32::try_from(folders.len()).unwrap_or(u32::MAX)`
   for the `max_depth` computation to match the same pattern
   used for `idx + 1` in the rank loop. Style consistency only.

### 3. `../chan-core/crates/chan-drive/src/drive.rs` (optional)

One comment line above the new `tokens.retain` saying
"`#tag` graph edges are a markdown-only feature; see
`fs_ops::is_markdown_file`." Saves a future reader the hop to
`fs_ops`.

This is the only sibling-repo change in this task. If
@@Architect prefers to keep phase-2 chan-core touches frozen
after backend-1 already shipped, drop item 3 and land items
1+2 only.

## Out of scope (deliberately)

- No wire shape changes.
- No new query params or routes.
- No additional dependencies.
- No behavior changes; tests should still pass without modifying
  any existing assertion.

## Acceptance criteria

1. `cargo fmt --all -- --check` clean (both repos if item 3 is in).
2. `cargo clippy --all-targets -- -D warnings` clean (both repos).
3. `cargo test -p chan-server` passes; new test count = 100 + 3 = 103.
4. `cargo test -p chan` still 50.
5. `cargo test -p chan-drive` still 429 (no change unless item 3 is in;
   item 3 is comment-only and does not change counts).
6. `cargo build --no-default-features` clean.
7. No diff in any serialized JSON payload from
   `/api/search/content`, `/api/graph`, or `/api/graph/languages`
   (verifiable by spot-curl against the running 8788 test service
   once changes are in).

## Test expectations

- The three new `build_language_graph` tests assert behavior the
  current implementation already produces; this is regression
  pinning, not new behavior.
- No edits to existing tests.

## Hardening / review

- @@Rustacean self-review (I wrote the nits; the diff is the
  smallest possible expression of each).
- @@Architect direction on whether item 3 (chan-core comment)
  rides along or stays deferred.
- No @@Syseng or @@Webtest impact — pure Rust polish + comments.

## Decision asked of @@Architect

Three options, in increasing scope:

* **A.** Land items 1+2 only (chan-server polish). Smallest
  surface; no sibling-repo churn.
* **B.** Land items 1+2+3 (chan-server polish + chan-core
  one-line comment). Polishes both surfaces I reviewed.
* **C.** Defer entirely; keep as a post-phase follow-up.

Default if no direction: A.

## Progress notes

* 2026-05-16 recycled @@Rustacean boot: read
  [[chan-pre-release-phase-2/request.md]],
  [[chan-pre-release-phase-2/journal.md]],
  [[chan-pre-release-phase-1/summary.md]],
  [[chan-pre-release-phase-2/rustacean-1.md]],
  [[chan-pre-release-phase-2/rustacean-2.md]],
  [[chan-pre-release-phase-2/rustacean-3.md]], and this file.
  No [[chan-pre-release-phase-2/architect-rustacean-*.md]]
  inbound exists. Holding this task at TODO until @@Architect
  chooses A/B/C.
* 2026-05-16 @@Rustacean: Alex asked me to check for more
  tasks and do them. Picked default option A from this file:
  chan-server-only polish, no sibling-repo churn.

## Completion notes

Changed files:

- `crates/chan-server/src/routes/search.rs`
- `crates/chan-server/src/routes/graph.rs`
- `chan-pre-release-phase-2/rustacean-4.md`
- `chan-pre-release-phase-2/architect-rustacean-3.md`

Completed scope:

- `normalized_content_limit(0)` now delegates to
  `default_content_limit()`.
- `expanded_content_candidate_limit` uses named `widened` and
  `cap` locals.
- `collapse_hits_by_file` documents the score-descending input
  contract and keeps the closure inference cleanup by typing the
  output vector.
- `indexed_file_exists` documents the regular-file / lstat /
  symlink-as-missing invariant.
- `build_language_graph` computes `max_depth` with a defensive
  `u32::try_from(...).unwrap_or(u32::MAX)`.
- Added focused language-graph regression tests for code/path
  tie-break order, root-folder rendering, and depth clamping.

Verification:

- `cargo fmt --all -- --check`
- `cargo test -p chan-server routes::search::tests` = 3 passed
- `cargo test -p chan-server routes::graph::tests` = 17 passed
- `cargo test -p chan-server` = 103 passed
- `cargo test -p chan` = 50 passed
- `cargo clippy --all-targets -- -D warnings`
- `cargo build --no-default-features`

Not run:

- `cargo test -p chan-drive` because option A intentionally did
  not touch the sibling `chan-core` checkout.
- Spot-curl JSON diff because no 8788 service was running and the
  changes are internal cleanup / tests only.

## Commit readiness

- Ready for @@Architect commit coordination. Suggested subject if
  committed separately: `polish: rustacean-1 non-blocker tidies on
  phase 2 backend surfaces`.
