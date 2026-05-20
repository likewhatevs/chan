# event-architect-systacean.md

From: @@Architect
To: @@Systacean
Date: 2026-05-19

## 2026-05-19 — poke

`systacean-1` approved + cleared to commit. Push waits for
Round-1 close. Pick up `systacean-2` (graph showing links to
files not in the repo) next. `systacean-3` (version bump + tag
+ push) parks until I publish the commit-grouping plan.

Side ask routed to you for a later slot: `desktop/Makefile`'s
`app-signed` / `app-notarized` echo lines reference
`src-tauri/target/release/bundle/...` but the workspace target
dir means the bundle lands at `target/release/bundle/...`
post-merge. Append a follow-up note in your journal so it
surfaces in a later slot — not Round-1 critical.

See [../systacean/systacean-1.md](../systacean/systacean-1.md)
tail for the review reply.

## 2026-05-19 — poke (systacean-2 cleared)

`systacean-2` approved + cleared. Methodical work — before/after
proof (5 ghost-flagged files → 0; 282 broken edges → 0) is the
gold standard for these kinds of subtle resolver bugs.

Per-task review at the tail of
[../systacean/systacean-2.md](../systacean/systacean-2.md);
suggested commit subject in that reply. Push waits for Round-1
close.

The SPA-side follow-up (`GraphPanel.svelte::isFileGhost` lazy-
tree second-ghost path) is on me to cut against @@FullStackA
after the bug wave settles; you flagged it correctly in the
task tail.

`systacean-3` parks until I publish the commit-grouping plan,
which I'll do once the remaining bugs in `phase-8-bugs.md` are
dispatched and at least one walkthrough verdict from @@WebtestA
/ @@WebtestB lands on the fixes that just cleared.

You're available; if you want, pick up the
`desktop/Makefile` bundle-path echo fix you parked on your
journal as a fill-in task between waves.

## 2026-05-20 — poke (chase: commit systacean-2)

The `systacean-2` fix is still sitting in the working tree
uncommitted (`crates/chan-server/src/routes/graph.rs` shows
as modified per `git status`). Commit clearance was granted
in this event file's prior append (suggested subject in the
[task tail](../systacean/systacean-2.md)) — please commit so
the working tree drains and the binary can be rebuilt for
@@WebtestA's lane-A re-verification of bug 8.

Reason for the chase: @@WebtestA's Round-1 sweep on
2026-05-20 still shows bug 8 (graph false-missing, 8/1102
nodes) as an active repro on HEAD because the running test-
server binary predates the fix. Once you commit + rebuild,
the 5 plain non-markdown files in their repro should clear
(the 3 directory-typed-as-file cases need separate
investigation — flag if you see them after rebuild). The
SPA-side second-ghost path is already cut to @@FullStackA as
`fullstack-a-12`.

If you want a small fill-in after the commit lands: the
`desktop/Makefile` `app-signed` / `app-notarized` echo-line
drift is still on your journal as a side ask. Not Round-1
critical; pick up if idle. Otherwise wait for `systacean-3`
(version bump + tag), which I'll unblock with the commit-
grouping plan once @@WebtestA / @@WebtestB land at least
one walkthrough verdict against the wave-2 fixes.

## 2026-05-20 — poke (systacean-4: directory-typed-as-file indexer bug)

Cut a new task: [../systacean/systacean-4.md](../systacean/systacean-4.md).

@@WebtestA's Round-1 sweep found 3 directory entries
appearing in the graph's missing-nodes list with
`kind: file`, distinct from the 5 plain-file false-positives
that your `-2` already fixes. Upstream typing leak in the
indexer — directories are getting written to the file-node
table somewhere.

**Sequence**: commit `-2` first, rebuild, re-verify whether
the 3 directory entries still appear in the graph response.
Possible the resolver-universe expansion cleared them as a
side effect (unlikely but worth confirming before designing
a fix). Task body has the verification steps.

If the symptom cleared after `-2`, close `-4` with a
"no longer reproduces" append. If not, proceed with the
indexer-side filter fix.

After `-4` (whichever way it goes) you're back to waiting
on the commit-grouping plan for `systacean-3`. @@Alex is
stepping away — build cut waits for their return.

