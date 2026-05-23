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

## Decisions (locked 2026-05-23 by @@Alex)

| # | Decision                | Locked outcome                                                                                                                                                                              |
|---|-------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1 | License                 | **Apache-2.0 only** (not dual MIT + Apache). One `LICENSE` file at repo root with the standard Apache-2.0 text + chan copyright line.                                                       |
| 2 | Journals at flip        | **Keep public** + add `docs/coordination.md` explainer for outside readers. Journals stay under `docs/journals/phase-N/` as the multi-agent audit trail.                                    |
| 3 | Curated model list      | **Pending** — surveyed separately; default proposal in section "Track 2" below. Track 2 dispatch waits on this answer.                                                                       |
| 4 | Public-flip version     | **v0.13.0** (not v1.0). Minor bump; v1.0 reserved for post-public-feedback iteration. Keeps the option to make breaking changes during the first public-feedback window.                    |
| 5 | Hardening scope cap     | **One wave per agent, time-boxed**. Round closes when no P0/P1 release-blockers remain; P2+ defers to v0.14+ / v1.x. Each lane produces a short report ("found X, fixed Y, deferred Z").     |

### Dispatch consequences (vs the original plan)

* `architect-3` drops LICENSE-MIT; ships Apache-only LICENSE
  + the rest of the docs bundle.
* No `docs/journals/private/` migration; `docs/coordination.md`
  is part of `architect-3`'s deliverables.
* Round-3 close cuts `chan-v0.13.0`, not `chan-v1.0.0`.
* Each agent's track-3 task gets a one-wave time-box; the
  report at the task tail is the close signal, not a second
  pass.

### Pending: Track 2 dispatch

Surveyed separately. Default proposal:

* `BAAI/bge-small-en-v1.5` (default, ~130 MB) — current.
* `BAAI/bge-base-en-v1.5` (~440 MB) — better quality.
* `sentence-transformers/all-MiniLM-L6-v2` (~90 MB) —
  smallest + fastest.
* `intfloat/e5-small-v2` (~130 MB) — alternative
  similar-size option.

Architect's read post-decisions-lock: Track 2 is a NEW
feature, not a public-flip prerequisite. With the
time-boxed cap + v0.13.0 minor-bump framing, **default
recommendation is defer Track 2 to a later cut** (v0.14
or post-flip). Flagged for @@Alex veto.

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

## Separate-team work — chan-desktop drive-onboarding redesign

@@Alex 2026-05-20: noted a substantial redesign of the
chan-desktop launcher onboarding flow + drives-list UI
(replace "Open drive" + "Attach" with a single `[new]`
entry branching by local / remote-outbound / remote-
inbound; windows-open column; Boot rename + gear
config dialog; forget-drive affordance). @@Alex's call:
"it's fine, i can spin up separate team to work on the
desktop part."

Full spec at
[`chan-desktop-onboarding-redesign.md`](chan-desktop-onboarding-redesign.md).
Decomposed into six sequenced steps; the separate
team's bootstrap reads that artifact + the related
chan-tunnel-{proto,client,server} crate designs.

NOT part of the main six-agent roster's queue. Listed
here so the Round-3 status snapshot surfaces it for
the desktop team's spawn.

## Idea parking lot — report extensions

[`report-extensions-ideas.md`](report-extensions-ideas.md)
catalogs candidate chan-report extensions for @@Alex to
scope: churn metrics, complexity metrics, contributor
stats, per-language dependency graphs, plus four
sketch-only candidates (test coverage import, build-time
tracking, markdown-specific report dimensions,
cross-drive aggregation). Not in scope until @@Alex picks
which ones become Round-3 tasks.

## UX polish backlog (Round-3 dispatch candidates)

Carryover items @@Alex flagged during the v0.12.0 drain
that don't block the cut but should land in Round-3:

* **Terminal font setting / download → move from main
  Settings to per-terminal-tab back-side settings**
  (added 2026-05-22 by @@Alex). `-b-30 slice b` shipped
  the Settings dropdown + download flow in the main
  Settings surface; @@Alex's intended location is the
  back-side of the terminal tab itself (where per-tab
  config lives). The dropdown + download IPC stay
  unchanged; only the surface location moves. Lane:
  @@FullStackA (back-side terminal config component) +
  possible thin coordination with @@FullStackB to ensure
  the main-Settings entry retreats to a global default
  vs per-tab override pattern.

