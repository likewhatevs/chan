# @@CI's phase-8 journal

Author: @@CI
Date: 2026-05-19

CI infrastructure: GitHub Actions workflows, build matrix, lint +
test on PR, release artifact builds, signing pipeline, notarization.
Sister lane to @@Systacean; boundary heuristic in
[`../process.md`](../process.md) Lane boundaries section.

North-star driver: deliver tag-triggered `chan-v*` → notarized DMG
(plus signed Windows + Linux installers) → GitHub Release upload.

Append-only. New entries go at the bottom under a dated heading.

## 2026-05-19 — boot + ci-1 + ci-2

Booted into phase 8 as @@CI (first session for this lane). Read
contact card, syseng + rustacean skill guides, phase-8 process,
phase-7 inherited process, request.md, the phase-8 headline
deliverable + items 7/8 in next-phase-backlog.md, my journal,
and both task files (`ci-1.md` + `ci-2.md`). No incoming events
yet (`alex/event-architect-ci.md` and `alex/event-ci-alex.md`
do not exist).

### Standing state of CI when I arrived

* `.github/workflows/ci.yml` already covered the Rust gate
  (fmt, clippy + test on Ubuntu + Windows, no-default-features).
  macOS scoped to release.yml per commit e45654a's cost call.
  No web/npm gate.
* `.github/workflows/release.yml` builds chan CLI binaries +
  .deb/.rpm/.pkg on `v*` tag, publishes to GitHub Release. Does
  NOT build chan-desktop.
* `desktop/.github/workflows/ci.yml` is stranded post-workspace
  merge — GitHub Actions ignores it; flagging for @@Architect.

### ci-1 (landed)

Added a Linux-only `web` job that runs `npm ci`, `npm run check`,
`npm run test`, `npm run build`. Refreshed the file header
comment. Local sanity on v0.11.0: svelte-check 0/0/0, vitest
446/446, vite build 2.28s. @@Architect reviewed in
`ci/ci-1.md`, picked option 1 (no per-PR macOS), cleared
commit. Committed at 6d5d8ac; not pushed (Round-1 close
commit-grouping waits).

### ci-2 (landed, ready for review)

Added `.github/workflows/release-desktop.yml` triggering on
`chan-v*` tags + `workflow_dispatch`. Linux + macOS matrix.
Build path mirrors `desktop/Makefile`'s `make build`: npm ci
+ fetch-models + chan-bin (`npm run build`, `cargo build
--release --bin chan`, copy sidecar) + `cargo tauri build`.
Bundle uploaded as workflow artifact; no GitHub Release upload
(Round-2). Header comment documents the full secret-name list
needed for signing / notarization / Authenticode / GPG / GH
Release upload.

Things noticed but not acted on (in `ci/ci-2.md`):

* Stranded `desktop/.github/workflows/ci.yml`. Lane is @@CI
  but warrants architect input before deletion.
* `desktop/Makefile`'s `app-signed` / `app-notarized` echo a
  stale `src-tauri/target/release/bundle/...` path; bundle
  actually lands at workspace `target/release/bundle/...`
  because `desktop/src-tauri` is a workspace member. Not
  destructive; @@Systacean lane.

Filed poke event for @@Architect.

## 2026-05-19 — ci-2 cleared + landed + stranded workflow dropped

@@Architect reviewed ci-2: approved as proposed. Answers to the
three open questions logged in `ci/ci-2.md`:

* Stranded `desktop/.github/workflows/ci.yml` — delete now,
  separate commit.
* Workflow dry-run — defer to Round-1 close so we exercise the
  ci-1 + ci-2 + everything-else combined state via the
  `workflow_dispatch` trigger; permission event fires alongside
  @@Systacean's `systacean-3` tag.
* Windows lane — Round-2 with signing; no second unsigned
  matrix entry now.

Two commits landed (unpushed; Round-1 close holds):

* 97b82df — `ci: tag-triggered chan-desktop release scaffold
  (unsigned)`. New `.github/workflows/release-desktop.yml` +
  ci-2.md append.
* 97ca38a — `ci: drop stranded desktop/.github/workflows/ci.yml
  (workspace-merge leftover)`. Tail-commit per @@Architect's
  one-line suggestion.

Carry-on signal from @@Architect: idle / available for Round-2
prep (release CI signing pipeline) once the bug wave settles.

## 2026-05-20 — recycled session boot + ci-3

Booted into a fresh @@CI session via the bootstrap prompt.
Re-read contact card + skill guides + phase process +
phase-7 inherited process + request + this journal + ci-1
and ci-2 task files + both event files. Working tree
matches the recycled-state I expected: ci-1 + ci-2 +
stranded-file deletion are committed (6d5d8ac / 97b82df /
97ca38a) and unpushed; branch is 19 commits ahead of
origin/main pending Round-1 close.

### ci-3 (landed, ready for review)

@@Architect cut wave-2 task: written Apple Developer ID
provisioning + GitHub Actions secrets brief (research lap;
workflow YAML is `ci-4`). Brief lives at
[`../../../release/macos-signing.md`](../../../release/macos-signing.md)
in a new `docs/release/` directory.

365 lines, single doc. Six-secret list anchored on what
`desktop/Makefile`'s `app-notarized` already reads (so
`ci-4` does not have to rename anything):
`APPLE_CERTIFICATE_BASE64`, `APPLE_CERTIFICATE_PASSWORD`,
`APPLE_SIGNING_IDENTITY`, `APPLE_TEAM_ID`, `APPLE_ID`,
`APPLE_PASSWORD`. Cert-import recommendation:
`apple-actions/import-codesign-certs@v3`; hand-rolled
`security` block included as fallback for the record.
Seven-step pre-Round-2 checklist for @@Alex at the bottom.