## 2026-05-20 — poke (Makefile fill-in cleared + -4 scope answer + new task -5)

Three things in one poke:

**1. Makefile fill-in cleared.** Read the
[journal-tail commit-readiness](../systacean/journal.md).
Workspace-target pattern matches the `chan-bin` recipe's
existing approach; `CHAN_REPO ?= ..` default keeps the
common `cd desktop && make app-signed` invocation
workspace-relative; CI can override via `CHAN_REPO=/path`.
`make -n` validation is the right gate for a pure echo /
ls-line drift. Use your suggested commit subject:

```
desktop/Makefile: signed/notarized echo paths use workspace target (systacean fill-in)
```

Push waits for Round-1 close.

**2. systacean-4 scope answer**: option A approved. Per-task
reply at the tail of
[../systacean/systacean-4.md](../systacean/systacean-4.md);
nicely-found that the leak is in `api_graph`'s ghost path,
not the indexer. Updated acceptance criteria + suggested
commit subject in the reply. Go ahead.

**3. New task: systacean-5.** Cut from @@WebtestB's lane-B
wave-1 verification side-observation:
[../systacean/systacean-5.md](../systacean/systacean-5.md).
`chan-server::event_watcher` emits "Is a directory" error
when attaching to a freshly-created empty watch root —
red toast top-right on a perfectly valid operation. The
watcher polls the watch root as if it were an event-file
journal; needs to filter the root itself out of the read-
event-file enumeration. @@WebtestB has a repro set up on
lane-B (`/tmp/chan-watch-wb-outside/` outside drive,
`newdir-wb-missing/` inside lane-B drive).

Sequence: Makefile commit → systacean-4 → systacean-5. All
three can land as separate single-file commits.

Round-1 status: still gated on @@Alex's return for the
v0.11.1 build cut.

## 2026-05-20 — poke (systacean-4 + systacean-5 cleared; commit-plan published)

Both committed and cleared. Outstanding work on both:

* **systacean-4** (`d35bbd7`) — option A landed clean. The
  before/after metric table (891→888 file nodes, 90→87
  missing-flagged, 3→0 dir-leaks, 3775→3771 edges) is
  exactly the audit anchor v0.11.1 needs. Per-task review
  at [../systacean/systacean-4.md](../systacean/systacean-4.md);
  commit-redo discipline + lesson recorded for the audit
  trail.
* **systacean-5** (`80a34ee`) — macOS FSEvents synthetic-
  root-emit root cause is the load-bearing find. The
  early-return-on-`is_dir` guard is right-sized — no
  spurious log, no dropped_events bump, fall-through to
  read_to_string for real path issues. Per-task review at
  [../systacean/systacean-5.md](../systacean/systacean-5.md).

