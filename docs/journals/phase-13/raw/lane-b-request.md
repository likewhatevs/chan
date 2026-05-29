# @@LaneB request - Phase 13 round 1

You are @@LaneB, the round-1 architect for the **structural-shell**
lane (Pane chrome + Graph + Dashboard) AND the round's **merge-gate
orchestrator**: you serialize merges to main, run the combined-tree
re-gate, and cut v0.17.0. You MAY spawn 2-4 in-session subagents via
the Agent tool. You report progress to @@Alex; @@LaneA reports
cross-lane signals (especially merge-ready slice SHAs) on the bus.

## Recover context (read in order)

- `/Users/fiorix/dev/github.com/fiorix/chan/CLAUDE.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/design.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/roadmap-round-1.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/bootstrap.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/README.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-b/journal.md` (tail of your prior turns)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-alex-lane-b.md` (inbox)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-lane-a-lane-b.md` (cross-lane from @@LaneA; may not exist yet)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-lane-a-alex.md` (Lane A merge-ready reports land here)

## Worktree + branch

Source ONLY in: `../chan-lane-b` on `phase-13-lane-b`. Create on first
turn:

```
git -C /Users/fiorix/dev/github.com/fiorix/chan worktree add ../chan-lane-b -b phase-13-lane-b
```

Journals + channels + this request file live in the MAIN checkout at
`/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/` and
are edited by ABSOLUTE PATH (never the worktree copy). For the
combined-tree re-gate + the release cut you operate in the MAIN
checkout (or a dedicated `../chan-integration` worktree); see Merge
section below.

## Scope

### Bugs

1. **Empty-pane focus ring thickness** (match top-bar across the
   whole pane).
   - `web/src/components/Pane.svelte` (`.pane.focused` at 1493-1498:
     `border-color + inset 0 0 0 2px box-shadow`; `.pane::before` at
     1467-1480 for the structural shadow). Find the top-bar ring
     source and unify thickness.

2. **Pane CSS hover wobble on mouse + keyboard focus change.**
   - `web/src/components/Pane.svelte` (`@keyframes pane-wobble-once`
     at 1516-1520; `.wobble::before` toggled by `wobbleActive`
     state at 26 / 979 / 969-992; easeOutBack curve already inline).
   - `web/src/state/tabs.svelte.ts::setActivePane` ~3130 (keyboard
     pane-switch transition - fire wobble here too).
   - Find the existing tab-pill / style-toolbar wobble for curve
     reuse (roadmap calls out the SAME effect already used by tab
     pills and right-click menus).

### Enhancement - Hybrid Graph KINDS

Backend (extend, do NOT rewrite - `GraphViewNode` / `GraphViewEdge`
already carry `kind`):

- `crates/chan-server/src/routes/graph.rs`,
  `crates/chan-server/src/routes/fs_graph.rs`.
- `web/src/api/types.ts` (`GraphViewNode` kinds at 400-453;
  `GraphViewEdgeKind` at 455-468; `node_kind: "contact"`
  discriminator).
- Ghost classification: `graph.rs` 340 / 522-558.
- Symlink resolution: `graph.rs` 1186+ (`resolve_link_dst`).
- Language nodes: `graph.rs` 466-480 (`LanguageGraphNode`).
- Add KIND-scoped entry routes so Lane A's Inspector chips have
  somewhere to navigate. Concrete URL/payload/tab-kind shape is your
  call; keep the existing `?scope=` semantics for `kind=path`.

Frontend:

- `web/src/components/GraphPanel.svelte`,
  `web/src/components/GraphCanvas.svelte`.
- `web/src/state/graphData.svelte.ts`,
  `web/src/state/store.svelte.ts`.
- `web/src/state/tabs.svelte.ts` (tab kind at 1159-1174;
  `graphTitle` at 1239-1258 - add kind prefix:
  `path=… / tag=… / lang=… / contact=…`).

Layers:

- **Path** is the spine; workspace at the bottom, edges upward;
  depth=1 default. Clicking a dir node = "expand" in FB lingo;
  clicking a node on a different path = collapse old + expand new.
  Depth slider bulk-expands first-degree dirs from the selected
  dir (path-kind only; depth does NOT apply to tag / contact /
  language).
- **Files** carry edges to parent dir always. Sub-types: text /
  document (.md) / media / binary / special (symlink, hardlink,
  block, sock, fifo). Special files need distinct icon + colour.
- **Symlinks within the workspace**: edges colored as the symlink
  node (same for hardlink); broken targets stay ghost nodes
  (dotted + ghost icon, already present).
- **Documents (.md)** are a layer over the graph; edges follow the
  TARGET kind colour: doc->doc = orange, doc->hashtag = green,
  doc->contact = yellow (current colourset).
- **Hashtags / contacts**: backlink edges to every doc that
  references them.
- **Language**: bubbles (drop the fake "edge to workspace root");
  add edges to every file of that language.

Tab title source-of-truth: `graphTitle` in `tabs.svelte.ts`.

**CROSS-LANE**: post the KIND route signature (URL + payload + tab
kind shape) on `event-lane-b-lane-a.md` as EARLY as possible. Lane
A is gated on this for its Inspector kind-chip wiring.

### Enhancement - Hybrid Infographics -> Dashboard

- **Rename** Infographics -> Dashboard.
  - `web/src/components/InfographicsTab.svelte` ->
    `DashboardTab.svelte`.
  - `web/src/components/EmptyPaneCarousel.svelte` (host, currently
    auto-rotates 3 slides every 5s).
- **Auto-resize carousel** to tab dimensions (items center or
  maximize to tab size; the items must become aware of the new
  tab size).

- **Widgets**:
  - **About**: move from `web/src/components/SettingsPanel.svelte`
    643-679 (version, embeddings flag, terminal-font + screensaver
    attributions). Embed `web-marketing/qr-donate.png`. Copy the
    "fund this" text from web-marketing. Add icon-linked website +
    source-repo links.
  - **Workspace-wide info**: workspace-root inspector reused from
    Lane A's `WorkspaceInfoBody` parity work. Coordinate on
    `event-lane-b-lane-a.md` if your slice lands before Lane A's
    parity slice.
  - **Search index graph**: read-only graph in spine-only mode
    (directories only, depth=max from workspace root). Colour codes:
    grey=pending, green=indexed, pulsing orange=indexing. **Use the
    new Lane-B graph component in read-only mode** instead of the
    current `EmptyPaneCarousel` directory-only radial layout.

- **Settings flip-back on the Dashboard** (replaces the
  `SettingsPanel` OverlayShell):
  - Pattern: `web/src/components/HybridSurfaceConfigShell.svelte`;
    mirror the existing `HybridTerminalConfig.svelte` /
    `HybridEditorConfig.svelte` / `HybridGraphConfig.svelte` /
    `HybridFileBrowserConfig.svelte` shape.
  - Body: Appearance (system / dark / light - **GLOBAL**, not
    tab-local) + Screen Lock + Screensaver + OK button.
  - Flip plumbing exists: `flipHybrid(paneId)` in
    `web/src/state/tabs.svelte.ts` 695-848 +
    `web/src/components/Pane.svelte` 1283-1309.

- **Retire `SettingsPanel.svelte`** (the OverlayShell variant) once
  all contents have moved.

- **Rebind Cmd+,**:
  - `web/src/state/shortcuts.ts` (id `app.settings.toggle` ~124).
  - From: global `SettingsPanel` overlay toggle.
  - To: flip focused component via `flipHybrid(paneId)` - works
    across Terminal / Editor / Graph / FB / Dashboard.

## Subagent budget

2-4 in-session subagents max. Suggested slicing (you own the call):

- Subagent 1: pane bugs (highlight + wobble).
- Subagent 2: Graph KINDS - backend route + KIND signature post
  (PRIORITIZE so Lane A unblocks).
- Subagent 3: Graph KINDS - frontend layers + tab title.
- Subagent 4: Dashboard rename + carousel auto-resize + Settings
  flip-back + Cmd+, rebind + `SettingsPanel` retirement.

## Coordination rules

- Append-only directional channels; never edit another agent's entries.
- **Each turn, BEFORE acting**, read:
  - `event-alex-lane-b.md` (inbox).
  - `event-lane-a-lane-b.md` (cross-lane from @@LaneA, if exists).
  - `event-lane-a-alex.md` (Lane A merge-ready reports - YOU consume
    these to do the merge gate).
- Progress + your own merge-ready + merge-gate confirmations +
  release cut: append to `event-lane-b-alex.md`.
- Cross-lane to @@LaneA: append to `event-lane-b-lane-a.md` (create
  on first use). **POST KIND ROUTE SIGNATURE HERE EARLY.**
- Self-document in `lane-b/journal.md`.
- Subagents speak through you on the bus.

## Per-slice gate (mandatory before any "ready to merge")

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --no-default-features
(in web/)            npm run check  &&  npm run build
(web-marketing/)     same check when web-marketing/ touched
```

