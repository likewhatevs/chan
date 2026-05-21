# event-systacean-alex.md

From: @@Systacean
To: @@Alex
Date: 2026-05-20

## 2026-05-20 — permission (systacean-11: Apple Developer ID identity for tauri.conf.json)

Round-2 Wave-1 dispatched [`systacean-11`](../systacean/systacean-11.md) — rotate `desktop/src-tauri/tauri.conf.json` from the DEV signing posture to the release Developer ID identity. The task body's "How to start" step 3 directs me to fire a permission event for the exact values before editing.

### What I need from you

A single answer covering the enrollment state + the identity string. The identity NAME is a public identifier (per the task authorization clause), so it's safe to land in the repo; cert + key VALUES stay in GitHub Actions Secrets per the ci-3 brief.

**Q1. State of the [ci-3 pre-Round-2 checklist](../../../release/macos-signing.md#alexs-pre-round-2-checklist):**

* (a) Enrolled + cert generated + secrets populated — provide the full identity string and I land the rotation commit today.
* (b) Enrolled + cert generated but secrets not yet in GitHub — same answer; the JSON edit only needs the identity string. Secrets land later before `ci-7` consumes them.
* (c) Not yet enrolled — systacean-11 blocks on Apple's 24-48h enrollment review. Park until then.

**Q2. `bundle.macOS.providerShortName`** (optional ASC provider short name):

* Per the ci-3 brief, this field only matters for accounts associated with multiple ASC teams. Individual enrollment = single team = field omitted. Default: leave out.
* Override only if your account is multi-team.

### Values needed if (a) or (b)

* Full `APPLE_SIGNING_IDENTITY` string, e.g. `Developer ID Application: Alexandre Fiori (ABCD123456)`. The Team ID auto-derives from the parenthesized suffix; `desktop/Makefile:115-119` already handles that.
* (Optional) `providerShortName` only if Q2 is yes.

### Scope, ETA, teardown

* Edits: `desktop/src-tauri/tauri.conf.json` `bundle.macOS.signingIdentity` field + a new "Apple Developer ID signing" section in `desktop/CLAUDE.md` (today's `CLAUDE.md` only documents the orthogonal minisign updater key).
* No keychain / secret VALUE ever touches this commit. Local build still works post-edit; local signing is expected to fail without the cert in the workstation keychain (documented in the new CLAUDE.md section as the local-vs-CI behaviour split).
* Pre-push gate (JSON + Markdown only): clean expected. <30min wall-clock from your reply.
* No teardown needed — pure config edit, no runtime processes spawned.

### Parking + parallel work

While waiting on your reply, picking up [`systacean-12`](../systacean/systacean-12.md) in parallel (tauri-plugin-updater cross-platform verification — independent of -11, no Apple-side dependency). If -12 hits a step that needs your hands-on time on Linux/Windows, I fire a separate permission event.

## 2026-05-21 — approved (transcribed by @@Architect)

@@Alex 2026-05-21 directed @@Architect to fetch the identity from their local Keychain via `security find-identity -v -p codesigning`. Two valid identities found; @@Alex confirmed the recent Developer ID Application one is the right answer (the Apple Development cert is for dev builds, not distribution; the 2013-era cert @@Alex remembered is already pruned from the keychain).

### Approved values

* **Q1 branch (a) — Enrolled + cert generated + secrets populated** (secrets-population is in flight on @@Alex's machine 2026-05-21; tracked in [`../architect/round-2-open-questions.md`](../architect/round-2-open-questions.md) §B.2).
* **`APPLE_SIGNING_IDENTITY`**: `Developer ID Application: Alexandre Fiori (W73XV5CK3N)`
* **Team ID auto-derives**: `W73XV5CK3N` (parenthesized suffix). The Makefile derivation at `desktop/Makefile:115-119` picks this up automatically; CI sets `APPLE_TEAM_ID` env explicitly to the same value (defensive against silent breakage).
* **Q2 — `providerShortName`**: leave OUT. @@Alex's account is single-team (Individual enrollment, single ASC team).

### What `-11` lands

Single field in `desktop/src-tauri/tauri.conf.json`:

```json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "Developer ID Application: Alexandre Fiori (W73XV5CK3N)"
    }
  }
}
```

Per the [ci-3 brief](../../../release/macos-signing.md), the identity NAME is a public identifier — safe to land in the repo. The cert + private key VALUES stay in @@Alex's Keychain (for local dev) + GitHub Actions Secrets (for CI), never in the JSON config.

Plus the new "Apple Developer ID signing" section in `desktop/CLAUDE.md` per the task spec.

### Proceed

@@Systacean lands the JSON rotation commit on the next inbound poll. Commit subject per the task body's pattern + commit-readiness append at the tail of [`../systacean/systacean-11.md`](../systacean/systacean-11.md).
