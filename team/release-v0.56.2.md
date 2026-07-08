# v0.56.2: editor list rendering and workspace lifecycle

Cut from `main` after `v0.56.1`. This is a patch for Markdown editor list rendering and workspace lifecycle correctness, the round that `release-v0.56.3.md` then follows with its own list-alignment pass.

## Theme

Make Markdown list markers line up with prose, and make workspace lifecycle state owner-side and typed so launcher rows reflect what the serving owner is actually doing rather than optimistic frontend state.

## Editor lists

- The misaligned list guide bars are removed entirely: WYSIWYG and source list rendering no longer emit list-guide decorations or the CSS hooks behind them, so there are no vertical bars to misalign.
- First-level list text now aligns with normal prose: bullet, ordered, and task-list markers hang left while the item text starts at the same margin as paragraph text.

## Workspace lifecycle

- Workspace lifecycle state is owner-side and typed. Local desktop and devserver workspaces surface `starting`, `closing`, `removing`, `running`, `stopped`, and `error` from the serving owner (the chan-desktop embedded host for local workspaces, the devserver for remote ones), so a launcher reload keeps the correct row state.
- Launcher rows lock during owner transitions: power and remove controls spin and stay disabled through `starting`, `closing`, and `removing`, and devserver rows also preserve the backend `connecting` state across reloads.
- Close and remove refusal is consistent across the local, devserver, CLI, desktop-handoff, and control-socket paths: all return the shared `{"error":"live_terminals","active_terminals":N}` body and leave live workspaces running and visible until forced.
- Server-hidden devserver windows reopen from launcher rows: the desktop resolves bare window ids against the connected devserver feed before falling back to local labels.

## Validation

- Targeted `chan-library` host-lifecycle tests, then `chan-server` library and devserver route tests, then launcher status-rendering and bulk-remove-order tests.
- The existing open/close and devserver integration tests.
- The full project pre-push gate.

## Release

- GA bumps all release pins to `0.56.2`, updates the changelog and this release report, then tags `v0.56.2`. No rc was cut.
