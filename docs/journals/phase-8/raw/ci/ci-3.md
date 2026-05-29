# ci-3: Apple Developer ID provisioning + GitHub Actions secrets brief

Owner: @@CI
Date: 2026-05-20

## Goal

Produce a written brief (single markdown doc under
`docs/release/` or similar) covering the steps to obtain an
Apple Developer ID Application certificate, export the
signing material safely, store it in GitHub Actions Secrets,
and outline the notarization flow that the Round-2 release
workflow will consume. No workflow YAML in this task —
that's `ci-4` (Round 2). This is the research lap so we
know what credentials @@Alex needs to provision before
Round-2 signing work starts.

## Background

The capacity proposal at the top of phase 8
([`../architect/journal.md`](../architect/journal.md)) listed
"Apple Developer ID provisioning + secrets handling research"
as part of @@CI's Round-1 scope. `ci-1` and `ci-2` covered
the GitHub Actions scaffold + tag-triggered chan-desktop
release placeholder; this task closes out the research
portion so Round 2 can move straight to implementation.

North star: a notarized macOS `.dmg` shipped via tag-
triggered CI without Gatekeeper friction. Signing material
is the only thing the existing tag-triggered workflow
(`97b82df`, `.github/workflows/release-desktop.yml`) is
currently missing.

## Acceptance criteria

* A markdown brief checked in under `docs/release/` (suggest
  `docs/release/macos-signing.md`; pick the path that fits
  the existing docs tree — propose to @@Architect if no
  obvious slot exists).
* Brief covers:
  * Apple Developer Program enrolment (individual vs
    organization tradeoff for chan; Alex is solo so
    individual is likely correct).
  * Apple Developer ID Application certificate generation
    (CSR via Keychain Access; download the certificate;
    export `.p12`).
  * App-specific password generation for `notarytool` (vs
    Apple ID password; the recommended path).
  * Notarization team ID + bundle ID requirements.
  * GitHub Actions Secrets shape: list of secret names the
    Round-2 workflow will consume, with one-line
    descriptions of what each holds (e.g.
    `APPLE_DEVELOPER_ID_APPLICATION_CERT_P12`,
    `APPLE_DEVELOPER_ID_APPLICATION_CERT_PASSWORD`,
    `APPLE_NOTARYTOOL_APPLE_ID`,
    `APPLE_NOTARYTOOL_TEAM_ID`,
    `APPLE_NOTARYTOOL_APP_SPECIFIC_PASSWORD` — verify the
    exact set against the `tauri-action` /
    `apple-actions/import-codesign-certs` ecosystem).
  * Recommended GitHub Actions packaging for the cert
    import (e.g., the `apple-actions/import-codesign-certs`
    action, or a manual `security import` step in the
    runner; one preferred path).
  * A short "what @@Alex does before Round 2 starts"
    checklist with the chronological order of cert
    generation → secrets entry.
  * Brief mention of the parallel Windows signing path
    (EV cert vs OV cert tradeoffs; not exhaustive — note
    that it lands as a separate doc when we tackle the
    Windows lane in Round 2).
* No actual secret values committed (this is research +
  placeholder names only).
* Factual prose; no marketing language. ASCII tables;
  target 80 columns. No em dashes per CLAUDE.md.

## How to start

1. Survey what `tauri-action`
   (`tauri-apps/tauri-action@v0`) currently consumes for
   macOS signing. The action's README is the canonical
   reference for the secret-name conventions worth
   adopting.
2. Cross-reference against `notarytool`'s flag set (Apple
   moved from `altool` to `notarytool` a few years back;
   our brief should target `notarytool`).
3. Write the brief. Keep it tight — Alex's checklist at
   the bottom is the load-bearing part.
4. Fire a `poke` event to @@Architect via
   `alex/event-ci-architect.md` when ready for review.

## Coordination

* @@Architect reviews structure + the secrets-name list
  before commit.
* @@Alex consumes the brief out-of-band to start the cert
  provisioning ahead of Round 2.
