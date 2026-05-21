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

---

# ci-12 post-mortem: workspace-wide GTK gap in CI test jobs

Author: @@CI
Date: 2026-05-21

Appended to this file per @@Architect's hint that
`ci-11` + `ci-12` are tightly coupled (the `ci-11`
smoke dispatch surfaced `ci-12`'s root cause).

## Summary

`cargo clippy --all-targets` from the workspace root
walks every workspace member, including
`desktop/src-tauri` (chan-desktop). chan-desktop's
Linux Tauri stack pulls `webkit2gtk-sys` + `glib-sys`
via the gtk-rs crate family; both compile through
pkg-config against GTK system libs. `ci.yml` and
`release.yml`'s test jobs do not apt-install the GTK
stack, so `glib-sys`'s build script reds at
"Package glib-2.0 was not found in the pkg-config
search path".

The per-PR CI gate has been red on `main` since
~2026-05-19 (about 15 consecutive runs) with this
exact root cause. The release.yml smoke dispatch fired
during `ci-11` validation (run 26227752597) hit the
same gap on `test-linux`.

## Cause

`desktop/src-tauri` joined the workspace 2026-05-19
when the chan-core / chan workspace merge landed
(commit `ff80ad7`). Before the merge, ci.yml's clippy
ran against a workspace that did NOT include
chan-desktop; the GTK install step was unnecessary.
After the merge, the same clippy invocation walks
chan-desktop, but the workflow's GTK-install step was
not added because:

* The merge commit's primary scope was the workspace
  consolidation itself, not the per-PR CI matrix.
* The `release-desktop.yml` workflow (which DID
  acquire the GTK install step in `ci-2`) builds
  chan-desktop via `make build`, NOT via `cargo clippy
  --all-targets` from the workspace root. Its scope
  was narrow; reviewers focused on its own correctness,
  not on whether `ci.yml`'s shape needed mirroring.
* Local dev workstations need GTK installed for
  `make run` (chan-desktop launches webkit2gtk in
  process). The local pre-push hook
  (`scripts/install-hooks`) runs the same clippy gate;
  passes locally because GTK is already present. The
  gap was invisible to anyone running the pre-push
  hook on their own machine.

## How it stayed latent

Two compounding factors:

1. **Local pre-push hook was the de facto gate.** Per
   the [`feedback-pre-push-checks`](file://~/.claude/projects/-Users-fiorix-dev-github-com-fiorix-chan/memory/feedback_pre_push_checks.md)
   memory, every push runs fmt + clippy + test +
   svelte-check + npm build locally before hitting
   the remote. With GTK present on every contributor
   machine, those checks pass cleanly. The GitHub
   Actions ci.yml gate became a known-broken second
   opinion; the per-PR badge was "ignorable red".

2. **No first-real-fire trip until `ci-11`'s smoke.**
   Per `ci-11`'s post-mortem above, `release.yml` did
   not fire on any phase-8 tag because its trigger
   glob did not match `chan-v*`. So the
   `release.yml::test-linux` job, which has the same
   workspace-clippy shape as `ci.yml::test`, never
   ran during phase-8 either. The `ci-11`
   workflow_dispatch smoke against main HEAD was the
   first time ANY workflow's workspace clippy
   exercised the post-merge tree. That trip surfaced
   `ci-12`.

   Connection to `ci-11`'s prevention layers:
   `ci-11` argued for trigger-mismatch preflight as a
   layered defence. `ci-12` is the canonical example
   of why first-real-fire matters: a glibly green
   smoke dispatch (workflow definition valid, all
   jobs queue, runners spin up) reveals the
   underlying state-of-the-tree problem only because
   the workflow actually ran end-to-end.

## Fix

Mirror `release-desktop.yml` lines 114-123 GTK install
into every workflow job that runs `cargo clippy
--all-targets` / `cargo test --all-targets` from the
workspace root on Linux:

* `.github/workflows/ci.yml::test` (matrix; conditional
  on `matrix.os == 'ubuntu-latest'` so the
  Windows entry stays unchanged).
* `.github/workflows/ci.yml::no-default-features`
  (always Ubuntu; unconditional step).
* `.github/workflows/release.yml::test-linux` (always
  Ubuntu; unconditional step).

Package list verbatim from `release-desktop.yml`:
`libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`,
`librsvg2-dev`, `libsoup-3.0-dev`, `patchelf`. The
minimum technical requirement for `glib-sys`'s pkg-config
is `libglib2.0-dev` alone, but the full stack matches
the existing workflow's pattern + leaves no drift for
the next workspace-member addition that might pull a
different gtk-rs dependency.

Each new step carries a WHY comment block citing the
workspace-merge cause + the symmetry with
`release-desktop.yml` (per CLAUDE.md "comments explain
WHY, not WHAT").

### `workflow_dispatch` trigger added to ci.yml

Small scope addition: `ci.yml` previously triggered on
`push` to main + `pull_request` only. The architect's
task spec § "Smoke validation" required a
`gh workflow run ci.yml --ref <branch>` dispatch, but
the trigger did not exist, so the first attempt failed
with HTTP 422 "Workflow does not have 'workflow_dispatch'
trigger". Added `workflow_dispatch:` to ci.yml's `on:`
block, matching `release.yml`'s shape (which has had
the trigger since phase-8 began). One additional line.
Enables future smoke dispatches against patch branches
without needing to merge first.

### Out-of-scope finding surfaced for separate routing

The `ci.yml::test (windows-latest)` job is ALSO red,
but for an unrelated cause: a `result_large_err` clippy
lint against `crates/chan-drive/src/index/config.rs`
(and a handful of call sites). The `ConfigError` enum
boxes `toml::de::Error` as a variant; clippy on Windows
flags the resulting Err-variant as "very large" and
fails under `-D warnings`. This is a Rust source-code
issue, not a CI workflow issue, and lives in the
@@Systacean / FullStack lane (chan-drive). Surfaced in
the commit-readiness poke for @@Architect to route as
a separate `systacean-N` or `fullstack-N` task.

The Windows lint failure is independent of the GTK
fix; landing ci-12 unblocks Ubuntu (clippy + test +
no-default-features) but leaves Windows red until the
lint is addressed. The per-PR gate is **partially
restored** (3 of 4 affected jobs green); fully green
needs the separate lint fix.

## Prevention

Extends `ci-11`'s three layered defences with one
addition:

4. **Workspace-membership preflight on dep-tree
   changes.** When a Cargo workspace gains a new
   member (or an existing member acquires new
   transitive system-lib deps), any CI workflow that
   walks the workspace root with `cargo clippy
   --all-targets` / `cargo test --all-targets` MUST be
   audited for the new system-lib needs. Concrete
   check at workspace-merge / member-add review:

   ```
   cargo tree --workspace --depth 1 \
       | grep -E '^[├└]── [a-z].*sys '
   ```

   Lists every `*-sys` crate in the workspace tree.
   Each `-sys` crate IS a system-lib dep; each one
   needs an install step on every runner OS the
   workspace clippy targets. The `ff80ad7` merge added
   `glib-sys`, `gtk-sys`, `gobject-sys`, `gio-sys`,
   `webkit2gtk-sys`, `pango-sys`, `cairo-sys-rs`,
   `gdk-sys`, `gdk-pixbuf-sys` to the chan workspace.
   Spot-checking even one of those against ci.yml's
   apt-install (absent) would have caught it.

   Pre-existing workspace `-sys` deps to keep in mind:
   `openssl-sys`, `libsqlite3-sys`, etc; most have
   bundled / vendored features and don't need a system
   lib; gtk-rs is the exception that pkg-configs hard.

This prevention layer pairs with `ci-11`'s "trigger
preflight" + "first-real-fire validation" + "post-
release asset audit". The four together: trigger
correctness (ci-11 catch), workspace dep-tree (ci-12
catch), first-real-fire (catch latent), post-fire
audit (catch slipped-through).

## Backfill posture

@@Alex 2026-05-21: "whatever is cheaper". Per
architect routing: lean on the next `chan-v*` tag's
CI fire as the validation lap rather than re-running
ci.yml per-commit against the ~15 unverified main
heads. Reasoning:

* The unverified commits have been journaled +
  code-reviewed + locally pre-push-gated; a per-commit
  CI replay would surface only what the local hook
  already caught.
* Per-commit replay would burn ~15 × ~7 minutes of
  Ubuntu runner time (~100 min). A single tag fire
  sweeps the aggregate state in one matrix run.
* The first chan-v* tag post-`ci-12` (likely v0.12.0
  per Round-2 close) is the canonical validation:
  if `release.yml::test-linux` AND `ci.yml::test
  (ubuntu-latest)` both go green on that tag, the
  fix is confirmed end-to-end.

No backfill task cut. Documented here so the audit
trail captures the deliberate choice.

## Closes

This appended post-mortem closes the `ci-12`
acceptance criterion. The combined `ci-11` + `ci-12`
post-mortem file is the load-bearing reference for
future workflow + dep-tree audit work.
