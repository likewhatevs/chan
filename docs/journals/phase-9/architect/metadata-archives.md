# Metadata Archives

Date: 2026-05-24
Owner: @@Architect
Status: implementation note for Phase 9

## Scope

Phase 9 carries the remaining Round 3 Track 4 work: export and import chan
metadata without copying user drive content. The archive is a manifest-first
`.tar.zst` containing approved metadata subtrees only.

## Implementation

The export path already wrote a manifest-first archive with `index`, `graph`,
`report`, `sessions`, and `drafts` payload subtrees while excluding locks,
tokens, trash, staging, temp files, and sqlite shared-memory files.

The import path now validates every archive entry before extraction, rejects
symlinks, hardlinks, special files, absolute paths, parent components, and
Windows prefixes, extracts into metadata staging, then replaces only the
approved subtrees. `chan metadata import --rescan` opens the drive after import
and rebuilds the index and graph.

The SCM guard compares the archive manifest identity with the target drive.
Remote mismatch blocks import unless the caller passes `--force-scm`.

## CLI

- `chan metadata export <drive-path> <archive.tar.zst>`
- `chan metadata import <drive-path> <archive.tar.zst> [--rescan] [--force-scm]`
- `chan metadata inspect <archive.tar.zst> [--json]`

## Tests

- `cargo test -p chan-drive metadata_archive`
- `cargo test -p chan metadata_subcommands_parse`
