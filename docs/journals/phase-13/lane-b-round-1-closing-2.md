# @@LaneB round-1 closing-2 - Phase 13

You are @@LaneB picking up the SECOND round of round-1 closing
work AND keeping the merge-gate orchestrator hat AND the v0.17.0
release-cut. The previous @@LaneB drained the original 12 closing
items + landed the chore(release): 0.17.0 commit; main is at
`e30f73ef` with the version bump committed but the v0.17.0 git tag
NOT yet cut. @@Alex's empirical walk over that tree turned up 9
more bugs; this file IS your task list. You own 7 of those + the
merge-gate cycles + the dry-run + the final tag. @@LaneA owns 2
inspector-side items (A5 + A6 in
`lane-a-round-1-closing-2.md`).

You DO cut v0.17.0 once both lanes drain + @@Alex confirms.

## Recover context (read in order)

- `/Users/fiorix/dev/github.com/fiorix/chan/CLAUDE.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/design.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/roadmap-round-1.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/bootstrap.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/retrospective-round-1.md` (the round-1 retrospective landed at `a57c259f`; read the "Constructive feedback to Lane B (myself)" section before starting)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-b-request.md` (original Lane B brief; channel + worktree + merge-gate convention)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-b-round-1-closing.md` (the prior closing brief @@LaneB worked from)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-b/journal.md` (your predecessor's full self-documentation; substantial)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-alex-lane-b.md` (your inbox)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-lane-a-alex.md` (Lane A merge queue)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-lane-a-lane-b.md` (cross-lane from @@LaneA)

## Worktree + branch

Reuse `../chan-lane-b` on branch `phase-13-lane-b`. First action:
rebase on the current main tip.

```
git -C /Users/fiorix/dev/github.com/fiorix/chan-lane-b fetch . main
git -C /Users/fiorix/dev/github.com/fiorix/chan-lane-b rebase main
```

Combined-tree re-gate cycle runs in `../chan-integration` (create
per merge-gate; tear down after each cycle). Release cut runs in
the MAIN checkout (`/Users/fiorix/dev/github.com/fiorix/chan`).

Source code work happens ONLY in `../chan-lane-b` (your own
fixes) and `../chan-integration` (during gate runs). Journals /
channels stay in the MAIN checkout edited by ABSOLUTE PATH.

## Scope

User words quoted verbatim where available; diagnoses are from the
Explore-agent triage that opened this round.

### B1c. Dashboard tab does not survive window reloads

User words: "first of all, the dashboard tab does not survive
window reloads, it simply vanishes... like any other tab, the
dashboard tab should survive reloads".

Diagnosis: `web/src/state/tabs.svelte.ts::restoreLayout` has
handler arms for `kind === "g"` (graph), `"b"` (browser), `"t"`
(terminal), then `"f"` (file), and a final
`if (kind !== "f") continue;` guard at ~line 4103 that silently
drops any unknown discriminator. The `"d"` (dashboard) arm is
missing entirely. The serializer at ~line 3846 already writes
`k: "d"` correctly; only the restore side is broken.

Fix:
1. Add a handler block above the `kind !== "f"` continue:
   ```ts
   if (kind === "d") {
     const tab: DashboardTab = {
       kind: "dashboard",
       id: id("dashboard"),
       title: "Dashboard",
     };
     p.tabs.push(tab);
     if (sertab.a) p.activeTabId = tab.id;
     continue;
   }
   ```
2. Vitest pin in `web/src/state/tabs.test.ts` round-tripping a
   Dashboard tab through `serializeLayout` -> `restoreLayout`.

Acceptance: opening a Dashboard tab + reloading the window
restores the tab in the same pane with the same active state.

### B2c. Cmd+, flip drifts on focus-switch; breaks other panes

