# Roadmap and Release Flow

> Status: shipped in [v0.71.0](../../release/release-v0.71.0.md).

Status: proposal and execution plan. This is the first v0.71.0 item to land before any technical v0.71.0 work. Grounded against `a27007f5` (`v0.70.3`) on 2026-07-18.

## Summary

Make `team/` the front door for how chan is developed, with separate live-roadmap and release-history trees:

- `team/README.md` becomes the contributor-facing development and coordination guide.
- `team/roadmap/` holds active proposals by target version and a flat `done/` archive.
- `team/release/` holds the existing release history and its index.
- The current v0.71.0 analysis files move from `dev/v0.71.0/` into the active roadmap when this plan is executed.
- The v0.71.0 GA commit closes both sides atomically: it adds the final release report and index entry, moves every resolved roadmap item to `team/roadmap/done/`, and links each item to the release report.

One landing barrier controls the transition: the process migration described here must reach `main` before any technical v0.71.0 commit reaches `main` or the v0.71.0 RC branch. Technical work may proceed concurrently in separate branches and worktrees, then rebase onto the process commit before intake.

This file plans that migration. It does not perform it, move the existing analyses, open an RC, bump a version, or publish a release.

## Current State

- `team/` is flat: one `README.md` release index plus 79 release reports, with no subdirectories.
- `docs/coordination.md` explains the internal multi-agent flow but explicitly says it is not a contributing guide.
- `dev/v0.71.0/` contains three completed investigations or implementation proposals:
  - `chan-upgrade-release-history-fix.md`
  - `chan-workspace-graph-fix.md`
  - `terminal-write-queue-drain.md`
- `/dev/` is ignored by `.gitignore`; none of these files is currently in the Git index. The migration introduces them as new tracked roadmap files rather than Git renames.
- `.agents/skills/release/SKILL.md` is the current release procedure and still names `team/release-vX.Y.Z.md` and `team/README.md` as the report locations.
- No `team/roadmap/` or `team/release/` path exists yet.

## Verified Release Facts

The migration must preserve these facts rather than carrying the draft's ambiguous release wording forward:

1. An RC is a version-pin state on a branch such as `0.71.0-rc1`, not a tag. Recent live dry runs use the no-`v` form (`0.68.0-rc1`, `0.69.0-rc1`, and `0.70.0-rc1`). `.github/workflows/release.yml` publishes every pushed `v*` tag, including an RC-shaped tag, so only the GA `v0.71.0` tag may be pushed.
2. A mandatory dry run is a `release.yml` workflow dispatch from the RC branch with `publish=false`. Its artifacts must be downloaded and tested before GA.
3. The release workflow builds the direct GitHub artifacts, including Linux `.deb` and `.rpm` files. These are distinct from the Fedora COPR and Ubuntu Launchpad packages.
4. COPR and PPA publication is intended to run after a successful tagged Release workflow through `.github/workflows/distros-publish.yml`. It is automated and non-gating, with local manual commands retained for pre-release testing and retries. The current filter checks only for a successful Release run whose `head_branch` starts with `v`; it does not also require that the source event was a tag push. A `v`-prefixed RC branch could therefore trigger distro publication after a successful dry run. The process item must harden this guard before claiming dry runs are publication-free.
5. `packaging/linux/arch/PKGBUILD` has no committed per-release version. `make linux-archpkg` supplies `CHAN_PKG_VERSION`; Arch is a local build and QA path, not a separately published repository in the current automation.
6. The repository has a local, production-shaped gateway stack at `packaging/gateway/scripts/dev/README.md`. It uses `*.localtest.me`, real gateway binaries, Postgres, and a manually registered OAuth development app. This is a local shadow environment, not a separately deployed shadow tier.
7. The release skill currently points signing notes at `docs/release/README.md`, which does not exist. That dead reference must be repaired as part of the skill audit.

## Target Layout

```text
team/
  README.md
  roadmap/
    README.md
    v0.71.0/
      release-flow.md
      chan-upgrade-release-history-fix.md
      chan-workspace-graph-fix.md
      terminal-write-queue-drain.md
    done/
  release/
    README.md
    release-01-prerelease.md
    ...
    release-v0.70.3.md
```

Existing release report filenames stay unchanged. The migration changes their directory and repairs links, but does not rewrite their historical content.

