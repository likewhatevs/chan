# Phase 13 round 2 bootstrap

Opened 2026-05-29 by @@Alex. Round-2 scope: `roadmap-round-2.md`.
Release target: **v0.18.0** (minor; round 2 removes the agent-event
API surface and adds features. Pre-release, so no back-compat /
migration paths per house rule).

Base: round 1 is fully merged to main (`76f5e18b docs(phase-13):
close round 1`); v0.17.0 is the current cut.

## Roster

| Handle  | Role                                                          |
|---------|---------------------------------------------------------------|
| @@Alex  | Human owner. Final word on scope; sole authority to push tags.|
| @@LaneA | Team Work full-stack revamp (roadmap area "Rich Prompt").     |
|         | MAY spawn up to 4 in-session subagents (Agent tool).         |
| @@LaneB | Editor lists + Bold/Italic chords + desktop Cmd+Shift+N +     |
|         | hamburger split labels. ALSO merge-gate orchestrator:        |
|         | combined-tree re-gate, serialize merges to main, v0.18.0     |
|         | cut. MAY spawn 2-3 subagents.                                |

## Lane split (Team Work full-stack vs editor/desktop/shell)

Lane A - Team Work revamp (one cohesive full-stack feature):
- Rename Rich Prompt -> Team Work everywhere (UI + code).
- Delete the entire filesystem-watcher task/event coordination backend
  (chan-server `event_watcher`, `routes/rich_prompts`, the watcher /
  event-reply / submit-mode endpoints, `terminal_sessions` dispatch;
  chan-workspace `rich_prompts` spool) AND its frontend feed
  (`watcherEvents`, the rich-prompt-workspace archival plumbing).
- New Cmd+P flow: instantiate a Team Work Lead Terminal (embedded
  editor) FIRST, then the Spawn-agents dialog over it; Cancel deletes
  that tab, Bootstrap runs the lead-first bootstrap.
- Redesign the dialog (Neo default, New/Load TOML config toggle, 1-9
  dropdown, "drag-me" chip), reorder bootstrap (lead first), broadcast
  deselect-all then enable-only-lead+workers, reset draft after submit.
- Reduce bubbles to a frontend-only STATIC stub (equivalent
  notification functionality returns in a later phase).
- Surfaces: see `lane-a-request-round-2.md`.

Lane B - editor + desktop + shell polish + release:
- Desktop Cmd+Shift+N opens a new window of the CURRENT workspace.
- Editor list glyphs per `image-1.png` (ordered `1.`, hyphen "-",
  bullet "*" -> filled / hollow nested).
- Bold (Cmd+B) + Italic (Cmd+I) editor chords; move Dashboard off
  Cmd+I (keeps Hybrid Nav `Cmd+. i`) - @@Alex decision.
- Hamburger split-right/bottom display as plain Cmd+/ and Cmd+?.
- Surfaces: see `lane-b-request-round-2.md`.

## Cross-lane sequencing

No hard cross-lane code dependency this round (unlike round 1's KIND
routes). The only shared files are owned entirely by Lane B:

- `web/src/state/shortcuts.ts` - Lane B owns ALL edits (Cmd+I /
  Dashboard, bold/italic, split chords, AND the Team Work *label* on
  `app.terminal.richPrompt`). Lane A supplies the label string only.
- `web/src/components/Pane.svelte` + `EmptyPaneWelcome.svelte` -
  Lane B owns (structural shell + split labels + Team Work menu label).

Lane A keeps the chord *id* `app.terminal.richPrompt` stable (internal
key; only the label changes). Lane A's Cmd+P flow logic lives in
`App.svelte` (Lane A's file), needing no `shortcuts.ts` change.
Everything else is file-disjoint. Declare any unexpected overlap on the
cross-lane channels BEFORE editing.

## Per-slice gate (mandatory before any "ready to merge")

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --no-default-features
(in web/)   npm run check  &&  npm run build  &&  npm test
```

Per `feedback_svelte_static_gate_misses_runtime`: browser-smoke every
component reactivity change (the Team Work Cmd+P flow, submit-reset,
bubble stub, and editor list/chord changes all fall in this bucket).
Per `feedback_terminal_webgl_wkwebview`: the desktop Cmd+Shift+N change
and any broadcast/terminal-adjacent behavior need a chan-desktop smoke,
not just Chrome.

Each lane reports merge-ready slices on `event-lane-{a,b}-alex.md`:

```
ready to merge: phase-13-r2-lane-{a,b}@<sha>  -  <one-line summary>
```

## Merge + release (Lane B owns)

@@LaneB re-gates the **combined** tree before merging either lane to
main. Lanes do NOT merge themselves; they hand merge-ready slices to
@@LaneB on the bus.

At round close, on a clean main:
1. Bump version in `Cargo.toml [workspace.package]`,
   `desktop/src-tauri/tauri.conf.json`; refresh `Cargo.lock`.
2. Dry-run `release.yml` via `gh workflow_dispatch` with
   `publish=false`.
3. Tag `v0.18.0` (annotated) on main.
4. Push the tag ONLY after explicit @@Alex confirmation -
   `release.yml` fires on tag (per `reference_release_cut_mechanics`).
5. Verify `/dl/latest.json` supersedes 0.17.0; verify self-upgrade
   0.17.0 -> 0.18.0 in chan-desktop.
6. Commit phase-13 docs as `docs(phase-13): close round 2`.

NEVER `git push` to origin without an explicit @@Alex ask (per
`feedback_merge_is_not_push`).

## Worktrees

Round-1 worktrees `../chan-lane-a` and `../chan-lane-b` still exist on
the stale `phase-13-lane-a` / `phase-13-lane-b` branches. Each lane, on
its FIRST turn, brings its worktree onto current main with a fresh
round-2 branch (run from the worktree dir or with `-C`):

```
git -C ../chan-lane-a checkout -B phase-13-r2-lane-a main
git -C ../chan-lane-b checkout -B phase-13-r2-lane-b main
```

(`checkout -B` resets the branch to main. Confirm the worktree is clean
first - `git -C ../chan-lane-{a,b} status`; round 1 left it merged so it
should be.) Source-only in the worktree. Coordination docs / journals /
channels / request files are edited in the MAIN checkout by ABSOLUTE
PATH so @@Alex sees one bus (per `feedback_shared_worktree_commits`).

## Coordination bus

Reuses the round-1 channels under
`docs/journals/phase-13/coordination/`. A `--- Round 2 ---` divider
marks where round 2 begins in each channel (the round-1 entries above
it are committed history; round-2 entries append below).

```
event-alex-lane-{a,b}.md        @@Alex inboxes for lanes
event-lane-{a,b}-alex.md        lane reports to @@Alex (B also: merge
                                gate confirmations + release cut)
event-lane-{a,b}-lane-{b,a}.md  cross-lane (low traffic this round)
```

Per-lane journals (append a round-2 section to the round-1 files):

```
docs/journals/phase-13/lane-a/journal.md
docs/journals/phase-13/lane-b/journal.md
```

Self-document in the journal (per `feedback_self_document_in_task`);
don't rely on @@Alex relaying chat. Curated highlights / lowlights /
contention on the bus (per `feedback_curated_status_reports`); detail
in the journal.

## Docs commit timing

Per `feedback_coordination_docs_commit_timing`: keep phase-13 round-2
plans / journals / channels / request files UNTRACKED / dirty as the
live bus during the round; commit the whole tree to main as
`docs(phase-13): close round 2` at round close (this opening scaffold
included).

## Out of scope this round

Anything not in `roadmap-round-2.md`. Escalate scope creep to @@Alex
on `event-lane-{a,b}-alex.md`.
