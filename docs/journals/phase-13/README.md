# Phase 13 - Graph/Dashboard rework, then the Team Work revamp

Status: closed (two rounds; round 1 cut v0.17.0, round 2 cut v0.18.0)
Span: 2026-05-28 to 2026-05-29 (estimate; see Duration)

## Initial asks

Two rounds, each with its own source request from @@Alex.

Round 1 ([raw/roadmap-round-1.md](raw/roadmap-round-1.md)): a bug list
(new-document cursor not ready to type, a spurious "Unsaved changes"
notice on brand-new documents, list markers not preserving what the user
typed, an empty-pane highlight inconsistency, missing pane hover wobble,
Shift+Enter submitting instead of inserting a newline) plus three
enhancement areas: a Hybrid Inspector (absolute path with copy, per-kind
"Graph from here" chips, workspace-root parity), a Hybrid Graph KINDS
rework (path/contact/hashtag/language layers, expand/collapse,
symlink-colored edges), and renaming Hybrid Infographics to Dashboard
(auto-resize carousel, About/Workspace/Search widgets, Settings flip-back,
retiring the settings overlay, rebinding Cmd+,).

Round 2 ([raw/roadmap-round-2.md](raw/roadmap-round-2.md)): a desktop
global chord for a new window of the current workspace (Cmd+Shift+N);
renaming Rich Prompt to Team Work in the UI and the code, removing the
Spawn-agent dialog, and "deleting the entire code and API related to
filesystem watcher for tasks and events between agents," with a new
lead-first Cmd+P flow and a redesigned Team Work dialog. The notification
bubbles were reduced to a frontend-only stub with an explicit caveat:
"we will later add equivalent notification bubble functionality, so, for
now, I would like to leave only the frontend parts and stub/example
code." Plus editor list-rendering glyphs, Bold/Italic chords (Cmd+B /
Cmd+I), and new hamburger split-shortcut labels.

## Team, profiles, and coordination

Two positional lanes across both rounds; only @@Architect-class concepts
resolve to cards. There was no separate @@Architect handle this phase:
@@Alex wore the planning hat (authoring the roadmaps, the closing brief,
and the round-2 request files).

```
handle    role this phase                              card
--------  ----------------------------------------     ----------------
@@Alex    owner and planner; sole authority to push    (human owner)
          tags; authored the roadmaps and briefs
@@LaneA   R1 content surfaces (editor/terminal/        (no card; nearest
          inspector); R2 Team Work full-stack lead      is fullstack-a.md)
          (rename, backend deletion, new flow)
@@LaneB   R1 structural shell (pane/graph/dashboard)   (no card; nearest
          + merge gate (cut v0.17.0); R2 editor lists   is fullstack-b.md)
          + chords + desktop + merge gate (cut v0.18.0)
```

Both lanes were allowed to spawn in-session subagents (round 2 lane A used
four: backend deletion, frontend foundation, the Team Work component, and
the bubble stub).

Coordination scheme: a two-round structure reusing the same
bootstrap/roadmap/request/channel pattern, with round-2 entries appended
below a divider in each channel. Per-author append-only directional
channels (`event-<from>-<to>.md`) and per-lane journals live in the main
checkout, while source code lives in per-lane git worktrees. The merge
gate is owned by @@LaneB: lanes hand merge-ready slices on the bus,
@@LaneB re-gates the combined tree, serializes merges, and cuts the
release, with no remote push without an explicit @@Alex ask. Round 1 had
one hard cross-lane dependency (the Inspector kind chips gated on lane B's
kind graph routes); round 2 had none, with lane B owning the shared files.

## Duration

Estimate: 2026-05-28 into 2026-05-29. Basis: git author dates run
2026-05-28 22:17 to 2026-05-29 11:16, with round 1 in the late-05-28 /
early-05-29 window and round 2 through 05-29. The round boundary straddles
the two days.

## Highlights and lowlights

Highlights:
- Both round-1 lanes closed end to end: every roadmap item and every
  closing-smoke item shipped. The KIND graph rework landed coherently
  across the backend discriminator, the open-graph helpers, and the
  Inspector chips.
- Round-2 auto-merge was clean across three overlapping files; the one
  real overlap was declared cross-lane before editing.
- Browser smoke earned its keep twice in round 2: it caught a nested-glyph
  gutter detachment and confirmed the Cmd+, per-pane invariants and the
  rich-prompt wire rename end to end, where the static gate was green on
  both broken intermediates.