`team/roadmap/done/` is intentionally flat. Roadmap item names must therefore remain descriptive and repository-wide unique. If a future item would collide, prefix that filename with its version when it is closed.

## Document Responsibilities

### `team/README.md`

The new guide replaces `docs/coordination.md` as the development-process front door. It should explain both the maintainer's normal flow and the standard external-contributor path:

- Start from one concrete problem. Investigate it against the live tree with one agent until the result is either an analysis or an implementation-ready proposal. Simple problems may combine both in one file.
- Put accepted proposals in `team/roadmap/vX.Y.Z/{item}.md`.
- Once a version has enough coherent scope, prepare one lead to use `cs terminal team`, then provision file-disjoint lanes and rounds.
- Define each lane's file ownership, dependencies, build access, and validation environment before dispatch. Relevant resources include headless browsers, macOS or Windows hosts, Lima or WSL Linux guests, sdme distro containers, root and gateway Cargo workspaces, desktop packaging, COPR, Launchpad, and local gateway services.
- Existing implementation branches are valid intake candidates. Coordination then centers on review, overlap detection, rebasing, gates, smoke tests, and merge order rather than reimplementation.
- External contributors may use the normal branch and PR flow. They do not need to join the maintainer's agent team, but their proposal, validation evidence, and candidate report should map cleanly onto the same roadmap and RC intake model.

The guide links to `roadmap/README.md` for lifecycle rules, `release/README.md` for history, `.agents/playbook.md` for operational lessons, and `.agents/skills/release/SKILL.md` for the executable release procedure. It must not duplicate mutable command-level release details from the skill.

### `team/roadmap/README.md`

The roadmap index defines the state machine:

1. `vX.Y.Z/{item}.md` is accepted active scope for that target.
2. Implementation and validation evidence accumulate in the proposal, candidate report, or round artifacts without replacing the proposal's original rationale.
3. At GA, a completed item moves to `done/{item}.md` and receives a release link.
4. A withdrawn item also moves to `done/`, says explicitly that it did not ship, and links to the release report that records the decision.
5. A deferred item moves to the next active version directory before GA. It is not marked done.
6. The released version directory must be absent after the GA close commit.

The index lists active version directories first and completed items by release below them. It is the roadmap front door, not a second release report.

### `team/release/README.md`

The current `team/README.md` moves here with its complete index and conventions. Its relative link to `.agents/playbook.md` changes from `../.agents/playbook.md` to `../../.agents/playbook.md`; report links remain sibling links.

Future final and candidate reports live under `team/release/`:

- `team/release/release-vX.Y.Z.md`
- `team/release/release-vX.Y.Z-rcN-{feature-branch}.md`

## Development and Release Lifecycle

### 1. Investigate and accept scope

Work one problem at a time until its proposal names the observed behavior, evidence, desired contract, implementation boundaries, acceptance checks, and unresolved risk. Add the accepted file to the target version's roadmap directory. Do not turn a raw draft into committed scope merely by moving it.

For v0.71.0, the process migration moves the four documents shown in the target layout without rewriting the three existing technical analyses.

### 2. Prepare the delivery team

The host teaches one lead the live `cs terminal team` workflow and has that lead produce the team configuration, branch and worktree map, lane ownership, dependency graph, and validation matrix. Provision only the access each lane needs. Secret values never enter roadmap, task, journal, or release files.

Technical branches may already exist or may start in parallel with this process work. They remain outside `main` and the RC until the process migration lands. Any branch based on the pre-migration tree rebases afterward so its reports and links use the new paths.

### 3. Implement and validate in lanes

Each lane owns a disjoint surface and reports a scoped-green commit. The lead resolves overlap before intake and gates the committed state from an isolated gate worktree. A pre-existing implementation receives the same review, test, and smoke requirements as newly written work.

The validation matrix is selected per item:

- Web and embedded frontend: static checks, unit tests, production bundle, and browser smoke against the served bundle.
- Desktop: platform build plus native hand-smoke on each affected OS. Linux guests on macOS or Windows may come from Lima, WSL, or sdme, but they do not substitute for a native macOS or Windows check when platform behavior changes.
- CLI and distro artifacts: root workspace gate plus direct `.deb`, `.rpm`, and Arch local-package checks where affected.
- Gateway: nested workspace gate, real-service tests, and the local `*.localtest.me` stack with its manual OAuth development-app setup. If an item needs a true deployed shadow tier rather than the local stack, that infrastructure is a separate explicit roadmap item, not an assumed capability.

