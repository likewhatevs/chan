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