User words: "when I click cmd+, to configure the dashboard, it
does not flip the tab, it immediately shows the settings and then
it gets buggy... if I switch focus to another pane and back, it
keeps flipping; and it also breaks the flip of other tabs... you
have to re-check the workflows of flipping each individual tab,
and these tabs all have individual properties: their size and
position, their state (e.g. if the inspector is open, the exact
size of the inspector, etc.. we want to reload the entire window
and get the exact same result back)".

Diagnosis: `web/src/state/tabs.svelte.ts::setActivePane` (~line
3160) sets `current.activePaneId = paneId` without clearing the
PREVIOUS pane's `showingBack`. If pane A is flipped and the user
focuses pane B, pane A stays flipped; flipHybrid(B) toggles B's
showingBack, but A is still stale. Rapid focus switches between
two flipped panes desync the visual state and the keymap intent.

Per-tab geometry state (inspectorWidth on file/graph/browser,
caret, outline, etc.) IS already serialized via SerTab fields in
`tabs.svelte.ts` ~lines 3522-3635 + restored at the matching
arms. The "exact inspector size" expectation is already satisfied
mechanically; the showingBack stale state is what makes "switch
focus and come back" feel broken.

Files:
- `web/src/state/tabs.svelte.ts` (`setActivePane` ~3160-3171,
  `flipHybrid` ~3173-3200)
- `web/src/components/HybridDashboardConfig.svelte` (onMount needs
  to fire `loadScreenLockState()` so the back surface always
  hydrates on first paint — currently it's defined but never
  called from onMount)
- `web/src/components/Pane.svelte` (the paneFlip `$effect` at
  ~435-450; verify no rAF desync after the setActivePane fix)
- `web/src/state/tabs.test.ts` (new pin for the showingBack-clear
  behavior)

Fix shape:
1. `setActivePane`: clear the previous pane's `showingBack` when
   focus moves:
   ```ts
   const previousActive = current.activePaneId;
   if (previousActive && previousActive !== paneId) {
     const prev = current.nodes[previousActive];
     if (prev && prev.kind === "leaf" && prev.showingBack) {
       prev.showingBack = false;
     }
   }
   current.activePaneId = paneId;
   ```