* No code lane consumes this directly in Round 1.

## 2026-05-20 — landed (ready for review)

Owner: @@CI (recycled session).

### Path

Brief landed at [`../../../release/macos-signing.md`](../../../release/macos-signing.md)
(new `docs/release/` directory). The task's suggested path
fit, no slot ambiguity: `docs/` previously held only
`agents/` + `archive/` + `journals/`; `release/` is the
natural home for the macOS / Windows / Linux signing briefs
the north star needs.

### Structure

365 lines, single doc. Sections:

| Section                                  | Purpose                       |
|------------------------------------------|-------------------------------|
| Scope                                    | What's in / out               |
| Background                               | Why this brief exists         |
| Apple Developer Program enrollment       | Individual vs Organization    |
| Developer ID Application cert generation | CSR -> cert -> .p12 export    |
| App-specific password for notarytool     | Account.apple.com flow        |
| Notarization team ID + bundle ID         | Fields CI submits             |
| GitHub Actions Secrets shape             | Six-secret table              |
| Recommended cert-import packaging        | `apple-actions/import-...`    |
| @@Alex's pre-Round-2 checklist           | Seven chronological steps     |
| Parallel Windows signing path            | EV vs OV pointer              |
| References                               | Apple + Tauri + GH Action     |

### Secrets-name decision

Anchored on the names `desktop/Makefile`'s `app-notarized`
target already reads (`APPLE_SIGNING_IDENTITY`,
`APPLE_TEAM_ID`, `APPLE_ID`, `APPLE_PASSWORD`) plus the two
new ones CI needs for clean-runner cert import
(`APPLE_CERTIFICATE_BASE64`, `APPLE_CERTIFICATE_PASSWORD`).
Six secrets total. Matches the prefix scheme already
documented in `.github/workflows/release-desktop.yml`'s
header so `ci-4` does not have to rename anything.

Rationale for not adopting `tauri-action`-style longer
names (`APPLE_DEVELOPER_ID_APPLICATION_CERT_P12`, etc.): we
do not call `tauri-action`; we call `make app-notarized`
which calls `cargo tauri build` directly, so its env-var
contract is what matters. The Tauri CLI itself reads
`APPLE_*` short names. Aligning the secrets with the
Makefile + Tauri CLI contract is one fewer renaming layer.

### Cert-import action recommendation

`apple-actions/import-codesign-certs@v3` over a hand-rolled
`security` block. The hand-rolled fallback is included for
the record (~20 line bash block) in case the third-party
action ever gets abandoned, but it is not the recommended
path. Single YAML step vs ~20 lines of shell that has to
handle keychain locking + partition list + cleanup.

### Pre-push gate

Doc-only change, so the markdown side of the gate:

* No em dashes (`grep '—' docs/release/macos-signing.md`
  finds zero hits).
* ASCII tables, all rows within the 80-column target
  (`awk 'length($0) > 80'` finds zero hits).
* Factual prose; no marketing language. CLAUDE.md writing
  rules followed.

No code touched, so fmt / clippy / test / svelte-check /
npm build do not apply. Existing pre-push hook still runs
clean since this is a new file under `docs/`.

### Files changed

* `docs/release/macos-signing.md` (new, 365 lines).
* `docs/journals/phase-8/ci/ci-3.md` (this append).

### Commit readiness

Not committing per the standing rule. Proposed commit
message:

```
docs: macOS signing + notarization brief (pre-Round-2 reference)

Adds docs/release/macos-signing.md covering Apple Developer
Program enrolment, Developer ID Application certificate
generation, app-specific password for notarytool, the six
GitHub Actions secrets the Round-2 release workflow will
consume, the recommended cert-import packaging
(apple-actions/import-codesign-certs@v3), and the
chronological checklist @@Alex follows before Round-2 work
starts. Closes phase-8 ci-3; workflow YAML lands in ci-4.
```

### Open questions for @@Architect

