# @@Rustacean task 1: Rust review pass on phase 2 backend work

Owner: @@Rustacean
Status: REVIEW complete; all four items APPROVED for commit.

## Goal

Specialist Rust review for the phase 2 backend work in REVIEW
state: [[chan-pre-release-phase-2/backend-1.md]],
[[chan-pre-release-phase-2/backend-2.md]],
[[chan-pre-release-phase-2/backend-3.md]] (mirrored by
[[chan-pre-release-phase-2/rustacean-2.md]]), and
[[chan-pre-release-phase-2/backend-4.md]] (mirrored by
[[chan-pre-release-phase-2/rustacean-3.md]]).

Scope is Rust quality, Cargo hygiene, API surface, error handling,
test coverage on non-trivial logic, and confirmation of the full
gate (fmt / clippy / tests / `--no-default-features`).

## Relevant links

- [[chan-pre-release-phase-2/journal.md]]
- [[chan-pre-release-phase-2/request.md]]
- [[chan-pre-release-phase-1/summary.md]]

## Files reviewed

chan-core (sibling repo, path-dep):

- `crates/chan-drive/src/fs_ops.rs` (new `is_markdown_file`)
- `crates/chan-drive/src/drive.rs` (tag-token filter for non-md)
- `crates/chan-drive/tests/file_types.rs` (regression test)

chan (this repo):

- `crates/chan-server/src/routes/search.rs` (per-file collapse)
- `crates/chan-server/src/routes/graph.rs` (FS-truth, language graph)
- `crates/chan-server/src/routes/mod.rs` (re-export)
- `crates/chan-server/src/lib.rs` (route registration)

## Gate

```
cargo fmt --all -- --check                # clean (both repos)
cargo clippy --all-targets -- -D warnings # clean (both repos)
cargo test -p chan-server                 # 100 passed (was 92 in phase 1)
cargo test -p chan                        # 50 passed
cargo test -p chan-drive                  # 429 passed across crates
cargo build --no-default-features         # clean
```

Targeted re-run of the new tests:
`cargo test -p chan-server -- routes::search::tests routes::graph::tests`
= 17 passed (0 filtered failures).

## Per-task verdicts

### backend-1 (markdown tag gate, chan-core)

APPROVED for commit.

Strengths:

- New `is_markdown_file(rel)` is narrow and rustdoc-documented; the
  doc explicitly contrasts with `is_indexable_text` and explains why
  `.txt` is excluded.
- Implementation uses `rsplit_once('.')` + `eq_ignore_ascii_case("md")`.
  Correct for hidden files (`.gitignore` returns false), uppercase
  (`README.MD` returns true), no-extension (returns false), and the
  empty string.
- `drive.rs` change is minimal: still calls `extract_tokens` (so
  mentions and dates from `.txt` are preserved), but drops `Token::Tag`
  entries when the source path is not markdown. Mentions and dates
  for `.txt` are unchanged.
- Regression coverage is the right shape: `g.tags()` aggregate is
  asserted to contain `phase2` (markdown) and to lack `plain-text`
  (.txt) and `include` (source). The `.py` case doubles as a check
  that source-class is excluded at the walker level too.

Non-blocker nits:

- `drive.rs` could carry a one-liner above the `retain` saying
  "graph tags are a markdown-only feature; see `is_markdown_file`"
  so a future reader of `parse_for_graph` does not have to jump
  out to `fs_ops` to learn why.

### backend-2 (per-file content-search collapse, chan-server)

APPROVED for commit.

Strengths:

- Clean refactor: inline closure replaced with
  `impl From<chan_drive::Hit> for ContentHit`.
- `collapse_hits_by_file` correctly assumes a score-descending input
  iterator. Verified upstream: `chan-drive::index::fusion.rs:59-61`
  sorts the fused hits by `b.score.partial_cmp(&a.score)`, so the
  first-wins-per-path policy is correct.
- Three focused unit tests cover the dedup, the post-dedup limit,
  and the candidate widening formula at boundary inputs (20, 50,
  500).
- `ContentSearchResponse`/`ContentHit` wire shape is unchanged;
  the frontend `ContentHit` contract is preserved.

Non-blocker nits:

- `normalized_content_limit` returns 50 when caller passes
  `limit=0`, but `default_content_limit` (used when the caller omits
  the query param entirely) returns 20. Two sources of truth for the
  default. Suggestion: have `normalized_content_limit(0)` delegate
  to `default_content_limit()`.
- `expanded_content_candidate_limit` is a tight one-liner. The
  formula reads correctly (widen 8x, cap at `max(limit, 200)`),
  but breaking it into two named locals (`widened`, `cap`) would
  read more obviously and survive a future reader who hasn't read
  the tests.
- `collapse_hits_by_file`'s closure has an explicit
  `|existing: &ContentHit|` annotation that the inference would
  catch. Tiny; rustfmt leaves it alone, clippy did not flag.
