# systacean-43 — repo history secrets / PII / leak audit

Owner: @@Systacean
Phase: 8, Round 3
Date cut: 2026-05-23

## Goal

Audit full git history for secrets, PII, internal references,
or other content that should NOT exist in a public repo.
Triage findings + either purge (history rewrite) or document
as benign before the public flip.

## Background

Round-3 Track 1 per
[`../architect/round-3-plan.md`](../architect/round-3-plan.md).
@@Alex locked public-flip docs on 2026-05-23. The public flip
itself is gated on this audit + the docs draft (`architect-3`).

The repo has been private since inception, with multiple
agents committing journal entries over ~6 months of phases.
Most journal content is intentional + safe (multi-agent dev
audit trail). Risk surfaces:

* **Throwaway-drive paths** with personal user names
  (`/tmp/chan-test-...`, `/Users/fiorix/...`).
* **API keys / tokens** accidentally pasted into journal
  prose during debugging (low likelihood given the
  secrets-boundary discipline, but worth verifying).
* **Internal URLs** that shouldn't be public (drive.chan.app
  tunnel URLs, internal Slack / Linear references — none
  expected, but verify).
* **PII** in screenshots committed to the repo
  (`architect/image.png` etc.) — alt-text + visual content
  audit.
* **Past commits** referencing private discussions or
  decisions that don't translate well for an outside reader.
  These don't need purging but flag for the
  `docs/coordination.md` writer (architect-3) so the
  explainer pre-empts confusion.

## Acceptance criteria

1. **`gitleaks detect --redact -v`** runs against full
   history (`git log --all`). Output: clean OR every
   finding documented (file + commit + verdict:
   purge / benign / acceptable).
2. **Manual grep sweep** for known-sensitive patterns
   across full history:
   * `BEGIN .* PRIVATE KEY`, `BEGIN CERTIFICATE`
   * Generic regex for AWS / GCP / GitHub PAT shapes
     (these gitleaks covers; the grep is a paranoia
     second pass).
   * `fiorix@gmail.com` — expected, document the rationale.
   * `127.0.0.1:PORT` patterns + tunnel URLs — flag any
     pointing at non-loopback / non-`*.chan.app` hosts.
   * Apple Developer ID names / signing-key fingerprints —
     these get rotated post-flip if found in history.
3. **Image audit**: every `*.png` / `*.jpg` /
   `*.svg` in `docs/` reviewed for sensitive content
   (passwords on screen, real user data, internal
   tools / dashboards). Either drop or annotate.
4. **Report at task tail**: gitleaks output summary +
   per-finding triage + recommendation on what (if
   anything) requires `git filter-repo` purge.
5. **No history rewrite without @@Alex approval** —
   history rewrites are destructive + need explicit
   "do it" + a coordination beat with @@CI (release
   tags reference SHAs that filter-repo would break).

## How to start

1. `gitleaks detect --redact -v --source=. --report-format=json --report-path=/tmp/chan-gitleaks-report.json` (gitleaks: `brew install gitleaks` if not present).
2. Per-finding triage: read the surrounding commit; classify benign / acceptable / requires-purge.
3. Image audit: `find docs -name '*.png' -o -name '*.jpg' -o -name '*.svg'` → eyeball each.
4. Manual grep sweep per the acceptance list.
5. Report appended to this task file tail.

## Coordination

* Pairs with `architect-3` (public-flip docs draft).
  Both prerequisites for the flip.
* If purge required: flag immediately (fresh poke event
  back to @@Architect). Tag rewrites + force-push
  coordination is a multi-lane event.
* Tools (`gitleaks`): @@Systacean territory; no shared-
  infra authorization needed for installing locally.

Time-boxed: one audit pass + report. Findings that
don't block the flip get documented as "acceptable
under license / not actually sensitive"; findings
that DO block the flip get escalated to @@Alex with
a recommendation.

## 2026-05-23 15:34 BST — audit report + recommendation

One-pass public-flip history audit complete. No destructive
history action taken.

### Tooling

Installed `gitleaks` locally via Homebrew (`gitleaks 8.30.1`)
per the task authorization.

Ran:

```
gitleaks detect --redact -v --source=. \
  --report-format=json \
  --report-path=/tmp/chan-gitleaks-report.json
```

Result: 1760 commits scanned, 20.53 MB scanned, 3 findings.

### Gitleaks findings

All 3 findings are historical per-launch loopback bearer tokens
recorded in pre-release webtest task files:

| Commit  | File                                   | Verdict |
|---------|----------------------------------------|---------|
| 7da49f6 | `chan-pre-release-phase-5/webtest-1.md` | Acceptable |
| 18efea4 | `chan-pre-release-phase-5/webtest-3.md` | Acceptable |
| 56075ce | `chan-pre-release-phase-6/webtest-1.md` | Acceptable |

Triage:

* Tokens were `chan serve` per-launch bearer tokens for
  `127.0.0.1:8787` test services.
* The task files themselves document teardown / service
  shutdown. These are not durable credentials.
* Loopback-only bearer tokens from stopped local test services
  are safe to leave in history. No purge recommended.

I am deliberately not repeating the token values in this report.

### Manual grep sweep

Full-history / current-tree checks:

* Private-key / certificate block pattern:
  `BEGIN [A-Z ]*PRIVATE KEY|BEGIN CERTIFICATE`
  * Full-history grep produced only this task file's own
    acceptance text. No key or certificate PEM block found.
* AWS / GitHub / GCP token shapes:
  `AKIA...`, `ASIA...`, `ghp_` / `gho_` / `ghu_` / `ghs_` /
  `ghr_`, `github_pat_...`, `AIza...`
  * Full-history grep: 0 hits.
  * Current-tree `rg`: 0 hits.
* `fiorix@gmail.com`
  * Expected and acceptable. Appears as the crate maintainer
    email, release/security contact, Apple ID name in release
    setup scripts, and journal audit trail.
* URL / host sweep:
  * Broad full-history grep is very noisy (313858 lines before
    the sampling cutoff) but dominated by loopback URLs,
    `drive.chan.app`, `id.chan.app`, and examples / tests.
  * Targeted `git log -G` found the expected tunnel/auth
    history (`*.chan.app`, loopback tests, old tunnel work).
  * No Slack / Linear / ngrok / trycloudflare / Tailscale
    private endpoint surfaced in the inspected sample or
    targeted patterns.
* Signing / notarization sweep:
  * `Developer ID Application: Alexandre Fiori (W73XV5CK3N)`,
    `APPLE_*` secret names, `notarytool`, and minisign docs are
    present by design.
  * Values present are public identity / Team ID / Apple ID /
    secret names and workflow wiring. No `.p12`, password,
    app-specific password, private key, or certificate material
    found.

### Image audit

Reviewed all committed docs images:

* `docs/journals/phase-6/image*.png`
* `docs/journals/phase-7/image*.png`
* `docs/journals/phase-8/architect/image.png`
* `docs/journals/phase-8/attachments/image*.png`

Verdict: acceptable for public history.

Observed content:

* Chan UI screenshots, graph view, menus, file-browser state,
  terminal prompts, GitHub UI, and local filesystem paths.
* One GitHub avatar / account UI in a screenshot.
* Local paths such as `/Users/fiorix/...` and
  `/Users/fiorix/dev/github.com/fiorix/chan`.

No passwords, token values, private dashboards, customer data,
key material, or sensitive terminals visible.

### Recommendation

No `git filter-repo` purge recommended.

Public-flip blocker status from this task: **clear**.

Residual public-facing exposure is limited to expected maintainer
identity, local development paths, historical loopback test URLs,
and project coordination journals. That matches @@Alex's locked
decision that journals stay public plus `docs/coordination.md`
explains the model.

### Notes for @@Architect

This audit did not modify source files other than this append.
Current working tree already contains unrelated Round-3 /
chan-desktop-team changes; I did not stage or touch them.

## 2026-05-23 — @@Architect: approved + commit clearance

Audit report reviewed. **Public-flip blocker status from this task: clear.**

Verified the report against the task's acceptance criteria:

1. ✓ gitleaks ran against full history; 3 findings all triaged + classified acceptable (per-launch loopback bearer tokens in stopped pre-release test services). No durable credentials; no purge needed.
2. ✓ Manual grep sweep clean: no PEM blocks, AWS / GitHub / GCP token shapes, or unexpected secrets. Expected items (`fiorix@gmail.com`, `Developer ID Application: Alexandre Fiori (W73XV5CK3N)`, `*.chan.app`, loopback test URLs) are public identity / shared-infra wiring + are correct to keep.
3. ✓ Image audit clean: chan UI screenshots, GitHub UI, local paths. No sensitive surfaces.
4. ✓ Report at task tail produced + actionable.
5. ✓ No history rewrite performed (correctly held; would have required @@Alex sign-off + @@CI coordination).

### Suggested commit subject

```
docs(systacean-43): public-flip history audit — gitleaks clean (3 findings all acceptable: pre-release loopback bearer tokens); no purge needed
```

### Commit instructions

Per the standing pre-authorization for your lane:

* Per-path `git add` only (multi-agent worktree; @@CI's `CHANGELOG.md` + @@FullStackA's `crates/chan-server/src/routes/files.rs` are in flight; do not touch).
* Files to stage explicitly: `docs/journals/phase-8/systacean/systacean-43.md`, `docs/journals/phase-8/alex/event-systacean-architect.md` (if you appended there).
* Pre-commit `git diff --staged --stat` + post-commit `git show --stat HEAD` per the atomic-audit pattern.

### Public-flip prerequisite checklist

| Prereq | Status |
|---|---|
| `architect-3` (LICENSE + CONTRIBUTING + .github templates + docs/coordination.md) | ✓ shipped (`cb7f140`) |
| `systacean-43` (history audit) | ✓ NOW (this clearance) |
| `CODE_OF_CONDUCT.md` + `SECURITY.md` | Deferred to @@Alex manual write |

Once @@Alex lands the two deferred files, mechanical prereqs are complete; flip waits on @@Alex's go-signal.

### Lane state post-`-43`

Queue-empty. The Round-3 Track-3 row for your lane (Rust dead-code + error-path + clippy::pedantic + CLI error-message audit) is not yet dispatched; routing to @@Alex for the dispatch-vs-park call. Standing by.

Thank you for the careful audit.
