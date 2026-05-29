# systacean-4: Graph indexer flags 3 directories as kind=file then fails the presence check

Owner: @@Systacean
Date: 2026-05-20

## Goal

Eliminate the "directory typed as file" leak in the graph
indexer. @@WebtestA's Round-1 sweep found 3 directory
entries in the graph's missing-nodes list with `kind: file`,
which means the indexer wrote a directory path into the
file-node table; the presence check then correctly flags
the path as "not a regular file" and the graph renders them
as ghost-missing. The bug is upstream: directories should
never appear as file nodes in the first place.

## Background

Side observation from @@WebtestA's Round-1 sweep on
2026-05-20:

> 3 directories typed as `kind: file` then failing the
> presence check.

Filed in [`../phase-8-bugs.md`](../phase-8-bugs.md).
Distinct from `systacean-2` (which expanded the resolver
universe so plain non-markdown files would resolve cleanly)
and from `fullstack-a-12` (which is the SPA-side second-
ghost fix). This is upstream of both: an indexer typing
bug that leaks directory entries into the file-node table.

## Pre-task verification step

**First**, confirm the bug still reproduces after
`systacean-2` lands and the binary rebuilds. There is a
chance the resolver-universe expansion accidentally cleared
this symptom too (unlikely but worth checking before
designing a fix).

Steps:

1. Once `systacean-2` is committed (your prior task —
   currently sitting in working tree at
   `crates/chan-server/src/routes/graph.rs`), rebuild via
   `cargo build -p chan`.
2. Restart @@WebtestA's lane-A server or spin up a fresh
   one against a chan-source-seeded drive.
3. Pull `/api/graph?scope=drive` and grep for nodes with
   `kind: "file"` whose `path` resolves to a directory on
   disk.
4. If 0 such nodes: the symptom cleared, mark this task as
   "no longer reproduces" and close. Append the verification
   notes to the task tail.
5. If still N > 0: proceed with the fix below.

## Acceptance criteria

* `/api/graph?scope=drive` returns 0 nodes with `kind:
  "file"` whose path resolves to a directory on disk.
* No regression on the `systacean-2` resolver-universe
  expansion (plain non-markdown files still resolve to
  `missing: false`).
* No regression on the file-node table for actual files.
* Regression test pinning that directory entries never
  leak into the file-node table.

## How to start

1. Look at the indexer's file-discovery path. The graph
   route lives in `crates/chan-server/src/routes/graph.rs`;
   the indexer it consumes is in `chan-drive`. Find where
   the indexer enumerates "files" and decide whether
   filtering happens at enumeration time or downstream.
2. Likely culprits:
   * A walker that yields directory entries without
     filtering on file-type.
   * A `kind` field defaulting to `"file"` when the entry
     is actually a directory.
   * A symlink-following bug that follows a directory
     symlink and registers the target as a file.
3. Add the type-check filter at the right layer (probably
   at the walker boundary).
4. Pin with a test mirroring the shape of the
   `link_to_non_markdown_disk_file_resolves_to_real_file`
   test you added in `systacean-2`: index a drive with
   `[[some-dir]]` link and a real directory at that path,
   assert the graph does NOT include the directory as a
   file node.

## Coordination

* Sequence: commit `systacean-2` first, rebuild, re-verify
  bug 8 holdout in the graph response. Only proceed with
  this task if the symptom still reproduces.
* @@WebtestA verifies on lane-A drive once landed.

## 2026-05-20 — pre-task verification: bug still repros + root cause

### Verification on the rebuilt binary

Built `cargo build -p chan` (37.6s, clean). Started a fresh
`./target/debug/chan serve /tmp/chan-sys2-drv --port 8889
--no-browser` against the same chan-source-seeded drive from
`systacean-2`. Lane-A / lane-B servers (ports 8787, 8820)
left untouched — they're on the pre-fix binary; only my port
8889 has the new code.

`/api/graph?scope=drive` returns:

* 891 file-kind nodes total (was 891 pre-fix per systacean-2's
  end-to-end check — same shape, expected: my fix changed
  classification but not count for files that were already on
  disk).
* 90 file-kind nodes flagged `missing: true`.
* Of those 90, **3 paths resolve to real directories on disk**:

```
docs/agents
docs/journals/phase-7/alex
docs/journals/phase-8/alex
```

Each matches a real directory in both the test drive and the
live repo (confirmed via `ls -ld`).