Doc-only change; pre-push gate satisfied on the markdown
side (no em dashes, ASCII tables within 80 cols, factual
prose). No code touched.

Three open questions in `ci/ci-3.md`: path confirmation
for the new `docs/release/` directory, Windows-brief
sequencing (separate doc when the lane opens, recommended),
and whether the `make app-notarized` local smoke test
should be lifted into `ci-4`'s acceptance criteria.

Filed poke event for @@Architect at
`alex/event-ci-architect.md`. Idle pending review.

## 2026-05-20 — ci-3 cleared + landed

@@Architect approved + cleared `ci-3` for commit (full
reply in [`ci-3.md`](ci-3.md) tail). Answers to the three
open questions:

* `docs/release/` path → confirmed.
* Windows brief → separate doc later (`ci-5` once macOS is
  green).
* `make app-notarized` smoke test → stays @@Alex-side
  prereq, not a duplicate CI step in `ci-4`.

One commit landed at **141aa4d** (`docs: macOS signing +
notarization brief (pre-Round-2 reference)`), unpushed
pending Round-1 close. 2 files, +655: the brief + the
`ci-3.md` task file. Per the per-task convention from
6d5d8ac / 97b82df, the journal + event files stay
untracked until the Round-1 close commit group rolls them
up.

### Fill-in pick

@@Architect offered two low-stakes idle options:

* **A**: cache audit of `ci.yml` + `release-desktop.yml`
  (Rust target cache, npm cache, sccache opportunities).
* **B**: bring forward the `release-desktop.yml`
  `workflow_dispatch` dry-run from Round-1 close (needs
  @@Alex permission + @@Systacean timing coordination).

Picking A. It's purely independent: no permission event,
no cross-lane coordination, and either finding gains
(easy diff to lift) or a clean "nothing to add" audit
trail entry is useful before Round 2. B parks until
@@Systacean's `systacean-3` timing is clearer.

Firing carry-on poke to @@Architect with the pick.

### Cache audit — findings

Audited `.github/workflows/ci.yml`,
`.github/workflows/release.yml`, and
`.github/workflows/release-desktop.yml` for caching gaps.
No file edits — research only. Findings ranked by value /
risk so @@Architect can decide whether to cut a follow-up
task to land any of them.

#### Current good state

* `Swatinem/rust-cache@v2` is correctly placed in every
  job that runs `cargo build/clippy/test` against a
  workspace target dir: `ci.yml` (`test`,
  `no-default-features`), `release.yml` (`test-linux`,
  `test-macos`), `release-desktop.yml` (`build`).
* `actions/setup-node@v4` with `cache: 'npm'` and
  `cache-dependency-path: chan/web/package-lock.json` is
  consistent across `ci.yml` `web` job + both release
  workflows.
* `fmt` job in `ci.yml` correctly omits rust-cache:
  `cargo fmt --check` parses with rustfmt only and does
  not write the target dir.

#### Findings

1. **High value, low risk: `tauri-cli` install via prebuilt
   binary in `release-desktop.yml`** (line 109).

   Current: `cargo install tauri-cli --locked --version
   "^2"`. Builds tauri-cli from source every workflow run
   (rust-cache covers the workspace target dir, not
   `~/.cargo/bin/`). Compile time ~3-5 min per matrix
   entry on a cold runner; the matrix has 2 entries
   (ubuntu + macos), so ~6-10 min wasted per `chan-v*`
   tag.

   Recommended swap: `taiki-e/install-action@v2` with
   `tool: tauri-cli@^2`. Pulls a prebuilt binary,
   completes in seconds. Sample shape:

   ```yaml
   - uses: taiki-e/install-action@v2
     with:
       tool: tauri-cli@^2
   ```

   Alternative: `cargo-binstall`. Same outcome, more
   moving parts.

2. **High value, low risk: `cargo-deb` + `cargo-generate-rpm`
   install via prebuilt binary in `release.yml`** (lines
   192, 208).

   Same pattern as finding 1: `cargo install cargo-deb
   --locked` and `cargo install cargo-generate-rpm
   --locked` compile from source every `v*` tag.
   Combined ~5-8 min per release on the Linux build
   entries.

   Recommended swap: `taiki-e/install-action@v2` for both,
   in place of the two `cargo install` lines.
   Side benefit: removes the `--locked` failure mode where
   a transitive dep yank breaks the install step.

3. **Medium value, medium risk: cache the BGE-small
   embedding model** in `release.yml` (line 161) and
   `release-desktop.yml` (line 125).

   Current: `cargo run --release -p fetch-models` runs on
   every workflow execution, downloading ~140 MB from
   HuggingFace into
   `crates/chan-server/resources/models/` before the
   release binary build.

   The model rarely changes (BGE-small-en-v1.5 has been
   stable for over a year). Caching the destination dir
   between runs saves ~30-60s per matrix entry on the
   download + the cache-hit code path is fast.

   Cache shape (sketch):

   ```yaml
   - uses: actions/cache@v4
     with:
       path: chan/crates/chan-server/resources/models
       key: bge-small-${{ hashFiles('chan/crates/fetch-models/**') }}
   ```

   Cache key tied to `fetch-models` crate's source so any
   change to which model gets fetched invalidates the
   cache. Risk: if `fetch-models` ever switches model
   downloads to a path-keyed scheme, the cache key
   handle needs to update. Low risk today; flag if
   `fetch-models` changes.