2. `HybridDashboardConfig.svelte`: add `onMount(loadScreenLockState)`
   right after the helper definition so the screensaver state
   loads every time the back surface mounts (not only "on the
   second mount").
3. Walk through the rAF/$effect interaction in `Pane.svelte`
   ~435-450 once the setActivePane fix is in; verify
   `lastFlipVersion` tracks correctly across rapid focus
   switches.
4. Vitest pin asserting that `setActivePane(b)` clears
   `nodes[a].showingBack` when `a` was the previous active pane
   with `showingBack === true`.

Acceptance:
- Cmd+, on a focused Dashboard flips to the back surface; Cmd+,
  again flips back to the carousel.
- Focusing another pane mid-flip resets the previous pane's
  showingBack to false (front-side renders on return).
- Reloading the window restores each pane's geometry + active tab
  exactly as it was (Dashboard tab now survives reload after B1c
  fix; other tabs already do).

### B3c. Screensaver config must be INSIDE the "Screen lock" block

User words: "screensaver configuration must be inside the 'Screen
lock' block in the Dashboard settings".

Diagnosis: `web/src/components/HybridDashboardConfig.svelte`
currently has a SEPARATE `<section class="screensaver">` block
(~lines 419-436) with its own `<h3>Screensaver</h3>` header,
sitting AFTER the Screen lock section. The screensaver theme picker
has no meaning when screen lock is off, so it logically belongs
inside the `{#if screensaverEnabled === true}` gate already
present in Screen lock (~line 339-394).

Files:
- `web/src/components/HybridDashboardConfig.svelte` (sections
  ~328-436)
- `web/src/state/screensaverSettings.test.ts:153-156` pins the
  separate-section shape and will need to be updated.

Fix:
1. Move the screensaver theme picker (~lines 425-435) INTO the
   `{#if screensaverEnabled === true}` block inside `screen-lock`,
   right before the `{/if}` at ~line 394. Keep the existing
   sub-label / hint copy.
2. Delete the standalone `<section class="screensaver">` block
   entirely.
3. Update `screensaverSettings.test.ts` to assert the theme
   picker appears INSIDE the `{#if screensaverEnabled === true}`
   gate within `<section class="screen-lock">`, not as a separate
   section.

Acceptance: the Dashboard back-of-card has Appearance / Screen
lock / Metadata archive sections only. Toggling Screen lock OFF
hides both the timeout/PIN controls AND the screensaver theme
picker; toggling ON reveals both together.

### B4c. QR-donate broken-image in chan-desktop

User words: "the dashboard's qr-donate still not showing... have
you included the correct image, and is the href link correctly
set? we are getting that picture of a missing file (a square with
question mark inside)".

Diagnosis: `web/src/components/EmptyPaneCarousel.svelte:411`
hard-codes `<img class="fund-qr" src="/qr-donate.png" ...>`.
Plain `<img src>` BYPASSES the SPA's `apiPath()` rewrite that
prepends the `chan-prefix` meta-tag prefix; under chan-desktop
the SPA can be mounted at a non-root URL (per-window
`?w=<label>` plus the embedded server's prefix), and the raw
`/qr-donate.png` resolves to the wrong base URL. The earlier
empirical curl-test ran against a debug binary at root-mounted
`/`, which is why the broken-image regression only surfaces in
chan-desktop's mounting model.

Files:
- `web/src/components/EmptyPaneCarousel.svelte:411`
- `web/src/api/transport.ts` (`apiPath` ~77-81 + `withTokenQuery`
  ~100-105 — already exported; reuse, don't reimplement)
- Test pin (likely
  `dashboardTabAndCarousel.test.ts` "About widget embeds the
  donation QR" assertion at ~line 136-142) — relax the regex to
  match either the raw path OR the rewritten path.

Fix:
1. Import `withTokenQuery` from `../api/transport`.
2. Change the `<img>` `src` to `src={withTokenQuery("/qr-donate.png")}`.
   That covers the prefix rewrite + the per-launch bearer token.
3. Update the vitest pin so it matches the rewritten attribute
   (the existing pin asserts a literal string; relax to a regex
   that accepts the helper-wrapped form).

Acceptance: opening the Dashboard About slide under chan-desktop
shows the QR code at the rendered 160x160 size. Confirm
empirically on a chan-desktop launch (your merge-gate cycle owns
this verification before the release cut).

### B7. Graph: missing dblclick "graph from here" + dead depth slider

User words: "i'd like to be able to double click a node in the
graph to 'graph from here' automatically. also, the depth slider
(automatically expand all Nth degree directory nodes) is not
working at all".

Two related sub-issues:

#### B7a. Dblclick on a graph node = "graph from here"

Diagnosis: `web/src/components/GraphCanvas.svelte` has no
`ondblclick` handler today. The "graph from here" action is
`graphFromHere(path, isDir)` in
`web/src/components/GraphPanel.svelte:222`. The canvas needs to
expose an `onSetAsScope?: () => void` prop that GraphPanel binds
to `() => graphFromHere(selectedFsPath, selectedFsIsDir)`.

Fix shape:
1. Add `onSetAsScope?: () => void` to GraphCanvas's `$props()`
   destructure.
2. Add `ondblclick={onDoubleClick}` to the canvas element (~line
   1500-1508 area). The handler:
   ```ts
   function onDoubleClick(e: MouseEvent): void {
     if (!canvas) return;
     const p = localCoords(e);
     const n = pickNode(p.x, p.y, PICK_SLACK_CLICK_PX);
     if (n && onSetAsScope) onSetAsScope();
   }
   ```
3. In `GraphPanel.svelte` (mount of `<GraphCanvas ... />` ~line
   2220 area) pass `onSetAsScope={selectedId ? () =>
   graphFromHere(selectedFsPath, selectedFsIsDir) : undefined}`.
4. Vitest pin for the GraphCanvas onSetAsScope wiring.

Acceptance: double-clicking a node on the graph canvas rescopes
the graph (same as Inspector "Graph from here" button + the
right-click menu entry). Single-click behaviour (select + label
node + 1-hop neighbours) is unchanged.

#### B7b. Depth slider not working

Diagnosis: `web/src/components/GraphPanel.svelte` has the slider
markup at ~line 1905 bound to `graphState.depth`. The `loadKey`
derived (~1692-1694) includes `graphState.depth`; the `$effect`
(~1702-1707) re-fires `load()` when `loadKey` changes. So
mechanically the slider DOES drive a re-fetch when nothing else
is in the way.

But: a per-kind clamp effect (~1736-1744) clamps
`graphState.depth` to `depthCap` on every change. For language
mode the cap is pinned to 1 (slider sits at `[max]` and can't
move). For tag/contact mode the cap is hardMax. For PATH mode the
cap should let the user drive the depth. Either:
- The clamp is firing before the slider commits the user's value
  (write/read race), OR
- The `depthDisabled` flag the markup uses doesn't include
  `languageMode`, so the slider is bind-active but
  visually-disabled in language mode, OR
- The path-scope `loadKey` re-fetch is firing but the backend
  isn't expanding directories at the new depth (verify
  `merge_filesystem_layer` in
  `crates/chan-server/src/routes/graph.rs` actually consumes the
  `depth` parameter for the spine expansion).

Investigate the three possibilities and fix in one place. The
"depth slider does nothing in path-mode" observation is the
load-bearing failure; language/tag/contact pinning is intentional
per the round-1 roadmap.

Files:
- `web/src/components/GraphPanel.svelte` (~1692-1744, ~1905-1914)
- `web/src/graph/depth.ts` (the `graphDepthCap` helper)
- `crates/chan-server/src/routes/graph.rs::merge_filesystem_layer`
  (the spine-expansion logic)

Acceptance: in a path-scoped graph (workspace / dir / file
scope), moving the slider from `1` to higher values progressively
reveals more directory tiers; moving back collapses. Slider is
visibly disabled for kind=language (pinned to 1); for kind=tag /
kind=contact it stays at max.

### B8. Pane hamburger menu missing Search + Dashboard after Graph

User words: "the pane's hamburger menu must include 'Search Cmd+S'
and 'Dashboard Cmd+I' after 'Graph Cmd+Shift+M'".

Diagnosis: `web/src/components/Pane.svelte` defines two arrays:
- `spawnActions` (~177-214): the 5 rows rendered in the pane
  top-bar hamburger menu via the loop at ~1187.
- `emptyPaneExtraActions` (~216-233): Search + Dashboard rendered
  ONLY in the empty-pane right-click menu, NOT in the pane
  hamburger.

User wants Search + Dashboard in the pane hamburger AFTER Graph.

Files:
- `web/src/components/Pane.svelte` (`spawnActions` 177-214; render
  loop ~1187)
- `web/src/components/Pane.test.ts:137-177` (pinned menu labels;
  needs update)

Fix shape:
1. Add Search and Dashboard rows to `spawnActions` after the
   Graph row (icons `Search`, `BarChart2` already imported per
   the empty-pane extras).
2. Decide whether `emptyPaneExtraActions` still differentiates
   from `spawnActions` (currently they're identical after this
   fix) — if not, fold them into one shared array. Confer with
   the empty-pane menu render path to avoid double-render.
3. Update `Pane.test.ts:137-177` to expect the new 7-row pane
   hamburger.

Acceptance: opening the pane top-bar hamburger menu lists New
Draft / Terminal / File Browser / Rich Prompt / Graph / Search /
Dashboard with their chord hints, in that order, before the
nav/closing rows.

### B9. Empty-pane welcome surface: Search missing, Dashboard chord absent

User words: "the pane's background must include search, and
include the shortcut for the dashboard" + the attached image (5
top tiles New Draft / Terminal / File Browser / Rich Prompt /
Graph, divider, one Dashboard tile below with NO chord hint).

Diagnosis: `web/src/components/EmptyPaneWelcome.svelte` renders
two arrays:
- `spawnEntries` (~57-88): the top 5 tiles each rendering
  `{chordLabel(row.chordId)}` via the markup at ~lines 124-136.
- `secondaryEntries` (~89-99): the single Dashboard tile rendered
  via separate markup at ~lines 140-152, where the chord render is
  HARDCODED to an empty `<span class="spawn-chord"></span>` at
  line 150.

Search is missing entirely; Dashboard's Cmd+I doesn't render
because the secondary markup never asks for it.

Files:
- `web/src/components/EmptyPaneWelcome.svelte` (~19, 57-99,
  140-152, and the `.spawn-row-secondary` CSS ~252-254)
- `web/src/components/dashboardTabAndCarousel.test.ts` (~252-260
  pins the welcome-surface entries; needs update)

Fix shape:
1. Add `Search` to the lucide imports at the top of
   EmptyPaneWelcome.
2. Add a Search row to `secondaryEntries` BEFORE the Dashboard
   row (or, alternatively, AFTER — confirm with @@Alex on the
   left/right ordering; the user's image shows only one tile so
   either works visually).
3. Fix the secondary-tile chord render at line 150: replace the
   hardcoded empty span with
   `<span class="spawn-chord">{chordLabel(row.chordId)}</span>`,
   identical to the primary-row markup.
4. Update `.spawn-row-secondary` CSS from
   `grid-template-columns: minmax(120px, 240px)` to
   `grid-template-columns: repeat(2, minmax(96px, 1fr))` so two
   secondary tiles sit side by side.
5. Update the vitest pin to expect Search + Dashboard in
   secondary entries (or to remove the exclusive "only
   Infographics" pin).

Acceptance: the empty single-pane welcome surface shows 5 top
tiles + a divider + 2 secondary tiles (Search Cmd+S, Dashboard
Cmd+I). Each tile renders its chord hint. Right-click menu order
unchanged.

## Cross-lane

- Lane A owns A5 (workspace inspector language links) + A6
  (contact pills). They touch `WorkspaceInfoBody.svelte` only;
  their mount-site prop pass-throughs may touch your
  `EmptyPaneCarousel.svelte` at line 428 (a single-line
  `<WorkspaceInfoBody variant="dashboard" ... />` arg list edit).
  Declare any unexpected overlap on `event-lane-b-lane-a.md`
  before editing.
- Lane A reports on `event-lane-a-alex.md`; your merge-gate cycle
  must walk both `event-lane-a-alex.md` AND your own
  `event-lane-b-alex.md` channel tails before opening
  `../chan-integration` (per the round-1 lowlight you wrote up in
  `retrospective-round-1.md` — don't repeat that miss).

## Merge-gate role (your hat for the round)

You serialize merges to main. @@LaneA does NOT merge itself.

Per merge-ready (yours or Lane A's):

1. Read `event-lane-a-alex.md` and your own
   `event-lane-b-alex.md` for queued merge-ready slices. Walk
   BOTH tails, not just the last-noted status (round-1's missed-tail
   incident is the load-bearing reason this is in the brief).
2. Set up `../chan-integration` (`git worktree add
   ../chan-integration -b phase-13-integration main`).
3. Merge the lane branches in. File-disjoint slices auto-merge;
   flag conflicts back to the relevant lane.
4. Run the FULL combined-tree gate:
   `cargo fmt --check`, `cargo clippy --all-targets -- -D
   warnings`, `cargo test`, `cargo build --no-default-features`,
   `(cd web && npm install && npm run check && npm run build
   && npm test)`.
5. The indexer test
   `chan_workspace::indexer::tests::writes_to_drafts_subtree_get_indexed_under_drafts_prefix`
   is a known flake; re-run `cargo test -p chan-workspace --lib`
   once before flagging.
6. If green: fast-forward main, confirm on
   `event-lane-b-alex.md` with the merged SHA. Clean up
   `../chan-integration` between cycles.
7. If red: report the failure to the affected lane on
   `event-lane-b-lane-a.md`; do NOT merge.
8. NEVER `git push` (per `feedback_merge_is_not_push`).

## v0.17.0 release cut (after both lanes drain)

The chore(release): 0.17.0 commit ALREADY landed at `e30f73ef`;
your fixes from this closing wave ride on top of that. After both
lanes drain + main is green on the full combined-tree gate:

1. Verify version pins still read 0.17.0:
   - `Cargo.toml [workspace.package].version` = `0.17.0`
   - `Cargo.toml [workspace.dependencies]` chan-* entries =
     `version = "0.17.0"`
   - `desktop/src-tauri/tauri.conf.json` `"version"` = `"0.17.0"`
   - `Cargo.lock` refreshed (`cargo build` if any chan-* version
     touched)
2. Append a `docs(phase-13): close round 1 wave 2` commit (the
   retrospective update + the closing-2 docs from this round) per
   `feedback_coordination_docs_commit_timing`.
3. Push main and the closing-2 commits ONLY after explicit @@Alex
   confirmation - `feedback_merge_is_not_push`. Workflow_dispatch
   runs against the remote HEAD, so the push has to land first.
4. Dry-run via `gh workflow run release.yml -f publish=false`.
5. Inspect artifacts. If good, tag `v0.17.0` on main (annotated).
6. Push the tag ONLY after explicit @@Alex confirmation -
   `release.yml` fires on tag.
7. Verify `/dl/latest.json` supersedes 0.16.0; verify
   self-upgrade from 0.16.0 -> 0.17.0 in chan-desktop.

## Per-slice gate (mandatory before any "ready to merge")

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --no-default-features
(in web/)   npm run check
(in web/)   npm run build
(in web/)   npm test
```

Per `feedback_svelte_static_gate_misses_runtime`: B1c / B2c / B3c
/ B7 / B9 are reactive Svelte changes - browser-smoke them. Per
`feedback_terminal_webgl_wkwebview`: B2c + B4c are WKWebView-
shape; chan-desktop smoke before "ready to merge" on those,
NOT after (the round-1 retrospective's load-bearing lesson).

Append your own merge-ready entries to `event-lane-b-alex.md`:

```
ready to merge: phase-13-lane-b@<sha>  -  <one-line slice summary>
```

Append merge-gate confirmations after fast-forwarding main:

```
merged: phase-13-lane-{a,b}@<sha> -> main@<sha>  -  <one-line summary>
```

## Coordination rules

- Append-only channels.
- Each turn, before acting, read:
  - `event-alex-lane-b.md` (your inbox)
  - `event-lane-a-lane-b.md` (cross-lane from @@LaneA)
  - `event-lane-a-alex.md` (Lane A merge queue) — walk the FULL
    TAIL, not just the last-noted status.
- Self-document in `lane-b/journal.md`.
- DO NOT push to origin without explicit @@Alex ask
  (`feedback_merge_is_not_push`).
- Pre-release per `feedback_pre_release_no_backcompat`: drop
  legacy fields/formats outright; don't add migration shims.

## Out of scope

- Lane A's A5 + A6.
- Anything not in the 9-bug round-2 list. Escalate scope creep to
  @@Alex on `event-lane-b-alex.md`.

## First-turn checklist

1. Rebase `../chan-lane-b` on `main`.
2. Read the recovery files - including the
   `retrospective-round-1.md` Lane B self-feedback section.
3. Append a turn-1 opening entry to `lane-b/journal.md`.
4. Pick the smallest item (B1c is the cleanest 6-line fix, B9 is
   small UI). Work it to the per-slice gate.
5. Report on `event-lane-b-alex.md`.
6. Walk channel tails and run a combined-tree merge gate the
   moment Lane A queues their first slice (or yours, whichever
   comes first).
