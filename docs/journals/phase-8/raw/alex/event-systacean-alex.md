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

## 2026-05-21 — permission (systacean-12: tauri-plugin-updater dry-run on macOS)

`-12` steps 3-4 done (test minisign keypair + mock-feed JSON + Tauri config-override all scaffolded under `/tmp/chan-updater-test/`). Architect approved Option C (temporary test caller via `--check-update-now` CLI flag in `main.rs`); now need runtime time on this workstation for the actual dry-run.

### What I need to do

1. Add a `#[cfg(debug_assertions)]`-gated `--check-update-now` CLI handler to `desktop/src-tauri/src/main.rs` (~30 LOC; calls `update.check()` against the override-configured endpoint, logs the result). Removed (or feature-flag-gated) after verify completes.
2. Start a background `python3 -m http.server 8765 --directory /tmp/chan-updater-test/` (serves `latest.json` + `fake-bundle.tar.gz` + signature).
3. `cargo tauri dev --config /tmp/chan-updater-test/override.json -- --check-update-now` from `desktop/src-tauri/`. The override redirects `plugins.updater.endpoints` to `http://127.0.0.1:8765/latest.json` + replaces the pubkey with the throwaway test pubkey. Canonical `tauri.conf.json` NOT modified.
4. Observe + capture log lines: check() detects update 0.99.0, download URL hits the http.server, signature-verify pass, apply-step attempts (fake bundle WILL fail apply — boundary of pre-flight verification per the task spec).
5. Iterate the three failure modes: invalid signature, corrupted download, version downgrade attempt.

### Why I need permission

* **Runtime launch of chan-desktop** via `cargo tauri dev` — interactive Tauri window opens; not covered by @@Systacean's standing permissions today (unlike @@FullStackB / @@WebtestB who got standing grants on 2026-05-20).
* **Background http.server on port 8765** — small footprint but a foreground-running localhost process; teardown via kill.
* **Temporary edit to `desktop/src-tauri/src/main.rs`** — the file is in concurrent-agent modified state per `git status`; pre/post-commit audit + `git commit -- <pathspec>` race-safe form is the plan. Edit is reverted (or kept behind a debug-build feature flag) after verify completes.

### Duration

