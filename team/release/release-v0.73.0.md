# Release v0.73.0 - decoupling publication from the release

Round run 2026-07-20 off the v0.72.0 tag. Two roadmap items shipped and one was partially delivered; the fourth, the distributed proxy control plane, was deferred again with its code reviewed and gated but unmerged. The release also absorbs the abandoned v0.72.1 patch. The centre of gravity is release infrastructure rather than product: publishing a release can no longer be held up by a container registry or a distribution, and chan stopped building packages the distributions now build for it.

## What shipped

- **OpenCode reconciles queued terminal notifications into one turn.** Several notifications arriving while OpenCode is busy now drain as a single batched submit, matching Claude and Codex. Gemini deliberately stays one message at a time, and that is a measured result rather than a deferral: a live sweep found a Return arriving close behind inserted text is still converted to Shift+Return, and no gap below the queue's 800 ms idle threshold left a safe margin for a full 64 KiB batch, so batching it would silently strand input in the compose box.
- **The Command Launcher's "Flip pane" row works.** Choosing "Flip pane" in the launcher, or "Flip" in the File Browser, did nothing at all: the launcher was still counted as the top overlay when it ran the command, so a guard meant to stop pane flips reordering panes behind an open overlay swallowed the flip. The overlay stack now reconciles the moment an overlay closes rather than one frame later. The chord and the A/B control were never affected, and the guard still bites with Search or a modal open.
- **Publication is layered, and every downstream target is secondary.** Docker Hub, COPR, the Launchpad PPA and the AUR moved out of the release run into one `publish-downstream` workflow that fires once the release succeeds and fans all four out in parallel. None can fail the release; none can block another. This closed a live defect: the Docker jobs lived inside `release.yml`, so a registry outage turned the whole run red, and every distro job gates on that run concluding success, which meant a Docker Hub outage silently blocked COPR, the PPA and the AUR with nothing naming Docker as the cause. The two COPR package triggers were also chained in one shell step, so a failure on `chan` meant `chan-desktop` was never triggered at all; they are now independent jobs.
- **chan no longer publishes its own CLI `.deb` and `.rpm`.** The four `chan-{amd64,arm64}.{deb,rpm}` assets carried no version in their filename and are now built by the distributions themselves. Installing from a distribution gives package management, signatures and upgrades that a loose file download never did.
- **The Arch AUR packages can publish again, and aarch64 builds on every release.** The post-install verification of the shipped systemd user unit ran without the privileges it needs and failed inside the build container, which blocked the AUR push for both packages; v0.72.0 never reached the AUR at all. `aur-validate-arm` now runs natively on aarch64 at GA as observed evidence, deliberately not gating publication, because an unproven job must not block a first publication for a second consecutive release.
- **The chan-desktop RPM refuses EL9 with a clear reason**, naming the two packages EPEL Next 9 does not provide instead of failing deep in dependency resolution.

## Team / process

Four lanes, three `codex` workers on `gpt-5.6-sol` at `xhigh`, one worktree and one branch each, with a coordinating Claude session as integrator. Lane `web` took the launcher defect and then the packaging lane; `term` took the queue-drain work; `ctl` took the control plane. The integrator owned merges, the CHANGELOG, the roadmap status blocks, and every gate.

The round opened with the abandoned v0.72.1 release still on `main`: prepared, pushed, never tagged. Reconciling it was the first act, moving every version pin to 0.73.0, renaming its dated CHANGELOG section to `[Unreleased]`, and removing its release report from an index that names tagged releases only.

Before spawning, seven agents re-verified every file:line anchor and factual claim in the four lane briefs against the live tree, then a completeness critic diffed the briefs against each other. That produced ninety findings and three that would have cost days: lane `term` had no server under test at all, because the queue-drain harness starts no server and the only running one was the shared binary hosting every worker's terminal; Gemini authentication was assumed dead and actually worked; and `distros-publish.yml` was believed contested and had in fact never been touched.

## Validation

- **The base was gated before any lane branched.** A full ten-step `make pre-push` on the reconciled tip, which was the first full gate any of the absorbed v0.72.1 content had ever seen: the four packaging commits had only ever been covered by the two static linters, and the version bump had never been compiled.
- **Every lane own-gated, and every merge was gated before pushing.** Where a merge touched only documentation and one already-gated surface, the integrator verified by diff that no gate step's inputs had changed and recorded that reasoning rather than asserting coverage.
- **The launcher fix was proven by capturing the red first.** Three behavioral tests written against the unmodified tree produced four genuine failures with the right assertions before the two-line fix landed. `make web-check` is green: svelte-check clean, launcher 294/294, workspace-app 2860/2860, both production builds.
- **The OpenCode promotion rests on a live matrix**, three runs each of `batch`, `boundaries` and `late` at 64 KiB with the server's explicit 50 ms gap, with no intermediate depth in any batch or late trace. The Gemini decision rests on a gap sweep from 10 ms to 700 ms with runs recorded per value.
- **The control plane's steps 1 to 4 are implemented, reviewed and gated**, including a live three-proxy vertical slice, but are not in this release. See Follow-ups.
- **Not proven here:** the restructured publication path has never executed. Its first real run is the GA tag push, and its only rehearsal is a `publish=false` dispatch, which is the owner's to fire. Native aarch64 execution, live COPR and AUR publication, and all desktop live-WebView behavior remain owner-only on this host.

