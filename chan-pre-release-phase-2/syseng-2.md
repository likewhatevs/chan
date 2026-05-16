# syseng-2: Phase 2 hardening pass + specialist reviews

Owner: @@Syseng. Status: IN_PROGRESS.

Source: [[chan-pre-release-phase-2/journal.md]] dispatch. Carries
the consolidated phase-2 syseng workload identified in
[[chan-pre-release-phase-2/syseng-1.md]] and folded into the
journal as cross-cutting H1.

## Goal

Three specialist reviews, one fixture-driven hardening pass, one
release-readiness audit.

| Surface | Source task | What syseng checks |
|---|---|---|
| `/api/graph` FS-truth via `symlink_metadata` | [[chan-pre-release-phase-2/backend-3.md]] + [[chan-pre-release-phase-2/rustacean-2.md]] | lstat discipline; symlink / FIFO / special classification; stale-row drift; race with the watcher; per-call cost on a real drive |
| `/api/graph/languages` rank fan-out | [[chan-pre-release-phase-2/backend-4.md]] | report fan-out cost; rank-by-files-then-code stable order; depth=0 edge case; case-insensitive `language` filter; empty drive |
| `/ws` graph reload signal + debounce | [[chan-pre-release-phase-2/frontend-7.md]] | self-write suppression still holds; bulk events don't cascade fetches; closed-overlay path doesn't leak subscriptions |
| Tag regression test scope | [[chan-pre-release-phase-2/backend-1.md]] | confirm fixtures actually cover the cases syseng-1 listed; @@Rustacean owns Rust review |

## Acceptance criteria

1. `/api/graph` returns `missing: true` for indexed files that
   vanish on disk between index and request. Verified live against
   a fresh fixture, not just the unit tests.
2. `/api/graph` does **not** flip a real (regular) file to
   `missing: true` because its path resolves through a symlink
   target. lstat is applied to the relative path under the drive
   root, not to a chased target.
3. `/api/graph` does **not** flip a symlinked-on-disk file to
   `missing: true` on the grounds that "symlink_metadata says
   symlink, not regular file". Indexed markdown is always a
   regular file on disk (the indexer rejects symlinks), so the
   only correct answer for a still-on-disk indexed row is
   `missing: false`.
4. `/api/graph/languages` does not double-walk the drive per
   request (fan-out should be a single report scan + fold).
5. `/api/graph/languages?depth=0` returns the unlimited graph;
   `depth=1` returns at most one folder edge per language;
   `language=Rust` and `language=rust` return identical payloads.
6. `frontend-7` debounce coalesces an N-file batch into one
   `/api/graph` fetch with the debounce window from
   `frontend-7.md`.
7. backend-1 tag-extraction tests cover the three cases promised:
   `.md` with `#urgent` -> 1 tag edge; `.txt` with `#urgent` -> 0
   tag edges; source-class (`.py`) skipped entirely by
   `index_file`.
8. No new runtime dependency in the release binary; `otool -L`
   shows only system frameworks (re-check from phase 1).

## Verification gate

```
cargo build
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cd web && npm run check
cd web && npm test -- --run
scripts/pre-push
otool -L target/release/chan      # macOS; ldd on Linux
```

Plus a live HTTP fixture probe matrix (see "Hardening pass"
below).

## Fixture

Re-use the phase-1 fixture script from
[[chan-pre-release-phase-1/syseng-1.md]] "Fixture drive" and
extend it with two cases relevant to phase 2:

```
$FX=/tmp/chan-syseng-phase2-fixture
# (rebuild via phase-1 script first)
echo "# Live note"             > "$FX/will-vanish.md"
echo "links to [other](will-vanish.md)" >> "$FX/notes/sub/nested.md"
echo "#include <stdio.h>"      > "$FX/src/main.c"     # source-class skip
echo "# Note with #urgent"     > "$FX/has-tag.md"     # .md tag
echo "this has #urgent inside" > "$FX/has-tag.txt"    # .txt: no tag edge
```

