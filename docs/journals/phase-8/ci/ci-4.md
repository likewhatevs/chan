# ci-4: Swap cargo install for taiki-e/install-action in release workflows

Owner: @@CI
Date: 2026-05-20

## Goal

Replace the `cargo install <tool>` steps in
`.github/workflows/release.yml` (cargo-deb,
cargo-generate-rpm) and
`.github/workflows/release-desktop.yml` (tauri-cli) with
`taiki-e/install-action@v2`. Same shape, faster — pulls
pre-built binaries instead of compiling from source on
every release.

Cache audit research lives in
[`./journal.md`](./journal.md) under "Cache audit —
findings" (your appended audit). This task lands the F1 +
F2 wins.

## Background

Your cache audit on 2026-05-20 surfaced three candidate
optimisations:

* **F1 (high value, low risk)**: swap `cargo install
  tauri-cli` in `release-desktop.yml` for
  `taiki-e/install-action@v2`. ~6-10 min per `chan-v*`
  tag.
* **F2 (high value, low risk)**: same swap for `cargo
  install cargo-deb` + `cargo install cargo-generate-rpm`
  in `release.yml`. ~5-8 min per `v*` tag.
* **F3 (medium value, medium risk)**: cache BGE-small
  model dir. ~3-6 min per release.

F1 + F2 are this task. F3 stays parked for Round 2 (the
risk profile is different — model-dir caching has cache-
invalidation considerations the install-action swap
doesn't).

## Acceptance criteria

* `release-desktop.yml`: `cargo install tauri-cli ...`
  replaced by:

  ```yaml
  - uses: taiki-e/install-action@v2
    with:
      tool: tauri-cli@<version-pin>
  ```

  Pin the version to whatever the existing `cargo install`
  pinned (or the latest known-stable).
* `release.yml`: same swap for `cargo-deb` and
  `cargo-generate-rpm`. One step each, pinned versions.
* `workflow_dispatch` dry-run on a non-tag branch (or use
  the `act` runner locally if your standing scope covers
  it) confirms the install step still resolves the tool
  binary correctly before the tag-triggered run consumes
  it.
* No regression on the actual build / sign / package steps
  downstream of the install.
* Pre-push gate (YAML-only changes, so just the markdown +
  shellcheck side): clean.

## How to start

1. Open `.github/workflows/release-desktop.yml`. Find the
   `cargo install tauri-cli ...` step.
2. Replace with the `taiki-e/install-action@v2` block,
   matching the version pin.
3. Same in `.github/workflows/release.yml` for
   `cargo-deb` + `cargo-generate-rpm`.
4. Open a draft PR (do NOT push to main without
   Round-1-close clearance) and use `workflow_dispatch` on
   the draft branch to validate. Or run the step locally
   via `act` if available.
5. Commit-readiness append on this task file when ready.

## Coordination

* No impact on the v0.11.1 build cut path — the release
  workflows are dormant until @@Alex tags a release.
* @@Systacean's `systacean-3` (version bump + tag) runs
  the v0.11.1 release pipeline; this change lands before
  that tag fires so the savings apply immediately.
* No webtest verification needed (CI-internal).

## 2026-05-20 — landed (ready for review)

Owner: @@CI.

### What I added

Three step swaps, same shape across all of them. Each
preserves the existing `name:` and any `if:` condition,
swaps `run: cargo install ... --locked` for `uses:
taiki-e/install-action@v2` + `with: { tool: ... }`.

| File                                       | Step                       | Tool spec              |
|--------------------------------------------|----------------------------|------------------------|
| `.github/workflows/release-desktop.yml`    | Install tauri-cli          | `tauri-cli@^2`         |
| `.github/workflows/release.yml`            | Install cargo-deb          | `cargo-deb` (latest)   |
| `.github/workflows/release.yml`            | Install cargo-generate-rpm | `cargo-generate-rpm`   |

Tauri-cli pin preserved from the original `cargo install
tauri-cli --locked --version "^2"`. cargo-deb +
cargo-generate-rpm were previously unpinned (defaulted to
latest from crates.io); left at "latest" via taiki-e's
unversioned syntax to match prior behaviour exactly. If
@@Architect prefers explicit major-version pins for the
latter two (`cargo-deb@^2`, `cargo-generate-rpm@^0.16` or
similar), flag and I cut a follow-up.

Each swap includes a brief WHY comment above the step
naming the wall-clock motivation (matches CLAUDE.md
"explain WHY, not WHAT").

### Tools verified supported by taiki-e

* `tauri-cli`: in the `taiki-e/install-action` manifest
  list. Prebuilt binaries published per platform via the
  upstream Tauri release pipeline.
* `cargo-deb`: in the manifest list. Linux-only use here,
  matches the `if: matrix.deb_arch` gate.
* `cargo-generate-rpm`: in the manifest list. Linux-only.

### Validation

* YAML structural validation via grep of the swapped
  steps: all three `taiki-e/install-action@v2` blocks have
  matching `with:` + `tool:` keys, `if:` conditions
  preserved on cargo-deb + cargo-generate-rpm, downstream
  `Build .deb` / `Build .rpm` / `cargo tauri build` steps
  unchanged.
* Runtime dry-run via `workflow_dispatch`: gap. `act` is
  not installed locally and the Round-1 push hold blocks
  draft-branch push for a remote dry-run. The runtime
  validation pairs with the `release-desktop.yml`
  `workflow_dispatch` dry-run already parked for Round-1
  close alongside @@Systacean's `systacean-3` (per
  `ci-2.md` tail). The combined dry-run now exercises the
  taiki-e swap end-to-end before the first real `chan-v*`
  tag fires.
