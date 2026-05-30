# C2 execution spec - docs/journals as the second brain

@@LaneC / the `/architect`, phase 14. This is the sign-off spec for
C2 (the journals reorg). No report content is written yet. Execution
starts only after @@Alex approves this shape.

Decisions already taken (from the kickoff survey):

- Spec first, then execute (this file).
- Raw per-author logs are ARCHIVED into a `raw/` subdir per phase; the
  new report is the unambiguous front door.

## Goal

`docs/journals/` becomes the project's second brain: how we got here,
what we tried, did, undid, and roughly how long it took. Per phase, one
uniform synthesized report is the front door. The raw per-author
journals, task files, coordination logs, and prior summaries move under
`phase-N/raw/` as provenance. All images are removed and replaced with
short text descriptions. The top-level index is refreshed to cover every
phase.

## Inventory (what we are working with)

```
phase   status        files  md   notes
------  ------------  -----  ---  -------------------------------------
1       closed           31   29  has summary.md
2       closed           43   42  has summary.md
3       closed           35   35  has summary.md
4       sparse            3    3  empty bugs.md, process.md, stale README
5       closed           33   33  has summary.md
6       closed           38   36  has summary.md; 2 images
7       closed          167  150  no summary; 17 images
8       closed          289  283  no top-level summary; 6 images
9       closed           37   21  no top-level summary; 15 images
10      closed           20   20  has summary.md
11      closed           39   36  has retrospective.md; 3 images
12      closed           49   45  has retrospective.md; 4 images
13      closed           41   26  has 2 round retrospectives; 15 images
14      in progress      11   11  THIS round; excluded from the sweep
```

Totals: 63 PNG files across 43 markdown files. 13 phases get a report
(1-13). Phase 14 gets an index entry only (see below).

## Report shape (one `README.md` per phase)

The report is `docs/journals/phase-N/README.md`. README is chosen
because it is the rendered front door when the directory is opened.
Every report uses the same section order and voice:

```
# Phase N - <one-line descriptor>

Status: closed | in progress
Span: <start> to <end> (estimate; see Duration)

## Initial asks
The source request(s) that opened the phase. Link the request file in
raw/. Quote the ask, do not paraphrase it into something it was not.

## Team, profiles, and coordination
Table of the lanes/agents involved and their role this phase. Link each
contact card under ../../agents/. Map legacy handles via the agents
README (for example @@Backend resolves to @@FullStack). Then describe
the coordination scheme used THIS phase and how it differed from the
neighboring phases: the process evolved over time (early phases used
flat task files at the phase root; later phases moved to one dir per
author, append-only dated journals, event-channel files, and
architect-orchestrated dispatch). Capture which scheme this phase ran
on, because the process is itself part of the history.

## Duration
An estimate with the method stated (see Duration estimation). Flag it
as an estimate, never as a measured fact.

## Highlights and lowlights
What went well; what did not. Factual, specific, no marketing.

## Constructive feedback
What to do differently next time. Honest, addressed to the team,
@@Alex, and the architect where each applies.

## What shipped, tried, and undone
The outcomes, including dead ends and reverts. This is the heart of the
second brain: the things we tried and undid matter as much as what
shipped.

## Raw material
Links into raw/ (journals, task files, coordination, the request, any
prior summary or retrospective).
```

Team-and-profiles table format (pure ASCII, target 80 columns):

```
handle        role this phase                         card
------------  -------------------------------------   -----------------
@@LaneA       graph + file-browser carryover           ../../agents/...
```

## Raw archive layout

For each phase 1-13:

- Create `phase-N/README.md` (the report).
- `git mv` every pre-existing item at the phase root into
  `phase-N/raw/`: per-author dirs, task files, coordination/, round-*.md,
  request files, and the existing summary.md / retrospective*.md (these
  become inputs, folded into the report and retained as provenance).
- The only thing left at the phase root is the new `README.md`.

Why block-move is link-safe: moving a phase's content as one unit into
`raw/` preserves every relative link AMONG those files (their relative
positions do not change). Only two classes of links need fixing:

1. the new report's links down into `raw/`;
2. the top-level `docs/journals/README.md` index entries.

`git mv` is used throughout so history follows the files.

Cost: phase-8 moves ~289 files and phase-7 ~167. This is mechanical but
large; it is the cost of the front-door separation that was chosen.

## Image removal

Two-part operation, applied to the report and to every file now under
`raw/`:

1. Rewrite each REAL embedded image reference to a text description.
2. `git rm` the 63 PNG files (and any now-empty `attachments/` dirs).

Replacement convention. A reference like:

```
... despite the caption to do so: ![](./attachments/image.png#w=250)
```

becomes:

```
... despite the caption to do so: _[screenshot removed: New File/Dir
menu not accepting directory entry]_
```

Description source, in priority order: (1) existing alt text;
(2) the caption or sentence immediately preceding the image; (3) if
neither exists, `_[screenshot removed; no caption preserved]_`. Do not
invent what a screenshot showed beyond what the surrounding text states.

Do NOT touch illustrative `![](pic.png)` that appears inside backticks
or code blocks as an EXAMPLE of markdown syntax (for example phase-3
"Image files referenced by markdown (`![](pic.png)`)"). Those have no
file behind them; rewriting them would corrupt the prose. Rule: only
rewrite an image reference whose target resolves to one of the 63 PNG
files being deleted.

## Top-level index refresh (`docs/journals/README.md`)

The current index stops at phase-7, marks phase-7 "in progress", calls
phase-4 "missing", and omits 8-14. All of that is stale. The refreshed
index:

- Lists phases 1-14 with current status and a one-line descriptor.
- Links each row to `phase-N/README.md`.
- Reconciles the phase-4 note: it is a real but sparse phase, not
  "missing". State that its material is limited and what survives.
- Marks phase-14 "in progress, report pending close".
- Updates the conventions section to describe the new layout (report at
  the phase root, provenance under `raw/`, no images).

Proposed index table (descriptors finalized during execution):

```
phase   status        front door
------  ------------  ---------------------
1       closed        phase-1/README.md
...
13      closed        phase-13/README.md
14      in progress   phase-14/  (live round)
```

## Phase-specific handling

- **Phase 4 (sparse):** write a short report from `process.md`;
  `bugs.md` is empty. Do not pad it to match the longer phases. Correct
  the contradictory "missing" / "skipped in numbering" notes.
- **Phase 14 (in flight):** EXCLUDED from the archive sweep. Its
  coordination files are the live bus for this very round and stay where
  they are; per round convention the round's docs commit at close. It
  gets its own report and `raw/` archive when phase 14 closes, not now.
  The index lists it as in progress.

## Blueprint caveat (addendum-1 #4)

The phase-13 report must record the Team Work notification bubbles
accurately: the overlay was reduced to a static stub and the fsnotify
watcher / agent-event backend was deleted in round 2, with equivalent
functionality planned to RETURN in a later phase. The orchestration
skill docs (`docs/agents/orchestration/`) intentionally retain the
removed watcher / event-file / bubble-reply design as the BLUEPRINT for
that return. The report describes this as "removed, returning later",
not as dead history. C2 does not edit the orchestration blueprint.

## Duration estimation method

Derive each phase span from, in order of trust: dated headers inside the
journals; first and last git commit touching `docs/journals/phase-N/`;
file mtimes as a last resort. Report a span (start to end) and label it
an estimate with the basis named. Wall-clock session time is not
recoverable, so do not state hours.

## Writing rules (workspace standard)

Factual, no marketing language, no em dashes, ASCII tables targeting 80
columns, claims verified against the source, written for a human reader.
Do not invent history; mark estimates as estimates.

## Execution order (after sign-off)

1. For each phase 1-13, digest the raw material (read-only subagents may
   gather per-phase notes; the architect authors the final report so the
   voice is uniform) and write `phase-N/README.md`.
2. `git mv` pre-existing phase content into `phase-N/raw/`.
3. Image sweep: rewrite real references, `git rm` the 63 PNGs and empty
   attachment dirs.
4. Fix report links into `raw/`; refresh the top-level README index and
   conventions.
5. Run the gate.

Method note: per-phase synthesis may use read-only Explore /
general-purpose subagents to digest the large raw trees (phase-7 and
phase-8 especially). This is subagent fan-out for reading, not a
Workflow run; the architect writes every report for one consistent
voice.

## Gate (C2 done when)

- `find docs/journals -iname '*.png'` returns nothing.
- No live `![](...)` reference resolves to a missing file (grep + a
  link check across reports and raw/).
- All links in the reports and the index resolve.
- The index lists phases 1-14 with correct status and links.
- Writing rules hold (no em dashes, ASCII tables, claims grounded).

## Commit cadence

Phase-14 round docs stay untracked during the round and commit at close
as `docs(phase-14)`. C2 is a large standalone deliverable; default is to
fold it into that round-close commit unless @@Alex wants C2 committed as
its own atomic commit when it lands. Flagging, not re-asking.

## C1 status (for context; not part of this spec)

C1 (the round-2 frontend comments/docs/copy pass) is BLOCKED. The
`chan-p14-lane-a` and `chan-p14-lane-b` worktrees both sit at the
current main HEAD (`10e0a1e1`), so nothing from @@LaneA or @@LaneB has
merged yet. C1 edits the same frontend code they are rewriting, so it
starts only after they land.
