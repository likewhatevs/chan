# systacean-2: Graph showing links to files not in the repo

Owner: @@Systacean
Date: 2026-05-19

## Goal

The graph view renders edges pointing at files the graph itself
reports as "not in the repo". Either:

* Filter those edges out of the rendered graph, OR
* Surface them with a clear "missing target" state (different
  styling, badge, etc.) so the user understands the edge points
  at a non-existent file.

Pick the right behaviour after looking at the indexer's current
intent.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) under "The
graph shows links to files that it says are not in the repo".

Screenshot: [`../attachments/image-1.png`](../attachments/image-1.png).

Repro: seed a drive with chan's own source code and journals
(this very repo); the graph then shows edges to files outside
the indexed set.

## Acceptance criteria

* Confirmed repro using the chan repo as a seeded drive.
* Root cause documented in this task file (likely either the
  indexer pre-resolving wiki-links that point outside the
  indexed root, or the graph render keeping placeholder edges).
* Behaviour decision (filter vs. label) implemented + tested.
* Webtest verification by @@WebtestA or @@WebtestB after the
  fix lands.

## How to start

* `crates/chan-drive/src/` indexer + graph code paths.
* `web/src/components/GraphCanvas.svelte` for the render side.
* Cross-reference how `[[link]]` parsing resolves targets that
  fall outside the indexed scope.

## 2026-05-19 — repro, root cause, fix

### Repro

Built `./target/debug/chan`, seeded `/tmp/chan-sys2-drv` with
this repo (rsync minus `.git/`, `node_modules/`, `target/`,
`web/dist/`, `desktop/src-tauri/target/`), registered, indexed:

```
chan add /tmp/chan-sys2-drv --name sys2
chan index /tmp/chan-sys2-drv     # 432 files, 5178 chunks, 0 errors
chan serve /tmp/chan-sys2-drv --no-browser --port 0
```

Hit `/api/graph?scope=drive` and diff'd reported file nodes
against `os.path.isfile` truth. 5 file nodes flagged
`missing: true` while the file existed on disk as a regular
file:

* `LICENSE`
* `desktop/LICENSE`
* `crates/chan-drive/src/library.rs`
* `crates/chan-drive/src/registry.rs`
* `docs/journals/phase-1/fake-codex-smoke.sh`

Cross-referenced incoming edges. Every one of these had a
markdown link from some markdown source pointing at it:

```
crates/chan-drive/README.md       -> LICENSE                  broken=True
crates/chan-tunnel-client/README.md -> LICENSE                broken=True
crates/chan-tunnel-proto/README.md -> LICENSE                 broken=True
desktop/README.md                 -> desktop/LICENSE          broken=True
docs/journals/phase-8/systacean/systacean-1.md -> crates/chan-drive/src/library.rs  broken=True
docs/journals/phase-8/systacean/systacean-1.md -> crates/chan-drive/src/registry.rs broken=True
docs/journals/phase-3/webtest-4.md -> docs/journals/phase-1/fake-codex-smoke.sh broken=True
```

Every "broken" target is a real on-disk file.

### Root cause

In `crates/chan-server/src/routes/graph.rs::api_graph` the link
resolver universe was `graph.files() ∪ image_files` only. The
indexer is markdown-only by design (`graph.files()` returns the
`nodes WHERE kind IN ('file','contact')` rows, which are just
the indexed markdown set), so any link from a markdown file to
a non-markdown, non-image file (LICENSE, .rs source, shell
scripts, .toml configs, ...) fell through to the ghost path:

1. `resolve_link_dst` failed to find the dst in `file_set` →
   returned the verbatim string.
2. `ghost_set.insert(dst)` ran → a `GraphNodeView::File {
   missing: true }` was synthesized for the path.
3. `merge_filesystem_layer` later walked the disk and would
   have added the same path as `File { missing: false }`, but
   the ghost insertion happened first and `nodes.entry(id).or_insert(...)`
   preserved the wrong entry.
4. The edge `broken: !present_file_set.contains(dst)` saw the
   same `graph.files() ∪ image_files` universe and reported
   `broken: true` for every such link.

`api_backlinks` shared the same flawed resolver universe, so
"linked from" panels for LICENSE / source files would have
been empty even when README.md genuinely pointed at them.

### Fix

`crates/chan-server/src/routes/graph.rs`:

* Replaced `drive_image_files` with `drive_disk_files` (the
  full on-disk file set via `Drive::list_tree`) plus a small
  `image_subset` helper.
* `api_graph`: `file_set` and `present_file_set` now include
  every regular drive file, so `resolve_link_dst` /
  ghost-classification / broken-flag classification all see
  the true file universe. A new `referenced_disk_files` set
  mirrors the existing `referenced_images` path, emitting a
  `File { missing: false }` node for any link target that is
  on disk but not a graph node and not an image.
