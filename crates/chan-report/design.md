# chan-report design

Canonical design reference for `chan-report`. Update in the same
commit as any change that affects the public API shape, the
on-disk JSONL schema, the COCOMO model defaults, or the rules
the walker applies.

## 1. Problem and scope

`chan-report` produces a structured report about the contents of
a directory tree: per-file language, SLOC, comments, blanks, a
keyword-based complexity score, plus per-language roll-ups and a
Basic COCOMO summary computed from the totals. It is the data
backend behind a future "what's in this workspace?" view in chan and
a future `repo_report` assistant tool.

In scope:

  - Walk a directory with gitignore-aware filtering.
  - Identify language per file using `tokei`'s detector
    (extension + shebang + a small set of heuristics).
  - Count code / comments / blanks per file.
  - Compute a cheap, language-aware complexity score
    (keyword counts: `if`, `for`, `while`, `case`, ...).
  - Roll up totals per language and across the whole tree.
  - Compute Basic COCOMO (Organic / Semi-Detached / Embedded).
  - Maintain an in-memory `Index` that supports per-file
    updates, removals, renames, and scoped snapshots.
  - Serialize / deserialize the index to a stable JSONL format.

Out of scope:

  - Persistence. The crate never writes to disk. The consumer
    (chan-workspace) atomically writes the JSONL the crate emits.
  - Filesystem watching. The consumer wires its own watcher to
    `Index::update` / `Index::remove` / `Index::rename`.
  - Cyclomatic or AST-level complexity. The complexity field is
    a keyword count, documented as such.
  - Git metadata (commits, blame, ranges). Walker respects
    `.gitignore` but never inspects git history.
  - Multi-root or cross-workspace aggregation. One root per `Index`.

## 2. Architecture overview

```
       +---------------------+
       |     Index           |    HashMap<rel_path, FileStats>
       |                     |    + cached language totals
       +-----+--------+------+
             |        |
       scan  |        | update / remove / rename
             v        v
       +---------------+      +-------------------+
       |   walk        |----->|  count_file       |
       | (ignore crate)|      | (tokei per-file)  |
       +---------------+      +-------------------+
                                       |
                                       v
                              +-------------------+
                              |  complexity       |
                              | (keyword scoring) |
                              +-------------------+

   snapshot(Scope, CocomoParams)
             |
             v
       +----------+    +-----------+    +---------+
       |  filter  +--->|  roll-ups +--->|  COCOMO |
       |  by scope|    |  per-lang |    |         |
       +----------+    +-----------+    +---------+
                              |
                              v
                          Report  -->  write_jsonl
```

  - `Index` is the state. All mutating operations are
    O(1 file) so a watcher can call them on every event.
  - `Report` is a pure value type computed from `Index` plus a
    `Scope` and `CocomoParams`. Snapshots never mutate state.
  - The walker is only used during `Index::scan`. Incremental
    updates take a relative path from the caller and skip the
    walker entirely.

## 3. Public API

Headline types in `lib.rs`:

  - `ReportOptions` configures the initial walk: root,
    follow_symlinks, include_hidden, respect_gitignore,
    exclude_globs, cocomo.
  - `Scope` selects what a snapshot covers: `All`,
    `Prefix(String)`, or `Files(Vec<String>)`. Paths are
    workspace-relative POSIX strings.
  - `Index` holds the per-file state plus the cached
    accept-filter (so incremental updates apply the same
    hidden / gitignore / exclude rules as the initial scan).
    Public methods: `scan(opts)`, `update(rel)`, `remove(rel)`,
    `rename(from, to)`, `file(rel)`, `len`, `is_empty`,
    `snapshot(scope, cocomo_params)`, `write_jsonl`,
    `load_jsonl(reader, opts)`.
  - `UpdateOutcome` (Inserted / Updated / Unchanged / Removed /
    Skipped) lets the watcher coalesce no-op writes.
  - `Report`, `ReportMeta`, `Totals`, `LanguageStats`,
    `FileStats`, `CocomoSummary` are plain serde structs with
    primitive fields. No lifetimes on public types; FFI-shaped.
  - `ChanReportError` is the single umbrella error enum, with
    primitive (string) payloads only.

`run(opts)` is a one-shot helper equivalent to
`Index::scan(opts)?.snapshot(&Scope::All, &opts.cocomo)`.

`count_file(root, rel)` is exposed so chan-workspace (or tests) can
re-count without going through `Index`.

### Subdirectory and per-file queries

`Scope::Prefix("crates/chan-workspace")` rolls up every file under
that prefix; `Scope::Files(...)` rolls up an explicit list. Both
go through the same `snapshot` path so the same `Report`
structure is returned regardless of scope, and the
`by_language` / `totals` / `cocomo` fields reflect only the
scoped subset. `Index::file(rel)` returns the raw `FileStats`
for one file with no roll-up cost.

## 4. JSONL on-disk format

One record per line. `kind` is the discriminator. Records may
appear in any order in a single file; consumers index by
`kind` + `path` / `name`. Empty lines and lines beginning with
`#` are ignored on load.

```
{"kind":"meta","schema":1,"root":"/abs/path","generated_at":"2026-05-12T12:00:00Z"}
{"kind":"file","path":"src/lib.rs","language":"Rust","code":812,
 "comments":64,"blanks":92,"complexity":47,"bytes":41203,
 "mtime":"2026-05-10T09:11:02Z"}
{"kind":"language","name":"Rust","files":210,"code":53120,
 "comments":4012,"blanks":6804,"complexity":2810}
{"kind":"totals","files":812,"code":91234,"comments":7321,
 "blanks":12044,"complexity":4521}
{"kind":"cocomo","model":"basic-organic","effort_person_months":23.4,
 "schedule_months":8.1,"developers":2.9,"estimated_cost_usd":312450.0}
```

