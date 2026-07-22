# The AUR Publish Verification Races The RPC Index And Reports Red On Success

> Status: shipped in [v0.74.0](../../release/release-v0.74.0.md).

Status: accepted scope for v0.74.0. A confirmed defect observed on the v0.73.0 GA run.

## Problem

`aur-publish` pushes the recipe to the AUR and then asserts that the AUR's RPC index already reports the new version. The push is the publication; the index is a downstream projection of it. On the v0.73.0 GA run (workflow run 29781883990) both pushes succeeded, both AUR repositories were created, and both `aur-publish` cells failed with `the AUR did not report <pkgbase> 0.73.0-1 after the push`. The workflow reported red on a run that shipped.

The publication step is `Push to the AUR` at `.github/workflows/publish-downstream.yml:536-561`. It clones `ssh://aur@aur.archlinux.org/$PKGBASE.git` (`.github/workflows/publish-downstream.yml:552`), copies the validated `PKGBUILD` and `.SRCINFO` from the `aur-validate` artifact, exits 0 early when the staged tree is unchanged (`.github/workflows/publish-downstream.yml:556-559`), and otherwise commits and runs `git push origin HEAD:master` under `set -euo pipefail` (`.github/workflows/publish-downstream.yml:561`). That `git push` is the transaction: if it returns 0, the AUR has the recipe.

The verification is a separate step, `Verify AUR metadata` at `.github/workflows/publish-downstream.yml:563-584`, and it is inline in the workflow. Nothing under `packaging/distros/arch/` performs it; `build-in-ci.sh` and `build-in-container.sh` cover rendering, building, namcap and the install smokes, not publication. It computes `expected="${RELEASE_TAG#v}-$AUR_PKGREL"` (`.github/workflows/publish-downstream.yml:569`) and loops six times (`.github/workflows/publish-downstream.yml:570`), each iteration fetching `https://aur.archlinux.org/rpc/v5/info/$PKGBASE` with `curl -fsSL --retry 3` and asserting through `jq -e` that `.resultcount == 1 and .results[0].Version == $expected` (`.github/workflows/publish-downstream.yml:571-573`). A match echoes success and exits 0 (`.github/workflows/publish-downstream.yml:574-575`). A miss sleeps a flat 10 seconds, except after the last attempt (`.github/workflows/publish-downstream.yml:579-581`). When the loop is exhausted it emits `::error::the AUR did not report $PKGBASE $expected after the push` and exits 1 (`.github/workflows/publish-downstream.yml:583-584`).

This corrects one assumption in the framing of this item: the check is not an immediate single-shot query. It already retries. The defect is that the budget is far too small and the timeout verdict is wrong, not that retry is missing.

Quantify what the code allows. Six attempts with five 10-second sleeps is 50 seconds of deliberate waiting, plus whatever the six `curl` calls cost. So the entire tolerance for AUR index lag is under a minute. For a brand-new pkgbase the RPC returns `resultcount: 0`, so the `jq` predicate fails on the first clause and no amount of version tolerance helps. On the v0.73.0 run the AUR RPC subsequently reported `chan` 0.73.0-1 and `chan-desktop` 0.73.0-1, maintainer `fiorix`, with `LastModified` 2026-07-20T22:21:56Z and 22:21:59Z respectively: the exact moment of the push. The push landed; the index simply had not caught up inside a 50-second window. Both pkgbases were previously unclaimed, so this run created both AUR repositories, which is the slowest case the index has.

## Why This Is Worth Doing

This repository has a standing rule that every new check must be proven capable of going red. The inverse failure is at least as damaging and has no such rule protecting against it. A check that fires on a successful run teaches the operator that its red is noise. The next genuine AUR publication failure, a rejected key, a push refused by the AUR's `.SRCINFO` validation hook, a version mismatch between the pushed recipe and the tag, arrives with the identical job name, the identical step name, and a nearly identical error line, and gets waved through with the same reasoning that was correct the previous time.

That is the argument for the item. It is not a cosmetic cleanup of a noisy log line. Left as is, this step converts the only automated confirmation that chan reached the AUR into a signal the release operator is trained to discard, which means the AUR publication effectively becomes unverified while still costing a red workflow every release.

## Desired Contract

The step distinguishes three outcomes and treats only the third as an error.

- **Pushed and confirmed.** `git push` succeeded and the RPC reports the expected `<version>-<pkgrel>`. Green, with the confirmation echoed.
- **Pushed but not yet indexed.** `git push` succeeded and the RPC has not caught up within the polling budget. Green, with `::warning::` naming the pkgbase, the expected version, how long it waited, and the RPC URL an operator can check by hand. The job summary says the recipe was pushed and the index confirmation timed out.
- **Push failed.** The clone, the commit or `git push` returned nonzero, or the RPC affirmatively reports a version that is neither the expected one nor an older one consistent with a failed push. Red.

Recommendation: **git's own push result is the authoritative signal and the RPC check is advisory.** The AUR is a git host; a zero exit from `git push origin HEAD:master` against `aur.archlinux.org` means the server accepted the ref, and the AUR's server-side hooks reject a malformed `.SRCINFO` at push time rather than silently. The RPC index is a separate, asynchronously populated system with no published freshness guarantee, so making the release verdict depend on it imports an unrelated service's latency into chan's release outcome. Under `set -euo pipefail` the push step already fails the job on a bad push, so the authoritative half is in place and needs no new machinery; the change is to stop the advisory half from being able to fail the job.