- The 160-reference, 35-file rich-prompt scrub landed with zero
  svelte-check collisions and a green browser smoke.

Lowlights:
- A round-1 Cmd+, bug on the desktop (WKWebView) could not be root-caused
  from the CLI (Chrome MCP cannot drive WKWebView); a defensive matcher
  shipped, flagged unverified.
- The round-1 merge gate missed three of lane A's closing slices on the
  first cycle because it gated against a stale status rather than the
  channel tail; @@Alex's nudge caught it.
- A round-2 list-glyph first attempt was wrong and only the browser smoke
  caught it; one avoidable round-trip.

## Constructive feedback

- Lane A: the "already satisfied" investigations (refusing to fabricate a
  change when current code handles the symptom) were the right
  pre-release call; the round-2 rename left a few user-facing residuals, so
  run a `grep -i "rich.?prompt"` sweep and flag any intentionally-kept
  internal ids before signalling merge-ready.
- Lane B / merge gate: read the channel tail before each gate, not the
  last-noted status; escalate desktop-only bugs to @@Alex sooner; and
  before a blanket rename, pre-scan for assertions that check the old
  string's absence.
- @@Alex: the single-document roadmaps and the per-item closing brief were
  the right shape; stating "no legacy identifiers, pre-release so no
  back-compat" up front would have let lane A do the full scrub in its own
  commit instead of the merge gate retrofitting it.

## What shipped, tried, and undone

Round 1 (v0.17.0): the new-document cursor focus, the fresh-draft
"Unsaved changes" suppression, list-marker source preservation, the
terminal Shift+Enter newline, the Hybrid Inspector (absolute path, copy,
workspace-root parity, per-kind chips), the KIND graph rework across
surfaces, and the Infographics-to-Dashboard rename with the carousel,
widgets, Settings flip-back, and the Cmd+, rebind.

Round 2 (v0.18.0): the desktop Cmd+Shift+N new-window chord; the Rich
Prompt to Team Work rename across UI and code; the new lead-first Cmd+P
flow with the redesigned dialog; editor list glyphs and the Bold/Italic
chords; and the new hamburger split labels.

Removed in round 2, returning later (recorded accurately, not as dead
history): the Team Work notification bubble overlay was reduced to a
frontend-only static stub, and the fsnotify-watcher agent-event
coordination backend (the event watcher, the rich-prompt routes and
endpoints, the terminal-session dispatch, and the workspace spool) was
deleted. Equivalent notification functionality is planned to return in a
later phase. The orchestration skill docs under `../../agents/orchestration/`
still describe the removed watcher / event-file / bubble-reply system
intentionally, retained as the blueprint for that returning
implementation; a fuller rewrite lands when the replacement does.

Tried then undone (within the round): a first list-glyph attempt using
absolute-positioned pseudo-elements was reverted to in-flow CSS after the
browser smoke caught a gutter detachment; a blanket-scrub edit that flipped
a test's absence-guard was caught by vitest and fixed; and a narrow
residual-cleanup commit was superseded by the broader scrub.

Release note: v0.18.0 cut green across all jobs. One post-cut item was a
`chan.app/dl/*` 404 from GitHub Pages CDN propagation lag (not a
release-cut failure), resolving as it propagated. The 0.17.0-to-0.18.0
self-upgrade is data-driven from `/dl`, left as an @@Alex desktop verify.

## Raw material

- Source requests: [raw/roadmap-round-1.md](raw/roadmap-round-1.md),
  [raw/roadmap-round-2.md](raw/roadmap-round-2.md)
- Round retrospectives:
  [raw/retrospective-round-1.md](raw/retrospective-round-1.md),
  [raw/retrospective-round-2.md](raw/retrospective-round-2.md)
- @@Alex's round-1 smoke walk (the primary image source):
  [raw/round-1-closing-tests.md](raw/round-1-closing-tests.md)
- The release-cut tail with the /dl saga:
  [raw/coordination/event-lane-b-alex.md](raw/coordination/event-lane-b-alex.md)
- The bootstraps, per-lane requests and journals, and the remaining
  channels live alongside them in [raw/](raw/).

The roadmaps and the round-1 smoke walk originally embedded screenshots of
the reported bugs and the target UI; per the journals-wide image removal
each is now a short text note in its source file.
