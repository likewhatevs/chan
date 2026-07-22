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

### v0.75.0

| item | state | what needs to happen |
| --- | --- | --- |
| [loopback-redirect-desktop-signin](v0.75.0/loopback-redirect-desktop-signin.md) | **NOT READY TO IMPLEMENT**; designed and security-reviewed, and that review found three exploitable gaps in the naive shape | refine the design against the real call graph, settle the verifier-keyed variant, and produce a file-level plan before implementing; the control-plane dependency cleared in v0.74.0 |
| [drop-self-built-desktop-packages](v0.75.0/drop-self-built-desktop-packages.md) | blocked behind the loopback redirect; the CLI half already shipped in v0.73.0 | once loopback lands, drop the four Tauri desktop `.deb`/`.rpm` and fix every release-asset consumer in the same commit |
| [windows-deeplink-second-instance](v0.75.0/windows-deeplink-second-instance.md) | inferred from source, never reproduced; very likely subsumed by the loopback redirect | reproduce it on Windows first, then confirm loopback closes it and decide whether to keep `chan://` registered at all |
| [terminal-mouse-toggle](v0.75.0/terminal-mouse-toggle.md) | accepted feature, deferred out of v0.74.0; no mouse toggle exists and "mouse" is five independent mechanisms | settle the "stop TUIs capturing the mouse" vs "kill all mouse" variant, then add a `terminal.*` config boolean and a `TerminalSection.svelte` checkbox wired like `scrollback_mb` |

## Completed

### v0.74.0

Shipped 2026-07-22; see [release-v0.74.0](../release/release-v0.74.0.md). Closed items in [`done/`](done/):

- [distributed-proxy-control-plane](done/distributed-proxy-control-plane.md) - the gateway coordinates devserver-proxies through one authenticated control service, replacing uncoordinated singletons.
- [distributed-proxy-control-plane-hardening](done/distributed-proxy-control-plane-hardening.md) - the accepted security hardening (Ed25519 admission leases, opaque sessions, durable revocation) shipped with it.
- [distributed-proxy-control-plane-implementation-security-review](done/distributed-proxy-control-plane-implementation-security-review.md) - the independent adversarial re-review that cleared the hardening to merge.
- [open-routing-multiple-local-instances](done/open-routing-multiple-local-instances.md) - `chan open` routes deterministically when several local instances run.
- [terminal-submit-chord-authority](done/terminal-submit-chord-authority.md) - the server owns the submit chord and `cs terminal list` shows each session's derived agent.
- [control-terminal-wake-rerun](done/control-terminal-wake-rerun.md) - a macOS wake no longer re-runs the devserver connect script on the control terminal.
- [devserver-token-rotation](done/devserver-token-rotation.md) - the devserver bearer token rotates by verb and by age, and stays out of WebView snapshots.
- [markdown-heading-detection-in-fences](done/markdown-heading-detection-in-fences.md) - fold chevrons no longer appear beside `#` comments in fenced code; headings come from the syntax tree.
- [release-asset-verification-coverage](done/release-asset-verification-coverage.md) - the release-asset verifier single-sources the required list and requires the Windows artifacts.
- [aur-publish-verification-race](done/aur-publish-verification-race.md) - the AUR post-push RPC check is advisory, not a false red.
- [copr-build-provenance](done/copr-build-provenance.md) - a frozen-main window plus a publication-provenance probe for COPR.
- [aur-aarch64-publication-gate](done/aur-aarch64-publication-gate.md) - withdrawn: aarch64 AUR CI validation was removed rather than made a gate; the aarch64 PKGBUILD still ships.

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
