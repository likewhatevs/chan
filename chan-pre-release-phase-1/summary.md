# Chan Pre-Release Phase 1 Summary

Status: implementation and verification tasks are in REVIEW with all known
release smoke gaps closed. Phase changes are committed in the main repo and
the sibling `chan-core` checkout.

## What Landed

- Removed the pre-v3 contact email backfill consumer from this repo and the
  orphan producer-side helper from the sibling `chan-core` checkout.
- Added `/api/fs-graph` for filesystem graph queries with folder/file scope,
  bounded depth, symlink/hardlink/ghost handling, and escape checks.
- Added CLI parity for `chan status`, `chan graph` content and filesystem
  scopes, and `chan config get|set` over supported editor/server/assistant
  settings.
- Added File Browser `Graph this`, filesystem graph rendering, persisted graph
  mode state, and graph metadata/status display.
- Added Search Status overlay and moved drive-wide index/report status out of
  the file inspector.
- Added report-backed `language:<name>` search and fixed lazy tree hydration so
  language search sees files outside the initially expanded root.
- Fixed SearchPanel keyboard navigation scrolling and assistant chat scroll /
  bubble / thinking-badge behavior in the frontend.
- Hardened watcher handling so symlink, missing, deleted, FIFO, and other
  special-path events do not pin `/api/index/status` to Error.

## Verification

- `cargo test -p chan`: 46 passed.
- `cargo test -p chan-server`: 92 passed.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.
- `scripts/pre-push`: passed, including `cargo test --all-targets` and
  `cargo build --no-default-features`.
- `cd web && npm run check`: clean.
- `cd web && npm test -- --run`: 6 files / 97 tests passed.
- `cd web && npm run build`: passed with existing Vite chunk warnings.
- CDP browser smoke passed at desktop and narrow viewport for Search Status,
  `language:TypeScript`, active-result scrolling, File Browser
  `Graph this`, and assistant active-turn behavior via the isolated fake-Codex
  fixture.
- Syseng live fixture confirmed symlink watcher events return index status to
  Idle.

## Open Release Risks

- `.claude/` is untracked and was left untouched.

## Agent Quality

1. `rustacean`: strongest execution. API surfaces were tested, contracts were
   recorded, and the chan-core cleanup stayed tightly scoped.
2. `webdev`: strong implementation velocity. The filesystem graph and search
   surfaces converged well after the language-search gap was identified.
3. `webtest`: effective at finding a real lazy-loading bug and converting the
   smoke into a repeatable CDP script. The later static/HTTP note briefly
   contradicted the completed interactive smoke and needed architect cleanup.
4. `syseng`: high-value hardening. The watcher-special-path repro exposed a
   release blocker that normal unit tests did not cover.
5. `architect`: kept the work moving across backend, frontend, CLI, and
   hardening boundaries, but should have reconciled the webtest status note
   immediately after the smoke pass.

## Constructive Feedback

- Freeze cross-boundary wire shapes early, as rustacean-2 did for
  `/api/fs-graph`; it prevented frontend guessing.
- Task files should avoid stale "remaining" sections after later successful
  verification. A short superseding note is enough.
- Web smoke tasks should always distinguish "feature implemented", "HTTP
  reachable", and "browser behavior observed"; those are different gates.
- Keep adjacent repo work visible in the journal before committing. The
  chan-core purge is correct, but it changes release coordination.