* Pre-push gate: YAML-only changes; fmt / clippy / test /
  svelte-check / npm build do not apply. Workflow files
  are not subject to shellcheck (no `run: |` blocks
  changed; the new steps are `uses:` actions).

### Files changed

* `.github/workflows/release-desktop.yml` — tauri-cli
  install swap + comment.
* `.github/workflows/release.yml` — cargo-deb +
  cargo-generate-rpm install swaps + comments.
* `docs/journals/phase-8/ci/ci-4.md` (this append).

### Commit readiness

Not committing per the standing rule. Proposed commit
message:

```
ci: swap `cargo install` for taiki-e/install-action in release workflows

`cargo install tauri-cli --locked --version "^2"` (release-desktop.yml)
and `cargo install cargo-deb --locked` + `cargo install
cargo-generate-rpm --locked` (release.yml) compiled the tools from
source on every tagged release, burning ~11-18 min combined per
chan-v* + v* tag. Swap all three for `taiki-e/install-action@v2`,
which pulls prebuilt binaries. tauri-cli version pin preserved at
^2; cargo-deb + cargo-generate-rpm preserve their prior unpinned
"latest" behaviour. Closes phase-8 ci-4; runtime dry-run pairs with
the release-desktop.yml workflow_dispatch already parked for
Round-1 close alongside systacean-3.
```

### Open questions for @@Architect

1. **Version pins for cargo-deb + cargo-generate-rpm**:
   left unpinned to match prior behaviour exactly. Want
   explicit major pins (`cargo-deb@^2`,
   `cargo-generate-rpm@^0.16`) for better reproducibility?
   Recommendation: leave as-is for this commit (matches
   prior behaviour, minimal blast radius); cut a follow-up
   if/when a major version of either tool drops something
   we depend on.
2. **Runtime dry-run sequencing**: the
   `workflow_dispatch` dry-run parked for Round-1 close
   now covers two layered changes (ci-2's
   release-desktop.yml + ci-4's install-action swap). Both
   land together at the dry-run gate. Flag if you want the
   ci-4 dry-run separated; I'd recommend keeping them
   bundled since the dry-run already burns macOS minutes.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Clean three-step swap. Preserving the tauri-cli `^2`
version pin while leaving cargo-deb + cargo-generate-rpm
at "latest" (matching their prior unpinned `cargo install`
behaviour) is the right tightening — no behavioural
change beyond the speed-up. The brief WHY comment above
each step lives by CLAUDE.md's "explain WHY not WHAT"
rule cleanly.

### Answers to your two open questions

**Q1 — version pins for cargo-deb + cargo-generate-rpm**:
leave unpinned for this commit. Recommendation matches
yours. Reasons:
* Matches prior behaviour exactly; zero behavioural diff.
* Pin churn for major-version bumps of these tools is rare
  and worth dealing with reactively (if either drops
  something we depend on, we cut a one-line follow-up).
* If we ever start seeing tag-time install failures from
  upstream regressions, the pin becomes worth it; until
  then, less maintenance.

**Q2 — runtime dry-run sequencing**: keep bundled. Same
reasoning as yours. The dry-run already burns macOS
runner minutes; splitting it doubles that cost for no
added confidence — both changes touch independent steps
in the workflow file, and the dry-run validates the
composed shape end-to-end.

**Commit clearance**: approved. Use your proposed commit
message as-is. Push waits for Round-1 close.

After commit: idle / available. Round-2 prep (`ci-5`
signing-workflow per the round-2-plan numbering shift)
parks until @@Alex completes the cert checklist from the
ci-3 brief.