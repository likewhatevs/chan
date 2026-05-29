# @@LaneB round-1 closing - Phase 13

You are @@LaneB picking up the round-1 CLOSING tasks AND the
merge-gate orchestrator role. The round-1 roadmap landed
end-to-end (the prior @@LaneB's Pane bugs + KIND lenses 2a /
2b + Dashboard slices 3a / 3b-1 / 3c / 3b-2 are all on main at
`5a241f0f`). The cumulative chan-desktop smoke surfaced
regressions; this file IS your task list.

You DO cut v0.17.0 once both lanes drain. You DO merge-gate
both lanes (combined-tree re-gate + fast-forward to main per
`reference_release_cut_mechanics`).

## Recover context (read in order)

- `/Users/fiorix/dev/github.com/fiorix/chan/CLAUDE.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/design.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/roadmap-round-1.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/bootstrap.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/round-1-closing-tests.md`  THE SMOKE REPORT
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/README.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-b-request.md` (your original brief; channel + worktree + merge-gate convention)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-b/journal.md` (tail of the prior @@LaneB's turns; substantial)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-alex-lane-b.md` (your inbox)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-lane-a-alex.md` (Lane A's merge queue)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-lane-a-lane-b.md` (cross-lane from @@LaneA)

## Worktree + branch

You inherit `../chan-lane-b` on branch `phase-13-lane-b`. First
action: rebase on the current main tip.

```
git -C /Users/fiorix/dev/github.com/fiorix/chan-lane-b fetch . main
git -C /Users/fiorix/dev/github.com/fiorix/chan-lane-b rebase main
```

Combined-tree re-gate runs in `../chan-integration` (create
per merge-gate; the prior @@LaneB cleaned it up after each
cycle). Release cut runs in the MAIN checkout
(`/Users/fiorix/dev/github.com/fiorix/chan`).

Source code work happens ONLY in `../chan-lane-b` (your own
fixes) and `../chan-integration` (during gate runs). Journals /
channels stay in the MAIN checkout edited by ABSOLUTE PATH.

## Scope (your round-1 closing items)

User words quoted verbatim where available; file pointers cite
the prior smoke investigation.

### B1. Tab title `kind=` prefix dropped when a node is selected

User words: "Does not show path=<dir> in the tab's name, just
<dir> without path=".

Root cause: `web/src/state/tabs.svelte.ts::graphTabLabel`
returns `selectedNodeLabel` (the clicked node's bare label)
when set, dropping the kind-prefixed `tab.title`.

Files:
- `web/src/state/tabs.svelte.ts` (graphTabLabel ~line 519,
  graphTitle ~line 1239).

Acceptance: tab strip shows the kind prefix (`path=` / `tag=`
/ `contact=` / `lang=`) AT ALL TIMES, including when a node
is selected. Cleanest path: prepend the prefix to
`selectedNodeLabel`, like `${prefix}=${selected}` where the
prefix is parsed from `tab.title` (everything before the first
`=`). Add vitest cases pinning the new shape.

### B2. Cmd+, second-press doesn't flip back to front

User words: "Cmd+, again on each -> flips back to front. Not
working".

Root cause unconfirmed; the keymap path is sound on paper
(`web/src/App.svelte:673` calls `flipHybrid` which toggles
`node.showingBack`). Possible causes:
1. Focus capture on the back surface (an input/select stealing
   the keydown).
2. `e.preventDefault()` happening after a child handler ate
   the event.
3. A double-fire path (e.g., `chan:command` dispatched in
   parallel) toggling twice.

Files:
- `web/src/App.svelte` (onWindowKey ~line 673).
- `web/src/state/tabs.svelte.ts` (flipHybrid ~line 3173).
- `web/src/components/HybridTerminalConfig.svelte`,
  `HybridEditorConfig.svelte`, `HybridGraphConfig.svelte`,
  `HybridFileBrowserConfig.svelte` (no obvious stopPropagation
  in any per the prior investigation).

Acceptance:
- Live-launch chan-desktop, add a `console.log` to the keymap
  to confirm the second press reaches the handler.
- Fix the actual cause (focus capture, double-fire, etc.).
- Cmd+, on Terminal / Editor / Graph / FB toggles cleanly:
  press 1 -> back, press 2 -> front, press 3 -> back, ...

### B3. Cmd+, on Dashboard shows empty back-of-card

User words: "Cmd+, on Dashboard back-of-card shows empty
window; right-click and Settings works, but clicking again
does not flip it back, only OK does".

Root cause: `web/src/components/Pane.svelte`'s back-of-card
switch (`{#if pane.showingBack ... {#if active?.kind === "terminal"} ...`,
around line 1294-1311) has arms for `terminal` / `file` /
`graph` / `browser` but NOT `dashboard`. The Dashboard tab
falls through to the "Empty pane" fallback. The
DashboardTab's own right-click "Settings" path uses a
separate component-local `settingsOpen` state that DOES
render the back content - that's why right-click works and
Cmd+, doesn't.

