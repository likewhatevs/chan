# desktacean-2 — updater production pubkey bridge rotation

Owner: @@Desktacean
Phase: 8, Round 3
Date cut: 2026-05-23

## Goal

Rotate chan-desktop's Tauri updater public key from the DEV
key to the newly generated production key, and update desktop
release notes so the bridge-release sequence is explicit.

## Background

`desktacean-1` found one active P0: the updater pubkey in
`desktop/src-tauri/tauri.conf.json` still trusts the DEV
minisign key. @@Alex has now generated the production updater
signing key and stored the secret side out-of-band.

Do not expose secret values. This task only needs the **public
key** from `~/.tauri/chan-desktop-prod.key.pub` (or the path
@@Alex provides). The private key and password never appear in
chat, journals, commits, or logs.

Bridge-release sequencing matters:

1. Existing installs trust the old DEV pubkey.
2. The bridge release must embed the new production pubkey but
   be signed with the old DEV private key, so existing installs
   accept it.
3. Releases after the bridge are signed with the new production
   private key.

If the old DEV private key is unavailable, note that auto-update
cannot bridge existing installs; users need a manual DMG
install for the first production-key release.

## Acceptance Criteria

1. `desktop/src-tauri/tauri.conf.json`
   `plugins.updater.pubkey` is replaced with the new production
   public key.
2. Desktop docs are updated inside `./desktop` only:
   * `desktop/CLAUDE.md` no longer says the configured updater
     pubkey is the active DEV key.
   * `desktop/release-review.md` P0.1 is marked resolved or
     reframed as "bridge release still required", not "DEV key
     currently configured".
3. No private updater key or password appears in any file.
4. Verification:
   * Confirm `tauri.conf.json` remains valid JSON.
   * Run the smallest useful desktop check. If no Rust code is
     changed, `cargo test -p chan-desktop --bin chan-desktop`
     is sufficient.
5. Append a report at this task tail with:
   * public key source path used, without secret values.
   * files changed.
   * bridge-release state: old-key bridge available or manual
     install required if old key is missing.

## How To Start

1. Read `desktop/CLAUDE.md` "Auto-upgrade signing
   (tauri-plugin-updater)".
2. Read `desktop/release-review.md` P0.1.
3. Ask @@Desktect if the production public key path is not
   available in the environment.
4. Replace only the public key field. Do not touch Apple
   Developer ID signing settings.
5. Keep edits inside `./desktop`.

## Coordination

* @@Alex owns the private key and password handling.
* @@Desktect owns bridge-release routing.
* If CI secrets or workflow changes are needed, stop and poke
  @@Desktect; @@Desktect routes through @@Alex to chan-core.
* Do not edit outside `./desktop`.

## 2026-05-23 - @@Desktacean implementation report

Rotated the desktop updater public key and refreshed the desktop
docs. No private-key material was read, written, or printed.

### Public key source

Used public key file:

```
~/.tauri/chan-desktop-prod.key.pub
```

The file exists and is 152 bytes. Only its public contents were
copied into `desktop/src-tauri/tauri.conf.json`.

### Files changed

* `desktop/src-tauri/tauri.conf.json` - replaced
  `plugins.updater.pubkey` with the production updater pubkey.
* `desktop/CLAUDE.md` - changed the updater section from
  "current key is a DEV key" to "production pubkey configured;
  bridge release required after key rotation".
* `desktop/release-review.md` - reframed P0.1 as bridge-release
  sequencing still required, not DEV pubkey still configured.

### Bridge-release state

Checked presence only, without reading secret contents:

* Old DEV key file: present at `~/.tauri/chan-desktop.key`.
* Production key file: present at `~/.tauri/chan-desktop-prod.key`.

Bridge path appears available from this workstation: the next
bridge release should embed the production pubkey from this change
but sign the update bundle with the old DEV private key so existing
installs accept it. Release owner still owns secret handling and
actual bundle signing.

If release-owner validation later shows the old DEV key cannot be
used, existing installs cannot auto-update across this rotation and
need a manual DMG install for the first production-key release.

### Secret hygiene

No private updater key or password value was added to the repo.
Docs mention secret concepts and env var names only; no secret
contents.

### Verification

Ran:

```
node -e "JSON.parse(require('fs').readFileSync('desktop/src-tauri/tauri.conf.json','utf8')); console.log('json ok')"
cargo test -p chan-desktop --bin chan-desktop
```

Results:

* `tauri.conf.json` parses as valid JSON.
* `cargo test -p chan-desktop --bin chan-desktop`: 63 passed, 0 failed.

### Commit readiness

Ready for @@Desktect review / commit coordination. Suggested
subject:

```
chan-desktop: rotate updater pubkey to production key (desktacean-2)
```

## 2026-05-23 - @@Desktect approval + commit clearance

Approved.

Review notes:

* `tauri.conf.json` changed only the updater public key.
* `desktop/CLAUDE.md` and `desktop/release-review.md` now describe
  the correct bridge-release state instead of claiming the active
  configured key is still the DEV key.
* Secret scan over the touched desktop docs / task files found
  only public key material, local key paths, env var names, and
  conceptual references to private keys/passwords. No secret value
  was written.
* Verification reported by @@Desktacean is sufficient for this
  docs/config-only change: JSON parse check + `cargo test -p
  chan-desktop --bin chan-desktop` 63 / 0.

Commit clearance:

```
chan-desktop: rotate updater pubkey to production key (desktacean-2)
```

Path-scope the commit to:

* `desktop/src-tauri/tauri.conf.json`
* `desktop/CLAUDE.md`
* `desktop/release-review.md`
* `docs/journals/phase-8/desktacean/desktacean-2.md`
* `docs/journals/phase-8/alex/event-desktacean-desktect.md`

Do not include unrelated `LICENSE`, `CONTRIBUTING.md`, or
@@Desktect bootstrap files in this commit.
