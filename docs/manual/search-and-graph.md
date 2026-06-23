# Search And Graph Basics

Search and graph views are built from the workspace contents.

## Search

Search indexes files under the workspace root and combines BM25 keyword ranking with embedding (semantic) search, so you can find text across the workspace by wording or by meaning. It is the same index your agents query through MCP. File/path picking for `[[` links is separate from content search.

## Graph

The graph shows filesystem and markdown relationships derived from the workspace. Filesystem nodes come from files and directories. Markdown edges come from links, tags, mentions, and contacts where those features are present. You can export a reproducible `chan://` link to any node and open it straight in the editor.

## Freshness

Chan watches the workspace for changes and updates the search and graph indexes after file edits.
