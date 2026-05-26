# Phase 10 Summary

Last updated: 2026-05-26.

This file is a short index of notable phase-10 changes and handoffs. Detailed
notes stay in focused files in this directory.

## 2026-05-26

### Stop Global Chan MCP Registration

Detailed note:
`docs/journals/phase-10/mcp-global-registration-removal.md`

Status: implemented and verified.

Summary:

- `chan serve` no longer writes Chan MCP entries into Codex, Claude, or Gemini
  global/user config files.
- Chan terminal `CHAN_MCP_*` environment discovery remains the supported path.
- The old `chan-server` global discovery writer module was deleted.
- Orchestration docs and templates now describe terminal-scoped MCP discovery.

Verification:

- `cargo fmt --check`
- `cargo test -p chan-server`
- `git diff --check` on the touched files

### Desktop In-Process Registry

Detailed note:
`docs/journals/phase-10/desktop-in-process-registry.md`

Status: implemented and verified.

Summary:

- chan-desktop now routes every registry mutation and feature toggle
  in-process through the single embedded `chan_drive::Library`, fixing
  the "drive not registered" failure when opening a brand-new folder.
- The `chan` binary is no longer probed, gated on, or shipped in the
  app bundle; the app is fully self-contained.
- This was a separate Track A change from the MCP registration removal
  above; the two landed together at round close but touch disjoint
  files.

Verification:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo build` + full `cargo test` (workspace, combined tree)