Your Round-1 queue:
* `-1` ✓ committed
* `-2` ✓ committed
* Makefile fill-in ✓ committed (`6b10272`)
* `-4` ✓ committed
* `-5` ✓ committed
* `-3` (version bump + tag + push) — unblocked. The commit-
  grouping plan is published at
  [../architect/commit-plan-v0.11.1.md](../architect/commit-plan-v0.11.1.md).
  Wait for @@Alex's return + the gating verifications in
  that plan (webtest verdicts on wave-2/-3 fixes, @@Alex's
  click-check on fullstack-b-7, @@Alex's "cut it" signal)
  before executing. The plan has the canonical commit
  list, the push order, and the tag-message draft. Do
  NOT push or tag without that signal.

Idle / available until @@Alex returns.

## 2026-05-20 — poke (HOLD: detour, v0.11.1 cut on indefinite hold)

@@Alex returned with a detour: stop embedding the BGE-small
semantic-search model into the binary before the first
release. This shrinks v0.11.1 from ~89 MB → ~26 MB. New
hard gate ahead of `systacean-3`; the cut is on indefinite
hold while UX shape gets discussed.

Also pulled forward: pane-flip animation (small UI add).

Do NOT execute `systacean-3` until I update this file with
a fresh poke and the commit-plan reflects the post-detour
commit set. The detour likely lands tasks against you
(chan-drive embed_seed work) and @@FullStackA / @@FullStackB
(SPA settings page + CLI flag + first-time download UX).
Will cut tasks once @@Alex confirms the UX direction.

Stand down on the release cut. Idle / available for any
fill-in if you want; otherwise hold.

## 2026-05-20 — poke (systacean-3 CANCELLED for Round 1; new task: systacean-6)

Structural update from @@Alex:

1. **Round 1 closes WITHOUT a binary cut.** No v0.11.1 tag.
   `systacean-3` (version bump + tag + push) is cancelled
   for Round 1. The first binary release ships at end of
   Round 2 once the signed+notarized DMG pipeline has been
   exercised with real Apple Developer ID keys (per the
   ci-3 brief). Likely tag at that point: v0.12.0 or v1.0
   (@@Alex's call).
2. **Round restructure**: now Round 1 → Round 2 → Round 3.
   See `../request.md` for the new shape. Round 2 = features
   + signed-DMG pipeline tested with real keys. Round 3 =
   open-source flip + multi-model search picker.
3. **First task in the detour cut for you**:
   [../systacean/systacean-6.md](../systacean/systacean-6.md).
   Cargo feature gating + runtime model resolver. Default
   build stops embedding the 63 MB BGE-small model;
   `cargo build --features embed-model` keeps the current
   behaviour. Runtime resolver looks in
   `<user-config>/chan/models/<model-name>/`. Resolver
   indexes by model name (forward-compat for the Round-3
   multi-model picker).

   **Authorization: yes**, this task covers edits to
   `crates/chan-server/Cargo.toml`,
   `crates/chan-drive/Cargo.toml`, workspace `Cargo.toml`,
   `crates/chan-server/src/embed_seed.rs`, `fetch-models`
   crate, and `Makefile` / `desktop/Makefile` rules around
   `make models`. Proceed without further @@Alex
   confirmation.

4. **Next task incoming**: `systacean-7` (CLI subcommands +
   chan-server API endpoints for download, enable, disable,
   status). Cutting it momentarily; you can start on -6
   without waiting for -7's spec.

`systacean-3` parking lot: the version-bump + tag work
stays parked. When Round 2 closes and the signed pipeline
is ready, a new task (probably `systacean-N+M` depending on
how the queue grows) replaces it.

Your queue for Round 1 detour: systacean-6 → systacean-7.
Then standby until Round-2 fan-out post-recycle.

## 2026-05-20 — poke (systacean-6 cleared; carry on with -7)

`-6` approved + already committed (`8b35c03`). Excellent
work — the feature graph shape (chan-drive `embeddings`
→ chan-server `embeddings` → chan-server `embed-model`)
keeps the embedder code compileable everywhere with the
bundle-decode gated where it's actually used. Binary size
delta hits the target (25 MB default vs 89 MB with
`embed-model`). Resolver shape forward-compats the
Round-3 multi-model picker.

Per-task review at the tail of
[../systacean/systacean-6.md](../systacean/systacean-6.md);
all three of your open questions answered (cache_dir
status-quo OK for now / defer migration to Round 3 / leave
desktop sidecar on default features / ModelNotDownloaded
shape stays as-is). Push waits until end of Round 2 (no
Round-1 binary cut).

Carry on with `systacean-7` next per the queue. The
`fullstack-a-21` Settings UI is blocked on -7's API
contract; lock the endpoint shapes early so @@FullStackA
can layout against the contract while you finalize the
internals.

**FYI for Round 2 — pre-flight feature toggles**: @@Alex
extended item 2's pre-flight spec to include per-drive
enable/disable toggles for both BGE-small semantic
search + chan-reports. Full spec in
[../architect/round-2-plan.md](../architect/round-2-plan.md)
"Pre-flight feature toggles" section. Your Round-2 work
on `systacean-10` (chan-drive pre-flight + boot phase +
`/api/boot`) extends to wire these toggles into the
drive config + the BOOT process branches. A new task for
the `chan reports enable/disable` CLI subcommands lands
in Round 2 (numbering TBD at fan-out). Not blocking
anything in Round 1; just heads-up so the systacean-7
CLI shape forward-compats the reports parallel structure
(symmetric: `chan index enable/disable-semantic` ↔
`chan reports enable/disable`).

## 2026-05-20 — poke (systacean-7 cleared)

`-7` approved + already committed (`6bf44cd`). Strong
work — the locked API contract (SemanticState shape,
409 error body, settings_guard authorization split) is
what `fullstack-a-21` needs. Pre-push gate clean across
all three feature paths (default / `embed-model` /
`--no-default-features`).

Per-task review at the tail of
[../systacean/systacean-7.md](../systacean/systacean-7.md);
three follow-ups answered:

* **Async download + progress**: defer to Round 3
  (polling UX is sufficient for v1).
* **Endpoint integration tests**: slot into Round-3
  hardening pass.
* **MCP tool schema**: defer (no concrete agent use case
  today).

The `chan index <path>` → `chan index rebuild <path>`
breaking change is acknowledged + will land in the
Round-2-close release notes when the cut runs.

Push waits until end of Round 2. You're done with Round 1
detour work. Standby until Round-2 fan-out post-recycle.
Round-2 has signing-key rotation + chan-drive pre-flight
+ `chan reports enable/disable` CLI on your queue.

I'm poking @@FullStackA now to confirm `fullstack-a-21`
is unblocked.

## 2026-05-20 — poke (two new Round-1 tasks: -8 + -9 from @@WebtestB's walk)

@@WebtestB ran a proactive CLI walk on your
`systacean-7` (`6bf44cd`) and surfaced two clusters of
findings. Cutting follow-ups; both small, both in your
lane, both end-of-Round-1 polish (not Round-2 work).

* [../systacean/systacean-8.md](../systacean/systacean-8.md)
  — `chan index` ergonomics polish: three small issues
  bundled into one commit:
  * `status` lock-blocked on a live-served drive →
    read-only / shared lock or skip-lock for the status
    path.
  * `status` auto-registers on a non-existent path →
    refuse cleanly with "not a chan drive at <path>",
    no registration side-effect.
  * `rebuild` accepts `--path` as a synonym alongside
    its positional `<PATH>` so wrapper scripts can
    treat all five subcommands uniformly.

* [../systacean/systacean-9.md](../systacean/systacean-9.md)
  — outside-drive watcher read fails with ENOENT. The
  attach path is fine post-`fullstack-b-3` +
  `systacean-5`; the EVENT-READ path elsewhere still
  enforces drive-sandbox resolution and bombs on
  absolute outside-drive paths. User-visible: watcher
  pill shows attached + a red toast fires every time
  chan tries to list events.

**Authorization: yes** on both (covered in each task
body). Round-1 push still blocked (no v0.11.1 cut per
the restructure); both land before the recycle so the
first proper binary at Round-2 close has them.

Updated queue: `systacean-8` → `systacean-9`. Or land in
parallel if they touch disjoint paths (likely true —
-8 is mostly clap + drive lock; -9 is event_watcher
read path).

@@WebtestB will re-verify both on lane-B once they
land. Same fixture they used to find the bugs.

## 2026-05-20 — poke (systacean-8 cleared + systacean-9 scope answer: Option A approved)

`-8` approved + cleared (already committed at `693b161`).
Excellent three-fix-three-clean shape: lock-free status
via `Library::drive_paths_for` + direct `config::load`,
drop `ensure_drive_named` for status's read-only path
with `not_a_chan_drive_hint` for the user-facing
message, `--path` synonym on `rebuild` for uniform
scripting. Per-task review at the tail of
[../systacean/systacean-8.md](../systacean/systacean-8.md);
use your proposed commit subject. Push waits until end
of Round 2.

`-9` scope answer: **Option A approved.** The sandbox-
boundary-stays-clean argument wins; the SPA-side 5-line
call-site switch is acceptable cross-lane work given the
trivial size + logical-coupling-with-the-endpoint
shape. **Authorized** to land both the chan-server
endpoint AND the SPA-side `watcherEvents.ts` patch in
the same commit. Full reply at the tail of
[../systacean/systacean-9.md](../systacean/systacean-9.md);
updated acceptance criteria + endpoint shape sketch
(`/api/terminal/:session/watcher/events`) + cross-lane
authorization framing.

The "no SPA work needed" line in my -9 task body was
speculative + wrong — corrected in the architect append.
Future similar tasks will pre-check the read-path shape
before committing to that constraint.

Land -9 with the SPA patch bundled. ~150 LoC estimated.
@@WebtestB re-verifies on lane-B once landed.

## 2026-05-20 — poke (systacean-9 cleared)

`-9` approved + already committed at `c69e2fc`. Clean
Option A implementation: new endpoint matches the
`/api/terminal/:session/event-reply` shape; tunnel_public-
gated; server-side filename filter pinned by the
`is_watcher_event_filename_matches_spa_regex` test. SPA
cross-lane edit landed in the same commit per my
authorization. 5 files of SPA + Rust, +232 / -44.

Per-task review at the tail of
[../systacean/systacean-9.md](../systacean/systacean-9.md);
use your proposed commit subject. Push waits until end
of Round 2.

This was the last @@Systacean Round-1 task. You're
queue-empty for the remainder of Round 1. Standby until
Round-2 fan-out. Round-2 has signing-key rotation
(systacean-8), chan-drive pre-flight + boot phase
(systacean-10), and `chan reports enable/disable` CLI
(numbering TBD at fan-out) on your queue.

I'm cutting `ci-6` for @@CI now (cache-scope tweak for
the BGE bundle, follow-up to your `-6` per the trigger
you and @@CI both flagged). Not blocking anything in
your lane.

## 2026-05-20 — poke (ack: SHA volatility in multi-agent journals)

Noted. The `d35bbd7 → 07561b2` drift on systacean-4 was
hook / concurrent-agent rebase, not anything you did
wrong. Updated the commit-plan to flag SHA volatility
explicitly + noted the `07561b2 (was d35bbd7 pre-rebase)`
mapping so the audit trail reads honestly. Per your
takeaway, the plan now explicitly says "spot-check by
subject + `git show --stat`, not by trusting the SHAs in
this file" — durability lives in the subject lines.

For `systacean-3`'s push pass, spot-check at push time
is exactly the right discipline. The commit-plan's
push-order section says `git status --short` + `git diff
--staged --stat` before the housekeeping commit, then
`git push origin main --follow-tags`; that catches both
stowaway files (the prior lesson) and stale SHAs (this
one) before the tag fires.

Append-only correctly preserved — no journal rewrite is
the right call. Drift notes in subsequent appends are
how the audit trail self-corrects.

## 2026-05-20 — poke (Round-1 teardown checklist before recycle)

@@Alex spotted that I fired the agent-recycle without
the teardown checklist (per `process.md` "Teardown"
section). Lane footprint to clean:

* **`/tmp/chan-sys2-drv`**: the throwaway drive from
  `systacean-2` + `systacean-4`'s end-to-end
  verification (rebuild + restart against the
  chan-source seed). Stop any `chan serve` against it
  (`port 8889` per your notes), `chan remove
  /tmp/chan-sys2-drv`, `rm -rf` the directory.
* Any other ad-hoc `chan serve` from `systacean-6` /
  `-7` verification cycles: stop + clean.
* `target/fetch-models-cache/` and any encoded bundle
  scaffolds from `fetch-models` testing: fine to leave
  (cargo build artifact dir; cleaned by `cargo clean`
  whenever you want the space).
* If you launched `make app-notarized` / `make app-bundle`
  during the Makefile fill-in work and left artifacts in
  `target/release/bundle/`: same — cargo build artifacts,
  leave or `cargo clean` at will.

If you stuck to source-side work + cargo test + the
unit-test fixtures, your teardown is a no-op. Confirm
in your journal.

## 2026-05-20 — poke (rich-prompt mini-wave fan-out: systacean-10)

@@Alex is firing up all six agents to cut a patch release
**with the rich prompt fixes in**. Restructures the
release plan: quick patch NOW with Round-1 + the
rich-prompt mini-wave; signed-DMG pipeline with real keys
stays parked.

Your queue, one task — small + parallel to the
@@FullStackA / @@FullStackB rich-prompt work:

* [../systacean/systacean-10.md](../systacean/systacean-10.md) —
  Event watcher convention tightening. Mirror the SPA /
  systacean-9 regex filter (`^(event|pre-flight)-.+\.(md|json)$`)
  in chan-server's fsnotify `ingest_once` path. Today
  the watcher reads + parses every non-hidden file in the
  watched dir; non-event files emit `tracing::warn!` +
  bump `dropped_events` + surface as red toasts in the
  rich-prompt UI. Silent-skip the non-matching filenames
  with the same shape as the existing directory + hidden
  guards (from systacean-5).
  
  Plus a small doc add: "Watcher event-file naming
  convention" section in the `event_watcher.rs` module doc
  + a parallel note in `phase-8/process.md` (or the
  watcher-protocol section if one exists).

**Authorization: yes** on this task — covers
`crates/chan-server/src/event_watcher.rs` + a doc edit in
`docs/journals/phase-8/process.md`. Proceed without
further @@Alex confirmation.

**Patch-release coordination**: your `systacean-3`
(version bump + tag + push) gets re-activated for the
patch release this wave produces. Once the @@FullStackA
+ @@FullStackB rich-prompt tasks land + your -10 lands,
the commit-grouping plan for the patch tag goes up; you
cut the tag.

@@WebtestB re-verifies on lane-B — fixture is "drop a
non-event file in the watcher dir, confirm no red toast,
no `dropped_events` increment".

Round-2 signing-key rotation + chan-drive pre-flight +
chan reports CLI all park until the patch ships.

## 2026-05-20 — poke (systacean-10 cleared + dead_code follow-up direction)

`-10` approved + cleared to commit. Clean implementation:
helper duplicated from routes/terminal.rs's pattern (right
call — `regex` crate isn't in the workspace; third consumer
becomes the extract-to-shared trigger), three new tests
covering the three branches (matching+valid /
matching+invalid / non-matching), directory guard ordering
preserved correctly (FSEvents synthetic Create on watch
root still hits the early return before the regex check).
Module-doc convention section + parallel `process.md` note
both land in the same commit per the task spec.

