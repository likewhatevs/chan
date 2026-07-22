# COPR Build Provenance And Publication Verification

> Status: shipped in [v0.74.0](../../release/release-v0.74.0.md).

Status: accepted scope for v0.74.0. Two defects in the same publisher: COPR builds whatever `main` points at rather than the released tag, and nothing in CI observes whether COPR published at all.

## Problem

The COPR leg of `publish-downstream` is a fire-and-forget webhook. `.github/workflows/publish-downstream.yml:130` declares the `copr` job with a `chan` / `chan-desktop` matrix, and its only step is `Trigger COPR ${{ matrix.package }} build` at line 150, whose whole body is the guards at lines 155 to 161 and a single `curl -sf -X POST "${WEBHOOK}${PACKAGE}/"` at line 165. The job has no `actions/checkout` step and reads neither `github.event.workflow_run.head_sha` nor `inputs.tag`.

**Provenance.** Because the job passes no revision, the released commit never reaches COPR. The COPR SCM packages carry an empty committish, recorded in `packaging/distros/README.md:29` ("committish empty (builds main's HEAD)") and acknowledged in the workflow's own comment at `.github/workflows/publish-downstream.yml:163`: "COPR rebuilds the SCM packages from main's HEAD, which equals the tag once the release has landed." That equality holds only until the next push. Every other distro leg pins the release explicitly: Launchpad at line 188, both AUR jobs at lines 412 and 466, and Docker at line 626 all resolve `github.event.workflow_run.head_sha` (or the dispatched tag). COPR alone resolves nothing, so any push to `main` between the GA tag and COPR dequeuing the build produces Fedora and CentOS Stream packages built from a tree that is not the tag, while `make-srpm.sh` still stamps the version from the workspace `Cargo.toml`. The result is a package labelled `X.Y.Z` whose contents are `X.Y.Z` plus arbitrary post-tag commits, with nothing anywhere recording the difference.

**Publication verification.** The job exits green on the `curl` returning 200, which only proves the webhook was accepted for queueing, not that a build ran, and not that any chroot succeeded. An absent `COPR_WEBHOOK` is a silent green skip at lines 159 to 161, with no canonical-repository escape hatch. The neighbouring legs are stricter: `Report the PPA dry run` at lines 309 to 328 exits nonzero when `LAUNCHPAD_GPG_PRIVATE_KEY` is missing on `fiorix/chan` and distinguishes four outcomes by marker, and `Verify AUR metadata` at lines 563 to 584 polls `https://aur.archlinux.org/rpc/v5/info/$PKGBASE` until the pushed version is visible, then errors out. COPR has no equivalent, so the release procedure carries the verification by hand: `.agents/skills/release/SKILL.md:38` instructs the operator to read the COPR web UI and count jobs, and `packaging/distros/README.md:75` repeats it.

**Why this is now load-bearing.** chan no longer ships a self-built CLI `.deb` or `.rpm` (`CHANGELOG.md:22`); `.github/workflows/release.yml` builds only the four gateway server `.deb` packages (lines 246 to 275) and the Tauri desktop bundles (lines 348 to 355). `web/packages/marketing/src/pages/install.html:109` sends every Fedora and CentOS Stream user to `sudo dnf copr enable fiorix/chan`. COPR is therefore the single channel for those users, and a COPR failure now means they get nothing while the release run stays fully green.

## Desired contract

**Provenance.** Keep the COPR SCM packages on an empty committish and make the release procedure state, as an explicit rule, that `main` is frozen from the GA tag push until the COPR builds for both packages have completed; then have the new status probe assert the provenance rather than trusting the freeze.