Fix path:
1. Extract the Dashboard back-of-card from
   `DashboardTab.svelte` (the
   `{#if settingsOpen}<HybridSurfaceConfigShell ... />{:else}<EmptyPaneCarousel />`
   block) into a new
   `web/src/components/HybridDashboardConfig.svelte` mirroring
   the existing `HybridTerminalConfig` / `HybridEditorConfig`
   / `HybridGraphConfig` / `HybridFileBrowserConfig` shape.
2. Move the Appearance / Screen Lock / Screensaver / Metadata
   archive sections + their state (`screensaverEnabled`,
   `screensaverTimeoutSecs`, `screensaverTheme`, `screensaverPinSet`,
   `metadataImportFile`, etc.) into the new component. Call
   `loadScreenLockState()` on mount so the back surface
   hydrates correctly on every flip.
3. Add the `dashboard` arm in `Pane.svelte`'s back-of-card
   switch:
   ```svelte
   {:else if active?.kind === "dashboard"}
     <HybridDashboardConfig onDone={() => flipHybrid(pane.id)} />
   ```
4. Drop the right-click "Settings" entry from DashboardTab's
   hamburger (now redundant - Cmd+, is the canonical flip,
   per @@Alex's call on this round's planning).
5. The DashboardTab front continues to render
   `<EmptyPaneCarousel />` unconditionally.