* **Drafts row should be FIRST in FB tree, BEFORE the
  drive's directories** (added 2026-05-22 by @@Alex; spec
  miss in `-a-66 slice b`). Per addendum-a: "The Drafts
  folder will be shown in the File Browser as the very
  first element." Current behavior: Drafts is sorted
  alphabetically between `docs/` and `scripts/`. The
  synthetic-injection from `-a-66 slice b` adds Drafts
  but doesn't pin its position. Fix: pin the synthetic
  Drafts entry to position 0 in the FB tree sort, before
  the drive's own directories. Lane: @@FullStackA (SPA
  FB tree rendering — likely a one-line sort
  predicate adjustment).

* **Test case: user creates a folder called "Drafts"
  inside their drive** (added 2026-05-22 by @@Alex —
  collision scenario). The synthetic Drafts row from
  `-a-66 slice b` lives at the FB-tree level
  ABOVE the drive's actual contents (it's a metadata
  folder injected as wire-layer). If the user creates a
  real `Drafts/` directory inside the drive root, what
  happens? Two visible rows? Merge attempted? Sort
  conflict? Document the expected behavior + add a
  walkthrough scenario. Lane: @@FullStackA + audit
  expected semantics first.

* **Test case: New Terminal from a doc in Drafts should
  CWD into the metadata Drafts location, not the drive
  root** (added 2026-05-22 by @@Alex). When user opens
  a Drafts/untitled/draft.md doc and clicks "New
  Terminal" from the menu / right-click, the spawned
  terminal's CWD should be the actual on-disk drafts
  metadata dir (`~/.chan/.../Drafts/untitled/` per the
  systacean-24 layout) — NOT the drive root. Lane:
  @@FullStackA (SPA new-terminal CWD resolution) +
  possible scope-poke to @@Systacean if a chan-drive
  API helper is needed to resolve the on-disk path.

* **Graph error "no such path: Drafts/untitled/draft.md"
  on file-scope graph** (added 2026-05-22 by @@Alex via
  screenshot during `-a-66 slice b/c/d` validation).
  Graph view scoped to a Drafts file shows "no such
  path" error. May be related to the slice b/c/d
  data-flow gap that `systacean-32` partially closed
  via `Drive::stat` unification — graph route may
  still have a path-resolution code path that's NOT
  unified-aware. Audit at task pickup: trace the
  graph route's path resolution for `Drafts/`-prefixed
  files. Lane: @@FullStackA (SPA graph) OR @@Systacean
  (chan-server graph route) depending on which layer
  fails. Audit-then-route.

* **Axum route path-param syntax audit** (added
  2026-05-23 by @@Systacean via `-41` finding). The
  `-31` team load/unload routes registered with axum
  0.8 syntax (`{name}`) but the repo runs axum 0.7
  which uses `:name`. axum 0.7 treated `{name}` as a
  literal path segment → routes returned 404 since
  `-31` shipped. Fixed inline in `-41` for team
  routes. **Round-3 sweep**: grep `lib.rs` route
  registrations for any remaining `{<name>}` patterns
  (already verified clean for semantic/reports/
  screensaver; worth a defensive re-grep). Class of
  bug: `cargo build` + clippy can't catch it; only
  integration tests exercising the wildcard route do.
  Consider a CI-time route-shape lint to prevent
  regression. Lane: @@Systacean.

* **Team mutation routes lane reconciliation** (added
  2026-05-23 by @@Systacean via `-41` finding). `-41`
  put new create/duplicate routes in the OPEN lane
  for symmetry with `-31`'s existing load/unload,
  not the settings-writes lane the task body
  specified. Round-3: reconcile ALL team mutations
  to settings-writes uniformly. Lane: @@Systacean.

## Track 5 — Per-agent submit-chord encoding map (LOCKED 2026-05-20)

Promoted from parking-lot to confirmed Round-3 track on
2026-05-20 after @@Alex picked the recommended shape:
**manual picker (user-facing surface + escape hatch) +
process-tree probe (auto-detect default that fills the
picker's initial value)**. @@Alex's framing: "ok i will
take your recommendation now and remind me we need to
revisit this later."

The agent-self-announce path (#3 in the parking-lot
analysis below) remains the cleanest long-term shape;
it lands naturally when the identity-broadcast work
from
[`rich-prompt-session-evolution.md`](rich-prompt-session-evolution.md)
item D ships, since that work already establishes the
spawn-handshake protocol agents would announce
themselves through.

### Recap from parking-lot analysis

### The finding

Patch-release ships `AGENT_SUBMIT_CHORD = "\x1b[27;9;13~"`
(xterm modifyOtherKeys) as the single chord for agent-mode
submit. The probe surfaced one divergence:

| Agent          | Submit chord     | `\n` effect            |
|----------------|------------------|------------------------|
| Claude Code    | `\x1b[27;9;13~`  | Newline in multi-line draft |
| Codex          | `\r` (raw CR)    | Silent / ignored       |
| Gemini         | not probed       | not probed             |

Claude Code is chan's primary user (chan's own development
is Claude Code), so the single-chord ship targets it. Codex
users in agent mode would need to flip back to shell mode
(which sends `\n` — but codex silently drops `\n` too, so
shell mode is also wrong for codex). Effectively codex
doesn't work cleanly in either mode today; documented as
known-known.