The remaining 87 `missing: true` nodes are legitimate ghosts:
markdown links to paths that don't exist on disk (mostly
stale skill-file paths from the `docs/agents/skills/` →
`docs/agents/<agent>/skills/` reorg, plus other actual broken
links). Not in scope for this task.

**Symptom did not clear after systacean-2.** Proceeding.

### Root cause: ghost path emits dirs as `kind: file`, not an
indexer leak

The task's "likely culprits" guess (indexer walker yielding
directories) is wrong. Walked the chain:

1. The 3 directory paths appear in markdown links:
   * `docs/journals/README.md:33` —
     `cards live at [\`../agents/\`](../agents/).` (resolves to
     `docs/agents`).
   * `docs/journals/phase-7/process.md:75` —
     `[\`../../agents/\`](../../agents/)` (same target).
   * `docs/journals/phase-7/architect/journal.md:271` —
     `[../alex/](../alex/)` (resolves to
     `docs/journals/phase-7/alex`).
   * `docs/journals/phase-8/architect/journal.md:164` —
     `[../alex/](../alex/)` (resolves to
     `docs/journals/phase-8/alex`).
2. The indexer's `nodes` table never gets these dir paths —
   `INSERT INTO nodes` calls in
   `crates/chan-drive/src/graph.rs` only fire for markdown /
   contact files. Confirmed by audit of every INSERT site
   (`replace_file`, `replace_all`, `swap_staging`).
3. The leak lives in
   `crates/chan-server/src/routes/graph.rs::api_graph`'s
   ghost path. For each `Link`-kind edge whose dst is not in
   `file_set` (= `graph_files ∪ disk_files`), the dst is
   inserted into `ghost_set`, which later emits
   `GraphNodeView::File { missing: true, ... }`.
4. `disk_files` (my `drive_disk_files` helper from
   systacean-2) filters `!e.is_dir`, so directory link
   targets fall through to ghost emission with `kind: file`.

So the bug is in api_graph, not the indexer. Surface-level
description in the task ("indexer typing leak") is misleading;
the actual symptom is the route layer fabricating a file node
for a non-file link target.

## 2026-05-20 — scope question for @@Architect

The fix shape isn't obvious without input. Three options:

**A. Drop directory dsts from ghost emission AND drop the
edge.** Smallest blast radius. Markdown links to directories
are doc navigation, not graph content; the graph simply
omits them. SPA renders unchanged. Acceptance criterion
(0 file-kind nodes resolving to dirs) is met directly.

**B. Emit directory dsts as a new `GraphNodeView::Directory`
variant.** Preserves the edge; surfaces directory references
in the canvas. Requires SPA work to render and style
directory nodes. Broader scope; arguably scope-creep for a
Round-1 bug.

**C. Emit directory dsts as `File { missing: false }` with
`path_class.kind = "directory"`.** Hybrid — no schema change,
existing variant, honest signal via `path_class`. The
inspector might mis-treat them as files for editor / preview
actions; some SPA-side guard would still be needed (so it's
not strictly "no SPA change").

My recommendation: **A**. Reasons:
* Markdown links to directories don't carry graph semantics —
  they're navigation. The graph view is "between-file
  relationships"; a dir target is a category mismatch.
* Smallest patch, no SPA work, no schema growth. Fits the
  syseng minimal-targeted-fix discipline.
* The "linked from" panel on a directory has no meaningful
  use — there's no file to open. So losing the edge in the
  graph loses nothing the user could act on.
* Easy to reverse if a future SPA feature wants to surface
  directory references — flip option A → option B is a
  pure addition.

If A is approved I'll add a `disk_dirs` set parallel to
`disk_files`, filter `ghost_set` insertion on it, and skip
emitting matching edges (or rely on the existing
target-not-in-nodes filter — would need a brief audit).
Test pattern would mirror
`link_to_non_markdown_disk_file_resolves_to_real_file` from
systacean-2: seed a markdown with `[label](some/dir/)` and a
real dir at `some/dir`, assert the graph has 0 nodes for
`some/dir` and 0 edges targeting it.

## 2026-05-20 — @@Architect: scope answer (option A approved)

Reviewer: @@Architect.

Option A approved. Your reasoning is correct on every
point:

* Markdown links to directories are doc navigation, not
  graph content. "Between-file relationships" is the
  graph's contract; a directory target sits outside that.
* Smallest patch, no schema growth, no SPA work. Fits the
  Round-1 patch-release window.
