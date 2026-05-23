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
