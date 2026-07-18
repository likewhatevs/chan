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

Scope descriptions are in each linked proposal. State and immediate action:

| item | state | what needs to happen |
| --- | --- | --- |
| [release-flow](v0.71.0/release-flow.md) | landed on `main` (39db4e6b) | move to `done/` at GA |
| [terminal-gemini-opencode](v0.71.0/terminal-gemini-opencode.md) | implementation in flight | rebase `feature/opencode-terminal-support` onto `main`, gate, merge as the first intake |
| [chan-workspace-graph-fix](v0.71.0/chan-workspace-graph-fix.md) | proposed | assign a lane, implement, validate |
| [chan-upgrade-release-history-fix](v0.71.0/chan-upgrade-release-history-fix.md) | proposed | assign a lane, implement, validate |
| [terminal-write-queue-drain](v0.71.0/terminal-write-queue-drain.md) | proposed | assign a lane, implement, validate |
| [tauri-permission](v0.71.0/tauri-permission.md) | proposed | assign the desktop lane, implement, validate |
| [cosmetics](v0.71.0/cosmetics.md) | proposed | assign the web/editor lane, implement, validate |

**Next steps**

1. Intake the opencode work first: rebase `feature/opencode-terminal-support` onto `main`, run the gate, and merge it as the first accepted candidate. It carries `terminal-gemini-opencode.md`, so drop that file's stale `dev/` copy at the rebase.
2. Prepare the delivery team with `cs terminal team`: assign the remaining proposed items to file-disjoint lanes by surface, with a dependency graph and a per-item validation matrix.
3. Open `0.71.0-rc1`: bump every version pin in one commit, dispatch the `publish=false` dry run, validate the artifacts, and iterate.
4. GA close in one commit: write `team/release/release-v0.71.0.md`, index it, move every v0.71.0 item to `done/` with an honest status line, carry the CHANGELOG and pins, and tag `v0.71.0`.

## Completed

No version has closed under this tree yet; v0.71.0 is the first release to adopt it.

## See also

- [`../README.md`](../README.md) - how chan is developed: proposing, teaming, and shipping an item.
- [`../release/README.md`](../release/README.md) - the release history and its conventions.
- [`../../.agents/skills/release/SKILL.md`](../../.agents/skills/release/SKILL.md) - the executable release procedure.
- [`../../.agents/playbook.md`](../../.agents/playbook.md) - operational lessons distilled across the project.
