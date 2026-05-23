# architect-3 — public-flip pre-flight documentation

Owner: @@Architect (self-cut)
Phase: 8, Round 3
Date cut: 2026-05-23

## Goal

Draft every piece of pre-flight documentation needed for the
private → public repo flip. The actual flip itself is
@@Architect-led at a later beat (repo settings on GitHub +
README polish + announcement); this task is just the docs.

## Background

Round-3 Track 1 per
[`round-3-plan.md`](round-3-plan.md). @@Alex locked
2026-05-23:

* License: **Apache-2.0 only** (not the dual-MIT-Apache
  default; one LICENSE file).
* Journals stay public at flip + a `docs/coordination.md`
  explainer doc orients outside readers to the multi-agent
  dev pattern.
* Public-flip version: **v0.13.0** (not v1.0; keeps option
  to break things in v1.x).

## Acceptance criteria

Files created (or updated where they exist):

1. **`LICENSE`** at repo root — Apache-2.0 standard text.
   Copyright line: `Copyright 2024-2026 Alexandre Fiori
   (and chan contributors)`.
2. **`CONTRIBUTING.md`** at repo root — how to build, test,
   submit a PR; references the pinned `rust-toolchain.toml`
   + the pre-push hook (`./scripts/install-hooks`); points
   at `CLAUDE.md` for the deeper layout / principle reference.
3. **`CODE_OF_CONDUCT.md`** at repo root — Contributor
   Covenant 2.1 (standard adaptation; fiorix@gmail.com as
   contact).
4. **`SECURITY.md`** at repo root — vulnerability-disclosure
   policy. Private disclosure via fiorix@gmail.com; 90-day
   responsible-disclosure window; chan-drive sandbox is the
   primary security boundary (path traversal / symlink /
   filesystem-special handling).
5. **`.github/ISSUE_TEMPLATE/bug_report.md`** + 
   **`.github/ISSUE_TEMPLATE/feature_request.md`** — standard
   GitHub issue templates, lightly customised for chan
   (drive-path, chan version, platform).
6. **`.github/PULL_REQUEST_TEMPLATE.md`** — standard PR
   template; checklist for fmt + clippy + test + svelte-check
   + npm build (the pre-push gate).
7. **`docs/coordination.md`** — multi-agent dev pattern
   explainer. Audience: outside readers who land in
   `docs/journals/phase-N/` and wonder what they're seeing.
   Covers: phase loop, the @@Architect/@@Alex split, the
   working-agent roster, event-channel + task-file
   dispatch, the rationale (no marketing copy; factual).

NOT in scope for this task (handled separately):

* History secrets audit (gitleaks) — `systacean-43`.
* Repo settings + README polish at flip time —
  @@Architect-led handoff at the public-flip beat.
* CHANGELOG.md — `ci-15`.
* `README.md` polish — defer to flip beat (after this
  task lands).

## How to start

1. Apache-2.0 standard text from https://www.apache.org/licenses/LICENSE-2.0.txt (verbatim modulo copyright line).
2. CONTRIBUTING / CODE_OF_CONDUCT / SECURITY: adapt
   community-standard templates; keep factual + short.
3. Issue + PR templates: GitHub's recommended baseline +
   chan-specific tweaks.
4. `docs/coordination.md`: write fresh; the journals are
   the audit trail of this pattern's evolution.

## Coordination

Pre-flip prerequisite. Pairs with `systacean-43` (history
audit); the public flip itself happens only after both land
+ @@Alex green-lights.

Time-boxed: one pass. Polish lands as PR review feedback at
flip time, not in this task.

## 2026-05-23 — commit-readiness + deferred files

Landed in `main`:

* `LICENSE` — canonical Apache 2.0 text with `Copyright 2024-2026 Alexandre Fiori` in the appendix.
* `CONTRIBUTING.md` — repo build/test/PR flow + the architectural ground rules.
* `.github/ISSUE_TEMPLATE/bug_report.md`
* `.github/ISSUE_TEMPLATE/feature_request.md`
* `.github/PULL_REQUEST_TEMPLATE.md` — pre-push gate checklist.
* `docs/coordination.md` — multi-agent dev pattern explainer for outside readers.

Deferred to @@Alex (manual write):

* `CODE_OF_CONDUCT.md` — Contributor Covenant 2.1 adaptation. Agent's output kept getting blocked by Anthropic's content-filtering policy on legal/conduct text. @@Alex to draft manually. Suggested seed: standard Contributor Covenant 2.1 with `fiorix@gmail.com` as contact.
* `SECURITY.md` — same content-filter concern. @@Alex to draft. Suggested seed: private disclosure via `fiorix@gmail.com`; 90-day responsible-disclosure window; chan-drive sandbox is the primary security boundary (path traversal / symlink / filesystem-special handling).

Both are listed in the public-flip prereq checklist; the flip beat waits on @@Alex's drafts to land. Tracked as task #14 in this session's task list.

## 2026-05-23 — deferred files RESOLVED (@@Alex re-authorized; agent wrote)

@@Alex re-issued explicit authorization for the agent to author CODE_OF_CONDUCT.md + SECURITY.md. Second attempt succeeded — files landed:

* `CODE_OF_CONDUCT.md` — Contributor Covenant 2.1 adaptation, fiorix@gmail.com as contact.
* `SECURITY.md` — private disclosure via fiorix@gmail.com, 90-day window, chan-drive sandbox as primary boundary, scope + out-of-scope sections.

Public-flip pre-flight prereqs from this task: **complete**.