Keep the poll, and widen it: bounded retry with exponential backoff, roughly 10s, 20s, 40s and so on capped at 60s per interval with a total budget on the order of 10 minutes, so an ordinary index refresh confirms in-band and only a genuinely stalled index falls through to the warning. Widening the budget without changing the verdict would be a strictly worse fix: it makes the same false red rarer, which is exactly what makes it more likely to be believed and waved through.

Two details worth carrying into the implementation. The predicate `.resultcount == 1 and .results[0].Version == $expected` at `.github/workflows/publish-downstream.yml:573` cannot distinguish "not indexed yet" (`resultcount: 0`) from "indexed at the wrong version", and those are different facts: the first is expected on a new pkgbase, the second is a real discrepancy that deserves the loud path. Split them. And the early `nothing to push` exit at `.github/workflows/publish-downstream.yml:556-559` returns 0 from the push step without pushing, after which the verification step still runs; under the new contract that path is "already published", which should confirm against the RPC and warn, not error, on lag.

## Acceptance

The deliberate mutation that proves the check can still go red: in a scratch branch, change the assertion so it demands a version that was never pushed, for example hardcoding `expected=999.0.0-1` at `.github/workflows/publish-downstream.yml:569`, and confirm the run ends red on the push-failure path rather than degrading to the advisory warning. The complementary mutation proves the authoritative half: point the clone at a pkgbase that does not exist and is not owned by the credential, for example `ssh://aur@aur.archlinux.org/chan-does-not-exist.git` at `.github/workflows/publish-downstream.yml:552`, and confirm the push step itself fails the job before verification runs. Both mutations must be reverted before the branch merges; neither belongs in the shipped workflow.

Be explicit about the limit: exercising this end to end requires a real AUR publication, which happens only at a GA. A `publish=false` dispatch never reaches the push or the verification step at all, because both are gated on `env.PUBLISH == 'true' && env.PUSHABLE == 'true'` (`.github/workflows/publish-downstream.yml:537`, `.github/workflows/publish-downstream.yml:564`), and it takes the dry-run reporting branch at `.github/workflows/publish-downstream.yml:522-534` instead. The mutations above can therefore be run against the polling logic in isolation, but the three-way contract is only observed for real at the next GA tag. The acceptance evidence is that GA run: both `aur-publish` cells green, and, if the index lagged, a `::warning::` in each naming the pkgbase and the wait.

## Confirmed Good News From The v0.73.0 Run

Recorded so the next reader has the full picture, not only the defect.

`aur-validate (chan)` and `aur-validate (chan-desktop)` both passed, for the first time. That leg (`.github/workflows/publish-downstream.yml:392-427`) renders, builds, installs and smokes each recipe in a clean upstream Arch container via `packaging/distros/arch/build-in-ci.sh`, and it had been failing on the post-install `systemd-analyze verify` of the shipped systemd user unit; routing that call through sudo (`packaging/distros/arch/build-in-container.sh:133-134`) unblocked it. It is the publication gate named in `needs: [aur-auth, aur-validate]` at `.github/workflows/publish-downstream.yml:499`, so its passing is what let the publication happen at all.

Both AUR repositories now exist. `chan` and `chan-desktop` are live at 0.73.0-1 with maintainer `fiorix`. Every future release pushes to an existing pkgbase, which is the faster indexing case, so the false red this item fixes will be intermittent rather than certain from here on. That makes it more dangerous, not less: a check that reds sometimes is exactly the shape an operator learns to dismiss.

For completeness, `aur-validate-arm` failed for both packages on the same run, at its very first step, with `curl: (60) SSL: no alternative certificate subject name matches target host name 'os.archlinuxarm.org'`: the upstream Arch Linux ARM host's TLS certificate does not cover its own hostname. The job never reached the keyring bootstrap, the rolling `-Syu`, `makepkg --syncdeps`, the native ARM Tauri build, the namcap gate or the install smokes, so everything below the rootfs import at `.github/workflows/publish-downstream.yml:470-483` remains unproven. It carries `continue-on-error: true` (`.github/workflows/publish-downstream.yml:454`) and is absent from `aur-publish`'s `needs`, so it blocked nothing. That leg is the subject of a separate item, `team/roadmap/done/aur-aarch64-publication-gate.md`, whose stated precondition is one GA release in which both ARM cells pass; the v0.73.0 run did not supply it.

## Boundaries

This item changes only the verification semantics in `aur-publish` and, if the split above requires it, the shape of the RPC query. It does not touch the push step's credential handling, the `aur-auth` probe (`.github/workflows/publish-downstream.yml:336-386`), the recipe rendering under `packaging/distros/arch/`, or the ARM leg's gating status.
```

Corrections made to the brief: the verification is not an immediate post-push query. It is already a bounded loop of six attempts with flat 10-second sleeps (`.github/workflows/publish-downstream.yml:570-581`), i.e. under 60 seconds of tolerance, and the failure is the size of that budget plus the error-on-timeout verdict, not the absence of retry. Also, no script under `packaging/distros/arch/` participates in the verification; it is inline in the workflow.
