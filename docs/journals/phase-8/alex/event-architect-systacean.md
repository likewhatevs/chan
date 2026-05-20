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
