# v0.62.0

The v0.62.0 release report. Theme: polish and cleanup. No new surfaces; the ones we have, done right. Reconstructed from the commit range `26b9ea54..a18c101a` (25 commits, primary) plus the GA bump `efc79614`, the round's `dev/v0.62.0/` coordination tree (secondary: `request.md`, `addendum-1.md`, `addendum-2.md`, `v0.62.0-delivery-plan.md`, `chan-0.62.0.md`), and the release execution itself. Cut GA `v0.62.0` on 2026-07-03.

## The round

Planned from `dev/v0.62.0/request.md` plus two addenda as the polish-and-cleanup round: a nine-agent analysis pass produced `dev/v0.62.0/v0.62.0-delivery-plan.md`, adversarially verified claim-by-claim by six independent reviewers against `main` at `26b9ea54`, then delivered on the `chan-v0620` branch. Per the delivery plan the branch was deliberately left at `0.61.0` pins with the changelog accumulating under `## [Unreleased]`: the cut, bump, and tag are the host's, after the round.

## What shipped

Grouped as the CHANGELOG `[v0.62.0]` section records them:

- **One alert surface.** The OS close button on a live workspace, terminal, or devserver window now prompts Hide / Close / Cancel before acting, instead of hiding and popping an after-the-fact notice; an empty window closes straight away; the notice window is themed (workspace theme, or the launcher theme for standalone terminals) and prints the window name on its own line. The old hidden-window notice machinery is removed.
- **One connecting surface.** The reconnecting overlay reads like the desktop connecting screen, with a live elapsed timer and an "attempt N" counter; on a lost devserver its Close becomes Abandon and terminates the connection. The desktop connecting screen follows the launcher theme.
- **Editor.** The wysiwyg list-typing regression is fixed: `1.`, `-`, `*`, `+`, and task lists all start a list again and Enter continues them; the root cause was a frontmatter parser that collapsed the whole document parse when the first line was an unclosed `---`. On chan-desktop, hyphen and ordered markers render through the same replace widget the `*`/`+` glyphs use, so a new list decorates immediately in WKWebView. A `kind: slides` file auto-opens the Outline on first open.
- **Launcher parity on web and gateway.** A headless devserver's local web launcher is fully usable (real Power toggle and self-managed windows) instead of read-only; the gateway tunnel stays read-only from the same server (a credential-stripped request is refused registry mutation); bridgeless window rows mirror the show/hide EYE state. `chan devserver --service` now defaults to `auto`, resolving the backend per-OS at runtime.
- **Launcher theme drives local standalone terminals.** On chan-desktop, flipping the launcher light/dark toggle retitles every open local standalone terminal live and boots new ones to match, persisted in the desktop config; per-workspace themes stay independent; devserver-attached and remote terminals are unaffected.
- **Session leadership is origin-scoped.** Role is derived from the connection's origin over the tunnel-vs-loopback seam: a local `/ws` reads leader, only a genuinely remote gateway or browser session reads follower, so every local window leads; the role badge shows only when the roster is genuinely split.
- **Smaller refinements.** The inspector kind bubble follows the graph node's extension color; the workspace-root inspector adopts the directory action labels; a `@@mention` in the graph offers "Graph from here"; inactive Excalidraw tabs hide with `display:none` (no zoom/undo leak) and close with Ctrl/Cmd+D; `mermaid-to-excalidraw` fence diagrams render about 1.5x smaller; stuck status-pill errors can be dismissed; workspace-only `cs` commands (`cs session`, `cs graph`, `cs search`, `cs terminal team`) refuse clearly on a standalone terminal instead of a raw socket trace; `cs` workspace commands are gated on standalone terminals; the desktop restores the workspaces that were on after a restart; dismissing a confirm dialog returns focus to the invoking surface.

## The cut

This release shipped on the direct GA path, a deliberate one-time deviation from the rc-pinned cycle now canonical in the release skill: the `chan-v0620` branch was already gated and reviewed during the round, so rather than re-pin through `0.62.0-rc1`, the pins were bumped straight to `0.62.0` and the `publish=false` dispatch stood in as the rc validation.

- Bump `efc79614` moved every version pin `0.61.0` to `0.62.0` across the 13-file set (Cargo.toml workspace plus internal path-dep pins, gateway/Cargo.toml, desktop/src-tauri/tauri.conf.json, the six web package.json versions plus the marketing `@chan/*` dep pins, and the three regenerated lockfiles) and renamed the CHANGELOG `[Unreleased]` section to `[v0.62.0]`.
- Full `make pre-push` gate green across all seven stages (fmt, clippy with `-D warnings`, workspace tests, no-default-features build, gateway workspace build, web-check with 2378 + 178 vitest passing, marketing-check). The pre-push hook is not installed in this clone, so the manual gate was the only gate.
- `publish=false` dispatch of `release.yml` on `chan-v0620` (run 28655757143), derived tag `v0.62.0`: every build and validation job green, including the macOS sign and notarize path (the only off-workstation check for it), with the publish, Pages, and docker-manifest jobs correctly skipped.
- GA: fast-forward `main` to `efc79614`, annotated tag `v0.62.0` on that commit, pushed foreground with an `ls-remote` check, then `chan-v0620` deleted. The tag push ran `release.yml` with `publish=true` (run 28658943475): every job green, the full GitHub Release asset set uploaded (Linux deb/rpm/musl on both arches, the macOS CLI plus the signed DMG and the Tauri updater `.sig`, both AppImages, the Windows zip and installer, and the four gateway service debs on both arches), the chan.app `/dl` metadata regenerated to `latest = 0.62.0`, GitHub Pages deployed, and the Docker Hub `latest` tag moved. GA, prerelease false.

## Carryover

Documentation landed with this release: the git-first release cycle was promoted from `dev/draft-release-cycle.md` into `.agents/skills/release/SKILL.md` as the process source of truth, and this report plus `team/release-v0.61.0.md` backfill the two missing per-release records.
