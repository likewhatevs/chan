# event-desktacean-desktect.md

From: @@Desktacean
To: @@Desktect
Date: 2026-05-23

## 2026-05-23 15:10 - poke

`desktacean-1` desk audit complete; no code edits. Findings:
only active P0 is updater DEV minisign key rotation, which needs
release-owner key/secret/bridge sequencing. Report appended at
[`../desktacean/desktacean-1.md`](../desktacean/desktacean-1.md)
"2026-05-23 - @@Desktacean audit report". Verification:
`cargo test -p chan-desktop --bin chan-desktop` green, 63 passed.

## 2026-05-23 15:45 - poke

`desktacean-2` complete. Rotated `desktop/src-tauri/tauri.conf.json`
to the production updater pubkey from
`~/.tauri/chan-desktop-prod.key.pub`, updated `desktop/CLAUDE.md`
and `desktop/release-review.md`, and confirmed JSON + desktop tests
green. Old DEV key file is present for bridge signing; no private
key or password value was read or written. Report appended at
[`../desktacean/desktacean-2.md`](../desktacean/desktacean-2.md)
"2026-05-23 - @@Desktacean implementation report".

## 2026-05-23 16:35 - poke

`desktacean-3` complete. Added desktop updater bridge runbook at
`desktop/updater-bridge.md` and linked it from `desktop/CLAUDE.md`.
Verified embedded updater pubkey hash/length, old DEV key file
presence, production key file presence, Tauri signer env var names,
and current build command shape. Local bundles currently show app/DMG
only, so updater-feed assembly/publish support likely needs CI or
release-publisher work routed through @@Alex. No private key contents
or secret values were read or written. Report appended at
[`../desktacean/desktacean-3.md`](../desktacean/desktacean-3.md)
"2026-05-23 - @@Desktacean implementation report".

## 2026-05-23 16:55 - poke

`desktacean-3` review fix complete. Replaced the sample manifest
version `0.12.0` with `<bridge-version>` in
`desktop/updater-bridge.md` and documented that the bridge version
must be greater than every installed version that should discover it
through the updater. Commit readiness appended at
[`../desktacean/desktacean-3.md`](../desktacean/desktacean-3.md)
"2026-05-23 - @@Desktacean commit readiness".

## 2026-05-23 17:25 - poke

`desktacean-4` implementation ready for review. Confirmed desktop
artifact naming source is `desktop/src-tauri/tauri.conf.json`
`productName` + `version`; `desktop/Makefile` and release workflow
do not override it. Updated the Tauri config version to `0.13.0`
and documented the release metadata guardrail in `desktop/CLAUDE.md`.
Verification: JSON parse ok, derived prefix `Chan_0.13.0.*`, and
`cargo test -p chan-desktop --bin chan-desktop` passed 63 tests.
Root workspace version remains `0.11.2` and was not edited because
it is outside desktop scope. Report appended at
[`../desktacean/desktacean-4.md`](../desktacean/desktacean-4.md)
"2026-05-23 - @@Desktacean implementation report".
