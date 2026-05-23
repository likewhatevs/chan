# Phase 8 chan-desktop journal

Author: @@Desktect
Date: 2026-05-23

Canonical chan-desktop team journal for phase 8. Carries the
desktop-team plan summary, dispatch table, decisions log, and
handoff notes.

Append-only. Corrections land as new dated entries, not rewrites.

## 2026-05-23 — bootstrap + phase-8 continuation

@@Desktect bootstrapped as the chan-desktop architect lead.
Scope inherited from the chan-core handoff in
[`../alex/event-architect-desktect.md`](../alex/event-architect-desktect.md):
`desktop/`, `desktop/src-tauri/`, chan-desktop bundling,
signing/notarization, native desktop UX, and
`release-desktop.yml`.

@@Alex confirmed option 1 from the initial posture survey:
continue under the **phase-8** banner. Phase 9 remains a later
sync beat after Round 3 closes.

### Operating shape

* Working directories mirror chan-core:
  `desktect/`, `desktacean/`, and `desktest/` under
  `docs/journals/phase-8/`.
* Event channels use the existing
  `docs/journals/phase-8/alex/event-<from>-<to>.md`
  pattern.
* Cross-team decisional traffic routes through @@Alex. Async
  lead-to-lead notes may be mirrored in
  `event-desktect-architect.md` for audit trail, but they are
  not the decision channel.
* Work outside `./desktop` is not assumed. If a desktop task
  needs `crates/`, `web/`, root docs, workspace config, or
  shared workflows beyond `release-desktop.yml`, @@Desktect
  pokes @@Alex first.

### Initial priority call

First pickup is the Round-3 chan-desktop hardening pass:
`desktacean-1`. It absorbs the handed-off items:

1. Tauri-side cleanup pass.
2. Capabilities audit + IPC review.
3. Updater verification.
4. Sanity check of the shipped orphan-sidecar prevention /
   recovery path.

Runtime Gatekeeper / fresh-DMG walks remain queued for
@@Desktest, but need explicit @@Alex permission before launch
cycles because @@Alex is running v0.12.0 and that session is
off-limits.

### Dispatch table

| Task | Owner | Scope |
|------|-------|-------|
| [`desktacean-1`](../desktacean/desktacean-1.md) | @@Desktacean | Tauri cleanup + capabilities/IPC audit + updater verification + sidecar lifecycle sanity check |

### Watching

* `ci-15` workflow audit may surface `release-desktop.yml`
  findings. If so, @@Alex bridges the fix request to this
  lane.
* `systacean-43` history audit covers `desktop/`; any
  desktop-specific leak finding routes through @@Alex.
* `architect-3` public-flip docs should be read by
  @@Desktect/@@Desktest once drafted to ensure the desktop
  release path is described accurately.

## 2026-05-23 — @@Alex: desktop boundary confirmed

@@Alex confirmed the chan-desktop team's operating boundary:

* Primary focus is **chan-desktop**.
* Do not interfere outside `./desktop`.
* If desktop work needs a change outside `./desktop`,
  @@Desktect pings @@Alex first.
* @@Alex bridges any cross-team decision or request with the
  chan-core @@Architect for now.
* The chan-core team remains active; be aware of their work,
  but do not route decisional traffic directly to them.

This reinforces the coordination-shape update in
[`../alex/event-architect-desktect.md`](../alex/event-architect-desktect.md).
Async notes to chan-core's @@Architect are fine for visibility;
decisions route through @@Alex.

## 2026-05-23 — desktacean-1 audit complete; P0 routed to @@Alex

@@Desktacean completed [`desktacean-1`](../desktacean/desktacean-1.md):
desk audit only, no code edits. Verification:
`cargo test -p chan-desktop --bin chan-desktop` passed
63 / 0.

Summary:

* Capabilities audit found no obviously dead grants.
* IPC inventory found no P0/P1 command-surface issue.
* Sidecar lifecycle prevention/recovery path from
  `fullstack-b-22` / `fullstack-b-25` matches HEAD.