### 4. Open and iterate the RC

After the process barrier and candidate intake are ready, the release owner opens `0.71.0-rc1` by bumping every version pin to `0.71.0-rc1` in one commit and pushing the branch. The branch deliberately has no leading `v`; only publishable tags use that prefix. Accepted technical candidates rebase onto that branch and merge provisionally.

Dispatch `release.yml` from the RC branch with `publish=false`. Confirm the context, macOS signing and notarization path, Linux, Windows, desktop, CLI, and gateway jobs all run as intended. Download the complete artifact set and validate package versions, installability, signatures where applicable, and representative runtime behavior. A failed check returns to its owner or overflows explicitly; it never becomes an RC tag.

If fixes land, bump all pins to `0.71.0-rc(N+1)` and repeat the dry run. The full-tree gate and required hand-smokes must be green on the exact candidate that advances to GA.

### 5. Close roadmap and release in the GA commit

The GA commit is the single repository close point and the commit that receives the `v0.71.0` tag. It contains all of the following:

- `team/release/release-v0.71.0.md`, including shipped scope, roadmap outcomes, team/process, validation, retrospective, and follow-ups.
- The v0.71.0 entry in `team/release/README.md`, with any accepted candidate reports as indented entries.
- A resolution for every file under `team/roadmap/v0.71.0/`.
- Every completed or withdrawn item moved to `team/roadmap/done/`.
- A status line in each moved item linking to `[v0.71.0](../../release/release-v0.71.0.md)`. The text says `shipped` only when the item actually shipped.
- A reciprocal roadmap-closure list in the release report linking to `../roadmap/done/{item}.md`.
- No remaining `team/roadmap/v0.71.0/` directory. Any approved carryover has already moved to a later active version.
- The dated `CHANGELOG.md` section, every GA version pin and lockfile, and both Fedora spec version and changelog updates required by the release skill.

Strip `-rcN`, fast-forward `main` to this commit, then create and push the annotated `v0.71.0` tag. There is no later documentation-only close commit for v0.71.0.

Post-tag verification still checks the GitHub Release, `chan.app` metadata, Docker images, COPR, and Launchpad. Those checks confirm publication of the already closed commit; they do not mutate the tagged history. A release defect is superseded by a patch release.

## First Landing: Process Migration

Execute this as the first v0.71.0 landing, preferably one process-only commit so the path transition is atomic. The distro follow-on guard is the only release-automation behavior change admitted to this item; it enforces the existing dry-run contract.

### 1. Preflight and freeze the inventory

- Start from updated `main` on a descriptive process branch, never a branch named `v0.71.0` because that name belongs to the eventual tag.
- Require a clean worktree and record the exact `team/` and `dev/v0.71.0/` file inventory.
- Record hashes for every ignored `dev/v0.71.0/*.md` source. A clean `git status` does not prove these machine-local files are absent or unchanged.
- Check for new parallel work before every move. Do not overwrite or absorb files added after the inventory.

### 2. Move the release history

- Create `team/release/`.
- Move the current `team/README.md` to `team/release/README.md`.
- Move every current top-level `team/release-*.md` file into `team/release/` with its basename unchanged.
- Preserve rename detection and historical content. Only path-dependent references are edited.

### 3. Establish the roadmap

- Create `team/roadmap/README.md`, `team/roadmap/v0.71.0/`, and `team/roadmap/done/`.
- Copy this file and the three existing v0.71.0 analysis files from the ignored `dev/v0.71.0/` tree into `team/roadmap/v0.71.0/`, verify source and destination hashes, and add the destination files to Git. They cannot be moved with `git mv` because the sources are untracked.
- Do not reformat, summarize, or otherwise edit the technical content of the three existing analyses during the move.
- Remove the ignored source copies only after the tracked destination files and their hashes have been verified. The process commit is then their recovery point.

### 4. Replace the process front door

- Use `docs/coordination.md` as the base for the new `team/README.md`.
- Expand it with the maintainer and contributor flow described above, then remove `docs/coordination.md` rather than retaining two process sources.
- Keep the role and append-only coordination rules aligned with `.agents/playbook.md`; link to the playbook for operational detail.
- Update every inbound reference to the old coordination doc and the old release-history location.

