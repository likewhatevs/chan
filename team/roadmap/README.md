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

### v0.71.0

- [release-flow](v0.71.0/release-flow.md) - make `team/` the development front door, split into this roadmap tree and the release-history tree, and land it as the barrier before any technical v0.71.0 work. This item is what the first v0.71.0 commit executes.
- [chan-upgrade-release-history-fix](v0.71.0/chan-upgrade-release-history-fix.md) - keep the last five CLI upgrade versions resolvable through `chan upgrade --version`.
- [chan-workspace-graph-fix](v0.71.0/chan-workspace-graph-fix.md) - unify workspace search and graph traversal behind one bounded contract used by `cs`, `chan workspace`, chan-server, and `chan-llm`.
- [terminal-write-queue-drain](v0.71.0/terminal-write-queue-drain.md) - drain queued terminal notifications within one agent turn.
- [tauri-permission](v0.71.0/tauri-permission.md) - authenticated exact-origin Tauri permissions.
- [terminal-gemini-opencode](v0.71.0/terminal-gemini-opencode.md) - Gemini verification and first-class OpenCode terminal support (implementation in flight on its feature branch).
- [cosmetics](v0.71.0/cosmetics.md) - editor light-mode codeblock box and dark-mode selection-highlight color fixes.

## Completed

No version has closed under this tree yet; v0.71.0 is the first release to adopt it.

## See also

- [`../README.md`](../README.md) - how chan is developed: proposing, teaming, and shipping an item.
- [`../release/README.md`](../release/README.md) - the release history and its conventions.
- [`../../.agents/skills/release/SKILL.md`](../../.agents/skills/release/SKILL.md) - the executable release procedure.
- [`../../.agents/playbook.md`](../../.agents/playbook.md) - operational lessons distilled across the project.