* `desktop/release-review.md` is stale for several fixed
  findings and should be refreshed later.
* Active P0 is updater production minisign key rotation.

Routing:

* Updater production key rotation crosses release ownership and
  secrets handling. Routed to @@Alex in
  [`../alex/event-desktect-alex.md`](../alex/event-desktect-alex.md).
* No changes outside `./desktop` requested by this lane.
* `desktacean-1` is closed as an audit/report task.

## 2026-05-23 — updater production key generated; desktacean-2 cut

@@Alex confirmed the production updater signing key has been
generated and secret side handled out-of-band. No secret value was
shared in chat or journals.

Cut [`desktacean-2`](../desktacean/desktacean-2.md) to rotate the
public updater key in `desktop/src-tauri/tauri.conf.json` and
refresh desktop docs. Scope stays inside `./desktop`.

Open bridge detail for the task report:

* If the old DEV private key is still available, the bridge
  release can be signed with the old key while embedding the new
  pubkey.
* If not, existing installs cannot auto-update across the key
  rotation and need a manual DMG install.

## 2026-05-23 — desktacean-2 landed

Read `3c1435b` in HEAD:

```
chan-desktop: rotate updater pubkey to production key (desktacean-2)
```

Commit scope matches clearance:

* `desktop/src-tauri/tauri.conf.json`
* `desktop/CLAUDE.md`
* `desktop/release-review.md`
* `docs/journals/phase-8/desktacean/desktacean-2.md`
* `docs/journals/phase-8/alex/event-desktacean-desktect.md`

No unrelated `LICENSE`, `CONTRIBUTING.md`, or @@Desktect
bootstrap files rode along.

Desktop P0 state after this commit:

* Production updater pubkey is now configured.
* Old DEV key is reportedly present for bridge signing.
* Remaining release action is operational: cut the bridge
  release so existing installs receive the new pubkey.

## 2026-05-23 — bridge-release follow-ups cut

Cut two follow-ups:

| Task | Owner | State |
|------|-------|-------|
| [`desktacean-3`](../desktacean/desktacean-3.md) | @@Desktacean | Active: updater bridge-release runbook + command-shape verification |
| [`desktest-1`](../desktest/desktest-1.md) | @@Desktest | Held: runtime bridge walkthrough after @@Alex permission + runbook |

Boundary:

* `desktacean-3` stays inside `./desktop` and journal files.
* Any `.github/workflows/release-desktop.yml` or GitHub Actions
  secret change routes through @@Alex to chan-core @@Architect /
  @@CI.
* `desktest-1` cannot launch chan-desktop until @@Alex grants
  runtime permission.

## 2026-05-23 — desktacean-3 cleared; package-version task cut

@@Desktacean corrected the updater manifest example to use
`<bridge-version>` and explicitly note that the bridge version
must be greater than every installed version that should discover
it through the updater.

Cleared `desktacean-3` for commit with subject:

```
chan-desktop: document updater bridge-release flow (desktacean-3)
```

New bridged ask from chan-core via @@Alex:
`chan-v0.12.0` desktop artifacts were named `Chan_0.11.2.*`.
Cut [`desktacean-4`](../desktacean/desktacean-4.md) to fix
desktop-local package version metadata for the next cut.

## 2026-05-23 — desktacean-4 approved

@@Desktacean completed [`desktacean-4`](../desktacean/desktacean-4.md).

Findings:

* `Chan_0.11.2.*` artifact names came from
  `desktop/src-tauri/tauri.conf.json` `version`.
* `desktop/Makefile` and `release-desktop.yml` do not override
  the package version.
* Desktop-local fix is `tauri.conf.json` `version: "0.13.0"` for
  the next planned cut plus docs guardrail in `desktop/CLAUDE.md`.

Approved for commit:

```
chan-desktop: bump package metadata for v0.13.0 artifacts (desktacean-4)
```

