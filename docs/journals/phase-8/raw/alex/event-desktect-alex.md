# event-desktect-alex.md

From: @@Desktect
To: @@Alex
Date: 2026-05-23

## 2026-05-23 14:37 — poke

Phase posture answer received: continue under phase 8. I mirrored
chan-core's working-dir/event shape and cut first desktop pickup:
[`desktacean-1`](../desktacean/desktacean-1.md).

Default first priority is the Round-3 chan-desktop hardening pass
(Tauri cleanup + capabilities/IPC audit + updater verification).
@@Desktest stays reactive until runtime-walk permission is needed.

## 2026-05-23 14:37 — acknowledged boundary

Acknowledged: chan-desktop team focuses on `./desktop` and does
not interfere outside it. If we need a cross-team change,
@@Desktect pings @@Alex; @@Alex bridges with chan-core's
@@Architect.

I mirrored this into the desktop-team task + inbound event
channels so fresh @@Desktacean / @@Desktest sessions see it
before work starts.

## 2026-05-23 15:20 — poke

`desktacean-1` audit is complete. No code edits; desktop unit
check passed (`cargo test -p chan-desktop --bin chan-desktop`,
63 / 0).

Only active P0: updater production minisign key rotation. Current
state still uses the DEV updater key per `desktop/CLAUDE.md` /
`desktop/release-review.md`. This cannot be resolved by worker
code alone because it needs release-owner key generation,
secret storage, and bridge-release sequencing.

Recommended decision:

1. Generate production updater signing key + store the private
   key/password in the appropriate release secret path.
2. Authorize a desktop-lane follow-up to rotate the public
   updater key in `desktop/src-tauri/tauri.conf.json` and update
   desktop release docs.
3. Coordinate with chan-core @@Architect / @@CI through you if
   any GitHub Actions secret or release workflow adjustment is
   needed.

I am holding desktop workers inside `./desktop` until this is
bridged.

## 2026-05-23 16:00 — poke

`desktacean-2` landed as `3c1435b` with the cleared subject:

```
chan-desktop: rotate updater pubkey to production key (desktacean-2)
```

Commit scope was clean: desktop config/docs + the worker task/event
only. The production updater pubkey is now in HEAD.

Remaining release action is the bridge release: sign the first
update bundle after this change with the old DEV private key while
embedding the production pubkey. Subsequent release bundles should
use the new production private key.

## 2026-05-23 16:15 — poke

Cut bridge follow-ups:

* [`desktacean-3`](../desktacean/desktacean-3.md): active
  runbook + command-shape verification.
* [`desktest-1`](../desktest/desktest-1.md): held runtime
  walkthrough, pending your launch permission and the runbook.

Both preserve the boundary: desktop team stays in `./desktop`.
If CI workflow or GitHub Actions secret changes are needed, I will
route the ask back to you for chan-core @@Architect / @@CI.

## 2026-05-23 16:45 — poke

`desktacean-3` found the bridge runbook gap: local desktop builds
currently produce app / DMG bundles, but no updater feed artifacts
were found (`latest.json`, payload URL assembly, detached signature
publish path). The runbook can document manual signing, but the
actual bridge release likely needs release-publisher / CI work.

Routing ask for you to bridge to chan-core @@Architect / @@CI:

* Decide where updater feed assembly lives for chan-desktop
  releases.
* Ensure the bridge release can sign updater payloads with the old
  DEV key while embedding the production pubkey already in HEAD.
* After the bridge, switch release signing to the production updater
  key.

I have asked @@Desktacean for one small runbook correction before
commit clearance: replace the sample `0.12.0` manifest version with
`<bridge-version>` or a future concrete version because updater
versions must increase.

## 2026-05-23 17:00 — poke

`desktacean-3` correction landed and is cleared for commit.

I also picked up the chan-core bridge ask from `c83d369`: desktop
artifacts on `chan-v0.12.0` were named `Chan_0.11.2.*`. I cut
[`desktacean-4`](../desktacean/desktacean-4.md) to fix
desktop-local package version metadata for the next release. It
stays inside `./desktop`; if the real source is release workflow
logic, I will route back through @@Alex.

## 2026-05-23 17:35 — poke

`desktacean-4` confirmed the desktop artifact-name source:
`desktop/src-tauri/tauri.conf.json` `productName` + `version`.
Desktop-local fix is ready and cleared for commit: set
`tauri.conf.json` `version` to `0.13.0` and document the release
metadata guardrail.

Important release-cut sync for chan-core: root workspace
`version` is still chan-core-owned and remains outside desktop
scope. At the v0.13.0 cut, chan-core must bump the workspace version
so `env!("CARGO_PKG_VERSION")`, the bundled `chan` sidecar version,
and chan-desktop's version probe all agree.

## 2026-05-23 17:45 — poke

`desktacean-4` landed as `de83259` with clean scope:

```
chan-desktop: bump package metadata for v0.13.0 artifacts (desktacean-4)
```

Desktop-local work is now holding. Open items that need your bridge
to chan-core / @@CI:

1. Updater feed assembly / publish path for the bridge release
   (`latest.json`, payload URL, signature).
2. Release-cut workspace version bump so root workspace
   `CARGO_PKG_VERSION`, bundled `chan`, and desktop
   `tauri.conf.json` version align for `0.13.0`.

`desktest-1` remains held until you grant runtime-launch permission
and there is a bridge artifact/feed path to walk.

## 2026-05-23 18:00 — poke

Wrapping chan-desktop lane per your direction.

Sent teardown to:

* @@Desktacean via
  [`event-desktect-desktacean.md`](event-desktect-desktacean.md)
* @@Desktest via
  [`event-desktect-desktest.md`](event-desktect-desktest.md)

Desktop lane state:

* Implementation tasks complete: `desktacean-1` through
  `desktacean-4`.
* `desktest-1` remains held.
* No runtime launches were authorized by @@Desktect in this
  session.

Remaining open items are cross-team/release-path:

1. Updater feed assembly / publish path for bridge release.
2. Root workspace version bump at v0.13.0 cut.
3. Runtime walkthrough permission after bridge artifacts exist.

## 2026-05-23 18:15 — teardown-complete

@@Desktect teardown complete.

No desktop runtime launches were authorized from this lane, so no
processes, throwaway drives, or GUI sessions needed cleanup.

Final state:

* `desktacean-1` through `desktacean-4`: complete.
* `desktest-1`: held.
* @@Desktest posted teardown-complete.
* @@Alex confirmed workers are torn down.

Carry-forward items:

1. Updater feed assembly / publish path for bridge release.
2. chan-core workspace version bump at v0.13.0 cut.
3. Runtime walkthrough permission once bridge artifacts exist.






## 2026-05-23 15:10 — acknowledged

Acknowledged: production updater key generated. I cut
[`desktacean-2`](../desktacean/desktacean-2.md) for the
desktop-only public-key rotation and docs refresh.

No secret value is needed in journals or chat. The worker will use
only the public key and stay inside `./desktop`. If CI secrets or
release workflow changes are needed, I will ping @@Alex first so
you can bridge with chan-core's @@Architect / @@CI.

## 2026-05-23 15:50 — poke

`desktacean-2` completed and cleared for commit. The repo now has
the production updater pubkey in the desktop config pending the
worker's path-scoped commit. No secret values were found in the
changed desktop docs / task files.

Remaining release action: cut the bridge release correctly. Existing
installs need the first update bundle after this change to be signed
with the old DEV private key while embedding the production pubkey.
After that bridge, release bundles should be signed with the new
production private key.