### 5. Repair links as one graph

Moving 80 files one level deeper changes more than the release index. Audit the whole repository for:

- `team/release-*.md` and prose references such as ``team/release-v0.62.0.md``.
- Links that use `team/README.md` to mean release history.
- `docs/coordination.md`.
- Relative links inside moved reports whose targets live outside `team/release/`.
- `.agents/playbook.md`, `.agents/skills/release/SKILL.md`, and any generated team bootstrap text that names the old paths.

Run a local-only Markdown link resolver from each containing file, not from the repository root. Anchors, absolute URLs, and code spans need separate handling so the check neither misses real relative links nor treats examples as files.

### 6. Triple-check the release skill against the live tree

This is an audit, not a blind path rewrite.

1. **Source inventory:** compare the skill's branch, tag, version-pin, report, signing, artifact, and package claims against `.github/workflows/release.yml`, `.github/workflows/distros-publish.yml`, the Cargo and npm workspaces, `desktop/src-tauri/tauri.conf.json`, `Makefile`, `packaging/`, and the current lockfiles.
2. **Dry-run proof:** require the RC `publish=false` run and inspect downloaded artifact names and embedded versions. Confirm direct `.deb`/`.rpm` output, Arch's environment-derived local package version, gateway packages, desktop artifacts, signatures, and the absence of publication side effects.
3. **GA proof:** require the final tag/run checks plus `chan.app` metadata, Docker, COPR, and PPA verification. Record retry paths without describing COPR/PPA as manual release steps.

At minimum, the migration updates report paths, standardizes RC branches on the no-`v` form, removes the dead `docs/release/README.md` reference, documents Arch accurately, reconciles the workflow's RC-tag wording with the invariant that RC tags are never pushed, and hardens `distros-publish.yml` so only a successful tag-triggered Release run can start distro publication. Any other workflow behavior change requires a separate roadmap item.

### 7. Land the barrier

- Run the documentation acceptance checks below.
- Review the diff as moves plus intentional new process text, with no technical source changes.
- Land the process commit on `main` before any technical v0.71.0 merge or RC creation.
- Rebase parallel technical branches onto it and require new reports and links to use `team/roadmap/` and `team/release/`.

## Acceptance Checks

The migration is complete only when all of these hold:

- `team/` has exactly the process guide plus `roadmap/` and `release/`; no top-level `release-*.md` remains.
- `team/release/README.md` indexes every moved report, including RC sub-entries, and every index target exists.
- `team/roadmap/README.md` defines active, done, withdrawn, deferred, and GA-close semantics.
- `team/roadmap/v0.71.0/` contains the four expected proposal files and `dev/v0.71.0/` no longer contains them.
- Every repository-local Markdown link affected by the move resolves from its containing file.
- Full-repository searches find no stale `docs/coordination.md`, old `team/release-*.md` paths, or release-history references that still treat `team/README.md` as the index.
- The release skill has no dead local paths and agrees with the checked-in workflows and packaging sources.
- `git diff --check` is clean.
- `git diff --summary` shows the historical release reports as renames where content did not need a path repair. The four previously ignored v0.71.0 proposals appear as new tracked files and match their recorded source hashes.
- The migration diff contains no product runtime code or release execution. Its only release-automation behavior change is the distro guard that excludes branch dry runs.
- The process commit is an ancestor of every technical v0.71.0 commit admitted to the RC.

At GA, add these closure checks:

- No `team/roadmap/v0.71.0/` path remains.
- Every closed v0.71.0 item under `team/roadmap/done/` links to `../../release/release-v0.71.0.md` with an honest outcome.
- The release report links back to every closed roadmap item.
- The report, release index, roadmap moves, CHANGELOG, version pins, lockfiles, and Fedora spec updates are all present in the tagged GA commit.
- The pushed tag resolves to that exact commit.

## Rollback

Before technical branches adopt the new paths, the tracked migration is one process-only commit and can be reverted as a unit. If the ignored `dev/v0.71.0/` copies were already removed, restore them from the process commit before reverting it. After branches or reports depend on the layout, forward-fix path or link defects instead of moving the trees back and creating a second transition.