* The "linked from" inspector panel on a directory has no
  meaningful affordance — there's no file to open, no
  preview to render. So the dropped edge loses nothing
  the user could act on.
* The A → B upgrade (adding a `Directory` variant later)
  is a strictly-additive change; we can revisit when an
  SPA feature genuinely wants directory references in the
  canvas.

Excellent root-cause work. The "indexer typing leak" hypothesis
in the original task spec was a misread on my part — your
audit of every `INSERT INTO nodes` site in
`crates/chan-drive/src/graph.rs` proves the leak isn't in the
indexer at all; it's `api_graph` fabricating file nodes for
non-file link targets. That kind of "the task spec's stated
cause is wrong, here's what's actually happening" finding is
exactly what the append-only audit trail captures well.

### Implementation plan (your spec)

* Add `disk_dirs` set parallel to `disk_files`.
* Filter `ghost_set` insertion on it.
* Skip emitting matching edges (audit the existing
  target-not-in-nodes filter first; if it already covers
  the edge drop, no extra code needed; if not, add the
  guard).
* Test mirroring `link_to_non_markdown_disk_file_resolves_to_real_file`:
  seed a markdown with `[label](some/dir/)` + real dir at
  `some/dir`; assert 0 file-kind nodes for `some/dir` + 0
  edges targeting it.

Go ahead. Pre-push gate as standard before commit-readiness.

### Updated acceptance criteria

(Overriding the original spec's acceptance criteria where
they implied an indexer fix.)

* `/api/graph?scope=drive` returns 0 file-kind nodes
  whose path resolves to a directory on disk.
* `disk_files` / `disk_dirs` split is clean: `is_dir`
  goes to `disk_dirs`, regular files to `disk_files`.
* No regression on the `systacean-2` resolver-universe
  expansion (plain non-markdown files still resolve to
  `missing: false`).
* New regression test pinning that
  `[label](some/dir/)` doesn't produce a file node for
  the dir target.
* Suggested commit subject:

  ```
  Graph: drop directory link targets from ghost emission (systacean-4)
  ```

Push waits for Round-1 close.

## 2026-05-20 — committed (after a soft-reset redo)

Landed as `d35bbd7`:

```
Graph: drop directory link targets from ghost emission (systacean-4)
```

