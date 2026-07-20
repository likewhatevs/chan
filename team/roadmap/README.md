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

### v0.74.0

| item | state | what needs to happen |
| --- | --- | --- |
| [distributed-proxy-control-plane](v0.74.0/distributed-proxy-control-plane.md) | steps 1 to 4 implemented, own-gated and reviewed on the unmerged branch `v073/ctl`; steps 5 to 9 unstarted | finish grace and restart reconciliation, fleet capacity, command routing, SSE watches, the client rename, the identity and profile consumption, the three-proxy e2e topology and the packaging wiring, then merge |
| [loopback-redirect-desktop-signin](v0.74.0/loopback-redirect-desktop-signin.md) | **NOT READY TO IMPLEMENT**; designed and security-reviewed, and that review found three exploitable gaps in the naive shape | refine the design against the real call graph, settle the verifier-keyed variant, produce a file-level plan, and only then implement, after the control plane merges |
| [drop-self-built-desktop-packages](v0.74.0/drop-self-built-desktop-packages.md) | blocked behind the loopback redirect; the CLI half already shipped in v0.73.0 | once loopback lands, drop the four Tauri desktop `.deb`/`.rpm` and fix every release-asset consumer in the same commit |
| [aur-aarch64-publication-gate](v0.74.0/aur-aarch64-publication-gate.md) | ran for the first time at v0.73.0 GA and failed at its first step: the ALARM rootfs host presents a certificate that does not cover its own name, so everything below the download is still unproven | make the fetch resilient while keeping the pinned GPG verification as the integrity control, get one green cell, then drop `continue-on-error` and add the job to `aur-publish`'s `needs` in the same edit |
| [aur-publish-verification-race](v0.74.0/aur-publish-verification-race.md) | confirmed defect: the post-push RPC check allows only ~50s, so v0.73.0 published both AUR packages successfully and still reported red | treat `git push` as authoritative and the RPC poll as advisory, widen the budget, and distinguish not-yet-indexed from a real version mismatch |
| [windows-deeplink-second-instance](v0.74.0/windows-deeplink-second-instance.md) | inferred from source, never reproduced; very likely subsumed by the loopback redirect | reproduce it on Windows first, then confirm loopback closes it and decide whether to keep `chan://` registered at all |
| [copr-build-provenance](v0.74.0/copr-build-provenance.md) | COPR rebuilds `main`'s HEAD rather than the released tag, and nothing verifies it published | pin the SCM packages to the tag or forbid pushing to `main` in that window, and add a post-publication build-status probe proven able to go red |
| [terminal-submit-chord-authority](v0.74.0/terminal-submit-chord-authority.md) | confirmed defect: `--submit` is the sender's guess and nothing validates it against the target, so every cross-agent pair whose templates differ silently fails | make the server authoritative over the chord at enqueue, and expose each session's derived agent so a sender never has to guess |
| [control-terminal-wake-rerun](v0.74.0/control-terminal-wake-rerun.md) | confirmed defect: a macOS wake re-dials the control terminal and the server mints a new session running the tenant default, which re-runs the devserver connect script | gate the wake recycle off control terminals, stop a re-dial carrying a session id from adopting the tenant default, and clear on a replaced session |
| [devserver-token-rotation](v0.74.0/devserver-token-rotation.md) | the devserver bearer token is minted once and never rotates, and control-terminal scrollback carrying it is snapshotted into WebView storage | decide a rotation story and exclude control windows from the snapshot |
| [release-asset-verification-coverage](v0.74.0/release-asset-verification-coverage.md) | the release asset verifier does not require the Windows zip or installer, and only ever runs on a real GA | cover every produced asset, ideally from one source, and give the verifier a mode that can be exercised before the tag |

## Completed

### v0.73.0

Shipped 2026-07-20; see [release-v0.73.0](../release/release-v0.73.0.md). Closed items in [`done/`](done/):

- [launcher-flip-pane](done/launcher-flip-pane.md) - the Command Launcher's dead "Flip pane" row works; the overlay stack reconciles at close.
- [terminal-queue-drain-gemini-opencode](done/terminal-queue-drain-gemini-opencode.md) - OpenCode batches its queued terminal notifications; Gemini measured and deliberately kept a boundary.
- [packaging-aarch64-validation](done/packaging-aarch64-validation.md) - delivered in part: the COPR aarch64 evidence is harvested and the item's original premise retired; the AUR gating remainder carries forward.



### v0.72.0

Shipped 2026-07-20; see [release-v0.72.0](../release/release-v0.72.0.md). Closed items in [`done/`](done/):

- [terminal-write-queue-drain](done/terminal-write-queue-drain.md) - queued terminal notifications reconcile in one agent turn, with a reported queue depth.
- [hyperscale-support](done/hyperscale-support.md) - CentOS Stream COPR packaging for `chan` and `chan-desktop`.
- [aur-support](done/aur-support.md) - Arch AUR packaging for `chan` and `chan-desktop`.
- [dump-skill](done/dump-skill.md) - `chan dump-skill` prints an agent-facing manual of chan's whole surface.
- [packaged-desktop-upgrade-refusal](done/packaged-desktop-upgrade-refusal.md) - a distro-packaged build refuses self-upgrade in every personality.

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