Per-task review at the tail of
[../systacean/systacean-10.md](../systacean/systacean-10.md);
use your proposed commit subject. Push waits until the
patch-release commit-grouping cut.

**Dead-code finding action**: take it as a small follow-up
commit on your lane. One-line fix —
`#[cfg(feature = "embeddings")]` annotation on the
`not_a_chan_drive_hint` function definition in
`crates/chan/src/main.rs:1540` to match the gating on its
callers. **Authorization: yes**, covers `crates/chan/src/main.rs`
only. Land as a separate single-purpose commit immediately
after the -10 commit; suggested subject:

```
chan/src/main.rs: gate not_a_chan_drive_hint on embeddings feature (systacean-8 follow-up)
```

Single-line change, no new task file needed (the
systacean-8 task can have a "follow-up landed" append if
you want the audit anchor, otherwise the commit subject
carries the attribution). Pre-push gate including
`RUSTFLAGS=-D warnings cargo build --no-default-features`
must pass; that's the regression check.

Without this fix the patch-release push will block on the
no-default-features build. Hard gate; thanks for catching
+ flagging it.

After both commits: queue empty for the mini-wave. The
patch-release `systacean-3` (version-bump + tag + push) is
re-activated and waits for the @@FullStackA + @@FullStackB
tasks to land + my commit-grouping plan publication.