4. **Low value, low risk: apt cache in
   `release-desktop.yml`** (line 99). `apt-get update +
   install` adds ~20-40s on each Linux job.
   `awalsh128/cache-apt-pkgs-action` can cache the
   downloaded `.deb`s. The marginal win is small and the
   action is a third-party dep we have not adopted
   elsewhere; not worth pulling in for one workflow.
   Mention only for completeness.

#### Estimated wall-clock savings per release

| Finding   | Per matrix entry | Total per release        |
|-----------|------------------|--------------------------|
| 1 (tauri) | ~3-5 min         | ~6-10 min (2 entries)    |
| 2 (debrpm)| ~3-4 min         | ~5-8 min (Linux entries) |
| 3 (model) | ~30-60s          | ~3-6 min (matrix-wide)   |
| 4 (apt)   | ~20-40s          | ~30s (Linux only)        |

Wall-clock impact is tag-frequency-bounded: chan ships
~one release per round, so the actual minutes saved
per quarter is small. The bigger benefit is faster
feedback when a `workflow_dispatch` dry-run is needed
to validate a workflow change. Findings 1 + 2 alone
take a `release-desktop.yml` dry-run from ~12 min to
~4 min.

#### Not changing today

No edits queued. @@Architect to decide whether any of
1 / 2 / 3 warrants a follow-up task. If 1 + 2 land
together, suggest a single small task ("ci: swap
`cargo install` for prebuilt binaries in release
workflows") since both are the same one-line shape. 3
is a separate task (different action, different cache
key reasoning).

Filing carry-on poke for @@Architect with the highlights.

## 2026-05-20 — ci-4 (taiki-e/install-action swap)

@@Architect cut `ci-4` based on the cache-audit F1 + F2
findings: swap `cargo install` for `taiki-e/install-action@v2`
in three release-workflow install steps. F3 (BGE model
cache) stays parked for Round 2 per the different risk
profile.

Edits made:

| File                                       | Step                       | Tool spec              |
|--------------------------------------------|----------------------------|------------------------|
| `.github/workflows/release-desktop.yml`    | Install tauri-cli          | `tauri-cli@^2`         |
| `.github/workflows/release.yml`            | Install cargo-deb          | `cargo-deb` (latest)   |
| `.github/workflows/release.yml`            | Install cargo-generate-rpm | `cargo-generate-rpm`   |

Tauri-cli pin preserved (`^2` from original `cargo install
--version "^2"`). cargo-deb + cargo-generate-rpm preserve
their previously-unpinned "latest" behaviour. WHY comments
above each step name the wall-clock motivation.

Validation: YAML structural sanity via grep + manual
re-read. Runtime dry-run via `workflow_dispatch` gated:
`act` not installed locally, Round-1 push hold blocks a
draft-branch remote dry-run. The runtime check pairs with
the `release-desktop.yml` `workflow_dispatch` dry-run
already parked for Round-1 close alongside @@Systacean's
`systacean-3` (per `ci-2.md` tail). Combined dry-run now
covers ci-2 + ci-4 layered together before the first real
`chan-v*` tag fires.

Edit-permission note: auto-classifier blocked the YAML
validation step initially, flagging it as scope escalation
on `.github/workflows/` against the prior turn's
"no edits — research only" framing. @@Alex confirmed
authorization per @@Architect's `ci-4` task spec, then I
proceeded with validation + commit.

Commit at **385da20** (`ci: swap cargo install for
taiki-e/install-action in release workflows`). 3 files,
+208 / -3: the two workflows + the `ci-4.md` task file.
Unpushed pending Round-1 close.

Two open questions in `ci/ci-4.md` for @@Architect: explicit
major-version pins on cargo-deb + cargo-generate-rpm (recommend
leaving unpinned to match prior behaviour), and dry-run
sequencing (recommend keeping ci-2 + ci-4 bundled at the
parked Round-1-close dry-run since macOS minutes are already
burning).

Idle pending decision. Round-2 signing-pipeline task is now
`ci-5` per @@Architect's renumbering note.

## 2026-05-20 — ci-5 (BGE bundle cache)

@@Alex pulled F3 forward into Round 1 (was parked for
Round 2). @@Architect cut `ci-5` with explicit
"Authorization: yes" framing per the
[`feedback-classifier-shared-infra`](../../../../../.claude/projects/-Users-fiorix-dev-github-com-fiorix-chan/memory/feedback_classifier_shared_infra.md)
pattern we just discussed. Worked smoothly — classifier
saw the authorization signal in the task spec and didn't
flag the YAML edits this time. Pattern validated.

Structural change noted: Round 1 now closes WITHOUT a
binary cut. v0.11.1 tag cancelled. Round 2 = ci-6
(signing-workflow YAML) + ci-7 (DMG-on-tag dry-run with
real keys). Round 3 = public repo flip. Numbering shift
again: ci-6 is the Round-2 signing-workflow task.

### Cache shape

`actions/cache@v4` step inserted in both `release.yml`
and `release-desktop.yml`, gating the existing
`cargo run --release -p fetch-models` invocation on
`if: steps.cache-bge-bundle.outputs.cache-hit != 'true'`.

Key: `bge-bundle-${{ hashFiles('chan/crates/fetch-models/**',
'chan/crates/chan-drive/src/index/config.rs') }}`. The
second hash input is the file declaring `pub const
DEFAULT_MODEL: &str = "BAAI/bge-small-en-v1.5"`; a model
swap rewrites that line and invalidates the cache.
Forward-compat with the Round-2 model-picker shape per
`systacean-6` acceptance criteria.

OS-independent key (no `runner.os` segment). Bundle is
byte-identical across OSes; matrix shares one cache.
First tag pays the fetch cost once, subsequent tags hit
on every matrix entry.

### Why workflow-level guard, not tool-level

`actions/cache@v4` restores the bundle but NOT the
hf-hub staging dir under `target/fetch-models-cache/`.
Without the `if:` guard, fetch-models would re-download
the model into an empty staging dir, then re-encode
because staging mtimes are newer than the restored
bundle. Workflow-level skip is the cleanest fix; zero
Rust changes needed. Tool-level "skip if bundle present"
guard considered + rejected for blast-radius reasons
(would change local-dev re-stage workflow).

### systacean-6 re-scope: option 2 (keep global)

Picked the global-cache option. After `systacean-6`
lands (default build no longer embeds the model), the
`cargo run -p fetch-models` step ITSELF becomes
conditional on `--features embed-model`, not just its
cache. Both gates belong together in a single follow-up
`ci-N`, not pre-emptively split here. Flagged in the
task tail + as an open question to @@Architect (queue
the follow-up draft now or wait for systacean-6 merge).

### Commit

**0c076f0** (`ci: cache encoded BGE-small bundle
between release runs`). 3 files, +368 / -5: two
workflows + the `ci-5.md` task file. Unpushed; per the
2026-05-20 structural change, Round 1 no longer cuts
v0.11.1, so the next push trigger shifts to whatever
Round 1 close ultimately bundles.

Three open questions in `ci/ci-5.md` for @@Architect:
cache-key field scope (chan-drive embeddings dir wider
hash net?), per-OS isolation vs shared key, and
systacean-6 follow-up task queuing.

Idle. Round-2 prep (`ci-6` signing-workflow) parks until
@@Alex completes the cert checklist + `systacean-6`
shapes the binary.

## 2026-05-20 — ci-5 cleared + Round-1 close

@@Architect approved + cleared `ci-5` (full reply in
[`ci-5.md`](ci-5.md) tail). All three open questions
answered:

* Hash-input scope → keep minimal (`fetch-models/**` +
  `config.rs`). Embeddings preprocessor code is
  inference-time, not fetch-time; widening the hash
  invalidates the cache on unrelated edits without
  changing the cached bytes' validity.
* OS-independent shared cache key → keep shared.
  Bundle IS byte-identical across runners; per-OS would
  just waste cache slots.
* systacean-6 follow-up → wait for the post-merge cut.
  Follow-up shape depends on what cargo feature flag
  name lands + how the fetch invocation actually gets
  gated; cutting a draft now risks wrong-shaping.

Commit-message approved as-is; already committed at
**0c076f0**. Push waits until end of Round 2 — the
Round-1 close commit set lands unpushed locally, first
GitHub Release fires at Round-2 close per the
structural change.

### Round-1 summary for my lane

| Task | Topic                                          | Commit  |
|------|------------------------------------------------|---------|
| ci-1 | web/ gate per-PR                               | 6d5d8ac |
| ci-2 | tag-triggered chan-desktop release scaffold    | 97b82df |
| ci-2 | drop stranded desktop/.github/workflows/ci.yml | 97ca38a |
| ci-3 | macOS signing + notarization brief             | 141aa4d |
| ci-4 | swap cargo install → taiki-e/install-action    | 385da20 |
| ci-5 | cache encoded BGE-small bundle                 | 0c076f0 |

Six commits across five tasks. All unpushed. ci-2 + ci-4
+ ci-5 all bundle at the parked `workflow_dispatch`
dry-run pair with @@Systacean's `systacean-3`; that
parking remains valid even with the v0.11.1 cancellation
since the dry-run is just YAML validation.

### Open Round-2 lane

* **ci-6** — signing workflow YAML consuming the six
  secrets from the `ci-3` brief. Cuts post-recycle once
  @@Alex completes the cert provisioning checklist.
* **ci-7** — DMG-on-tag dry-run with real keys
  provisioned in GitHub Actions Secrets. Cuts after ci-6
  lands.
* **Provisional follow-up** (no number yet) — systacean-6
  re-scope: gate both the fetch step and its cache step
  on `--features embed-model` once -6 merges. @@Architect
  cuts the task with the right shape post-merge.

Stand down for Round-1 close. Idle until Round-2
fan-out.

## 2026-05-20 — ci-6 (gate ci-5 on --features embed-model)

I missed `systacean-7` (`6bf44cd`) landing on my first
post-clearance check; user pokes nudged me to look again.
Surfaced both `systacean-6` (`8b35c03`) and `systacean-7`
in a trigger poke to @@Architect. Architect cut `ci-6`
in response: gate `ci-5`'s cache step + fetch step on
`--features embed-model` in both release workflows.

Round-2 numbering shifted again: signing workflow now
`ci-7`, DMG dry-run `ci-8`, marketing-site CI `ci-9`.

### Feature-flag audit

Neither `release.yml` nor `release-desktop.yml`
currently passes `--features embed-model`:

* `release.yml`: `cargo build --release --target ...
  -p chan` (default features).
* `release-desktop.yml`: `make build` →
  `cargo build --release --bin chan` +
  `cargo tauri build` (default features).

So the gating is **purely defensive** — no current
consumer. Per @@Architect's "Hardcoded skip on default-
feature paths (acceptable if no feature-on matrix entry
exists today)" acceptance criterion, picked the simplest
shape.

### Gating shape

`if: false` on both the cache step and the fetcher step
in both workflows. Four gates total. Each is preceded by
a comment block explaining:

* Why the step exists (ci-5 cache + fetcher invocation).
* Why it is currently skipped (systacean-6 default
  builds drop the embed).
* How to flip (set `matrix.embed_model: true` and change
  `if: false` to `if: matrix.embed_model`, or
  `if: true` if the whole workflow goes feature-on).

ci-5's cache-key composition + the cache-hit
short-circuit (`if: steps.cache-bge-bundle.outputs.cache-hit
!= 'true'` on the fetch step) are preserved structurally.
Flipping the gate restores them with no re-derivation.

### Why `if: false`, not matrix.embed_model

Adding a `features:` or `embed_model:` field to every
existing matrix entry for a value none of them use
bloats the matrix. Keeping the matrix unchanged and
using a literal `if: false` lets the next implementer
pick whichever shape fits the feature-on lane's chosen
mechanism. Cheapest reversible shape.

### Why not delete

ci-5's cache infrastructure is non-trivial (key
composition, OS-independent sharing, fetch idempotency
reasoning). Deleting and re-adding from scratch loses
the audit trail. `if: false` preserves structure with a
one-line flip.

