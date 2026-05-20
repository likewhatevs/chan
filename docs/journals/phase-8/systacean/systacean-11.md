# systacean-11: chan-desktop signing-key rotation (DEV → release)

Owner: @@Systacean
Date: 2026-05-20

## Goal

Rotate the chan-desktop signing-key configuration in
`desktop/src-tauri/tauri.conf.json` from the development
identity (used during phase-7 / phase-8 dogfooding) to the
release Developer ID identity. This is a config-file rotation,
not a key generation — @@Alex provides the release identity
name + the team ID via the ci-3 brief checklist.

## Background

* **`desktop/CLAUDE.md`**: contains the canonical signing-key
  rotation procedure for chan-desktop. Read it first; the
  rotation pattern is established.
* **`ci-3` brief**: [`../../../release/macos-signing.md`](../../../release/macos-signing.md).
  Documents the six secret NAMES + the release identity shape.
  This task consumes the identity-name secret; the keychain
  passphrase + cert blob are CI-side concerns owned by `ci-7`.
* **Round-2 plan**: [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
  §"North-star through-line". This task is the Wave-1 step that
  unblocks ci-7's signing step.
* **Today's state**: `desktop/src-tauri/tauri.conf.json` carries
  the DEV identity from phase-7. Building chan-desktop locally
  works; signing for a notarized DMG would fail because the dev
  identity isn't trusted by Apple's notarization service.

## Authorization

**Authorization: yes**, this task covers edits to
`desktop/src-tauri/tauri.conf.json` (+ any `desktop/CLAUDE.md`
docs updates if the rotation procedure needs sharpening). The
release identity NAME is authorized to appear in the JSON
config (it's a public identifier, not a secret); the
keychain-stored cert + key VALUES stay out of the repo entirely
(handled CI-side via the secrets). @@Systacean may proceed
without further in-chat confirmation from @@Alex.

## Acceptance criteria

* `tauri.conf.json` `bundle.macOS.signingIdentity` field
  references the release Developer ID identity name (per the
  ci-3 brief).
* `bundle.macOS.providerShortName` (if used) populated with the
  team's ASC provider short name (per the brief).
* `desktop/CLAUDE.md`'s rotation section (or wherever the
  rotation procedure lives) reflects the post-rotation state +
  documents the rollback path if needed.
* Local build (`cargo build -p chan-desktop` or `make app`)
  still works. Local signing will fail if the keychain doesn't
  have the release cert — that's expected and not a regression;
  document the local-build vs CI-build behaviour split.
* Pre-push gate (JSON-only + Markdown): clean.
* No release identity values (cert blob, private key, keychain
  passphrase) appear anywhere in the commit / repo / journal.

## How to start

1. Read `desktop/CLAUDE.md`'s signing section end-to-end.
2. Read the ci-3 brief's "Identity rotation" section.
3. Confirm with @@Alex (permission event) the exact release
   identity NAME + team ID + any short-name conventions.
4. Edit `tauri.conf.json` minimally — just the identity
   field(s).
5. Spot-check the desktop crate compiles + the JSON parses
   (Tauri's config schema validation runs on build).
6. Append commit-readiness to the task tail.

## Coordination

* **Feeds `ci-7`**: `ci-7`'s workflow signs against this
  rotated identity. ci-7 builds in parallel against the
  pre-rotation config; once `-11` is in HEAD + secrets are
  populated, ci-7 can sign for real.
* **Independent of `systacean-12`**: tauri-plugin-updater
  cross-platform verify is a separate concern; either order
  works.

## Next in your queue

* `systacean-12` — tauri-plugin-updater cross-platform
  verification (item 7 prereq).

## Open questions

(populated as you investigate)
