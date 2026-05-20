# Phase 8 Round-3 plan (draft, pre-discussion with @@Alex)

Author: @@Architect
Date: 2026-05-20

Status: **draft, not dispatched**. Created 2026-05-20 when
@@Alex split the original Round 2 into Round 2 + Round 3 so
the signed-release pipeline gets exercised with real keys
behind a private repo first. Round 3 is the public-flip +
polish + readiness wave.

Three headline tracks:

1. **Public flip**: backlog item 8 — open-source the repo
   with full LICENSE / CONTRIBUTING / SECURITY plumbing,
   history leak audit, then flip private → public.
2. **Multi-model search picker** — added 2026-05-20.
   Curated list of embedding models; user picks one. Extends
   the Settings UI + CLI from the Round-1 detour
   (`systacean-6` + `systacean-7` + `fullstack-a-21`).
3. **Whole-codebase cleanup + hardening + efficiency + docs
   review + release readiness** — added 2026-05-20. Polish
   pass before declaring chan publicly stable.

## Decisions @@Alex needs to confirm before fan-out

1. **License choice for item 8**: backlog recommends dual
   MIT + Apache-2.0 (Rust convention). Confirm pick.
2. **Phase journal handling at public flip**: archive under
   `docs/journals/private/`, OR keep public with a
   `docs/coordination.md` explainer of the multi-agent dev
   pattern? Recommendation: keep public with explainer —
   the coordination model is itself interesting and the
   journals are the audit trail.
3. **Curated model list for the picker**: which models go
   in? Initial proposal (@@Architect):
   * `BAAI/bge-small-en-v1.5` (default, ~130 MB) — current.
   * `BAAI/bge-base-en-v1.5` (~440 MB) — better quality.
   * `sentence-transformers/all-MiniLM-L6-v2` (~90 MB) —
     smallest + fastest.
   * `intfloat/e5-small-v2` (~130 MB) — alternative
     similar-size option.
   * @@Alex flags any additions / removals.
4. **Public-version number**: Round-2-close ships v0.12.0
   (or whatever); does Round 3 ship v1.0 to signal "public"
   or just a minor bump (v0.13.0)? Recommendation: v1.0 at
   the public flip — the version number tells users we
   consider the surface stable.
5. **Release-readiness scope cap**: hardening passes can be
   bottomless. Recommend a time-box (one wave per agent,
   not until-bug-free) — call it done when no
   release-blockers remain even if minor polish opportunities
   exist. @@Alex confirms the bar.

## Track 1 — Open-source flip (backlog item 8)

### Tasks (preliminary numbering)

| Task           | Owner          | Source                                                                                  |
|----------------|----------------|-----------------------------------------------------------------------------------------|
| architect-3    | @@Architect-led + @@Alex review | Draft LICENSE-MIT + LICENSE-APACHE + CONTRIBUTING.md + CODE_OF_CONDUCT.md + SECURITY.md + GitHub issue + PR templates + (optional) `docs/coordination.md` |
| systacean-N    | @@Systacean    | Repo history secrets / PII / leak audit (gitleaks + manual grep + journal spot-check)   |
| (architect-led) | @@Architect   | Coordinate the public flip itself: repo settings, README polish, README badge updates  |

### Acceptance criteria

* Repo is public on GitHub.
* `gitleaks detect --redact -v` runs clean against full
  history (or each finding triaged + documented).
* CONTRIBUTING.md + CODE_OF_CONDUCT.md + SECURITY.md +
  templates render correctly on GitHub.
* README renders cleanly for an outside contributor.
* By this point the signed-DMG pipeline (Round 2) has been
  exercised end-to-end + at least one real release has
  shipped, so the public flip is low-risk.

## Track 2 — Multi-model search picker

### Tasks (preliminary numbering)

| Task            | Owner        | Source                                                                                |
|-----------------|--------------|---------------------------------------------------------------------------------------|
| systacean-N+1   | @@Systacean  | Curated-list config schema + CLI subcommand (`chan index list-models`, `--model` flag on `download-model`) |
| fullstack-a-N   | @@FullStackA | Settings UI: model dropdown replacing the static "Model:" info row from `fullstack-a-21` |
| (architect-led) | @@Architect  | Confirm the curated list with @@Alex (per decision 3 above)                          |

### Acceptance criteria

* `chan index list-models --json` returns the curated list
  with name + size + quality blurb per entry.
* Settings UI shows a dropdown. Switching model triggers
  the download flow from `systacean-7` (idempotent — skips
  if already downloaded).
