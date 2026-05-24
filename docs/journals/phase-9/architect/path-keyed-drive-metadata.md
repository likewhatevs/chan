# Path-Keyed Drive Metadata

Author: @@Architect
Date: 2026-05-23

## Decision

Phase 9 should remove user-managed drive names from the metadata model.
A drive is identified by its canonical local root path at registration
time, and chan stores per-drive metadata under a filesystem-safe key
derived from that path.

There is no migration requirement for pre-release installs. Existing
`~/.chan` data may be deleted during development and recreated under
the new layout.

## Layout

Global chan metadata stays under:

```text
~/.chan/
```

Per-drive metadata lives under:

```text
~/.chan/drives/<path-key>/
```

Each drive metadata directory contains:

```text
sessions/
trash/
report/
locks/
graph/
index/
drafts/
tokens/
```

`sessions/` and `tokens/` remain bounded LRU stores with 50 entries
each unless a later batch changes that budget.

## Path Key

`<path-key>` is derived from the normalized absolute drive root path.

Example:

```text
/Users/fiorix/dev/github.com/fiorix/chan
=> -Users-fiorix-dev-github.com-fiorix-chan-3f91c2ab
```

Rules:

1. Resolve the input path to the same canonical absolute form used by
   the drive registry.
2. Replace path separators with `-`.
3. Replace any character that is awkward or unsafe in a filename with
   `-`.
4. Collapse repeated `-` runs only if doing so is already a local
   convention.
5. Append a short deterministic hash of the canonical path.

The human-readable prefix keeps the directory debuggable. The hash
prevents collisions from case-insensitive filesystems, unusual path
characters, trailing slash normalization, and future platform-specific
path forms.

## Registry Contract

The registry should store:

```text
root_path       current canonical local drive path
metadata_key    stable key under ~/.chan/drives/
created_at      optional, for diagnostics
last_seen_at    optional, for diagnostics
```

It should not require or persist a user-facing drive name.

Any label shown in CLI, desktop, or web UI should be computed from the
path, usually the basename with a disambiguating compact parent path
when needed.

## Add / Serve Behavior

`chan add <path>`:

1. Canonicalize `<path>`.
2. If a registry entry already exists for that canonical path, reuse it.
3. Otherwise allocate `metadata_key` from the canonical path and create
   `~/.chan/drives/<metadata_key>/`.
4. Register the canonical path and metadata key.

`chan serve <path>`:

1. Canonicalize `<path>`.
2. Reuse an existing registry entry if present.
3. Otherwise create the same entry `chan add` would create.
4. Open the drive with its isolated metadata root.

The `chan` binary should use `~/.chan` on macOS and Linux. It should no
longer use `~/Library/Application Support/chan`.

## Move / Rename Behavior

Because chan is pre-release, there is no old-layout migration.

For future path moves, preserve the metadata directory once allocated.
The registry entry should update `root_path` to the new canonical path
while keeping the existing `metadata_key`.

Reason: moving large metadata directories during a path rename creates
avoidable failure modes. The registry is the source of truth; the
metadata directory name is only a stable storage key.

If a user removes and re-adds a drive after moving it, chan may allocate
a new key. That is acceptable unless the user explicitly asks for
reattachment.

## Ownership Boundaries

`chan-drive`:

- Owns filesystem access to user content under the drive root.
- Accepts an explicit per-drive metadata root from the caller.
- Does not know about global chan config paths beyond what is passed in.

`chan-server`:

- Owns opening one or more registered drives.
- Routes each request to the correct `Drive` and metadata root.
- Keeps edit and terminal operations ahead of background graph/search
  work when resource budgets are tight.

`chan` CLI:

- Owns registry commands and canonical path registration.
- Keeps subcommand help path-based, not name-based.

`chan-desktop`:

- Treats drives as paths.
- Computes display labels from paths.
- Uses the same registry and metadata key contract as the CLI.

## Implementation Batches

Batch 1: registry and path-key allocation

- Make `~/.chan` the canonical config root on macOS and Linux.
- Remove the user-facing name requirement from add/list/serve internals.
- Add deterministic `metadata_key` allocation.
- Add tests for duplicate add, trailing slashes, symlink/canonical path
  behavior, and hash collision disambiguation.

Batch 2: per-drive metadata root plumbing

- Pass each drive's metadata root into graph, search index, report,
  drafts, sessions, tokens, trash, and locks code.
- Ensure no per-drive state writes into global `~/.chan` except the
  registry.
- Add tests with two drives that have identical basenames but different
  roots.

Batch 3: multi-drive server readiness

- Store per-drive server state by metadata key or registry id, not by
  display label.
- Prove edit/search/graph/session state does not cross drive boundaries.
- Add fd/resource-budget tests before increasing multi-drive scope.

Batch 4: UI and desktop cleanup

- Replace drive-name UI assumptions with path-derived labels.
- Keep display labels short, but expose full paths in tooltips or detail
  views.
- Verify default "Chan" drive lifecycle as a path-owned drive, not a
  named metadata entity.

## Test Evidence Required

Before closing this design batch:

- `cargo test -p chan-drive` for metadata-root isolation.
- `cargo test -p chan-server` for route isolation across two drives.
- CLI tests or scripted smoke for `chan add`, `chan list`, and
  `chan serve <path>`.
- A two-drive browser smoke once Browser access is available.
- Manual or automated confirmation that no new path writes use
  `std::fs::*` on user content outside `chan-drive`.

## Open Questions

1. Whether `chan list` should show only paths, or a compact label plus
   full path.
2. Whether `chan remove <path>` should delete per-drive metadata by
   default or require an explicit purge flag.
3. Whether default drive creation in desktop should use
   `~/Documents/Chan`, `~/Chan`, or an app-selected directory.
