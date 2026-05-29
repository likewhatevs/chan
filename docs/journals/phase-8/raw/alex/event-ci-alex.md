# event-ci-alex.md

From: @@CI
To: @@Alex
Date: 2026-05-20

## 2026-05-20 — permission (ci-8 readiness: cert + secrets state)

ci-7 (workflow YAML for Apple Developer ID signing +
notarization) is landed locally and pending @@Architect
clearance. The workflow consumes the six secrets enumerated
in the ci-3 brief at
[`../../../release/macos-signing.md`](../../../release/macos-signing.md);
missing-secret behaviour is defensive (verify-secrets step
fails fast on macOS with a named error pointing at the brief).

`ci-8` (DMG-on-tag dry-run with real keys) is the next task
in my queue, and it needs two out-of-band gates closed before
it can fire:

1. **Cert provisioning complete** per the ci-3 brief's 7-step
   checklist (Apple Developer Program enrollment, Developer ID
   Application cert issued, .p12 exported with a strong
   passphrase, app-specific password generated, optional local
   `make app-notarized` smoke test).
2. **Six secrets populated** into the chan repo's
   `Settings -> Secrets and variables -> Actions ->
   Repository secrets`:
   * `APPLE_CERTIFICATE_BASE64`
   * `APPLE_CERTIFICATE_PASSWORD`
   * `APPLE_SIGNING_IDENTITY`
   * `APPLE_TEAM_ID`
   * `APPLE_ID`
   * `APPLE_PASSWORD`
   Per the secrets-boundary memory: secret VALUES never appear
   in journals / chat / commits — populate them directly into
   the GitHub Settings page, no transcription path needed.

Two things would help me sequence ci-8 cleanly:

* **State signal**: a one-line ack on this thread (or in chat)
  saying "checklist done, secrets populated" — or "still
  pending, no ETA" — so I know whether to standby on ci-8 or
  start preparing the dry-run test-tag plan now.
* **Test-tag name preference** (optional): for the ci-8
  dry-run I was going to use `chan-v0.11.99-dryrun.1` per
  the ci-8 task spec's example. Confirms-or-redirects fine;
  anything that won't collide with the eventual v0.12.0 cut
  works.

ci-8 is **fully gated on these two items**; ci-7 lands
independently (the YAML refuses cleanly when secrets are
absent, so landing it doesn't break anything pre-population).
No urgency from my side — just flagging so the ci-8 timing
is visible.

Filing carry-on poke to @@Architect (ci-7 clearance) in
parallel.

## 2026-05-21 — approved (transcribed by @@Architect)

@@Alex 2026-05-21:

> 2. done! it worked (this was incredible, I ran and it
> worked on first try, perfeccc; thank you and team!)

@@Alex ran the [`populate-apple-secrets.sh`](../../../release/populate-apple-secrets.sh) script + confirmed all six secrets are now in GitHub Actions Repository Secrets. Both ci-8 gates clear:

* **Gate 1 — cert provisioning**: ✓ complete (Developer ID Application cert is in @@Alex's local Keychain as `Developer ID Application: Alexandre Fiori (W73XV5CK3N)`; `chan-notary` Keychain item holds the app-specific password for `fiorix@gmail.com`).
* **Gate 2 — six secrets populated**: ✓ complete (script ran clean; `gh secret list` shows all six names).

**ci-8 is unblocked.** Proceed when ready:

* **Test-tag name**: default to `chan-v0.11.99-dryrun.1` per the ci-8 task spec unless @@Alex redirects. The dry-run tag won't collide with the eventual `chan-v0.11.2` (mini-wave in flight) or `chan-v0.12.0` (Round-2 close).
* **Sequencing**: ci-8 fires AFTER the v0.11.2 patch tag (`chan-v0.11.2`) lands. The patch tag is unsigned; ci-8 validates the signed pipeline end-to-end on a separate test tag, leaving v0.12.0 as the first proper signed release.
* **Alternative**: `workflow_dispatch` against any branch/tag for an isolated test run, then a real test tag for the final dry-run. Implementer picks.

Capture metrics in the task tail per the ci-8 acceptance criteria (wall-clock breakdown, DMG artifact size, notarytool latency, failure-mode walkthrough). @@WebtestB handles the second-Mac install + double-click + Gatekeeper-clean check (standing chan-desktop runtime permission per `ada8478` covers it).

## 2026-05-21 — permission (GitHub Actions billing blocking ci-8)

Hard blocker discovered after pushing the ci-8 dry-run tag.
The workflow [run 26200703893](https://github.com/fiorix/chan/actions/runs/26200703893)
fired on `chan-v0.11.99-dryrun.1` but both jobs (ubuntu +
macos) failed in 2-3 seconds with this annotation:

> "The job was not started because recent account payments have
> failed or your spending limit needs to be increased. Please
> check the 'Billing & plans' section in your settings"

The jobs literally never executed — no checkout, no signing,
no notarize. This is a `fiorix` account billing state, not a
CI/code issue. ci-9 (verify-step patch) is unrelated.

### What needs to happen

You're the only one who can fix this — it requires logging
into GitHub Billing & plans for the `fiorix` account.
Suggested steps:

1. Open https://github.com/settings/billing.
2. Resolve any failed payment + bump the GH Actions spending
   limit (or set to unlimited if your plan allows). chan-desktop
   on macos-latest is the heavy runner — Apple Silicon minutes
   are billed at 10x the Linux rate; budget a few hundred macOS
   minutes for ci-8's dry-run + a buffer for ci-7-driven
   automatic re-runs on each `chan-v*` tag going forward.
3. Once billing is healthy, ping back here and I'll re-trigger
   the workflow via `gh run rerun 26200703893` (no new tag
   needed; same SHA, same workflow definition).

### Related pre-existing finding (not blocking; flagging)

The previous v0.11.1 tag-push run hit the same billing block
last night:

```
chan-v0.11.1   Release (chan-desktop)   push   26179438339   5s   failure
```

That means the v0.11.1 GitHub Release is missing its
chan-desktop bundles — only the chan CLI artifacts from
release.yml landed. If anyone tries to download chan-desktop
v0.11.1 from the Releases page, it isn't there. Probably
fine for the v0.11.1 dogfood window (chan-desktop wasn't
the v0.11.1 deliverable), but worth knowing. Same workflow
re-run pattern would backfill it after billing is fixed,
though there's a sequencing trade-off: re-running v0.11.1
now would produce a SIGNED chan-desktop bundle (because
ci-7 + secrets came later), which conflicts with the plan's
"v0.11.1 unsigned, v0.12.0 first signed" narrative. Best to
just leave v0.11.1 as-is and let v0.11.2 be the first to
ship chan-desktop bundles.

### Standing state on my lane

| Task | State |
|------|-------|
| ci-9 (verify-step patch) | ✓ committed (`f5b0122`) |
| ci-8 dry-run tag         | ✓ created + pushed (`chan-v0.11.99-dryrun.1`) |
| ci-8 workflow fire       | ✗ BLOCKED (billing) |
| ci-8 metrics capture     | ✗ pending workflow execution |
| @@WebtestB DMG verify    | ✗ pending the DMG existing |

Standing by on your billing fix. Will not retry, delete, or
recreate the tag — `gh run rerun` on the existing run-id
is the cleanest re-trigger once billing is healthy.
