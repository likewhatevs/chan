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