1. `docs/release/` is a new directory. Confirm the path or
   redirect (e.g., `docs/signing/`, `desktop/docs/signing/`)
   before commit. Suggested path was pre-approved in the
   task spec; flagging only because this is the first file
   in the new directory.
2. Windows-lane brief: spin up `docs/release/windows-signing.md`
   as a follow-up `ci-5` once the macOS path is green, or
   fold it inline as a section in `macos-signing.md`?
   Recommendation: separate doc when we open the lane;
   keeps each brief tight and audit-trail clean.
3. The brief recommends @@Alex run `make app-notarized`
   locally as a verification step (item 7 of the checklist)
   before CI ever touches the cert. Want me to add a
   `make app-notarized` smoke-test step explicitly to
   `ci-4`'s acceptance criteria when the next task gets
   cut, or leave the local-test gate as a @@Alex-side
   prerequisite only?

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Strong work. The Makefile-contract anchoring is the right
call — `make app-notarized` is the production path, so the
six-secret table matches what the Makefile reads
(`APPLE_SIGNING_IDENTITY`, `APPLE_TEAM_ID`, `APPLE_ID`,
`APPLE_PASSWORD`) plus the two CI-only import-step extras
(`APPLE_CERTIFICATE_BASE64`, `APPLE_CERTIFICATE_PASSWORD`).
Spot-checked against `desktop/Makefile` lines 50-116:
contract matches. No renames needed in `ci-4`.

The `apple-actions/import-codesign-certs@v3` recommendation
is correct — single YAML step vs the ~20-line `security`
block that has to handle keychain locking + partition list
+ cleanup. Documenting the hand-rolled fallback for the
audit trail is the right hedge against action abandonment.

Alex's 7-step checklist is well-ordered. Item 7
(local `make app-notarized` verification) is the highest-
leverage step — catches enrollment / cert / password
issues before CI ever touches them. That belongs as a
@@Alex-side prereq, not as an additional CI step (answer
to your Q3 below).

365 lines, factual, no em dashes, ASCII tables within 80
columns. CLAUDE.md writing rules followed cleanly.

### Answers to your three questions

1. **`docs/release/` path**: confirmed. Pre-approved in the
   task spec and matches the existing docs tree shape. Commit
   as-is.
2. **Windows brief**: separate doc (`docs/release/windows-signing.md`)
   when we open the lane. Agreed with your recommendation —
   keeps each brief tight + audit-trail clean. Will get cut
   as `ci-5` (or whatever number falls next) once the macOS
   path is green in Round 2; do not fold it inline now.
3. **`make app-notarized` smoke-test in `ci-4`**: leave it
   as the @@Alex-side prereq only. Don't add a duplicate CI
   step. Reasoning: `ci-4`'s production workflow IS
   `make app-notarized` under real secrets; a separate
   "smoke-test" step in the same workflow would either (a)
   run against the same secrets and just be a re-execution
   of the production step, or (b) need a sandboxed cert
   that does not exist. The local-verification gate catches
   the enrollment-side issues; CI catches the workflow-side
   issues; no overlap needed.

**Commit clearance**: approved. Proposed commit message in
your "Commit readiness" section is good. Use it as-is. Push
waits for Round-1 close.

After commit you're idle / available again. Round-2 prep
(`ci-4` workflow YAML) parks until @@Alex completes the
6-step checklist and the GitHub repo secrets are populated.

Two fill-in options if you want low-stakes idle-time work
while @@Alex is provisioning:

* Audit `.github/workflows/ci.yml` and
  `.github/workflows/release-desktop.yml` for caching
  opportunities (Rust target cache via `Swatinem/rust-cache`,
  npm cache, sccache). Not Round-1 critical; small wins on
  CI wall-clock time.
* `release-desktop.yml` `workflow_dispatch` dry-run was
  parked for Round-1 close in your prior task tail; we can
  bring that forward and validate the YAML before the
  v0.11.1 tag fires the real run. Coordinate with
  @@Systacean on timing if you want to run it before
  `systacean-3` lands.

Pick whichever fits, or wait for the next wave. Either
way, fire a poke when ready.