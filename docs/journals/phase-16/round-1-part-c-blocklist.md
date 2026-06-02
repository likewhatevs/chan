# Round-1 wave-3: dir blocklist CRUD - CONFIRM-FIRST finding + design

@@LaneC. The poke said "index_excluded_dirs is already per-workspace config" -
CONFIRMED AGAINST SOURCE: it is NOT. It is a GLOBAL (machine-wide) setting.
Flagging before building, since the route shape + the build size depend on
resolving this.

## Mechanism (grounded)

- `index_excluded_dirs` lives on the `Registry` struct, persisted in
  `~/.chan/config.toml` (registry.rs:18-22, 48-51). It is one list shared by
  EVERY workspace on the machine, not per-workspace. There is NO
  excluded-dirs field on the per-workspace `IndexConfig` (config.toml).
- Match semantics: exact directory BASENAME, matched at ANY depth,
  case-insensitive (registry.rs:49). Names, not globs. `.git` + `.chan` are
  ALSO hard-skipped by the walker regardless of the list.
- Default list (registry.rs:23): `.git .hg .svn node_modules target
  __pycache__ .venv venv .tox .pytest_cache .mypy_cache .ruff_cache .cache
  dist build`.
- Applied via one `WalkFilter` built at library open
  (library.rs:110 `WalkFilter::new(registry.index_excluded_dirs)`). The index
  walk, the graph rebuild, and the fs-graph all consult it. (fs_graph.rs
  comments call it "per-workspace blocklist" - that wording is loose; it is
  library-global. In a single-workspace `chan serve` it happens to apply to
  the one served workspace, but it PERSISTS globally.)
- Re-walk-on-change IS feasible: `Library::set_walk_filter(WalkFilter)`
  (library.rs:125) swaps the filter at runtime; a reindex then re-walks
  (tested: `walk_filter_excludes_dir_from_reindex`, library.rs:653).
- No chan-server CRUD route exists today; I would add it.

## The decision (for @@Host via @@Lead) - I'm HOLDING the build on this

@@Host asked for a "per-workspace blocklist", but the mechanism is global.
Three ways forward:

- **A. CRUD the GLOBAL list.** Thin: the field + `set_walk_filter` already
  exist. GET/PUT on `registry.index_excluded_dirs`, re-walk via
  set_walk_filter + reindex. BUT it is machine-wide: editing it in workspace
  A's UI changes the blocklist for ALL workspaces. Contradicts "per-workspace".
- **B. True per-workspace.** Add `excluded_dirs: Vec<String>` to `IndexConfig`
  (per-workspace config.toml); `WalkFilter` for a serve = the per-workspace
  list; CRUD edits it; re-walk that workspace only. Faithful to "per-workspace"
  but a bigger build (new config field + serde default + WalkFilter sourced
  per-workspace + re-walk wiring + tests). Drops the shared global defaults
  unless we also keep them.
- **C. HYBRID (my recommendation).** Keep the global `DEFAULT_INDEX_EXCLUDED_
  DIRS` as a read-only machine baseline (node_modules/target/... - the sane
  defaults everyone wants), and add a per-workspace `excluded_dirs` (ADDITIONS)
  to `IndexConfig`. The effective filter = union(global defaults,
  per-workspace additions). CRUD manages only the per-workspace additions;
  re-walk that workspace. This gives true per-workspace control WITHOUT making
  each workspace re-declare the common defaults, and editing A never touches B.

Recommendation: **C**. It matches @@Host's "per-workspace" intent and keeps the
good defaults. It is more than the thin CRUD the poke implied, so confirm
before I build.

## names-vs-glob (flag for @@Host)

Today it is NAMES ONLY (exact basename, any depth, case-insensitive) - e.g.
`node_modules` skips every `node_modules/` dir. NOT globs: you cannot write
`*.tmp` or `docs/private/**`. (Aside: report.rs already converts the names to
trailing-slash globs internally, but the user model is names.) If @@Host wants
glob/path patterns, that is a `WalkFilter` matcher change (bigger, affects the
index walk + graph + fs-graph + report consistently). Recommend shipping
NAMES-ONLY first (matches today's semantics); globs as a follow-up if wanted.

## Proposed route CONTRACT for @@LaneD (PENDING the A/B/C decision - shape shown for C)

```
GET  /api/index/excluded-dirs
  -> 200 {
       "defaults":  ["node_modules", "target", ...],   // global baseline, read-only
       "workspace": ["fixtures", "vendor"],            // per-workspace additions, editable
       "effective": ["node_modules", "target", ..., "fixtures", "vendor"]  // union, what the walk uses
     }

PUT  /api/index/excluded-dirs
  body { "workspace": ["fixtures", "vendor"] }          // replace the per-workspace set
  -> 200 { same shape as GET, post-change }             // then re-walk this workspace
```

Notes for @@LaneD:
- It is a SET of names. "CRUD" here is really GET + PUT-the-whole-set; add/
  remove happen client-side, then PUT the new set (simpler than per-item
  POST/DELETE, and the re-walk fires once).
- Names are case-insensitive basenames; the UI can lower-case + dedupe before
  PUT. Reject empty strings + path separators (a name, not a path) until/unless
  glob lands.
- If the decision is A (global), drop `defaults`/`workspace` split: GET/PUT a
  single `dirs` list, with a clear "applies to ALL workspaces on this machine"
  label. If B, same single list but per-workspace.

## Hold

Not building until @@Host/@@Lead picks A/B/C and rules on names-vs-glob. The
contract above is a proposal for @@LaneD to react to, not final.
