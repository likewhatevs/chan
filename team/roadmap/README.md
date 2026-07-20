# Roadmap

Active development scope for chan, organized by the release it targets. This is the roadmap front door: what has been accepted as work for an upcoming version, and where each item goes once it ships, is withdrawn, or slips. It is not a second release report; closed history lives in [`../release/`](../release/README.md), and the process that moves an idea from a problem to a shipped release is described in [`../README.md`](../README.md).

Each item is one Markdown file that names an observed behavior or need, the evidence for it, the desired contract, its implementation boundaries, and its acceptance checks. An item earns a place here only once it is accepted scope for a concrete target version; a raw draft in the gitignored `dev/` tree is not a roadmap item until it is copied in and accepted.

## Lifecycle

1. `vX.Y.Z/{item}.md` is accepted active scope for that target version.
2. Implementation and validation evidence accumulate in the proposal, its candidate report, or the round's artifacts, without replacing the proposal's original rationale.
3. At GA the item moves to `done/{item}.md` and gains a status line linking to `[vX.Y.Z](../release/release-vX.Y.Z.md)`; the text says `shipped` only when the item actually shipped.
4. A withdrawn item also moves to `done/`, states plainly that it did not ship, and links to the release report that records the decision.
5. A deferred item moves to the next active version directory before GA. It is not marked done.
6. After the GA close commit the released version's directory is gone; every one of its items lives in `done/` or in a later active version.

## Layout rules

`done/` is intentionally flat, so item filenames must stay descriptive and repository-wide unique. If a future item would collide with a closed one, prefix that filename with its version when it is closed.

## Active

### v0.72.0

| item | state | what needs to happen |
| --- | --- | --- |
| [hyperscale-support](v0.72.0/hyperscale-support.md) | validated locally on x86_64; COPR builds pending | submit the configured matrix, accept the native x86_64 and aarch64 builds (aarch64 is unproven anywhere), and verify that no EL9 desktop job is scheduled |
| [aur-support](v0.72.0/aur-support.md) | implemented; release validation pending | dry-run the credential probe before the tag, hand-smoke CachyOS, confirm both AUR repositories at `0.72.0-1`, and prove the unverified aarch64 leg |
| [terminal-write-queue-drain](v0.72.0/terminal-write-queue-drain.md) | implemented and unit-tested; batching proven live for Codex and Claude | every case of `scripts/e2e/terminal-queue-drain.sh` passed 3/3 for both agents and the rows are in the item's Live Matrix Results; Gemini and OpenCode stay single-message until a host with those CLIs runs the same cases |
| [dump-skill](v0.72.0/dump-skill.md) | implemented | merged; the review findings in the item's Known Gaps are closed |
| [packaged-desktop-upgrade-refusal](v0.72.0/packaged-desktop-upgrade-refusal.md) | implemented | merged; a packaged build refuses `chan upgrade` and `chan upgrade --check` in every personality |

## Completed

### v0.71.0

Shipped 2026-07-19; see [release-v0.71.0](../release/release-v0.71.0.md). Closed items in [`done/`](done/):

- [terminal-gemini-opencode](done/terminal-gemini-opencode.md) - OpenCode as a first-class terminal agent.
- [tauri-permission](done/tauri-permission.md) - authenticated exact-origin desktop native trust.
- [chan-workspace-graph-fix](done/chan-workspace-graph-fix.md) - unified workspace search and graph traversal.
- [chan-upgrade-release-history-fix](done/chan-upgrade-release-history-fix.md) - `chan upgrade --version` resolves the last five GA releases.
- [cosmetics](done/cosmetics.md) - editor light-codeblock and dark-selection fixes.
- [release-flow](done/release-flow.md) - the team/roadmap + team/release process migration.

## See also

- [`../README.md`](../README.md) - how chan is developed: proposing, teaming, and shipping an item.
- [`../release/README.md`](../release/README.md) - the release history and its conventions.
- [`../../.agents/skills/release/SKILL.md`](../../.agents/skills/release/SKILL.md) - the executable release procedure.
- [`../../.agents/playbook.md`](../../.agents/playbook.md) - operational lessons distilled across the project.