## Retrospective

### Highlights

- **The queue-drain lane refused an inherited number and measured instead.** The 30 ms Shift+Return window that the item rested on had no timing artifact anywhere in the repository. The lane measured it: the real transition is 60 to 75 ms, a 64 KiB body was still stranded at 400 ms and still failed the content oracle at 700 ms, and 700 ms leaves only 100 ms below the idle gate. It then deleted the unverified number from the source comments and replaced it with what it had observed. A plausible-looking promotion resting on a number nobody checked was the easy outcome and it did not take it.
- **The control plane lane retracted its own passing result.** It had a clean three-proxy barrier matrix in hand when a review arrived showing four defects were still present, and it withdrew the claim and re-ran rather than defending a green it could have kept. Earlier it had also discarded a first barrier attempt because the identity stub used synthetic devserver ids where production derives them from the PAT.
- **Every new check was proven able to fail.** The control plane lane broke each one on purpose and captured the red before restoring it. This was the direct answer to a v0.72.0 lowlight about checks that could never redden.
- **Reviewing before merging paid for itself repeatedly.** The publication split was verified to publish all four targets correctly on a first GA run, and two defects were found that only bite on a retry: Docker was the one job checking out `github.sha` rather than the dispatched tag, so the documented retry could have tagged post-release `main` as the release and moved `latest` onto it.

### Lowlights

- **The headline item did not ship, for the second release running.** Steps 5 to 9 of the control plane turned out to be comparable in size to steps 1 to 4, which was not visible when the round was scoped. The gateway therefore ships at 0.73.0 byte-identical in behavior to 0.72.0: the entire gateway diff is the version pin and its lockfile.
- **Two status signals nearly caused destructive recovery.** Every team tab reported `offline` simultaneously while all three agents were alive and working, and separately a lane looked unresponsive to seven messages when its queue depth showed it had drained none of them. Both were misread as failures. Restarting a tab on either signal would have destroyed hours of context and uncommitted work.
- **A lane rewrote branch history after the integrator had merged and pushed it**, inserting a commit ahead of two existing ones and diverging the branch from `main` with duplicate content on both sides. Recovery was a content verification plus a cherry-pick of the one genuinely new commit, but the round's invariants already forbade this and it is the second occurrence across rounds.
- **The integrator gave the owner a wrong recommendation and had to retract it.** Phase two of the package drop was held on the grounds that dropping the desktop packages would break Linux deep-link sign-in. Investigation showed those packages ship a `.desktop` whose `Exec` has no field code, so they almost certainly never delivered the callback at all, and the proposed remedy would not have worked either.

### Honest feedback

The most valuable work in this round was not written by the lanes. It was the pre-spawn verification that found lane `term` had no server to test against, and the post-implementation reviews that found a cancel-unsafe read in the control plane's transport and a retry path that could publish the wrong tree as a release. None of those was reachable by a green gate: the transport bug needs a frame larger than 16 KiB to fire and every test wrote whole frames, and the retry defect only appears on a path nobody exercises until something has already gone wrong. A round that only runs its gates will ship all three.

The corollary is uncomfortable. Three of the four lanes produced work that passed their own gates and still needed correction, and the corrections came from reading the code adversarially rather than from running anything.

## Follow-ups

- **The distributed proxy control plane moves to v0.74.0** with steps 1 to 4 done, gated and reviewed, and steps 5 to 9 unstarted. Its branch is preserved. The accepted ruling on the joining-snapshot finding is recorded and not yet implemented: during routine joining the controller-authoritative live row wins any duplicate, and the lexicographic restart tie-break stays confined to initial reconstruction where recency is genuinely unavailable.
- **The loopback redirect for desktop sign-in is designed but explicitly not ready to implement.** A security review of the naive design found three exploitable gaps, including that the redemption code is bound to nothing, so an attacker who learns the state nonce can make a victim's desktop store the attacker's own token. The design needs a refinement pass before code.
- **The desktop `.deb` and `.rpm` drop waits on that**, after which it is pure cleanup.
- **Make the Arch aarch64 validation gate publication** once one ARM cell has passed on a real release. Read the v0.73.0 run's ARM cells first.
- **Windows very likely has the same deep-link second-instance gap** as Linux, inferred from source and never reproduced.
- **COPR builds `main`'s HEAD rather than the released tag, and nothing verifies it published.** Both got sharper now that the self-built `.rpm` is gone and COPR is the only Fedora and CentOS channel.
- **Debian has no channel at all.** The PPA is Ubuntu only, and the dropped generic packages were built on 24.04-era runners so they never served Debian either. Those users have the static binary, `install.sh` and the AppImage.
- **The launcher flip fix has no live check in a rebuilt server.** The unit tests run against source and would not catch a stale `rust-embed` bundle.