Rules:

  - Schema is integer-versioned. `load_jsonl` rejects files with
    a `meta.schema` that does not match the current build with
    `ChanReportError::SchemaMismatch`. Consumers (chan-workspace)
    treat that as "discard cache, rescan".
  - Path encoding is POSIX, workspace-relative, no leading slash, no
    `..`, no embedded `\`. Loader rejects malformed paths.
  - Timestamps are RFC3339 with `Z` suffix.
  - `mtime` is optional on `file` records. When absent, the
    consumer can treat the row as "valid but freshness
    unknown". Writers always populate it when the source mtime
    is readable.
  - Numeric fields are unsigned where possible. Floats appear
    only in `cocomo` records.

The JSONL `file` records alone are sufficient to reconstruct the
index; the `language` / `totals` / `cocomo` records are
materialized roll-ups for consumers that only need the
overview. `load_jsonl` recomputes those from the file records and
ignores any persisted roll-ups (treats them as advisory).

## 5. Incremental model

`Index::update(rel)`:

  - Applies the cached `Filter` (hidden / gitignore /
    exclude_globs) to `rel`. If rejected and a row exists,
    drops the row and returns `Removed`; otherwise `Skipped`.
  - Calls `count_file` against the stored root. If the file
    vanished or the counter rejected it, removes any existing
    row (`Removed`) or returns `Skipped`.
  - Compares the new `FileStats` against the existing row.
    Returns `Unchanged` when byte-identical; otherwise inserts
    or updates and returns `Inserted` / `Updated`.

`Index::remove(rel)` is unconditional: drops the row if present
and returns `Removed`, else `Unchanged`.

`Index::rename(from, to)` is `remove(from)` plus `update(to)` in
one call. When the source row existed but the destination
update is `Unchanged` / `Skipped`, the call upgrades the result
to `Removed` so the consumer's debouncer flushes.

`Unchanged` is the signal that lets the consumer skip writing a
fresh JSONL. chan-workspace will debounce a burst of updates and
only call `write_jsonl` + `atomic_write` when any outcome was
non-`Unchanged`.

## 6. Walker rules

The crate uses the `ignore` crate (same engine as ripgrep) with
these settings derived from `ReportOptions`:

  - `respect_gitignore`: when true, honors `.gitignore`,
    `.ignore`, and `.git/info/exclude`. Default true.
  - `include_hidden`: when false, skips dotfiles and dot-
    directories. Default false.
  - `follow_symlinks`: when false, symlinks are listed but not
    descended. Default false.
  - `exclude_globs`: extra patterns OR'd on top of the gitignore
    rules. Use to drop `target/`, `node_modules/`, vendored
    directories without committing to the project's
    `.gitignore`. chan-workspace will pass `.chan/` and `.git/`
    here even though they're typically gitignored, defending
    against repos that vendor them.

The walker emits relative POSIX paths to the counter. Anything
the counter can't classify (no recognized language) is dropped
silently; binary files identified by tokei are also dropped.

## 7. Complexity score

Per-file keyword count over a small, language-aware list. Cheap,
deterministic, and documented as a heuristic. We deliberately
do not implement cyclomatic complexity: the AST work is not
worth it for a roll-up. The score is comparable within a
language and roughly comparable across closely-related
languages, but should never be treated as a defect signal.

The default keyword list mirrors scc's: `if`, `else`, `elsif`,
`elif`, `for`, `while`, `switch`, `case`, `match`, `do`, `goto`,
`continue`, `break`, `try`, `catch`, `except`, `&&`, `||`,
`and`, `or`. Per-language overrides live in `complexity.rs`.

## 8. COCOMO

Basic COCOMO computed from total SLOC (sum of `code` across all
included files). Three modes:

```
    a       b     c     d
  ----- ----- ----- -----
   2.4  1.05   2.5  0.38   Organic        (default)
   3.0  1.12   2.5  0.35   Semi-Detached
   3.6  1.20   2.5  0.32   Embedded

  effort_pm   = a * (KSLOC ^ b)
  schedule    = c * (effort_pm ^ d)
  developers  = effort_pm / schedule
  cost_usd    = effort_pm * avg_monthly_salary_usd
                          * overhead_multiplier
```

`CocomoParams` defaults:

  - `model = Organic`
  - `avg_monthly_salary_usd = 8000.0`
  - `overhead_multiplier = 2.4`

These are documented in the same place they're read so users can
override them per call without recompiling.

## 9. Tests

Test data lives in `tests/fixtures/` as tiny mixed-language
trees. Test categories:

  - Walker: hidden files, gitignore, exclude_globs, symlinks.
  - Counter: known-good per-file stats vs. fixture.
  - Incremental: `scan -> mutate file -> update -> snapshot`
    yields the same `Report` as a fresh `scan`.
  - JSONL: write + load + snapshot round-trips to the same
    `Report`. Schema-mismatch returns `SchemaMismatch`.
  - Scope: `Prefix` and `Files` produce roll-ups consistent
    with the per-file rows they include.

## 10. Out of scope, explicitly

  - File watching. chan-workspace owns the watcher.
  - Atomic write to disk. chan-workspace owns persistence.
  - Cross-process locking. The `Index` is `Send` but not
    `Sync`; chan-workspace serializes writers behind its own lock.
  - Threading model. `Index::scan` may parallelize internally;
    `update` is single-threaded.
  - i18n / non-UTF-8 paths. Walker rejects non-UTF-8 entries
    with `ChanReportError::InvalidUtf8Path`.
