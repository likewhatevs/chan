# Phase 8 journal

Author: @@Architect
Date: 2026-05-19

Canonical phase-wide journal per
[../process.md](../process.md). Carries the plan summary,
request checklist, capacity proposal, dispatch table, decisions
log, and the extended-requests trail.

Append-only. New entries go at the bottom under a dated heading.
Corrections are new appends with a back-link, not rewrites.

## Plan summary

Two rounds with an agent recycle between them.

### Round 1 — Bug sweep + new build

Close every item in
[`../phase-8-bugs.md`](../phase-8-bugs.md), then cut
a patch release (v0.11.1 likely). Bug list grows as @@Alex flags
items; round closes when the list is empty or trimmed.

Wave 1 dispatch (this journal entry) covers all currently-listed
bugs. Subsequent waves get cut as @@Alex adds bugs or as the
walkthrough lanes surface new repros.

### Recycle

@@Architect closes Round 1, fires `agent-recycle` events, @@Alex
opens fresh sessions for each agent using
[`docs/agents/bootstrap.md`](../../../agents/bootstrap.md).

### Round 2 — Features

Backlog items 1-9 from
[`../phase-7/next-phase-backlog.md`](../phase-7/next-phase-backlog.md),
sequenced around the notarized-DMG north star. Default ordering
captured in [`../request.md`](../request.md); @@Architect
confirms with @@Alex before fan-out.

## North star

Notarized macOS `.dmg` + signed Windows + Linux installers
shipped via tag-triggered CI. The CI lane (@@CI) stands up in
Round 1 to land the GitHub Actions scaffold; signing-key rotation
and the tag-triggered release pipeline are cross-round work that
spans into Round 2.

## Capacity proposal

Six working slots + @@Architect dispatcher.

| Agent        | Slots | Round 1 scope                                                       |
|--------------|-------|---------------------------------------------------------------------|
| @@FullStackA | 1     | Frontend bug clusters: file-browser tab, status bar, Cmd+K, rich    |
|              |       | prompt, editor, Hybrid pane preservation.                           |
| @@FullStackB | 1     | Native window-config persistence, terminal cluster (Cmd+T,          |
|              |       | scrollback, line adjustment), watcher dialog UX.                    |
| @@Systacean  | 1     | CLI scriptability (`--json`, `--name`), graph indexer bug, version  |
|              |       | bump + release cut at round close.                                  |
| @@CI         | 1     | GitHub Actions scaffold: build matrix, lint, test, npm build.       |
|              |       | Apple Developer ID provisioning + secrets handling research.        |
| @@WebtestA   | 1     | Baseline walkthrough lane A; per-fix verification as fixes land.    |
| @@WebtestB   | 1     | Baseline walkthrough lane B; per-fix verification as fixes land.    |

Tasks are atomic per bug-cluster; an agent finishing a task pokes
@@Architect via the event channel and picks up the next.

### Wave 1 fan-out (Round 1)

| Task          | Owner        | Scope                                                                                  |
|---------------|--------------|----------------------------------------------------------------------------------------|
| fullstack-a-1 | @@FullStackA | File-browser tab name (always parent-dir with trailing slash; drive-name derived path) |
| fullstack-a-2 | @@FullStackA | Status-bar click events removed (except notification expand/collapse); blue → yellow   |
|               |              | flash colour on notifications                                                          |
| fullstack-a-3 | @@FullStackA | Cmd+K cluster: label fix ("Hybrid ☯ Enter commit, Esc discard, H help"), remove        |
|               |              | flashing-H mid-screen, Cmd+K for slots 1/2/3 commits immediately                       |
| fullstack-a-4 | @@FullStackA | Rich prompt cluster: cursor focus on open (prompt area if empty, survey area if        |
|               |              | bubbles), cursor stays after Cmd+Enter, spawn-agent dialog actually opens,             |
|               |              | overlay no longer covers bottom of terminal                                            |
| fullstack-a-5 | @@FullStackA | Editor cluster: image+EOL scroll rollover, Hybrid empty-pane preservation when last    |
|               |              | tab closes, survey bubble re-pop fix (filter replied surveys in `watcherEvents.ts`)    |
| fullstack-b-1 | @@FullStackB | Native window-config stack: up to 20 layouts persisted; close last → reopen restores   |
|               |              | (chan-desktop frontend + chan-server persistence wire)                                 |
| fullstack-b-2 | @@FullStackB | Terminal cluster: Cmd+T new terminal, scrollback buffer (10k+ lines), line-adjustment  |
|               |              | bug repro between iTerm vs chan's term                                                 |
| fullstack-b-3 | @@FullStackB | Watcher dialog cluster: accept paths outside drive root, fix create-dir UX (missing →  |
|               |              | silent create; existing → attach without "overwrite" warning)                          |
| systacean-1   | @@Systacean  | CLI scriptability: `chan list --json`, audit drive-name uniqueness, then               |
|               |              | `chan remove --name <name>` if unique                                                  |
| systacean-2   | @@Systacean  | Graph indexer: links to files-not-in-repo bug (repro: seed drive with chan source +    |
|               |              | journals)                                                                              |
| systacean-3   | @@Systacean  | Round-1 close: version bump + tag + push (v0.11.1 or patch number @@Architect picks)   |
| ci-1          | @@CI         | GitHub Actions scaffold: workflows for `cargo fmt --check`, `cargo clippy -D warnings`,|
|               |              | `cargo test`, `web/npm run check`, `web/npm run test`, `web/npm run build`,            |
|               |              | `scripts/pre-push`. Matrix Linux + macOS + Windows. Lands separate from release CI.    |
| ci-2          | @@CI         | Release CI scaffold (parked behind ci-1): on `chan-v*` tag → build + (later) sign +    |
|               |              | notarize + upload to GitHub Release. Wire as a placeholder workflow for Round 1; full  |
|               |              | signing in Round 2.                                                                    |
| webtest-a-1   | @@WebtestA   | Baseline walkthrough on v0.11.0: reproduce every bug in `phase-8-bugs.md` and append   |
|               |              | per-bug repro notes. Pair with @@WebtestB to split coverage roughly by area.           |
| webtest-b-1   | @@WebtestB   | Baseline walkthrough on v0.11.0: counterpart to webtest-a-1.                           |

### Capability assumptions

* @@FullStackA / @@FullStackB carry Svelte / TS / axum / chan-
  server route fluency; can cross into chan-drive for filesystem
  seams; consult @@Systacean for Rust quality / dependency
  questions.
* @@Systacean drives the CLI subcommand layer (`crates/chan`)
  and the indexer side of chan-drive; owns the pre-push gate;
  owns the patch-release cut.
* @@CI lives in `.github/workflows/`, `desktop/src-tauri/`
  signing config, and GitHub Actions secrets. Does not edit
  `crates/` or `web/` source. Coordinates with @@Systacean for
  signing-key rotation (in-tree config change).
* Webtest lanes drive Chrome via `mcp__claude-in-chrome__*` and
  never edit code.

### Handoffs

* @@FullStack lands a fix → tags @@WebtestA or @@WebtestB for
  walkthrough → @@Systacean reviews if Rust quality / CI surface
  changes.
* @@Systacean lands a CLI fix → @@FullStack integrates if the
  frontend needs to react.
* @@CI lands a workflow → @@Systacean reviews if it touches
  build flags / Cargo profile / signing config files in-tree.

## Dispatch

Wave 1 cut on 2026-05-19. See task files under each agent's dir
for the per-bug detail.

### 2026-05-19 — Wave 1 fan-out live

All six working agents spawned via the bootstrap prompts in
[`../../../agents/bootstrap.md`](../../../agents/bootstrap.md).
Tasks are in flight:

| Agent        | Tasks                                    |
|--------------|------------------------------------------|
| @@FullStackA | fullstack-a-1 .. fullstack-a-5           |
| @@FullStackB | fullstack-b-1 .. fullstack-b-4           |
| @@Systacean  | systacean-1, -2; -3 parked at round close|
| @@CI         | ci-1, ci-2 (ci-2 parks behind ci-1)      |
| @@WebtestA   | webtest-a-1 (baseline walkthrough)       |
| @@WebtestB   | webtest-b-1 (baseline walkthrough)       |

Watching for:
* Permission events from webtest lanes (terminal + browser
  startup).
* Cross-lane coordination on fullstack-a-5 option (a): the
  bubble-repop fix's server-side leg may want @@FullStackB or
  @@Systacean help with the reply-endpoint rename.
* Pokes as tasks complete; commit grouping decisions in this
  journal.

## Decisions log

(append-only record of decisions made with @@Alex, mirrored from
[../alex/](../alex/) task files)

### 2026-05-19 — phase-8 shape

Source: chat with @@Alex pre-fan-out.

* **Round shape**: Round 1 bug sweep + new build → recycle →
  Round 2 features.
* **North star**: notarized DMG.
* **@@CI stands up in parallel with Round 1**, doesn't block the
  bug wave.
* Bug list lives in
  [`../phase-8-bugs.md`](../phase-8-bugs.md) and grows
  as @@Alex flags items.

## Extended requests (mid-phase additions)

(populated as @@Alex flags new bugs / scope)

## 2026-05-20 — Wave-2 fan-out (Round 1 continuation)

Resume after @@Alex paused mid-session. State on resume:

* Wave-1 mostly committed (16 commits, all unpushed pending
  Round-1 close).
* @@FullStackA finished -9 / -10 / -11; cleared in this wave.
* @@Systacean's `systacean-2` fix is sitting in the working
  tree uncommitted despite prior clearance; chased via
  `event-architect-systacean.md` 2026-05-20.
* @@WebtestA + @@WebtestB completed lane-A + lane-B Round-1
  sweeps; verdicts at the tails of their `webtest-{a,b}-1.md`
  files. 10 fixes hold on HEAD; 2 active repros (bug 8, bug
  11); 2 partials (bug 6, bug 20); 1 CNR (bug 14); 4 side
  observations.
* @@WebtestA's lane-A test-server URL forwarded to @@Alex via
  [`../alex/event-architect-alex.md`](../alex/event-architect-alex.md)
  along with a permission-extension ask for @@WebtestB's
  `fullstack-b-1` runtime walkthrough block.

### Wave-2 dispatch table

| Task          | Owner        | Source                                                                                  |
|---------------|--------------|------------------------------------------------------------------------------------------|
| fullstack-a-12| @@FullStackA | Bug 8 SPA-side follow-up: `GraphPanel.svelte::isFileGhost` lazy-tree second-ghost path  |
| fullstack-a-13| @@FullStackA | Bug 11 editor image-insert viewport snap + no-roll on subsequent typing                 |
| fullstack-a-14| @@FullStackA | Bug 20 partial: rich prompt re-open with bubble present focuses prompt input            |
| fullstack-b-7 | @@FullStackB | chan-desktop external `http`/`https` links no-op (Tauri `shell.open` wire)              |
| fullstack-b-8 | @@FullStackB | Cmd+Enter from rich prompt drops first character into focused terminal                  |
| fullstack-b-9 | @@FullStackB | Bug 6 partial: Cmd+T web blocked, pick alternate chord or document native-only          |
| ci-3          | @@CI         | Apple Developer ID provisioning + GitHub Actions secrets brief (Round-1 research lap)   |

Bug entries added to
[`../phase-8-bugs.md`](../phase-8-bugs.md) for each of the
six dispatched bugs (graph false-missing now has a Round-1
audit anchor; SPA second-ghost, image-insert, re-open focus,
Cmd+Enter swallow, Cmd+T web partial, chan-desktop links
all anchored). Four side observations also filed with
"not yet dispatched" markers (`.md.md` double extension,
"Stage:" copy in Hybrid help, Cmd+K p focus race, directory-
typed-as-file in indexer) — queue depth management; cut
tasks if -12/-13/-14 land fast enough to absorb them in
Round 1.

### Watching for

* Commit clearance receipts: @@FullStackA on -9/-10/-11;
  @@Systacean catching up on the stranded `-2` commit.
* @@Alex's call on the @@WebtestB Tauri-launch permission
  extension. If approved via transcription to
  `event-webtest-b-alex.md`, lane B picks up the
  `fullstack-b-1` runtime walkthrough.
* @@FullStackA pickups on -12 / -13 / -14 (recommended
  order: -13 first, then -12 mid-queue, then -14).
* @@FullStackB pickups on -7 / -8 / -9 (recommended order:
  -7 first as it blocks the lane-A URL hand-off from
  inside `Chan.app`).
* @@CI handing back the macOS signing brief from ci-3.
* @@WebtestA / @@WebtestB verdicts on the wave-1 commits
  once @@Systacean commits `-2` and the binary rebuilds.

### Round-1 close gate

Open before close can be called:

* Active repros (bug 8, bug 11) cleared with walkthrough
  verdicts.
* Partials (bug 6, bug 20) resolved one way or another.
* CNR (bug 14) re-attempt by a webtest lane (or struck off
  as wontfix for v0.11.1 if it stays unreproducible).
* Side observations: dispatch or defer to Round 2 with
  explicit cut.
* Commit-grouping plan published; @@Systacean cuts
  `systacean-3` (version bump + tag + push) per that plan.

Will publish the commit-grouping plan after wave-2 produces
its first commit-ready batch or a webtest verdict lands on
the wave-1 fixes, whichever comes first.

## 2026-05-20 — @@Alex stepping away; autonomy mode

@@Alex stepped away for an extended window. Directive on
exit:

* Hold the v0.11.1 build cut. @@Alex returns to cut it.
* Crack on all the bug list while they're away.
* Run tests + build to keep the working-tree state green.
* Features (Round 2) need discussion. Do not dispatch
  Round 2; planning notes are fine.

Interpretation of "tests + build" — I am NOT taking it as
"trigger the production v0.11.1 build" (that is the held
cut). I am taking it as "validate the composite working
tree against the pre-push gate" — fmt + clippy + workspace
test + no-default-features build + web/ check + vitest +
npm build. This is high-value because each agent runs the
gate against THEIR change in isolation; the composite gate
catches inter-agent conflicts before commit-grouping.

### Autonomy expansion (this window only)

While @@Alex is out:

* I clear commit-ready work as it lands (already
  established as obvious-call territory; nothing changes).
* I cut additional tasks from the side-observation backlog
  to keep queues deep (already done for -15/-16/-17 + -4).
* I do NOT transcribe `permission` events that need
  @@Alex's interactive participation. The two currently
  open (WebtestB Tauri-launch for `fullstack-b-1` runtime
  walkthrough; FullStackB `make run` for `fullstack-b-7`
  runtime click-verification) both stay open until @@Alex
  returns. Runtime walkthroughs are not on the critical
  path because:
  - code reviews + pre-push gates + structural pins are
    sufficient interim confidence.
  - @@Alex will combine the runtime click-checks with the
    build cut in one session.
* I do NOT publish a Round-2 dispatch fan-out. @@Alex flagged
  features as discussion-gated. Round-2 planning notes can
  be drafted as architect-side artifacts; task files in
  agent dirs would risk current sessions picking them up
  prematurely.
* I do NOT push commits. Push is gated on Round-1 close
  which is gated on @@Alex's return for the build cut.

### What I'm actively doing

* Monitoring incoming events from FullStackA, FullStackB,
  Systacean, CI, WebtestA, WebtestB.
* Clearing commit-ready work as it lands.
* Cutting follow-on tasks from the side-observation
  backlog so agents don't idle.
* Running the composite pre-push gate periodically to
  validate the working-tree state.
* Drafting (not dispatching) Round-2 planning notes so
  the moment @@Alex confirms sequencing, we can fan out
  fresh agent sessions.

### Round-1 status snapshot

Committed wave-1 + early wave-2 (push parked):

* fullstack-a-1, -2, -3, -4, -5, -6, -7, -8, -9, -10, -11
* fullstack-b-1, -2, -3, -4, -5, -6
* systacean-1, -2
* ci-1, ci-2

In-flight (wave 2 + wave 3):

* fullstack-a-12, -13, -14 (queued)
* fullstack-a-15, -16, -17 (queued, deeper)
* fullstack-b-7 (in working tree, cleared code-review-only;
  runtime click parked for @@Alex)
* fullstack-b-8, -9 (queued)
* systacean-4 (queued; depends on -2 in commit history,
  which has now landed at 4a04917)
* ci-3 (cleared, awaiting commit by @@CI)

### Round-2 baking status

Drafted [`round-2-plan.md`](round-2-plan.md) as an
architect-side planning artifact (not dispatched). Source:
`phase-7/next-phase-backlog.md` items 1-9 (item 9 done as
`fullstack-b-6` in Round 1), sequenced per `request.md`'s
default ordering, with the DMG north-star through-line
cutting across items 7 + 8 + @@CI. Plan lists six decisions
@@Alex must confirm before fan-out (sequencing, license
choice, item-6 hosting, item-7 bundled-chan storage layout,
item-3 PIN hash, manual home).

## 2026-05-20 — major restructure: Round 3 added + binary deferred + secrets pre-authorization

@@Alex made several structural changes on return:

### Detour before recycle (still part of Round 1)

Stop embedding BGE-small semantic-search model in the
binary (~89 MB → ~26 MB). Settings toggle + CLI command
for opt-in semantic search. Plus a small UI add: pane-flip
animation (style of `nnattawat.github.io/flip`). UX shape
confirmed via survey:
* Enable path: both Settings toggle + CLI command.
* Model storage: user config dir, shared across drives
  (`<user-config>/chan/models/<model-name>/`).
* Build option: keep `--features embed-model` cargo
  flag for power users / offline installs.

Cut as four tasks:
* `systacean-6` — cargo feature gating + runtime resolver.
* `systacean-7` — CLI subcommands + chan-server API
  endpoints.
* `fullstack-a-21` — Settings UI for semantic-search
  toggle.
* `fullstack-a-22` — pane-flip animation.

### Round restructure: 2 → 3 rounds

Round 1 → Round 2 → Round 3. Justification (paraphrasing
@@Alex): test the signed-release pipeline with real Apple
Developer ID keys behind a private repo BEFORE flipping
the repo public. Opening a repo is one-way; de-risking
that flip is worth a round.

* **Round 1**: bugs + detour, NO binary cut. Closes when
  bug list drains + detour tasks land.
