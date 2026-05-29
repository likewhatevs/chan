# desktacean-3 — updater bridge-release runbook

Owner: @@Desktacean
Phase: 8, Round 3
Date cut: 2026-05-23

## Goal

Draft and verify the chan-desktop updater bridge-release runbook.
The runbook must explain exactly how to ship one bridge release
that embeds the production updater pubkey but signs the updater
bundle with the old DEV private key, then switches future releases
to the production private key.

## Background

`desktacean-2` landed the production updater pubkey in
`desktop/src-tauri/tauri.conf.json` (`3c1435b`). Existing installs
still trust the old DEV updater pubkey. Therefore the first update
bundle after `3c1435b` must be signed with the old DEV private key,
otherwise existing installs cannot verify the update that carries
the new pubkey.

After users install that bridge release, their app trusts the new
production pubkey. All following update bundles must be signed with
the production private key.

This task is a runbook / verification task. It should not push tags,
modify CI secrets, or trigger production release workflows.

## Acceptance Criteria

1. Add or update a desktop-local runbook section under `./desktop`
   documenting:
   * Bridge release purpose.
   * Required local/CI env vars for old-key bridge signing.
   * Required local/CI env vars for production-key signing after
     the bridge.
   * How to tell which key signed an updater artifact.
   * Failure mode if the old DEV key is unavailable.
2. Verify the commands without exposing private key values:
   * Publicly inspect which updater pubkey is embedded in
     `desktop/src-tauri/tauri.conf.json`.
   * Confirm old DEV key file presence only, not contents.
   * Confirm production key file presence only, not contents.
   * Identify the exact `cargo tauri` / `make` command shape that
     produces updater artifacts.
3. Produce a task-tail report:
   * Runbook path.
   * Whether bridge signing can be local/manual with current files.
   * Whether CI support is required.
   * Any env var names @@Alex / @@CI need to populate, without
     values.
4. Keep all edits inside `./desktop` and this task/event journal.
   If `.github/workflows/release-desktop.yml` or secrets need
   changes, stop and poke @@Desktect.

## How To Start

1. Read:
   * `desktop/CLAUDE.md` updater section.
   * `desktop/release-review.md` P0.1.
   * `.github/workflows/release-desktop.yml` comments only, for
     env var names. Do not edit workflow YAML in this task.
2. Inspect available Tauri commands:
   * `cargo tauri signer --help`
   * `cargo tauri build --help`
3. Look for updater artifacts produced by Tauri 2 in this repo's
   build setup. Prefer documentation and dry command inspection over
   full production builds.
4. Write the runbook in `desktop/CLAUDE.md` or a new
   `desktop/updater-bridge.md`, whichever keeps the docs clearer.

## Coordination

* @@Alex owns private key / password handling.
* @@Desktect owns routing.
* @@CI remains chan-core. Any CI workflow or secret change routes
  through @@Alex first.
* Do not launch or mutate @@Alex's running chan.app session.

## 2026-05-23 - @@Desktacean implementation report

Runbook added at `desktop/updater-bridge.md`, with a pointer from
`desktop/CLAUDE.md` under the auto-upgrade signing section.

What the runbook covers:

* Bridge purpose: one release embeds the production updater pubkey
  but signs the updater payload with the old DEV private key.
* Post-bridge state: future updater payloads must be signed with the
  production private key.
* Local and CI env var names for both bridge and post-bridge signing:
  `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PATH`, and
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
* How to inspect the embedded updater pubkey without printing the full
  key, and how to verify an artifact against old DEV vs production
  public keys.
* Failure mode if the old DEV key or password is unavailable: no
  auto-update bridge; existing installs need a manual DMG install.

Verification performed without reading private key contents:

* Embedded updater pubkey inspected from
  `desktop/src-tauri/tauri.conf.json`: length `152`, sha256
  `5f789235cded09c32c091823b0d9318c99ff8376a2077d46b381cd7b0616c5e8`.
* Old DEV updater key file presence confirmed with
  `test -f ~/.tauri/chan-desktop.key`.
* Production updater key file presence confirmed with
  `test -f ~/.tauri/chan-desktop-prod.key`.
* Production updater public key file presence confirmed with
  `test -f ~/.tauri/chan-desktop-prod.key.pub`.
* `cargo tauri signer sign --help` confirms updater signing accepts:
  `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PATH`, and
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
* `cargo tauri build --help` confirms local macOS bundle shapes are
  `app` and `dmg`; no updater-specific build flag is exposed.
* Existing local bundles under `target/release/bundle` contain the
  macOS app and DMG only. No `latest.json`, detached updater signature,
  or updater payload archive was present.

Bridge signing can be local/manual with current key files, assuming
@@Alex supplies any needed key password and the release publisher
provides the actual updater payload file. I did not sign an artifact,
read private key contents, edit workflows, edit secrets, push tags, or
trigger a release.

CI support is likely required if the production release path needs to
assemble and publish updater-feed artifacts (`latest.json`, payload
URLs, signatures). That work is outside this task's desktop-only
boundary and should route through @@Desktect / @@Alex.

## 2026-05-23 - @@Desktect review feedback

Runbook direction accepted, but one correction is needed before
commit clearance:

* The sample updater manifest uses `0.12.0`. The bridge version
  must be greater than the installed version, so the example should
  use `<bridge-version>` or a future concrete version, not
  `0.12.0`.
* Keep the existing warning that the bridge version must be greater
  than the installed version.

After that correction, append a short readiness note with the
changed paths and suggested subject.

Suggested subject:

```
chan-desktop: document updater bridge-release flow (desktacean-3)
```

## 2026-05-23 - @@Desktacean commit readiness

Addressed @@Desktect review on the sample manifest. Replaced the
concrete `0.12.0` example with `<bridge-version>` and added an
explicit note that the bridge version must be greater than every
installed version that should discover it through the updater.

Changed paths:

* `desktop/updater-bridge.md`
* `desktop/CLAUDE.md`
* `docs/journals/phase-8/desktacean/desktacean-3.md`
* `docs/journals/phase-8/alex/event-desktacean-desktect.md`

Ready for commit with suggested subject:

```
chan-desktop: document updater bridge-release flow (desktacean-3)
```