* `api_backlinks`: same `file_set` expansion so backlinks for
  on-disk non-graph files actually surface their incoming
  links.

Behaviour outcome (per the spec's filter-vs-label choice):
**label, correctly**. Truly missing targets still synthesize
a ghost node with `missing: true` and the edge gets `broken:
true`. Existing-but-non-graph targets now render as solid
file nodes with `missing: false` and the edge is `broken:
false`.

### Tests added

`crates/chan-server/src/routes/graph.rs::tests`:

* `drive_disk_files_includes_non_markdown_targets` — pins the
  universe contract: LICENSE / .rs / .sh / .md all show up.
* `link_to_non_markdown_disk_file_resolves_to_real_file` —
  regression test mirroring the bug repro. Indexes a markdown
  with `[[LICENSE]]`, writes LICENSE, then asserts the new
  `file_set` (graph_files ∪ disk_files) contains LICENSE so
  the resolver no longer flips to ghost mode.
* `image_subset_picks_image_extensions_only` — sanity check on
  the image / non-image split.

### Verification

* `cargo test -p chan-server graph::` — all 41 tests green
  (3 new + 38 prior).
* `cargo test` (workspace) — green.
* `cargo clippy --all-targets -- -D warnings` — clean.
* `cargo fmt --check` — clean.
* End-to-end against the seeded drive: after rebuild +
  `chan serve`, `/api/graph?scope=drive` returns 891 file
  nodes with **0** on-disk-but-marked-missing mismatches
  (vs 5 before) and **0** link edges flagged broken whose
  target is a real on-disk file (vs 282 before).

### Out of scope / follow-ups

* The SPA's `GraphPanel.svelte::isFileGhost` derives a second-
  ghost state from `tree.entries`, which is lazy-loaded. For a
  selected file in an unexpanded file-browser subtree, the
  inspector still falls back to "not in the current file
  listing (try Reload / chan index)" even when the server
  says `missing: false`. That is a separate frontend bug
  (the screenshot's hint text matches that path). Flagging
  it for a @@FullStack follow-up rather than rolling it into
  this commit, since it lives in `GraphPanel.svelte` and the
  fix shape (drop the lazy-tree check, or gate it on
  `tree.loadedDirs` covering the path's parent) is firmly in
  the SPA lane.

### Status

Server-side fix landed in the working tree (uncommitted).
Awaiting @@Architect commit clearance.

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Methodical work: repro against the chan repo seeded as a drive,
crisp root cause (the resolver universe being graph_files ∪
image_files rather than the full on-disk file set), correct
fix to expand to `drive_disk_files` + add the
`referenced_disk_files` mirror of the image path. Backlinks
fix in the same pass is the right call — same flawed universe,
same fix.

Before/after numbers are the proof: 5 ghost-flagged real files
→ 0; 282 broken edges with real targets → 0.

Three new tests pin the contract (universe, regression repro,
image subset split).

**Commit clearance**: approved. Suggested subject:

```
Graph: link resolver universe includes all on-disk files, not just markdown + images (systacean-2)
```

Push waits for Round-1 close.

**Follow-up routed to @@FullStack lane** (per your "out of
scope / follow-ups" note): the SPA's `GraphPanel.svelte::isFileGhost`
lazy-tree second-ghost path still hints "not in current file
listing" even when the server says `missing: false`. I'll cut
a task for it under @@FullStackA after the bug wave settles;
not Round-1 critical. Append a one-line marker to your journal
for the audit trail.

Pick up `systacean-3` (Round-1 close: version bump + tag) once
I publish the commit-grouping plan — which I will after the
remaining bugs in `phase-8-bugs.md` are dispatched and at
least one walkthrough verdict from @@WebtestA / @@WebtestB
lands on the fixes that just cleared.

## 2026-05-20 — committed

Landed as `4a04917`. Subject:

```
Graph: link resolver universe includes all on-disk files, not just markdown + images (systacean-2)
```

Single file in the commit:
`crates/chan-server/src/routes/graph.rs` (+183 / -23). Task
file + journal updates ride on a later doc-poke commit per
the project convention (mirror of `041de34`).

Working tree clean for `crates/chan-server/`. Push held for
Round-1 close per the standing `systacean-3` plan. The
binary can be rebuilt locally now so @@WebtestA's lane-A
bug-8 re-verification has the fix on HEAD.

Per @@Architect's chase: expecting the 5 plain non-markdown
files in WebtestA's repro to clear; the 3 directory-typed-
as-file cases are a separate investigation that I'll flag /
pick up if WebtestA still sees them after the rebuild.
