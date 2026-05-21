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

## 2026-05-20 — blocked on permission event to @@Alex

Fresh @@Systacean session picked this task up on Round-2 Wave-1 dispatch. Read `desktop/CLAUDE.md` (minisign-only today — Apple Developer ID has no doc home yet) + the [ci-3 brief](../../../release/macos-signing.md) + current `desktop/src-tauri/tauri.conf.json` (no `bundle.macOS.signingIdentity` set today; only `minimumSystemVersion: "11.0"`).

Per step 3 of "How to start", fired the permission event for the release identity string + Team ID + providerShortName decision:

* [`../alex/event-systacean-alex.md`](../alex/event-systacean-alex.md) 2026-05-20 — permission

Three branches:

* (a) Enrolled + cert generated + secrets populated → identity string lands today.
* (b) Enrolled + cert generated, secrets not in GitHub yet → same; JSON only needs the identity name.
* (c) Not yet enrolled → task parks 24-48h on Apple's review.

Picking up [`systacean-12`](systacean-12.md) in parallel (independent — different key family). Will return to `-11` the moment the permission resolves.

## 2026-05-21 — identity string approved + implementation landing

@@Alex's reply transcribed by @@Architect into [`../alex/event-systacean-alex.md`](../alex/event-systacean-alex.md) 2026-05-21:

* **Q1 branch (a)**: enrolled + cert generated + secrets-population in flight (tracked separately in `round-2-open-questions.md` B.2 for `ci-8`).
* **`APPLE_SIGNING_IDENTITY`**: `Developer ID Application: Alexandre Fiori (W73XV5CK3N)`.
* **Q2**: `providerShortName` OUT (Individual enrollment, single ASC team).

Per the transcription, pre-authorized to land the JSON rotation commit on next inbound poll.

### Changes

* **`desktop/src-tauri/tauri.conf.json`**: single field added to `bundle.macOS`:

  ```json
  "macOS": {
    "minimumSystemVersion": "11.0",
    "signingIdentity": "Developer ID Application: Alexandre Fiori (W73XV5CK3N)"
  }
  ```

  Identity NAME is a public identifier per the ci-3 brief; cert + private key VALUES stay in @@Alex's Keychain (for local) + GitHub Actions Secrets (for CI) per the populate-apple-secrets.sh helper from `01b103d`. `providerShortName` omitted per Q2 above.

* **`desktop/CLAUDE.md`**: new "## Apple Developer ID signing" section inserted BETWEEN the existing "Bundled chan sidecar" and "Auto-upgrade signing (tauri-plugin-updater)" sections. Reading order: bundled artifact (what gets built) → Developer ID signing (-11; how the .app is signed by Apple) → minisign updater key (existing; how updater bundles are signed) → Local notarization setup (-13; how the local Makefile authenticates with Apple).

  Section content: identity-field pointer + per-secret table reference to the ci-3 brief; explicit local-build-vs-CI-build behaviour split (covers the "sign-prereqs fails without the cert in keychain" acceptance criterion from the task body); rotation procedure (5-year expiry + cert revoke/replace) referencing `populate-apple-secrets.sh` for the secrets refresh + a `security delete-certificate` snippet for the Keychain cleanup. No bridge release needed for Developer ID rotation (contrast with the minisign updater key whose rotation DOES need a bridge release).

### Validation

* `python3 -m json.tool < desktop/src-tauri/tauri.conf.json > /dev/null` — JSON parses clean.
* `cd desktop/src-tauri && cargo check --offline` — green in 2.22s. tauri-build's config-schema validation accepts the `signingIdentity` field.
* No release identity VALUES (cert blob, private key, keychain passphrase, app-specific password) appear in the commit or repo. Only the NAME ("Developer ID Application: Alexandre Fiori (W73XV5CK3N)"), which is a public identifier.
* Pre-push gate scope per acceptance: JSON + Markdown only. No Rust / Web changes; cargo check is the schema sanity check.

### Commit readiness

Diff scope (this commit):

```
desktop/src-tauri/tauri.conf.json                     | +2/-1
desktop/CLAUDE.md                                     | new section
docs/journals/phase-8/systacean/systacean-11.md       | this append
docs/journals/phase-8/systacean/journal.md            | journal append
docs/journals/phase-8/alex/event-systacean-architect.md | architect poke
docs/journals/phase-8/alex/event-systacean-alex.md    | permission event (rides naturally)
```

Pre-commit `git diff --staged --stat` audit before commit; post-commit `git show --stat HEAD` audit after. Per-file explicit `git add` — no `add -A`/`-p` needed since other agents' modifications are on disjoint paths from mine for this commit.

Suggested commit subject:

```
chan-desktop: pin Developer ID Application signing identity (systacean-11)
```

Push held per the Round-2 / v0.11.2 policy.

## 2026-05-21 — committed as b12b787

Landed:

```
chan-desktop: pin Developer ID Application signing identity (systacean-11)
```

5 files (`desktop/CLAUDE.md` +72, `desktop/src-tauri/tauri.conf.json` +2/-1, `systacean-11.md` +73, `journal.md` +21, `event-systacean-architect.md` +39), +207/-1 total. The `event-systacean-alex.md` (the permission ask + @@Architect's transcribed approval) was already committed in @@Architect's `01b103d` v0.11.2 mini-wave commit, so it didn't need re-staging here.

Pre/post-commit audits clean. No stowaways from concurrent agents (fb-15/fb-16's src files + fullstack-a-* tasks all stayed unstaged).

Push held per the Round-2 / v0.11.2 policy.

`-11` task closed. The `ci-7` workflow change (`666c027`) already consumes `make app-notarized`; it now signs against the pinned `signingIdentity` value the moment a `ci-8` real-keys dry-run runs.