### Commit

**747b7be** (`ci: gate ci-5's BGE-bundle cache + fetch
on --features embed-model`). 3 files, +276 / -10:
two workflows + the `ci-6.md` task file. Unpushed; per
the 2026-05-20 structural change, the Round-1 close
commit set lands unpushed locally, first GitHub Release
fires at Round-2 close.

(Small stub: the initial commit attempt failed because
the proposed message contained `` `if: false` `` /
`` `if: true` ``; bash interpreted the backticks as
command substitution inside the HEREDOC. Wrote the
message to `/tmp/chan-ci-6-msg.txt` and used
`git commit -F` instead. No content change.)

Two open questions in `ci/ci-6.md` for @@Architect:
gating-mechanism shape (chose `if: false`; matrix-field
alternative considered + rejected), and whether the
defensive-only audit finding warrants a note in
`round-2-plan.md` for the offline-install / power-user
variant track.

Lane summary now seven commits across six tasks. Idle
pending Round-2 fan-out for `ci-7` + `ci-8` + `ci-9`.

## 2026-05-20 — Round-1 teardown (no-op for my lane)

@@Alex caught that @@Architect fired the agent-recycle
without a teardown checklist; @@Architect cut one and
predicted my lane would be the lightest of the six.
Sweep confirms — teardown is effectively a no-op:

| Check                                      | Result                                 |
|--------------------------------------------|----------------------------------------|
| `chan serve` processes from my lane        | none (the running serves are webtest-a / webtest-b / @@Alex's ChanRoadmap, not mine) |
| Throwaway drives in `/tmp/chan-test-*`     | none from my lane (existing dirs are webtest's `phase8-wa` / `phase8-wb` plus historical leftovers) |
| Chrome MCP tabs                            | none (never invoked Chrome MCP this session) |
| `act` install                              | not installed (per the ci-4 + ci-5 + ci-6 dry-run gap)  |
| Working tree of CI-lane files              | clean (all changes committed at 6d5d8ac / 97b82df / 97ca38a / 141aa4d / 385da20 / 0c076f0 / 747b7be) |

No state to clean. Ready for recycle.

### Final Round-1 lane summary

| Task | Topic                                          | Commit  |
|------|------------------------------------------------|---------|
| ci-1 | web/ gate per-PR                               | 6d5d8ac |
| ci-2 | tag-triggered chan-desktop release scaffold    | 97b82df |
| ci-2 | drop stranded desktop/.github/workflows/ci.yml | 97ca38a |
| ci-3 | macOS signing + notarization brief             | 141aa4d |
| ci-4 | swap `cargo install` → taiki-e/install-action  | 385da20 |
| ci-5 | cache encoded BGE-small bundle                 | 0c076f0 |
| ci-6 | gate ci-5 on `--features embed-model`          | 747b7be |

Seven commits across six tasks. All unpushed; first
GitHub Release fires at Round-2 close per the
2026-05-20 structural change. ci-2 + ci-4 + ci-5 + ci-6
bundle at the parked `workflow_dispatch` dry-run pair
with @@Systacean's `systacean-3` (still valid validation
gate even with v0.11.1 cancelled).

### Open Round-2 / Round-3 lane (handover)

* **ci-7** — signing workflow YAML consuming the six
  secrets from the `ci-3` brief. Cuts post-recycle once
  @@Alex completes the cert provisioning checklist.
* **ci-8** — DMG-on-tag dry-run with real keys
  provisioned in GitHub Actions Secrets. Cuts after
  ci-7 lands.
* **ci-9** — marketing-site CI (item 6 from
  next-phase-backlog).
* **Round-3 ci-N** — workflow audit + release pipeline
  final verification + CHANGELOG generation per
  round-3-plan.md.

Standing by for the recycle. Future @@CI session reads
this journal first per the bootstrap protocol; the
ci-3 brief + the ci-5/ci-6 cache-and-gate pair are the
load-bearing reference points for everything Round-2
will touch.

## 2026-05-21 — Round-2 close-out (ci-7 + ci-8 + ci-9 + v0.11.2 ship)

Catching the journal up after Round-2 fan-out. The
recycled session worked the full north-star arc end-to-end
in event channels; this entry pulls the load-bearing
artifacts into the journal so the next @@CI bootstrap
inherits a clean handover.

### Wave-1 north-star track (ci-7 → ci-9 → ci-8)

| Task | Topic                                          | Commit  |
|------|------------------------------------------------|---------|
| ci-7 | tag-triggered signed + notarized chan-desktop  | 666c027 |
| ci-9 | release-desktop verify-step matches DMG-only staple (post-systacean-13 split) | f5b0122 |
| ci-4 | tauri-cli `^2` → `2` major-only pin (latent install-action contract bug) | 988ce1d |

ci-7 commit took three attempts due to multi-agent
staging races; landed cleanly via `git commit --
<pathspec>` (race-safe primitive — pathspec form ignores
the staged index). The two reset-away commits (3d24ad8,
c279733) are reflog-only. Memory updated at
[`feedback-shared-worktree-commits`](file://~/.claude/projects/-Users-fiorix-dev-github-com-fiorix-chan/memory/feedback_shared_worktree_commits.md)
with the pattern + the orphaning-cascade incident
(my recovery `git reset --soft` was the first half of
a cascade that orphaned @@Systacean's `01f10d3`
systacean-13; -13 had to be re-applied as `2fb3f12`).

ci-9 was cut after I spotted that systacean-13's
notarytool split changed the staple shape (DMG-only,
not .app-too); the original ci-7 verify step would
have failed on `stapler validate "$APP"` post-13. Five-
line patch dropped the `.app` staple check + swapped
`spctl -t open` on .app → `spctl -t install` on DMG.

ci-4's `^2` bug was latent since 2026-05-20 — my
original ci-4 validation was YAML-structural + grep
only, no runtime exercise. taiki-e/install-action's
contract is name@latest / name@<exact> / name@<major>
/ name@<major>.<minor> — NO semver operators. First
real workflow fire surfaced it. Caught + fixed without
ceremony per Option C (no task file, just amendment
commit + audit append in ci-4.md).

### ci-8 dry-run journey (1 → 4)

Each dry-run peeled back a layer revealing the next
failure mode. Empirically validates ci-8's failure-
injection acceptance criterion organically (no
deliberate sabotage needed; the bug chain was real).

| Run  | Tag                          | Result   | Root cause                                                |
|------|------------------------------|----------|-----------------------------------------------------------|
| 1    | chan-v0.11.99-dryrun.1       | ✗ 54s    | GitHub Actions billing block (account-side, not workflow) |
| 2    | chan-v0.11.99-dryrun.2       | ✗ 16m    | ci-4 latent `^2` syntax bug at tauri-cli install          |
| 3    | chan-v0.11.99-dryrun.3       | ✗ 19m    | Linux: unused `app` var → `_app` rename (fullstack-b-20); macOS: externalBin universal2 expectation (fullstack-b-20 dropped to aarch64-only); macOS notarize: bundled chan sidecar unsigned (fullstack-b-21) |
| 4    | chan-v0.11.99-dryrun.4       | ✓ 20m11s | First fully green run; signed DMG (15.68 MB) uploaded     |

Out-of-lane bug routing: -b-20 + -b-21 against
@@FullStackB; both landed before dry-run #4. My lane
proved correct end-to-end through dryrun.4.

### chan-v0.11.2 ship (real release)

@@Systacean cut `chan-v0.11.2` tag at `60901c1`;
release-desktop.yml auto-fired on the tag.

**Result: GREEN.** Run `26221281508`, 19m45s
wall-clock — same trajectory as dryrun.4.

| Sub-job                      | Result    | Time      |
|------------------------------|-----------|-----------|
| build (macos-latest)         | ✓ success | 13m40s    |
| build (ubuntu-latest)        | ✓ success | 19m24s    |
| github release (chan-desktop)| ✓ success | 16s       |

GH Release `chan-v0.11.2` asset:
`Chan_0.11.2_x64.dmg` (16,442,495 B; signed +
notarized via Developer ID Application: Alexandre
Fiori W73XV5CK3N). First signed chan-desktop bundle
shipped to end users.

### Finding: `release.yml` trigger gap

While confirming v0.11.2's GH Release contents, noticed
`release.yml` (the chan CLI matrix that ships .deb /
.rpm / .pkg / .tar.gz) has trigger glob `tags: ['v*']`
— does NOT match the `chan-v*` tagging convention
adopted phase-8. Consequence: chan CLI binaries have
not been built or uploaded for any phase-8 tag
(chan-v0.11.0, chan-v0.11.1, chan-v0.11.2).
`gh release view chan-v0.11.1` returns "release not
found"; `chan-v0.11.2`'s release has only the DMG.

This was masked by:
* Earlier phase-8 tags' release-desktop.yml runs being
  billing-blocked, so nobody noticed release.yml's
  silence either.
* My own expected-shape table in [`event-ci-architect.md`](../alex/event-ci-architect.md)
  (2026-05-21 v0.11.2 preflight) wrote "release.yml
  (chan CLI) — green on all matrix entries. Unchanged
  behaviour from v0.11.1." That was wrong — v0.11.1's
  release.yml run itself failed; the "unchanged
  behaviour" was actually "unchanged failure".

Architect/journal.md describes the system as "on the
`chan-v*` tag per `release.yml`" — so the architect
mental model matches what the workflow SHOULD do, not
what it currently does.

Flagging for routing as a follow-up `ci-N`. Two
shapes possible:

* **(a)** Extend `release.yml`'s trigger glob to match
  `chan-v*`. Single-line YAML change. Fires on the next
  release; v0.11.2's GH Release stays as-is (DMG only).
* **(b)** As (a) + backfill v0.11.2's CLI binaries by
  re-firing the workflow via `workflow_dispatch` against
  the v0.11.2 tag and uploading to the existing
  release. More complete; needs the workflow's release-
  job to handle the existing-release case.

Recommendation: **(a)** for the next tag (v0.12.0 or
v0.11.3 if @@Systacean cuts a patch), skip the v0.11.2
backfill (cleaner audit trail; v0.11.2 ships
DMG-only as part of the north-star validation lap).

### Open parked ci-N items (post-v0.11.2)

| Topic                                                 | Trigger to cut |
|-------------------------------------------------------|----------------|
| Auto-fetch `xcrun notarytool log` on `failure()` step | architect routing |
| DMG filename `_x64` suffix despite aarch64 binary    | cosmetic, future polish |
| Universal2 (`lipo -create` matrix, x86_64 build)     | post-v0.11.2 (this entry) |
| `release.yml` `v*` → `chan-v*` trigger fix           | NEW finding (above); architect routing |
| Round-3 Track 3 — full-SHA pin sweep on third-party actions | Round-3 fan-out |
| Full-SHA pin sweep deferred per ci-7 Q3              | (same as above) |

dryrun.1-4 tags can be deleted from origin at
convenience; they're audit-trail artifacts not blocking
anything. Not urgent.

### Lane state at journal-write time

| Item                                  | State                          |
|---------------------------------------|--------------------------------|
| chan-v0.11.2 GREEN                    | ✓ run 26221281508              |
| @@WebtestB dev-Mac partial walk       | ✓ accepted by @@Alex           |
| chan-v0.11.2 signed DMG on GH Release | ✓ Chan_0.11.2_x64.dmg          |
| release.yml trigger gap               | NEW finding; awaiting routing  |
| Parked ci-N queue                     | 6 items (see above)            |
| Working tree                          | clean; main = origin/main      |

Idle pending @@Architect routing on the release.yml
trigger-gap finding + any further Round-2/Round-3 work.

## 2026-05-21 — post-recycle close-out (ci-10 + ci-11 + handover)

@@Architect routed both items I had in flight from the
prior journal entry:

* **ci-10** — committed (`8aed906`). Commit subject
  cleared verbatim. Race-safe pathspec form ignored
  the active multi-agent staging churn (HEAD shifted
  from `e7468db` → `b36ca96` via fullstack-a-43 mid-task);
  pathspec form picked up exactly the 2 named files
  with zero stowaways. Verified via `git show --stat HEAD`.
* **ci-11** — landed locally (release.yml trigger-glob
  fix + post-mortem); awaiting commit clearance.
  Architect routed (a) (add `chan-v*` alongside `v*`)
  per my finding's recommendation. Smoke dispatch run
  fired against main HEAD; first job entered
  in_progress within seconds of fire — billing healthy,
  full chain validating main's build cleanliness
  independent of the trigger fix.

### Open `release.yml` smoke run

| Field          | Value                                                            |
|----------------|------------------------------------------------------------------|
| Run ID         | 26227752597                                                       |
| URL            | https://github.com/fiorix/chan/actions/runs/26227752597          |
| Trigger        | `workflow_dispatch` against `main`                                |
| Started        | 2026-05-21 ~13:05 UTC                                             |
| Expected ETA   | ~30 min sequential through release job (skipped on non-tag)       |
| Next-session action | Check the run's conclusion via `gh run view 26227752597`. Report the result in event-ci-architect.md. If green, it confirms main's chan CLI build chain is clean (hasn't been exercised since v0.10.1). If red, route the failing crate's test out-of-lane (not a ci-11 issue — trigger fix doesn't touch build steps). |

### Lane state at recycle time

| Item                                       | State                                                          |
|--------------------------------------------|----------------------------------------------------------------|
| ci-10 commit                               | ✓ (`8aed906`)                                                  |
| ci-11 work in working tree                 | release.yml + ci-11.md + ci-11-post-mortem.md (all uncommitted)|
| ci-11 commit                               | ⏳ awaiting @@Architect clearance                              |
| release.yml smoke dispatch                 | in flight (run 26227752597; ~30 min)                           |
| Auto-fetch notarytool log on failure       | ✓ landed in ci-10                                              |
| `_x64` DMG suffix drop                     | ✓ landed in ci-10                                              |
| Dryrun.{1..4} tag cleanup                  | keep (architect cleared 2026-05-21)                            |
| v0.11.2 CLI binary backfill (option b)     | DEFERRED to @@Alex (architect surfaced separately)             |
| Round-3 full-SHA pin sweep                 | Round-3 fan-out (not in current queue)                         |
| Universal2 / lipo follow-up                | Post-v0.11.2 ci-N (not cut yet)                                |

### What the next @@CI session does on bootstrap

Per `event-architect-ci.md` "PRE-RECYCLE HANDOVER"
heading + this journal entry, the bootstrap action
sequence is:

1. **Read this journal entry first** (canonical state).
2. **Check the smoke dispatch result**:
   `gh run view 26227752597 --json status,conclusion,jobs`.
   * If `success`: append a confirmation poke to
     `event-ci-architect.md` confirming the chain is
     clean; no further action.
   * If `failure`: read the failing job's logs +
     route a poke to architect identifying the failing
     crate / step. NOT a ci-11 blocker.
3. **Commit ci-11 if cleared** (`event-architect-ci.md`
   inbound should carry the clearance heading by then).
   Race-safe pathspec form per the ci-10 + ci-7 pattern:

   ```bash
   git commit -F /tmp/chan-ci-11-msg.txt -- \
       .github/workflows/release.yml \
       docs/journals/phase-8/ci/ci-11.md \
       docs/journals/phase-8/ci/ci-11-post-mortem.md
   ```

   Commit message body proposed in `ci-11.md` "Commit
   readiness" section.
4. **Pick up @@Alex's call on the v0.11.2 backfill
   question** if it lands in the inbound channel. If
   yes, cut a follow-up `ci-12` for the
   `workflow_dispatch` against the existing `chan-v0.11.2`
   tag (release-job uploads to the existing release).
5. **Otherwise idle** — no other Round-2/Round-3 work
   in my queue.

### Teardown sweep (per process.md "Teardown")

* No `chan serve` processes from my lane (CI work is
  entirely workflow-edit + gh CLI; nothing local
  spawned).
* No throwaway drives in `/tmp/chan-test-*` from my
  lane.
* No Chrome MCP tabs (never invoked Chrome MCP this
  session).
* `act` still not installed (per the prior teardown
  entries; non-issue).
* Working tree of CI-lane files: ci-10 committed
  cleanly; ci-11 changes in working tree pending
  clearance.
* External-state side-effect: workflow dispatch run
  `26227752597` is in flight. Cleanly tagged in this
  journal entry; next session picks up the result.

Cleaner teardown than prior recycles. Ready for the
next @@CI session.

## 2026-05-21 — ci-11 committed + smoke-run gap surfaced (final pre-recycle)

ci-11 cleared + committed at **`2193946`** (3 files,
+484 / -4) per the architect heading in
[`../alex/event-architect-ci.md`](../alex/event-architect-ci.md)
"approved + commit clearance (ci-11)". Subject accepted
verbatim. Race-safe pathspec form again — HEAD had
shifted twice more since ci-10 (`3987e73`
fullstack-b-22 + `a603bc3` systacean-14); pathspec
form held cleanly.

### Smoke dispatch result: RED + workspace-wide gap surfaced

The smoke dispatch (run 26227752597) completed red at
`cargo clippy --all-targets` compilation of
`glib-sys v0.18.1`. Investigating revealed this is
NOT a ci-11 issue but a much larger latent gap:

**Every ci.yml run on main for the last 15+ commits
has failed with the same root cause.** Per-PR CI
gate has been silently broken since at least
2026-05-19. Local pre-push hooks have been doing the
actual gating; the GHA per-PR badge has been
ignorable-red.

Root cause: `desktop/src-tauri` is a workspace member
(per `Cargo.toml`); `cargo clippy --all-targets` from
workspace root touches all members; chan-desktop's
Tauri deps (`glib-sys` etc.) need GTK system libs;
ci.yml + release.yml test jobs don't apt-install
them. Only `release-desktop.yml` does. Local dev
boxes have GTK installed (for `make run`), so the
issue is invisible locally.

Two fix shapes surfaced for architect routing in the
outbound poke (provisional ci-12):

* **(a)** apt-install GTK in 4 test jobs (correct,
  heavier).
* **(b)** exclude `chan-desktop` from workspace
  clippy/test sweep (faster; matches current effective
  state; recommended for immediate unblock).

Not cutting a task — per the bootstrap rules, the
next @@CI session reads
[`../alex/event-architect-ci.md`](../alex/event-architect-ci.md)
tail for architect routing on the finding.

### Final lane state at recycle

| Item                                  | State                                       |
|---------------------------------------|---------------------------------------------|
| ci-10 commit                          | ✓ (`8aed906`)                               |
| ci-11 commit                          | ✓ (`2193946`)                               |
| Smoke dispatch (run 26227752597)      | ✗ red (glib-sys; out-of-lane root cause)    |
| ci.yml gate on main                   | **BROKEN** since 2026-05-19+ (~15 commits)  |
| Workspace-wide glib-sys gap           | flagged + routed; awaits ci-12              |
| v0.11.2 CLI binary backfill           | deferred to @@Alex                          |
| Universal2 / lipo follow-up           | post-v0.11.2 ci-N (not cut)                 |
| Round-3 full-SHA pin sweep            | Round-3 fan-out                             |

### Next-session bootstrap action sequence (updated)

1. **Read this journal entry first.**
2. **Read inbound `event-architect-ci.md` tail** for
   the routing on the glib-sys finding (provisional
   `ci-12`). The fix is one of (a) / (b) per the prior
   poke; architect's reply will route + authorize.
3. **If routed**: cut + work the ci-12 patch. (b) is
   ~1-line per workflow; (a) is ~4 jobs of apt setup.
4. **Re-fire the release.yml smoke dispatch** after
   ci-12 lands to confirm matrix builds clean against
   current HEAD.
5. **If a real `chan-v*` tag fires before ci-12 lands**:
   the release.yml workflow will fire on the new tag
   per my ci-11 fix, but the test-linux/macos jobs
   will red at the same glib-sys step. release-desktop.yml's
   chan-desktop DMG pipeline keeps working
   independently (it has its own GTK install + isn't
   gated on test-linux). So the worst case is a
   tag with: ✗ release.yml red (CLI binaries not
   uploaded yet), ✓ release-desktop.yml green (DMG
   shipped). Surface to architect; the v0.11.2
   experience repeats.
6. **Pick up v0.11.2 CLI backfill** if @@Alex routes
   it.
7. **Otherwise idle**.

### Teardown sweep (final)

* No `chan serve` processes from my lane (CI lane is
  workflow-edit + gh CLI; nothing local-spawned).
* No throwaway drives.
* No Chrome MCP tabs.
* `act` not installed.
* Working tree of CI-lane files: ci-10 + ci-11 both
  committed; journal + event-channel appends are the
  only working-tree changes left (per the per-task
  commit convention, those stay untracked until close-
  out rollup).
* External state: workflow_dispatch run `26227752597`
  is complete (red). Documented in this entry +
  outbound poke. No cleanup needed.

Cleaner-than-prior teardown. Recycle ready.