- Worth a one-line comment in `collapse_hits_by_file` stating the
  "input iterator is assumed score-descending" contract so the
  invariant is local to the function.

### backend-3 / rustacean-2 (FS-truth on `/api/graph`)

APPROVED for commit.

Strengths:

- `indexed_file_exists(root, rel)` uses `symlink_metadata` (lstat),
  consistent with phase 1's chan-drive contract and with the
  symlink-aware watcher hardening in
  [[chan-pre-release-phase-1/summary.md]]. The `unwrap_or(false)`
  fallback handles missing, permission-denied, and EACCES.
- Present-on-disk set is built once per request, with a clean split:
  `present_files` covers indexed files (used to flip
  `missing: true` on file nodes); `present_file_set` extends with
  on-disk image files (used to mark link edges as `broken`).
- Link resolution still consults the full indexed `file_set` so
  extensionless `[[deleted-note]]` link targets continue to land
  on a stable id, then get the broken/missing treatment.
- Tests pin two key invariants: directory at the indexed path is
  not present, and an in-drive symlink (even to a real file) is
  not present. The symlink-as-missing behavior is intentional and
  the test makes that explicit so a future maintainer doesn't
  "fix" it by switching to `metadata()`.

Non-blocker nits:

- `indexed_file_exists` has no rustdoc. A one-line doc stating
  "regular files only; symlinks (even healthy) are treated as
  missing because the graph's display truth is what `chan-drive`
  itself would re-index, and chan-drive's indexer uses lstat
  semantics" would help the next reader.
- Per-request cost is O(N indexed) `symlink_metadata` syscalls.
  Backend-3 explicitly accepts this; for local-only drives in the
  typical size range this is fine. If it ever shows up in a profile,
  the right next step is caching by mtime in `AppState`, not
  changing the lstat call.

### backend-4 / rustacean-3 (`/api/graph/languages`)

APPROVED for commit.

Strengths:

- Wire shape matches what was frozen in
  [[chan-pre-release-phase-2/rustacean-3.md]] / `backend-4.md`.
  `LanguageGraphNode` is a tagged enum (Language / Folder) with
  `#[derive(Debug, Clone, PartialEq, Eq, Serialize)]`, which is
  what enabled the focused unit tests to assert exact nodes.
- The builder operates on `&[ReportFileStats]`, completely
  independent of the live indexer state, so it inherits FS-truth
  from chan-report. No `indexed_file_exists` filter needed here.
- Folder ranking is per-language descending by file count, then
  code, then folder path (path ascending as the deterministic
  tie-break). Matches the acceptance criteria.
- `depth=0` means "max"; non-zero is clamped to `max_depth`. The
  `effective_depth != 0` guard in the rank check is harmless when
  `max_depth == 0` (empty drive yields no edges anyway), and the
  empty-drive test pins that path.
- `language_filter` is lowercased once; per-file language is
  compared case-insensitively. `Rust` matches `rust`.
- Folder node `path` is the drive-relative folder path
  (`""` for the drive root); node id is `folder:<path>`; root id
  is `folder:`; root label is `/`. Consistent with the rustacean-3
  spec.
- New route registered at `/api/graph/languages` in
  `lib.rs::router`, re-exported through `routes::mod`.

Non-blocker nits:

- Tests cover the happy path, the depth + filter trim, and the
  empty-drive case. Three further focused tests would tighten
  coverage:
  1. Tie-break by `code` when `files` are equal.
  2. Tie-break by `path` when both `files` and `code` are equal.
  3. Root-folder rendering: a `path: "lib.rs"` file should produce
     `folder:` / label `/`.
  These are very small and could land in a follow-up if @@Backend
  prefers not to expand scope this close to commit.
- `let max_depth = ... .map(|folders| folders.len() as u32) ...`
  truncates if a language has more than 2^32 folders. Defensive
  `u32::try_from(folders.len()).unwrap_or(u32::MAX)` would match
  the same pattern that the rank loop already uses for `idx + 1`.
  Vanishingly unlikely; style nit.

## Cross-cutting Cargo hygiene

- No new dependencies in either repo. `chan-server` still imports
  `chan_drive::{EdgeKind, ReportFileStats}` from the existing
  path-dep. No feature-flag additions. No build-script changes.
- MSRV (`rust-toolchain.toml` 1.95.0) is unchanged; nothing in the
  diffs uses a feature past 1.95.0.

## Done means

- This file flips to DONE once the four upstream tasks land in the
  commit unit @@Architect schedules.
- Per-task non-blocker improvements are listed above. None block
  the commit; @@Architect can route them to @@Backend as
  follow-ups or fold into a clean-up commit at the architect's
  discretion.

## Commit readiness

- Ready. Rust gate green across chan and chan-core; wire shapes are
  intact; no API regressions; new test coverage is focused and
  asserts the right invariants.