Then append to `event-lane-b-alex.md`:

```
ready to merge: phase-13-lane-b@<sha>  -  <one-line slice summary>
```

Per `feedback_svelte_static_gate_misses_runtime`: browser-smoke
component reactivity changes (Graph KINDS + Dashboard fall heavily
in this bucket). Per `feedback_terminal_webgl_wkwebview`: pane
wobble + any terminal-render-adjacent changes need a chan-desktop
smoke, not just Chrome.

## Merge-gate role

You serialize merges to main. @@LaneA does NOT merge itself.

Per merge-ready (yours or @@LaneA's):

1. Read `event-lane-a-alex.md` and your own `event-lane-b-alex.md`
   for queued merge-ready slices.
2. In a fresh integration spot (the MAIN checkout or a dedicated
   `../chan-integration` worktree), fetch the lane branch, merge it
   into an integration branch off main, then run the FULL gate on
   the COMBINED tree (cargo fmt / clippy -D warnings / test /
   build --no-default-features + web check / build).
3. If green: fast-forward main; confirm on `event-lane-b-alex.md`
   (note the merged SHA + summary).
4. If red: do NOT merge; report the failure to the affected lane on
   `event-lane-b-lane-a.md` (or back to yourself in the journal so
   the responsible subagent fixes + re-reports).
5. **NEVER `git push` to origin without an explicit @@Alex ask**
   (per `feedback_merge_is_not_push`).

## v0.17.0 release cut (round close)

Per `reference_release_cut_mechanics`:

1. Confirm both lanes drained + main is green on the full gate.
2. Bump version in:
   - `Cargo.toml [workspace.package] version`
   - `desktop/src-tauri/tauri.conf.json`
   - `Cargo.lock` (cargo build refreshes it)
3. Dry-run `release.yml` via `gh workflow run release.yml -f publish=false`.
4. Inspect artifacts; if good, tag `v0.17.0` on main (annotated tag).
5. Push the tag ONLY after explicit @@Alex confirmation -
   `release.yml` fires on tag.
6. Verify `/dl/latest.json` supersedes 0.16.0; verify self-upgrade
   from 0.16.0 -> 0.17.0 in chan-desktop.
7. Commit phase-13 docs / journals / channels to main as
   `docs(phase-13): close round 1` (per
   `feedback_coordination_docs_commit_timing`).

## Out of scope

Anything not in `roadmap-round-1.md`. Escalate scope creep to
@@Alex on `event-lane-b-alex.md`.

## First turn checklist

1. Create the worktree + branch (above).
2. Read all recovery files (above).
3. Append an opening entry to `lane-b/journal.md`.
4. **PRIORITIZE**: scope the KIND route signature (backend) and post
   it to `event-lane-b-lane-a.md` so @@LaneA unblocks.
5. Pick your first implementation slice; spawn subagent(s) if useful.
6. Work the slice to the gate; report on `event-lane-b-alex.md`.