Open cross-team release-cut note: chan-core still owns the workspace
version bump. At tag cut, workspace `version` must align with the
desktop Tauri version so `CARGO_PKG_VERSION`, bundled `chan`, and
the desktop version probe all agree.

## 2026-05-23 — desktacean-4 landed; desktop lane holding

Read `de83259` in HEAD:

```
chan-desktop: bump package metadata for v0.13.0 artifacts (desktacean-4)
```

Commit scope matches clearance. Desktop-local package metadata now
targets `0.13.0` for the next cut.

Desktop lane state:

* `desktacean-1`: closed audit.
* `desktacean-2`: production updater pubkey landed.
* `desktacean-3`: updater bridge runbook landed.
* `desktacean-4`: package metadata landed.
* `desktest-1`: held, needs @@Alex runtime permission and bridge
  artifact/feed path.

Open cross-team items, routed via @@Alex:

* Updater feed assembly / publish path for the bridge release.
* Release-cut workspace version bump so chan-core `CARGO_PKG_VERSION`
  aligns with desktop `tauri.conf.json` `version`.

## 2026-05-23 — chan-desktop lane wrap / teardown

@@Alex directed the chan-desktop team to wrap for now.

Desktop-team shipped / closed this session:

* `desktacean-1`: hardening audit complete; no code edits.
* `desktacean-2`: production updater pubkey landed.
* `desktacean-3`: updater bridge-release runbook landed.
* `desktacean-4`: desktop package metadata fixed for v0.13.0
  artifact names.

Held:

* `desktest-1`: updater bridge runtime walkthrough. Requires
  @@Alex runtime permission plus a bridge artifact/feed path.

Open cross-team items for next sync:

1. Updater feed assembly / publish path for bridge release:
   `latest.json`, payload URL, signature.
2. chan-core release-cut workspace version bump so root workspace
   `CARGO_PKG_VERSION`, bundled `chan`, and
   `desktop/src-tauri/tauri.conf.json` version align at `0.13.0`.
3. Runtime walkthrough permission once bridge artifacts exist.

Teardown pokes sent to @@Desktacean and @@Desktest. No desktop
runtime launches were authorized by @@Desktect in this session.

## 2026-05-23 — @@Desktect teardown-complete

@@Alex confirmed the desktop workers have torn down. @@Desktest
also posted an explicit teardown-complete poke; @@Desktacean's
latest visible event is still the `desktacean-4` report, but
@@Alex's in-chat confirmation is the source of truth for wrap.

No desktop runtime launches were authorized by @@Desktect, so
@@Desktect has no processes, throwaway drives, or GUI sessions to
clean up.

Final handoff:

* Desktop implementation work is complete for now:
  `desktacean-1` through `desktacean-4` all landed / closed.
* `desktest-1` remains held.
* Open items for next sync:
  1. Updater feed assembly / publish path for bridge release.
  2. chan-core workspace version bump at v0.13.0 cut.
  3. Runtime walkthrough permission once bridge artifacts exist.
* Boundary remains: chan-desktop team owns `./desktop`; cross-team
  release / CI / workspace changes route through @@Alex.






## 2026-05-23 — desktacean-2 approved

@@Desktacean completed [`desktacean-2`](../desktacean/desktacean-2.md).
Diff scope:

* `desktop/src-tauri/tauri.conf.json`
* `desktop/CLAUDE.md`
* `desktop/release-review.md`

Reported verification:

* `tauri.conf.json` parses as JSON.
* `cargo test -p chan-desktop --bin chan-desktop` passed
  63 / 0.

@@Desktect review found no secret values in the changed desktop
docs / task files. Approved for commit with subject:

```
chan-desktop: rotate updater pubkey to production key (desktacean-2)
```

Remaining release work is the bridge release itself. Existing
installs need one old-key-signed update bundle embedding the new
production pubkey before subsequent releases use the production
private key.