* Active model surfaces in `chan index status`.
* Per-drive override stays possible (config schema).

## Track 3 — Cleanup + hardening + efficiency + docs + release readiness

The bottomless track. Time-boxed per @@Alex's bar (decision
5 above). Each agent does ONE pass within their scope; the
pass produces a short report at the end ("here's what I
found, here's what I fixed, here's what I deferred").

### Tasks (preliminary numbering)

| Task          | Owner         | Scope                                                                              |
|---------------|---------------|------------------------------------------------------------------------------------|
| fullstack-a-N+1 | @@FullStackA | Frontend cleanup: dead code, deprecated patterns, accessibility audit (keyboard nav, ARIA, screen-reader), performance pass on editor/graph/carousel |
| fullstack-b-N | @@FullStackB | chan-desktop / Tauri cleanup + hardening (capabilities audit, IPC surface review, updater verification) |
| systacean-N+2 | @@Systacean   | Rust dead-code sweep, error-path audit, `unwrap()` audit, `clippy::pedantic` pass (selective) |
| systacean-N+3 | @@Systacean   | Input-validation pass at chan-server route boundaries (use the `security-review` skill against chan-drive's filesystem seams) |
| systacean-N+5 | @@Systacean   | **CLI error-message audit + polish** (seed: `chan serve` bind-port error doesn't name the port; @@Alex dogfooded the gap 2026-05-20). Audit every chan / chan-server / chan-drive error path; every user-facing error names the input that produced it (port, path, env var, secret name, etc.). See `phase-8-bugs.md` entry for the seed example. Broader theme per @@Alex: "we need to up our cmdline game by a lot." |
| systacean-N+4 | @@Systacean   | Efficiency profiling — Linux-kernel benchmark from backlog item 2 (if BOOT lands by Round 3), report hot paths |
| architect-4   | @@Architect-led | CLAUDE.md accuracy review + design.md updates + every `docs/` public-facing markdown gets a fresh read |
| webtest-a-N   | @@WebtestA    | Comprehensive walkthrough: "are we ready to ship publicly?"                       |
| webtest-b-N   | @@WebtestB    | Counterpart walkthrough on lane B                                                 |
| ci-N          | @@CI          | Workflow audit + release pipeline final verification + CHANGELOG generation        |
| (architect-led) | @@Architect | Release notes for the public-flip version + smoke tests + final pre-push gate    |

### Acceptance criteria

* Each agent's pass produces a written report at the tail
  of their task file.
* No release-blockers remain (any P0 / P1 from the audit
  passes gets fixed in-round; P2+ deferred to v1.x).
* CLAUDE.md + design.md reflect actual state.
* `CHANGELOG.md` exists at repo root with v0.12.0 + v1.0
  entries.
* Release notes for the public-flip version drafted by
  @@Architect + reviewed by @@Alex.
* All six platforms (macOS DMG, Linux AppImage + .deb +
  .rpm, Windows MSI/EXE) smoke-tested on real installs.

## Round-3 close

Round 3 closes with the repo public + v1.0 tag (or whatever
public-flip version) + an announcement (@@Alex-driven, out
of scope for this plan).

After Round 3 → phase 8 complete. Phase 9 (or v1.x
maintenance) gets a fresh request.md.

## Track 4 — Chan metadata import/export (added 2026-05-20)

@@Alex pulled this from Round 2 → Round 3 on 2026-05-20
("we will def need to recycle the session before doing
all that"). Pairs with Round-2's pre-flight + BOOT work
(item 2): Round 2 detects broken / missing chan-drive
metadata at boot, surfaces a remediation card with
Rebuild + Skip read-only; Round 3 adds the third option
"Import from backup" alongside the export feature itself.

### Headline

* `chan metadata export <drive-path> <output-path>` —
  dump per-drive `.chan/` metadata to a `.tar.zst`
  archive with a manifest (SCM identity + chan version
  + schema version + timestamp).
* `chan metadata import <drive-path> <archive-path>
  [--rescan]` — restore on a host with the same SCM
  identity (git remote URL matches; HEAD can differ).
  `--rescan` reconciles the FS delta. Refuses cleanly
  on remote mismatch.

### UI surfaces

1. **Infographics tab for the drive** (canonical
   surface): "Export metadata" + "Import metadata"
   buttons in the drive-overview slide. Prerequisite:
   the Round-2 Infographics tab container (item 4) lands
   first.
2. **Pre-flight remediation card** (recovery surface):
   the broken/missing state cards from Round 2 gain the
   "Import from backup" option. Same underlying action;
   different entry point.

### Use cases

* Local metadata backup against `.chan/` corruption.
* Cross-host session transfer between same-repo clones
  (laptop ↔ desktop).
* Recovery during pre-flight when chan opens a drive
  with broken metadata.

### Benchmark — Linux kernel round-trip (@@Alex
2026-05-20)

```bash
# Cold-index baseline (pairs with backlog item 2 BOOT bench)
git clone --depth 1 https://github.com/torvalds/linux /tmp/chan-bench-linux
chan add /tmp/chan-bench-linux
chan open  # let BOOT complete

# Round-trip #1 — clean
chan metadata export /tmp/chan-bench-linux /tmp/linux-meta-v1.tar.zst
chan metadata import /tmp/chan-bench-linux-mirror /tmp/linux-meta-v1.tar.zst
# (mirror = fresh clone of the same commit on a different path)
# assert: post-import state == pre-export across search / graph / report

# Round-trip #2 — branch + code delta
git -C /tmp/chan-bench-linux checkout <some-active-branch>
# OR
$EDITOR /tmp/chan-bench-linux/drivers/usb/core/hub.c
chan metadata export /tmp/chan-bench-linux /tmp/linux-meta-v2.tar.zst
chan metadata import /tmp/chan-bench-linux-mirror /tmp/linux-meta-v2.tar.zst --rescan
# assert: rescan reconciles the FS delta cleanly
```

Acceptance bar (rough; @@Alex confirms at run-time):

| Metric                                              | Target  |
|-----------------------------------------------------|---------|
| Export wall-clock (Linux kernel, warm SSD)          | < 30 s  |
| Import wall-clock (clean)                           | < 60 s  |
| Import wall-clock (with `--rescan`, small/med delta)| < 90 s  |
| Compressed archive size                             | 100-500 MB order of magnitude |

If numbers come in worse, Track 3 (Round-3 efficiency
pass) revisits.

### Why @@Alex's earlier spec attempt didn't land

Previous attempt (predates phase 8) tried to do too much
at once — likely conflated "export metadata" with
"export the whole drive" or generic cross-layout
adaptation. This shape is intentionally narrow:

* Same logical drive (SCM-identity gate).
* Slight FS-layout differences allowed (different
  absolute paths to the same files, rescan picks up the
  delta).
* No CRDT-style cross-host merge; snapshot + replay
  only.

### Tasks (preliminary numbering)

| Task          | Owner        | Scope                                                                                |
|---------------|--------------|---------------------------------------------------------------------------------------|
| systacean-N   | @@Systacean  | `chan metadata export` + `import` CLI + `.tar.zst` manifest shape + chan-server endpoints |
| systacean-N+1 | @@Systacean  | BOOT integration: "Import from backup" path on the remediation card from Round-2 item 2 |
| fullstack-a-N | @@FullStackA | Infographics tab drive-overview slide: Export / Import buttons (depends on Round-2 Infographics container) |
| fullstack-b-N | @@FullStackB | Pre-flight UI: extend the Round-2 remediation card with the "Import from backup" option |

Sized small per @@Alex's "very easy to implement and
reproduce with our local tools today" framing. Sequencing
in Round 3: lands alongside the cleanup + hardening pass
(Track 3) so the benchmark numbers feed the same
release-readiness audit.

## Idea parking lot — report extensions

[`report-extensions-ideas.md`](report-extensions-ideas.md)
catalogs candidate chan-report extensions for @@Alex to
scope: churn metrics, complexity metrics, contributor
stats, per-language dependency graphs, plus four
sketch-only candidates (test coverage import, build-time
tracking, markdown-specific report dimensions,
cross-drive aggregation). Not in scope until @@Alex picks
which ones become Round-3 tasks.

## Capacity assumptions for Round 3

Same six slots + @@Architect dispatcher. The bottomless-
track work mostly fans out in parallel since the agents
don't tread on each other's audit passes. Sequencing:

* Track 1 (open-source) can start as soon as Round 3 opens.
* Track 2 (model picker) can start once the curated list
  is confirmed by @@Alex.
* Track 3 (cleanup) runs in parallel with both, time-boxed.

The public flip itself (the actual visibility change on
GitHub) happens at the very end of Round 3 once tracks 1 +
3 have both produced clean passes.

## What this plan is NOT

* Bottomless polish. Track 3 is time-boxed per @@Alex's
  bar (decision 5).
* The v1.0 release-cut artifact. That gets its own
  `commit-plan-v1.0.md` at Round-3 close.
* A push trigger. Public flip is gated on @@Alex's
  explicit "go public" signal.