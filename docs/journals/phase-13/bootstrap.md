# Phase 13 round 1 bootstrap

Opened 2026-05-28 by @@Alex. Round-1 scope: `roadmap-round-1.md`.
Release target: **v0.17.0** (minor; the Graph KINDS + Dashboard rework
are architectural, not patch-level).

## Roster

| Handle  | Role                                                       |
|---------|------------------------------------------------------------|
| @@Alex  | Human owner. Coordinates execution; final word on scope.   |
| @@LaneA | Content surfaces: Editor + Terminal + Inspector. MAY spawn |
|         | 2-4 in-session subagents (Agent tool, one per bug or       |
|         | enhancement section).                                      |
| @@LaneB | Structural shell: Pane chrome + Graph + Dashboard. ALSO    |
|         | merge-gate orchestrator: combined-tree re-gate, serialize  |
|         | merges to main, v0.17.0 cut. MAY spawn 2-4 subagents.      |

## Lane split (content vs structural)

Lane A - content surfaces:
- Bugs: new-doc cursor focus; "unsaved changes from a previous session"
  on fresh docs; list marker preservation (hyphen / `*` / number);
  Shift-Enter in Hybrid Terminal agent prompt submitting instead of
  inserting a newline.
- Enhancement: Hybrid Inspector. "Show path" becomes absolute path +
  copy button reusing the existing right-click "Copy path to file"
  code; FILE KIND / LANGUAGE / hashtag / contact chips become
  Graph-from-here links per kind; workspace-root inspector reaches
  parity with the folder inspector.
- Surfaces: `web/src/editor/*`, `web/src/terminal/*`, the shared
  Inspector (InspectorBody / FileInfoBody / TagInfoBody /
  WorkspaceInfoBody / KindChip).

Lane B - structural shell:
- Bugs: empty-pane focus ring thickness (match top-bar across the
  pane); pane CSS hover wobble on mouse enter/leave AND on keyboard
  focus change.
- Enhancement: Hybrid Graph KINDS rework (path/contact/hashtag/language
  layers, expand/collapse with workspace at the bottom,
  depth-only-for-path, symlink-colored edges, document layer with
  target-kind edge colours, language bubbles with edges to files,
  kind in the tab title).
- Enhancement: Hybrid Infographics -> Hybrid Dashboard rename +
  auto-resize carousel; widgets (About w/ qr-donate + repo/website
  links; Workspace inspector; Search index graph in read-only
  spine-only mode reusing the new graph); Settings flip-back replacing
  the SettingsPanel OverlayShell (Appearance with system/dark/light,
  Screen Lock, Screensaver, OK); retire the SettingsPanel OverlayShell;
  rebind Cmd+, to flip the focused component.
- Surfaces: `web/src/components/{Pane, GraphPanel, GraphCanvas,
  InfographicsTab->DashboardTab, EmptyPaneCarousel,
  HybridSurfaceConfigShell additions, SettingsPanel removal}`,
  `web/src/state/{graphData, tabs, shortcuts}`,
  `crates/chan-server/src/routes/{graph, fs_graph}.rs`.

Full per-lane file map lives in `lane-{a,b}-request.md`.

## Cross-lane sequencing

One real dependency: Lane A's "Inspector -> Graph from here for
kind=X" needs Lane B's KIND-scoped graph routes + tab plumbing landed
first.

1. Lane B ships KIND routing (backend `?kind=` + frontend tab kind +
   `graphTitle` prefix) as an early slice.
2. Lane B posts the route signature on `event-lane-b-lane-a.md`.
3. Lane A wires `KindChip` / Inspector click handlers afterwards.

All other touches are file-disjoint. Lane A can land the absolute-path
+ copy + workspace-root parity slices in parallel without waiting.
Declare any unexpected `web/src` overlap on the cross-lane channels
BEFORE editing.

## Per-slice gate (mandatory before any "ready to merge")

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --no-default-features
(in web/)   npm run check  &&  npm run build
(web-marketing/ check when web-marketing/ touched)
```

Each lane reports merge-ready slices on `event-lane-{a,b}-alex.md`:

```
ready to merge: phase-13-lane-{a,b}@<sha>  -  <one-line slice summary>
```

Per `feedback_svelte_static_gate_misses_runtime`: browser-smoke
component reactivity changes. Per `feedback_terminal_webgl_wkwebview`:
pane wobble + terminal-render-adjacent changes need a chan-desktop
smoke, not just Chrome.

## Merge + release (Lane B owns)

@@LaneB re-gates the **combined** tree before merging either lane to
main. Lanes do NOT merge themselves; they hand merge-ready slices to
@@LaneB on the bus.

At round close, on a clean main:
1. Bump version in `Cargo.toml [workspace.package]`,
   `desktop/src-tauri/tauri.conf.json`; refresh `Cargo.lock`.
2. Dry-run `release.yml` via `gh workflow_dispatch` with
   `publish=false`.
3. Tag `v0.17.0` (annotated) on main.
4. Push the tag ONLY after explicit @@Alex confirmation -
   `release.yml` fires on tag (per
   `reference_release_cut_mechanics`).
5. Verify `/dl/latest.json` supersedes 0.16.0; verify self-upgrade
   from 0.16.0 -> 0.17.0 in chan-desktop.
6. Commit phase-13 docs as `docs(phase-13): close round 1`.

NEVER `git push` to origin without an explicit @@Alex ask (per
`feedback_merge_is_not_push`).

## Worktrees

Source-only worktrees outside the main checkout:

```
git -C /Users/fiorix/dev/github.com/fiorix/chan worktree add ../chan-lane-a -b phase-13-lane-a
git -C /Users/fiorix/dev/github.com/fiorix/chan worktree add ../chan-lane-b -b phase-13-lane-b
```

Coordination docs / journals / channels are edited in the MAIN
checkout by ABSOLUTE PATH so @@Alex sees one bus (per
`feedback_shared_worktree_commits`).

## Coordination bus

```
docs/journals/phase-13/coordination/
  README.md                       channel convention
  event-alex-lane-{a,b}.md        @@Alex inboxes for lanes
  event-lane-{a,b}-alex.md        lane reports to @@Alex
  event-lane-{a,b}-lane-{b,a}.md  cross-lane (created on first use)
```

Per-lane journals:

```
docs/journals/phase-13/lane-a/journal.md
docs/journals/phase-13/lane-b/journal.md
```

Self-document in the journal (per `feedback_self_document_in_task`);
don't rely on @@Alex relaying chat.

## Docs commit timing

Per `feedback_coordination_docs_commit_timing`: keep phase-13
plans / journals / channels UNTRACKED / dirty as the live bus during
the round; commit the whole tree to main as `docs(phase-13): ...` at
round close (this opening scaffold included).

## Out of scope this round

Anything not in `roadmap-round-1.md`. Escalate scope creep to @@Alex
on `event-lane-{a,b}-alex.md`.
