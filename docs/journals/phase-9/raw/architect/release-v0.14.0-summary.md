# Phase 9 Architect Release Summary

Date: 2026-05-24

Release tag: `chan-v0.14.0`

Release commit: `083b3e4 release: prepare v0.14.0`

## Release State

- `main` pushed to `origin/main`.
- `chan-v0.14.0` pushed and points at `083b3e4`.
- CI on `main` passed:
  https://github.com/fiorix/chan/actions/runs/26369613827
- Desktop release passed:
  https://github.com/fiorix/chan/actions/runs/26369617703
- CLI release passed:
  https://github.com/fiorix/chan/actions/runs/26369617685

## Shipped

- Rich Prompt workspaces with Core-owned workspace and spool creation,
  active markers, session-aware status, exact-buffer submit archival,
  and Core-owned close cleanup.
- Rich Prompt web workflow for draft editing, preflight confirmation,
  history, status, close/discard, and Hybrid Nav entry points.
- Editor page breaks and PDF export.
- Metadata archive export/import core, CLI import, and Infographics UI.
- Drafts lifecycle cleanup: close, discard, no-clobber creation, boot
  warnings, hidden inactive internals, and Drafts-aware graph/index flow.
- Server state and route hardening around lock poisoning, blocking work,
  fd pressure, stale indexers, and watcher noise.

## Verification

- `./scripts/pre-push`
  - First sandboxed run hit macOS sandbox `Operation not permitted` in
    temp/socket path tests.
  - Rerun outside the sandbox passed.
- `npm run test` passed after raising Vitest `testTimeout` to 30s for
  the full Svelte component suite.
- `npm run build` passed.
- `git diff --check` passed.

## Release-Time Fixes

- Removed clippy's `if_same_then_else` warning in
  `crates/chan-server/src/indexer.rs`.
- Removed clippy's `needless_return` warning in `crates/chan/src/main.rs`.
- Raised `web/vite.config.ts` test timeout from Vitest default 15s to 30s
  because the full component suite can exceed 15s while isolated specs pass.

## Parking Lot

- Native desktop literal `Cmd+P` validation remains skipped for this
  release. Alex will report if it fails in the new `Chan.app`.
- Rich Prompt visual polish is filed for the next round.
- Low-FD live stress is filed for the next round.
- No extra mtime CAS release code was needed in this session.
