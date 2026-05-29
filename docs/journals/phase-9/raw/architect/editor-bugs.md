# Editor Bugs

Date: 2026-05-24
Owner: @@Architect
Status: implementation notes for Phase 9

## Horizontal Rule Source

Roadmap item: bare `---` rendered as a horizontal rule in WYSIWYG.

Decision: keep horizontal-rule source text visible in WYSIWYG instead of
painting it as a rendered rule. Users often use `---` as an authoring
separator, and replacing it with a transparent-text rule makes the markdown
harder to edit.

Implementation:

- `HorizontalRule` tokens are now ignored by `chanDecorations`.
- The old `.cm-md-hr` styling was removed from `Wysiwyg.svelte`.
- `blocks.test.ts` now mounts an editor with `one\n---\ntwo` and asserts the
  source remains visible with no `.cm-md-hr` row.

Evidence:

- `npm run test -- src/editor/decorations/blocks.test.ts`

## Codex MCP Check

Roadmap item: Codex failed to start the `chan` MCP server against v0.13.0.

Current HEAD already contains the transport and stale-socket fixes from the
first Phase 9 wave:

- `6d6f9f0 Accept framed MCP stdio transport`
- `4903128 Repair stale MCP proxy startup`

Live probe:

- Started a throwaway current `chan serve` process with isolated `HOME`.
- Verified it published a current `[mcp_servers.chan]` entry to the isolated
  Codex config.
- Sent a Content-Length framed `initialize` request through
  `target/debug/chan __mcp-proxy` using a deliberately stale configured socket.
- The proxy fell back to the live socket and returned a valid MCP initialize
  response.

Observed release behavior:

- The installed v0.13.0 app binary still fails against a stale socket because
  it predates the fallback and framed-transport fixes. This matches the
  roadmap report against v0.13.0, not current HEAD.
