# Phase 8 requests

## North star

**Ship a notarized macOS `.dmg` (plus signed Windows + Linux
equivalents) that users can download and install without
Gatekeeper / SmartScreen friction.** Tag-triggered CI produces
the signed installer artifact, hosted via the release pipeline.

Background and decomposition in
[`../phase-7/next-phase-backlog.md`](../phase-7/next-phase-backlog.md)
under "Phase 8 headline deliverable".

## Round shape

Round 1 → recycle → Round 2 → recycle → Round 3.

(Restructured 2026-05-20: @@Alex split the original Round 2
into Round 2 + Round 3 so the signed-release pipeline gets
exercised end-to-end with real keys BEFORE the repo flips
public. Reasoning: opening a repo is one-way; testing the
heavy machinery while the repo is still private de-risks
the public flip.)

### Round 1 — bug sweep + working tree clean

Close the bugs in
[`phase-8-bugs.md`](phase-8-bugs.md). The bug list keeps growing
as @@Alex flags items; the round closes when the list is empty
(or explicitly trimmed by @@Alex). **No binary cut at end of
Round 1.** First proper binary release ships at end of Round 2
once the signed+notarized DMG pipeline has been exercised
with real keys.

@@CI stands up in parallel with Round 1 — does not block the bug
wave but lands the GitHub Actions scaffold so Round 2's release
pipeline has its plumbing in place.

**2026-05-20 detour added**: stop embedding the BGE-small
semantic-search model into the binary (~89 MB → ~26 MB binary
for the eventual Round-2 release). Semantic search becomes
opt-in via Settings toggle + CLI command. Plus a small UI add:
pane-flip animation. Both land in Round 1 (the detour brought
them forward of Round 2 so they're in the first proper
release).

**2026-05-20 deferral**: the originally-planned v0.11.1
patch-release tag is cancelled. Round 1's commits stay
unpushed locally until end of Round 2; the first GitHub
Release is whatever tag fires at the end of Round 2 (likely
v0.12.0 or v1.0 depending on @@Alex's call).

### Round 2 — features + DMG-on-tag pipeline tested with real keys

Backlog items 1-7 from
[`../phase-7/next-phase-backlog.md`](../phase-7/next-phase-backlog.md),
sequenced around the north star, **plus** the full
signed+notarized DMG pipeline exercised with real Apple
Developer ID secrets provisioned in CI. The repo stays
private; the secrets get tested behind closed doors.

Default ordering:

1. Backlog item 7 — chan-desktop upgrade model + bundled chan
   binary (DMG north-star prereq).
2. `ci-6` — tag-triggered signed + notarized chan-desktop
   workflow consuming the six secrets from the `ci-3` brief.
   Real Apple Developer ID cert provisioned; a real `chan-v*`
   tag fires and produces a notarized DMG that opens cleanly
   on a second Mac.
3. Backlog item 6 — website migration + manual + first-launch
   UX + CI.
4. Backlog items 1 + 4 — drive metadata carousel redesign +
   Infographics tab container (coupled per the backlog).
5. Backlog item 2 — drive pre-flight + BOOT process.
6. Backlog item 3 — screensaver with PIN unlock.
7. Backlog item 5 — chan config currency audit.

Item 9 (FB watcher scope) already done in Round 1 as
`fullstack-b-6`.

### Round 3 — public flip + polish + hardening

Three headline items:

1. Backlog item 8 — **open-source the repo + CI test lane**.
   LICENSE-MIT + LICENSE-APACHE, CONTRIBUTING.md,
   CODE_OF_CONDUCT.md, SECURITY.md, GitHub issue + PR
   templates, history leak audit, then flip the repo public.
   By this point the signed-DMG pipeline has been exercised
   end-to-end with the real keys (Round 2), so the public
   flip is a low-risk operation.
2. **Multi-model search picker** — added 2026-05-20.
   Curated list of embedding models (user picks one); the
   default stays BAAI/bge-small-en-v1.5. Extends the
   Settings UI + CLI surface from the Round-1 detour
   (`systacean-6` + `systacean-7` + `fullstack-a-21`). The
   resolver in `systacean-6` is forward-compat'd to index
   by model name so this lands as a strict addition.
3. **Code cleanup + hardening + efficiency + docs review +
   release readiness** — added 2026-05-20. Whole-codebase
   stabilization pass before the public flip:
   * Frontend cleanup (@@FullStackA): dead code, deprecated
     patterns, accessibility audit, performance pass on the
     editor + graph + carousel surfaces.
   * Backend cleanup (@@Systacean + @@FullStackB): Rust
     dead code, error-path audit, input-validation pass at
     chan-server route boundaries, security-review skill
     against chan-drive's filesystem seams.
   * Efficiency (@@Systacean): profiling under realistic
     load (the Linux-kernel benchmark from backlog item 2's
     bench notes is the right stress test if BOOT lands by
     Round 3); remove obvious hot-path waste.
   * Docs review (@@Architect-led): CLAUDE.md accuracy +
     design.md updates + the manual content from backlog
     item 6 (Round-2 deliverable). Every public-facing
     markdown gets read by a fresh pair of eyes.
   * Release readiness (@@Architect + @@Systacean + @@CI):
     CHANGELOG, release notes for the public-flip version,
     smoke tests against the signed DMG, final pre-push
     gate run.

@@Architect to confirm sequencing + cut tasks before fan-out.

## How items flow into this file

@@Alex adds new bug items to
[`phase-8-bugs.md`](phase-8-bugs.md) as they
surface. @@Architect groups them into task files under
`<agent>/<agent>-N.md`. Once a bug ships, the corresponding task
file gets a "Commit readiness" append and the bug stays in
`phase-8-bugs.md` as the audit trail (struck through or marked
landed).

Round-2 features get task files cut from
`next-phase-backlog.md`; when an item is dispatched, the backlog
entry gets a "Dispatched as `<agent>-N.md`" note so the queue
stays clear.
