# Core Follow-Up Task: Rich Prompt Watcher Warnings

Date: 2026-05-24
Owner: Core Architect
Status: Closed

## Task

Classify and, if needed, fix the repeated watcher rebuild warnings seen during
Rich Prompt browser validation:

```text
chan_server::indexer: watcher event stream lost scope; requesting rebuild
```

The validation agent reported the warning for path-less event and path-less
rename activity while Rich Prompt draft/workspace operations were running.

## Repro Context

Validation ran from worktree `b0869b1` with uncommitted Phase 9 changes:

```bash
npm run build
cargo build -p chan
mkdir -p /tmp/chan-iab-home /tmp/chan-iab-drive
HOME=/tmp/chan-iab-home ./target/debug/chan serve --no-browser /tmp/chan-iab-drive
```

Then in the in-app browser:

- Open app at the printed bearer URL.
- Use Mod+. p to create a Rich Prompt terminal.
- Let the workspace create and watcher attach.
- Open plus menu, reload, submit blank prompt, and close the terminal.

The warning appeared during Rich Prompt draft/workspace activity. No current-run
browser console errors were observed.

## Expected Core Decision

Pick one and document the result:

- Warning is expected noise for Rich Prompt metadata/spool activity. Downgrade
  or suppress it narrowly so normal Rich Prompt use does not look broken.
- Warning indicates missing event scope/path propagation. Fix scope derivation
  so Rich Prompt workspace events do not force broad rebuilds.
- Warning is a real broad-rebuild signal and should stay visible. Document why
  it is acceptable during Rich Prompt activity.

## Files To Inspect

- `crates/chan-server/src/event_watcher.rs`
- `crates/chan-server/src/indexer.rs`
- `crates/chan-server/src/routes/rich_prompts.rs`
- `crates/chan-server/src/terminal_sessions.rs`
- `crates/chan-drive/src/rich_prompts.rs`

## Acceptance Criteria

- The warning is either removed from normal Rich Prompt activity or explicitly
  documented as expected.
- If code changes are made, add a focused regression test around the
  path-less event or rename case.
- Do not change Web shortcut semantics. Browser validation should use Cmd+Alt+P
  or Mod+. p; native/desktop may validate literal Cmd+P.

## Resolution

Core classified path-less non-provider watcher events as macOS notify noise for
normal metadata activity. The indexer now ignores path-less create, modify,
remove, and rename events instead of treating them as lost scope. Provider
errors and broadcast lag still request a full rebuild.

Regression coverage:

- `indexer::tests::classify_watch_event_ignores_pathless_non_provider_noise`
- `indexer::tests::classify_watch_event_requests_rebuild_on_provider_loss`

Verification:

- `cargo test -p chan-server indexer::tests::classify_watch_event --lib`
- `cargo test -p chan-server --lib`
- `cargo fmt --check`
- `git diff --check`
