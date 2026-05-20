# ci-8: DMG-on-tag dry-run with real Apple Developer ID keys

Owner: @@CI
Date: 2026-05-20

## Goal

Fire a real `chan-v*` tag against the `ci-7` workflow with the
six signing/notarization secrets populated, produce a notarized
`.dmg`, verify it opens cleanly on a second Mac (no Gatekeeper
warning), and capture the wall-clock + artifact size as the
audit baseline for the v0.12.0 cut at Round-2 close.

## Background

* This is the **Round-2 north-star verification gate**. Before
  v0.12.0 ships, the signed-DMG pipeline must have been
  exercised end-to-end with real keys behind the private repo.
  Opening the repo (Round 3) is one-way; de-risking the signed
  pipeline is the rationale @@Alex split Round 2 → 3.
* **Test tag shape**: pre-release SemVer (e.g. `chan-v0.12.0-rc1`
  or `chan-v0.11.99-dryrun.1`). Picks a tag that won't collide
  with the eventual v0.12.0 cut. @@Architect confirms the
  exact tag name when ci-8 lands.
* **Second-Mac verification**: per the ci-3 brief, the
  notarized DMG must open cleanly on a Mac that has NEVER seen
  the chan-desktop dev signing identity. @@Alex's secondary
  machine (or a fresh VM) is the test bed. NOT a security
  guarantee; just the user-facing "double-click opens, no
  warning" verification.

## Authorization

**Authorization: yes**, this task covers firing a test tag
against `release-desktop.yml` + capturing artifacts +
documenting findings. Secret VALUES still never appear in
journals / chat / commits.

## Acceptance criteria

* A real `chan-v*` test tag fires the workflow with all six
  secrets populated. Workflow completes green.
* Output artifact: a notarized `.dmg` uploaded to the test
  GitHub Release. Stapled ticket verified via
  `stapler validate <bundle>` (or equivalent).
* On a second Mac (one that has never seen the dev identity):
  double-click the DMG → mount → drag to Applications → launch
  → no Gatekeeper warning, no "unidentified developer", no
  notarization-pending prompt.
* Wall-clock metrics captured in the task tail:
  * Workflow total time.
  * Build + sign step time.
  * Notarization wait time (typically the dominant cost).
  * DMG artifact size.
* Failure-mode walkthrough: at least one intentional failure
  injection (wrong cert password, expired team ID, etc.) and
  observation of the workflow's error message — confirms the
  failure path is diagnosable from CI logs alone.
* Pre-push gate (YAML-only if you tweak the workflow): clean.

## How to start

1. Confirm with @@Alex (permission event) that the six secrets
   are populated and the cert provisioning per the ci-3 brief
   is complete.
2. Pick a test tag name (e.g. `chan-v0.11.99-dryrun.1`). Avoid
   any name that conflicts with the eventual v0.12.0 cut.
3. Push the test tag. Monitor the workflow run.
4. On success: pull the notarized DMG from the release page;
   ship it to @@Alex's second Mac for the user-flow test.
5. Document everything: wall-clock breakdown, artifact size,
   notarytool logs (redacted of secret values), any rough
   edges in the workflow.
6. Append "ci-8 dry-run complete" entry to your journal +
   commit-readiness append to this task file.

## Coordination

* **Depends on `ci-7`**: ci-7's workflow YAML must be in HEAD
  + secrets populated before ci-8 can fire.
* **Depends on `systacean-11`**: signing-key rotation lands the
  real Developer ID config; without it, the workflow signs
  against stale dev keys.
* **Feeds `systacean-12`**: cross-platform `tauri-plugin-updater`
  verification consumes the dry-run DMG as the macOS half of
  the cross-platform check.
* **Webtest verification**: lane-B owns the empirical
  "double-click + open" check on the second Mac per
  `feedback_lane_boundaries`. @@WebtestB's standing
  chan-desktop runtime permission covers this (per the
  STANDING grant in `event-webtest-b-alex.md`).

## Open questions

(populated as you investigate)