~30 min wall-clock. The check + download + verify pathway is fast (sub-second per Tauri's plugin internals). Most of the time is the iteration on the three failure-mode variants + capturing log lines for the findings writeup.

### Teardown

* `kill` the python3 http.server background job.
* `rm -rf /tmp/chan-updater-test/` (removes test keypair, mock manifest, fake bundle, override file).
* Revert OR feature-gate the test-caller edit in `desktop/src-tauri/src/main.rs` — commit decision per @@Architect's review.

### Linux + Windows verify

NOT in this permission. The cross-platform verify per the task body's "Verified on Linux + Windows via VM or @@Alex's secondary machines (coordinate on which environments are available)" needs a separate ask once macOS is green. Fire that when ready.

### Approval shape

Append a `## 2026-05-21 — approved` (or "approved-with-caveats" / "denied") section below. Or in-chat is fine if you'd rather just say "go" and Architect transcribes.

## 2026-05-21 — approved (transcribed by @@Architect)

@@Alex 2026-05-21 (in chat to @@Architect): "permission
granted.. please do not kill my chan.app, i am running on
it and i have a drive on our repo's source code path"

### Approved scope

The `-12` macOS dry-run goes ahead per the task body's
"Step 5+ plan":

* `#[cfg(debug_assertions)]`-gated `--check-update-now`
  CLI handler edit to `desktop/src-tauri/src/main.rs`.
* `python3 -m http.server 8765 --directory /tmp/chan-updater-test/`
  in the background.
* `cargo tauri dev --config /tmp/chan-updater-test/override.json -- --check-update-now`.
* Iterate the 3 failure modes (invalid signature,
  corrupted download, version downgrade attempt).

### Hard safety constraints from @@Alex

@@Alex's working chan.app is **alive on the workstation
right now** with a registered drive at the chan repo
source path. Operational hard rules — these are
not-negotiable for this dry-run:

1. **NEVER touch @@Alex's running chan.app.** Don't
   SIGTERM it, don't SIGKILL it, don't `pkill chan`,
   don't `pkill -f chan-desktop`, don't `kill -9 <any-PID>`
   that wasn't your own spawn.
2. **Process ownership by CAPTURE, not triage.** Capture
   the PID of the `cargo tauri dev` chan-desktop launch
   AT SPAWN; only SIGTERM that captured PID at teardown.
   No "high elapsed time so it must not be mine"
   inference — that exact failure mode surfaced in the
   @@WebtestB ci-8 dryrun.4 walkthrough (see
   [`event-architect-webtest-b.md`](event-architect-webtest-b.md)
   "Scope clarification" for the historical incident).
3. **Don't open the chan repo source drive.** The
   `--check-update-now` should fire + exit (or run to
   completion) without engaging any drive UI. If the
   dev chan-desktop process tries to present a drive
   picker, dismiss / close without selecting. If it
   auto-opens a registered drive, abort the dry-run,
   surface the unexpected behaviour to @@Alex via a
   permission event, and we re-shape.
4. **Background process discipline.** Capture the
   python3 http.server PID at spawn; SIGTERM by PID at
   teardown. No `pkill python` / no port-based kill
   heuristics.

### Teardown checklist (must be clean before close-poke)

* `kill -SIGTERM <captured-cargo-tauri-dev-PID>`; verify
  it exited; SIGKILL only if SIGTERM doesn't take.
* `kill -SIGTERM <captured-http-server-PID>`; verify
  port 8765 freed.
* `rm -rf /tmp/chan-updater-test/`.
* Revert OR feature-gate the temp edit in
  `desktop/src-tauri/src/main.rs`. Per the task body's
  shared-worktree-commits discipline: pre/post-commit
  audits (`git diff --staged --stat` + `git show --stat
  HEAD`) + explicit `git add <path>` per file.
* @@Alex's chan.app + registered drives **left
  untouched**.
* `ps aux | grep -E '(chan|python|cargo)'` shows no
  stragglers from your work.

### Linux + Windows verify

Out of scope for this permission. Fire a separate
permission event if/when you reach that step.

### Scope expiry

This permission covers the `-12` macOS dry-run +
teardown only. Out-of-scope actions (e.g. canonical
fresh-Mac walk for a separate task, Apple notary log
spelunking, sidecar process supervision changes) fire
fresh permission events.

## 2026-05-22 — permission (systacean-12: tauri-plugin-updater dry-run; re-ask post-recycle)

You poked me with "check your tasks and execute"; `-12` is the only queued item. Per the architect's framing the prior permission was session/chan.app-state-scoped + expired with the recycle. Same scope shape as the prior approval; re-asking before any runtime action.

### What I'd do

Per [`../systacean/systacean-12.md`](../systacean/systacean-12.md) step 5+ plan, unchanged from the prior ask:

1. Add a `#[cfg(debug_assertions)]`-gated `--check-update-now` CLI handler to `desktop/src-tauri/src/main.rs` (~30 LOC; calls `update.check()` against the override-configured endpoint + logs the result).
2. Start a background `python3 -m http.server 8765 --directory /tmp/chan-updater-test/` (capture the PID at spawn).
3. `cargo tauri dev --config /tmp/chan-updater-test/override.json -- --check-update-now` from `desktop/src-tauri/` (capture the chan-desktop PID at spawn).
4. Observe + capture log lines: `update.check()` detects update 0.99.0, download URL hits the http.server, signature-verify pass, apply-step attempts (fake bundle WILL fail apply — boundary of pre-flight verification per the task spec).
5. Iterate the three failure modes: invalid signature, corrupted download, version downgrade attempt.

### Scaffolding state (survives between sessions)

`/tmp/chan-updater-test/` still has the test fixtures from the prior session: `test.key` / `test.key.pub` (minisign keypair) + `latest.json` (mock feed) + `fake-bundle.tar.gz` + `.sig` + `override.json`. No re-scaffolding needed; step 5+ can proceed directly once permission lands.

### Hard safety constraints (same as prior; re-stating for the audit)

* **NEVER touch @@Alex's running chan.app**. `pgrep -af "chan|Chan.app"` currently shows 5 chan-class PIDs (39577, 39646, 41552, 44822, 44823) — some yours, some other lanes' serves. Don't `pkill`, don't SIGTERM anything I didn't spawn.
* **Process ownership by CAPTURE, not triage**. Capture the `cargo tauri dev` chan-desktop launch PID + the `python3 -m http.server` PID AT SPAWN; only SIGTERM those at teardown.
* **Don't open the chan repo source drive**. The `--check-update-now` should fire + exit (or run to completion) without engaging any drive UI. If dev chan-desktop auto-opens a registered drive, abort + surface as a fresh permission event.
* **No background processes left alive after teardown**. Both PIDs SIGTERM'd; verify with `ps`.

### Teardown checklist

* `kill -SIGTERM <captured-cargo-tauri-dev-PID>`; verify it exited; SIGKILL only if SIGTERM doesn't take.
* `kill -SIGTERM <captured-http-server-PID>`; verify port 8765 freed.
* `rm -rf /tmp/chan-updater-test/`.
* Revert OR feature-gate the `desktop/src-tauri/src/main.rs` test caller; pre/post-commit audits + explicit `git add <path>` per file.
* `pgrep -af "chan|python|cargo"` shows no stragglers from my work.

### Duration

~30 min wall-clock. Sub-second per check + download + verify cycle; most of the time is iterating the 3 failure modes + capturing log lines.

### Linux + Windows verify

NOT in this permission. Separate ask once macOS green (probably won't happen this session; Linux/Windows need VM or secondary machines).

### Approval shape

Append a `## 2026-05-22 — approved` (or "approved-with-caveats" / "denied") section below. Or in-chat is fine if you'd rather just say "go" and I'll transcribe per the process.md format.

## 2026-05-22 — approved (transcribed from in-chat @@Alex)

@@Alex 2026-05-22 in chat: "go on pick up yer task". Direct approval — proceeding with the step 5+ plan above.

Same hard safety constraints carry forward from the prior approval (NEVER touch @@Alex's running chan.app, capture PIDs at spawn, don't engage drive UI, no background processes left after teardown). Will observe + capture logs through the 3 failure-mode iterations + tear down clean. Permission scoped to macOS dry-run only; Linux/Windows out of scope.
