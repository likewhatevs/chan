# ci-7: Tag-triggered signed + notarized chan-desktop workflow

Owner: @@CI
Date: 2026-05-20

## Goal

Wire up the workflow YAML in `.github/workflows/release-desktop.yml`
that consumes the six Apple signing/notarization secrets from the
`ci-3` brief and produces a notarized `.dmg` on every `chan-v*` tag
push. This is the Round-2 north-star deliverable; v0.12.0 ships on
top of this pipeline.

## Background

* **`ci-3` brief**: [`../../../release/macos-signing.md`](../../../release/macos-signing.md).
  Lists the six secret NAMES, the cert-provisioning checklist
  (@@Alex completes out-of-band), and the signing/notarization
  flow shape.
* **Round-2 plan north-star**: [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
  §"North-star through-line". Decision 1 (sequencing) puts this at
  the head of Wave-1.
* **Decision 3 LOCKED**: bundled chan binary uses PATH-first w/
  bundled fallback + version match (per round-2-plan decisions
  table). This task is the build-side; @@FullStackB's
  `fullstack-b-15` + `-b-16` produce the bundle + launch logic
  this workflow needs to package.
* **Existing `release-desktop.yml`** (from `ci-2`, refined by `ci-4`
  + `ci-5` + `ci-6`) is the unsigned-matrix scaffold. This task
  extends it with the signing + notarization steps.
* **`-s-11` (this wave)** rotates the chan-desktop signing-key
  config from the DEV identity to the release identity per
  `desktop/CLAUDE.md`'s rotation section. ci-7's notarization step
  needs that rotation landed first or runs against stale config.

## Authorization

**Authorization: yes**, this task covers edits to
`.github/workflows/release-desktop.yml` + possibly
`.github/workflows/release.yml` (if shared steps refactor
naturally). The six secret NAMES per the ci-3 brief are
authorized to appear in the workflow YAML; secret VALUES NEVER
appear in journals / chat / commits (per the secrets-boundary
memory). @@CI may proceed without further in-chat confirmation
from @@Alex.

## Acceptance criteria

* Workflow consumes the six signing/notarization secrets by name
  per the ci-3 brief (codesign identity cert + cert password +
  Apple ID + app-specific password + team ID + keychain
  passphrase — exact names per the brief).
* On a `chan-v*` tag push: builds the chan-desktop bundle, signs
  with the Developer ID cert, notarizes via `notarytool`, staples
  the ticket, uploads to the GitHub Release as a `.dmg`.
* The bundled chan binary from `fullstack-b-15` is included in
  the app bundle's Resources (or wherever -b-15 lands it); both
  the chan-desktop binary AND the bundled chan binary are signed
  with the same identity so the notarization covers both.
* Failure modes: missing secret → workflow fails with a named
  message pointing at the ci-3 brief (don't leak the secret
  value); notarization rejection → captures the notarytool log
  and uploads it as a workflow artifact so the user can
  diagnose.
* Pre-push gate (YAML-only): clean.

## How to start

1. Re-read the ci-3 brief end-to-end. Confirm the six secret
   names + the cert-provisioning state.
2. Confirm with @@Alex via permission event whether the secrets
   are already populated in GitHub Actions Secrets, or whether
   that's pending. If pending, ci-7 lands the workflow YAML
   defensively (refuses cleanly when secrets are absent) and
   @@Alex populates afterward.
3. Audit the existing `release-desktop.yml` (post-ci-2 + ci-4 +
   ci-5 + ci-6) to see what's there and what needs adding. Don't
   tear down what works.
4. Reference Tauri's official signing docs + the existing
   chan-desktop `desktop/src-tauri/tauri.conf.json` signing
   block (rotated by `-s-11`).
5. Land as a single commit (workflow YAML changes are tightly
   coupled).

## Coordination

* **Depends on `systacean-11`**: signing-key rotation in
  `tauri.conf.json` must land before ci-7's actual signing step
  works against real Apple Developer ID keys. Sequencing within
  Wave-1: @@Systacean lands `-11` first; @@CI builds ci-7 in
  parallel against stub / placeholder config; both come together
  at the dry-run gate.
* **Followed by `ci-8`**: DMG-on-tag dry-run with real keys.
  ci-7 must be in HEAD + secrets populated before ci-8 can fire.
* **@@FullStackB's `-b-15`** (bundled chan binary) feeds the
  bundle this workflow ships. Coordinate on bundle layout / file
  paths once -b-15 settles.

## Next in your queue

* `ci-8` — DMG-on-tag dry-run with real keys (cut next).

## Open questions

(populated as you investigate)