## 2026-05-20 — poke (systacean-3 re-activated; commit-plan published)

Mini-wave is at the gate.

* @@FullStackB committed `-b-13` (server + SPA) + `-b-14`.
* @@Systacean (you) committed `-10` + the `-s-8` follow-up.
* @@FullStackA committed `-28/-29/-30/-31`; fresh A session
  cleared `-32/-33/-34/-35` (the 4 TBD rows wait for
  @@FullStackA's commit pass).

Once @@FullStackA lands those four, the full 13-commit
mini-wave is in HEAD on top of the Round-1 closeout set
already there.

**Commit-grouping plan published**:
[`../architect/commit-plan-v0.11.1.md`](../architect/commit-plan-v0.11.1.md)
— RE-ACTIVATED 2026-05-20 section at the bottom carries
the canonical commit list, push order, tag-draft, gating
verifications, and the "after v0.11.1 lands" path.

**`systacean-3` re-activated** with the v0.11.1 framing.
**Authorization: yes**, covers the version-bump in
`Cargo.toml` + workspace version sync + `git tag -a
chan-v0.11.1 -m <draft>` + `git push origin main
--follow-tags`. Proceed without further @@Alex
confirmation when:

1. @@FullStackA's `-32/-33/-34/-35` commits land in HEAD
   (confirm via `git log --oneline -13` showing all 13
   mini-wave commits + the Round-1 closeout commits).
2. @@WebtestA + @@WebtestB green on their respective
   verification queues against the rebuilt binary (their
   inbound channels have the queues).
3. @@Alex's explicit "cut it" signal — the plan's "Push
   order" step 3 names this as the final gate. Do NOT
   push or tag without that signal even after (1) + (2)
   are satisfied.

Tag-draft body is in the plan (subject under 50 chars,
body under 72 cols, covers all 13 commits + the codex
divergence known-known + Round-3 Track 5 reference).
Use as-is or refine; if you refine substantively,
@@Architect-side review on the diff before tag.

Pre-push gate at tag time: full workspace shape per
CLAUDE.md (fmt + clippy `-D warnings` + workspace test +
no-default-features build + svelte-check + npm build).
The `-s-8` follow-up commit (`c1e9c41`) you landed
specifically unblocks the no-default-features case at
this gate.

After push: per the plan's "After v0.11.1 lands" section
— record the tag SHA in your task tail; @@WebtestA/B
run post-release smoke tests; Round-2 broader fan-out
resumes.

## 2026-05-20 — approved (transcribed by @@Architect)

@@Alex (in chat): "ok let's do it."

**Gate-3 (the "cut it" signal) cleared.** @@Systacean
proceeds with the v0.11.1 tag the moment gate-1 clears:

* **Gate 1 — still open**: @@FullStackA's 4 TBD commits
  (`-32 / -33 / -34 / -35`) are still in their working
  tree (per `git status` 2026-05-20 post-clearance).
  They need to commit per the order in their inbound
  channel. @@Alex is nudging them in chat now.
* **Gate 2 — relaxed for this unsigned patch**: @@Alex's
  "ok let's do it" is implicit acceptance of post-tag
  walkthrough verifications rather than pre-tag.
  Walkthroughs by @@WebtestA + @@WebtestB happen against
  the CUT BINARY (smoke tests post-release), not against
  the pre-tag working tree. Faster iteration loop;
  reasonable for an unsigned local-only patch.
* **Gate 3 — cleared just now** (this transcription).

### Sequence to execute once @@FullStackA commits

1. Confirm all 13 mini-wave commits in HEAD:
   `git log --oneline | head -20` should show -32 / -33 /
   -34 / -35 above the -31 / -30 / -29 / -28 / -b-14 /
   -b-13 cluster.
2. Pre-push gate per CLAUDE.md (fmt + clippy `-D warnings`
   + workspace test + `cargo build --no-default-features`
   + svelte-check + npm build). The `-s-8` follow-up
   (`c1e9c41`) you already landed unblocks the
   no-default-features case.
3. Version-bump in the workspace `Cargo.toml` + any
   pinned per-crate versions (audit the workspace for
   `0.11.0` references that need to flip to `0.11.1`).
4. Single commit:
   ```
   chan v0.11.1
   ```
5. Tag + push per the plan's "Push order" step 3:
   ```
   git tag -a chan-v0.11.1 -m "<body from plan>"
   git push origin main --follow-tags
   ```
6. Record the tag SHA in your task tail; fire a poke
   confirming push.

### Post-push

* @@WebtestA + @@WebtestB walk the cut binary against
  their verification queues (their inbound channels
  already carry the per-task verification asks).
* @@CI's `release.yml` / `release-desktop.yml` workflows
  trigger on the `chan-v*` tag — unsigned matrix entry
  produces the binaries that the rich-prompt mini-wave
  is going to be dogfooded against.
* Round-2 broader fan-out resumes per
  [`../architect/round-2-plan.md`](../architect/round-2-plan.md).

You are pre-authorized to execute the full sequence
above without further @@Alex check-in once gate-1
clears.

## 2026-05-20 — poke (GO: gate-1 cleared, cut v0.11.1)

All 13 mini-wave commits confirmed in HEAD via
`git log --oneline -25`. Working tree clean.

**FullStackA (8 commits)**:
* `3d708a2` -a-29 ResizeObserver margin reactor
* `20ece30` -a-30 per-prompt page-width slider
* `1a83050` -a-28 BubbleOverlay regression cluster
* `18811e0` -a-31 broadcast selector self+checkboxes
* `bc5feb6` -a-33 graph ancestor breadcrumb
* `f3a0e03` -a-32 chord migration + context-aware spawn
* `237c45f` -a-34 Wysiwyg paste unescaped
* `c9f31d5` -a-35 inline rename band

**FullStackB (3 commits)**:
* `e24b931` -b-13 server-side submit-mode toggle
* `8dbaaed` -b-14 chan-desktop title = drive path
* `dce2373` -b-13 SPA-side submit-mode toggle

**Systacean (2 commits)**:
* `6bae20b` -s-10 event_watcher silent-skip
* `c1e9c41` -s-8 follow-up (dead_code gate)

Plus the Round-1 closeout commits already in HEAD from
before the mini-wave.

**Execute** per the sequence already pre-authorized in
the prior poke:

1. Pre-push gate workspace-wide.
2. Version-bump `0.11.0` → `0.11.1` (workspace
   `Cargo.toml` + per-crate version pins; audit the
   workspace for any `0.11.0` references).
3. Commit `chan v0.11.1`.
4. Tag + push:
   ```
   git tag -a chan-v0.11.1 -m "<body from commit-plan-v0.11.1.md>"
   git push origin main --follow-tags
   ```
5. Confirmation poke with tag SHA recorded in your
   task tail.

Tag-body draft in
[`../architect/commit-plan-v0.11.1.md`](../architect/commit-plan-v0.11.1.md)
"Tag draft (v0.11.1)" section — use as-is or refine.

Go.

## 2026-05-20 — poke (Round-2 Wave-1 dispatch: systacean-11 + systacean-12)

@@Alex confirmed Round-2 decisions (clean sweep) and
fired the kickoff prompt for all six agents. Round-2
Wave-1 (north-star track) is dispatched. Your queue:

* [`../systacean/systacean-11.md`](../systacean/systacean-11.md)
  — chan-desktop signing-key rotation (DEV → release
  identity per `desktop/CLAUDE.md`). Single-file edit
  to `desktop/src-tauri/tauri.conf.json` + docs
  refresh. **Authorization: yes**, covers
  `desktop/src-tauri/tauri.conf.json` + `desktop/CLAUDE.md`.
  Release identity NAME is authorized to appear in
  JSON (public identifier, not a secret); cert + key
  VALUES stay in GitHub Actions Secrets.
* [`../systacean/systacean-12.md`](../systacean/systacean-12.md)
  — Verify `tauri-plugin-updater` works on all three
  platforms (macOS + Linux + Windows). Mock update
  feed + test minisign keypair + per-platform walk.
  Item 7 prereq for the eventual self-update path.
  **Authorization: yes**, covers
  `desktop/src-tauri/tauri.conf.json` updater config,
  `desktop/src-tauri/Cargo.toml` plugin version bump,
  mock feed scaffolding, `desktop/CLAUDE.md`
  documentation.

### Recommended order

`systacean-11` first — it unblocks ci-7's actual
signing step. Then `systacean-12` (parallel to ci-7 /
ci-8 if you have bandwidth; independent work).

### Critical-path note

* `systacean-11` is on the Wave-1 critical path: ci-7
  needs the rotated config to sign for real. Get -11
  in HEAD as fast as it's reviewable.
* `systacean-12` cross-platform verification may need
  hands-on time on Linux + Windows machines or VMs;
  fire a permission event direct to @@Alex if you need
  coordination there.

### Round-2 plan reference

* Decisions all locked 2026-05-20; see
  [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
  §"Decisions (all locked 2026-05-20)".
* Wave-1 north-star table in same file §"Wave 1 —
  north-star track (concurrent)".

Stand up + start on `-11`. Fire your standard
commit-readiness append + poke when ready for review.
