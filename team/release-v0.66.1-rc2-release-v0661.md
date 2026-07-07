# v0.66.1-rc2 - release-v0661

Second candidate on `release-v0661`, folding in every finding from the release owner's rc1 smoke pass (`dev/v0.66.1/host-smoke.md`): the post-connect control-terminal rule, two new bugs, four small fixes, and three UX additions. Base is the rc1 state (`6f71eaec`, dry-run-validated).

## Scope on top of rc1

- Control-terminal scripts: a post-connect script exit now stops the connection. Clean exit within a 10s registration grace = the daemonize handshake (connection kept, terminal reaped); clean exit past it = full disconnect flow; failing exit = connection stopped, windows closed, terminal kept readable. Fixes the lima/ssh forwarded-^C case that left a connection registered with no control terminal.
- Terminal surveys queue per target tab through a bounded server-side FIFO (cap 100, explicit queue-full error); a second survey no longer clobbers the first overlay and starves its caller.
- Slides: `POST /api/drafts/new` accepts `{"kind":"slides"}` and seeds the canonical slides frontmatter; a "New slide deck" Apps command uses it.
- Editor: failed excalidraw embeds are clickable-to-edit; rendered diagrams (mermaid, mermaid-to-excalidraw, inline excalidraw) gain a Copy-as-PNG action; the Delete row drops its misleading Backspace hint.
- Launcher: Enter on a no-match query is a no-op; ArrowUp-from-top wrap preserved.
- The pane focus-colour watch subscribes only on desktop surfaces (no more standalone-server 404 churn).
- Empty pane: workspace-path label removed (retires the rc1 pill), the chan mark hides on short panes (container query), and a floating Apps button menus every app surface with assigned chords.

## Branch And Commits

- Base: `6f71eaec` (rc1 tip).
- Commits:
  - `8b59e181` `feat(server): queue terminal surveys per target, seed slide decks`
  - `8298f328` `fix(desktop): stop the devserver connection when its script exits`
  - `d1da3711` `feat(web): rc2 editor, launcher, and empty-pane fixes`
  - `44634cd4` `docs(release): update v0.66.1 unreleased notes for rc2`
  - rc2 pin commit (this commit's parent set) + this report.

## Validation

- Server lane: clippy + `cargo test -p chan-server -p chan-shell` (553 + 85 passed, 14 new tests incl. queue ordering, queued-timeout, queue-full, slides seed) + scoped fmt.
- Desktop lane: clippy + `cargo test -p chan-desktop` (173 passed; the source-text semantics test rewritten to pin the grace split and the windows-close-before-mark compose, which the lane verified is the only non-leaking order) + scoped fmt.
- Web lane: svelte-check (0 errors) + full workspace-app vitest (266 files, 2536 tests) after completing the lane across two agents (the first died mid-run on an API error; the second salvaged and finished). Adversarial reviews on all three lanes; the one user-visible catch (launcher ArrowUp-from-top dead stop) was fixed and pinned before commit.
- Full `make pre-push` green on the integrated branch at `44634cd4`.
- Headless-Chrome browser verification of the web items: recorded in `dev/v0.66.1/browser-smoke/` (rc2 pass pending at report-writing time; result noted in the smoke doc).

## Release Workflow

- Run 1 (rc1): `28889761776`, FAILURE (macOS SUN_LEN, fixed in rc1 by `d7085700`).
- Run 2 (rc1): `28891608356`, SUCCESS.
- Run 3 (rc2): dispatched on the rc2 pin head with `publish=false`; id and result recorded when concluded.

No `v*` tag is pushed for an rc.

## Known Risks And Notes

- Grace-window heuristics (both accepted): a clean ^C within 10s of a connection (re)registration reads as the daemonize handshake and keeps the connection; a poll-recovery re-set also re-stamps the grace. A very slow handshake (>10s from registration to script exit observation) would full-teardown a healthy connect; the constant is fn-local and tunable.
- Survey queueing keys on the resolved target; two group surveys with overlapping window sets are not serialized against each other (accepted; the reported per-tab repro is covered).
- Old-CLI-new-server skew on the queue_full control response decodes as an error (ships together in the release; accepted).
- The desktop clipboard-image IPC is PNG-only; SVG-on-clipboard exists only as the optional web-path ride-along, which was not taken this round.
- The colour-watch gate uses the desktop capability check: a hand-opened browser tab against a headless devserver loses live colour push (not a minted surface; accepted).