Acceptance: Cmd+, on a Dashboard tab flips to a back
showing Appearance + Screen Lock + Screensaver + Metadata
archive + OK; pressing Cmd+, again flips back to front
(ties into B2's fix).

### B4. Cmd+, on empty pane visually flips the pane

User words: "Cmd+, on empty pane flips the whole pane".

Root cause: `flipHybrid` has no empty-pane guard; the
flip animation fires even when there's no surface to flip.

File: `web/src/state/tabs.svelte.ts::flipHybrid` (~line 3173).

Fix:
```ts
export function flipHybrid(paneId: string): void {
  const node = activeLayout().nodes[paneId];
  if (!node || node.kind !== "leaf") return;
  if (node.tabs.length === 0) return;  // NEW empty-pane guard
  if (!node.back) node.back = {};
  node.showingBack = !node.showingBack;
  requestPaneFlip(node.id);
}
```

Acceptance: Cmd+, on an empty pane is a no-op (no animation,
no state change). Vitest case for the guard.

### B5. Empty-pane menu + welcome menu say "Infographics"

User words: "missing the 'Dashboard' option which should be
after Graph here" + "the main button still says Infographics,
it should say Dashboard".

User-visible labels to rename `"Infographics"` -> `"Dashboard"`:

| File                                              | Anchor          |
|---------------------------------------------------|-----------------|
| `web/src/components/Pane.svelte`                  | ~line 227 (emptyPaneExtraActions label) |
| `web/src/components/EmptyPaneWelcome.svelte`      | ~line 91 (secondaryEntries label)       |
| `web/src/state/shortcuts.ts`                      | (label or description) |
| `web/src/state/tabs.svelte.ts`                    | ~line 2813 (`title: "Infographics"`) on `openDashboardInPane` default |

Per the user's order request "missing the 'Dashboard' option
which should be after Graph here" the menu order in both
`Pane.svelte`'s emptyPaneExtraActions and
`EmptyPaneWelcome.svelte`'s secondaryEntries must be:
- New Draft / Terminal / File Browser / Rich Prompt / Graph /
  **Dashboard** / Search

Currently Dashboard sits after Search; move it between Graph
and Search.

Update the test fixtures
(`web/src/components/Pane.test.ts:251`, the
`dashboardTabAndCarousel.test.ts` assertions) to match.

Acceptance: every user-visible reference to "Infographics" is
gone; menu order has Dashboard after Graph.

### B6. DashboardTab aria-label / title still say "Infographics"

Files:
- `web/src/components/DashboardTab.svelte` aria-label (~line
  324), `title` prop on HybridSurfaceConfigShell (~line 347),
  `ariaLabel` prop (~line 349).
- The new `HybridDashboardConfig.svelte` (B3) should use
  `"Dashboard"` for its title + ariaLabel from the start.

Acceptance: the back-of-card header reads "Dashboard".

### B7. QR-donate not loading

User words: "the qr-donate is not showing; we must embed this
image in our server".

Root cause hypothesis: `web/public/qr-donate.png` exists but
`crates/chan-server/src/static_assets.rs` (or wherever
rust-embed wires the SPA bundle) may not include the file in
the embed. Verify the asset is served at `/qr-donate.png` from
chan-desktop's launched server.

Files:
- `crates/chan-server/src/static_assets.rs` or the equivalent
  rust-embed mount.
- `web/src/components/EmptyPaneCarousel.svelte` (the About
  slide's `<img src="/qr-donate.png" ... />`).

Acceptance: opening the Dashboard About slide renders the QR
code at 160x160 (or whatever size the prior @@LaneB picked).
Test via chan-desktop dev launch.

### B8. Tag lens graph empty

User words: "tags do show in the tab name, but nothing comes
up in the graph; I'd expect to see the backlinks to the
documents".

Root cause: the tag BFS in
`web/src/components/GraphPanel.svelte::computeScopedNodeSet`
(~line 654-672) is FORWARD-ONLY:

```ts
for (const e of edges) {
  if (frontier.has(e.source) && !visited.has(e.target)) {
    next.add(e.target); visited.add(e.target);
  }
}
```

Backend edges (`crates/chan-server/src/routes/graph.rs`) emit
tag edges with `source: <file>, target: <tagId>` - direction
file -> tag. Forward-only BFS from the tag id finds no
outgoing edges. Empty lens.

Fix: make the tag arm bidirectional (mirror the contact arm
~line 682-706). Add the reverse check:

```ts
if (frontier.has(e.target) && !visited.has(e.source)) {
  next.add(e.source); visited.add(e.source);
}
```

Acceptance: tag chip click opens a graph showing the tag node
+ every doc that references it + (with depth>1) those docs'
outgoing edges. Vitest unit for the BFS.

### B9. Language lens shows only 1 directory

User words: "clicked on Markdown, it is only showing 1
directory (out of MANY that we have markdown on), and the
language itself has no Inspector" (image-7.png).

(The inspector concern is Lane A's A3.)

Root cause (per the prior smoke investigation): the workspace
graph payload from `/api/graph?scope=workspace` includes
language bubble + edges, but the language layer may be
incomplete or the BFS only finds 1-hop neighbors. The
language layer's edges (file -> language node) may not all
emit when `scope=workspace`.

Investigate:
1. Does `crates/chan-server/src/routes/graph.rs::api_graph`
   emit a `language:<lang>` node + edge per file-of-that-lang
   when `?scope=workspace`? Or are language nodes only
   emitted by `/api/graph/languages`?
2. If the language layer is only in `/api/graph/languages`,
   the GraphPanel needs to fetch BOTH endpoints when the
   scope is `language:<lang>` and merge the responses (or the
   backend needs to include the layer in the workspace
   response by default).

File: `web/src/components/GraphPanel.svelte` (language arm in
computeScopedNodeSet ~line 712-720; data-load path ~line 1413
`api.graphStream(...)`).

Acceptance: language chip click opens a graph showing the
language bubble + every file of that language splayed as
1-hop neighbors. Depth slider is hidden / pinned at 1 per the
roadmap.

### B10. Source Code Pro toggle moves to Terminal back-of-card

User words: "How to enable / disable Source Code Pro? imho
this should be a setting in the back of the TERMINAL because
it's a terminal setting".

Files:
- The Dashboard About slide in
  `web/src/components/EmptyPaneCarousel.svelte` currently
  shows Source Code Pro attribution. Keep the attribution
  there (license credit), but the on/off TOGGLE moves to
  `web/src/components/HybridTerminalConfig.svelte`.
- Whatever preference key drives the terminal font (likely in
  `web/src/state/preferences.svelte.ts` or
  `web/src/terminal/`). Wire the toggle.

Acceptance: Terminal back-of-card has a Source Code Pro
on/off control; off falls back to the system monospace font.
The Dashboard About slide keeps the attribution but drops
any control if there was one.

### B11. Reload Cmd+R on every pane-top-bar right-click

User words: "We must add the Reload Cmd+R especially for
widgets like the workspace and the graph in case users want
to refresh" + "the right-click on pane top bar".

Scope: the Reload is the SAME window reload that Cmd+R fires
today (the chan-desktop KEY_BRIDGE_JS at `case 'KeyR':
invokeIpc(e, 'reload_window')` path). It lives on the right-
click menu that opens from the pane top bar - i.e. on EVERY
pane (Terminal / Editor / Graph / FB / Dashboard), not
Dashboard-only. The menu row reads "Reload" with the Cmd+R
chord rendered alongside.

Files:
- `web/src/components/Pane.svelte` - find the pane-top-bar
  right-click menu (the HamburgerMenu opened by the top-bar
  context menu handler; search for `oncontextmenu` on the
  tab strip / top bar). Add a Reload entry to the canonical
  section (probably the navigation / pane-controls section).
- Same right-click menu on `DashboardTab.svelte` - it owns
  its own HamburgerMenu (~line 131) since it's a Hybrid
  surface; ensure parity with the Pane.svelte version OR
  consolidate to a shared menu fragment.
- The Reload action dispatches via the chan command bus
  (`dispatchCommand("app.reload_window")` or equivalent -
  check `web/src/App.svelte::runCommand` for the canonical
  id; the desktop KEY_BRIDGE_JS already uses
  `invokeIpc(e, 'reload_window')` which Tauri's IPC handles
  natively).

Acceptance: right-click on the pane top bar of every pane
shows a Reload entry with the Cmd+R chord visible; clicking
it reloads the SPA window (same effect as pressing Cmd+R on
the keyboard).

### B12. Indexing graph default zoom + click labels

User words:
- "When we click a node, we should show the label of that
  node and all immediate siblings / 1st degree connections".
- "Ideally the graph should maximise the use of the viewport
  by default, like a decent zoom showing the entire spine.
  the default here is too small".

Files:
- `web/src/components/GraphCanvas.svelte` - fit-to-viewport
  logic + selection-based labeling.
- The indexing slide in
  `web/src/components/EmptyPaneCarousel.svelte` (just mounts
  GraphCanvas with focalIds = [""]; the layout fit + label
  behavior is GraphCanvas's responsibility).

Acceptance:
- Indexing graph: on initial render, the layout fits the
  entire spine inside the viewport with a decent zoom (no
  manual zoom needed).
- Clicking a node displays the label of THAT node + its
  immediate 1-hop neighbors (siblings + parent + children),
  matching the main graph's selection-labeling rule.

GraphCanvas already exposes some of this for non-read-only
mode; verify the read-only path inherits it.

## Cross-lane

- B8 (tag lens BFS bidirectional) is purely Lane B; ship
  independently.
- B9 (language lens) may need backend changes to the workspace
  graph payload; coordinate with Lane A if the language
  layer's emission overlaps with their mention-resolver work
  (A5).
- B3's HybridDashboardConfig houses Appearance + Screen Lock
  + Screensaver + Metadata. The Screen Lock + Screensaver
  state plumbing is shared with `lockNow()` / `loadScreenLockState()`
  helpers in `web/src/state/`; reuse, don't reimplement.

## Merge-gate role (your hat for the round)

You serialize merges to main. @@LaneA does NOT merge itself.

Per merge-ready (yours or Lane A's):

1. Read `event-lane-a-alex.md` and your own
   `event-lane-b-alex.md` for queued merge-ready slices.
2. Set up `../chan-integration` (`git worktree add
   ../chan-integration -b phase-13-integration main`).
3. Merge the lane branches in. File-disjoint slices auto-
   merge; flag conflicts back to the relevant lane.
4. Run the FULL combined-tree gate:
   `cargo fmt --check`, `cargo clippy --all-targets -- -D
   warnings`, `cargo test`, `cargo build --no-default-features`,
   `(cd web && npm install && npm run check && npm run build
   && npm test)`.
5. The indexer test
   `chan_workspace::indexer::tests::writes_to_drafts_subtree_get_indexed_under_drafts_prefix`
   is a known flake; re-run once before flagging.
6. If green: fast-forward main, confirm on
   `event-lane-b-alex.md` with the merged SHA. Clean up
   `../chan-integration` between cycles.
7. If red: report the failure to the affected lane on
   `event-lane-b-lane-a.md`; do NOT merge.
8. NEVER `git push` (per `feedback_merge_is_not_push`).

## v0.17.0 release cut (round close)

After both lanes drain and main is green on the full gate:

1. Bump version in:
   - `Cargo.toml [workspace.package].version` -> `0.17.0`
   - `desktop/src-tauri/tauri.conf.json` -> `"version": "0.17.0"`
   - `Cargo.lock` (refreshed by `cargo build`)
2. Dry-run via `gh workflow run release.yml -f publish=false`.
3. Inspect artifacts; if good, tag `v0.17.0` on main
   (annotated tag).
4. Push the tag ONLY after explicit @@Alex confirmation -
   `release.yml` fires on tag.
5. Verify `/dl/latest.json` supersedes 0.16.0; verify
   self-upgrade from 0.16.0 -> 0.17.0 in chan-desktop.
6. Commit phase-13 docs / journals / channels to main as
   `docs(phase-13): close round 1` per
   `feedback_coordination_docs_commit_timing`.

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

`feedback_svelte_static_gate_misses_runtime`: B1 + B2 + B3
+ B8 + B12 are reactive Svelte changes - browser-smoke them.
`feedback_terminal_webgl_wkwebview`: B2 + B10 touch terminal
rendering; chan-desktop smoke before reporting ready-to-merge
on those.

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
  - `event-lane-a-alex.md` (Lane A merge queue)
- Self-document in `lane-b/journal.md`.
- DO NOT push to origin without explicit @@Alex ask.

## Out of scope

- Anything not in the smoke report. Escalate scope creep to
  @@Alex on `event-lane-b-alex.md`.
- Lane A's items (A1-A5). They land separately.

## First-turn checklist

1. Rebase `../chan-lane-b` on `main`.
2. Read the recovery files.
3. Append a turn-1 opening entry to `lane-b/journal.md`.
4. Pick the smallest item (B1, B4, or B5 are the smallest).
   Work it to the per-slice gate.
5. Report on `event-lane-b-alex.md`.
6. Run a combined-tree merge gate when Lane A queues their
   first slice.
