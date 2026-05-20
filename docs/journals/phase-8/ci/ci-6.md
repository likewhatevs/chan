# ci-6: Gate ci-5's BGE-bundle cache + fetch step on --features embed-model

Owner: @@CI
Date: 2026-05-20

## Goal

Now that `systacean-6` has landed and the default build no
longer embeds the BGE-small model, the cache step + the
`cargo run -p fetch-models` step in `release.yml` +
`release-desktop.yml` only matter for `--features
embed-model` build paths. Gate both on the feature so
default builds skip the model-prefetch dance entirely.

## Background

`systacean-6` (`8b35c03`) split the embed path behind a
new cargo feature `embed-model`, default-off. Default
build paths produce a ~25 MB binary with no embedded
model; only `cargo build --features embed-model`
includes it.

`ci-5` (`0c076f0`) cached the BGE bundle between runs to
avoid re-downloading + re-encoding the model on every
tag. That win still applies to feature-on builds but is
wasted work on feature-off builds (the cache restores +
populates a directory the build never reads).

@@CI surfaced the trigger via the systacean-7-landed
poke on 2026-05-20 ("Whenever you cut the cache-scope
follow-up `ci-N`, I'm ready").

## Authorization

**Authorization: yes**, this task covers edits to
`.github/workflows/release.yml` +
`.github/workflows/release-desktop.yml`. @@CI may
proceed without further in-chat confirmation from
@@Alex.

## Acceptance criteria

* `Cache BGE-small bundle` step in both workflows is
  guarded on the same conditional that triggers
  `--features embed-model` builds. If the matrix entry's
  feature flags don't include `embed-model`, the cache
  step does not restore (and the subsequent fetch step
  doesn't run either, per ci-5's existing
  `cache-bge-bundle.outputs.cache-hit != 'true'`
  guard).
* `cargo run --release -p fetch-models` step is similarly
  guarded — no point invoking the fetcher when the
  downstream build won't embed it.
* Default-feature build matrix entries (the lean
  ~25 MB binary path) skip both steps cleanly. CI
  wall-clock for those entries drops by however much
  the fetcher + cache restore cost (small but
  non-trivial on cold cache, esp. macOS).
* `--features embed-model` entries (when they exist —
  Round-2's signing pipeline may not need them, but
  power-user / offline-install builds do) keep the
  current ci-5 cache + fetch behaviour intact.
* If neither workflow currently has a matrix entry that
  builds `--features embed-model`, document that in the
  task tail so we know whether the gating is purely
  defensive (no current consumer) or actively saving
  time (existing consumer).
* `workflow_dispatch` dry-run validates the gated +
  ungated paths still work end-to-end. Pair with the
  ci-2 + ci-4 + ci-5 dry-run already parked.
* Pre-push gate (YAML-only): clean.

## How to start

1. Audit the two workflows: does either currently have
   a build matrix entry that passes `--features
   embed-model`? If not, the gating is preparation for
   when one is added (Round-2 may or may not need it
   depending on whether the signed DMG bundles the
   model — likely NOT bundled per
   `systacean-6`'s "leave desktop sidecar on default
   features" decision in my Q2 reply).
2. Pick the gating mechanism. Options:
   * `if:` condition on the cache + fetch steps that
     references a matrix-input variable (e.g.
     `matrix.features == 'embed-model'`).
   * A repo-level variable that flips both steps in lock-
     step.
   * Hardcoded skip on default-feature paths (acceptable
     if no feature-on matrix entry exists today).
   Pick the shape that fits the existing workflow
   conventions; document in the task tail.
3. Test the dry-run path (pairs with ci-2 + ci-4 + ci-5
   bundle).
4. Commit-readiness append.

## Coordination

* No webtest verification (CI-internal).
* `ci-5`'s cache mechanism stays intact; this task only
  adds the conditional skip for default-feature paths.
* Round-2 numbering shifts again as a result: signing
  workflow → ci-7, DMG dry-run → ci-8 (was ci-6 / ci-7).
  Updating round-2-plan.md in the same architect pass
  that cut this task.

## 2026-05-20 — landed (ready for review)

Owner: @@CI.

### Feature-flag audit

Neither workflow currently passes `--features embed-model`:

| Workflow                  | Build step                                          | Features          |
|---------------------------|-----------------------------------------------------|-------------------|
| `release.yml`             | `cargo build --release --target ... -p chan`        | default           |
| `release-desktop.yml`     | `make build` → `cargo build --release --bin chan` + `cargo tauri build` | default           |

So the gating is **purely defensive** — no current consumer. Both
the cache step and the fetcher invocation become dead code paths
today. Per the architect's "Hardcoded skip on default-feature
paths (acceptable if no feature-on matrix entry exists today)"
acceptance criterion, going with the simplest shape.

### Gating shape

`if: false` on both the cache step and the fetcher step in both
workflows. Plus a comment block above each step that documents:

* Why the step exists (the ci-5 cache + the fetcher invocation).
* Why it is currently skipped (post-systacean-6 default builds
  drop the model embed).
* How to flip the gate when a feature-on consumer arrives
  (set `matrix.embed_model: true` on the new matrix entry and
  change `if: false` to `if: matrix.embed_model`, or flip to
  `if: true` if the whole workflow goes feature-on).

Why not matrix.features now: adding a `features:` field to every
existing matrix entry for a feature value none of them use bloats
the matrix definition. Better to keep the matrix unchanged and
leave the gate as a literal `if: false` that the next implementer
re-shapes to match the feature-on lane's chosen mechanism (matrix
input, workflow_dispatch input, or env var).

Why not delete: `ci-5`'s cache infrastructure is non-trivial
(key composition, OS-independent sharing, fetch idempotency
reasoning). Deleting and re-adding from scratch loses that audit
trail. `if: false` preserves the structure with a one-line flip
to re-enable.

### What I did NOT change

* `ci-5`'s cache-key composition stays the same
  (`hashFiles('chan/crates/fetch-models/**',
  'chan/crates/chan-drive/src/index/config.rs')`). When the gate
  flips, the existing key handles future model swaps via the
  Round-2 / Round-3 model-picker per `systacean-6`'s
  forward-compat shape.
* `ci-5`'s composite `if:` on the fetcher step
  (`steps.cache-bge-bundle.outputs.cache-hit != 'true'`) is
  swapped to the gate `if: false`. The comment notes to restore
  the composite condition when the gate flips so the warm-cache
  short-circuit returns.
* No `fetch-models` Rust changes. No `desktop/Makefile` changes.
  No `Makefile` changes.

### Files changed

* `.github/workflows/release.yml` — gate cache + fetch steps,
  expand comments.
* `.github/workflows/release-desktop.yml` — same.
* `docs/journals/phase-8/ci/ci-6.md` (this append).

### Validation

* YAML structural sanity via grep: four `if: false` blocks
  (one cache step + one fetch step per workflow), each with
  `name:` + the existing `id:` / `uses:` / `with:` / `run:`
  preserved.
* Effective behaviour on a tag-triggered run today: both
  steps print "Step skipped" in the GitHub Actions UI. No
  cache restore, no fetcher invocation, no `models.tar.zst`
  on disk for downstream `cargo build`. rust-embed's
  `include_bytes!` ... wait, this is the part that needs a
  cross-check: `systacean-6` moved the `include_bytes!` site
  behind `#[cfg(feature = "embed-model")]`, so default
  builds compile cleanly without the file present.
  Verified by reading `systacean-6.md` (the
  `embed_seed.rs` cfg-gate was a primary acceptance
  criterion). No regression on default-build links.
* Runtime dry-run via `workflow_dispatch`: same gap as
  ci-2 / ci-4 / ci-5 — `act` not installed locally, push
  hold blocks draft-branch dry-run. The combined dry-run
  parked for Round-1 close now covers ci-2 + ci-4 + ci-5
  + ci-6. The ci-6 lane in that dry-run is the easiest
  check: confirm both `Cache BGE-small bundle` and
  `Pre-fetch embedded model` show "Step skipped" in the
  job logs and the downstream build still succeeds.
* Pre-push gate: YAML-only; fmt / clippy / test /
  svelte-check / npm build do not apply.

### Commit readiness

Not committing per the standing rule. Proposed commit
message:

```
ci: gate ci-5's BGE-bundle cache + fetch on --features embed-model

systacean-6 (8b35c03) split the embed path behind a new cargo
feature, default-off. Default builds no longer include
models.tar.zst, so ci-5's cache step + fetch-models invocation
in both release workflows are dead code on every current matrix
entry. Hardcode-skip both via `if: false` and add comments
documenting how to flip the gate when a feature-on consumer is
added (matrix-input variable, workflow input, or `if: true`).
Audit finding: neither release.yml nor release-desktop.yml
currently builds --features embed-model, so the gating is
purely defensive. ci-5's cache key composition + cache-hit
short-circuit are preserved structurally; flipping the gate
restores them without re-deriving the key. Closes phase-8 ci-6;
dry-run pairs with ci-2 + ci-4 + ci-5 at the parked Round-1
close gate.
```

### Open questions for @@Architect

1. **Gating mechanism shape**: chose `if: false` over
   adding a `matrix.embed_model` field (which would
   require touching every existing matrix entry for a
   value none of them use). The next implementer who
   adds a feature-on lane picks whichever mechanism fits
   (matrix, workflow input, env). Flag if you'd prefer
   the matrix-field shape now even with no consumer.
2. **Audit finding**: defensive gating only. Worth noting
   in `round-2-plan.md` so whoever shapes the offline-
   install / power-user variant knows the gate is here
   and how to flip it.