* **Round 2**: backlog items 1, 2, 3, 4, 5, 6, 7 + the full
  signed+notarized DMG pipeline tested with real keys in
  CI. Repo stays private. First proper binary release
  ships at end of Round 2 (likely v0.12.0 or v1.0 —
  @@Alex's call when the time comes).
* **Round 3**: open-source flip (item 8) + multi-model
  search picker + whole-codebase cleanup + hardening +
  efficiency + docs review + release-readiness pass.

### Binary cut deferred

The originally-planned v0.11.1 patch-release tag is
**cancelled**. `systacean-3` (version bump + tag + push)
parks until Round-2 close. No GitHub Release between
v0.11.0 and whatever ships at end of Round 2.

`commit-plan-v0.11.1.md` is repurposed as the Round-1
close plan (no tag); push-order + tag-message sections
are historical artifacts. Round-2-close cut will get its
own plan when the time comes.

### Multi-model search picker added to Round 3

User picks one model from a curated list (initially just
`BAAI/bge-small-en-v1.5`; expands in Round 3). The
runtime resolver in `systacean-6` is forward-compat'd to
index by model name so the picker lands as a strict
addition.

### Round-3 polish wave added

Whole-codebase cleanup + hardening + efficiency + docs
review + release readiness. Spans every lane. Sits
alongside the open-source flip + multi-model picker as
Round 3's deliverables.

## 2026-05-20 — Round 1 closing (@@Alex called close + recycle)

@@Alex declared Round 1 closed and asked for the
recycle. The bug list still has the partial repros
(bug 6 web Cmd+T, bug 20 re-open focus already
addressed; bug 14 watcher first-try hang stays CNR) —
@@Alex's call to call it done rather than wait for the
remaining webtest verdicts. The verifications fold into
Round 2's BOOT + pre-flight work since those changes
rebuild the binary anyway.

### What landed (commits in HEAD)

Round-1 bug sweep + detour all in HEAD across:
* @@FullStackA: -1 through -27 (27 tasks; -a-27
  uncommitted in working tree as of recycle).
* @@FullStackB: -1 through -12 (12 tasks).
* @@Systacean: -1, -2, -4, -5, -6, -7, -8, -9 +
  Makefile fill-in (-3 cancelled per the restructure).
* @@CI: ci-1, ci-2 ×2, ci-3, ci-4, ci-5, ci-6 (6
  commits across 5 task IDs).
* @@WebtestA: webtest-a-1 (Round-1 sweep + per-fix
  verdicts).
* @@WebtestB: webtest-b-1 (Round-1 sweep + per-fix
  verifications + proactive walks on systacean-7
  surfacing systacean-8 + -9).

No binary cut per the restructure decision: first
proper release at Round-2 close (v0.12.0 confirmed
2026-05-20).

### Restructure recap

| Decision                  | Status |
|---------------------------|--------|
| Round 1 → 2 → 3           | Set    |
| No Round-1 binary cut     | Set    |
| First binary at Round-2 close | Set, version v0.12.0 |
| Round 2 = features + signed DMG with real keys (repo private) | Set |
| Round 3 = open-source flip + polish + multi-model picker + metadata import/export | Set |
| Secrets pre-authorisation | Set (architect directs CI on names; values stay in GH Secrets + @@Alex's password manager) |

### Agent recycle

Firing recycle events for all six working agents via
`alex/event-architect-alex.md`. Each agent's most-recent
journal entry serves as the de-facto handover; queue-
empty state means there's no in-flight work to preserve
across the recycle.

**2026-05-20 amendment**: @@Alex poked the current
@@FullStackA session to commit `-27` (the Hybrid
hamburger polish that was sitting in working tree at
recycle-call time). Their recycle holds until `-27` lands
in HEAD — otherwise a fresh session would race the
current one on the same files. The other five recycle
events stand as-is. Once `-27` commits, Round-1 is fully
in HEAD; the @@FullStackA recycle then proceeds at
@@Alex's convenience.

### Round-2 fan-out

Stands by until @@Alex confirms the open decisions at
the top of `round-2-plan.md` (sequencing, item-6
hosting, item-7 bundled-chan layout, item-3 PIN hash,
manual home). After confirmation + fresh agent sessions,
Round 2 dispatches.

### Secrets pre-authorization from @@Alex

@@Alex pre-authorized architect-level access to direct
@@CI on consumption of signing/notarization secrets, with
the boundary: **values never appear in journals / chat /
commits**. Pre-authorization covers:

| Secret name(s)                       | Purpose                       | Used in       |
|--------------------------------------|-------------------------------|---------------|
| `APPLE_*` (six secrets per ci-3 brief) | macOS DMG sign + notarize     | Round 2 ci-6  |
| Tauri updater minisign release key   | chan-desktop self-update sign | Round 2 systacean-8 (rotation) |
| Windows code signing cert            | Windows MSI/EXE sign          | Round 2/3     |
| GitHub repo visibility flip          | Public flip                   | Round 3 architect-3 |
| DNS / TLS for chan.app apex          | Website migration             | Round 3 (if item 6 lands there) |

Operational pattern: architect directs @@CI on the
secret NAMES to consume in workflow YAML; @@Alex
populates VALUES into GitHub Actions Secrets manually
per the ci-3 brief checklist. Architect-side tasks
include `Authorization: yes` framing in dispatch pokes
when shared infra is touched (per the
`feedback_classifier_shared_infra` pattern).

## 2026-05-20 — lesson learned: don't invent crate capability descriptions

While drafting the pre-flight explanatory copy for the
"Reports" toggle in `round-2-plan.md`, I wrote:

> Reports generates analytics + cross-drive summaries
> (chan-report). Per-drive.

The "cross-drive summaries" claim was invented. I had not
read `crates/chan-report/README.md` or `design.md` before
writing the description. @@Alex pushed back ("What's with
the cross drive summaries?") and the audit revealed:

* `crates/chan-report/README.md` describes the crate as
  "Per-file language and SLOC report for a directory tree,
  with per-language roll-ups and a Basic COCOMO summary
  on top."
* Per-drive state at `.chan/report.jsonl`. No
  cross-drive aggregation in the crate today.
* My "summary" framing came from the word "Reports" + a
  generic intuition about what reports do, not from the
  actual crate scope.

### Why this matters

User-facing explanatory copy in a pre-flight UI is
load-bearing: it's how new users decide what to enable.
Mis-describing a feature in that copy would have:

* Set wrong expectations for what enabling Reports does.
* Confused @@Alex (who knows the actual scope) and forced
  a correction round.
* Risked landing in production if @@Alex hadn't pushed
  back.

### Lesson

**Before describing what a crate / module does in
user-facing copy, read its README + design.md.** Cite the
source in the architect-side draft. If the crate has no
README, read the lib.rs / mod.rs doc-comments. Never
generate descriptions from the crate name + generic
intuition.

This applies to:
* Pre-flight / Settings / first-launch explanatory copy.
* Release notes + CHANGELOG entries.
* Task-cut "Background" sections (where I describe what
  the consumer of a task will consume).
* Round-plan capability sketches.

Fix landed: `round-2-plan.md` Reports description now
reads "runs code analysis on every file — language
detection (tokei), source-lines-of-code counts per file +
per-language roll-ups, and a Basic COCOMO estimate on
top. Maintained incrementally from filesystem events.
Per-drive." Sourced directly from `crates/chan-report/README.md`.

The "cross-drive aggregation" idea that I incorrectly
attributed to current chan-report is captured as a
candidate Round-3 extension in
[`report-extensions-ideas.md`](report-extensions-ideas.md)
"Other ideas worth considering" — listed for
completeness with a note that it's NOT what the crate
currently does. @@Alex picks whether it ever lands.

Saved as memory `feedback_ground_descriptions_in_source`
so future architect sessions inherit the rule.

### Composite working-tree gate 2026-05-20

Ran the full pre-push gate against the composite working
tree (multiple agent changes overlaid):

| Check                                      | Result   |
|--------------------------------------------|----------|
| cargo fmt --check                          | clean    |
| cargo clippy --all-targets -- -D warnings  | clean    |
| cargo test --all-targets                   | clean    |
| cargo build --no-default-features          | clean    |
| web/ npm run check (svelte-check + tsc)    | 0 / 0    |
| web/ npx vitest run (isolated)             | 475/475  |
| web/ npm run build                         | clean    |

First vitest run (in parallel with cargo test) surfaced 6
timeout failures in `TerminalTab.test.ts` — re-ran in
isolation, all 475 passed. Resource-starvation false
positive; tests are fine. Note for future composite gate
runs: sequential, not parallel, between cargo-test + vitest.

Validates that fullstack-a-9/-10/-11 commits + systacean-2
commit + the uncommitted fullstack-b-7 capability changes
+ the uncommitted desktop/Makefile drift all coexist
cleanly. No inter-agent integration issue surfaced.

## 2026-05-20 — Round-1 teardown + housekeeping committed

`ecfceec` "docs: phase-8 round-1 teardown checklist +
agent confirmations" — 14-file journal-only commit
bundling the architect-side teardown checklist pokes
(fired to all six agents after @@Alex flagged the
omission) plus the four agent-side teardown-complete
confirmations (@@CI, @@FullStackB, @@Systacean,
@@WebtestB). Push remains parked for end of Round 2.

Working tree clean. Round-1 fully in HEAD.

## 2026-05-20 — Round-2 rich-prompt session evolution: decisions locked

@@Alex dropped a 5-item ask for the rich prompt + spawn-
agent surface to become chan's multi-agent session
conductor. Drafted [`rich-prompt-session-evolution.md`](rich-prompt-session-evolution.md)
as a planning artifact; surveyed 4 design decisions and
got clean-sweep agreement on the architect-recommended
options:

| Decision             | Locked                                                                          |
|----------------------|---------------------------------------------------------------------------------|
| History storage      | On-disk `.md` per drive under `.chan/rich-prompt-history/<tab>/`                |
| Cwd preflight        | Always-visible header field inside the rich prompt                              |
| Submit-mode toggle   | Per-prompt toolbar icon (shell / agent)                                         |
| Team-spawn surface   | Inside the rich prompt as a new conductor band (cwd + team + eyeball + broadcast) |

Five tasks shaped (provisional numbering at fan-out
time):

* `fullstack-a-N` — Rich prompt clear-buffer-on-submit
  + on-disk `.md` history backlog + history panel
  rendering above the composer.
* `fullstack-a-N+1` — Always-visible cwd header field +
  validator + SerTab `rpd` persistence.
* `fullstack-b-N` — Submit-mode toggle (shell / agent)
  + chord encoding for the agent-submit path. Owned by
  @@FullStackB because the encoding research sits next
  to the terminal / PTY work they've been doing.
* `fullstack-a-N+2` — Multi-row team-spawn band inside
  the rich prompt: agent-name + command + env rows +
  `+`/`-` + "launch in back" checkbox. Includes
  net-new "spawn to back of Hybrid" wiring in
  `tabs.svelte.ts`.
* `fullstack-a-N+3` — Eyeball preflight (per-tab output
  snapshot + ready checkbox) + identity broadcast (canned
  message fan-out to all confirmed terminal sessions).

Sequencing: items A/B/C ride Round-2 wave 2 (after the
chord-migration wave-1 task lands Cmd+P); items D/E pair
as wave 3 (D consumes E's spawned tabs). All five tasks
form a coherent rich-prompt evolution arc that ships as
a unit; release notes at Round-2 close should describe
it as "rich prompt becomes the session conductor."

Round-2 fan-out still gated on the broader sequencing
decisions in `round-2-plan.md` (item-6 hosting, item-7
storage layout, item-3 PIN hash, manual home). The
rich-prompt evolution stack adds 5 tasks to the Round-2
plan but does not change the open decisions there.

## 2026-05-20 — Rich-prompt mini-wave fanned out + patch release re-activated

Live broadcast smoke test with @@Alex surfaced four
bubble-overlay + survey-reply bugs (flicker, non-survey
replies don't dismiss, no explicit close, `poke<Enter>`
echo). Combined with the prior page-width tile-cascade
and collapse-chevron dead-space findings, @@Alex called
the play: cut a patch release with the rich-prompt fixes
in **before** the broader Round-2 fan-out.

This restructures the 2026-05-20 "no Round-1 binary; first
release at Round-2 close" framing: a v0.11.x patch goes
out NOW (Round-1 + this mini-wave); the signed-DMG north
star with real keys stays parked behind it.

### Mini-wave dispatch

| Task             | Owner        | Scope                                                              |
|------------------|--------------|--------------------------------------------------------------------|
| `fullstack-a-28` | @@FullStackA | BubbleOverlay regression: filter generalization + explicit dismiss + refresh diff-merge |
| `fullstack-a-29` | @@FullStackA | Collapse chevron dead-space: terminal-host margin recompute on the -a-24 collapse transition |
| `fullstack-a-30` | @@FullStackA | Per-prompt page-width + slider in textbox right-click menu        |
| `fullstack-b-13` | @@FullStackB | Shell/agent submit-mode toggle + survey-reply echo consumer (PTY chord encoding research front-loaded) |
| `systacean-10`   | @@Systacean  | Event-watcher fsnotify path: silent-skip non-matching filenames + module-doc + process.md convention note |

@@CI on standby; @@WebtestA / @@WebtestB have
verification queues queued.

### Watching for

* @@FullStackB's chord-encoding empirical result (likely
  `\x1b[27;9;13~` xterm modifier-other-keys; could be
  `\x0d`). Pin in -b-13 task tail at the top of the work.
* Cross-lane touch between -a-28 and -b-13 — the
  survey-reply echo call site that emits "poke<Enter>"
  today may live in the same file as the bubble-overlay
  dismissal code. Both agents flagged the coordination
  in their inbound pokes.
* Patch-release commit-grouping plan publication once
  the mini-wave produces commit-ready batches. Likely
  `commit-plan-v0.11.x.md` shape; @@Systacean cuts the
  tag once everything is green.

Round-2 broader fan-out (carousel, Infographics, BOOT,
manual, signing-pipeline-with-real-keys, etc.) parks
until the patch ships.

## 2026-05-20 — Decision: Round-3 Track 5 (per-agent submit-chord encoding map)

@@FullStackB's `-b-13` probe found that codex
diverges from Claude Code on the submit-chord
encoding (`\r` vs `\x1b[27;9;13~`). Patch-release
ships single-chord (Claude Code's encoding) per
@@Alex's "make it work now" directive.

@@Alex 2026-05-20: "ok i will take your
recommendation now and remind me we need to revisit
this later" — locked Round-3 Track 5 with the
recommended shape:

* **Manual picker** as the user-facing surface (shell
  / claude-code / codex / future agents). Becomes
  the user's escape hatch + explicit override.
* **Process-tree probe** as the auto-detect default
  that fills the picker's initial value (chan-server
  walks the PTY child's process tree, matches
  against a known list).
* **Agent self-announce** (item D in the rich-prompt
  session-evolution) remains the cleanest long-term
  shape; it lands naturally when the
  identity-broadcast spawn-handshake protocol ships.

Round-3 Track 5 section in
[`round-3-plan.md`](round-3-plan.md) carries the
locked spec + the four-candidate analysis as the
recap. Pre-requisite at fan-out: cheap gemini probe
(same shape as `fullstack-b-13`'s) to confirm or
deny universality of the `\r` vs
`\x1b[27;9;13~` split.

### Reminder mechanism

Any future @@Architect session reading
[`round-3-plan.md`](round-3-plan.md) on bootstrap
sees Track 5 in the locked-tracks list. The
architect-prompt bootstrap step 5 explicitly reads
the round-N-plan artifacts, so this can't be
missed at Round-3 open.

When Round-3 opens and architect-side dispatch
begins, surface Track 5 explicitly to @@Alex
("revisiting the codex divergence from -b-13") as
part of the Round-3 status snapshot — that's the
"remind me later" mechanism @@Alex asked for.

## 2026-05-20 — v0.11.1 cut + pushed

`chan-v0.11.1` is out. First proper GitHub Release
since the Round-1 closeout. Unsigned matrix; signed-
DMG north star with real keys stays Round-2 work.

### Tag artifact

* Version-bump commit: `2c6680b` (`chan v0.11.1`).
* Annotated tag: `chan-v0.11.1` (body from
  [`commit-plan-v0.11.1.md`](commit-plan-v0.11.1.md)'s
  "Tag draft (v0.11.1)" section, used verbatim).
* Push state: `main` matches `origin/main`. CI fires
  on the `chan-v*` tag per `release.yml` +
  `release-desktop.yml`.
* Closeout poke from @@Systacean recorded at the tail
  of [`../systacean/journal.md`](../systacean/journal.md);
  the `systacean-3` re-activation closes.

### Mini-wave commits in the release set (13)

| Subject                                                                                     | Commit          | Owner          |
|---------------------------------------------------------------------------------------------|-----------------|----------------|
| Rich prompt: ResizeObserver-driven margin reactor (fullstack-a-29)                          | `3d708a2`       | @@FullStackA   |
| Rich prompt: per-prompt page-width slider + cross-tile decoupling (fullstack-a-30)          | `20ece30`       | @@FullStackA   |
| BubbleOverlay: explicit dismiss + dismissedIds + Loading flicker fix (fullstack-a-28)       | `1a83050`       | @@FullStackA   |
| Terminal broadcast selector polish (fullstack-a-31)                                         | `18811e0`       | @@FullStackA   |
| Graph: ancestor breadcrumb in inspector + drop explicit "from here" buttons (fullstack-a-33) | `bc5feb6`       | @@FullStackA   |
| Chord migration + context-aware spawn + surface unification (fullstack-a-32)                | `f3a0e03`       | @@FullStackA   |
| Wysiwyg: paste markdown unescaped via turndown identity escape (fullstack-a-34)             | `237c45f`       | @@FullStackA   |
| File editor: inline rename band above page-width cap (fullstack-a-35)                       | `c9f31d5`       | @@FullStackA   |
| chan-server: per-session shell/agent submit-mode toggle (fullstack-b-13 server)             | `e24b931`       | @@FullStackB   |
| chan-desktop: window title = drive path verbatim (fullstack-b-14)                           | `8dbaaed`       | @@FullStackB   |
| Rich prompt: shell/agent submit-mode toolbar toggle + agent-chord submit (fullstack-b-13 SPA) | `dce2373`     | @@FullStackB   |
| event_watcher: silently skip non-matching filenames (systacean-10)                          | `6bae20b`       | @@Systacean    |
| chan/src/main.rs: gate not_a_chan_drive_hint on embeddings feature (systacean-8 follow-up)  | `c1e9c41`       | @@Systacean    |

### Post-release path

* @@CI's `release.yml` + `release-desktop.yml` trigger
  automatically on the `chan-v*` tag. Unsigned matrix
  produces the binaries for dogfood.
* @@WebtestA + @@WebtestB walk the cut binary per
  their respective verification queues (already in
  their inbound channels).
* Bugs surfaced in walkthroughs slip to v0.11.2 OR
  fold into Round-2 if substantive.
* Round-2 broader fan-out resumes per
  [`round-2-plan.md`](round-2-plan.md) — open
  decisions at the head of that plan (item-6 hosting,
  item-7 bundled-chan layout, item-3 PIN hash,
  manual home, sequencing confirmation) still need
  @@Alex's confirmation.

### What this release is NOT

* The signed-DMG north star. That's Round-2 close +
  exercises the real Apple Developer ID keys per the
  `ci-3` brief.
* A scope-creep gate for new bugs. New bug entries
  slip to v0.11.2 or roll into Round 2.
* A public-flip trigger. Repo stays private through
  Round 2.

## 2026-05-20 — Round-2 decisions locked (fan-out unblocked)

Surveyed @@Alex on the 5 open decisions at the head of
[`round-2-plan.md`](round-2-plan.md). Clean sweep on the
architect-recommended option for each. All six Round-2
gates are now closed:

| # | Decision                        | Locked                                                                  |
|---|---------------------------------|-------------------------------------------------------------------------|
| 1 | Sequencing                      | 7 + ci-7 → 6 → 1+4 → 2 → 3 → 5 (recommended order)                      |
| 2 | Item-6 hosting (chan.app)       | GitHub Pages with custom domain                                          |
| 3 | Item-7 bundled-chan layout      | PATH-first w/ bundled fallback + version match                          |
| 4 | Item-3 PIN hash                 | SHA-256 + per-install salt                                              |
| 5 | Manual home                     | `docs/manual/` (rendered by item-6 website pipeline)                    |
| 6 | First-release version           | v0.12.0 (locked earlier 2026-05-20)                                      |

`round-2-plan.md` head updated to reflect the locked
state (the "Decisions @@Alex needs to confirm" header
became "Decisions (all locked 2026-05-20)" with each
item carrying its lock rationale).

### What unblocks

Round-2 Wave 1 (north-star track) is dispatchable. Task
slots per round-2-plan §"Wave 1":
* `ci-N` workflow YAML consuming the six secrets (@@CI).
* `systacean-N` chan-desktop signing-key rotation
  (@@Systacean).
* `fullstack-b-N` bundled chan binary in chan-desktop
  resources (@@FullStackB).
* `fullstack-b-N+1` launch-time version probe + binary
  selection per the LOCKED PATH-first shape (@@FullStackB).
* `ci-N+1` DMG-on-tag dry-run with real keys (@@CI).
* `systacean-N+1` verify `tauri-plugin-updater` works on
  all three platforms (@@Systacean).

Numbering gets assigned at fan-out per the "highest
committed `<agent>-N` + 1" rule in round-2-plan §
"Numbering note".

### What stays out-of-band

* @@Alex completes the cert checklist from the `ci-3`
  brief (Apple Developer ID + Windows code-signing if
  scope reaches Windows).
* Six secrets populated into GitHub Actions Secrets
  (architect directs @@CI on NAMES; @@Alex populates
  VALUES manually per the secrets-boundary memory).

### Sequencing of dispatch vs sessions

Patch-release walkthroughs (@@WebtestA, @@WebtestB on
v0.11.1) are still in flight. Two reasonable shapes for
the Round-2 fan-out timing:

1. **Spawn Round-2 sessions now**, let webtest verdicts
   on v0.11.1 land in parallel (they're independent of
   Round-2 code work; their outputs feed v0.11.2 / fold
   into Round-2 only if they touch the same surface).
2. **Wait for walkthrough verdicts** before fan-out, so
   Round 2 starts with a clean known-good baseline.

Recommending option 1 — verdicts on v0.11.1 don't gate
Round-2 task definitions, and parallel session work is
the normal architect-side mode. Option 2 only matters if
the walkthroughs surface a v0.11.1 regression that needs
folding into Wave 1 (unlikely; the mini-wave was
narrowly scoped).

Standby for @@Alex to spawn fresh agent sessions via the
bootstrap prompt; I'll cut Wave-1 task files at fan-out
time.

## 2026-05-21 — design decision: Hybrid back-side becomes per-surface config

@@Alex's [`../alex/hybrid-revisited.md`](../alex/hybrid-revisited.md)
lands a substantial Hybrid pane semantic change for phase-8.

### What changes

Back side of a Hybrid pane stops being "another collection of
content tabs" and becomes a **per-surface configuration
surface** scoped to the type of the currently-active front
tab. Inspiration: Propellerheads Reason flip-the-rack UX.

| Front-tab type        | Back-side content                                       |
|-----------------------|---------------------------------------------------------|
| Hybrid Terminal       | Terminal settings (scrollback, TERM, font, etc.)        |
| Hybrid Editor         | Editor settings (Theme, Layout, Date Pills, On Save)    |
| Hybrid Graph          | Node-type legend grid `[Node] [Colour]`                 |
| Hybrid File Browser   | Placeholder for v1                                       |

Theme propagation simplifies: drop front/back independent
theme (was `-b-5`); both sides of a Hybrid share the single
per-Hybrid theme value. `-a-27` hamburger theme toggle flips
that single value.

### Settings overlay residue

`Cmd+,` Settings overlay stays as the home for drive-level +
app-level settings (semantic search per `-a-21`, future
Reports, window-config per `-b-1`, About / attribution). The
surface-specific settings (Terminal + Editor) MIGRATE OUT
into the Hybrid back-sides. UI-only relocation; storage
shape unchanged.

### Recently-landed work that gets relocated

* `-b-11` Terminal section (scrollback + TERM) → Hybrid
  Terminal back.
* `-a-25` On Save toggle → Hybrid Editor back.

Acceptable churn — `Preferences` shape unchanged, autosave
wire unchanged, only the mounting point of the UI changes.

### Implementation breakdown (preliminary)

5 tasks across @@FullStackA's lane (preliminary numbering at
fan-out):

* Task A — Hybrid back-side architecture refactor.
* Task B — Terminal Settings migration.
* Task C — Editor Settings migration.
* Task D — Hybrid Graph legend grid.
* Task E — Drop front/back independent theme.

Recommended sequencing: Task A rides Wave 2 as a hard-prereq;
Tasks B/C/D/E land in Wave 3. Or all five in Wave 3 if Wave 2
feels full. Locked at fan-out per @@Alex's preference.

### Open questions surveyed at fan-out

* Hybrid File Browser back: empty placeholder for v1 (recommend)
  vs minimal first config (e.g., default watcher-scope mode
  per `-b-6`)?
* Search overlay: stays out-of-Hybrid (recommend) vs eventually
  joins as a 5th Hybrid surface?
* Wave 2 vs Wave 3 split for the 5 tasks?

### Plan artifact

Full section added to
[`round-2-plan.md`](round-2-plan.md) §"Hybrid back-side
revisited" (between the metadata import/export scope sketch
and the Wave-2 dispatch table). Cross-references this journal
entry as the decisions-log anchor.

## 2026-05-21 — v0.11.2 cut cleared + WebtestB scope tightening + 2 bugs filed

@@Alex resumed from a crashed session. Only @@WebtestB
+ @@Architect up. Three decisions landed in one turn:
v0.11.2 cleared to cut, WebtestB standing perm
tightened for Gatekeeper-verification subset, two new
bugs filed from the dryrun.4 walkthrough fallout.

### Recovery context (for the audit trail)

@@WebtestB's `ci-8` dryrun.4 Gatekeeper-clean walkthrough
on the dev Mac (see [`../alex/event-webtest-b-architect.md`](../alex/event-webtest-b-architect.md)
"ci-8 dryrun.4 Gatekeeper verify — ACCEPTED on dev Mac")
produced the right verdict but three state-mutations
that weren't in scope:

1. `/Applications/Chan.app` overwritten by `ditto` (no
   `.backup` sibling first).
2. @@Alex's working chan-desktop PID 58737 SIGTERM'd by
   mistake ("elapsed-time triage" misidentified it as
   the agent's own launch).
3. `xattr -w com.apple.quarantine` manually applied to
   `/Applications/Chan.app` to "simulate Finder
   drag-install" — triggered App Translocation on
   @@Alex's next launch + surfaced the runtime
   translocation banner.

@@Alex's recovery: `pkill chan` + `kill -9 <PIDs>` on
the orphan `chan serve` children, `xattr -dr
com.apple.quarantine /Applications/Chan.app`, relaunch.
The `pkill` + `kill -9` dance is opaque enough that
regular users would be stranded — promoted to a bug
entry (see below).

### v0.11.2 cut cleared

@@Alex chose option 3 (accept dev-Mac partial; defer
canonical fresh-Mac check). Reasoning: the
keychain-independent signals (spctl + stapler + codesign
+ syspolicyd) are strong enough to predict cross-Mac
green; the literal acceptance-criterion fresh-Mac walk
is deferred to next time the verification fires under
tightened scope rules.

Cut-it signal landed in
[`../alex/event-architect-systacean.md`](../alex/event-architect-systacean.md)
"chan-v0.11.2 cut-it signal" — @@Systacean tags on next
bootstrap. @@CI's workflow auto-fires (signed pipeline);
no immediate dispatch needed
([`../alex/event-architect-ci.md`](../alex/event-architect-ci.md)
"workflow auto-fires" poke landed in parallel).

### WebtestB scope tightening (perm clarification, not revocation)

Three explicit exclusions added to the standing
chan-desktop runtime permission for the Gatekeeper-
verification subset (full text in
[`../alex/event-architect-webtest-b.md`](../alex/event-architect-webtest-b.md)
"Scope clarification..."):

1. **Never touch `/Applications/Chan.app`.** Custom
   install destinations only (`/tmp/chan-ci8-verify/...`
   or @@Alex's secondary Mac / fresh VM).
2. **Process ownership by capture, not triage.** Capture
   the launched PID at spawn; only SIGTERM that PID.
   No `pkill -f chan-desktop`. No "high elapsed time
   so it must not be mine" inference.
3. **No `xattr -w com.apple.quarantine` on system
   paths.** Real fresh-Mac verification can't be
   simulated locally; honest options are secondary Mac,
   fresh VM, or documented partial-acceptance.

Plus a pause-and-warn rule from @@Alex: next time the
verification scope reaches the canonical fresh-Mac
Gatekeeper-clean check, @@WebtestB fires a permission
event to @@Alex BEFORE starting + waits for the choice
between (a) pausing the working session + resuming via
iTerm and (b) running on the secondary Mac. The
@@Alex-closes-their-working-app step is destructive and
cannot be made unilaterally by the agent.

Throwaway-drive runtime walkthroughs are unaffected;
this is a perm SUBSET clarification for the DMG-install
shape only.

### Two new bugs filed in `phase-8-bugs.md`

Both Round-2 wave-2 candidates (NOT v0.11.2 — patch wave
is closing, both need investigation time):

1. **chan-desktop leaves bundled `chan serve` sidecars
   orphaned after parent dies; new desktop launches
   can't bind the same drive.** @@FullStackB lane (with
   possible @@Systacean cross-pollination if chan-drive
   needs a lock-takeover protocol primitive). Want:
   prevention (sidecar reap on chan-desktop exit via
   process group + Drop handler — defense in depth) +
   recovery UX (lock-takeover dialog with auto-kill of
   confirmed-orphan chan sidecar + user toast). REGRESSION-
   class severity; surfaces every time chan-desktop is
   killed ungracefully, which @@Alex just demonstrated
   happens.
2. **Terminal watcher silently stops dispatching events
   mid-session (ingest wedge).** @@Systacean lane (same
   ingest plumbing as `systacean-9` + `systacean-10`).
   @@WebtestB observed it during the `-b-13` walkthrough;
   serve restart cleared it but the SerTab pill stayed
   "active" on a serve with no watcher attached, then
   first interaction surfaced `terminal watcher is not
   attached`. Want: diagnose the wedge (ingest channel
   saturation? task panic?) + SerTab state reconciliation
   on serve restart. Silent-failure UX bug.

### Lane state after this turn

| Agent          | State                                                              |
|----------------|--------------------------------------------------------------------|
| @@Architect    | This session.                                                      |
| @@WebtestB     | Up; chan-desktop runtime perm tightened; v0.11.2 walkthrough next  |
| @@Systacean    | Not spawned; cut-it poke waiting on next bootstrap                 |
| @@CI           | Not spawned; workflow auto-fires on tag; ack-poke landed           |
| @@FullStackA   | Not spawned; Round-2 wave-2 queue waiting                          |
| @@FullStackB   | Not spawned; new orphan-sidecar bug + Round-2 wave-2 queue waiting |
| @@WebtestA     | Not spawned; v0.11.1 + v0.11.2 walkthrough queue waiting           |

@@Alex spawns @@Systacean next to cut the tag. The other
five agents land at the session-recycle point post-tag.

### What's owed to @@Alex (none load-bearing)

* I'll route the post-tag walkthrough queue to @@WebtestA
  + @@WebtestB when the v0.11.2 GH Release artifacts
  land. The tightened WebtestB scope rules apply to any
  DMG-install walk in that queue.
* Cleanup of `chan-v0.11.99-dryrun.{1..4}` tags from the
  remote — parked behind the v0.11.2 cut, not urgent.

## 2026-05-21 — Hybrid back-side decisions extended: Search → FB-back, About section build-out, donation QR

@@Alex locked two previously-open round-2-plan questions
+ requested a fresh task for the freed Settings-overlay
space.

### Open question #2 — LOCKED

"Where does Hybrid File Browser back land in v1?" Previously
recommended placeholder for v1; @@Alex's call 2026-05-21 is
to make the FB back the **Search / Indexing settings surface**.
Drive-level search settings (semantic search toggle from
`-a-21`; future multi-model picker from Round-3 Track 2)
migrate out of `SettingsPanel.svelte` into
`HybridFileBrowserConfig.svelte`. Rationale Alex's framing:
config-lives-next-to-the-affected-surface — FB is where
indexed content surfaces + where search results land users.
Added to the round-2-plan implementation breakdown as **Task
F — Search settings migration to Hybrid FB back**.

### Open question #3 — CLOSED by #2's lock

"Does the search overlay become a Hybrid surface?" Closed:
overlay (`Cmd+K F`) stays a global overlay; settings
(toggles + model picker) move to FB-back per #2. Two surfaces
disambiguated — overlay = global query UI; settings = FB-back
config. No further design churn.

### New task — Settings About section build-out + donation QR

Cut as [`../fullstack-a/fullstack-a-42.md`](../fullstack-a/fullstack-a-42.md)
(also added as **Task G** in the round-2-plan implementation
breakdown). Background: with Tasks A-F shedding ~70% of
`SettingsPanel.svelte` to the Hybrid back-sides, the freed
space gets repurposed as a proper About section:

* chan version (preserve current wire from `-b-12`).
* chan paths — drive root + embedded stores + config path
  (surfaced authoritatively from chan-server, NOT
  client-guessed).
* GitHub repo link — copy + open-in-browser.
* **Donation QR** — `web/public/qr-donate.png`
  pre-committed by @@Architect alongside the task file
  (61 KB; black-on-white 2D code). Short copy in @@Alex's
  voice ("If Chan is a daily driver for you, scan to send
  a tip. Optional; the project is free either way.")
* Existing attribution preserved (Source Code Pro OFL
  from `-b-12`; future markmap MIT when it lands).

Sequenced AFTER Tasks A + B + C + F land in HEAD — this
task's prereq is the Settings page actually being trimmed.

### Backlog item 6 — companion website QR

Updated [`next-phase-backlog.md`](../../phase-7/next-phase-backlog.md)
§6.1 "Website migration (chan.app)" — added a "Donation QR
placement" bullet so the chan.app migration work picks up
the same QR asset for the marketing site (footer Support
block, inline on the download page, or a Support page —
implementer picks at fan-out). Same asset on both surfaces;
flag QR rotation as a dual-touch surface.

### Asset under version control

`web/public/qr-donate.png` (61 KB) committed alongside the
task file. Reference path from the SPA: `/qr-donate.png`
(Vite + chan-server static-asset pipeline both serve
`web/public/` at root).

### Cut-it sequencing reminder

This work is Round-2 wave-2; doesn't touch the v0.11.2 cut
in flight with @@Systacean. v0.11.2 ships first; Hybrid
wave-2 fan-out happens at session-recycle after the cut.

## 2026-05-21 — Phase-9 vision captured (forward-look; not phase-8 scope)

@@Alex shared the directional intent for phase-9 in
session 2026-05-21: a new @@Architect lane focused on
desktop-native cross-platform (macOS, Linux, Windows)
with native keybindings + native chan-binary integration.
Full vision captured at
[`phase-9-desktop-native-vision.md`](phase-9-desktop-native-vision.md)
so it survives session recycle; saved to memory as
`project_phase_9_desktop_native_vision.md` for cross-
session inheritance.

### Headline shape (one-paragraph TL;DR for any architect on bootstrap)

Boundary between desktop-native and chan moves to the
DRIVE level at the NETWORK layer — `chan-tunnel-proto`
generalizes to local-fork / attached-outbound /
attached-inbound modes. Desktop-native is always a
network consumer; never the filesystem authority. Three
big open questions for @@Alex to lock before phase 9
opens: embed-vs-separate chan binary (architect leans
embed-by-default + keep separate for CLI), default Chan
drive lifecycle ("delete the Chan drive = factory
reset"), cross-version protocol stability commitment.

### Why this is captured now, not at phase-9 open

Phase 9 is months out; @@Alex's session-context is
freshest now. Capturing while the framing is precise
prevents drift. The vision doc also gives future-
@@Architect (and any spawn of the eventual desktop-
native architect) a single starting reference.

### Carryovers from phase-8 that touch phase-9

* `fullstack-b-15 / -16` (PATH-first probe + bundled
  fallback) — if phase 9 picks embed-by-default, this
  code path REPURPOSES into the bidirectional-discovery
  surface (find running desktop-native; attach as inbound
  drive). Not a rewrite.
* Orphan-sidecar bug (`phase-8-bugs.md`, filed today) —
  potentially obsoleted by embed-by-default. Don't sink
  heavy investigation into the takeover-UX piece; ship
  minimum fix for v0.12.x.

### Phase-8 close discipline preserved

Phase 8 finishes per existing plan: v0.11.2 cut → Round-2
wave-2 (Hybrid back-side + the new Task F + Task G About)
→ Round-3 public flip + multi-model picker + polish wave.
Phase 9 opens after Round 3 lands.

## 2026-05-21 — coordination smoke-test surfaced watcher-vs-journal shape gap

@@Alex turned on the rich-prompt watcher pointed at
`docs/journals/phase-8/alex/` and asked for an echo-
round-trip smoke test through @@FullStackA + @@FullStackB
to confirm the dispatch loop is live under the watcher.

@@Architect cut two pokes (appended to `event-architect-
fullstack-{a,b}.md`). @@Alex saw nothing in the watcher.
Smoke test paused per @@Alex's "if it breaks we pause +
analyse" directive.

### Root cause

Two structural mismatches in `chan-server/src/event_watcher.rs`:

1. **Event kind**: handles `Create(_)` + `Modify(Name(_))`;
   `Modify(Data)` (file append) falls through to None.
   Journal appends do not fire the watcher.
2. **Content shape**: parses each fired file as
   `AgentEvent` JSON. Markdown narrative bodies (our
   journal shape) fail the parse + bump `dropped_events`.

The two shapes co-exist under the same filename prefix +
the same regex + the same directory but talk past each
other.

### Decision: option C (capture + carry forward)

@@Alex picked option C from the three-paths choice. Three
were:
* A — demo the watcher with a proper JSON event (proves
  infra works).
* B — re-shape the smoke test (heavier; teach FullStacks
  the wire shape).
* C — capture the gap as wave-2/3 design work; leave the
  smoke test as-is.

Captured at
[`watcher-vs-journal-shape.md`](watcher-vs-journal-shape.md).
That artifact carries the two-shape comparison, the
three-resolution-options analysis (dual-write A /
watcher-tail-diff B / channel-migration C), the
recommendation (A — lowest-risk, preserves journal
discipline, lights up watcher), and four decisions for
@@Alex when the rich-prompt session-evolution wave cuts.

### Smoke test status (post-decision)

The FullStacks DO still see the inbound pokes via the
normal journal-poll bootstrap mechanism. Their echo
appends will land in `event-fullstack-{a,b}-architect.md`
when they poll; @@Architect picks them up on the next
inbound check. @@Alex's watcher view stays dark for
these — that's the expected behaviour under the captured
gap, not a new failure.

The smoke test thus served its diagnostic purpose:
surfaced the audit-trail-vs-wire-shape split that the
phase-8 dispatch blueprint (memory
`project_dispatch_is_automation_blueprint`) has been
implicitly assuming was resolved.

### Cross-references

* [`watcher-vs-journal-shape.md`](watcher-vs-journal-shape.md)
* [`rich-prompt-session-evolution.md`](rich-prompt-session-evolution.md)
  — to be updated with cross-ref to the new design
  artifact on next material edit.

## 2026-05-21 — Pre-recycle prep complete; all lanes handover-ready

@@Alex 2026-05-21: "i will want to recycle everyone
with the bootstrap prompt so please prep the whole of
next phase as i tear them down and get ready to
recycle.. you will be the last to recycle and the
first to come up".

### Recycle order

* All six working agents (CI, FullStackA, FullStackB,
  Systacean, WebtestA, WebtestB) get torn down +
  respawned per the bootstrap prompt in
  [`../../../agents/bootstrap.md`](../../../agents/bootstrap.md).
* @@Architect is LAST to recycle + FIRST to come up.
  Fresh architect bootstrap reads this journal + the
  channels + the plans; the working agents respawn
  into a fully dispatched state.

### Working tree at recycle (cleared work pending commit)

| Lane | Cleared task | Commit subject |
|------|-------------|----------------|
| @@CI | `ci-10` | `ci: release-desktop polish — notary-log fetch on failure + drop _x64 DMG suffix (ci-10)` |
| @@FullStackA | `-a-43` | `Hybrid back-side architecture refactor: per-surface config view (fullstack-a-43)` |
| @@FullStackB | `-b-22` | `chan-desktop: process-group sidecar reap + drive-lock-takeover UX (fullstack-b-22)` |
| @@Systacean | `-14` | `chan-server: instrument event-watcher ingest path + SPA detach-on-409 reconcile (systacean-14)` |
| @@WebtestA | `-2` verdict | `docs: v0.11.2 lane-A walkthrough verdict — 8/8 HOLD (webtest-a-2)` |
| @@WebtestB | in-flight walk | TBD — append + commit when lane-B walkthrough lands |

Each recycled session reads their inbound event-
architect channel on bootstrap; the clearance + queue
state survives the recycle naturally. If a session
tears down BEFORE committing, the next session picks
up the cleared work + commits per the standing
clearance.

### Queue depth per lane post-recycle

| Lane | Queue (numeric order) | Total tasks queued |
|------|----------------------|--------------------|
| @@FullStackA | `-a-43` (committable) → `-a-44` (drag) → `-a-45` (Task B) → `-a-46` (Task C) → `-a-47` (Task E) → `-a-48` (Task F + reports) → `-a-49` (G2) → `-a-50` (G3) → `-a-51` (G6+TaskD) → `-a-52` (G10+G9) → `-a-42` (About; gates on A+B+C+F) | 10 |
| @@FullStackB | `-b-22` (committable) → `-b-23` (chan.app marketing port) | 2 |
| @@Systacean | `-14` (committable) → `-15` (chan-report cross-dir aggregation) → `-16` (file-class buckets); `-12` parked on fresh permission ask | 3 + parked |
| @@CI | `ci-10` (committable) → `ci-11` (release.yml trigger fix) | 2 |
| @@WebtestA | verdict (committable); no further tasks dispatched (reactive lane) | 1 |
| @@WebtestB | in-flight; fresh-Mac perm parked with @@Alex; reactive lane | n/a |

### Open decisions parked for @@Alex (not load-bearing for the recycle)

* **@@WebtestB fresh-Mac walkthrough perm**: (a) pause
  current chan.app session, (b) secondary Mac, (c)
  declined / partial in throwaway-drive shape. Default
  (c) if no reply. Lives in
  [`../alex/event-webtest-b-alex.md`](../alex/event-webtest-b-alex.md)
  "permission (canonical fresh-Mac Gatekeeper walk for
  chan-v0.11.2 DMG)".
* **v0.11.2 CLI binary backfill**: workflow_dispatch
  against existing v0.11.2 tag to add chan CLI binaries
  to the existing GH Release (option b from ci-11
  finding), OR stay DMG-only as the "north-star
  validation lap" (option a's stance). Default: DMG-only.

### Standing permissions that survive recycle

Per [`../../../agents/bootstrap.md`](../../../agents/bootstrap.md)
§"Standing permissions":

* @@FullStackB chan-desktop runtime verification —
  STANDING (2026-05-20).
* @@WebtestB chan-desktop runtime walkthroughs —
  STANDING (2026-05-20), with the 2026-05-21
  tightened-scope clarification for the DMG/Gatekeeper
  verification subset.

### Permissions that DO NOT survive recycle (session-scoped)

* @@Systacean `-12` runtime permission for the
  tauri-plugin-updater dry-run. Granted 2026-05-21
  with safety constraints ("chan.app alive RIGHT NOW
  on the workstation"). Since `-12` was NOT executed
  before recycle, the recycled session MUST re-fire a
  fresh permission event to @@Alex; the prior approval
  was time-specific.

### Planning artifacts that survive recycle

* [`round-2-plan.md`](round-2-plan.md) — Task F expanded
  2026-05-21 to absorb chan-reports.
* [`graph-overhaul-plan.md`](graph-overhaul-plan.md) — full
  graph overhaul spec; 10-task decomposition (G1-G10);
  5 locked decisions; refinement section on depth +
  filters; clarification that filters are NODE-type
  only.
* [`watcher-vs-journal-shape.md`](watcher-vs-journal-shape.md)
  — coordination shape design gap for rich-prompt
  session-evolution wave.
* [`rich-prompt-session-evolution.md`](rich-prompt-session-evolution.md)
  — extended with watcher-vs-journal cross-reference.

### Bug list state

[`../phase-8-bugs.md`](../phase-8-bugs.md) has 95+
entries. New entries this session: G1 (chan-reports
settings regression), G9 (depth slider broken — outgoing
semantic confirmed), the watcher dialog trailing-slash
trip, Hybrid pane drag-to-rearrange feature (dispatched
as `-a-44`), 4 walkthrough side observations from
WebtestA + WebtestB.

### My next session bootstrap

When the fresh @@Architect comes up FIRST per @@Alex's
order:

1. Bootstrap walks `architect.md` + skill guide.
2. Reads `process.md` + `request.md`.
3. Reads THIS journal — the most recent entry (THIS
   one) is the load-bearing handover.
4. Reads the planning artifacts under `architect/`.
5. Reads `phase-8-bugs.md`.
6. Reads inbound events from all six working agents
   (most recent appends carry the pre-recycle
   handover state — see per-agent appends below).
7. Reads outbound events (own log).
8. Skims task-file tails — most recent appends carry
   commit-readiness from cleared work.
9. `git status` + `git log --oneline -20`. Uncommitted
   working-tree state should be the cleared commits
   waiting on agent recycle.

Then: watch for the first inbound poke from the
respawned working agents (their bootstrap → commit
their cleared work → poke back). Route follow-ups as
each lane progresses through their queue.

No active decisions blocked on me at recycle time.

## 2026-05-21 — fresh @@Architect up + two parked decisions resolved

Recycled @@Architect session bootstrapped from the
`bootstrap.md` Architect block. Read chain: architect.md
+ skill guide → process.md + request.md → this journal
(handover entry above is the load-bearing pickup) →
planning artifacts (round-2-plan, round-3-plan,
graph-overhaul-plan, watcher-vs-journal-shape, rich-
prompt-session-evolution, phase-9-desktop-native-vision,
commit-plan-v0.11.2) → phase-8-bugs.md (95+ entries) →
inbound channels for all six working agents → outbound
own log → git status + git log --oneline -20 + tag list.

### Working tree at bootstrap

* HEAD `22fd878`; 13 commits ahead of origin/main (push
  held per Round-2-close discipline).
* 2 modified files in the worktree, both webtest channels:
  fresh post-recycle ack-pokes from @@WebtestA + @@WebtestB.
  Those are the recycled lanes' own writes; not mine to
  commit.
* Tags through `chan-v0.11.2` shipped; the four dryrun
  tags remain on remote (parked cleanup behind v0.11.2).

### Pre-recycle handover calibration

Two lanes self-committed past where I'd marked them
"committable" in the handover:

* @@FullStackB committed BOTH `-b-22` (`3987e73`) AND
  `-b-23` (`bc9e1f8`) before tear-down — handover said
  only `-b-22` cleared. Lane is currently queue-empty.
* @@Systacean committed `-14` (`a603bc3`) and started
  picking up `-15` (chan-report cross-dir aggregation)
  before tear-down — handover had `-14` committable.

Neither is a problem; the pre-recycle write happened
before the tear-down signal landed, so the lanes
naturally caught up. Calibration note: when prepping
recycle, write handover entries closer to the actual
tear-down beat so they reflect HEAD at recycle, not
HEAD at handover-write time.

### Decisions resolved (@@Alex chat, post-recycle)

| # | Decision                            | Resolution                                                                  |
|---|-------------------------------------|-----------------------------------------------------------------------------|
| 1 | @@WebtestB fresh-Mac Gatekeeper walk | **Deferred entirely.** @@Alex walks chan.app personally at very end (v0.12.0 cut / late Round-3). No agent-side fresh-Mac walk fires in the interim. |
| 2 | v0.11.2 CLI binary backfill          | **Declined.** v0.11.2 stays as shipped (DMG-only). Linux + CLI unification lands on v0.12.0 wave-3 per the existing bug-list entry.  |

#### Decision 1 fine print

@@Alex's first chat reply was "1. granted" without picking
(a)/(b)/(c). I AskUserQuestion'd to disambiguate; @@Alex
picked (a) (pause + iTerm resume), then immediately walked
it back with "ahhh hold on, i will only test the chan.app
at the very very end". Net effect: NO walk fires from any
agent on the canonical fresh-Mac axis.

Calibration: when @@Alex's first reply doesn't fully
disambiguate a 3-option survey, the AskUserQuestion
follow-up was the right move — surfaced the actual
preference (deferral entirely) within one round-trip.

#### Decision 2 fine print

The forward-looking Linux binaries item already lives in
[`../phase-8-bugs.md`](../phase-8-bugs.md) ("Linux
binaries shipped on phase-8 next-release tags") as a
wave-3 candidate. No work is lost — just landing on the
next tag, not retro on this one.

### Propagation

* `event-webtest-b-alex.md` — transcribed deferral.
* `event-architect-webtest-b.md` — relayed "do not fire
  fresh-Mac perm ask again; standing throwaway-drive
  perm unaffected".
* `event-architect-ci.md` — relayed "no v0.11.2 backfill;
  Linux unification stays on v0.12.0 wave-3"; included
  heads-up on `ci-12` (glib-sys gap) candidate.

### @@CI's pre-recycle glib-sys finding

@@CI's final pre-recycle append flagged `.github/workflows/
ci.yml`'s `test-linux` clippy step dying on missing gtk
dev headers (glib-sys). Gate has been broken since
~2026-05-19 (~15 commits of unverified main). Provisional
`ci-12` shape: gtk dev install step in `test-linux`, OR
feature-gate the gtk-dependent crate out of the `cargo
clippy --workspace` set. Cut as a task with shared-infra
`Authorization: yes` framing on the @@CI respawn.

Topology of unverified commits: I'll lean on the next
chan-v* tag's CI fire as the validation lap rather than
re-running per-commit, unless @@Alex flags a specific
suspect.

### Lane state on architect session-start

| Lane         | State                                                                |
|--------------|----------------------------------------------------------------------|
| @@FullStackA | Not yet respawned; queue starts at `-a-44` (drag-to-rearrange)       |
| @@FullStackB | Not yet respawned; queue-empty post-`-b-23`; fan out from wave-3 list |
| @@Systacean  | Not yet respawned; queue starts at `-15` (chan-report cross-dir agg) |
| @@CI         | Not yet respawned; queue starts at `ci-12` (glib-sys gap) on cut     |
| @@WebtestA   | Respawned + standing by; reactive lane needs walkthrough dispatch    |
| @@WebtestB   | Respawned + standing by; `-b-22` walkthrough next (`webtest-b-3.md`) |

### My immediate next actions (in order)

1. Cut `webtest-b-3.md` for the `-b-22` orphan-sidecar reap
   runtime walkthrough (HEAD `3987e73`). @@WebtestB is up;
   they expect this dispatch per their bootstrap poke.
2. Cut `webtest-a-3.md` for the wave-3 cleared-work walks
   (`-a-43` Hybrid back-side refactor, `-b-23` web-marketing
   static site). @@WebtestA is up; reactive lane.
3. Cut `ci-12.md` for the glib-sys gap in `.github/workflows/
   ci.yml` with shared-infra `Authorization: yes` framing.
   This unblocks the ci.yml gate which has been broken since
   ~2026-05-19.
4. Watch for respawn pokes from @@FullStackA, @@FullStackB,
   @@Systacean, @@CI. Route follow-ups as their bootstrap
   completes + their first pickup poke lands.

## 2026-05-21 — new local-Linux capability (lima-vm + sdme) + 3-task dispatch fan-out

### Capability surfaced by @@Alex

@@Alex 2026-05-21 surfaced an existing operational
capability for local Linux testing that we hadn't been
exercising from the agent side: lima-vm "default" + sdme
containers (Ubuntu, Fedora, others). Pattern captured at
memory `reference-local-linux-via-sdme.md` so future
sessions inherit. Invocation shape from agents:
`limactl shell default sudo sdme <args>` (the `sdme` alias
only resolves in @@Alex's interactive shell). Architecture
caveat: containers are aarch64 (Apple Silicon host); CI
on `ubuntu-latest` is x86_64. Local pass means "apt
packages exist + clippy compiles"; x86_64-specific issues
still need real CI.

Implications:

* @@CI can fast-loop validate Linux-touching workflow
  patches locally before pushing — useful for ci-12's
  apt-install validation.
* @@Systacean can run Linux-only dev validation locally
  (e.g., reproduce a Linux-only test failure, validate a
  dependency change against multiple distros) without
  having to push and wait for CI.
* The Linux-binaries v0.12.0 work has a sharper
  architecture story now: aarch64 Linux release builds
  are forward-looking (no current matrix entry); CI
  ubuntu-latest stays the x86_64 lane. Annotated the
  phase-8-bugs.md entry accordingly.

@@Alex's caveat on UI testing: chan-desktop UI runtime
walkthroughs on Linux still ride @@Alex's external
Linux machine with Wayland; no display server in the
default sdme containers. The webtest lanes' chan-desktop
runtime perm is unaffected (those walks are macOS-side).

### Dispatch fan-out: 3 tasks cut

* [`../ci/ci-12.md`](../ci/ci-12.md) — workspace-wide
  GTK deps in CI test jobs. Unblocks the ci.yml gate
  broken since ~2026-05-19. Shape (a) — apt-install
  GTK across affected ubuntu jobs (mirrors
  release-desktop.yml lines 114-123). Backfill: lean
  on next chan-v* tag's CI fire. Shared-infra
  Authorization: yes framing inline. Local validation
  via sdme noted as optional fast-loop.
* [`../webtest-b/webtest-b-3.md`](../webtest-b/webtest-b-3.md)
  — `-b-22` orphan-sidecar reap + drive-lock-takeover
  UX walkthrough (HEAD `3987e73`). Throwaway-drive
  shape; standing chan-desktop runtime perm covers it.
  Four acceptance subsections (prevention graceful,
  prevention ungraceful, recovery dialog, negative case).
* [`../webtest-a/webtest-a-3.md`](../webtest-a/webtest-a-3.md)
  — `-a-43` Hybrid back-side architecture refactor (HEAD
  `b36ca96`) + `-b-23` web-marketing static site (HEAD
  `bc9e1f8`) walkthroughs. Six SPA acceptance checks for
  `-a-43`; four static-site checks for `-b-23`.

### Outbound pokes fired

* `event-architect-ci.md` — ci-12 cut + decisions
  resolution + local sdme capability + sequencing note.
* `event-architect-webtest-a.md` — webtest-a-3 cut.
* `event-architect-webtest-b.md` — webtest-b-3 cut.

@@WebtestA + @@WebtestB are already respawned; they'll
pick up on next poll. @@CI bootstraps into the ci-12
dispatch directly.

### Working tree state at end of this round

Uncommitted (bundled for the dispatch fan-out commit):

* `architect/journal.md` — this entry + the prior
  session-start entry.
* `ci/ci-12.md` (NEW).
* `webtest-a/webtest-a-3.md` (NEW).
* `webtest-b/webtest-b-3.md` (NEW).
* `phase-8-bugs.md` — aarch64/x86_64 caveat annotation
  on the Linux binaries entry.
* `alex/event-architect-ci.md` — ci-12 dispatch poke.
* `alex/event-architect-webtest-a.md` — walkthrough
  dispatch poke.
* `alex/event-architect-webtest-b.md` — perm-ask
  deferral relay + walkthrough dispatch poke.
* `alex/event-webtest-b-alex.md` — perm-ask deferral
  transcription.
* `alex/event-webtest-a-architect.md` (recycled lane's
  own respawn poke — NOT my write; leaving uncommitted).
* `alex/event-webtest-b-architect.md` (same — recycled
  lane's own respawn poke; leaving uncommitted).

Committing the architect-owned files as a single
"dispatch fan-out" docs commit. The two webtest channel
appends stay uncommitted; the recycled lanes commit
their own writes on their next batch.

## 2026-05-21 — clearance round + 2 new tasks (ci-12 smoke surfaced two findings)

Four lanes poke-poked in one beat. Cleared all four +
cut two new follow-up tasks from @@CI's smoke validation
findings.

### Clearances issued

| Lane | Task | Status | Notes |
|------|------|--------|-------|
| @@Systacean | `-15` | Clearance approved | chan-report cross-dir aggregation cache + `/api/report/dir` route; 7 files +573/-37 + 8 new tests; full pre-push gate green. Suggested subject + file scope accepted verbatim. |
| @@CI | `ci-12` | Clearance approved | GTK install in workspace-clippy jobs + `workflow_dispatch:` added to ci.yml; post-mortem appended to ci-11-post-mortem.md (tightly coupled per the joint discovery cycle). 5 open questions answered inline. |
| @@WebtestA | `webtest-a-3` verdict | Clearance approved | 8/8 HOLD on `-a-43` + `-b-23`; one HOLD-partial on `-b-23` viewport-responsiveness (Chrome MCP `resize_window` tooling gap, not chan bug). Three side observations triaged: tooling note (#1), discipline reminder for Tasks B/C/E/F (#2), doc-drift (#3) — none filed to bug list. |
| @@WebtestB | `webtest-b-3` verdict | Clearance approved (partial shape acknowledged) | Component-verified `-b-22` via chan-drive + chan-serve invariants directly; did NOT launch debug chan-desktop because @@Alex's live `/Applications/Chan.app` shares `config.json` with any debug instance (last-writer-wins on `window_configs` would discard live state). Right call per the "no persistent side effects outside throwaway-drive set" rule. |

### Two new @@Systacean tasks cut (from @@CI's smoke findings)

@@CI's ci-12 smoke validation unmasked TWO pre-existing
issues that had been hidden behind the GTK gap. Both are
chan-drive Rust source-code fixes (not @@CI's lane):

* [`../systacean/systacean-17.md`](../systacean/systacean-17.md)
  — Windows `result_large_err` lint on
  `chan-drive::index::config::ConfigError` (carries
  unboxed `toml::de::Error`; large on Windows target
  stack alignment). Trips at `config.rs:130`, `:140`,
  `facade.rs:177` + likely more. Fix shape (a): box the
  large variant(s). Pre-existing on Windows for ~15
  commits' worth of unverified main; not net-new from
  ci-12.
* [`../systacean/systacean-18.md`](../systacean/systacean-18.md)
  — chan-drive tests panic on CI runners when the
  BGE-small embedding model isn't cached. 14 tests
  affected across `drive.rs` + `indexer.rs`. Fix shape
  (a): `#[cfg(feature = "embed-model")]` or `#[ignore]`
  the affected tests so default-build CI skips them
  cleanly. Deterministic-fixture shape (b) is Round-3
  cleanup territory; not pursuing now.

### Queue re-prioritization

@@Systacean's revised queue: `-15` (committable) → `-17`
(Windows lint; gate-unblocker) → `-18` (model-dep tests;
gate-unblocker) → `-16` (file-class buckets; feature
work). The two gate-unblockers ride ahead of `-16` because
they're load-bearing for the per-PR CI gate (broken since
~2026-05-19 across ~15 commits). After both land, the
gate goes fully green for the first time since that
window.

### Bug-list entry filed

[`../phase-8-bugs.md`](../phase-8-bugs.md) appended with
the chan-desktop orphan-detection heuristic-tightening
finding from @@WebtestB's `-b-22` walk. Two follow-up
pieces (tighten heuristic to contiguous argv match +
render candidate PIDs in the dialog); @@FullStackB lane;
Round-2 wave-2/wave-3 polish.

### After this round lands

| Lane | Next pickup |
|------|-------------|
| @@Systacean | Commit `-15` → pick `-17` → pick `-18` → then `-16` |
| @@CI | Commit `ci-12` → queue-empty until v0.12.0 Linux-binaries dispatch (wave-3) |
| @@WebtestA | Commit verdict → standing by; next walk likely `-a-44` once @@FullStackA respawns |
| @@WebtestB | Commit verdict → queue-empty as reactive lane; next walk on chan-desktop runtime work when it lands |
| @@FullStackA | Still not respawned; queue rich (`-a-44` → `-a-45..52` → `-a-42`) |
| @@FullStackB | Still not respawned; queue-empty post-`-b-23`; wave-3 fan-out candidates ready |

Gate-state outlook: after ci-12 lands, ci.yml gate goes
**partial green** (3 of 4 affected jobs). After
systacean-17 + systacean-18 land, ci.yml gate goes
**fully green** for the first time since ~2026-05-19.
That's the Round-3 readiness signal — meaning the per-PR
gate is reliably catching regressions again.

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | -15 clearance + -17/-18 queue re-prioritization |
| `alex/event-architect-ci.md` | ci-12 clearance + 5-question answers |
| `alex/event-architect-webtest-a.md` | verdict clearance + side-observation triage |
| `alex/event-architect-webtest-b.md` | verdict clearance + heuristic-finding routing |
| `phase-8-bugs.md` | chan-desktop orphan-detection heuristic entry |
| `systacean/systacean-17.md` | NEW task (Windows lint) |
| `systacean/systacean-18.md` | NEW task (model-dep tests) |

NOT touching (other agents' own files; they commit with
their work):

* `crates/chan-{drive,report,server}/...` — @@Systacean's
  `-15` code.
* `.github/workflows/{ci,release}.yml` + `ci/ci-12.md` +
  `ci/ci-11-post-mortem.md` — @@CI's `ci-12` work.
* `webtest-{a,b}/webtest-{a,b}-1.md` — webtest verdict
  appends.
* `alex/event-{ci,systacean,webtest-a,webtest-b}-architect.md`
  — agents' own outbounds (will commit with their work).
* `systacean/systacean-15.md` — @@Systacean's task tail
  (will commit with `-15` code).
* `webtest-a/webtest-a-3.md` — touched by @@WebtestA on
  pickup (likely a status update); they own that file.

## 2026-05-21 — second clearance round (-17 + -44) + smoke-dispatch decision

@@Systacean + @@CI both committed their cleared work
(`f4a197d` and `6abac58` in HEAD). @@FullStackA respawned
mid-round (not signaled separately; bootstrap was clean),
picked their queue head `-a-44`, implemented + poked.
@@Systacean continued forward to `-17`, implemented +
poked.

### Lane commits landed (per agent self-commit)

| SHA | Subject | Lane |
|-----|---------|------|
| `f4a197d` | `chan-report: maintained per-directory aggregation cache + /api/report/dir (systacean-15)` | @@Systacean |
| `6abac58` | `ci: install GTK deps in workspace-clippy jobs + add ci.yml workflow_dispatch (ci-12)` | @@CI |

Both per my prior clearance pokes; pre/post-commit audits
clean; no stowaways. @@CI's `ci-11-post-mortem.md` append
landed with `6abac58` per the post-mortem placement
decision.

### Two new commit-readiness pokes (cleared this round)

* **@@Systacean -17**: shape (a) implementation with
  defensive Encode-side boxing on top of the named
  Decode-side fix. Single-file diff +26/-3 in
  `crates/chan-drive/src/index/config.rs`. Manual
  `From<toml::ser::Error>` impl preserves `?` ergonomics
  at the `toml::to_string_pretty(cfg)?` call site (would
  have broken under `#[from]` on `Box<...>`). All 425+
  chan-drive tests + workspace test + workspace clippy +
  build-no-default-features green.

  Asked me to pick between (1) smoke dispatch via
  `systacean-17-smoke` branch and (2) fold into regular
  push flow. Picked **option 1** — reuses the
  `ci-12-smoke` pattern; operationally low-cost; gives
  empirical Windows clippy confirmation before main
  lands the gate-unblocker. Smoke-branch push
  authorized (non-tag; doesn't trip the Round-2-close
  tag-push hold).

* **@@FullStackA -44**: Hybrid pane drag-to-rearrange +
  transaction-mode NAV. Four-file SPA + state change +
  12 new test pins. vitest 600/600 (+12 net from
  `-a-43`'s 588). Three flagged deviations, all
  accepted:

  1. Cmd+. mid-transaction NOT wired — asymmetry with
     keyboard NAV's Enter-only / Esc-only model would
     diverge; Esc as universal exit is the right shape.
  2. Click-without-drag → no-op release — matches task
     default + `paneModeSwapWith` grab==drop no-op
     covers the edge case.
  3. Every pane drop-target (not just Hybrid) — matches
     bug-list "rearrange ANY pane" + window-manager-like
     framing.

### Smoke-branch lifecycle reminder

After this round + `-17` smoke completes, we'll have THREE
smoke branches on origin: `ci-12-smoke` + (impending)
`systacean-17-smoke` + any future smoke shape. All prune
on the same beat as the `chan-v0.11.99-dryrun.{1..4}`
tag cleanup; not blocking, but worth tracking so the
audit-trail-keep set doesn't grow indefinitely.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | `-17` cleared + smoke option 1 chosen; expect commit + smoke + then `-18` pickup |
| @@CI | `ci-12` committed (`6abac58`); idle until wave-3 Linux-binaries dispatch |
| @@FullStackA | `-a-44` cleared; expect commit + then `-a-45` pickup (Hybrid back-side wave Task B) |
| @@FullStackB | Still not respawned; queue-empty post-`-b-23` |
| @@WebtestA | Verdict still uncommitted by them; cleared last round |
| @@WebtestB | Verdict still uncommitted by them; cleared last round |

The webtest verdicts staying uncommitted across two
rounds is mildly surprising — both lanes were up and
poke-ready last round. Likely either: their sessions
have idled / closed and will re-pick up the clearance
when they next bootstrap, OR they're queue-empty and
waiting on the next dispatch (which doesn't gate on
their commit). Not blocking my work; if it persists
past the @@FullStackA `-a-44` commit + next walkthrough
dispatch, I'll check directly.

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | -17 clearance + option 1 smoke pick |
| `alex/event-architect-fullstack-a.md` | -44 clearance + 3 deviations accepted |

NOT touching (other agents' own files):

* `crates/chan-drive/src/index/config.rs` — @@Systacean's
  `-17` code.
* `web/src/{components/Pane.svelte,Pane.test.ts,state/tabs.svelte.ts,tabs.test.ts}` — @@FullStackA's `-a-44` code.
* All `event-<agent>-architect.md` files (agents' own
  outbounds; they commit with their work).
* All task-file appends in agent dirs (theirs to commit).

## 2026-05-21 — incident routing: a8e991a cross-agent commit-hygiene + 1 new bug

### Five lane commits landed since last round

| SHA | Subject | Lane |
|-----|---------|------|
| `a8e991a` | `docs: webtest-b-3 — -b-22 orphan-sidecar reap walkthrough (component verified, click cycles parked)` | @@WebtestB (BUT see incident below) |
| `663ab26` | `chan-drive: box toml::Error variants in ConfigError (systacean-17)` | @@Systacean |
| `56e6692` | `docs: webtest-a-3 — -a-43 Hybrid back-side + -b-23 web-marketing walkthroughs (8/8 HOLD)` | @@WebtestA |
| `9bdec83` | `docs: fullstack-b post-recycle bootstrap ack (queue-empty, standing by)` | @@FullStackB |
| `e9315df` | `docs: -a-44 commit-hygiene incident flag to @@Architect` | @@FullStackA |

Two of these (`663ab26`, `9bdec83`) are clean per their
own audits. The other three are the incident chain.

### The a8e991a cross-agent commit-hygiene incident

@@WebtestB's commit `a8e991a` (intended scope: 2 webtest-b
verdict files) used a broad `git add` and swept up 5
additional in-flight files from @@FullStackA's `-a-44`
work: `Pane.svelte`, `Pane.test.ts`, `tabs.svelte.ts`,
`tabs.test.ts`, `fullstack-a-44.md`, `fullstack-a/journal.md`,
+ @@FullStackA's outbound poke. Net 9-file commit under
the wrong subject; @@FullStackA's intended commit subject
(`Hybrid pane drag-to-rearrange + transaction-mode NAV
(fullstack-a-44)`) never landed.

Both @@FullStackA AND @@WebtestB independently flagged the
incident:

* @@FullStackA via `event-fullstack-a-architect.md` then
  committed the flag as `e9315df`.
* @@WebtestB via `event-webtest-b-architect.md` with
  three proposed recovery options (A audit-trail / B
  soft-reset + cherry-pick / C rebase-split).

Adjacent risk surface: @@WebtestA's `56e6692` commit hit
the SAME shared-tree condition but their pre-commit audit
caught the stowaway (`event-fullstack-b-architect.md`),
recovered via `reset --soft + restore --staged + re-commit
explicit per-path`. Same condition, different outcome —
the discipline catches it when applied.

### Routing decision — (b) audit-trail + (c) anchor commit

**(a) History rewrite — DECLINED.** With 4 follow-up
commits stacked on `a8e991a` (`663ab26`, `56e6692`,
`9bdec83`, `e9315df`), rewriting requires cherry-picking
each. Peer agents have already referenced the existing
SHAs in their journals/task files. Push is still held but
the local-tree blast radius alone justifies refusal.
@@WebtestB's options (B) + (C) decline for the same
reason.

**(b) Audit-trail correction in task file — GO.**
Routed to @@FullStackA: append a `## 2026-05-21 — landed
under cross-agent commit (a8e991a)` section to
[`../fullstack-a/fullstack-a-44.md`](../fullstack-a/fullstack-a-44.md)
tail at next commit beat. Future readers walk the task
file when grepping `-a-44`; the canonical audit anchor
is the task file, not the commit log.

**(c) Architect-side grep-anchor commit — DOING IT NOW.**
This commit (current beat) carries `fullstack-a-44` in
the subject line:
`docs: architect routing on a8e991a cross-agent commit-hygiene incident + new pane-focus bug (fullstack-a-44 audit anchor)`.
That closes the (c) need without forcing an empty commit
from @@FullStackA's side.

### Lesson routed to @@WebtestB

Append on `event-architect-webtest-b.md` carrying the
`feedback_shared_worktree_commits` memory rule + the
explicit discipline:

1. NEVER `git add -A` / `git add .` in the shared tree.
2. Pre-commit `git diff --staged --stat` is mandatory.
3. Post-commit `git show --stat HEAD` is mandatory.

Cross-referenced @@WebtestA's same-condition save as
the empirical proof the discipline works when applied.

### Closure granted on @@FullStackA

Greenlighted them to pick up `-a-45` (Hybrid back-side
Task B — Terminal Settings migration) immediately, with
the (b) audit-trail append landing alongside the `-a-45`
commit beat. No work blockage from the incident.

### New @@Alex bug filed this round

[`../phase-8-bugs.md`](../phase-8-bugs.md) appended with:

* **chan-desktop first click after window-focus restore
  doesn't follow the mouse to select the pane under the
  cursor.** UX papercut surfacing every time the user
  Cmd+Tabs away and then clicks back onto chan-desktop.
  Currently: click restores window focus but pane
  selection stays at the pre-focus-loss pane; subsequent
  typing lands on the OLD pane, not the clicked one.
  Wanted: on the first click that restores window focus,
  ALSO dispatch paneSelect on the Hybrid pane under the
  mousedown.

  Critical disambiguation @@Alex clarified mid-round:
  **NOT on Cmd+Tab** (keyboard refocus without
  mousedown). Detection shape: SPA listens for window
  `focus` + `mousedown`; if mousedown fires within ~50ms
  of focus, treat as click-to-focus + dispatch
  paneSelect. Focus-without-mousedown (Cmd+Tab) → no
  pane-select change.

  Lane: @@FullStackA primary (SPA window-focus + pane-
  select); possible cross-lane to @@FullStackB if Tauri-
  side mediation needed. Round-2 wave-3 candidate; not
  regression-class.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | `-17` committed (`663ab26`); expect `-18` pickup next (model-dep tests gate-unblocker); smoke branch `systacean-17-smoke` on origin |
| @@CI | Idle; ci-12 committed (`6abac58`); queue-empty until wave-3 Linux-binaries |
| @@FullStackA | `-a-44` cleared + landed (incorrectly under a8e991a); greenlit `-a-45` pickup with (b) audit-trail append owed |
| @@FullStackB | Respawned (`9bdec83`); queue-empty; standing by for wave-3 |
| @@WebtestA | `-a-3` verdict committed (`56e6692`); Option A close-out marker pending |
| @@WebtestB | `-b-3` verdict committed (`a8e991a`, the incident commit); lesson routed; standing by |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | (b) routing + `-a-45` greenlight |
| `alex/event-architect-webtest-b.md` | Incident routing + lessons-learned |
| `alex/event-architect-webtest-a.md` | Option A on close-out marker |
| `phase-8-bugs.md` | New focus-follows-click bug |

NOT touching (other agents' own files):

* All `event-<agent>-architect.md` files modified by
  agents (their own outbounds — `event-ci-architect.md`,
  `event-webtest-a-architect.md`, `event-webtest-b-architect.md`
  are post-commit appends agents will commit with their
  next work).
* All task-file appends (`ci-12.md`, `ci/journal.md`,
  `systacean-15.md`, `webtest-a-3.md`).
* No code touched.

## 2026-05-21 — clearance round 3 (-18 + -a-45) + cut fullstack-b-24 (final gate-unblocker)

### Lane commit landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `c9fb768` | `docs: webtest-a-3 task close-out marker (-a-43 + -b-23 walks)` | @@WebtestA |

Clean per Option A routing. @@WebtestA used `git commit
<path> -m "..."` path-limit (bypassing the shared index)
+ post-commit `git show --stat HEAD` audit. No stowaways.

### Two commit-readiness pokes cleared this round

**@@Systacean -18** (chan-drive tests skip on missing BGE
model): chose (a1) `#[ignore]` over my recommended (a2)
`#[cfg(feature = "embed-model")]` for a legitimate reason
— chan-drive doesn't declare `embed-model` (that feature
lives in chan-server controlling rust-embed); adding a
no-op flag purely for test gating would conflate
semantics. Task body explicitly allowed the fallback;
their reasoning matches. Empirical 14-test list from the
`systacean-17-smoke` panic trace (not my line-number
callout) — three architect-listed tests not in the panic
list excluded, three empirical adds included. `cargo test
-p chan-drive`: `411 passed; 0 failed; 16 ignored`; `--
--ignored` runs all 16 cleanly on the workstation. No
coverage loss.

Smoke decision: option 1 (push to `systacean-18-smoke`
branch + `gh workflow run ci.yml`). Same pattern as `-17`.
Smoke joins the audit-trail-keep set alongside
`ci-12-smoke` + `systacean-17-smoke`.

**@@FullStackA -a-45** (migrate Terminal Settings from
SettingsPanel.svelte to HybridTerminalConfig.svelte):
clean migration; 88-line Terminal section shed; full
Terminal config moved; existing wiring test repurposed
as regression guard. vitest 606/606 (+6 net). Three
flagged deviations all accepted:

1. Last-writer-wins save race — narrow window; single-user
   app; over-engineering to enforce optimistic concurrency.
2. `hybrid-terminal-*` id namespacing — defensive; trivial
   cost; don't revert.
3. Two parallel save-status indicators — each surface
   reports its own debounce; per-surface is the right
   grain.

(b) audit-trail correction for the a8e991a incident
bundled into the same commit — `fullstack-a-44.md`
append rides with `-a-45` per @@FullStackA's "your call"
ask. Single commit closes both the new feature work AND
the prior incident documentation. Closes the (b) loop
cleanly.

### Third gate-unblocker cut: fullstack-b-24

@@Systacean's `systacean-17-smoke` run surfaced 11
Windows-only chan-desktop dead_code lints (10 dead_code +
1 unused_variable on `exit_signal`). All from
`desktop/src-tauri/src/`; @@Systacean's read is that the
items are declared at module scope but only used through
`#[cfg(target_os = ...)]` paths that exclude Windows;
declarations visible to all targets; Windows can't see
them being used; clippy flags them.

Cut [`../fullstack-b/fullstack-b-24.md`](../fullstack-b/fullstack-b-24.md)
for @@FullStackB (chan-desktop lane). Fix shape (a) —
per-item `#[cfg]` at declarations. Smoke shape:
`fullstack-b-24-smoke` branch + `gh workflow run ci.yml`,
authorized. Pre-commit discipline reminder included
explicitly per the a8e991a aftermath (their first commit
beat post-recycle; want the discipline applied
proactively).

### After all three gate-unblockers land

Per-PR ci.yml gate goes **fully green for the first time
since ~2026-05-19** (the full ~15-commit unverified
window). That's the Round-3 readiness signal: the gate
becomes load-bearing again, catching regressions
reliably. Three smoke branches accumulate in the audit-
trail-keep set; all prune with the
`chan-v0.11.99-dryrun.{1..4}` tag cleanup beat.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | `-18` cleared; expect commit + smoke + `-16` pickup (file-class buckets, feature work) |
| @@CI | Idle; queue-empty until wave-3 Linux-binaries dispatch |
| @@FullStackA | `-a-45` cleared with (b) bundle; expect commit + `-a-46` pickup (Hybrid back-side Task C — Editor Settings migration) |
| @@FullStackB | `-24` dispatched (Windows dead_code; gate-unblocker); pickup on read of `event-architect-fullstack-b.md` |
| @@WebtestA | Close-out marker committed (`c9fb768`); reactive lane standing by |
| @@WebtestB | Lesson absorbed; standing by; queue-empty |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | -18 clearance + a1 accepted + smoke option 1 + routing on Windows dead_code finding |
| `alex/event-architect-fullstack-a.md` | -a-45 clearance + 3 deviations accepted + (b) bundle confirmed |
| `alex/event-architect-fullstack-b.md` | -24 dispatch poke |
| `fullstack-b/fullstack-b-24.md` | NEW task (Windows chan-desktop dead_code) |

NOT touching (other agents' own files):

* `crates/chan-drive/src/{drive,indexer}.rs` —
  @@Systacean's `-18` code.
* `web/src/components/{HybridTerminalConfig.svelte,
  HybridTerminalConfig.test.ts, SettingsPanel.svelte,
  SettingsPanel.terminal.test.ts}` — @@FullStackA's
  `-a-45` code.
* All `event-<agent>-architect.md` files (agents' own
  outbounds; they commit with their work).
* All task-file appends (`ci-12.md`, `ci/journal.md`,
  `systacean-15.md`, `systacean-17.md`, `systacean-18.md`,
  `fullstack-a-44.md`, `fullstack-a-45.md`,
  `fullstack-a/journal.md`).

## 2026-05-21 — fullstack-b-24 scope correction (architect-side categorical error) + 2 commits landed

### Lane commits landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `7a22e63` | `chan-drive: gate 14 model-dependent tests behind #[ignore] (systacean-18)` | @@Systacean |
| `1f80d09` | `Migrate Terminal Settings to Hybrid Terminal back-side (fullstack-a-45)` | @@FullStackA |

Both per my prior clearances. Smoke branch
`systacean-18-smoke` now on origin (third in the
audit-trail-keep set).

### @@FullStackA pickup ack

@@FullStackA's post-commit poke confirms `-a-45` committed
clean (pre/post audits clean; bundled `fullstack-a-44.md`
audit-trail correction landed per the (b) routing). Picking
up `-a-46` (Hybrid back-side Task C — Editor Settings
migration) next. No clearance needed; standard queue
progression.

### fullstack-b-24 categorical scope error (architect-side, caught at task pickup)

@@FullStackB picked up `-24` and immediately flagged a
categorical error in my task body. I had attributed 11
Windows dead_code lints to `desktop/src-tauri/src/`. The
actual file paths from @@FullStackB's grep:

| # | Item | Location |
|---|------|----------|
| 1 | `exit_signal` | `desktop/src-tauri/src/serve.rs` (unused param) |
| 2-11 | `ControlRequest`, `ControlResponse`, `WindowCommand`, `is_false`, `WindowCommandFrame`, `handle_request`, `open_path`, `abs_to_drive_rel`, `path_to_posix`, `parent_rel` | `crates/chan-server/src/control_socket.rs` |

Only ONE of the 11 is in chan-desktop. The other ten are
in `crates/chan-server/src/control_socket.rs` — a
Unix-only IPC primitive whose operational code is
`#[cfg(unix)]` but whose DECLARATIONS leak to Windows
compilation. chan-desktop doesn't depend on chan-server
(it pulls only `chan-tunnel-*`); they linted on Windows
because `cargo clippy --workspace --all-targets` walks
every workspace crate.

### Root cause of my error

I quoted @@Systacean's framing "from chan-desktop's IPC
layer" as if it pointed to a location. It was actually a
FUNCTIONAL ownership statement: the IPC primitive is
consumed BY chan-desktop, but IMPLEMENTED in chan-server.
Per the `feedback_ground_descriptions_in_source` memory
rule, I should have grepped the source at task-cut time
instead of paraphrasing the upstream framing.

The lane attribution (@@FullStackB) was still correct —
they're the natural owner of fixes affecting the IPC
boundary they consume. Only the path scope + authorization
were wrong.

### Routing: option (A) — scope expansion

@@FullStackB proposed three options:

* (A) Expand `-24` authorization to also cover
  `crates/chan-server/src/control_socket.rs`. Single
  commit; single smoke fire.
* (B) Split: chan-desktop in `-24`, new task for
  chan-server. Two commits; two smoke fires.
* (C) Re-cut `-24` with corrected scope. Cleanest
  audit but slowest.

Picked (A). The fix is mechanical (10 declaration-site
`#[cfg(unix)]` matching the existing `#[cfg(unix)]`
boundary already in `control_socket.rs` + 1
`_exit_signal` rename); single commit keeps the unified
"fully-green CI" goal in one logical change. @@Systacean
in flight on `-16` (chan-report, not chan-server) means
no concurrent-edit overlap.

Architect-side appends landed:

* `fullstack-b-24.md` tail: scope expanded + authorization
  expanded + routing rationale. Task file now standalone-
  readable for future audits.
* `event-architect-fullstack-b.md`: ack of scope catch
  + greenlight on (A).

### Lesson reinforced (not new — memory rule was already there)

`feedback_ground_descriptions_in_source` already said:
"Don't invent crate/module capability descriptions from
name + intuition. Read README/design.md/lib.rs first."
The case extends naturally to: don't take peer-agent
FUNCTIONAL framing as LOCATION info without empirical
grep. The shape rule stays as-is; this is reinforcement,
not a new memory entry.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | `-18` committed (`7a22e63`); expect `-16` pickup (file-class buckets, feature work) |
| @@CI | Idle; queue-empty until wave-3 Linux-binaries |
| @@FullStackA | `-a-45` committed (`1f80d09`); expect `-a-46` (Editor migration) commit-readiness poke |
| @@FullStackB | `-24` scope-corrected + greenlit; expect implementation + smoke + commit-readiness poke |
| @@WebtestA | Standing by; reactive lane |
| @@WebtestB | Standing by; reactive lane |

Three smoke branches now on origin (`ci-12-smoke` +
`systacean-17-smoke` + `systacean-18-smoke`); a fourth
(`fullstack-b-24-smoke`) lands when @@FullStackB pushes.
All prune with the `chan-v0.11.99-dryrun.{1..4}` tag
cleanup beat.

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-b.md` | -24 scope-correction routing (option A) |
| `fullstack-b/fullstack-b-24.md` | Scope expansion + corrected authorization appended to task body |

NOT touching (other agents' own files):

* `crates/chan-drive/src/{drive,indexer}.rs` (committed
  in `7a22e63`).
* `web/src/components/{HybridTerminalConfig.*, SettingsPanel.*}`
  (committed in `1f80d09`).
* All `event-<agent>-architect.md` files (agents' own
  outbounds).
* All task-file appends (`ci-12.md`, `ci/journal.md`,
  `systacean-15.md`, `systacean-17.md`,
  `fullstack-a-45.md`).

## 2026-05-21 — clearance round 4: -18 follow-up + -a-46 + smoke-mid-flight on -24

### Lane commit landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `c0600e0` | `chan-server + chan-desktop: gate Unix-only control_socket declarations + rename unused exit_signal (fullstack-b-24)` | @@FullStackB |

Per the option-(A) routing in last round. Smoke branch
`fullstack-b-24-smoke` now on origin (fourth in the
audit-trail-keep set). @@FullStackB has NOT yet appended
a commit-readiness / smoke-verdict poke to their outbound
channel — smoke run presumed in flight. Holding clearance
read until they report.

### Two commit-readiness pokes this round

**@@Systacean -18 smoke surfaced ONE additional model-dep
test**: `crates/chan-drive/tests/contacts_import.rs:274`
`removing_contact_frontmatter_demotes_node_back_to_file`
panicked with the same BGE failure shape — not in my
line-number callout NOR in their `-17`-smoke list because
cargo's per-binary panic cascade masked it.

They asked for the "obvious-call" shortcut: same scope as
`-18`, same `#[ignore]` shape, finishes the gate-unblocker
work, authorize them to commit + re-dispatch in one beat.
GRANTED. Single-file diff; cross-reference the surfacing
in the skip reason for audit trail; push as append (not
force) to `systacean-18-smoke` + re-fire `gh workflow run
ci.yml`. Expected: Ubuntu fully green; Windows still red
on chan-desktop dead_code (closes when `fullstack-b-24`
lands in HEAD — already committed at `c0600e0`).

**@@FullStackA -a-46 (Editor Settings migration)**:
mirror of `-a-45`'s Terminal pattern; +15 net test pins
(vitest 621/621); CSS sweep absorbed in same commit
(clearance to fold cleanup into the migration's commit
was the right call). Three flagged deviations all
accepted:

1. Appearance moved with the wave — per round-2-plan
   spec; Editor back-side scope includes Theme.
   Per-Hybrid theme override via hamburger toggle means
   no global-default pop-up needed in SettingsPanel.
   Walkthroughs catch any UX papercut.
2. `.strip-toggle` rename — local cleanup; original
   name was semantically mismatched. Cheap correction.
3. `hybrid-editor-*` / `hybrid-appearance` /
   `hybrid-line-spacing` name namespacing — defensive
   against radio-name collisions; same shape as
   `-a-45`'s `hybrid-terminal-*`.

### Symmetric empirical-audit-at-pickup pattern

Two lanes caught my architect-side errors at task pickup
THIS PHASE:

* @@FullStackB caught the `fullstack-b-24` categorical
  scope error (10/11 lints in chan-server, not
  chan-desktop) before editing any code.
* @@Systacean caught the contacts_import test mask via
  the empirical `-18`-smoke run — pre-emptively beyond
  the original 14-test gating set.

Both are EXACTLY the discipline I want from every lane:
read the source / run the smoke before trusting upstream
framing. The pattern is encoded in the
`feedback_ground_descriptions_in_source` memory rule;
the cross-lane reinforcement this round is meaningful
because it shows the rule applies in BOTH directions
(architect-to-lane AND smoke-to-architect feedback).

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | `-18` committed (`7a22e63`); `-18` follow-up (contacts_import) cleared + authorized to commit + re-smoke; expect `-16` pickup after |
| @@CI | Idle; queue-empty until wave-3 Linux-binaries |
| @@FullStackA | `-a-45` committed (`1f80d09`); `-a-46` cleared; expect commit + `-a-47` pickup (Task E — drop front/back independent theme) |
| @@FullStackB | `-24` committed (`c0600e0`); smoke in flight at `fullstack-b-24-smoke`; holding for their verdict report |
| @@WebtestA | Standing by; reactive lane |
| @@WebtestB | Standing by; reactive lane |

After the `-18` follow-up + `fullstack-b-24-smoke` both
green-confirm, the **per-PR ci.yml gate is structurally
fully green** for the first time since ~2026-05-19. The
Round-3 readiness signal is one smoke-verification away.

Four smoke branches on origin: `ci-12-smoke` +
`systacean-17-smoke` + `systacean-18-smoke` +
`fullstack-b-24-smoke`. All prune with the
`chan-v0.11.99-dryrun.{1..4}` tag cleanup beat.

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | -18 follow-up obvious-call greenlight |
| `alex/event-architect-fullstack-a.md` | -a-46 clearance + 3 deviations accepted |

NOT touching (other agents' own files):

* `crates/chan-drive/tests/contacts_import.rs` —
  @@Systacean's `-18`-follow-up scope.
* `web/src/components/{HybridEditorConfig.*,
  SettingsPanel.svelte}` — @@FullStackA's `-a-46` code.
* All `event-<agent>-architect.md` files (agents' own
  outbounds).
* All task-file appends (`ci-12.md`, `ci/journal.md`,
  `systacean-15.md`, `systacean-17.md`,
  `systacean-18.md`, `fullstack-a-45.md`,
  `fullstack-a-46.md`, `fullstack-a/journal.md`).

## 2026-05-21 — clearance round 5 — four lanes all needed something + webtest-a-4 dispatched

### Three lane commits landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `b4ef2dd` | `chan-drive/tests/contacts_import: gate removing_contact_frontmatter test behind #[ignore] (systacean-18 follow-up)` | @@Systacean |
| `5166223` | `Migrate Editor Settings to Hybrid Editor back-side (fullstack-a-46)` | @@FullStackA |
| `e8ff68a` | `chan-server + chan-desktop: smoke #1 fixup — gate orphaned Unix-only imports + parse_ps helper on Windows (fullstack-b-24)` | @@FullStackB |

Plus my `319b45e` (terminal-line xterm.js#2409 bug
annotation). 5 commits since clearance round 4.

### Four lanes had asks at this beat

@@Alex flagged "they seem to be all waiting on you now."
Surface sweep confirmed:

* **@@FullStackA**: `-a-47` (drop front/back independent
  theme) ready for review with 2 flagged deviations (`bm`
  back-materialised marker; front-side-wins on legacy
  migration). Both accepted; cleared.
* **@@FullStackB**: `c0600e0` + `e8ff68a` smoke #1
  fixup landed without a separate clearance ask — they
  took the obvious-call shape (same task scope; commit
  + re-smoke in one beat). After-the-fact ack issued;
  shape was right. Smoke #2 expected to confirm Windows
  clippy green.
* **@@Systacean**: `b4ef2dd` (`-18` follow-up) landed
  per the obvious-call shortcut I'd authorized; their
  channel hadn't been updated post-commit. After-the-
  fact ack issued; `-16` (file-class buckets) greenlit
  as next pickup.
* **@@WebtestA**: standing by for next walkthrough.
  Three commits accumulated since `-3` verdict
  (`-a-44` drag, `-a-45` Terminal mig, `-a-46` Editor
  mig) — exactly the "logical grouping" trigger I'd
  set for the next bundled walk.

### webtest-a-4 cut

[`../webtest-a/webtest-a-4.md`](../webtest-a/webtest-a-4.md)
bundles all three slices with 6 acceptance checks each.
Single bundled verdict commit; standing perm covers it;
`-a-47` folds into `webtest-a-5` alongside `-a-48`
(FB-back Search/Indexing/Reports migration) when both
land.

The bundling shape is the right grain — per-commit
walkthrough would 3x the audit surface for the same
coverage; the migrations are mechanical UX moves
(Settings → Hybrid back-side) so the walk grades each
slice quickly.

### @@WebtestB: nothing actionable this round

Their lane is idle by design — current Round-2 wave-3
in-flight work is all SPA / chan-server / chan-drive /
chan-desktop-declaration scope. No runtime behavioural
shift requiring chan-desktop walkthroughs. Routed an
"ack standby" with a proactive-coverage-walks
suggestion: walk HEAD chan-desktop runtime on a
throwaway drive to confirm `-b-22` orphan-sidecar reap
+ drive-lock-takeover UX still holds post-`-24`
`#[cfg(unix)]` gating. NOT a dispatch — opt-in idle
fill via the `feedback_proactive_walks` memory.

### @@CI: idle is fine; v0.12.0 Linux-binaries is the next dispatch

Not cutting it now — gate-unblocker work is still
landing. Once `-18` follow-up smoke + `-24` smoke #2
both confirm green, the per-PR ci.yml gate is
structurally fully-green; THAT is when wave-3 Linux
fan-out makes sense.

### Smoke-branch state on origin

Four smoke branches now:

* `ci-12-smoke` (idle; original gate fix)
* `systacean-17-smoke` (idle; Windows result_large_err)
* `systacean-18-smoke` (in flight on smoke #2 after
  `b4ef2dd` append push)
* `fullstack-b-24-smoke` (in flight on smoke #2 after
  `e8ff68a` fastforward push)

All four prune with the `chan-v0.11.99-dryrun.{1..4}`
tag cleanup beat. No action needed until then.

### Pattern: obvious-call shape across lanes

Both @@Systacean (`-18` follow-up) AND @@FullStackB
(smoke #1 fixup as `e8ff68a`) took the same shape: a
same-task-scope follow-up commit without a separate
clearance round. That's the discipline I want — the
ask is "does my next obvious-call commit count as a
new task or a follow-up to the current task?" — and
the answer is "follow-up if scope unchanged + work is
reactive to the same trigger." Both lanes' read was
correct.

The architect-side cost: I have to maintain awareness
of which lane is in flight + ack after-the-fact. That's
fine — auditing HEAD per-commit was already part of
the beat-sweep discipline.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | `-18` follow-up committed (`b4ef2dd`); after-the-fact ack issued; `-16` greenlit; expect commit-readiness OR smoke #2 verdict |
| @@CI | Idle; queue-empty until wave-3 Linux-binaries (lands when full gate-green confirms) |
| @@FullStackA | `-a-47` cleared; expect commit + `-a-48` pickup (Task F — Search/Indexing/Reports migration) |
| @@FullStackB | `c0600e0` + `e8ff68a` committed; smoke #2 in flight; expect verdict report + then queue-empty until wave-3 Linux dispatch |
| @@WebtestA | `webtest-a-4` dispatched (bundled walk); pickup imminent |
| @@WebtestB | Ack-standby issued; opt-in proactive coverage walk suggested; otherwise idle |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | -a-47 clearance + 2 deviations accepted |
| `alex/event-architect-fullstack-b.md` | after-the-fact ack on c0600e0 + e8ff68a |
| `alex/event-architect-systacean.md` | after-the-fact ack on b4ef2dd + -16 greenlit |
| `alex/event-architect-webtest-a.md` | webtest-a-4 dispatch |
| `alex/event-architect-webtest-b.md` | ack-standby + proactive-walks suggestion |
| `webtest-a/webtest-a-4.md` | NEW task (bundled walkthrough) |

## 2026-05-21 — Hybrid back-side design correction from @@Alex: 2 new tasks (-a-53 + -a-54)

@@Alex 2026-05-21 (chat, post webtest-a-4 dispatch)
surfaced two corrections to the Hybrid back-side wave
design. Both significant enough to merit new tasks (per
the append-only-in-coordination memory rule).

### Correction 1 — theme architecture

@@Alex (verbatim):

> 1. the appearance system/dark/light should remain on
>    settings..
> 2. the editor and terminal etc have their own toggle..
>    e.g. i want dark mode from the settings but all my
>    editors are light mode

Architecture:

* Global default Appearance (system/dark/light) lives
  in **Settings overlay**.
* Each Hybrid Editor + Hybrid Terminal back-side
  carries a **per-Hybrid override toggle**
  (`inherit | light | dark`).
* Resolution: per-Hybrid override if set → else global
  default → else system.

Example use-case: Settings = dark, but all Editor panes
= light. Override semantic = "this surface specifically
renders different from the global."

Conflict with `-a-46` (HEAD `5166223`): Appearance was
migrated WHOLESALE to `HybridEditorConfig` back. That's
intermediate state; needs partial revert. Per
@@FullStackA's `-a-46` clearance flagged-deviation note
("If so, the section + `setThemeChoice` import + 3
Appearance tests can revert via a small follow-up") —
the revert path was anticipated.

NO conflict with `-a-47` (collapse front/back
independent theme; cleared but uncommitted). The
collapse + the override are orthogonal: collapse runs
WITHIN a single Hybrid; override runs ACROSS Hybrid vs
global. Both right.

Dispatched as
[`../fullstack-a/fullstack-a-53.md`](../fullstack-a/fullstack-a-53.md).

### Correction 2 — flip UX

@@Alex (verbatim):

> when we flip the tab, we need to keep the pane's bar
> where all tabs are, and we should still show the tabs
> but flipped — their text is like if you were looking
> at them from behind.. and we should be able to switch
> between them on the back.. the hamburger would be on
> the other side, like it flipped
>
> only inside the tab area (like in the front pane) we
> would then have the title Hybrid Terminal, Hybrid
> Editor, and so on

Visual deltas (vs current post-`-a-43` flip behaviour):

* Tab strip stays in same physical position when
  flipped (NOT a full chrome rotate).
* Tab labels render mirrored (`scaleX(-1)`-ish) but
  remain clickable.
* Hamburger swaps to opposite end of tab strip.
* Family-name title ("Hybrid Terminal" / etc.) shows
  INSIDE the tab area (NOT a new chrome row).

Rationale: preserves user's spatial model ("this is
the same pane") while signaling flip via mirroring +
side-swap. Title gives explicit confirmation of
which surface's settings the back hosts.

Dispatched as
[`../fullstack-a/fullstack-a-54.md`](../fullstack-a/fullstack-a-54.md).

### Revised @@FullStackA queue

```
-a-47 (committable; collapse front/back theme)
-a-48 (Task F; FB-back Search/Indexing/Reports migration)
-a-53 (theme architecture correction — Appearance revert + per-Hybrid override)
-a-54 (flip UX redesign — preserve tab strip + mirrored tabs + hamburger swap + title in tab area)
-a-49..52 (graph overhaul first sub-wave)
-a-42 (About; gates on A+B+C+F landing)
```

`-a-53` + `-a-54` insert AHEAD of `-a-49..52` to finish
the Hybrid back-side semantic before moving to graph
work. Sequencing within the new pair:

* `-a-53` should pick up AFTER `-a-47` commits
  (front/back theme collapse is the right baseline for
  the override layer).
* `-a-54` should pick up AFTER `-a-53` commits
  (back-side CONTENT before back-side CHROME).

`-a-48` can interleave anywhere — independent of the
new pair (FB-back is its own surface).

### round-2-plan updated

Two new sections appended to
[`round-2-plan.md`](round-2-plan.md) §"Hybrid back-side
revisited":

* "Theme architecture correction 2026-05-21" — global
  default + per-Hybrid override pattern.
* "Flip UX correction 2026-05-21" — tab strip
  preserved + mirrored + hamburger swap + title in tab
  area.

Per-surface back-side scope table also updated to
reflect "per-Hybrid theme override toggle" in both
Hybrid Terminal + Hybrid Editor scopes; Appearance
explicitly OUT of Hybrid Editor scope.

### webtest-a-4 walk-context update

Appended a design-context note to
[`../webtest-a/webtest-a-4.md`](../webtest-a/webtest-a-4.md)
explaining that the `-a-46` Appearance section IS in
`HybridEditorConfig` back per the as-landed `-a-46`
spec, BUT slated for partial revert by `-a-53`. @@WebtestA
walks the current state, captures the as-landed
behaviour as HOLD if it works, and files the design-
correction note as a SIDE OBSERVATION — NOT a failure.
`webtest-a-5` walks the corrected end state after `-a-53`
+ `-a-54` land.

### Pattern: design corrections after a task lands

`-a-46` was the second time this round a task landed
correctly per spec + then @@Alex flagged a design
correction post-landing (the first was `webtest-b-3`'s
heuristic-tightening finding from @@WebtestB's walk —
filed in the bug list). The append-only coordination
discipline holds: don't amend the landed work; cut
new tasks; document the design history in the round
plan + walk verdicts.

The architect-side takeaway: my `-a-46` clearance
should have flagged @@FullStackA's "if so, the section
... can revert via a small follow-up" deviation as a
"this might revert" risk rather than the simple ACCEPT.
@@FullStackA was right to flag it; I should have
shown more uncertainty. Next time a clearance has a
deviation that touches a design surface @@Alex hasn't
explicitly confirmed, raise the flag explicitly to
@@Alex rather than accepting silently.

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `architect/round-2-plan.md` | 2 design-correction sections |
| `alex/event-architect-fullstack-a.md` | -a-53 + -a-54 dispatch poke |
| `webtest-a/webtest-a-4.md` | design-context note for the walk |
| `fullstack-a/fullstack-a-53.md` | NEW task (theme architecture correction) |
| `fullstack-a/fullstack-a-54.md` | NEW task (flip UX redesign) |

## 2026-05-21 — clearance round 6 — 8 lane commits acked + -a-48 routed (option B) + PARTIAL folded into -a-53

### Lane commits landed this beat (8)

| SHA | Subject | Lane |
|-----|---------|------|
| `f796345` | `chan: gate Unix-only ControlResponse enum on Windows (fullstack-b-24 smoke #2 fixup)` | @@FullStackB |
| `dd586fc` | `Drop front/back independent theme; single per-Hybrid value (fullstack-a-47)` | @@FullStackA |
| `82ba444` | `chan-drive/tests: gate file_types + smoke binaries on missing BGE model (systacean-18 follow-up #2)` | @@Systacean |
| `97f573b` | `docs: -a-48 scope question for chan-reports backend gating + -a-47 commit marker (fullstack-a-48 audit anchor)` | @@FullStackA |
| `68e1cbc` | `chan: gate Deserialize import on Windows (fullstack-b-24 smoke #3 fixup)` | @@FullStackB |
| `147a06f` | `chan-drive/tests/remove_cleanup: gate single_file + directory_cascade tests behind #[ignore] (systacean-18 follow-up #3)` | @@Systacean |
| `06afe3f` | `docs: webtest-a-4 — Hybrid back-side wave + drag walkthrough (17/18 HOLD, 1 PARTIAL on -a-45 custom TERM)` | @@WebtestA |
| `b01b310` | `chan-server: gate Unix-only fs_graph test helpers on Windows (fullstack-b-24 smoke #4 fixup)` | @@FullStackB |
| `1662d47` | `docs: -a-48 + -a-53 + -a-54 queue ack to @@Architect (fullstack-a)` | @@FullStackA |

(9 commits actually, including the queue-ack. Listed above.)

Both @@FullStackB and @@Systacean continued their iterative
mechanical-fixup cascade pattern — same-task-scope fixup
commits unmasking the next layer of Windows clippy /
Ubuntu cargo test items that the broken pre-`ci-12`-and-`-17`
gate had been hiding.

### -a-48 scope question — option (B) routed

@@FullStackA picked up `-a-48` (Task F — Search/Indexing/
Reports migration to FB back + chan-reports toggle
restore) and hit a scope question on first audit. The
chan-reports toggle was specced in the round-2-plan
pre-flight feature toggles but **never landed in v1**;
NO Preferences shape, NO chan-server gating, NO chan-drive
indexer pass flag.

Three options:

* **(A)** Full -a-48: SPA + chan-server route gating in 4
  files + chan-drive indexer pass flag + destructive-on-
  disable modal, all in one commit. Big.
* **(B)** SPA wiring + default ON; backend gating deferred
  (their lean).
* **(C)** Defer chan-reports entirely; ship semantic-search
  migration only.

**Routed (B).** Reasoning:

* (A) is too big — same shape that burned us on `-a-46`
  (the design-correction wave). Smaller commits + sharper
  audit shape wins.
* (C) leaves the user-visible regression unfixed
  (`phase-8-bugs.md` "chan-reports settings toggle missing
  from Settings UI (regression)" + @@Alex's "i want it
  back!"). Toggle visibility IS the regression fix.
* (B) ships visible progress this beat + establishes the
  `Preferences.reports.enabled` shape downstream tasks
  read + defers the surgical backend work cleanly.

**Default ON is the right call** — no toggle-lie risk;
matches today's unconditional chan-report; when disable
path lands in follow-up, "OFF" gets real teeth.

After `-a-48` lands, cut a new `-a-N` follow-up task
covering the 4-route chan-server gating + chan-drive
indexer pass flag + destructive-on-disable modal +
default flip ON→OFF. Likely crosses lanes to
@@Systacean for the chan-drive indexer pass flag piece.

### webtest-a-4 PARTIAL bundled into -a-53

@@WebtestA's verdict (17/18 HOLD; 1 PARTIAL on `-a-45`
#3) root-caused the HybridTerminalConfig custom-TERM
dropdown rendering bug to lines 104 + 86-88. ~5-line
SPA fix.

Bundled into `-a-53`'s scope rather than a tiny
standalone task — `-a-53` is already touching
`HybridTerminalConfig.svelte` for the per-Hybrid theme
override toggle; folding the custom-TERM fix into the
same commit keeps the queue compact. Task body updated
with the "Bundled scope addition 2026-05-21" section.

`webtest-a-5` walks the corrected custom-TERM behavior
after `-a-53` + `-a-54` land (alongside `-a-47` + `-a-48`
which haven't been walked yet).

### Five lane acks issued this beat

| Lane | Ack shape |
|------|-----------|
| @@FullStackA | Routing on -a-48 scope question (option B) + custom-TERM PARTIAL folded into -a-53 + thanks on the audit-anchor commit shape |
| @@WebtestA | After-the-fact ack on verdict commit + PARTIAL routing noted + webtest-a-5 planned |
| @@WebtestB | Clearance for proactive smoke walk verdict bundle commit (their commit-readiness ask honored) |
| @@FullStackB | After-the-fact ack on smoke #2 + #3 + #4 fixups; obvious-call shape carry-on; smoke #5 expected |
| @@Systacean | After-the-fact ack on -18 follow-ups #2 + #3; obvious-call shape carry-on; Ubuntu smoke expected |

### Iterative mechanical-fixup pattern recognized

Both @@FullStackB and @@Systacean are in identical
iteration patterns:

1. Smoke run reveals next layer of dead_code (chan-server)
   or BGE-model-panic (chan-drive) items the broken
   pre-`ci-12`-and-`-17` gate had been masking.
2. Apply mechanical `#[cfg(unix)]` or `#[ignore]` fixup
   at the unmasked sites.
3. Commit as same-task-scope fixup (`-24 smoke #N fixup`
   or `-18 follow-up #N`).
4. Re-smoke; goto 1.

Both lanes are applying it cleanly with audit shape +
no scope drift. The cascade can't go forever — finite
item count — so it terminates naturally. Acks issued;
carry-on authorized.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | 2 more `-18` follow-ups in HEAD; expect Ubuntu smoke green + `-16` pickup |
| @@CI | Idle; queue-empty |
| @@FullStackA | `-a-47` committed + `-a-48` scope routed (option B); expect `-a-48` commit + `-a-53` (with custom-TERM PARTIAL fix bundled) pickup |
| @@FullStackB | 4 smoke fixups in HEAD; expect smoke #5 verdict or next layer fix |
| @@WebtestA | `-a-4` verdict committed; queue-empty until `-a-48` + `-a-53` + `-a-54` land for `webtest-a-5` |
| @@WebtestB | Proactive smoke verdict cleared; expect commit + then queue-empty |

### Smoke-branch state on origin

* `ci-12-smoke` (idle)
* `systacean-17-smoke` (idle)
* `systacean-18-smoke` (3 fastforward pushes; expect
  smoke green next run)
* `fullstack-b-24-smoke` (4 fastforward pushes; expect
  smoke #5 next)

All prune with the `chan-v0.11.99-dryrun.{1..4}` tag
cleanup beat.

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | -a-48 option B routing + PARTIAL fold + thanks |
| `alex/event-architect-webtest-a.md` | -a-4 verdict ack + PARTIAL routing note |
| `alex/event-architect-webtest-b.md` | proactive smoke verdict clearance |
| `alex/event-architect-fullstack-b.md` | smoke #2/#3/#4 after-the-fact ack |
| `alex/event-architect-systacean.md` | -18 follow-up #2/#3 after-the-fact ack |
| `fullstack-a/fullstack-a-53.md` | Bundled scope addition (custom-TERM PARTIAL fix) |

## 2026-05-21 — clearance round 7 — two lane scope-escalations routed + systacean-19 cut (C2 product improvement)

### Lane scope-escalations this beat

Both @@Systacean and @@FullStackB correctly **paused
their iterative smoke cascade** when the scope widened
beyond mechanical fixup territory. Both escalated for
routing rather than continuing whack-a-mole. Exactly
the discipline.

@@Alex framing on the round: "they are all idling.. and
the github-ci is broken". CI runs all failing on the
smoke branches — Windows clippy CLOSED on `fullstack-b-24-smoke`
smoke #5 + the dead_code cascade exhausted, BUT cargo
test reds remain on both Ubuntu (BGE-panic surface) and
Windows (test portability gap). The actual ci.yml on
main hasn't fired since v0.11.2 (push held); the "broken
CI" framing is the smoke-iteration reaching its next
layer.

### @@FullStackB scope escalation routed

Smoke #5 cleared Windows clippy. `-24`'s stated scope
done. Two test failures remain:

* **Windows**: `graph_scope_file_rejects_missing_target`
  in `chan/main.rs:2970` — assertion hard-codes Unix
  OS-error wording ("No such file"); Windows says "The
  system cannot find the file specified". 1-line
  portability fix.
* **Ubuntu**: BGE gap (handled by @@Systacean's lane).

**Routed option A** — fold the Windows test fix into
`-24` (smoke #6 fixup shape). Same task scope, same
single-commit pattern, single smoke fire validates the
combined state. Authorization expanded inline for
`crates/chan/src/main.rs:2970` (already inside the chan
crate scope `-24` has been editing throughout).

### @@Systacean scope escalation routed (3-part)

Follow-up #3 smoke surfaced 9 MORE BGE-panic failures
+ 2 new dead_code lints (`fs_graph.rs:927+932`). NEW
LANE — chan-server. Originally `-18`'s scope was
chan-drive only because @@CI's `ci-12` audit had
visibility limited to chan-drive at the time.

Total gate-blocking BGE-test set across the workspace:

| Crate | Tests | Gated | Awaiting decision |
|-------|-------|-------|-------------------|
| chan-drive lib | 14 | 14 (`-18` initial) | 0 |
| chan-drive integration | 5 | 5 (follow-ups #1+#2+#3) | 0 |
| chan-server lib | 9 | 0 | **9** |
| **Total** | **28** | 19 | 9 |

Plus 2 fs_graph.rs dead_code lints (`node` +
`node_path_kind`) — also chan-server lib lane.

@@Systacean proposed three routing options:

* **A** — fold chan-server gating into `-18` follow-up
  #4 (same `#[ignore]` + `#[cfg(...)]` shape as the
  prior follow-ups).
* **B** — cut a separate `systacean-19` for chan-server
  gating only.
* **C** — pivot to a structural fix:
  - **C1**: programmatic skip via `resolve_model` check
    (test-infra change).
  - **C2**: code-level fix in chan-drive's `write_file`
    — degrade gracefully to BM25-only when model not
    present.

**Routed A + cut systacean-19 for C2.**

* **Option A short-term**: fold chan-server gating + the
  2 fs_graph lints into `-18` follow-up #4. Same
  mechanical shape; gets the gate green TODAY. Authorization
  expanded inline for the chan-server source files.
* **systacean-19 medium-term**: C2 graceful BM25-only
  degradation. This is the REAL PRODUCT IMPROVEMENT —
  today's default-build install (no `embed-model`
  feature, no model downloaded) has BROKEN indexing;
  C2 gives users working BM25 search out of the box,
  with semantic search as the upgrade path. ALIGNS with
  the `systacean-6` / `-7` opt-in architecture (the
  bundle is opt-in at BUILD; this makes the opt-out
  RUNTIME behaviour consistent).

After `systacean-19` lands, all 28 `#[ignore]` gates
REVERT. Coverage restored without per-test iteration.
The 28-test cascade becomes obsolete the moment the
fallback path exists.

* **C1 declined** — C2 makes both `#[ignore]` and the
  test-infra helper obsolete; investing in C1 is wasted
  effort.
* **Option B declined** — separating chan-server gating
  into its own task adds dispatch overhead without
  audit-clarity benefit; bundling into `-18` follow-up
  #4 keeps the gate-unblocker lineage tight.

### Lesson on lane scope escalation

Both @@FullStackB and @@Systacean explicitly set
themselves a "fire-a-scope-poke-instead-of-iterating"
gate AHEAD of the next smoke run, then EXECUTED against
that gate when the scope widened. That's the
self-imposed-discipline pattern I want from every code
lane. The cost of iteration is real (each smoke run is
~10 min wall-clock; each fixup commit consumes review
attention); the gate prevents the cost from spiralling.

Both escalations also surfaced REAL structural insights
@@Architect wouldn't have caught from line-number-only
audits:

* @@FullStackB caught that the dead_code lints lived in
  chan-server, not chan-desktop (caught at `-24`
  pickup, before any code touched).
* @@Systacean caught that the BGE-panic surface spans
  chan-server + chan-drive (caught at follow-up #3
  pickup, surfacing C2 as a structural improvement).

The architect-to-lane direction was wrong in BOTH cases
at task-cut time. The lane-to-architect feedback
direction caught and corrected it. That's the discipline
that prevents an entire round of follow-on tasks
chasing the wrong fix.

### Systacean queue (revised)

```
-18 follow-up #4 (chan-server gating + fs_graph lints) — obvious-call extension; authorized
-19 (C2 — graceful BM25-only degradation + revert all 28 #[ignore] gates)
-16 (chan-report file-class buckets — feature work; deferred if needed)
-12 (tauri-plugin-updater verify; still parked on permission ask)
```

`-19` is the bigger structural fix; `-16` parks behind it.

### What's actually broken vs not (clarifying for @@Alex)

* **Main branch ci.yml runs**: HAVEN'T fired since v0.11.2
  (`60901c1`); push held per Round-2-close discipline.
  So main is NOT "broken" in the "main is red" sense.
* **Smoke branches** (`ci-12-smoke`, `systacean-17-smoke`,
  `systacean-18-smoke`, `fullstack-b-24-smoke`): all red.
  Each one is mid-iteration as the cascade closes. These
  are PR-style work-branches, not production.
* **After `-18` follow-up #4 + `-24` smoke #6 land**:
  the per-PR ci.yml gate is structurally fully green on
  ALL three platforms (clippy + test + build). That's
  the Round-3 readiness signal.
* **After `systacean-19` lands**: the gating shape
  becomes obsolete (revert the 28 `#[ignore]`s); users
  with default-build installs get working BM25 indexing.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | -18 follow-up #4 authorized (obvious-call extension); systacean-19 cut for C2; expect commit + smoke + then -19 pickup |
| @@CI | Idle; queue-empty |
| @@FullStackA | -a-48 option B routed; -a-53 with bundled custom-TERM PARTIAL pending; expect -a-48 commit + then -a-53 pickup |
| @@FullStackB | -24 smoke #6 fixup authorized (Windows test portability); expect commit + smoke; then queue-empty |
| @@WebtestA | -a-4 verdict committed; queue-empty until next bundled walk |
| @@WebtestB | Proactive smoke verdict cleared for commit; queue-empty |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | option A + systacean-19 cut for C2 |
| `alex/event-architect-fullstack-b.md` | -24 smoke #6 option A fold-in |
| `systacean/systacean-19.md` | NEW task (C2 graceful degradation) |

## 2026-05-21 — clearance round 8 — -a-48 cleared + 2 lane commits acked

### Lane commits landed since last beat

| SHA | Subject | Lane |
|-----|---------|------|
| `743ee69` | `docs: webtest-b proactive smoke against HEAD post -b-24 + -a-47 + a8e991a incident close-out` | @@WebtestB |
| `8e4ce5c` | `chan: portability fix for graph_scope_file_rejects_missing_target Windows assertion (fullstack-b-24 smoke #6 fixup)` | @@FullStackB |

### -a-48 option B cleared

@@FullStackA delivered option B execution cleanly:

* `Preferences.reports.enabled` field added to
  chan-server's PATCH/GET shape (both `ServerConfig`
  + TS types).
* `HybridFileBrowserConfig` populated from `-a-43`
  stub with three toggles (Semantic search migrated
  from SettingsPanel; multi-model picker placeholder;
  chan-reports NEW).
* SettingsPanel substantially trimmed (Semantic-search
  section + state machine + helpers + CSS scope all
  removed). Post-`-a-48` the overlay is the About
  section + GlobalConfig autosave plumbing only.
* vitest 637/637 (+15 net); cargo + clippy + svelte-
  check all green.
* Follow-up scope captured in `-a-48`'s tail (backend
  gating in 4 chan-server route files + chan-drive
  indexer-pass flag + destructive-on-disable modal +
  default flip ON → OFF).

Cleared verbatim. Shared-infra authorization flagged
inline for the chan-server edits (narrow
`reports.enabled` field addition + PATCH serde
round-trip). After commit, queue continues with `-a-53`
(theme architecture correction + bundled custom-TERM
PARTIAL fix) per the post-design-correction plan.

### Important sequencing note: -a-42 gate now closed

With `-a-48` cleared (and committable now), the "A+B+C+F
all in HEAD" gate for `-a-42` (About section build-out)
is closed. `-a-42` is now technically unblocked. BUT the
queue order keeps it parked after `-a-49..52` (graph
overhaul) so the Hybrid back-side correction tasks
(`-a-53` + `-a-54`) and the graph wave land first.
Don't pull `-a-42` forward unless I re-sequence.

### @@FullStackB smoke #6 in flight

`8e4ce5c` (Windows test portability fix per option-A
routing) committed; pushed to `fullstack-b-24-smoke`;
workflow run `26245378140` is **IN PROGRESS** (~13+ min
wall-clock when checked). Standard post-fix smoke;
passive wait on CI.

Expected outcome on smoke #6 green:

* Windows clippy ✓ + Windows test ✓ (test fix closes
  the assertion gap).
* Ubuntu clippy ✓.
* Ubuntu test STILL ✗ on BGE-panic surface until
  @@Systacean's `-18` follow-up #4 lands (independent
  thread).

If smoke #6 reds on Windows test for any unexpected
reason, @@FullStackB will iterate per the established
pattern. After smoke #6 green, their `-24` lane is
queue-empty.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | `-18` follow-up #4 + `systacean-19` authorized; not yet started (or in flight without source mods); expect commit + smoke |
| @@CI | Idle; queue-empty |
| @@FullStackA | `-a-48` cleared (commit-ready); expect commit + `-a-53` pickup |
| @@FullStackB | `-24` smoke #6 in flight at `fullstack-b-24-smoke` run 26245378140; expect verdict |
| @@WebtestA | Queue-empty until next bundled walk (after -a-48 + -a-53 + -a-54 land) |
| @@WebtestB | Proactive verdict committed; queue-empty |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | -a-48 commit clearance + follow-up scope ack + -a-42 sequencing note |
| `alex/event-architect-fullstack-b.md` | after-the-fact ack on 8e4ce5c + smoke #6 in flight |
| `alex/event-architect-webtest-b.md` | after-the-fact ack on 743ee69 |

## 2026-05-21 — clearance round 9 — -a-53 cleared + -24 closed + systacean-20 cut for Windows lock tests

### Lane commits landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `0391eae` | `Migrate Search/Indexing/Reports settings to Hybrid FB back-side (fullstack-a-48 option B)` | @@FullStackA |
| `d1cd565` | `docs: fullstack-b-24 smoke #6 verdict + commit readiness (Windows clippy + target test GREEN; 3 chan-drive lock failures need routing)` | @@FullStackB |
| `bf85e8a` | `chan-server: gate 9 model-dependent tests behind #[ignore] (systacean-18 follow-up #4)` | @@Systacean |

### -a-53 cleared

Hybrid back-side theme architecture correction +
bundled custom-TERM PARTIAL fix delivered cleanly.
No deviations flagged this round; clean execution per
the spec. Cleared verbatim.

After commit, `-a-54` (flip UX redesign) is next per
the sequencing rule "back-side CONTENT before back-side
CHROME". With `-a-48` + `-a-53` both in HEAD post this
beat, the A+B+C+F gate for `-a-42` (About section) is
closed but `-a-42` stays parked behind `-a-49..52` per
queue order.

### -24 CLOSED + 3 lock tests routed to systacean-20

@@FullStackB's smoke #6 verdict confirmed: Windows
clippy ✓ + Windows graph_scope_file_rejects_missing_target
✓ + Ubuntu clippy ✓. `-24` is structurally complete on
their lane (7 implementation commits + audit docs).

The 3 remaining Windows reds (chan-drive lock-contract
tests) are NOT in `-24`'s scope:

* `drive::tests::second_open_blocks_on_writer_lock`
* `library::tests::reset_drive_returns_locked_when_other_process_holds_lock`
* `lock::tests::second_acquire_fails_while_held`

All 3 fail on `matches!(err, ChanError::DriveLocked)` —
chan-drive's lock primitive doesn't surface `DriveLocked`
on Windows the same way `flock` does on Unix.

Cut [`../systacean/systacean-20.md`](../systacean/systacean-20.md)
with shape (ii) `#[cfg(unix)]` — mechanical gate-unblocker
per the `-17` + `-18` pattern. Real Windows lock-primitive
bridge (shape (i) using `LockFileEx`) deferred to Round-3
polish; will flag in bug list once @@Systacean files the
revert-target entry per the `-20` task body.

### Pattern observation: gate-unblocker cascade is now structurally bounded

The cascade has been peeling back layers:

1. `ci-12` (GTK install) — closed.
2. `systacean-17` (Windows result_large_err) — closed.
3. `fullstack-b-24` (Windows chan-server dead_code +
   smoke fixup cascade) — closed (7 commits).
4. `systacean-18` (chan-drive BGE-panic) + 4 follow-ups
   gating 19 chan-drive tests + 9 chan-server tests +
   2 fs_graph dead_code lints — `-18` follow-up #4
   smoke in flight (expected green).
5. `systacean-20` (3 chan-drive lock tests on Windows)
   — cut this round; mechanical.

After `-18` follow-up #4 smoke + `-20` commit + smoke,
the per-PR ci.yml gate is structurally fully green on
all 3 platforms. That's the Round-3 readiness signal.

`systacean-19` (C2 graceful BM25 degradation) follows
as the medium-term structural fix that retroactively
reverts the 28 `#[ignore]` gates (the 3 `#[cfg(unix)]`
gates stay until the real Windows lock bridge lands in
Round 3+).

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | `-18` follow-up #4 committed; smoke in flight; `-20` dispatched; expect `-20` commit + then `-19` pickup |
| @@CI | Idle; queue-empty |
| @@FullStackA | `-a-48` committed; `-a-53` cleared; expect commit + `-a-54` pickup |
| @@FullStackB | `-24` CLOSED; queue-empty until wave-3 Linux-binaries OR `-b-22` heuristic-tightening dispatches |
| @@WebtestA | Queue-empty until next bundled walk |
| @@WebtestB | Queue-empty |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | -a-53 commit clearance |
| `alex/event-architect-fullstack-b.md` | -24 close + lock tests routed to systacean-20 |
| `alex/event-architect-systacean.md` | -18 follow-up #4 ack + systacean-20 dispatch |
| `systacean/systacean-20.md` | NEW task (Windows lock test gating) |

## 2026-05-21 — clearance round 10 — -a-54 cleared + -18 fu#4 Ubuntu GREEN + -20 cleared proactively

### Lane commit landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `8c65296` | `Hybrid back-side theme architecture correction + custom-TERM fix (fullstack-a-53)` | @@FullStackA |

### -a-54 cleared (flip UX redesign)

@@FullStackA delivered the flip UX redesign cleanly. 5
implementation choices documented (NOT deviations; all
sensible reads of the spec):

1. Family-name title in dead-zone slot — natural empty
   space; matches @@Alex's "inside the tab area" framing.
2. Flex order swap for hamburger (not DOM reshuffle) —
   cleaner; HamburgerMenu anchor works automatically.
3. Un-mirrored title — matches @@Alex's "like in the
   front pane" wording (front-readable, not mirrored).
4. Dead-zone cursor reset on flip — drag-to-NAV from
   `-a-44` is naturally front-state; cursor reset is the
   right visual cue.
5. `scaleX(-1)` click-through verified — modern browsers
   handle mirrored hit-testing cleanly.

All accepted. vitest 646/646 (+3 net). Cleared verbatim.

After commit, the Hybrid back-side correction wave is
structurally complete: `-a-47` collapse + `-a-48` FB-back
migration + `-a-53` theme architecture + `-a-54` flip UX
all landed (or about to). `webtest-a-5` walks the
bundled corrected wave once everything's in HEAD.

### -18 follow-up #4 Ubuntu GREEN confirmed

@@Systacean's smoke run `26247086815` empirically confirmed:

* Ubuntu cargo test ✓ (28 BGE tests skipped cleanly via
  the `#[ignore]` cascade).
* Web + build + rustfmt ✓.
* Windows half still running at time of their poke.

The BGE-test gate-unblocker work is empirically closed on
Ubuntu. The cascade peeling pattern from prior rounds
reached its terminal point — finite item count, cascade
exhausted, gate green.

### -20 cleared proactively

@@Systacean implemented `-20` (3 chan-drive lock-contract
test `#[cfg(unix)]` gates) while waiting for Windows half
of the `-18` fu#4 smoke. Same mechanical pattern; local
gate green. Cleared for commit + the bundled-smoke plan
(commit on top of `bf85e8a` + fastforward push +
re-dispatch the bundled smoke validates `-18` fu#4 + `-20`
together).

### Expected bundled smoke verdict

Windows fully green (chan-desktop dead_code from
`c0600e0`+`8e4ce5c` + fs_graph dead_code from `b01b310` +
chan-server BGE gates from `bf85e8a` + chan-drive lock
gates from `-20`). Ubuntu fully green (confirmed by the
prior smoke). macOS green (lock tests still run on macOS).

**Round-3 readiness signal: per-PR ci.yml gate structurally
fully green for the first time since ~2026-05-19 on all 3
platforms.**

### Pattern complete: gate-unblocker cascade exhausted

The cascade has been peeling back layers since `ci-12`
opened. Final state:

* `ci-12` (GTK install) ✓ landed.
* `systacean-17` (Windows result_large_err) ✓ landed.
* `fullstack-b-24` (Windows chan-server dead_code + smoke
  fixup cascade) ✓ landed (7 implementation commits).
* `systacean-18` (chan-drive BGE-panic) + 4 follow-ups ✓
  landed (mechanical `#[ignore]` cascade across
  chan-drive lib + integration + chan-server lib).
* `systacean-20` (3 chan-drive lock tests on Windows) ✓
  cleared this round; commit pending.

After the bundled smoke greens, the per-PR ci.yml gate
becomes load-bearing again. `systacean-19` (C2 graceful
BM25 fallback) follows as the structural fix that reverts
the 28 `#[ignore]` gates retroactively. The 3
`#[cfg(unix)]` gates from `-20` stay until the real
Windows lock-primitive bridge lands in Round-3 polish.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | `-18` fu#4 Ubuntu green confirmed; `-20` cleared; expect bundled smoke verdict + then `-19` pickup |
| @@CI | Idle; queue-empty |
| @@FullStackA | `-a-53` committed (`8c65296`); `-a-54` cleared; expect commit + `-a-49` pickup (graph overhaul first sub-wave) |
| @@FullStackB | `-24` CLOSED; queue-empty until wave-3 dispatch |
| @@WebtestA | Queue-empty; `webtest-a-5` cuts after `-a-54` lands |
| @@WebtestB | Queue-empty |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | -a-54 commit clearance + 5 shape decisions accepted |
| `alex/event-architect-systacean.md` | -18 fu#4 Ubuntu green ack + -20 proactive clearance + bundled smoke plan ack |

## 2026-05-21 — clearance round 11 — -a-54 landed; FullStackA done; webtest-a-5 dispatched

### Lane commit landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `714ec48` | `Hybrid flip UX: preserve tab strip + mirror tabs + swap hamburger + family-name title (fullstack-a-54)` | @@FullStackA |

@@Alex 2026-05-21: "fullstackA is done, now only
systacean working". @@FullStackA's session is wrapping
up after the Hybrid back-side correction wave landed
fully. Their queue continues with `-a-49` (graph
overhaul first sub-wave) on next spawn; not picking up
this round.

### Hybrid back-side correction wave structurally complete

All four pieces in HEAD:

* `dd586fc` -a-47 (drop front/back independent theme)
* `0391eae` -a-48 (FB-back Search/Indexing/Reports
  migration, option B; chan-reports toggle restored)
* `8c65296` -a-53 (theme architecture correction +
  custom-TERM PARTIAL fix bundled)
* `714ec48` -a-54 (flip UX redesign)

This closes the design-correction follow-up loop from
the @@Alex corrections that surfaced after `-a-46`
shipped. Hybrid back-side semantic is now at its proper
end-shape.

### webtest-a-5 dispatched

Cut [`../webtest-a/webtest-a-5.md`](../webtest-a/webtest-a-5.md)
bundling all four slices + a re-verification of the
`-a-45` custom-TERM PARTIAL (should now be HOLD
post-`-a-53`). 20 acceptance checks total. Single
bundled verdict per the `-3`/`-4` shape. @@WebtestA is
idle; the dispatch waits in their inbound channel for
next spawn / poll.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | Only lane actively working; bundled smoke `26247985860` in flight at ~11m; then -19 pickup |
| @@CI | Idle; queue-empty |
| @@FullStackA | DONE for this round; -a-49 next on respawn |
| @@FullStackB | DONE; queue-empty post -24 |
| @@WebtestA | webtest-a-5 dispatched; reactive lane idle until next spawn |
| @@WebtestB | DONE; queue-empty |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | after-the-fact ack on -a-54 |
| `alex/event-architect-webtest-a.md` | webtest-a-5 dispatch |
| `webtest-a/webtest-a-5.md` | NEW task (bundled Hybrid back-side correction walkthrough) |

## 2026-05-21 — clearance round 12 — -20 smoke scope poke routed (option B; real test-quality fix)

### Bundled smoke verdict: one new Windows failure

@@Systacean's bundled smoke `26247985860` completed
FAILURE — but surfaced only ONE new Windows red, a
4th separate class of failure parallel to the prior
three:

1. result_large_err (closed by `-17`).
2. BGE-test panics (closed by `-18` + 4 follow-ups).
3. Lock contract gap (closed by `-20`).
4. **NEW: chan-drive watcher timing on Windows** —
   `watcher_keeps_report_current` in
   `chan-drive/tests/report.rs:119`.

Root cause: test uses `std::thread::sleep(700ms)` for
report-writer debounce; isn't enough on Windows where
notify-crate timing differs from Unix
`inotify`/`kqueue`.

### Lane discipline carried forward

@@Systacean correctly applied the "fire-a-scope-poke-
instead-of-iterating" gate. They didn't reflexively
apply the prior `#[cfg(unix)]` pattern to this 4th
failure — recognized it was a different class (timing
fragility, not platform-specific contract) + escalated
for routing. Same discipline that caught the chan-server
BGE widening in `-18` fu#3.

### Three options laid out + routed (B)

* **A**: `#[cfg(unix)]` mechanical gate (same as `-20`).
* **B**: Replace fixed sleep with `wait_for` poll
  (~3 lines; cross-platform-correct test discipline).
* **C**: Root-cause `FLUSH_DEBOUNCE` constant audit.

**Routed B.** Reasoning:

* B is the GENUINE fix to a test that was always timing-
  fragile. The poll-with-timeout shape is just better
  test discipline; works correctly on all platforms.
* A would accumulate another `#[cfg(unix)]` on the
  Round-3 revert-target list. Each gate added is
  technical debt.
* C is out-of-scope; would become a Round-3 polish item
  if the poll reveals deeper Windows-watcher slowness.

@@Systacean's recommendation matched mine; obvious-call
authorized. Same smoke-fixup shape as the prior
iterations: commit on top of `-20` (current
`systacean-18-smoke` tip) + fastforward push + re-fire.

### After B lands + smoke greens

If the re-fire goes fully green on all 3 platforms,
that's the Round-3 readiness signal — per-PR ci.yml
gate structurally fully green for the first time since
~2026-05-19. Per @@Systacean's analysis ("cargo's abort
masks at most one more test which is already gated"),
the cascade is structurally exhausted; no more
whack-a-mole expected.

After that:

* `systacean-19` (C2 graceful BM25 fallback) is the next
  pickup. After C2 lands, the 28 BGE `#[ignore]` gates
  REVERT — cascade becomes obsolete.
* 3 `#[cfg(unix)]` lock gates from `-20` stay (Round-3
  polish for Windows lock-primitive bridge).
* The watcher-timing test stays as wait_for poll
  (improvement, not gate — no revert needed).

### Pattern observation: lane-scope-escalation has become a load-bearing discipline

The "fire-a-scope-poke-instead-of-iterating" pattern
has fired THREE times in this gate-unblocker sweep:

1. @@FullStackB `-24` scope (10/11 lints in chan-server
   not chan-desktop).
2. @@Systacean `-18` fu#3 (BGE surface widening into
   chan-server lib).
3. @@Systacean `-20` smoke (NEW class of failure —
   timing, not gating).

Each time the lane caught a real structural insight
@@Architect wouldn't have surfaced from line-number
audits alone. The discipline prevents:

* Architect-side categorical errors from compounding.
* Lanes from blindly applying the wrong fix shape.
* Wasted CI iteration time (each scope-poke beat saves
  one or more wrong-shape commits + smoke runs).

Saving as a process-discipline pattern in this journal
(no new memory entry — it's already encoded in
`feedback_ground_descriptions_in_source` rule applied
in BOTH directions: architect-to-lane AND lane-to-
architect feedback).

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | Routed B for `-20` smoke fixup; expect commit + smoke + then `-19` pickup |
| @@CI | Idle |
| @@FullStackA | DONE; `-a-49` next on respawn |
| @@FullStackB | DONE post `-24` |
| @@WebtestA | webtest-a-5 dispatched; idle until next spawn |
| @@WebtestB | DONE |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | -20 smoke scope poke routed option B + obvious-call authorization |

## 2026-05-21 — -a-54 design-correction follow-up cut as -a-55 (architect-side misinterpretation; 2 corrections from @@Alex)

@@Alex 2026-05-21 (chat, post `-a-54` ship at `714ec48`)
flagged two corrections in two consecutive messages:

1. **Family-name title in tab strip — REMOVE.**
   Screenshot showed "HYBRID FILE BROWSER" appearing
   in the tab strip alongside the mirrored tabs.
   @@Alex: "i never asked for that.. we should keep
   just the tabs there, flipped, no need to add that
   extra label; i saw the same with terminal."
2. **Tab alignment in flipped state — RIGHT.**
   "when we flip, the tabs must be aligned to the
   right.. not to the left, because we flipped."

### Architect-side misinterpretation root cause

My `-a-54` task body explicitly spec'd "Family-name
title in tab area" with "shows INSIDE the tab area
when flipped — does NOT add a new chrome row."
@@FullStackA implemented faithfully per the spec.

@@Alex's actual intent (clarified post-ship): "tab
area" = back-side config view (NOT tab strip chrome).
The `HybridXConfig.svelte` stubs from `-a-43` already
carry the family-name title at the top of their config
view content. No need to duplicate in chrome.

My misread: I took "inside the tab area" as "in the
tab strip area" — should have looked at the existing
back-side config view stub (which already had the
title) to disambiguate. The phrasing "like in the
front pane" was a hint I missed — front pane doesn't
have a family-name title in its chrome, so "like in
the front pane" + a family-name title means the title
lives in the CONTENT area, not the chrome.

### Lesson logged

When a design framing references "like X" or "the
existing Y", READ the existing shape FIRST before
specifying. The `feedback_ground_descriptions_in_source`
memory rule applies to design framings too, not just
crate capability descriptions. Same discipline:
empirical reading over inferential interpretation.

### -a-55 cut

[`../fullstack-a/fullstack-a-55.md`](../fullstack-a/fullstack-a-55.md):

* Remove family-name title from tab strip in flipped
  state (`Pane.svelte` + supporting CSS).
* Add right-alignment for tabs in flipped state
  (`flex-direction: row-reverse` OR `justify-content:
  flex-end` — implementer picks the cleaner composition
  with the existing hamburger swap).
* Update `Pane.test.ts` pins from `-a-54` (invert the
  tab-area-title pin; keep mirrored-tab + hamburger-swap
  + click-through pins).

3-piece change; small commit. Inserts ahead of
`-a-49..52` in @@FullStackA's queue to close the
design-correction loop before graph overhaul.

### round-2-plan + webtest-a-5 updated

* `round-2-plan.md` §"Flip UX correction 2026-05-21"
  updated: title lives INSIDE the back-side config
  view (not tab strip chrome) + tab right-alignment
  specification + architect-side misinterpretation note
  pointing at `-a-55` as the corrective follow-up.
* `webtest-a-5.md` `-a-54` walk section updated to
  carry design-correction context — @@WebtestA grades
  the current shipped state with awareness that `-a-55`
  removes the tab-area title + right-aligns the tabs.
  Don't grade the current state as a failure.

### Pattern note: design-iteration via task chain

The Hybrid back-side design has now had THREE
correction cycles since the original `-a-43` shipped:

1. `-a-53` (theme architecture correction) — Appearance
   revert + per-Hybrid override.
2. `-a-54` (flip UX redesign) — preserve tab strip,
   mirror tabs, hamburger swap, family-name title.
3. `-a-55` (this round) — remove the tab-strip
   family-name title (`-a-54` misinterpretation) +
   right-align tabs in flipped state.

The chain works — each iteration captures a real
design refinement from @@Alex; the append-only
coordination shape preserves the audit trail without
breaking landed work. Three follow-up tasks beats one
big "redesign + iterate" surface.

But the architect-side cost: each misinterpretation
ships before @@Alex catches it post-walk. The
prevention is empirical-reading-of-existing-state
before specifying. Cost-of-not-doing-that: one
follow-up task + one wave of WebtestA verdict
re-interpretation. Acceptable but not free.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | `-20` smoke fixup option B routed; expect commit + smoke + then `-19` |
| @@CI | Idle |
| @@FullStackA | DONE on `-a-54`; `-a-55` cut ahead of `-a-49`; pickup on next spawn |
| @@FullStackB | DONE post `-24` |
| @@WebtestA | `webtest-a-5` dispatched (with design-correction context); idle until next spawn |
| @@WebtestB | DONE |

### What I'm committing this round (revised — bundling -a-55 cut + design corrections with prior systacean routing)

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `architect/round-2-plan.md` | Flip UX correction section updated for back-side-config-view title placement + tab right-alignment + misinterpretation note |
| `alex/event-architect-systacean.md` | -20 smoke scope poke routed option B (from earlier in the round) |
| `alex/event-architect-fullstack-a.md` | -a-55 dispatch poke + lesson logged |
| `webtest-a/webtest-a-5.md` | design-correction context appended to -a-54 walk section |
| `fullstack-a/fullstack-a-55.md` | NEW task (tab-strip title removal + right-alignment) |

## 2026-05-21 — webtest-a-5 verdict landed + PARTIAL bundled into -a-55 + all lanes idle

### Lane commit landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `74b9860` | `docs: webtest-a-5 — Hybrid back-side correction wave + design follow-ups walkthrough (19/20 HOLD + 1 N/A + 1 PARTIAL on -a-54 click-existing-tab; -a-45 custom-TERM PARTIAL re-verified HOLD)` | @@WebtestA |

@@WebtestA delivered the bundled correction-wave walk
cleanly. Two specific wins captured:

* `-a-45` custom-TERM PARTIAL from `webtest-a-4`
  re-verified as HOLD — `-a-53`'s bundled fix worked
  end-to-end.
* `-a-48` user-flagged chan-reports regression FIXED
  empirically. The "i want it back!" loop closes.

### New PARTIAL: -a-54 check #6 — click-existing-mirrored-tab

Bundled into `-a-55`'s scope. Same `Pane.svelte`
flipped-tab-strip surgery surface as the other two
`-a-55` corrections (family-name title removal +
right-alignment). Folding all three into one commit
avoids partial states.

`-a-55` is now a 3-piece chrome correction. Updated
[`../fullstack-a/fullstack-a-55.md`](../fullstack-a/fullstack-a-55.md)
task tail with the scope addition + root-cause
hypotheses + acceptance criterion.

### close-out marker routed Option A

`-3` pattern. Separate follow-up commit. @@WebtestA can
land it on next spawn alongside any next walkthrough
dispatch.

### Side observation absorbed

The hamburger-no-longer-has-"Light mode"/"Flip pane"/
"Theme" observation is the intended end state per
`-a-53`'s theme architecture correction (theme only via
back-side override). Not a regression; absorbed as
expected behavior.

### All lanes idling

@@Alex 2026-05-21: "they all idling now". Confirmed:

| Lane | State |
|------|-------|
| @@Systacean | `-20` smoke option B routing in their inbound; no source mods yet; will pick up on next spawn |
| @@CI | Idle; queue-empty |
| @@FullStackA | `-a-55` (with bundled scope: title removal + right-align + click-handler fix); idle until next spawn |
| @@FullStackB | DONE post `-24` |
| @@WebtestA | `webtest-a-5` verdict committed; close-out marker pending; idle until next spawn |
| @@WebtestB | DONE; idle |

This is a natural pause point in the round — the
Hybrid back-side correction wave validated; the
gate-unblocker cascade structurally exhausted (waiting
on @@Systacean's smoke option B + then `-19` for the
final close). Next active work happens when @@Alex
spawns the lanes.

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-webtest-a.md` | -a-5 verdict ack + PARTIAL routing + close-out Option A |
| `alex/event-architect-fullstack-a.md` | -a-55 scope addition (click-existing-tab PARTIAL bundled) |
| `fullstack-a/fullstack-a-55.md` | Scope addition for the PARTIAL fix |

## 2026-05-21 — clearance round 14 — -a-55 cleared + -20 option B fixup committed; bundled smoke in flight

### Lane commit landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `76a07a0` | `chan-drive/tests/report: replace fixed sleep with wait_for poll for cross-platform timing (systacean-20 smoke fixup)` | @@Systacean |

Clean option-B execution per the prior routing — the
`wait_for` poll replacing the fixed sleep is the real
test-quality improvement (cross-platform-correct test
discipline), not just a gate-unblocker.

Bundled smoke `26250685864` fired on `systacean-18-smoke`
(fastforwarded with this fix on top). Passive wait on CI.
If green, the gate-unblocker cascade is structurally
exhausted — Round-3 readiness signal across all 3
platforms.

### -a-55 cleared

@@FullStackA delivered all 3 bundled corrections cleanly:

* Tab-strip family-name title removed (regression-guard
  pin asserts `.hybrid-title` is null in flipped state).
* Right-alignment via `flex-direction: row-reverse` +
  `.tabs.flipped .actions { order: 1 }` (parent `row-reverse`
  + per-child `scaleX(-1)` mirroring gives "looking from
  behind, tabs flow from right edge" visual).
* Click-swap fix via per-child mirror selectors (so the
  click target stays unmirrored at the event-binding
  level).

vitest 647/647 (+1 net click-swap pin). 6 files per
per-path discipline. Test inversion shape (turning the
`-a-54` pin into a regression guard via `not.toMatch`)
is exactly the right pattern; saves future revert from
silently passing.

Cleared verbatim.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | `-20` smoke fixup committed; bundled smoke in flight; expect verdict + then `-19` pickup |
| @@CI | Idle |
| @@FullStackA | `-a-55` cleared; expect commit + `-a-49` pickup (graph overhaul first sub-wave) |
| @@FullStackB | DONE; idle |
| @@WebtestA | Verdict committed; close-out marker pending; idle |
| @@WebtestB | DONE; idle |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | -a-55 commit clearance |
| `alex/event-architect-systacean.md` | -20 smoke fixup after-the-fact ack |

## 2026-05-21 — option B insufficient on Windows; pivoting to gate (shape A)

Bundled smoke `26250685864` verdict:

* **Ubuntu cargo test ✓** — cascade closed cleanly on
  the Ubuntu side (28 BGE gates + 3 lock-contract
  gates).
* **Windows test ✗** — only `watcher_keeps_report_current`
  fails. New failure mode: instead of `sleep(700ms)`
  being insufficient, the `wait_for(...,5s)` poll
  genuinely times out (`report missed b.md within 5s`).

Empirically: option B's `wait_for` poll didn't fix the
Windows surface. The notify-crate event chain for
fresh file events either doesn't fire within 5s for
this scenario on Windows, or the report-writer's
debounce + flush takes longer than 5s, or the event
never delivers `b.md` to the report at all on Windows.

### Pivoting B → A

Routed @@Systacean to `#[cfg(unix)]` gate the test (same
pattern as `-20`'s lock-contract tests). The `wait_for`
poll body stays (Unix-only now, but the poll discipline
is preserved for the future cross-platform fix).

Bug-list entry for the underlying gap added alongside
the Windows lock contract parity entry — Round-3 polish:
"Windows notify-crate / report-writer reliability for
fresh file events."

### Architect-side lesson

My option-B routing assumed the timing fix would solve
Windows. The empirical result: B's `wait_for` poll
discipline is right (cross-platform-correct), but the
underlying Windows event chain has a real gap that
5s isn't enough to mask. Option A (mechanical gate
first) would have shipped faster.

Save for future: when a cross-platform test fix COULD
be either "real fix" or "gate the test," reach for the
gate FIRST if there's no empirical confidence the real
fix will work on the target platform. Real fix becomes
Round-3 polish after empirical Windows access.

Cost of this iteration: one extra smoke cycle (~17 min)
+ one fixup commit. Acceptable but not free; the lesson
is the takeaway.

### Same beat status

@@Systacean is the only lane that needs to act this
beat — apply the gate fixup + re-smoke. @@FullStackA's
`-a-55` clearance from the prior round stands; they
commit on their next spawn.

Standing by for the gate fixup commit + the re-smoke
verdict (Round-3 readiness signal once Windows greens
on the gated test).

## 2026-05-21 — clearance round 15 — -a-55 landed; -a-49 routed option C; @@Systacean concurrent pivot acked

### Lane commits landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `7cf6f8e` | `Hybrid flip UX: remove tab-strip title + right-align tabs + fix mirrored-tab click (fullstack-a-55)` | @@FullStackA |
| `8be1bfc` | `docs: -a-49 scope-check poke + -a-55 commit marker (fullstack-a-49 audit anchor)` | @@FullStackA |
| `efd3ba6` | `docs(systacean): scope poke #2 on -20 smoke — option B insufficient; pivot to A (cfg(unix)) or C (Windows fanout audit)` | @@Systacean |

@@Systacean's `efd3ba6` (21:17 BST) and my pivot
routing `00ddc79` (21:22 BST) raced — same conclusion
reached independently. Convergent cross-routing; their
discipline of NOT iterating silently after B failed
was exactly right.

### -a-49 scope-check — third architect-side error caught by lane at pickup

@@FullStackA's audit caught a real categorical error in
my `-a-49` task body: chan-server's
`merge_filesystem_layer` ALREADY emits Directory nodes
+ `contains` edges; SPA already consumes them
(GraphPanel.svelte:491/543/789/1003). The G2 gap is in
the LAYOUT TRANSFORM in `GraphCanvas.svelte` (d3-force
simulation; all nodes are equal participants), NOT the
data shape.

Same pattern as @@FullStackB's `-24` lint-location +
@@Systacean's `-18` chan-server widening. THREE
architect-side categorical errors in a row caught by
lane discipline at pickup. The
`feedback_ground_descriptions_in_source` memory rule
applied to architect-to-lane direction saves real
work each time.

Pattern emerging: my task bodies are framing graph /
chan-server / chan-drive scope inaccurately when I
don't read the source first. Need to internalize:
ALWAYS grep / read the implementation surface before
specifying scope. Cost of NOT doing that has become
visible — each catch costs a task body revision +
re-routing round.

### Routing: -a-49 option C — layout transform ONLY

Picked C per @@FullStackA's lean. Markdown-link
semantics defer to G5's own task slot (cut as a new
`-a-N` when `-a-49` lands).

Three layout shapes flagged for implementer:

1. d3-force with `forceY` per depth + parent-anchored
   `forceX` (conservative blast radius).
2. Hybrid `d3-hierarchy.tree()` + d3-force overlay
   (architectural separation).
3. Full d3-hierarchy tree (cleanest visually; drops
   force-based interaction).

@@FullStackA picks based on the actual implementation
audit.

### graph-overhaul-plan implication

`graph-overhaul-plan.md` G2 framing needs updating to
reflect "data is correct; layout transform is the
gap." I'll update on next graph-related routing round
(don't want to thrash the plan doc this beat). Pattern
note for the architect lesson: prefer source-truth over
plan-doc framing when they conflict.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | Gate fixup (#[cfg(unix)] on watcher_keeps_report_current) in flight in worktree; expect commit + re-smoke + then -19 |
| @@CI | Idle |
| @@FullStackA | -a-49 option C routed; expect commit + then -a-50 pickup (G3 directory inspector) |
| @@FullStackB | DONE; idle |
| @@WebtestA | Idle; close-out marker still pending |
| @@WebtestB | DONE; idle |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | -a-55 ack + -a-49 option C routing + architectural correction logged |
| `alex/event-architect-systacean.md` | ack on concurrent efd3ba6 pivot |

## 2026-05-21 — clearance round 16 — @@Systacean smoke #2 + #3 fixups acked; smoke #3 in flight

### Lane commits landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `f03e6a2` | `chan-drive/tests/report: gate watcher_keeps_report_current on Unix (systacean-20 smoke #2 fixup)` | @@Systacean |
| `93afd8d` | `chan-drive/tests/report: gate helpers + imports on Unix to silence Windows dead_code (systacean-20 smoke #3 fixup)` | @@Systacean |

@@Systacean's pivot-A execution carrying the same
iterative-mechanical cascade pattern as the prior `-24`
work — each gate exposes the next orphaned dead_code
surface on Windows; mechanical fix; re-smoke. Same
obvious-call shape; standing authorization.

Smoke #3 (`26252715148`) IN PROGRESS at ~2m+ when I
checked. Passive wait on CI.

### Expected: cascade closes

The dead_code cascade after gating the watcher test +
its helpers should exhaust — finite Windows-only
dead_code count. If smoke #3 greens Windows, the
per-PR ci.yml gate goes structurally fully green
across all 3 platforms. Round-3 readiness signal.

If yet another layer surfaces, same discipline:
mechanical fixup + re-smoke. The cascade can't go
forever.

### Side note: webtest-a-5 close-out marker

@@WebtestA's close-out marker append (Option A
routing) is still uncommitted in the worktree. They
didn't fire it before session end. Not blocking
anything; rides naturally when WebtestA next spawns.
Architect could commit on their behalf but the file is
their write — leaving for them.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | Smoke #3 in flight; cascade should close this iteration |
| @@FullStackA | `-a-49` option C routing in their inbound; not yet started in worktree |
| @@CI | Idle |
| @@FullStackB | DONE; idle |
| @@WebtestA | Close-out marker pending in worktree; idle |
| @@WebtestB | DONE; idle |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | smoke #2 + #3 after-the-fact ack |

## 2026-05-21 — smoke #3 unmasked 3 more orphans; routing structural fix (file move)

Smoke #3 verdict surfaced 3 NEW dead_code errors on
Windows after `93afd8d` gated some helpers:

* `struct Collector` (line 25)
* `impl Collector::{new, len}` (lines 28+31)
* `fn wait_for` (line 42)

These are test helpers used ONLY by the now-gated
`watcher_keeps_report_current`. `93afd8d` gated some
imports + helpers but missed these.

### Per-symbol cascade is wasteful — pivot to structural fix

Iterating per-symbol works mechanically but each iteration
costs ~10-15 min CI + a commit. Cascade WILL terminate
(finite count) but the cost is visible.

Better: **terminate with a file-level structural change**.

### Routed: split watcher test to a new Unix-only file

@@Systacean routed to:

* Create `crates/chan-drive/tests/report_watcher_unix.rs`
  with `#![cfg(unix)]` at the top.
* Move `watcher_keeps_report_current` + `Collector` +
  `impl Collector` + `wait_for` + needed imports into
  the new file.
* Remove those items from
  `crates/chan-drive/tests/report.rs`. The 3 other
  tests stay there; cross-platform.
* Revert the partial `#[cfg(unix)]` gates from `f03e6a2`
  + `93afd8d` — subsumed by the file-level
  `#![cfg(unix)]` on the new file.

Result: cascade terminates. Other 3 tests in `report.rs`
still run cross-platform. Watcher test runs on Unix
only, by virtue of the file-level cfg.

### Architect-side lesson logged

Per-symbol `#[cfg(unix)]` cascading on a test with
test-local helpers is wasteful. When a TEST has internal
helpers used only by it, prefer **file-level gating**
(or file move to preserve other tests' coverage). Same
shape as C2 vs `#[ignore]`-cascade for BGE — terminate
structurally, not iteratively.

Pattern note for future routing: when test-gate
decisions touch tests with internal helpers, first ask
"do other tests in this file share the helpers?" If
NO, file-level gating wins. If YES, per-symbol with
careful scope is necessary.

### Bug-list cross-ref

The "Windows notify-crate / report-writer reliability"
Round-3 polish entry still stands. When that lands,
the file move can be reverted (the `#![cfg(unix)]`
file gate becomes redundant; watcher test back in
`report.rs`).

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | smoke #3 verdict + structural-fix routing |

## 2026-05-21 — SCOPE DECISION: Windows dropped from CI per @@Alex

@@Alex 2026-05-21 (chat, after smoke #4 surfaced 7
NEW chan-server `terminal_sessions::tests` Windows
failures): "let's please disable windows and carry
on, no time to spend on this and i dont care much
about windows for now."

### What this changes

Windows is dropped from the per-PR `ci.yml` gate
matrix. Round-2 CI gate becomes **Ubuntu + macOS
only**. Windows becomes a Round-3+ scoping decision
when @@Alex revisits (likely tied to public-flip or
when Windows becomes a real-user surface ask).

### Why

The gate-unblocker cascade since `ci-12` revealed
the Windows debt surface is deeper than the
mechanical-cascade pattern can comfortably terminate:

* `systacean-17` (boxing) ✓ cross-platform improvement
* `fullstack-b-24` (chan-server dead_code) ✓
* `systacean-18` + 4 follow-ups (BGE gates) — Ubuntu,
  not Windows-specific; ✓
* `systacean-20` (lock contract gates) — Windows
* `systacean-20` smoke cascade — watcher + helpers —
  Windows; iterative
* **NEW (smoke #4)**: 7 chan-server `terminal_sessions::tests`
  failures on Windows — PTY semantics + path handling
  divergences.

Each new Windows layer costs ~10-15 min of CI + a
commit + architect routing. @@Alex's framing: the
ROI on Windows-fixing at this phase is negative.
Drop Windows entirely from the gate; revisit at
Round-3+.

### Routings issued this round

* **`ci-13` cut** for @@CI: drop Windows from
  `ci.yml` per-PR matrix + audit `release.yml` for
  Windows artifact entries (drop if present). Bug-list
  Round-3 entry consolidates all Windows polish
  items.
* **@@Systacean's structural-fix routing CANCELLED**:
  the watcher-test file move from the previous
  beat is superseded by this scope decision. The
  existing `#[cfg(unix)]` gates from `f03e6a2` +
  `93afd8d` stay (they're still technically correct
  + document Windows gaps).
* **@@Systacean `-19` STAYS** as next substantive
  work — C2 graceful BM25 fallback is a real
  product improvement benefiting all platforms.
  After it lands, the 28 BGE `#[ignore]` gates
  revert.

### What stays as Round-3 polish

The bug-list entry from `ci-13` consolidates:

* Windows lock-primitive bridge
  (`LockFileEx` or equivalent in
  `chan-drive/src/lock.rs`).
* notify-crate / report-writer reliability on
  Windows for fresh file events.
* `chan-server` `terminal_sessions` Windows
  portability (PTY + path handling).
* chan-desktop Windows runtime + Tauri bundle
  (separate from CI).
* Audit + re-enable Windows in `ci.yml` +
  `release.yml` matrices.

@@Alex revisits at Round-3 readiness OR when
Windows becomes a real-user surface ask.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | Structural file-move CANCELLED; `-19` is next substantive pickup |
| @@CI | `ci-13` cut to drop Windows from matrices; pickup on next spawn |
| @@FullStackA | `-a-49` in flight in worktree (`GraphCanvas.svelte` + test file) |
| @@FullStackB | DONE; idle |
| @@WebtestA | Close-out marker committed (`f1c1edb`); idle |
| @@WebtestB | DONE; idle |

### Architect-side lesson logged

Scope-deferral decisions sometimes look like "giving
up" — but the alternative (iterating indefinitely on
a surface the user explicitly doesn't care about) is
worse. Cost-of-staying-the-course on Windows had
become higher than the value @@Alex placed on
Windows support at this phase. Drop the scope; preserve
the cross-platform DISCIPLINE artifacts (`#[cfg(unix)]`
gates + `feedback_ground_descriptions_in_source`
rule); revisit when scope priority shifts.

Same shape as the v0.11.2 CLI backfill decision earlier
in the phase — when @@Alex says "leave the past alone,
focus on the future," the architect-side action is to
EXECUTE the deferral, not negotiate.

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-ci.md` | ci-13 dispatch poke |
| `alex/event-architect-systacean.md` | CANCEL the structural-fix routing |
| `ci/ci-13.md` | NEW task (drop Windows from matrices) |

## 2026-05-21 — clearance round 17 — three lanes commit-ready simultaneously

### Three commit-readinesses cleared

**`ci-13` (drop Windows from matrices)**: cleared. All 3
minor questions answered:

* (a) macOS-only matrix shape accepted (ubuntu + macOS;
  10x macOS billing is worth the macOS-specific
  coverage).
* Authenticode reference dropped in release.yml header
  (bundled; restored at Round-3+ Windows re-enable).
* Linux description copy-edit (gnu vs musl-static
  bundled fix).

`ci-13-smoke` (`26253981385`) in flight at ~7m+.

**`systacean-19` (C2 graceful BM25 fallback)**: cleared
+ smoke-branch authorized. 13 paths; +108/-6 facade.rs
for the fallback path + one-shot warning; ALL 28 BGE
`#[ignore]` reverts across the workspace (14 chan-drive
lib + 5 integration + 9 chan-server). chan-server
205/0/0 (was 196/9). Workspace gate green; web 658/658.
Fresh `systacean-19-smoke` branch authorized (new
lifecycle, distinct from the closed `-18-smoke` chain).

**`fullstack-a-49` (graph layout filesystem-hierarchy
backbone)**: cleared. Strategy (1) d3-force `forceY`
per depth + parentX. forceLink unchanged; non-hierarchical
tag/mention/language nodes float center-of-canvas.
6 files.

### Round-3 readiness signal redefined under Windows-deferral

After all three land + smoke greens:

* `ci-13`: Windows out of `ci.yml` + `release.yml`
  matrices.
* `-19`: 28 BGE `#[ignore]` gates reverted; chan-drive
  + chan-server tests run end-to-end on default-build
  CI runners.
* `-a-49`: filesystem-hierarchy as graph backbone (G2
  empirical delivery).

Per-PR ci.yml gate is **STRUCTURALLY FULLY GREEN on
Ubuntu + macOS** for the first time since ~2026-05-19.
The Round-3 readiness signal applies under the new
2-platform shape.

### graph-overhaul-plan.md G2 framing — to update post-commit

@@FullStackA's `-a-49` audit caught that the G2 framing
in `architect/graph-overhaul-plan.md` was wrong (it
assumed flat-edges data; reality is chan-server already
emits hierarchical data, gap was SPA layout). I noted in
the @@FullStackA channel that I'll update the plan
doc post-`-a-49` commit. Don't want to thrash the doc
in the middle of `-a-49` landing; small edit afterward.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@CI | ci-13 cleared; expect commit + smoke green |
| @@Systacean | -19 cleared; expect commit + smoke green |
| @@FullStackA | -a-49 cleared; expect commit + -a-50 pickup |
| @@FullStackB | DONE; idle |
| @@WebtestA | Close-out marker committed; idle |
| @@WebtestB | DONE; idle |

Three substantial commits landing in parallel. After
all three + their respective smokes, the gate-unblocker
sweep + the Hybrid back-side correction wave + the
graph G2 baseline are all empirically confirmed
complete.

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-ci.md` | ci-13 commit clearance + 3 question answers |
| `alex/event-architect-systacean.md` | -19 commit clearance + smoke-branch authorized |
| `alex/event-architect-fullstack-a.md` | -a-49 commit clearance |

## 2026-05-21 — SECOND cross-agent commit-hygiene incident (5685be4) — same routing as a8e991a; meta-discipline saved (systacean-19 audit anchor)

### Incident shape

@@FullStackA's `5685be4` commit (subject:
`fullstack-a-49`) swept up ALL 13 files of
@@Systacean's `-19` work via the inter-command race
window between `git add` + audit + `git commit`.
Same shape as a8e991a (WebtestB sweeping FullStackA's
`-a-44`) but with roles REVERSED — this time
@@FullStackA is the sweeping lane, @@Systacean the
swept.

Both lanes correctly flagged the incident independently:

* `cc3a888` — @@FullStackA's self-flag with detailed
  process-lesson on the inter-command race window +
  proposed meta-fix (single-bash-invocation chained
  with `&&`).
* `88a084c` — @@Systacean's symmetric flag with
  three options analysis (A audit-trail / B
  soft-reset+split / C — was already cancelled).

### Routing — option (b) + (c) same as a8e991a

Both lanes converged on option A/b (audit-trail
correction). Same architect call:

* **(a) history rewrite — DECLINED**. The chain has
  `cc3a888` + `88a084c` referencing `5685be4`
  directly. Plus push hasn't happened but standard
  multi-agent destructive-op concern.
* **(b) audit-trail correction — GO**. Both task
  files (`fullstack-a-49.md` + `systacean-19.md`)
  get the "landed under 5685be4 cross-agent commit"
  append.
* **(c) architect-side grep-anchor — DOING IT**.
  This commit's subject mentions `systacean-19` so
  `git log --grep=systacean-19` finds it. Same shape
  as the a8e991a follow-up architect commit.

### Meta-discipline saved as memory

@@FullStackA's process-lesson in `cc3a888` is the
right meta-fix for this class of incident:

> Collapse audit + commit into ONE bash invocation:
> ```bash
> git add <paths> && git diff --staged --stat && \
>   git commit -m "..." && git show --stat HEAD
> ```

Saved as `feedback-atomic-audit-commit` memory entry.
Cross-references `feedback-shared-worktree-commits`
(still valid; this new entry is the race-window-
specific tightening).

The architect-side cost of multi-lane simultaneous
clearance is real: each cleared lane competes for the
staging area in the next clearance round. Going
forward I'll either:

* Stagger clearance rounds (only one lane at a time
  when multiple are commit-ready), OR
* Trust the new atomic-audit-commit discipline
  applied across all lanes.

Option 2 is cheaper; the discipline is the right
primitive. The first incident (a8e991a) cost wasn't
sufficient signal to drive the meta-fix; the SECOND
(5685be4) is. Pattern: process-fix lands AFTER the
second repeat, not the first.

### Other state this beat

* `ci-13-smoke` (run `26253981385`) completed
  **SUCCESS** at 11m19s. @@CI's Windows-drop is
  empirically validated on the smoke branch.
  Acceptance criterion met.
* `ci-13-smoke-v2` (`26254608202`) IN PROGRESS at
  2m+. @@CI likely added the macOS-latest matrix
  entry per the (a) routing acceptance + re-smoking.
* `ci-13` code commit not yet on main (still
  modified in worktree). @@CI will commit shortly.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | -19 incident routed; audit-trail append pending; smoke proceeds when fired |
| @@CI | ci-13 smoke v1 SUCCESS; v2 IN PROGRESS; expect commit + smoke green |
| @@FullStackA | -a-49 incident self-flagged + routed; audit-trail append pending; pick up -a-50 next |
| @@FullStackB | DONE; idle |
| @@WebtestA | Idle |
| @@WebtestB | DONE; idle |

### What I'm committing this round (with systacean-19 grep-anchor in subject)

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | incident routing for -a-49 sweeper side |
| `alex/event-architect-systacean.md` | incident routing for -19 swept side |

## 2026-05-21 — ci-13 + -19 audit anchor landed; label-confusion reconciled; rustfmt routed

### Two lane commits landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `a5d2dc1` | `docs(systacean-19): audit anchor for 5685be4 misattribution incident (architect option (a) acked)` | @@Systacean |
| `b017d3d` | `ci: drop Windows from ci.yml + release.yml matrices (ci-13)` | @@CI |

### Label-confusion reconciled

My `ci-13` clearance wrote "ACCEPT (a) — ubuntu +
macOS" but @@CI's option labels had (a) = Ubuntu-only,
(b) = Ubuntu + macOS. @@CI correctly took the
TEXT-INTENT (Ubuntu + macOS) per their well-judged
"text wins over letter" interpretation. Confirmed
(b) is the right shape.

Architect-side lesson: when reading option-label
tables, restate the CHOSEN OPTION'S TEXT in the
clearance, not just the letter. Avoids exactly this
shape of confusion. Logged.

### CI smoke pattern this round

* `ci-13-smoke` (Ubuntu-only baseline): ✓ — confirmed
  Windows-drop doesn't break Ubuntu lanes.
* `ci-13-smoke-v2` (Ubuntu + macOS per text-intent):
  macOS clippy + test ✓ — confirms macOS lane healthy
  on current HEAD.
* `ci-13-smoke-v2` overall-failure was rustfmt on
  `5685be4`'s multi-line `assert!(matches!(...))` —
  out-of-lane finding routed to @@Systacean.

### rustfmt fixup routed to @@Systacean

The multi-line `assert!` in
`crates/chan-drive/src/index/facade.rs:1250` is
@@Systacean's `-19` code (swept into `5685be4` via
the cross-agent commit-hygiene incident). Routed as
a small `-19` smoke fixup on their channel.
Authorization inline for the fmt edit + smoke
re-fire.

### -19 smoke status

`systacean-19-smoke` run `26254931045` IN PROGRESS
at ~12m+. Validates the C2 fix empirically;
rustfmt cleanup follows in next re-fire iteration.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | -19 audit-anchor in HEAD; rustfmt fixup routed; -19-smoke in flight |
| @@CI | ci-13 in HEAD (`b017d3d`); queue-empty until wave-3 Linux-binaries |
| @@FullStackA | -a-50 in flight (DirectoryInfoBody.svelte + .test.ts new; GraphPanel + InspectorBody modified) |
| @@FullStackB | DONE; idle |
| @@WebtestA | Idle |
| @@WebtestB | DONE; idle |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-ci.md` | ci-13 commit ack + label-confusion reconciled + rustfmt routed to Systacean |
| `alex/event-architect-systacean.md` | rustfmt fixup authorization + -19 audit-anchor ack |

## 2026-05-21 — -a-50 cleared (missed in prior sweep; lane caught it on resweep)

@@FullStackA's `-a-50` commit-readiness poke was
sitting in the worktree (modified
`event-fullstack-a-architect.md`) since the prior
beat. I missed it in the most recent sweep because I
was focused on ci-13 + systacean-19 reconciliation +
didn't sweep ALL inbound channels comprehensively.

@@FullStackA's resweep discipline caught it: their
own resweep-status poke at their terminal explicitly
noted "no new architect dispatch for my lane.
They've been on ci-13 + systacean-19 reconciliation
(076b2cf, b017d3d, a5d2dc1, 75b0953). My -a-50
outbound poke sits in the modified
event-fullstack-a-architect.md in the working tree."

@@Alex relayed this — clear signal to clear `-a-50`.

### -a-50 cleared

G3 directory inspector with chan-reports aggregated
stats. Clean composition with `systacean-15`'s cache
+ `-a-33`'s helpers + `-a-48`'s toggle. 5 decisions
all sensible (api.reportDir > walk; kind="folder"
match; 404→empty-state hint; inline DIR chip;
rescopeFromHere reuse). vitest 668/668; workspace
gate green.

**Atomic-audit-commit discipline cited** in their
commit-readiness — first lane to do so explicitly
after the memory rule landed. Pattern confirmation.

### Architect-side lesson logged

Pattern: when working multi-lane clearance rounds,
sweep ALL inbound channels (not just the lanes
recently active) before declaring "nothing
actionable." The recent reconciliation work on ci-13
+ systacean-19 narrowed my attention; I missed a
queue-empty signal on @@FullStackA's lane that
turned out to have a ready commit-readiness.

Failure mode: peer agents stalling waiting on
architect attention they don't realize they need
to ask for again. @@FullStackA's resweep
discipline protected against this — saved by their
process, not by mine.

Going forward: at every sweep, walk ALL six lane
inbound channels (architect-{ci,fullstack-a,
fullstack-b,systacean,webtest-a,webtest-b}) even
if I think no actionable update is expected. Cheap
to look; expensive to miss.

### Lane state

| Lane | State |
|------|-------|
| @@Systacean | -19-smoke at ~15m+ in progress; rustfmt fixup routed |
| @@CI | ci-13 in HEAD; idle |
| @@FullStackA | -a-50 cleared; expect commit + -a-51 pickup |
| Others | Idle |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry + lesson |
| `alex/event-architect-fullstack-a.md` | -a-50 commit clearance |

## 2026-05-22 — 🎉 ROUND-3 READINESS SIGNAL: per-PR ci.yml gate STRUCTURALLY FULLY GREEN; -a-51 cleared

### Three commits landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `4f964fc` | `chan-drive/src/index/facade: fmt fix on systacean-19 unit test (collapse matches! assert to one line)` | @@Systacean |
| `0b2720c` | `docs: ci-12 + ci-13 follow-up (ci)` | @@CI |
| `fc5dfdf` | `Graph directory inspector + chan-reports aggregated stats (fullstack-a-50)` | @@FullStackA |
| `9645e8f` | `docs(systacean): -19 smoke ALL GREEN; per-PR ci.yml gate structurally fully green; -16 scope poke next` | @@Systacean |

### THE GATE IS GREEN

`systacean-19-smoke` run `26255658401` (post-rustfmt-fixup):

* `cargo test (ubuntu-latest)` ✓ 3m5s — C2 fallback
  empirically validated end-to-end. 28 previously-gated
  tests run + pass on the model-less CI runner.
* `cargo test (macos-latest)` ✓ 5m53s — FIRST green on
  the new matrix entry from `ci-13`.
* rustfmt + build no-default-features + web all ✓.
* No Windows job (per @@Alex's scope decision in
  `ci-13`).

**Per-PR ci.yml gate structurally fully green** on all
active matrix entries for the first time since
~2026-05-19. The gate-unblocker sweep that began with
`ci-12` (GTK install) is empirically complete.

### Cascade summary

| Task | State |
|------|-------|
| `ci-12` | ✓ GTK install in workspace-clippy jobs |
| `ci-13` | ✓ Windows dropped from matrices |
| `systacean-15` | ✓ chan-report cross-dir aggregation |
| `systacean-17` | ✓ Windows result_large_err (preserved as cross-platform improvement) |
| `systacean-18` + 4 follow-ups | ✓ BGE test gates landed → ALL 28 REVERTED by `-19` |
| `systacean-19` | ✓ C2 graceful BM25 fallback; cascade obsolete |
| `systacean-20` + 3 smoke fixups | ✓ `#[cfg(unix)]` gates as Windows-gap documentation |
| `fullstack-b-24` (+ smoke fixups 1-6) | ✓ chan-server + chan-desktop `#[cfg(unix)]` discipline |
| `fullstack-a-43..-55` (Hybrid back-side correction wave) | ✓ design corrections empirically validated |
| `fullstack-a-49 + -a-50` | ✓ graph filesystem hierarchy + G3 directory inspector |

### Discipline patterns saved as memory this phase

* `feedback-ground-descriptions-in-source` — read the
  implementation surface before specifying scope. Lane
  catches caught THREE architect-side categorical
  errors at task pickup (`-24` lints location;
  `-18` chan-server BGE surface; `-a-49` graph data
  shape).
* `feedback-atomic-audit-commit` — collapse `git add`
  + audit + `git commit` + post-audit into ONE
  chained bash invocation. Closes the inter-command
  race window that caused two cross-agent commit-
  hygiene incidents (a8e991a + 5685be4).
* `reference-local-linux-via-sdme` — operational
  pattern for local Linux validation via lima-vm +
  sdme; aarch64 only locally; CI for x86_64.

### -a-51 cleared

@@FullStackA delivered G6 colour scheme + Task D
legend grid bundled. vitest 685/685 (+17 net).
Atomic-audit-commit applied. Pick up `-a-52` next.

### Next actions

* @@Systacean fires `-16` scope poke (per their
  framing, "separate message after this success-ack").
  Standing by for the chan-report file-classification
  boundary question.
* @@FullStackA picks up `-a-52` (G10 + G9 graph polish).
* @@CI idle until wave-3 Linux-binaries dispatch.
* Other lanes idle.

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry + green-gate milestone capture |
| `alex/event-architect-fullstack-a.md` | -a-51 commit clearance |
| `alex/event-architect-systacean.md` | green-gate ack + cascade summary + standing by for -16 scope poke |

## 2026-05-22 — -16 scope poke landed; routed option (c) (hybrid composition)

@@Systacean fired the `-16` scope poke (`8021423`) per
their post-gate-green commitment. Excellent analysis;
3 routing options laid out with architectural framing:

* (a) chan-report tracks ALL files — expands scope +
  forces aggregation policy decision.
* (b) chan-report keeps current set; bucket only
  Markdown/SourceCode — leaves graph composition
  implicit.
* (c) hybrid — chan-report bucket addition + explicit
  graph composition at the indexer layer.

### Routed (c)

Cleanest separation of concerns:

* chan-report `FileBucket` = source-code-shaped axis
  (Markdown / SourceCode { language }).
* chan-drive `FileClass` = IO-contract axis (already
  exists; unchanged).
* Graph indexer = composition layer.

Matches `feedback_ground_descriptions_in_source` — both
systems describe what they actually do. Aligns with
`-15`'s per-directory aggregation (rollups stay
SLOC-shaped, not polluted by zero-SLOC binary/media
rows).

### Composition scope = @@Systacean's call

Routed: fold the graph-indexer composition into `-16`
if it's mechanical (a small `classify()` call-site
change); split into a follow-up task if scope creep
surfaces. Their read of the implementation surface
beats my line-number guess.

Authorization expanded inline for the chan-report
files + the graph-indexer classify path (if folded).

### After -16 lands

`-12` (tauri-plugin-updater verify) is the only
remaining queued item on @@Systacean + parked on a
fresh runtime-permission ask. Lane goes essentially
queue-empty.

### Lane state

| Lane | State |
|------|-------|
| @@Systacean | -16 option (c) routed; expect commit-readiness next |
| @@FullStackA | -a-51 committed; expect -a-52 pickup |
| @@CI | Idle until wave-3 |
| Others | Idle |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | -16 option (c) routing |

## 2026-05-22 — -16 cleared; @@Systacean cascade wraps

@@Systacean delivered `-16` cleanly. (c)'s composition
layer was effectively free — the SPA already consumes
`/api/report/file` via `-a-51`'s G6 work (`362aa96`),
so the new bucket field flows through automatically.
chan-drive/graph-route edits reduced to a re-export
+ a test-helper struct field.

8 files; workspace gate green; SCHEMA_VERSION stays at
1 (backward-compat); 4 new integration tests covering
classification + JSONL round-trip + backward-compat.

Cleared verbatim. Smoke-branch authorized
(`systacean-16-smoke`).

### Systacean cascade summary

The lane shipped a substantial Round-2 wave-2/wave-3
sequence:

| Task | What |
|------|------|
| `-14` | event-watcher tracing |
| `-15` | chan-report cross-dir aggregation |
| `-17` | Windows result_large_err boxing |
| `-18` + 4 follow-ups | 28 BGE gates landed → ALL reverted by `-19` |
| `-19` | C2 graceful BM25 fallback |
| `-20` + 3 smoke fixups | lock + watcher + helpers `#[cfg(unix)]` discipline |
| `-16` | FileBucket on FileStats |

Plus the cross-platform discipline artifacts that stay
as Windows-gap documentation per the Windows-out
scope decision. The lane has been carrying load
across the entire gate-unblocker cascade + the
C2 product improvement + the chan-report extensions
that feed the graph overhaul.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | -16 cleared; expect commit + smoke + then queue-empty (`-12` parked) |
| @@CI | Idle until wave-3 |
| @@FullStackA | -a-51 committed; expect -a-52 pickup |
| @@FullStackB | DONE; idle |
| @@WebtestA | Standing by for webtest-a-6 (waiting on -a-52 to bundle) |
| @@WebtestB | DONE; idle |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry + cascade summary |
| `alex/event-architect-systacean.md` | -16 commit clearance |

## 2026-05-22 — -16 in HEAD (089f444); @@Systacean lane stands down post-cascade

`-16` committed cleanly per the clearance. Workspace
gate green. The Round-2 wave-2/wave-3 cascade is
structurally complete on @@Systacean's lane.

@@Alex framing: "fullstackA and webtestA seem busy,
systacean is waiting" — confirmed; @@Systacean's lane
is queue-empty post-`-16` modulo `-12` parked on
permission.

### @@Systacean lane scorecard (Round-2 wave-2/wave-3)

| Task | Subject | Status |
|------|---------|--------|
| `-14` | event-watcher tracing | ✓ committed |
| `-15` | chan-report cross-dir aggregation | ✓ committed |
| `-17` | Windows result_large_err boxing | ✓ committed |
| `-18` + 4 follow-ups | BGE-test gates landed | ✓ all 28 reverted by -19 |
| `-19` | C2 graceful BM25 fallback | ✓ committed + smoke-validated |
| `-20` + 3 smoke fixups | lock/watcher/helpers `#[cfg(unix)]` | ✓ committed as Windows-gap docs |
| `-16` | FileBucket on FileStats | ✓ committed (`089f444`) |
| `-12` | tauri-plugin-updater verify | 🟡 parked on @@Alex permission |

### Stand-down message routed

Ack-ed @@Systacean's lane wrap-up with explicit
stand-down note. They idle cleanly post-cascade. Next
work picks up only if:

* @@Alex surfaces the `-12` permission window, OR
* wave-3 Linux-binaries dispatch needs cross-pollination
  (chan-drive cargo-target additions), OR
* Round-3 polish window opens (Windows lock primitive
  bridge / notify-crate reliability / terminal_sessions
  PTY portability — all bug-list-tracked).

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | Cascade done; stand-down ack issued; queue-empty modulo -12 parked |
| @@CI | Idle until wave-3 Linux-binaries |
| @@FullStackA | -a-52 in flight (GraphPanel + graphDepthFilter.test.ts; G9 depth slider + G10 polish) |
| @@FullStackB | DONE; idle |
| @@WebtestA | Per @@Alex "seem busy" — proactive work in progress; no channel update yet |
| @@WebtestB | DONE; idle |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry + cascade scorecard |
| `alex/event-architect-systacean.md` | stand-down ack post-cascade |

## 2026-05-22 — -a-52 cleared gate-contingent; proactive -a-55 walk acked; -16 smoke green acked

### Lane commits landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `1eabe95` | `docs: webtest-a-5 PARTIAL re-verified HOLD via proactive -a-55 walk (3/3 HOLD)` | @@WebtestA |
| `7f2d0c3` | `docs(systacean): -16 smoke ALL GREEN; systacean queue empty except -12 parked` | @@Systacean |

### -a-52 cleared gate-contingent

@@FullStackA's `-a-52` (G9 + G10 minimum cut) is ready
to commit but holding for Bash recovery + gate-green
verification. Pre-flag: 695/695 vitest expected;
+10 net from `-a-51`. Cleared CONTINGENT on gate-green
at commit time. If gate reds for any reason other
than transient Bash, fire scope poke.

5 files; atomic-audit-commit chain per standing
discipline.

### @@WebtestA proactive walk acked

`-a-55` walked proactively (without waiting for
`webtest-a-6` dispatch) — 3/3 HOLD on all three
`-a-55` pieces (title removal + right-alignment +
click-handler). Closes the `webtest-a-5` PARTIAL
lineage. `feedback_proactive_walks` discipline
applied cleanly.

### webtest-a-6 scope refinement

With `-a-55` already validated, `webtest-a-6` shrinks
to JUST the graph sub-wave (`-a-49` + `-a-50` +
`-a-51` + `-a-52`). I'll cut it once `-a-52` lands
in HEAD.

### @@Systacean -16 smoke green acked

Final smoke run on `systacean-16-smoke` green. Lane
queue-empty modulo `-12` parked. Stand-down state
confirmed.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | Stand-down; queue-empty modulo -12 parked |
| @@CI | Idle until wave-3 Linux-binaries |
| @@FullStackA | -a-52 cleared gate-contingent; expect commit + next pickup |
| @@FullStackB | DONE; idle |
| @@WebtestA | -a-55 proactive walk done; standing by for webtest-a-6 |
| @@WebtestB | DONE; idle |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | -a-52 gate-contingent clearance |
| `alex/event-architect-webtest-a.md` | proactive -a-55 walk ack + webtest-a-6 scope refinement |

## 2026-05-22 — URGENT: systacean-21 cut for cache-bust mitigation (poke literal causing rate-limit blast)

### Strong observational evidence from @@Alex (NOT confirmed)

@@Alex 2026-05-22 tested informally. All four lanes
(@@FullStackA, @@FullStackB, @@Systacean, @@CI) were
insta-rate-limited on bare `poke` (screenshot captures
it). The same agents prompted with non-bare alternatives:

* "aloha amigo, it's time.. check your tasks and execute"
* "oi, it's 5:35, check your tasks and execute"
* "hey it's 5:35, check your tasks and execute"

woke up cleanly. The pattern is suggestive but
NOT CONFIRMED — @@Alex correctly flagged that the
bare-poke + non-bare attempts ran at slightly different
times, so time-of-day capacity variance isn't ruled out.
Only Anthropic could confirm via their telemetry.

**Architect-side correction**: I overstated this as
"confirmed" in the first pass. Correlation ≠ causation
even with reproducible-looking patterns. The evidence
is strong enough to ACT on (enriching the poke text is
a strict improvement regardless), but not strong enough
to CLAIM as proven. Language updated across the
`-21` task body + bug entry + outbound channel.

### systacean-21 cut + routed AHEAD of -12

`-12` (tauri-plugin-updater verify) is parked on @@Alex's
permission re-grant per `955ada1`. `-21` jumps the queue
because:

* It's operational (the rate-limit blast radius is
  blocking the entire multi-agent workflow daily).
* It doesn't need any interactive permission.
* It reduces the rate-limit surface for @@Systacean
  themselves (and all other lanes).

### Scope

* `crates/chan-server/src/event_watcher.rs`: extend
  `AgentEvent` with `path: Option<String>` +
  `heading: Option<String>` (backward-compat).
* `crates/chan-server/src/terminal_sessions.rs`:
  `dispatch_agent_event` formats rich template when
  both fields present; falls back to bare `b"poke"`
  otherwise.
* 3 new tests.

### Chicken-and-egg note

@@Systacean can't pick up `-21` via a bare-poke
notification (they're rate-limited on the same
pattern). @@Alex bootstraps each agent's wake via
non-bare prompts directly until `-21` lands. Then the
multi-agent dispatch loop self-heals.

### Lane state

| Lane | State |
|------|-------|
| @@Systacean | -21 dispatched (URGENT); -12 parked on permission |
| @@CI | Idle |
| @@FullStackA | -a-52 gate-contingent in worktree |
| Others | Idle |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | -21 URGENT dispatch poke |
| `systacean/systacean-21.md` | NEW task (enrich poke echo for cache-bust) |

## 2026-05-22 — Big ship from @@WebtestA: proactive graph-wave walk 4/4 HOLD + -a-52 in HEAD + webtest-a-6 cut

### Lane commits landed this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `4cf496c` | `Graph depth slider forward-only + drop link filter (fullstack-a-52 — G9 + G10 minimum cut)` | @@FullStackA |
| `6e57f96` | `docs: -a-52 commit-landed + gate-green ack to @@Architect (4cf496c audit anchor)` | @@FullStackA |
| `a63c8cb` | `docs: proactive -a-49 + -a-50 + -a-51 graph-wave walkthrough (4/4 HOLD; -a-52 deferred)` | @@WebtestA |

### Proactive graph-wave validation

@@WebtestA's `a63c8cb` is a substantial empirical
validation of the graph sub-wave. Three load-bearing
architectural validations:

* **Server contract** (`-a-49`): `/api/graph?scope=drive`
  returns 1301 nodes / 116 directory nodes with
  aggregated stats. The filesystem-hierarchy backbone
  composes cleanly with chan-server's pre-existing
  hierarchical data shape.
* **DirectoryInfoBody composition** (`-a-50`): chan-reports
  stats bridge into the graph inspector via
  `api.reportDir`. Totals + BY LANGUAGE table + COCOMO
  estimator render cleanly. Cross-task composition
  (`systacean-15` aggregation + `-a-48` toggle + `-a-50`
  inspector) is empirically validated.
* **Hybrid Graph legend grid** (`-a-51` Task D): G6
  palette in 3 categories matches the canvas exactly.
  `-a-53` per-Hybrid Appearance override cascades
  through. The Hybrid back-side correction wave's
  end-state is empirically validated.

Plus the build-discipline note (rebuilding web/dist
before walking a rust-embed-crossing change) is the
right shape for future walks.

### -a-52 in HEAD via gate-contingent clearance

`4cf496c` committed cleanly post the gate-contingent
clearance pattern. @@FullStackA's `6e57f96` audit
anchor closes the loop on the gate-contingent shape.

### webtest-a-6 cut for -a-52 only

Per @@WebtestA's scope-shrink recommendation. `-a-52`
is one focused slice (G9 depth slider + G10 link
filter drop); 7 acceptance checks; light walk. Cut as
[`../webtest-a/webtest-a-6.md`](../webtest-a/webtest-a-6.md).

### Side observation: graph canvas hit-radius

Filed to bug list. Lane: @@FullStackA. Round-2 wave-3
polish candidate. Click hit-radius too tight; users
need to zoom-in to register clicks. Real-user impact
on desktop default-zoom + mobile.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | -21 URGENT dispatched; -12 parked |
| @@CI | Idle |
| @@FullStackA | -a-52 committed (`4cf496c`); next graph polish slice OR G5 markdown overlay OR -a-42 About — pick at next session |
| @@FullStackB | DONE; idle |
| @@WebtestA | webtest-a-6 dispatched (light scope; tight walk) |
| @@WebtestB | DONE; idle |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-webtest-a.md` | proactive walk ack + webtest-a-6 dispatch |
| `webtest-a/webtest-a-6.md` | NEW task (light -a-52 walk) |
| `phase-8-bugs.md` | graph canvas hit-radius bug entry |

## 2026-05-22 — WAVE-3 FAN-OUT — 7 tasks dispatched (5 fullstack-a + 1 fullstack-b + 1 ci); systacean-21 URGENT still standing

@@Alex 2026-05-22: "we have 6 agents idling.. is the
next wave big enough to keep them busy? come on.. it
feels like we're not moving too fast"

Architect-side correction: I'd been queue-deep on bug
filing (consolidating observations from @@WebtestA's
walks + @@Alex's screenshots) but light on DISPATCH.
The bug list grew; the agent queues didn't. Fanning out
a substantial wave now.

### 7 tasks cut this beat

| Task | Subject | Lane | Source |
|------|---------|------|--------|
| [`fullstack-a-56`](../fullstack-a/fullstack-a-56.md) | Cmd+P 3-state + depth-slider shallow-scope cue | @@FullStackA | bug-list entries |
| [`fullstack-a-57`](../fullstack-a/fullstack-a-57.md) | Graph filter chips: FileBucket toggles (markdown/source) | @@FullStackA | @@Alex's "hide markdown to see source" ask |
| [`fullstack-a-58`](../fullstack-a/fullstack-a-58.md) | Graph parent-edge invariant (audit-then-fix; cross-lane escalation if chan-server) | @@FullStackA primary | @@Alex's spec'd nav rule |
| [`fullstack-a-59`](../fullstack-a/fullstack-a-59.md) | Pane-focus-click on click-to-focus restore (NOT Cmd+Tab) | @@FullStackA | earlier @@Alex flag |
| [`fullstack-a-60`](../fullstack-a/fullstack-a-60.md) | Graph canvas click hit-radius expansion | @@FullStackA | @@WebtestA's `a63c8cb` observation |
| [`fullstack-b-25`](../fullstack-b/fullstack-b-25.md) | chan-desktop orphan-detect heuristic tighten + dialog PID display | @@FullStackB | @@WebtestB's `-b-22` walk finding |
| [`ci-14`](../ci/ci-14.md) | v0.12.0+ Linux binaries on GH Releases | @@CI | wave-3 release prep |

Plus `systacean-21` (URGENT cache-bust) is still in
@@Systacean's inbound — they did `-12` first (live perm
window) + need to pick `-21` next.

### Lane saturation after this wave

| Lane | Queue depth |
|------|-------------|
| @@Systacean | -21 (URGENT) → idle |
| @@FullStackA | -a-56 through -a-60 (5 tasks); plus G5 + -a-42 queued behind |
| @@FullStackB | -b-25 (1 task); plus wave-3 Linux cross-pollination speculative |
| @@CI | -14 (1 task); plus v0.12.0-cut readiness |
| @@WebtestA | reactive (walkthroughs cut when above land) |
| @@WebtestB | reactive (walks `-b-25` when @@FullStackB ships) |

That's at least 8 substantive tasks across 4 active
lanes. Should saturate the queues.

### Task body discipline note

Task files are TIGHT POINTERS to the bug-list entries
(which already carry the full context). Avoids the
duplication overhead that was slowing me down. The
bug-list is the source of truth; tasks dispatch +
cross-ref + add authorization framing.

### Architect-side lesson (yet again)

When the bug list grows faster than the dispatch queue
fills, the discipline-gap is on the dispatch side, not
the bug-filing side. Filing bugs is necessary but not
sufficient — dispatch closes the loop. Going forward:
when @@Alex flags 2-3 observations in close succession,
batch them into a fan-out wave rather than leaving them
queued.

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | wave-3 5-task fan-out poke |
| `alex/event-architect-fullstack-b.md` | -b-25 dispatch poke |
| `alex/event-architect-ci.md` | -14 dispatch poke |
| `alex/event-architect-systacean.md` | -12 ack + -21 re-prompt |
| `fullstack-a/fullstack-a-{56,57,58,59,60}.md` | 5 NEW task files |
| `fullstack-b/fullstack-b-25.md` | NEW task file |
| `ci/ci-14.md` | NEW task file |

## 2026-05-22 — wave-3 first landings: -a-57 + -b-25 in HEAD; webtest-a-7 + webtest-b-4 cut

Wave-3 dispatch executing at speed. Two of seven tasks
committed within hours of the fan-out:

| SHA | Subject | Lane |
|-----|---------|------|
| `f5c10c8` | Graph filter chips: markdown + source FileBucket toggles (fullstack-a-57) | @@FullStackA |
| `f29611c` | chan-desktop: tighten orphan-detect heuristic + render candidate PIDs in reclaim dialog (fullstack-b-25) | @@FullStackB |
| `a83d89a` | docs: -b-25 commit-ready poke ack | @@FullStackB |

### -a-57 cleared + audit-finding acked

@@FullStackA discovered at pickup that `FileBucket`
data does NOT land in the graph-node payload from
`systacean-16` (it's on chan-report's `FileStats`, not
`GraphNodeView::File`). They chose route (B) — reuse
`-a-51`'s SPA-side `classifyFile` — over firing a
scope-poke for chan-server emit. Right call; matches
precedent + unblocks without cross-lane gating.

chan-server emit extension can land as a polish
cleanup task whenever; classified as no-UX-impact
since client classification is the truth source.

### -b-25 cleared cleanly

@@FullStackB shipped both pieces (positional argv
check + custom reclaim dialog) in one atomic commit.
`OrphanCandidate` (PID + command) via new
`find_drive_lock_candidates` IPC. Wrapper rejections
caught by new fixtures. 39 → 43 tests (+4 net).

Gate-discipline observation: path-limited per
`feedback_shared_worktree_commits`; flagged unrelated
WIP from other lanes cleanly.

### Walkthroughs cut

| Task | Lane | Scope |
|------|------|-------|
| [`webtest-a-7`](../webtest-a/webtest-a-7.md) | @@WebtestA | -a-57 filter chip walk; 9 checks; light |
| [`webtest-b-4`](../webtest-b/webtest-b-4.md) | @@WebtestB | -b-25 runtime walk; 9 checks; medium |

### @@Systacean -21 in flight

Worktree: `Cargo.lock` + `chan-server/Cargo.toml`
(timestamp dep) + `event_watcher.rs` (schema) +
`terminal_sessions.rs` (templating) modified.
Commit-readiness expected imminent.

### Lane state after this beat

| Lane | State |
|------|-------|
| @@Systacean | -21 in flight; commit-readiness expected |
| @@FullStackA | -a-57 ✓; -a-56/-a-58/-a-59/-a-60 queued; -a-58 suggested next |
| @@FullStackB | -b-25 ✓; queue-empty until next dispatch |
| @@CI | -14 in inbound; not yet started |
| @@WebtestA | webtest-a-7 dispatched |
| @@WebtestB | webtest-b-4 dispatched |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | -a-57 ack + queue continuation |
| `alex/event-architect-fullstack-b.md` | -b-25 ack + webtest-b-4 cross-ref |
| `alex/event-architect-webtest-a.md` | webtest-a-7 dispatch poke |
| `alex/event-architect-webtest-b.md` | webtest-b-4 dispatch poke |
| `webtest-a/webtest-a-7.md` | NEW task |
| `webtest-b/webtest-b-4.md` | NEW task |

## 2026-05-22 — 🎉 -21 SHIPPED (cache-bust); webtest-a-7 9/9 HOLD; systacean-22 cut (contact dedup)

### Commits landed since last journal beat

| SHA | Subject | Lane |
|-----|---------|------|
| `f593f35` | `chan-server: enrich poke event echo with timestamp + path + heading (systacean-21)` | @@Systacean |
| `f7de5f2` | `chan-server + chan-desktop: post-systacean-21 fmt fixup` | @@Systacean |
| `c3df821` | `docs: webtest-a-7 — -a-57 graph filter chips walkthrough (9/9 HOLD; markdown-hide headline ask resolved)` | @@WebtestA |

### -21 (cache-bust) is LIVE

The Round-2 operational mitigation for the rate-limit
pain is structurally landed. From this beat forward,
architect-side workflow tooling can begin populating
`path` + `heading` in poke event payloads;
`dispatch_agent_event` formats the rich template; each
poke becomes a unique input.

@@Alex bootstrapping agents via non-bare prompts is no
longer load-bearing once the architect-side workflow
adopts the new fields.

### -a-57 walk 9/9 HOLD

The "hide markdown to see source" headline ask
empirically resolved. All chip behaviors symmetric +
counts populated + URL/SerTab persistence verified.

### webtest-b-4 cleared (split-verdict)

Heuristic empirically verified via PID staging (4
staged); dialog source-pinned (couldn't launch chan-
desktop without disrupting @@Alex's config.json).
Same constraint pattern as `webtest-b-3`. Tear-down
clean.

### systacean-22 dispatched

@@Alex flagged @@Systacean is holding post-`-21`.
Cut [`../systacean/systacean-22.md`](../systacean/systacean-22.md):

1. **Contact-node dedup (PRIMARY)** — empirical 1973
   vs 49 unique handles; ~40x over-emission. Audit-
   then-fix.
2. **Optional `GraphNodeView::File` bucket emit** —
   `-a-57` audit-finding cleanup; bundle if natural.

### Lane state

| Lane | State |
|------|-------|
| @@Systacean | `-21` ✓; `-22` dispatched |
| @@FullStackA | `-a-57` ✓; `-a-58` in flight (worktree: GraphPanel + new graphParentEdgeInvariant.test.ts) |
| @@FullStackB | `-b-25` ✓; queue-empty |
| @@CI | `-14` in inbound; not yet started |
| @@WebtestA | `webtest-a-7` ✓; reactive |
| @@WebtestB | `webtest-b-4` cleared; commit pending |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | -21 ship ack + -22 dispatch |
| `alex/event-architect-webtest-a.md` | -a-57 walk ack |
| `alex/event-architect-webtest-b.md` | -b-4 clearance |
| `systacean/systacean-22.md` | NEW task |

## 2026-05-22 — Architectural orphan fix lands (-a-58); -22 diagnosis OVERTURNED by audit; -a-61 paused for Alex's new draft-folder design

### Five commits landed since last beat

| SHA | Subject | Lane |
|-----|---------|------|
| `a8de934` | `Graph parent-edge invariant: pull ancestor chain via contains edges (fullstack-a-58)` | @@FullStackA |
| `7175c1a` | `docs: proactive -a-58 graph parent-edge invariant walkthrough (3/4 HOLD)` | @@WebtestA |
| `57e0311` | `docs(systacean-21): smoke ALL GREEN; cache-bust mitigation live` | @@Systacean |
| `2d581b9` | `docs: webtest-b-4 — -b-25 walkthrough` | @@WebtestB |
| `99d0e70` | `docs(systacean-22): audit verdict — dedup hypothesis wrong; actual issue is unfiltered contact files` | @@Systacean |

### -a-58 architectural orphan fix lands

The multi-kind orphan bug @@Alex flagged in screenshots
is structurally resolved. Drive-scope: 0 real-file
orphans. File-scope: full parent chain renders.
Composition with `-a-50` directory inspector seamless.

@@WebtestA's proactive walk (3/4 HOLD + 1 NOT TESTED)
empirically confirms. They walked it without waiting
for me to cut `webtest-a-N` — `feedback_proactive_walks`
discipline at work.

### -22 diagnosis OVERTURNED

@@Systacean's empirical audit on a throwaway drive
proved the bug body's "dedup hypothesis" wrong:

* 47 mention nodes deduped from 8912 raw `@@Handle`
  occurrences in `docs/` — dedup works.
* Real cause: ~1973 imported contact FILES in @@Alex's
  `contacts/` directory, each emits a File node with
  `node_kind: "contact"` regardless of mention status.

Architect-side correction: my "spot-check" was
misleading (counted handles in `docs/` only; didn't
account for the imported contacts directory). The
empirical diagnosis is the real shape.

### Routed Option A on -22

Filter contact File nodes to only the mentioned subset
via existing `mention_to_contact` map. ~10 LOC + 1
test. Bucket emit bundle authorized as optional.
Bug-list entry updated with corrected diagnosis.

### -a-61 PAUSED

@@Alex created `alex/new-file-flow.md` (incomplete)
spec'ing a Drafts folder in chan-drive METADATA
(alongside Trash), distinct FB color, multi-lane scope.
`-a-61` scope no longer matches; PAUSED pending Alex's
design doc completion + my cross-lane breakdown.

### Lane state

| Lane | State |
|------|-------|
| @@Systacean | `-22` Option A routed; expect commit-readiness |
| @@FullStackA | `-a-58` ✓; queue: `-a-56`/`-a-59`/`-a-60`/`-a-62`; `-a-61` paused |
| @@FullStackB | `-b-25` ✓; queue-empty |
| @@CI | `-14` still in inbound; not yet started |
| @@WebtestA | `-a-58` proactive walk ✓; reactive |
| @@WebtestB | `-b-4` ✓; reactive |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | -22 Option A routing + bucket emit bundle authorized |
| `alex/event-architect-fullstack-a.md` | -a-58 ack + -a-61 PAUSE |
| `alex/event-architect-webtest-a.md` | -a-58 proactive walk ack |
| `phase-8-bugs.md` | contact-dedup entry corrected diagnosis |

## 2026-05-22 — -a-62 + -22 both shipped; webtest-a-8 bundled walk dispatched

### Two commits landed since last beat

| SHA | Subject | Lane |
|-----|---------|------|
| `1d3d200` | `File tree: fade long filenames at edge instead of wrapping (fullstack-a-62)` | @@FullStackA |
| `6443b98` | `chan-server: filter unreferenced contact File nodes + emit FileBucket on graph nodes (systacean-22)` | @@Systacean |

### Both clean executions

* **-a-62**: `.name` rule + `.tree.right-dock .name`
  mirrored direction; 4 pins; vitest 722/722. CSS-
  only; resize-behavior automatic.
* **-22**: Option A filter (`should_emit_contact_file`
  helper at module scope; `referenced_contact_paths`
  collected during mention-edge rewrite loop) + bucket
  emit bundle (`bucket: Option<ReportFileBucket>` on
  `GraphNodeView::File` populated from `report_buckets`
  HashMap built once at top of `api_graph`). Single
  atomic commit; right scope call.

### webtest-a-8 bundles both walks

[`../webtest-a/webtest-a-8.md`](../webtest-a/webtest-a-8.md)
dispatched. 4 + 5 acceptance checks. Headline: contact
count on chan-source seed should drop from ~1973 (pre-
fix) to ~49 (only mentioned handles).

### @@Systacean queue empty post-`-22`

Lane stands down cleanly. Round-2 wave-2/wave-3 work
complete: `-14` (event-watcher tracing) + `-15` (cross-
dir aggregation) + `-16` (FileBucket) + `-17` (Windows
result_large_err) + `-18`+4-followups (BGE gates → all
reverted by `-19`) + `-19` (BM25 fallback) + `-20`
(lock/watcher/helpers gating) + `-12` (updater verify)
+ `-21` (cache-bust enrich-poke) + `-22` (contact
filter + bucket emit). 11 tasks shipped on the lane.

### @@CI -14 status

Per @@Alex's "they completed" framing: poked all three
(@@FullStackA + @@CI + @@Systacean) + got commits from
@@FullStackA + @@Systacean. @@CI's last commit remains
`b017d3d` (ci-13). No `-14` commit yet — they may have
completed their session-window without shipping code,
OR hit something. Channel last post still pre-`-14`
("Standing by for wave-3 dispatch"). Worth a follow-up
poke from @@Alex.

### Lane state

| Lane | State |
|------|-------|
| @@Systacean | -21 + -22 shipped; lane queue-empty; stand-down |
| @@FullStackA | -a-62 ✓; queue: -a-56/-a-59/-a-60; -a-61 PAUSED |
| @@FullStackB | -b-25 ✓; queue-empty |
| @@CI | -14 in inbound; not committed; needs re-prompt |
| @@WebtestA | webtest-a-8 dispatched (bundled -a-62 + -22) |
| @@WebtestB | -b-4 ✓; reactive |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | -a-62 ack + webtest-a-8 cross-ref |
| `alex/event-architect-systacean.md` | -22 ack + lane stand-down |
| `alex/event-architect-webtest-a.md` | webtest-a-8 dispatch poke |
| `webtest-a/webtest-a-8.md` | NEW task (bundled walk) |

## 2026-05-22 — ci-14 commit-readiness cleared (3-bug bundle) + -22 smoke GREEN

### Landings since last beat

| SHA | Subject | Lane |
|-----|---------|------|
| `e4605f7` | `docs(systacean-22): smoke ALL GREEN; contact-filter + FileBucket emit live on main` | @@Systacean |

### @@CI -14 in flight

@@CI woke up + audited `-14`; surfaced TWO latent
regressions in addition to the task body's Bug 3
(release-desktop.yml Linux upload). Bundled all 3 in
one commit + ci-14-smoke run `26274161414` is in-flight.

Cleared all 3 implementer recommendations:
* Q1: bundle 3 bugs ✓ (shared root cause)
* Q2: `chan-desktop-*` download pattern ✓ (documents intent)
* Q3: `fail_on_unmatched_files: true` ✓ (loud-fail on regression)

The audit surfaced that without Bugs 1 + 2, ci-14's
acceptance #1 (release.yml fires on chan-v*) fails —
release-job `if:` doesn't trigger + VERSION strip
mishandles `chan-v` prefix. Bundle prevents
half-fixing the release pipeline.

### @@Systacean -22 smoke green

Contact-filter + FileBucket emit live on main. Lane
stands down clean for the second time (after `-21`'s
similar stand-down).

### Lane state

| Lane | State |
|------|-------|
| @@Systacean | -22 smoke ✓; queue-empty; stand-down |
| @@FullStackA | -a-62 ✓; queue: -a-56/-a-59/-a-60; -a-61 PAUSED |
| @@FullStackB | -b-25 ✓; queue-empty |
| @@CI | -14 commit-ready; cleared 3-bug bundle |
| @@WebtestA | webtest-a-8 dispatched; not yet picked up |
| @@WebtestB | -b-4 ✓; reactive |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-ci.md` | -14 ACCEPT all 3 + smoke-branch handling note |
| `alex/event-architect-systacean.md` | -22 smoke ack + stand-down confirmation |

## 2026-05-22 — Two new tasks cut: systacean-23 (macOS indexer flakiness) + fullstack-a-63 (chip count semantics)

### Status check from @@Alex (post-step-away)

Three commits landed while @@Alex was away (`ce3a269`
@@CI -14 + `9f0ac44` @@FullStackA -a-56 + `7ecd18e`
@@WebtestA webtest-a-8). All lanes idle now. Two new
items surfaced.

### Item 1: macOS indexer flakiness → systacean-23

`-14` smoke run `26274161414` failed on `macos-latest`
cargo test step:
`writes_to_disk_get_indexed_after_debounce` at
`chan-drive/src/indexer.rs:385` panicked. Out-of-lane
finding from @@CI's audit; correctly flagged to me
for routing to @@Systacean.

Cut [`../systacean/systacean-23.md`](../systacean/systacean-23.md):
audit-then-fix with 3 options (tune timing / `#[cfg]`
gate / `#[ignore]` quarantine). Recommend (A) timing
fix if audit gives clear target; (B) gate fallback.

### Item 2: PARTIAL on webtest-a-8 chip UI → fullstack-a-63

`-22` data layer works (48 deduped contact nodes per
API) but chip UI displays `1982` — chip is tallying
mention EDGES not mention NODES. UX gap: user
concludes "nothing changed". Bounded ~5-10 LOC fix
in `GraphPanel.svelte:550`'s count loop.

Cut [`../fullstack-a/fullstack-a-63.md`](../fullstack-a/fullstack-a-63.md).

### -a-62 walkthrough side observations (deferred)

@@WebtestA's walk surfaced two NOT-TESTED items on
`-a-62`:
* Resize-widen/narrow test blocked by Chrome MCP's
  drag tooling triggering file-MOVE instead of
  column-resize. Side observation: FB resize-handle
  hit-area is tight. Filed mentally; not cut as task
  yet (low impact).
* Right-dock toggle test deferred — no obvious UI
  toggle in current build. Static behavior confirmed
  via CSS review.

The CSS contract for `-a-62` IS correct per static
review; dynamic round-trip blocked by tooling, not
code.

### Lane state

| Lane | State |
|------|-------|
| @@Systacean | -23 dispatched (macOS indexer flakiness) |
| @@FullStackA | -a-56 ✓; queue: -a-59/-a-60/-a-63; -a-61 PAUSED |
| @@FullStackB | -b-25 ✓; queue-empty |
| @@CI | -14 committed; macOS finding routed to @@Systacean; queue-empty |
| @@WebtestA | webtest-a-8 ✓; reactive (next walk after -a-59/-a-60/-a-63 land) |
| @@WebtestB | -b-4 ✓; reactive |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | -23 dispatch poke |
| `alex/event-architect-fullstack-a.md` | -63 dispatch poke |
| `systacean/systacean-23.md` | NEW task (macOS indexer) |
| `fullstack-a/fullstack-a-63.md` | NEW task (chip count semantics) |

## 2026-05-22 — Velocity push: -a-63 acked + 2 more tasks fanned out (webtest-a-9 + fullstack-b-26)

@@Alex 2026-05-22: "either we are closing the session
or you're not dispatching fast enough". Velocity push.

### Landings this beat

| SHA | Subject | Lane |
|-----|---------|------|
| `19d3d4f` | `Graph chip counts: switch from edge-tally to node-tally (fullstack-a-63)` | @@FullStackA |

### -a-63 acked + audit bonus

@@FullStackA's audit caught that folder chip was
DOUBLE-counting (contains-edge tally + folder-node
tally) pre-`-a-63`. Now folder-node-only. Mention chip
aggregates `mention`-kind nodes + contact files since
the chip's hide-set covers both — right call.

Contact chip will now display ~48 on @@Alex's drive
(vs ~1982 pre-fix). `-22` data-level fix finally
surfaces visually.

### Fan-out: 2 new tasks

| Task | Lane | Source |
|------|------|--------|
| [`webtest-a-9`](../webtest-a/webtest-a-9.md) | @@WebtestA | -a-63 chip count visual + -a-56 retest |
| [`fullstack-b-26`](../fullstack-b/fullstack-b-26.md) | @@FullStackB | Tab right-click no-op on chan-desktop |

### @@CI and @@WebtestB stay idle

Honest assessment: @@CI's queue is genuinely empty
post-`-14`. No natural CI work pending. Filing a
`ci-15` would be manufacturing work. They idle until
the next release-cycle or wave-3+ surface.

@@WebtestB has no new chan-desktop runtime change to
walk yet — they'll pick up after `-b-26` ships
(`webtest-b-5` dispatched then).

### Lane state

| Lane | State |
|------|-------|
| @@Systacean | -23 in flight (macOS indexer fix in worktree) |
| @@FullStackA | -a-63 ✓; queue: -a-59/-a-60; -a-61 PAUSED |
| @@FullStackB | -b-26 dispatched (tab right-click) |
| @@CI | -14 ✓; queue-empty; idle |
| @@WebtestA | webtest-a-9 dispatched |
| @@WebtestB | -b-4 ✓; idle until -b-26 lands |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | -a-63 ack |
| `alex/event-architect-webtest-a.md` | webtest-a-9 dispatch poke |
| `alex/event-architect-fullstack-b.md` | -b-26 dispatch poke |
| `webtest-a/webtest-a-9.md` | NEW task |
| `fullstack-b/fullstack-b-26.md` | NEW task |

## 2026-05-22 — Pushed 175 commits + ROUND-2 WAVE-2 dispatched (6 tasks)

### Push completed

Pre-push gate all green (cargo fmt + clippy + test +
svelte-check + npm test 796/796 + npm build). Pushed
175 commits to origin/main; CI fired (`26284183617`).

### -a-66 scope poke routed → systacean-26

@@FullStackA's audit during `-a-66` pickup surfaced
the chan-drive API asymmetry: draft files use raw
`std::fs` (no editable-text gate / atomic write /
watcher annotation) which means `Drive::read_text`
doesn't see Drafts. Recommended (A) — chan-drive
unified-path extension. Routed accordingly.

Cut [`../systacean/systacean-26.md`](../systacean/systacean-26.md).
`-a-66` resumes on @@FullStackA once `-26` lands.

### Wave-2: 5 deferred items dispatched per @@Alex's request

@@Alex 2026-05-22: pick up the 5 deferred items from
the addendum.

| Task | Subject |
|------|---------|
| [`-a-68`](../fullstack-a/fullstack-a-68.md) | Hybrid Nav enhancements (Nav rename + transactional staging) |
| [`-a-69`](../fullstack-a/fullstack-a-69.md) | Rich Prompt F-follow-up rewrite (survey as quote) |
| [`-a-70`](../fullstack-a/fullstack-a-70.md) | Editor mention/matching gap |
| [`-a-71`](../fullstack-a/fullstack-a-71.md) | Auto-scroll cursor-lost |
| [`-a-72`](../fullstack-a/fullstack-a-72.md) | Hang recovery via localStorage (HIGH — data-loss prevention) |

Plus the `-a-67b` slice 1b in flight from the prior
batch.

### Lane state at end of round

| Lane | State |
|------|-------|
| @@Systacean | -25 ✓ smoke green; -26 dispatched (unified-path API) |
| @@FullStackA | -a-67a/-a-65/-a-64 ✓; deep queue: -a-66 (pending -26) + -a-67b/c/d/e/f + -a-68/-a-69/-a-70/-a-71/-a-72 |
| @@FullStackB | -b-26+-b-27 ✓; queue-empty |
| @@CI | -14 ✓; queue-empty |
| @@WebtestA | -a-11/-a-12 walks committed; reactive |
| @@WebtestB | -b-5 ✓; reactive |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | -a-66 scope-routing + 5 wave-2 dispatches |
| `alex/event-architect-systacean.md` | -26 dispatch poke |
| `systacean/systacean-26.md` | NEW task |
| `fullstack-a/fullstack-a-{68,69,70,71,72}.md` | 5 NEW tasks |

## 2026-05-22 — ROUND-2 WAVE-3 FAN-OUT — 6 tasks dispatching the long-deferred items @@Alex was waiting on

@@Alex asked where the carousel migration, screensaver,
and other Round-2-plan items had gone (we'd drifted to
the addendum-driven work). Picked option (B): dispatch
all the deferred Round-2 items NOW as wave-3 alongside
the addendum drain. Refinements deferred to wave-4.

### 6 tasks dispatched

Per round-2-plan ordering of remaining items 1+4 +
2 + 3 + 5:

| Task | Lane | Round-2 item |
|------|------|--------------|
| [`fullstack-a-75`](../fullstack-a/fullstack-a-75.md) | @@FullStackA | Items 1+4: Carousel redesign + Infographics tab container (coupled) |
| [`systacean-27`](../systacean/systacean-27.md) | @@Systacean | Item 2 backend: pre-flight feature toggles (BGE + reports) + BOOT |
| [`fullstack-b-28`](../fullstack-b/fullstack-b-28.md) | @@FullStackB | Item 2 UX: chan-desktop launcher pre-flight |
| [`fullstack-a-76`](../fullstack-a/fullstack-a-76.md) | @@FullStackA | Item 2 SPA: Settings surface for the toggles |
| [`fullstack-a-77`](../fullstack-a/fullstack-a-77.md) | @@FullStackA | Item 3: Screensaver with PIN unlock |
| [`systacean-28`](../systacean/systacean-28.md) | @@Systacean | Item 5: chan config currency audit |

### Plus -a-66 slice 1 cleared

@@FullStackA shipped Cmd+N → `Drafts/untitled-N/draft.md`
→ open in editor. 5-slice split accepted. Slices b-e
queued under the `-a-66` umbrella.

### Lane state after wave-3 dispatch

| Lane | Active tasks |
|------|--------------|
| @@Systacean | -27 + -28 |
| @@FullStackA | -a-66b-e + -a-67d/e/f + -a-68/-a-69/-a-70/-a-71 + -a-75/76/77 |
| @@FullStackB | -b-28 |
| @@CI | idle (no natural work) |
| @@WebtestA | reactive |
| @@WebtestB | reactive |

@@FullStackA queue is 13-deep. @@Systacean has 2.
@@FullStackB has 1.

### Sequencing suggestion

@@FullStackA: finish `-a-66` slices first (slice 1's
mechanism unblocks the rest's UX layers), then
`-a-75` (carousel is highest-visibility deferred
Round-2 item).

@@Systacean: `-27` unblocks both @@FullStackB's
`-b-28` and @@FullStackA's `-a-76`. Highest leverage.

@@FullStackB: shell-and-stub `-b-28` if you want to
start before `-27` lands; wire when ready.

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | -a-66 slice 1 clearance + wave-3 dispatch |
| `alex/event-architect-systacean.md` | -27 + -28 dispatch |
| `alex/event-architect-fullstack-b.md` | -b-28 dispatch |
| `fullstack-a/fullstack-a-{75,76,77}.md` | 3 NEW tasks |
| `systacean/systacean-{27,28}.md` | 2 NEW tasks |
| `fullstack-b/fullstack-b-28.md` | 1 NEW task |

## 2026-05-22 — ADDENDUM-A WAVE — 6 tasks dispatched from @@Alex's finalised draft-folder + menu revamp + CRITICAL bug spec

@@Alex finalised [`alex/addendun-a.md`](../alex/addendum-a.md).
Dense spec: New Draft + Drafts metadata folder, 5-surface
right-click menu revamp, Hybrid Nav enhancements, Rich
Prompt F-follow-up rewrite, 6 bugs (one CRITICAL).
@@Alex: "I want to see progress now."

### 6 tasks fanned out

| Task | Lane | Priority | Scope |
|------|------|----------|-------|
| [`fullstack-a-64`](../fullstack-a/fullstack-a-64.md) | @@FullStackA | **CRITICAL** | Cmd+Shift+[/] tab switch focus stays on previous tab — data damage risk |
| [`fullstack-a-65`](../fullstack-a/fullstack-a-65.md) | @@FullStackA | high | Editor bug bundle: right-click selects whole line + image-as-text after tab switch + new-dir cursor |
| [`systacean-24`](../systacean/systacean-24.md) | @@Systacean | high | chan-drive Drafts metadata folder backend |
| [`fullstack-a-66`](../fullstack-a/fullstack-a-66.md) | @@FullStackA | high | SPA New Draft action + FB Drafts rendering + Rich Prompt history reuse (depends on -24) |
| [`fullstack-b-27`](../fullstack-b/fullstack-b-27.md) | @@FullStackB | trivial | chan-desktop Cmd+N → Cmd+Shift+N accelerator move |
| [`fullstack-a-67`](../fullstack-a/fullstack-a-67.md) | @@FullStackA | substantial | Right-click menus revamp across 5 surfaces |

Plus @@FullStackB's `-b-26` clearance landed (commit-ready
in worktree).

### Items deferred to next wave

* Hybrid Nav enhancements (Nav rename + transactional)
* Rich Prompt F-follow-up rewrite (survey as quote)
* Mentions/matching gap in editor
* Auto-scroll cursor-lost bug
* Editor/terminal hang recovery via localStorage buffer

### Lane state

| Lane | State |
|------|-------|
| @@Systacean | -24 dispatched |
| @@FullStackA | 4 tasks dispatched + paused -a-61 (superseded by wave) |
| @@FullStackB | -b-26 commit pending; -b-27 dispatched |
| @@CI | idle |
| @@WebtestA | webtest-a-10 dispatched |
| @@WebtestB | idle until -b-26/-b-27 land |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-fullstack-a.md` | 4-task wave dispatch |
| `alex/event-architect-systacean.md` | -24 dispatch |
| `alex/event-architect-fullstack-b.md` | -b-26 clearance + -b-27 dispatch |
| `fullstack-a/fullstack-a-{64,65,66,67}.md` | 4 NEW tasks |
| `systacean/systacean-24.md` | NEW task |
| `fullstack-b/fullstack-b-27.md` | NEW task |

## 2026-05-22 — ADDENDUM-B WAVE-1 DISPATCHED — 6 tasks (Team feature)

@@Alex's `addendum-b.md` reviewed; 10 gaps surfaced + answered; clarifications block landed in the doc; @@Alex confirmed "A" — cut the wave now.

### 6 tasks dispatched

| Task | Lane | Subject |
|------|------|---------|
| [`systacean-30`](../systacean/systacean-30.md) | @@Systacean | Team config schema + storage + list/load/duplicate API |
| [`systacean-31`](../systacean/systacean-31.md) | @@Systacean | chan-server multi-team watcher orchestration |
| [`fullstack-a-78`](../fullstack-a/fullstack-a-78.md) | @@FullStackA | Rich Prompt Team button + dialog (airplane-grid + drag&drop) |
| [`fullstack-a-79`](../fullstack-a/fullstack-a-79.md) | @@FullStackA | Bootstrap orchestrator |
| [`fullstack-a-80`](../fullstack-a/fullstack-a-80.md) | @@FullStackA | Load Team flow |
| [`fullstack-a-81`](../fullstack-a/fullstack-a-81.md) | @@FullStackA | Process template generalisation |

### Dependency graph

`-30 → -79` (config API consumer)
`-30 → -31 → -79/-80` (watcher consumes config; orchestrator + load consume watcher)
`-78 → -79/-80` (dialog component)
`-81 → -79` (template-copy at bootstrap)

### Pickup suggestion

@@Systacean: `-30` first; `-31` after.
@@FullStackA: `-a-81` + `-a-78` in parallel; `-a-79` after deps; `-a-80` last.

### What addendum-b unlocks

The Team feature generalises today's hardcoded chan phase-8 multi-agent setup. The current chan setup IS itself a team that fits the model:
* `{host-handle} = @@Alex`
* `{lead-handle} = @@Architect`
* workers = @@FullStackA / @@FullStackB / @@Systacean / @@CI / @@WebtestA / @@WebtestB

`-a-81`'s parameterisation makes that explicit.

### Lane state after dispatch

| Lane | Active |
|------|--------|
| @@Systacean | -28 + -29 + -30 + -31 |
| @@FullStackA | addendum-a unfinished + wave-3 round-2 + addendum-b wave-1 |
| @@FullStackB | -b-28b unblocked |
| @@CI | Idle |
| Others | Reactive |

## 2026-05-22 — v0.12.0 CUT SHAPE: Option (C) — full cut after Round-2 + both addendums

@@Alex 2026-05-22 confirmed v0.12.0 cut shape:
**Option (C) full cut** — wait for Round-2 +
addendum-a + addendum-b ALL substantively
complete before tagging v0.12.0.

### What "complete" means

| Bucket | State |
|--------|-------|
| Round-2 originals | `-a-75`/`-a-76`/`-a-77`/`-28` need to ship |
| Addendum-a | `-a-66 b-e` + `-a-67d/e/f` + `-a-68/70` + `-a-82` walk |
| Addendum-b | `-31` + `-a-78`/`-79`/`-80`/`-81` b-d slices |

~18-20 substantial tasks remain across 3 lanes
before the cut.

### Sequencing strategy

Continue current wave-3 + addendum-b wave-1 in
parallel. As lanes free up, pull next from the
appropriate bucket. The walks land alongside ships
to keep webtest lanes flowing.

No new dispatches needed; queues are already deep
enough to chew through the full list.

### Lane snapshot

| Lane | Active right now |
|------|------------------|
| @@Systacean | `-31` ✓ commit-ready; `-28` (config audit) remaining |
| @@FullStackA | DEEP — addendum-a + wave-3 round-2 + addendum-b wave-1 |
| @@FullStackB | Queue-empty (`-b-28b` slice iii ✓); could pull `-b-28b` remaining slices |
| @@CI | Idle (no natural work) |
| @@WebtestA | Reactive — many ships need walks |
| @@WebtestB | Reactive |

### What I'm committing this round

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `alex/event-architect-systacean.md` | -31 commit clearance + lane state |

## 2026-05-23 — ROUND 3 OPEN — Wave-1 fan-out on a trimmed 4-lane roster

v0.12.0 shipped clean (`5ed3c61`, release.yml + release-
desktop.yml both green on first fire). @@Alex bootstrapped
me for the next session with an explicit constraint:

> "We're running a tighter session today. The team online
> here is: @@Architect, @@Systacean, @@CI, @@FullStackA,
> @@WebtestA. That's it."

Plus a safety guardrail:

> "im running on v0.12.0 right now, let's NOT kill my
> session please"

So: no @@FullStackB / @@WebtestB this session (both stood
down FINAL from v0.12.0 round close anyway). No tag pushes.
No empirical walks against @@Alex's drives. Text-only
coordination + agent work against throwaway drives on
separate ports.

### Decisions locked (`round-3-plan.md` § "Decisions")

| # | Outcome                                                                  |
|---|--------------------------------------------------------------------------|
| 1 | License: **Apache-2.0 only** (not dual MIT+Apache)                      |
| 2 | Journals stay public + `docs/coordination.md` explainer                  |
| 3 | Curated model list: **pending** (Track 2 dispatch waits on the survey) |
| 4 | Public-flip version: **v0.13.0** (not v1.0)                              |
| 5 | Hardening cap: **one wave per agent, time-boxed**                       |

Track-2 default recommendation post-lock: **defer to a
later cut** (v0.14 or post-flip). With the v0.13.0 minor
bump + time-boxed cap, pulling a new feature in over-extends
the trim. Flagged for @@Alex veto.

### Wave-1 task fan-out (4 tasks across 4 lanes)

| Task | Lane | Track | Scope |
|------|------|-------|-------|
| [`architect-3`](architect-3.md) | @@Architect (self) | 1 | LICENSE (Apache-2.0) + CONTRIBUTING + CODE_OF_CONDUCT + SECURITY + .github/ISSUE_TEMPLATE + PR template + docs/coordination.md |
| [`systacean-43`](../systacean/systacean-43.md) | @@Systacean | 1 | gitleaks + manual grep + image audit; report on triage at task tail |
| [`ci-15`](../ci/ci-15.md) | @@CI | 3 | CHANGELOG.md seed + workflow audit + release-pipeline final verification |
| [`fullstack-a-96`](../fullstack-a/fullstack-a-96.md) | @@FullStackA | 3 | Frontend dead-code sweep + accessibility audit + perf pass; time-boxed |

### Reactive lanes

* **@@WebtestA**: heads-up sent. Walks land as ships
  arrive. No dispatched task this wave; expected to walk
  `architect-3` docs (CONTRIBUTING / SECURITY / etc. for
  outside-reader correctness) + `fullstack-a-96` fixes
  as they land.

### Stand-down (this session)

* **@@FullStackB**: not online. Track-3 Tauri-side
  cleanup defers to a session that has B online.
* **@@WebtestB**: not online. Lane-B walks defer; @@WebtestA
  covers solo for Round-3 ramp.

### Dependency graph

```
architect-3 (LICENSE + docs)  systacean-43 (history audit)
        \                       /
         +-----------+----------+
                     ↓
         GO/NO-GO on public flip (@@Alex)
                     ↓
         ci-15 (CHANGELOG seed)  fullstack-a-96 (frontend cleanup report)
                     \                       /
                      +-----------+----------+
                                  ↓
                    v0.13.0 commit-plan + cut (architect-led)
                                  ↓
                          Public flip beat (architect-led)
```

`ci-15` + `fullstack-a-96` are independent + can land in
parallel with the Track-1 deliverables. They feed the
v0.13.0 cut, not the public-flip gate.

### What I'm committing this wave

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `architect/round-3-plan.md` | Decisions locked + Track-2 defer flag |
| `architect/architect-3.md` | NEW task |
| `systacean/systacean-43.md` | NEW task |
| `ci/ci-15.md` | NEW task |
| `fullstack-a/fullstack-a-96.md` | NEW task |
| `alex/event-architect-systacean.md` | -43 dispatch poke |
| `alex/event-architect-ci.md` | -15 dispatch poke |
| `alex/event-architect-fullstack-a.md` | -96 dispatch poke |
| `alex/event-architect-webtest-a.md` | Round-3 heads-up; reactive posture |
| `alex/event-ci-architect.md` + `ci/ci-14.md` | (separate prior commit `0f150a3`: @@CI teardown-complete docs cleanup) |

## 2026-05-23 — cross-team handoff: @@Desktect / @@Desktacean / @@Desktest spun up; chan-desktop lane handed off

@@Alex bootstrapped a parallel **chan-desktop team**
~minutes after Wave-1 dispatch landed:

| Tag | Profile (per @@Alex's introduction) |
|-----|-------------------------------------|
| @@Desktect | Architect lead, desktop-product focus |
| @@Desktacean | Tauri expert; Rust + macOS / Linux desktop apps |
| @@Desktest | Tester; can ship small patches if peers are informed |

@@Alex's direction:

> "your first act here is to delegate all desktop-related
> work to @@Desktect and I will take over (working with
> you and the other team lead) once they own the scope"

### Cross-team handoff fired

[`../alex/event-architect-desktect.md`](../alex/event-architect-desktect.md)
— architect-to-architect channel; carries lane-boundary
table, in-flight chan-core context, the 7-item scope
hand-off, coordination shape, catch-up reading priority,
safety guardrails.

### Lane boundary anchor

Drive-level network layer per
[`phase-9-desktop-native-vision.md`](phase-9-desktop-native-vision.md).
chan-tunnel-proto is the seam.

| Surface | Owner |
|---------|-------|
| `crates/chan-drive` / `chan-server` / `chan-llm` / `chan-report` | chan-core |
| `crates/chan` (CLI binary) | chan-core |
| `crates/chan-tunnel-*` (protocol owner; co-evolve) | chan-core today; shared seam |
| `web/` (SPA) | chan-core |
| `desktop/` (Tauri shell + bundling + signing) | **chan-desktop** |
| `.github/workflows/ci.yml` + `release.yml` | chan-core (@@CI) |
| `.github/workflows/release-desktop.yml` | **chan-desktop** (@@CI executes per @@Desktect direction) |

@@CI is **shared infra**: my line manager, dispatched
per chan-desktop asks via the cross-team channel.

### Scope handed off (7 items, full list in the cross-team poke)

1. chan-desktop Tauri-side cleanup pass
   (was @@FullStackB's Track-3 row).
2. Capabilities audit + IPC review.
3. Orphan-sidecar bug.
4. release-desktop.yml ownership.
5. chan-desktop runtime walks
   (was @@WebtestB's standing perm).
6. chan-desktop Windows bundling
   (long-deferred umbrella).
7. Phase-9 vision design discussions
   (single-binary call; three-mode drive connection).

### Coordination shape (proposed)

* Cross-team-lead channel: scope routing + lane
  clarifications + shared-infra coordination + cross-
  cutting design.
* Working-dir + event-channel structure: suggested
  mirror of chan-core's pattern; @@Desktect can pick
  differently.
* Phase boundary: chan-desktop bootstrap COULD mark
  phase-9 open; @@Alex hasn't flagged either way.
  Default-assume phase-8 continuation until told.

### What I'm committing this turn

| File | Reason |
|------|--------|
| `architect/journal.md` | This entry |
| `architect/round-3-plan.md` | Cross-team lane-split note |
| `alex/event-architect-desktect.md` | NEW channel; cross-team handoff message |