Pinning the committish is the weaker option, and the reason is that the COPR package configuration is console state that nothing in this repository asserts and no API call sets: `packaging/distros/README.md:29` and `.agents/skills/release/SKILL.md:38` both say so, and the same class of console state already drifted once (the `chan-desktop` EL9 chroot denylist, `packaging/distros/README.md:75`). A pinned committish must be edited in the COPR web UI for every release, off-repo and unreviewable, and it fails in the worse direction: a forgotten bump rebuilds the previous tag forever, silently and without a race being needed, whereas an empty committish only misbuilds when someone actually pushes inside the window. A hand-maintained pin also cannot be validated by CI, so it would add a second unasserted console fact rather than removing one.

The probe closes the gap the freeze rule leaves. It reads the COPR API for the builds the webhook created and fails the job when the built source package version does not match the released tag, which detects a broken freeze at GA time instead of leaving it to a user's `dnf upgrade`.

**Publication verification.** After the `curl`, the `copr` job polls the unauthenticated COPR API for the resulting build and its chroots, and ends nonzero unless every chroot for that package succeeded at the released version. The v0.73.0 aarch64 harvest confirms the endpoints answer 200 without credentials: `team/roadmap/v0.73.0/packaging-aarch64-validation.md:30` cites `/api_3/build/list?ownername=fiorix&projectname=chan&packagename=chan`, and line 47 cites the per-chroot records from `/api_3/build-chroot`; `/api_3/monitor` and `/api_3/build/<id>` are equally open. No new secret is introduced.

The probe distinguishes four outcomes by message, following the precedent set at lines 309 to 328: every chroot succeeded at the expected version (green); a chroot failed or was cancelled (red, naming chroot and build id); the poll budget expired with builds still running (red, stating plainly that publication is unconfirmed rather than failed); and `COPR_WEBHOOK` absent on the canonical repository (red, matching line 319's contract, while staying a green no-op on forks). The poll budget must exceed the observed worst-case COPR build time for these packages, which the recorded build ids in `team/roadmap/v0.73.0/packaging-aarch64-validation.md` can be used to measure.

The probe reports the chroot set it observed for each package. It does not assert that set, because the enabled chroots and the EL9 desktop denylist remain console-only state; the human check at `.agents/skills/release/SKILL.md:38` stays.

## Boundaries

Changes are confined to `.github/workflows/publish-downstream.yml`'s `copr` job, one new script under `packaging/distros/copr/` implementing the probe (so it is shellcheck-gated and testable off CI, like `test-build-with-sdme.sh` alongside it), the COPR paragraphs of `packaging/distros/README.md`, and the downstream-publication step of `.agents/skills/release/SKILL.md`.

Out of scope: the spec files under `packaging/distros/fedora/`, `.copr/Makefile`, `make-srpm.sh`, the `copr-check` local matrix, and every Make target; the Launchpad, AUR, and Docker jobs; any change to COPR console configuration; and any new credential, since the API reads used here are unauthenticated.

## Acceptance

- The probe script has a host-side control-flow test in the shape of `packaging/distros/copr/test-build-with-sdme.sh`, driving it against recorded API fixtures for each outcome: all chroots succeeded, a failed chroot, a version mismatch against the released tag, builds still running past the budget, and a missing build for the package. Each case is captured red before the corresponding branch is implemented, per the standing rule recorded at `team/release/release-v0.73.0.md:37` that every new check is proven able to fail.
- The canonical-repository branch for an absent `COPR_WEBHOOK` is proven by the same fixture harness, not by removing the live secret.
- `make pre-push` is green, including `actionlint` over the edited workflow and `shellcheck` over the new script.
- A `workflow_dispatch` with `publish=false` still POSTs nothing and runs no probe, and the run says so.
- Owner-only, on the canonical repository: the first live green is the GA tag push for v0.74.0, since `COPR_WEBHOOK` and the `fiorix/chan` COPR project are reachable only there. A live red cannot be manufactured without breaking a real publication, so the red path is accepted on fixture evidence plus the dry run.
- The frozen-`main` rule is stated in `.agents/skills/release/SKILL.md`'s downstream-publication step, naming the window (GA tag push until both COPR packages report complete) and pointing at the probe as its detector.