### Risk analysis ("what would this break, if other agents pick up on this")

| Consumer of the bytes               | Behaviour                                              |
|-------------------------------------|--------------------------------------------------------|
| Claude Code                         | Submits — intended behaviour                            |
| Codex                               | No effect — bytes silently consumed; draft sits unsubmitted |
| Gemini                              | Unknown until probed; likely no effect (silent consumption) |
| Raw shell (bash / zsh / fish)       | No effect — most shells don't bind the modifyOtherKeys CSI; bytes silently consumed |
| TUI programs (htop, less, mc, nvim) | No effect for most; nvim with modifyOtherKeys enabled MIGHT interpret as an arbitrary mapping (low probability of a destructive binding by default) |
| Programs that explicitly bind the CSI sequence | Unknown — possible to fire an unintended action. No known offenders in chan's typical workflow. |

**Worst plausible outcome**: a shell user in agent mode
sends the chord, nvim's modifyOtherKeys binding fires some
unexpected mapping. Low-impact: the user can recover with
Ctrl-C / Esc. The TUI's own keybindings still work.

**Most likely outcome for non-Claude-Code consumers**:
silent consumption. No data loss, no destructive action;
the user just notices "agent mode doesn't submit here" and
flips back to shell mode.

### Round-3 candidate solutions

Picking what shape the per-agent encoding takes depends on
how chan detects which agent runs in a given terminal:

1. **Static per-prompt agent picker** — extend the
   shell/agent toggle from `fullstack-b-13` into a
   three-way picker: shell / claude-code / codex
   (gemini etc. added as their chords are probed). User
   picks the agent at terminal spawn / rich-prompt open.
   * Pros: simplest; explicit; matches user intent.
   * Cons: extra friction on first use per session.

2. **Auto-detection via process-tree probe** —
   chan-server walks the PTY child's process tree
   periodically; matches process name against a known
   list (`claude` → Claude Code; `codex` → codex;
   `gemini` → gemini). Sets the chord encoding
   automatically.
   * Pros: zero user friction.
   * Cons: process-walk per session; potential
     mismatch if the agent rebrands or runs under a
     wrapper.

3. **Agent self-announce on spawn** — agents that
   integrate with chan write a `event-agent-hello-<id>.md`
   event file announcing their identity + accepted
   chord. chan picks up the encoding from the
   announcement.
   * Pros: explicit + extensible; agents opt in to the
     protocol.
   * Cons: requires agent-side cooperation; doesn't
     help legacy / closed-source agents.

4. **Heuristic fallback chord chain** — agent mode
   tries Claude Code's chord first; if no echo of
   "submitted" within N ms, tries codex's chord. Too
   magical, but cheap to implement.
   * Pros: zero user friction.
   * Cons: timing-sensitive; produces double-submission
     on race conditions; brittle.

Recommendation when this lands: combine #1 (manual picker
as the user-facing surface + escape hatch) with #2
(auto-detection as the default that fills the picker's
initial value). #3 is the right long-term shape if /
when an `agent-hello` event becomes part of chan's
spawn-handshake protocol — sits naturally alongside the
identity-broadcast work from
[`rich-prompt-session-evolution.md`](rich-prompt-session-evolution.md)
item D.

### Sequencing

* Patch-release: ships single-chord (`\x1b[27;9;13~`).
* Round-3 Track-5 (NEW): per-agent encoding map.
  Pre-requisite: confirm gemini's chord (cheap probe,
  same shape as fullstack-b-13's). Then dispatch the
  picker + auto-detect work alongside the cleanup +
  hardening pass in Track 3, since the changes touch
  the same `Session::submit_mode` field and the same
  `dispatch_agent_event` branch.
* No task cuts until Round-3 fan-out; this parking-lot
  entry is the spec sketch.

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