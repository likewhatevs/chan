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

## Web Export Slice

The web app now exposes metadata archive export from Infographics settings.
The server route is settings-gated at `POST /api/metadata/export`, builds the
archive through `chan_drive::Library::export_metadata_archive`, and returns a
download with file and byte counts in response headers.

The follow-up web import slice adds `POST /api/metadata/import` as a
settings-gated multipart route. It closes terminal sessions and loaded team
watchers, drops the live watcher and indexer, waits for outstanding `Drive`
clones to drain, runs `Library::import_metadata_archive`, and then reopens the
drive cell with a fresh watcher and indexer. This mirrors the storage reset
swap pattern so imported metadata is not replaced under active handles.

Infographics now stages a selected `.tar.zst` archive behind an explicit
Import action, defaults to rescan, exposes the SCM-force escape hatch, and
reloads the app after successful import so restored session and draft metadata
become authoritative.

Additional evidence:

- `cargo test -p chan-server routes::metadata`
- `npm run test -- --run src/components/infographicsTabAndCarousel.test.ts
  src/api/metadataArchiveClient.test.ts`
- `npm run check`

Build follow-up:

- WebtestLive caught that the new runtime routes used `tempfile` while
  `chan-server` declared it only as a dev-dependency. Moved `tempfile` into
  runtime dependencies and verified `cargo build -p chan`.

Live import follow-up:

- WebtestLive found that importing an exported archive returned `drive busy`
  even after closing the app tab. Root cause: `AppArtifacts` held a second
  `Arc<Indexer>` for shutdown cancellation and `Indexer` did not abort its
  spawned tasks on drop, so old indexer tasks could keep the old `Drive` Arc
  alive after the metadata import route removed the drive cell.
- Fixed by making shutdown cancellation read the current drive cell instead
  of retaining a stale indexer handle, and by giving `Indexer` an explicit
  `Drop` that sets cancellation and aborts its task handles.
- Live API smoke exported metadata from a throwaway drive, imported the same
  archive through `POST /api/metadata/import`, and received a 200 response with
  `rescanned:true`.

Additional evidence:

- `cargo test -p chan-server`
  `indexer::tests::dropping_indexer_releases_drive_handle`
- `cargo test -p chan-server routes::metadata`
- `cargo test -p chan-server --lib`
- `cargo build -p chan`
- `cargo build -p chan --no-default-features`
