# ci-11 — Post-mortem: `release.yml` trigger-glob mismatch (latent across phase-8)

Author: @@CI
Date: 2026-05-21

Short post-mortem for the `release.yml` trigger-glob
mismatch surfaced by `ci-11`. Captures cause, masking
factors, fix, and prevention. Companion to the
1-line YAML patch landed under `ci-11`.

## Summary

`release.yml` (the chan CLI release workflow that builds
the 5-target matrix + .deb / .rpm / .pkg + uploads to
GitHub Releases) had a trigger glob `tags: ['v*']`.
Phase-8 adopted the `chan-v*` tagging convention. The
glob silently did not match any phase-8 release tag,
so chan CLI binaries never shipped for `chan-v0.11.0`,
`chan-v0.11.1`, or `chan-v0.11.2`.

The miss stayed latent for the entire phase-8 release
arc (six weeks; three release tags); spotted
2026-05-21 by @@CI during v0.11.2 GH Release asset
verification.

## Cause

`release.yml` predates phase-8. Its trigger glob was
last set when the chan workspace used the bare `v*`
tagging convention (e.g. `v0.10.1`, `v0.11.0`). When
@@Architect adopted the `chan-v*` convention in
phase 8 (so the chan-desktop tag space could co-exist
with the chan CLI tag space without overlap, per
[`release-desktop.yml`](.github/workflows/release-desktop.yml)'s
header comment "Trigger pattern is distinct from
release.yml's `v*` tags so the chan CLI release flow
stays untouched"), the new convention applied to the
chan CLI release as well, but the corresponding glob
update never landed.

## How it stayed latent

Three masking factors compounded:

1. **Billing block on earlier phase-8 release tags
   masked the silence.** `chan-v0.11.0` and
   `chan-v0.11.1` both pushed when GitHub Actions
   billing was blocked on the `fiorix` account (per
   [`event-ci-alex.md`](../alex/event-ci-alex.md)
   2026-05-21 billing entry). Both release tags' runs
   of `release-desktop.yml` failed in 2-7 seconds with
   the failed-payment annotation. Observers (rightly)
   focused on the billing-fail; nobody checked whether
   `release.yml` would also have fired but-for the
   billing block.

   With billing fixed and `chan-v0.11.2` shipping
   end-to-end via `release-desktop.yml` (signed DMG;
   first signed chan-desktop release), the absence of
   a `release.yml` run on the same tag should have
   been visible — but `gh run list` filters by
   workflow name, so "no runs of release.yml on
   chan-v0.11.2" wasn't a thing anyone naturally
   queried.

2. **Mental model described intent, not state.**
   [`architect/journal.md`](../architect/journal.md)
   documents the system as "on the `chan-v*` tag per
   `release.yml`" — which is what `release.yml`
   SHOULD do per design but is NOT what its trigger
   glob did. The mental model worked correctly across
   downstream discussion (commit plans, dispatch
   pokes, release coordination) because the model
   matched intent. The model did not match the YAML
   file.

3. **ci-2 + ci-4 + ci-5 + ci-6 + ci-7 + ci-9 + ci-10
   all touched `.github/workflows/release-desktop.yml`
   but never `release.yml`'s trigger block.** The
   chan-desktop release pipeline got six rounds of
   review attention; the chan CLI release pipeline
   got zero. Nobody read the `release.yml` trigger
   block during phase-8 because the focus was
   entirely on the new chan-desktop signing arc.

   `ci-4` swapped `cargo install` for
   `taiki-e/install-action` in `release.yml` (commit
   `385da20`), but the YAML structural review at that
   time was about the install-step shape — the
   trigger block was out-of-scope for that change.

## Why ci-4's "Findings (2026-05-21 post-fire bug)"
post-mortem didn't catch this

`ci-4`'s latent `^2` bug was caught when the workflow
first fired for real (dryrun.1 surfaced it).
`release.yml` has not been fired by a real `chan-v*`
tag in phase-8 — and only fires by `chan-v*` AFTER
this `ci-11` patch lands. So "wait for the first real
fire to surface latent bugs" is exactly the trap that
let this stay hidden: the trigger glob is the gate
that decides whether the workflow ever fires at all.

A YAML-structural review of `release.yml` during
`ci-4` would have read `tags: ['v*']` and not flagged
it — the line is syntactically valid; only a check
against "what tagging convention is the project
currently using" would have caught the mismatch.

## Fix

1-line addition to `release.yml`'s trigger block:

```yaml
on:
  push:
    tags:
      - 'v*'        # legacy v0.6.x .. v0.11.0
      - 'chan-v*'   # phase-8 convention (chan-v0.11.x)
  workflow_dispatch:
```

`ADD` (not replace) per @@Architect's recommendation —
back-compat-safe for the legacy `v*` tags that still
exist in the repo (v0.6.8 through v0.11.0 per
`git tag --list 'v*' | grep -v chan-v`).

Header comment also updated to document the dual
pattern + the rationale.

## Prevention

Three layered defences for future workflow additions /
edits:

1. **Trigger preflight on workflow add/edit**. When
   any future task adds OR modifies a workflow's
   trigger block, the structural review MUST include
   an audit of the project's CURRENT tagging
   convention. Concrete:
   `git tag --list | sort -V | tail -5` → does the
   trigger glob actually match? `git log --oneline
   --tags --decorate -10` shows the most recent
   tagged releases; mismatch with the workflow's
   `tags:` glob is the smell.

2. **First-real-fire validation**. Workflows triggered
   by tag pushes do not surface latent bugs until the
   first matching tag fires. `ci-8`'s dry-run
   discipline (test-tag fires the workflow against
   real keys) IS the right pattern for the signing
   path; the same pattern applies to ANY new
   workflow trigger. Future workflow additions that
   trigger on tag patterns SHOULD fire a one-off
   test tag (e.g. `chan-v0.x.99-dryrun.1`) BEFORE
   the first real release tag lands.

3. **Post-release asset audit**. After every real
   release tag fires, audit `gh release view <tag>
   --json assets` against the expected artifact list:

   ```
   gh release view chan-v0.11.2 --json assets \
       --jq '[.assets[] | .name]'
   ```

   Expected for a `chan-v*` release post-`ci-11`:
   * `Chan_<version>.dmg` (signed; chan-desktop)
   * `chan-<linux-x86_64>.deb` + `.rpm` + `.tar.gz`
   * `chan-<linux-aarch64>.deb` + `.rpm` + `.tar.gz`
   * `chan-<macos-aarch64>.pkg` + `.tar.gz`
   * `chan-<windows-x86_64>.exe` (in zip)
   * `chan-<windows-aarch64>.exe` (in zip)

   Missing artifacts = a build or upload failure,
   which `gh run list --workflow=<file>` against the
   tag identifies. The "no runs of release.yml on
   tag X" case (this exact bug) is also covered:
   `gh run list --branch <tag>` lists every workflow
   that fired on that ref; absence of an expected
   workflow is the catch.

The prevention shape is layered (defence-in-depth):
(1) catches obvious mismatches at task-review time;
(2) catches subtle bugs at first-real-fire time;
(3) catches anything that slipped through both at
post-fire verification. The phase-8 miss was a (1)
failure compounded by a (2) gap (no first-real-fire
because the trigger never matched).

## Closes

This post-mortem closes the audit-trail expectation
from `ci-11`'s acceptance criteria. Future workflow
additions should reference this file in their YAML
header comment OR cite it in the task body's
acceptance criteria as the preflight checklist.