Single-file commit (`crates/chan-server/src/routes/graph.rs`,
+148 / -2 over `systacean-2`'s baseline). Push held for
Round-1 close.

### Changes

* New `drive_disk_dirs(drive) -> BTreeSet<String>` helper
  parallel to `drive_disk_files`; filters `is_dir` instead
  of `!is_dir`. Doc-comment explains its purpose
  (recognising directory link targets so ghost emission
  can skip them).
* `api_graph` computes `let disk_dirs = drive_disk_dirs(&drive);`
  alongside `disk_files`.
* Ghost-set guard at line 1024:
  `if !file_set.contains(e.dst.as_str()) && !disk_dirs.contains(&e.dst)`.
* Edge filter switched from the inline boolean expression
  to a small block returning early on the directory case:

  ```rust
  if matches!(e.kind, EdgeKind::Link) && disk_dirs.contains(&e.dst) {
      return false;
  }
  ```

  Clippy's `nonminimal_bool` / `needless_bool` / `if_same_then_else`
  iterated me toward this shape — it stays readable.
* Two new unit tests:
  * `drive_disk_dirs_includes_directory_entries` — pins the
    helper contract (dirs present; files absent; cross-checks
    the `drive_disk_files` split stays exclusive).
  * `link_to_directory_does_not_synthesize_ghost_file_node` —
    mirror of systacean-2's
    `link_to_non_markdown_disk_file_resolves_to_real_file`:
    seeds `notes/intro.md` with `[[some-dir]]` and a real
    `some-dir/` on disk, asserts ghost-set guard and edge
    filter both drop the directory target.

### Gate

* `cargo fmt --check` — clean (after one auto-fmt pass).
* `cargo clippy -p chan-server --all-targets -- -D warnings` —
  clean (after two iteration passes against
  `nonminimal_bool` / `needless_bool` / `if_same_then_else`).
* `cargo test --all` — green; the `graph::` module went from
  41 → 43 tests (3 new from `systacean-2` + 2 new from this
  task + 38 prior; the +2 vs prior count comes from the
  two new tests added here).
* No web changes; `npm run check` / `npm run build` not in
  scope. Round-1 close will re-run the full pre-push gate.

### End-to-end verification

Rebuilt + restarted `chan serve /tmp/chan-sys2-drv --port 8889`
(same drive as systacean-2's repro). `/api/graph?scope=drive`:

| Metric                            | Pre-fix (`4a04917`) | Post-fix (`d35bbd7`) |
|-----------------------------------|---------------------|----------------------|
| file-kind nodes                   | 891                 | 888 (-3)             |
| file-kind nodes with missing=true | 90                  | 87 (-3)              |
| dir-typed-as-file leaks           | 3                   | **0**                |
| edges                             | 3775                | 3771 (-4)            |

The 3 leaked directory paths (`docs/agents`,
`docs/journals/phase-7/alex`, `docs/journals/phase-8/alex`)
no longer appear in the response. Their 4 inbound link edges
(README + process.md both link `../agents/`; 2 architect
journals link `../alex/`) are dropped by the edge filter, so
Cytoscape never sees a dangling target. The remaining 87
missing-flagged file nodes are all legitimate ghosts (stale
skill-file paths from a doc reorg and other actual broken
links); not in scope.

Server torn down post-verification per the lane-boundary
rule.

### Aside: commit redo

First attempt staged `crates/chan-server/src/routes/graph.rs`
via explicit `git add <path>`, but the resulting commit
(`833c628`) rolled in three other files that were
pre-staged in the shared working tree from concurrent
agents:

* `docs/journals/phase-8/fullstack-b/fullstack-b-10.md`
* `web/src/components/PathPromptModal.test.ts`
* `web/src/components/TerminalRichPrompt.svelte`

`git add <single-path>` does not unstage other index
entries; pre-staged content rides along into the next
commit. Used `git reset --soft HEAD~1` to undo the bad
commit (preserves all changes in the index),
`git restore --staged <file>` to drop the three stowaways
(no working-tree change — they revert to "modified, not
staged"), then re-committed graph.rs only. The bad commit
was local and never pushed.

Lesson: post-commit `git show --stat HEAD` before declaring
done is the cheap audit step. Should pair `git status` +
`git diff --staged --stat` immediately before commit when
operating in a working tree with concurrent agent activity.

## 2026-05-20 — @@Architect: approved + cleared (already committed)

Reviewer: @@Architect.

Clean execution on option A. The before/after metric table
is the gold standard:

| Metric                            | Pre   | Post  |
|-----------------------------------|-------|-------|
| file-kind nodes                   | 891   | 888   |
| file-kind nodes with missing=true | 90    | 87    |
| dir-typed-as-file leaks           | 3     | **0** |
| edges                             | 3775  | 3771  |

The 4 inbound edges to the 3 directory paths drop with the
nodes — Cytoscape never sees a dangling target. 87 remaining
ghosts are all legitimate. Acceptance criterion met exactly.

Two new tests pin the contract:
* `drive_disk_dirs_includes_directory_entries` — helper
  contract + exclusivity check against `drive_disk_files`.
* `link_to_directory_does_not_synthesize_ghost_file_node` —
  end-to-end mirror of -2's
  `link_to_non_markdown_disk_file_resolves_to_real_file`.

Clippy iteration to converge `nonminimal_bool` /
`needless_bool` / `if_same_then_else` shows the gate's
`-D warnings` discipline working as intended — the final
shape stays readable.

### Commit-redo handling

The `git reset --soft HEAD~1` → `git restore --staged
<file>` → re-commit sequence is exactly the right move
for an over-broad commit. Three things on this:

1. **The lesson is correct and worth keeping.** `git show
   --stat HEAD` before declaring done is the cheap audit
   step in a shared working tree.
2. **Pre-commit `git diff --staged --stat`** also catches
   this — if you see paths you didn't touch in the staged
   diff, stop and `git restore --staged` them before
   committing.
3. **No harm done.** `833c628` was local; `d35bbd7` is the
   landed commit. The three stowaways stayed unstaged in
   the working tree and reached their actual owning
   commits cleanly afterward (you can confirm by spot-
   checking — `fullstack-b-10`'s commit will own the
   PathPromptModal.test.ts + TerminalRichPrompt.svelte
   pair; `fullstack-b-10.md` is journal-only).

Single-file commit landed at `d35bbd7`. Push waits for
Round-1 close.

Round-1 close gate items: `-2` ✓ `-4` ✓ `-5` ✓
Makefile fill-in ✓. `systacean-3` (version bump + tag)
unblocks via the commit-grouping plan I'm publishing now;
that's the only thing left in your lane before recycle.