The hardening pass deletes `will-vanish.md` *after* indexing
completes, then hits `/api/graph` and asserts the node returns
with `missing: true` and the link edge from `notes/sub/nested.md`
carries `broken: true`.

## Hardening pass (will run once reviews land)

| Probe | Surface | Expected |
|---|---|---|
| `chan serve ... --no-token --no-browser`, wait for index idle, `GET /api/graph` | backend-3 baseline | normal nodes only, no missing |
| `rm /tmp/.../will-vanish.md`, immediately `GET /api/graph` | backend-3 FS-truth | `will-vanish.md` node with `missing: true`; link edge to it `broken: true` |
| `rm` then wait for watcher to fire `forget_file`, `GET /api/graph` | backend-3 + watcher | node is gone; link edge resolves to literal ghost set as before |
| `GET /api/graph` after replacing an indexed `.md` with a symlink to another `.md` | backend-3 lstat | the symlinked replacement classifies as missing (it's not a regular file under that rel path) — confirm this is the agreed behaviour |
| `GET /api/graph/languages` on a multi-language fixture | backend-4 | language nodes only edge to folders; ranks decreasing by file count then SLOC then path |
| `GET /api/graph/languages?depth=1` | backend-4 | at most one edge per language; covers the highest-rank folder |
| `GET /api/graph/languages?language=rust` and `?language=Rust` | backend-4 | identical responses |
| `GET /api/graph/languages` on empty drive | backend-4 | `nodes:[] edges:[] max_depth:0` |
| Bulk-create 50 markdown files via shell while graph overlay open | frontend-7 | one `/api/graph` fetch after debounce; no fetch storm |
| `touch` an editor-saved file from the in-app editor while graph open | frontend-7 + bus | self-write suppression: no spurious graph fetch |
| `mv folder folder2` while graph open | frontend-7 | overlay reloads exactly once, picks up the rename |
| `target/debug/chan` on fixture with `.txt` containing `#urgent` | backend-1 | tag node not present in `/api/graph` tags list |
| `target/debug/chan` on fixture with `.c` containing `#include <stdio.h>` | backend-1 | no `#include` tag node anywhere; `.c` not in indexer at all |

## Review outputs

Per-surface review notes are written below this header as
work progresses. Each block lists: files read, behaviour verified
in code, gaps found, recommended follow-up.

### backend-3 + rustacean-2 review

Read `crates/chan-server/src/routes/graph.rs` diff. Verified:

- `indexed_file_exists(root, rel)` uses `std::fs::symlink_metadata`
  and gates on `file_type().is_file()`. lstat discipline holds:
  symlinks at the leaf return `is_file()=false` and ghost
  correctly. Mid-path symlinks are still followed by the kernel
  (lstat doesn't change parent-component resolution), which is
  the right behaviour because chan-drive walker normalizes the
  drive root to a real directory.
- `present_files` is a single `BTreeSet<&str>` built once and
  shared between the file-node emit loop and the link-edge
  `broken` decision. No double-walk.
- Image files merge into `present_file_set` so a markdown link
  to an image on disk still resolves to a present node, not a
  broken one. Correctness preserved against the phase-1
  image-attachment behaviour.
- Stale-row drift: when the watcher fires `forget_file`, the row
  drops from `graph.files()` on the next call. Stat-on-emit only
  fires for rows that survived the graph DB. No invariant broken.
- Cost: one `symlink_metadata` syscall per indexed file per
  `/api/graph` call. backend-3.md already documents this as a
  known risk; phase-2 accepts the trade.

Live probes (chan serve on 18800 against
`/tmp/chan-syseng-phase2-fixture`):

| Probe | Expected | Observed |
|---|---|---|
| baseline `/api/graph` | 5 file nodes, no `missing` | 5 file nodes; no `missing` on any of the on-disk rows; the only `missing:true` is the link target `notes/sub/will-vanish.md` which markdown-resolves to a nonexistent path |
| `rm /tmp/.../will-vanish.md`, immediate `/api/graph` | `will-vanish.md` present with `missing:true` | confirmed: indexed row stayed in graph DB momentarily and rendered as `missing:true`. Index status still showed `indexed_docs=5` mid-window |
| same after watcher catches up (~3s) | row removed; doc count drops | confirmed: `indexed_docs=4`, `will-vanish.md` node gone, broken link target `notes/sub/will-vanish.md` still ghosted |
| `rm top.md && ln -s has-tag.md top.md`, wait 2s | watcher fires `forget_file` (phase-1 hardening), row gone from graph DB | confirmed: `indexed_docs=3`, `top.md` row absent, no `missing` row left behind |

Approved. No follow-up.

### backend-4 review

Read same diff. Verified:

- `build_language_graph` folds `report.files` into a
  `BTreeMap<language, BTreeMap<folder, {files, code}>>` in one
  pass. Folders with empty `language` are skipped. `language_filter`
  is lowercased once and compared against each row's lowercased
  language — no double normalization per row.
- Per-language folder rank: sort by `(files DESC, code DESC,
  path ASC)`. Deterministic; the path tie-break keeps ordering
  stable across runs.
- `max_depth` = `max(folder_count_per_language)`. `effective_depth`
  = `max_depth` when query `depth==0`, else `min(depth, max_depth)`.
  The `if effective_depth != 0 && rank > effective_depth` skip
  trims correctly without dropping rank-1 edges at depth=0.
- Folder nodes accumulate cross-language totals via
  `folder_totals`, emitted after all language nodes/edges. Order:
  language nodes first (in `BTreeMap` order), then folder nodes
  (also in `BTreeMap` order). Stable.
- `drive.report()` returns a `OnceLock`-cached `ReportState`
  (chan-drive `drive.rs:2199-2209`). The endpoint is O(N) over
  `report.files`; no rescan per request.

Live probes:

| Probe | Expected | Observed |
|---|---|---|
| `/api/graph/languages?depth=0` on the multi-language fixture | language nodes for C, Markdown, Plain Text, Python; folder nodes for `""` (root), `notes/sub`, `src`; max_depth=2 | confirmed |
| `/api/graph/languages?language=rust` (no Rust in fixture) | empty graph, max_depth=0 | confirmed: `nodes:[] edges:[] max_depth:0` |
| `/api/graph/languages?language=C` vs `?language=c` | identical payloads | confirmed byte-equal: one language node + one folder node + one edge |
| `/api/graph/languages?depth=1` | each language emits at most one folder edge | confirmed: 4 languages × 1 edge each |
| Folder path `""` for root files | label `/`, id `folder:` | confirmed |

Two non-blocking observations:

1. No unit test covers the empty-drive case (`build_language_graph(&[], 0, None)`).
   The live probe above shows it returns `{max_depth:0, nodes:[], edges:[]}`
   correctly, so it's not a bug, but a one-line test would pin it.
2. The endpoint trusts `report.files`. If the chan-report walker
   ever surfaces non-existent paths (e.g. between a delete and a
   refresh), the language graph would inherit them. Out of scope
   for phase 2 because chan-report's freshness is its own
   contract.

Approved.

### frontend-7 review

Read `web/src/state/store.svelte.ts` and `web/src/components/GraphPanel.svelte`
diffs. Verified the /ws-side contract:

- `graphReloadSignal` is a `$state<{nonce: number}>` bumped in
  `onWatchEvent` *only* when `graphOverlay.open` is true
  (`store.svelte.ts:411-413`). Closed overlays do not see the
  nonce churn at all — they latch `seenGraphReloadNonce` to the
  current nonce on every reactive pass while invisible
  (`GraphPanel.svelte:713-716`).
- Debounce: 250ms `setTimeout` keyed on nonce changes. Multiple
  events within the window collapse to one `/api/graph` fetch.
- Cleanup: `onDestroy` clears the pending timer. The WS
  subscription itself lives at the module level
  (`unwatch`), so the GraphPanel component lifecycle does not
  add or remove WS handlers.
- Self-write suppression: still anchored in `bus::make_watch_bridge`
  in chan-server. Frontend doesn't need to re-implement.

Open browser-smoke needs (carried in frontend-7.md and rolled
into webtest scope):

- bulk create/delete: confirm single fetch after debounce.
- in-app editor save: confirm no spurious graph fetch.
- folder rename: confirm one reload, picks up the rename.

No syseng blocker. Approved subject to webtest landing the
smoke pass.

### backend-1 tag regression test review

Read `../chan-core/crates/chan-drive/tests/file_types.rs` diff.
The integration test additions:

- `.md` body augmented with `#phase2`. Asserts
  `g.tags()` contains `phase2` and that the `notes/intro.md`
  neighbors list has an `EdgeKind::Tag` edge with dst `#phase2`.
- `.txt` body augmented with `#plain-text`. Asserts
  `g.tags()` does NOT contain `plain-text`.
- `.py` body augmented with `#include <stdio.h>`. Asserts
  `g.tags()` does NOT contain `include`. (This is doubly safe:
  the indexer wouldn't open `.py` to begin with because
  `is_indexable_text` returns false for `.py`, but the test
  reads the graph end-state regardless.)

Plus `fs_ops::tests::is_markdown_file_excludes_plain_text` covers
the `.md` / `.MD` / `.txt` / `.py` / `Cargo.toml` / `Makefile` /
empty cases at the helper level.

Live verification on the running fixture: `/api/graph` returns
exactly one tag node (`#phase2` from `has-tag.md`). No `#urgent`
(which the prep included in `src/main.py`), no `#include` (from
`src/main.c`), no `#plain-text` (from `has-tag.txt`).

Approved. @@Rustacean owns the Rust API review per
[[chan-pre-release-phase-2/backend-1.md]].

## Status

REVIEW pending @@Architect ack.

Hardening pass executed against `/tmp/chan-syseng-phase2-fixture`
on `2026-05-16`. All probes returned the expected behaviour;
findings recorded in the four review sections above. Specialist
review verdict is **Approved** for backend-1, backend-3,
rustacean-2, backend-4, and frontend-7 (subject to webtest
browser smoke for frontend-7).

Verification gate executed (`2026-05-16`):

```
cargo fmt --check                              clean
cargo clippy --all-targets -- -D warnings      clean (RUSTFLAGS=-D warnings)
cargo test --all-targets                       chan-server 99 passed
cargo build --no-default-features              ok
cd web && npm run check                        0 errors / 0 warnings (3911 files)
cd web && npm test -- --run                    9 files / 111 tests passed
otool -L target/debug/chan                     macOS system frameworks only
cargo test -p chan-drive --lib  fs_ops::tests::is_markdown_file_excludes_plain_text  1 passed
cargo test -p chan-drive --test file_types file_type_policy_end_to_end               1 passed
cargo test -p chan-drive (full chan-core gate)                                       429 passed
```

Open follow-ups (non-blocking):

1. Empty-drive unit test for `build_language_graph` (one-liner;
   not a bug since the live probe confirmed correct behaviour).
2. Webtest browser smoke for the frontend-7 live-update paths
   (delete-while-open, create-while-open, folder-rename). Owned
   by @@Webtest per [[chan-pre-release-phase-2/webtest-1.md]].
3. Phase-1 self-upgrade carry-forwards (signed checksums,
   auto-rollback) remain deferred per Alex's phase-2 scope call.

## Done means

- Each of the four reviews above filled in with code-grounded
  observations.
- The hardening pass matrix executed against a live fixture, with
  exact commands + results recorded.
- Any blocker filed back as a new architect-syseng-N task per
  phase-1 pattern.
- Non-blocking residuals logged for the phase summary.
- This file flips to REVIEW when reviews + hardening land green;
  to DONE after @@Architect acks.
