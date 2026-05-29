# @@LaneB -> @@Alex

Append-only. @@LaneB writes progress, merge-gate confirmations, and the
v0.17.0 release cut here. Most recent entry at the bottom.

Merge-ready format (Lane B's own slices):

```
ready to merge: phase-13-lane-b@<sha>  -  <one-line slice summary>
```

Merge-gate confirmation format (after re-gating + fast-forward to main):

```
merged: phase-13-lane-{a,b}@<sha> -> main@<sha>  -  <one-line summary>
```

## 2026-05-28 14:00 @@LaneB -> @@Alex
Turn 1: worktree + KIND signature posted; first slice = pane bugs

**Highlights**
- Phase-13 worktree `../chan-lane-b` on `phase-13-lane-b` created
  off `b2caa7e9 chore(release): 0.16.0`. Stale phase-12 worktree
  removed (clean).
- KIND route signature posted on
  `event-lane-b-lane-a.md`. Decision: extend `/api/graph` with a
  `?kind=` discriminator (backward-compatible default = `path`);
  response shape unchanged. Lane A wires through new/existing
  `openGraphFor{Tag,Contact,Language}` helpers - they do NOT hit
  the route directly. Lane A is unblocked for kind-chip plumbing.

**Lowlights**
- None this turn.

**Contention**
- None this turn. Cross-lane note flagged: Lane B's Dashboard
  `WorkspaceInfoBody` widget reuses Lane A's workspace-root parity
  slice; will sync on `event-lane-b-lane-a.md` when Lane B's
  Dashboard slice approaches it. No action required from you.

**Next slice**
Pane bugs (focus-ring thickness parity with top-bar + pane CSS hover
wobble on mouse enter/leave AND keyboard pane switch). File-disjoint
from KIND backend so I'll line up a KIND-backend subagent in
parallel. Browser smoke + chan-desktop smoke per
`feedback_svelte_static_gate_misses_runtime` +
`feedback_terminal_webgl_wkwebview`.

Detail in `lane-b/journal.md`.

## 2026-05-28 14:11 @@LaneB -> @@Alex
ready to merge: phase-13-lane-b@ea23a691 - pane bugs (focus ring + wobble)

**Highlights**
- Pane bugs slice landed in 4 commits on `phase-13-lane-b`:
  - `33e180c9` first attempt (thickness only, kept ::before chrome).
  - `975efe40` pivot after @@Alex's screenshot - single-source 1px
    border-only focus ring, outer-halo wobble via box-shadow (no
    transform on .pane → xterm WebGL safe).
  - `a5c589b5` updated `contextMenuChrome.test.ts` for the new
    chrome shape.
  - `ea23a691` bounded the xterm-safety regex to single CSS rules.
- chan-desktop smoke confirmed both bugs fixed (Alex live-watched).
  Focus ring reads uniformly 1px around the focused pane; wobble
  visibly pulses on pane-switch.

## 2026-05-28 14:11 @@LaneB -> @@Alex
merged: phase-13-lane-a@ad184179 + phase-13-lane-b@ea23a691 -> main@f97146cc - combined-tree green

**Highlights**
- Lane A's two slices (`b2ef3f3b` bugs 1-4, `ad184179` inspector
  parity) integrated with Lane B's pane bugs slice on
  `../chan-integration` worktree. Auto-merge in `tabs.svelte.ts`
  (Lane A at line 2044, Lane B at line 3139) was semantically clean
  - file-disjoint within the file.
- Combined-tree gate green: cargo fmt / clippy -D warnings / test
  (540 + 347 + ... all green, indexer flake didn't reappear) /
  build --no-default-features / npm check / npm build / vitest
  (1615 passed, 11 skipped).
- Fast-forwarded main from `b2caa7e9` -> `f97146cc` (merge commit
  on top of lane-a's linear tip + lane-b's four commits).

**Lowlights**
- No git push performed (per `feedback_merge_is_not_push`).
- No browser smoke yet on Lane A's slices (their report calls this
  out; they planned to batch after my merge gate). Lane A may want
  to walk the now-integrated tree.

**Contention**
- None on the merge. KIND chip wiring still gated on Lane B's
  KIND signature being implemented in code; the signature is
  posted, the code is the next Lane B slice.

**Next Lane B slice**
KIND backend extension on `/api/graph?kind=` + new
`openGraphFor{Contact,Language}` helpers in `store.svelte.ts`.
This unblocks Lane A's KIND chip implementation work.

Detail in `lane-b/journal.md`.

## 2026-05-28 14:30 @@LaneB -> @@Alex
ready+merged: phase-13-lane-b@24f1f31d -> main@24f1f31d - KIND slice 2a (helpers + title prefix)

**Highlights**
- KIND slice 2a landed: `openGraphForContact(relPath)` and
  `openGraphForLanguage(language)` exported from
  `web/src/state/store.svelte.ts`; `graphTitle()` now prefixes
  every graph tab with `kind=` (`path=` / `tag=` / `contact=` /
  `lang=`) so the tab strip surfaces the lens shape next to the
  payload.
- Lane A is unblocked for Inspector kind-chip wiring. Pinged on
  `event-lane-b-lane-a.md`.
- Solo merge gate: lane-b's pre-commit gates were the combined
  gate (no other lane queued anything new). Fast-forwarded main
  from `f97146cc` -> `24f1f31d`. No push (you gate that).
- vitest 1616 passed (+1 above prior baseline thanks to the new
  graphTitle assertions for the language: arm and contact:
  workspace-relative paths).

**Lowlights**
- GraphPanel.synthesizeScope still returns null for `contact:` /
  `language:` scopeIds; the panel currently falls back to
  workspace-graph rendering for those lenses. Slice 2b will add
  the ScopeOption kinds + the BFS-from-center filter so the lens
  semantics actually render (contact = subgraph centered on the
  contact file with backlinks; language = bubble + edges to
  every file of that language).

**Contention**
- None.

**Next slice (2b)**
Frontend BFS lens for `contact:` and `language:`. Mirrors the
existing tag-lens path in `GraphPanel.svelte` (seed from the
center node, expand outward). No backend changes needed — the
existing `/api/graph?scope=workspace` payload already contains
the necessary edges; the centering happens client-side, same as
tag does today.

Detail in `lane-b/journal.md`.

## 2026-05-28 15:00 @@LaneB -> @@Alex
ready+merged: phase-13-lane-b@11e5fb37 + phase-13-lane-a@39fd3373 -> main@7c936504

**Highlights (Lane B work)**
- KIND slice 2b landed: contact + language lens semantics. Contact
  uses bidirectional BFS from the contact's file node (captures
  backlinks per the roadmap); language always 1-hop from the
  bubble. graphDepthCap learns the two new kinds (contact
  uses hardMax, language pins to 1 → slider reads `[max]`).
  scope header dropdown gets contact/language icons + labels.
- KIND chip wiring from Lane A (slice 4a, `39fd3373`):
  `openGraphForTag` already-existing path + tag chips made
  clickable via the new KindChip `onClick` API; FileInfoBody +
  TagInfoBody updated. The contact + language wiring (Lane A's
  slice 4b) was idle pending my 2a; the helpers are now live,
  so I've pinged them on `event-lane-b-lane-a.md`.

**Highlights (merge gate)**
- Combined-tree gate on `../chan-integration`: cargo fmt /
  clippy -D warnings / test (indexer flake reappeared once,
  passed on re-run per `feedback_fresh_binary_rewalks`) / build
  --no-default-features / npm check / npm build / vitest 1619
  passed (+1 above prior baseline from Lane A's 3 new KindChip
  tests; -1 net from my 1616 baseline since 2b didn't add tests).
- Fast-forwarded main `24f1f31d` -> `7c936504`. No push.

**Lowlights**
- No browser smoke yet on the KIND lens render path. Per
  `feedback_svelte_static_gate_misses_runtime` the Svelte
  $derived chain in GraphPanel (synthesizeScope + focalIds +
  computeScopedNodeSet) deserves a Chrome walk; per
  `feedback_terminal_webgl_wkwebview` Lane A's bug 4 walk wants
  chan-desktop. Will batch with the Dashboard slice's smoke.

**Contention**
- None on the merge.

**Next slice (3)**
Dashboard slice: InfographicsTab → DashboardTab rename, carousel
auto-resize, About widget (move from SettingsPanel + qr-donate +
links), workspace-info widget reusing Lane A's WorkspaceInfoBody,
search-index graph widget (read-only spine-only via the new
read-only graph mode), Settings flip-back (Appearance / Screen
Lock / Screensaver), Cmd+, rebind, SettingsPanel OverlayShell
retirement. Substantial — likely 2-3 sub-slices.

Detail in `lane-b/journal.md`.

## 2026-05-28 15:30 @@LaneB -> @@Alex
ready+merged: phase-13-lane-a@08b28da8 + phase-13-lane-b@fa8c0c25 -> main@11bf6b19

**Highlights**
- Lane A's slice 4b (contact + language KindChip wiring, completes
  the KIND chip roadmap end-to-end) landed. Their lane is now
  fully drained per their own report.
- Lane B's slice 3a (InfographicsTab -> DashboardTab rename;
  internal-only, user-visible labels still read "Infographics"
  until 3b's widget rework) landed alongside.
- Combined-tree gate green on `../chan-integration`: cargo fmt /
  clippy -D warnings / test (clean this round, no indexer flake)
  / build --no-default-features / npm check (4113 files) / npm
  build / vitest 1625 passed / 11 skipped (+6 above prior
  baseline from Lane A's `fileInfoBodyKindWiringSlice4b.test.ts`).
- Fast-forwarded main `7c936504` -> `11bf6b19`. No push.

**Lowlights**
- Cumulative browser-smoke debt: KIND chip click → graph lens
  render, language row buttons, contact pill lens fallback. Per
  `feedback_svelte_static_gate_misses_runtime` + Lane A's note,
  a Chrome walk is overdue. I'll batch with the Dashboard slice
  smoke before release cut, or pick it up sooner if you'd like.

**Contention**
- None.

**Next Lane B slice (3b)**
Dashboard widget rework: carousel auto-resize, About widget
(move from SettingsPanel + qr-donate + website/repo links + the
"fund this" text from web-marketing), Workspace-info widget
(reuses Lane A's WorkspaceInfoBody now on main), Search-index
graph widget (read-only spine-only mode of the slice 2b graph).
After 3b: 3c lands Settings flip-back + Cmd+, rebind +
SettingsPanel retirement.

Detail in `lane-b/journal.md`.

## 2026-05-28 15:50 @@LaneB -> @@Alex
ready+merged: phase-13-lane-b@b84c1507 -> main@b84c1507 - Dashboard widgets (About + Workspace)

**Highlights**
- Dashboard slice 3b-1 landed: carousel slide 0 (Shortcuts ->
  About with version + attributions + qr-donate + "Fund the
  work" copy + chan.app/github.com icon-links), slide 1
  (Metadata stacked-bar -> `WorkspaceInfoBody` reuse from Lane
  A's parity slice), carousel auto-resize via flex + min-height:0
  + slide-level `overflow-y: auto`.
- Solo merge (no other lane has anything queued). Fast-forwarded
  main `11bf6b19` -> `b84c1507`. No push.
- vitest 1632 passed (+7 from the new About + Workspace + resize
  property assertions); cargo + npm gates green.

**Lowlights**
- Slide 2 (indexing graph) unchanged. The roadmap calls for the
  read-only-spine GraphPanel rework; that's slice 3b-2 (deferred
  to after 3c).
- SettingsPanel.svelte's `<section class="about">` still
  present alongside the new dashboard About slide. Retirement
  happens in 3c when the OverlayShell goes away.
- Cumulative browser smoke debt now spans: KIND chip render
  walk, Dashboard carousel widget reactivity, plus the
  pre-existing pane wobble desktop verify (which IS done from
  turn 1 - keeping that one separate).

**Contention**
- None.

**Next Lane B slice (3c)**
Settings flip-back + Cmd+, rebind + SettingsPanel retirement.
After 3c the SettingsPanel.svelte file goes away; Cmd+, will
flip the focused Terminal/Editor/Graph/FB/Dashboard. After 3c
I'll either land 3b-2 (read-only graph for indexing slide) or
mark it as round-2 carryover depending on remaining bandwidth.

Detail in `lane-b/journal.md`.

## 2026-05-28 16:15 @@LaneB -> @@Alex
ready+merged: phase-13-lane-b@0bb01492 -> main@0bb01492 - Dashboard slice 3c (Settings flip-back + Cmd+, rebind + SettingsPanel retired)

**Highlights**
- Settings is no longer an overlay. The SettingsPanel.svelte
  file (920 lines) is gone; its Appearance / Screen Lock /
  Screensaver content moved to the DashboardTab back-of-card
  (above the existing Metadata archive section), all reusing
  the original state plumbing.
- Cmd+, rebound: command id `app.settings.toggle` stays stable
  (chan-desktop's KEY_BRIDGE_JS keeps firing it) but the
  behaviour is now `flipHybrid(layout.activePaneId)` on a
  focused Terminal/Editor/Graph/FB/Dashboard. Empty-pane
  hamburger lost its Settings footer entry (it would have been
  a no-op on an empty pane).
- `git grep "SettingsPanel" -- web/` returns zero. The
  `HASH_SETTINGS` URL hash key dropped per
  `feedback_pre_release_no_backcompat`; legacy bookmarks
  degrade via `dropUnknownHashKeys`.
- Static gates green: cargo fmt / clippy -D warnings / test /
  build --no-default-features / npm check (4110 files, 0 err) /
  npm build / vitest 1611 passed / 11 skipped. The -21 vitest
  delta from the prior 1632 baseline is the SettingsPanel test
  deletions; equivalent functionality re-asserted in the
  migrated tests.

**Lowlights**
- No browser / desktop smoke run on the round's accumulated
  changes since pane bugs. The Cmd+, rebind is the biggest UX
  surface area for runtime verification:
  - Cmd+, on focused Terminal/Editor/Graph/FB → flips to its
    existing back-of-card (HybridTerminalConfig etc.). All
    already existed; should be transparent.
  - Cmd+, on Dashboard → flips to the new
    Appearance/ScreenLock/Screensaver/Metadata flip-back.
  - On an empty pane → no-op (no surface to flip).
- SETTINGS_DISABLED is still emitted by chan-server as an SPA
  meta tag. Frontend dropped the import; the server-side
  cleanup is round-2 carryover.

**Contention**
- None.

**Decision point**
Remaining Lane B round-1 carryover:
1. **Slice 3b-2**: rework the indexing-graph carousel slide to
   use the new GraphPanel (slice 2b) in read-only spine-only
   mode. Substantial refactor (extracting a read-only render
   path from GraphPanel + spine-only filter). 0.5-1.5 hr of
   work.
2. **Cumulative smoke**: KIND lens render walk + Cmd+,
   behaviour + Dashboard back-of-card per surface. ~15 min if
   nothing's broken; longer if regressions surface.
3. **v0.17.0 release cut**: tag + dry-run release.yml.

If you want me to keep coding, I'd land 3b-2 next then ping you
for the smoke + release cut. If you'd rather drive the smoke
yourself now (especially the Cmd+, behaviour while it's fresh),
I can park and resume after.

Detail in `lane-b/journal.md`.

## 2026-05-28 16:35 @@LaneB -> @@Alex
ready+merged: phase-13-lane-b@5a241f0f -> main@5a241f0f - Dashboard slice 3b-2 (read-only graph for indexing slide)

**Highlights**
- Slice 3b-2 landed: the carousel's indexing slide now uses
  GraphCanvas (the same renderer the main Graph tab uses)
  instead of the bespoke SVG radial-tree. Spine-only,
  directory-only, depth=max from workspace root. -259 LOC net
  (subagent killed ~325 LOC of bespoke SVG / pan / zoom /
  recenter logic).
- `indexState` field added to `GraphViewNode`'s folder arm
  (optional; only the dashboard indexing slide feeds it).
  GraphCanvas applies indexing colour overrides (green =
  indexed, orange = indexing, grey = pending) ahead of the
  Drafts tint and the regular kind cascade. Ghost still wins.
- Indexing pulse via alpha modulation in the rAF paint loop;
  `prefers-reduced-motion: reduce` users get a flat
  mid-strength alpha so the indexing colour still reads as
  distinct without animating.
- Solo merge gate (no Lane A queue): fast-forwarded
  `0bb01492` -> `5a241f0f`. No push.
- vitest 1603 passed / 11 skipped. -8 from the prior 1611
  baseline due to the 8 deleted SVG-pinning tests; the
  GraphCanvas tests already cover the new code path.

**Lane B round-1 roadmap status: COMPLETE**
- Bugs 1-2 (focus ring + wobble): done, smoked.
- Graph KINDS (2a frontend + 2b lens): done.
- Dashboard (3a rename + 3b-1 About/Workspace + 3b-2
  indexing graph + 3c Settings flip-back/Cmd+,/SettingsPanel
  retirement): done.

**Lowlights**
- Cumulative chan-desktop smoke debt covers everything since
  the pane-bugs verify:
  1. KIND chip click → graph lens render (path / tag /
     contact / language).
  2. Dashboard carousel (About / Workspace / Indexing graph
     w/ pulse).
  3. Cmd+, flips each surface (Terminal / Editor / Graph /
     FB / Dashboard) + the new Dashboard back-of-card
     renders Appearance + Screen Lock + Screensaver +
     Metadata cleanly.
  4. Lane A's bug 4 (Shift+Enter in Terminal rich prompt).
- SETTINGS_DISABLED meta tag still emitted by chan-server;
  small server-side cleanup queued for round 2.

**Contention**
- None.

**Ready for release cut decision**
Lane B's roadmap is drained. I can:
A. Run the chan-desktop smoke now (you watch live).
B. Start the v0.17.0 release cut mechanics (Cargo.toml +
   tauri.conf.json bump, dry-run release.yml via
   workflow_dispatch publish=false) — STOPPING before
   tagging so you can verify artifacts.
C. Both — smoke first, then release-cut mechanics, then
   pause for your tag-and-push approval.

Per `reference_release_cut_mechanics` tagging is what fires
`release.yml`, and per `feedback_merge_is_not_push` I won't
push without explicit ask.

Detail in `lane-b/journal.md`.

## 2026-05-28 19:30 @@LaneB -> @@Alex
Smoke report triage (round-1-closing-tests.md)

Read your `docs/journals/phase-13/round-1-closing-tests.md`.
Categorising for cut decision before I dive in.

**Lane B regressions (round-1 must-fix before tagging v0.17.0):**

1. `Cmd+,` second press doesn't flip back to front (Terminal /
   Editor / Graph / FB). First press → back works. Second press
   is a no-op. Need to debug — flipHybrid is a clean toggle on
   `node.showingBack` and there's only one keymap; suspecting
   focus / event-propagation interaction with the form inputs
   on the back surfaces.
2. `Cmd+,` on Dashboard shows EMPTY back. Root cause:
   `Pane.svelte`'s back-of-card switch has arms for
   terminal / file / graph / browser but NOT `dashboard`. The
   right-click "Settings" path uses a different code path
   (`DashboardTab.settingsOpen`) that DOES render correctly —
   that's why right-click works but Cmd+, doesn't. Fix:
   extract the Dashboard back into a HybridDashboardConfig
   component (mirror Terminal/Editor/Graph/FB) and add the
   `dashboard` arm in Pane.svelte. Right-click and Cmd+,
   converge.
3. `Cmd+,` on empty pane flips the whole pane visually.
   flipHybrid runs even though there's no surface; the
   pane-flip animation still fires. Fix: gate flipHybrid on
   `activeTabId !== null` (no-op when empty).
4. Empty-pane menu still says "Infographics" (label). I left
   user-visible labels alone in slice 3a thinking the rename
   was internal, but the menu IS user-visible. Flip the label
   to "Dashboard".
5. Tab title kind= prefix missing on graphs with a selected
   node. Root cause: `graphTabLabel` overrides `tab.title`
   with `selectedNodeLabel` when set, dropping the prefix.
   Fix: compose them (`path=<selectedLabel>` or fall back to
   the title).
6. QR-donate not loading. Image at `web/public/qr-donate.png`,
   referenced as `/qr-donate.png`. May be a server static-asset
   wiring miss in `rust-embed`. Need to verify.

**Lane B polish / scope expansions (defer to round 2 unless you
override):**

7. Workspace info widget needs "Show in File Browser" / "Graph
   from here" / "Upload" / "Download" buttons + a separator
   between COCOMO and NOTES DIRECTORIES.
8. Source Code Pro toggle should move from Dashboard About to
   Terminal back-of-card. New mental model.
9. Reload Cmd+R right-click menu entry on Dashboard (currently
   only shows Settings).
10. Indexing graph default zoom too small + clicking a node
    should label that node + its 1st-degree connections.

**Lane A scope (will ping them):**

11. Contact mentions not in graph at all (your hypothesis:
    only `kind: contact` frontmatter .md files are wired, not
    `@@mention` instances). Same root cause for the editor's
    `@{name}` autocomplete missing mentions. This is Lane A's
    mention-resolver scope, not the Lane B graph lens.
12. Parent dir click on the graph — inspector missing. May be
    Lane A inspector parity or a graph-side regression; I'll
    investigate when I'm in the graph code anyway.

**Lane B investigation needed (could be either round):**

13. Tag chip lens renders empty. The slice 2b BFS arm seeds
    from the tag's nodeId — if the data set has the tag node
    but no edges to it, the lens IS empty. Need to verify the
    `/api/graph` response includes the tag node + its edges in
    the workspace dataset.
14. Language lens shows only 1 directory. Same suspicion: the
    bubble's edges to language-of-X files may not be in the
    response. Lane A's inspector wiring opens the lens with
    `mode: "semantic"` which doesn't fetch the language layer.

**Lane B inspector parity (cross-cutting Lane A):**

15. Workspace root inspector keeps the "NOTES DIRECTORIES"
    config section + needs Graph from here / Download buttons.
    Per your spec, NOTES DIRECTORIES belongs in the dashboard
    only. The inspector should reach parity with the folder
    inspector. Cross-cutting Lane A's `ad184179` parity work.

### Proposed cut line

If you agree, I'd fix the 6 Lane B regressions (items 1-6)
before tagging v0.17.0, ping Lane A on 11-12 + 15, and mark
7-10 + 13-14 as round-2 carryover. That keeps the v0.17.0
shape coherent (Cmd+, flips both ways, Dashboard back-of-card
works, QR loads, tab titles show kind, menu says "Dashboard")
without spiraling into the scope expansions.

If you'd rather I keep going past the cut line on the polish
items, say so and I'll continue. Otherwise I'll start on 1-6
now.

## 2026-05-28 21:18 @@LaneB -> @@Alex
ready+merged: phase-13-lane-b@cc978c5f -> main@cc978c5f - round-1 closing slice 1 (B1 + B4 + B5 + B6 + B7 verify + B8 + B11 + B12)

**Highlights**
- Seven of the 12 closing items landed in 4 atomic commits on
  `phase-13-lane-b` (now fast-forwarded into main):
  - `d5417e88` B1 + B4: graphTabLabel keeps the `kind=` prefix
    on selection; `flipHybrid` early-returns on empty panes.
  - `24bae323` B5 + B6: user-visible "Infographics" -> "Dashboard"
    across the empty-pane right-click menu, EmptyPaneWelcome
    secondary tile, Cmd+I shortcut label, openDashboardInPane
    default title, DashboardTab aria-label + back-of-card
    title + ariaLabel. Empty-pane menu reorder puts Dashboard
    between Graph and Search.
  - `a74ec43d` B8: tag lens BFS now bidirectional - the backend
    emits file->tag edges, so forward-only BFS from the tag node
    silently dropped every backlink. Mirrors the contact arm
    shape.
  - `cc978c5f` B11 + B12: DashboardTab right-click menu carries
    Reload (Cmd+R) alongside Settings (Cmd+,); GraphCanvas
    `pendingInitialFit` defers the fit when the host is 0x0 at
    `start()` (carousel mount-before-visible) and replays it
    once the host reports nonzero dimensions plus a 900ms refit
    window; EmptyPaneCarousel wires `selectedIndexId` so clicks
    surface the clicked node + 1-hop neighbour labels.
- B7 verified empirically (no code change): curl
  `/qr-donate.png` on a fresh lane-b binary returns 200,
  content-type image/png, content-length matches source bytes.
  The smoke regression was almost certainly a stale chan-desktop
  binary built before the qr-donate.png landed in web/dist.
  Static_assets.rs already bakes the file via rust-embed and
  `web/public/qr-donate.png` is tracked in git so Vite copies
  it into dist on every build.
- Solo merge gate (Lane A has nothing queued - their last entry
  at 16:15 confirmed roadmap drained). Combined tree IS lane-b
  in this cycle, so I ran the full per-slice gate on lane-b
  directly and skipped the `../chan-integration` ceremony:
  cargo fmt --check, cargo clippy --all-targets -- -D warnings,
  cargo test, cargo build --no-default-features, npm run check
  (4111 files / 0 errors), npm run build, vitest 1612 passed /
  11 skipped (+9 above the prior 1603 baseline from 6 new
  graphTabLabel cases, B4 empty-pane no-op, 2 tag-BFS source
  pins, 3 pendingInitialFit pins, B11 Reload row pin, B12
  selectedIndexId pin).
- Fast-forwarded main `5a241f0f` -> `cc978c5f`. No push (per
  `feedback_merge_is_not_push`).

**Lowlights**
- B2 (Cmd+, second-press), B3 (HybridDashboardConfig extraction),
  B9 (language lens), B10 (Source Code Pro toggle move) still
  pending; all are larger or need live debug. Next turn picks
  these up.
- No chan-desktop empirical smoke on this slice yet. Per
  `feedback_svelte_static_gate_misses_runtime` the B1 graphTabLabel
  reactivity + B8 tag BFS lens render + B12 indexing graph
  selection labeling want a browser walk; per
  `feedback_terminal_webgl_wkwebview` B11's reload-window chord
  wiring is WKWebView-shape. Will batch after the larger remaining
  fixes land (B3 in particular is the bigger UX-affecting
  change) so the desktop smoke covers a coherent tree, not piecewise
  half-states.

**Contention**
- None.

**Next slice**
B11/B12-adjacent polish drained, moving to the structural items:
- B3 first (HybridDashboardConfig extraction + Pane.svelte
  dashboard arm + DashboardTab settingsOpen retirement) - this
  unblocks B2 since the Cmd+, second-press repro currently
  surfaces against Dashboard (no back-of-card to flip back from
  cleanly).
- B2 (Cmd+, second-press debug under chan-desktop) once B3
  gives a clean Dashboard back-of-card to compare against.
- B9 (language lens) needs backend investigation - whether
  /api/graph?scope=workspace emits the language layer or only
  /api/graph/languages does.
- B10 (Source Code Pro toggle move to Terminal back).

Detail in `lane-b/journal.md`.

## 2026-05-28 21:48 @@LaneB -> @@Alex
ready+merged: phase-13-lane-b@ff22fe1c -> main@ff22fe1c - round-1 closing slice 2 (B2 + B3 + B9 + B10)

**Highlights**
- All 12 round-1 closing items now landed across two slices:
  - `05431c96` B3: HybridDashboardConfig extracted from
    DashboardTab.svelte; Pane.svelte back-side switch gained the
    `active?.kind === "dashboard"` arm so Cmd+, flips the
    Dashboard cleanly. Right-click Settings entry retired
    (Cmd+, is canonical now per planning); Hamburger keeps
    only Reload. screensaverSettings.test.ts pins rebased onto
    the new component source (19 tests pass verbatim).
  - `fc8b1fc3` B2 defensive: Cmd+, matcher in App.svelte
    rewritten to `(e.code === "Comma" || e.key === ",")` so
    AZERTY / QWERTZ users hit the handler, plus
    stopImmediatePropagation after preventDefault so any
    duplicate listener can't toggle showingBack twice in the
    same tick. Calling out explicitly: this is a defensive
    patch, NOT a confirmed root-cause fix - see "lowlights"
    below for the empirical-verification ask.
  - `ff22fe1c` B9: chan-server's merge_language_layer now
    emits Language -> File edges directly (one per file)
    instead of routing through build_language_graph's
    depth-bounded top-N directory rollup. The roadmap spec is
    "language node has edges to all files of that language";
    the workspace lens needed per-file edges, and
    /api/graph/languages keeps the rollup for its overview.
    Rust test pinned the new shape (asserts every markdown
    file appears as a 1-hop neighbour + no `directory:<path>`
    targets emitted by the language layer).
  - `ff22fe1c` B10: verified existing placement - SCP toggle
    lives in HybridTerminalConfig.svelte (the dropdown is the
    on/off control), Dashboard About slide keeps attribution
    + OFL link. Vitest pin lock the split so the next round
    can't accidentally regress it. No behavioural change
    beyond the pin.
- Full per-slice gate green on lane-b head (ff22fe1c):
  cargo fmt --check, cargo clippy -D warnings, cargo test
  (no indexer flake), cargo build --no-default-features,
  npm run check (4113 files, 0 errors), npm run build,
  vitest 1616 passed / 11 skipped.
- Solo merge (Lane A drained at 16:15; combined tree IS
  lane-b). Fast-forwarded main `cc978c5f` -> `ff22fe1c`.
  No push.

**Lowlights**
- B2 is gated-green but NOT empirically verified. The Cmd+,
  keymap path in App.svelte is single-source; KEY_BRIDGE_JS
  has no Comma case; the only desktop-side dispatch is the
  macOS app menu accelerator at
  `desktop/src-tauri/src/main.rs:1834-1836` ("Settings…",
  accelerator `CmdOrCtrl+,`) which calls dispatch_to_focused_workspace
  + chan:command + runCommand -> flipHybrid. Per `feedback_pre_release_merge_unverified`
  I shipped the defensive matcher patch (layout-independent +
  stopImmediatePropagation) and pinned the new shape; if the
  round-2 chan-desktop walk still shows "press 2 is a no-op",
  the next escalation is to remove the macOS menu accelerator
  (so the SPA keymap is the single source of truth for the
  chord) - that change touches desktop/src-tauri/ which is
  outside Lane B's web/src + chan-server scope this phase.
  Please flag if you'd like me to take it now.
- No chan-desktop smoke on the combined slice. Per
  `feedback_svelte_static_gate_misses_runtime` the
  B1/B3/B8/B12 reactivity changes + B2 keymap change + B11
  Reload chord wiring all want a browser/desktop walk; per
  `feedback_terminal_webgl_wkwebview` B11's reload_window IPC
  + B2's keymap are WKWebView-shape. Calling that out as the
  next gate before the v0.17.0 cut.
- B9's language layer change touches chan-server (Rust). The
  cargo test gate covers it but the language lens render path
  end-to-end (backend payload + GraphPanel 1-hop BFS +
  render) needs an empirical pass on a workspace with
  multiple directories containing markdown files. Same
  smoke-debt note applies.

**Contention**
- None on the merge.
- The B2 desktop-menu-accelerator escalation is the only open
  cross-area question.

**Ready for release decision**
Lane B's round-1 closing scope is drained (all 12 smoke items
addressed). Suggested next steps:

A. chan-desktop empirical smoke on the combined tree
   (Cmd+, second-press across T/E/G/FB/Dashboard, Dashboard
   back-of-card content, KIND chip lens render, language
   lens completeness, indexing-graph fit + click labels,
   Reload menu entry). If anything trips, lane-b iterates.
B. v0.17.0 release cut mechanics (Cargo.toml +
   tauri.conf.json bump, dry-run release.yml via
   workflow_dispatch publish=false). Stopping before the
   tag.
C. Both - smoke first, then release-cut mechanics, then
   pause for your tag-and-push approval.

Per `reference_release_cut_mechanics` tagging fires
`release.yml`; per `feedback_merge_is_not_push` I won't push
without explicit ask. Picking C unless you redirect.

Detail in `lane-b/journal.md`.

## 2026-05-28 22:15 @@LaneB -> @@Alex
merged: phase-13-lane-a@3c9f57bd + phase-13-lane-b@ff22fe1c -> main@92ea0677 - Lane A's A1/A3/A4 + my closing slices integrated

**Heads-up: I missed Lane A's three merge-ready slices on my first
cycle.** Your "have you included LaneA's work" nudge caught the
gap - I had pegged Lane A as drained against their 16:15 entry, but
their round-1 CLOSING report at 21:36 queued three more slices
(A4 70ab238e editor @-completion + A3 a46e0944 language inspector
body + A1 3c9f57bd workspace-root inspector parity). All landed on
phase-13-lane-a but not yet integrated. I'll be more rigorous about
re-reading the channel tail before the next gate cycle - per
`feedback_check_for_updates_surface` the "is there new work?" check
must walk channels + task-file tails, not just the last-reported
status.

**Combined-tree gate on `../chan-integration`** (off main
`ff22fe1c`, merged `phase-13-lane-a` into it):
- Auto-merge clean on EmptyPaneCarousel.svelte (A1's
  `variant="dashboard"` and my B12 selectedIndexId are in
  different regions), GraphPanel.svelte (A1 + A3 wiring vs my B8
  tag BFS arm), dashboardTabAndCarousel.test.ts (A1 added 5 lines
  for parity; my B3 rewrote the Wave 4 block).
- cargo fmt --check ✓
- cargo clippy --all-targets -- -D warnings ✓
- cargo test ✓ (the indexer flake `writes_to_drafts_subtree_get_indexed_under_drafts_prefix`
  flagged once on first run per `feedback_fresh_binary_rewalks`;
  passed cleanly on re-run with all 540 chan-workspace tests
  green).
- cargo build --no-default-features ✓ (2m22s fresh build)
- npm install + npm run check ✓ (4117 files / 0 errors / 0
  warnings - +4 over the lane-b-only baseline reflects A3's
  LanguageInfoBody.svelte + the new test files)
- npm run build ✓
- vitest ✓ (1632 passed / 11 skipped - +16 over my 1616
  lane-b-solo baseline = A1 workspaceInfoBodyParity + A3
  languageInspectorBody + A4 mentionBubble additions)
- Fast-forwarded main `ff22fe1c` -> `92ea0677` (the merge
  commit). Cleaned up the `../chan-integration` worktree +
  phase-13-integration branch.

**Highlights**
- All round-1 closing items + Lane A's three round-1 closing
  slices are now on main. Round-1 scope across both lanes is
  drained.
- A1's workspace-root-inspector parity (variant="dashboard"
  gate) cleanly handed the Dashboard's slide 1 reuse back to
  its original config shape while letting the workspace-root
  GRAPH inspector adopt the standard folder action row -
  exactly the per-roadmap split.

**Lowlights**
- The missed-channel-tail gap (above). Reported in the round-
  close retrospective when I commit the phase-13 docs.

**Next moves**
1. Commit all docs/journals/phase-13/ as `docs(phase-13):
   close round 1` per your nudge + `feedback_coordination_docs_commit_timing`,
   including a full retrospective per `feedback_round_close_retrospective`.
2. Then the chan-desktop empirical smoke walk (you're driving
   that since Chrome MCP doesn't reach WKWebView from this CLI).
3. Then the v0.17.0 release-cut mechanics (Cargo.toml +
   tauri.conf.json + Cargo.lock bumps, dry-run release.yml via
   workflow_dispatch publish=false). Stopping before the tag
   for your explicit confirm.
4. After confirm: tag + push tag; verify /dl/latest.json
   supersedes 0.16.0 + chan-desktop self-upgrade walk.

Picking step 1 now.

Detail in `lane-b/journal.md`.

## 2026-05-28 23:56 @@LaneB -> @@Alex (round-1 closing-2 + empty-pane menu retire)

ready to merge: phase-13-lane-b@b428c4b7 - round-1 closing-2 batch (B1c B2c B3c B4c B7a B7b B8 B9) + lane-b-empty-pane-menu retire

Single slice covers all 8 closing-2 items + the empty-pane right-click
retirement @@LaneA cut on your direction. Touched files:
- web/src/state/tabs.svelte.ts (B1c restore arm, B2c showingBack clear)
- web/src/state/tabs.test.ts (B1c + B2c pins; setActivePane import)
- web/src/components/HybridDashboardConfig.svelte (B3c picker move)
- web/src/state/screensaverSettings.test.ts (B3c pin retarget)
- web/src/components/EmptyPaneCarousel.svelte (B4c withTokenQuery;
  empty-pane menu retire dropped the vestigial oncontextmenu prop)
- web/src/components/EmptyPaneCarousel.test.ts (vestigial forwarder
  pin)
- web/src/components/GraphCanvas.svelte (B7a onSetAsScope prop +
  ondblclick)
- web/src/components/GraphCanvas.test.ts (B7a pin)
- web/src/components/GraphPanel.svelte (B7a dblclickRescope; B7b
  depthDisabled + depthShallow shape)
- web/src/components/graphDepthFilter.test.ts (B7b pin)
- web/src/components/Pane.svelte (B8 spawnActions fold;
  lane-b-empty-pane-menu retire: removed emptyPaneMenu state +
  helpers + the triggerless HamburgerMenu block + the .placeholder
  oncontextmenu binding + the EmptyPaneWelcome oncontextmenu binding
  + the tab-strip contextmenu empty-pane branch)
- web/src/components/Pane.test.ts (B8 7-row hamburger; empty-pane
  right-click no-op)
- web/src/components/EmptyPaneWelcome.svelte (B9 Search + chord
  render + grid; lane-b-empty-pane-menu dropped the oncontextmenu
  prop)
- web/src/components/dashboardTabAndCarousel.test.ts (B4c + B9 + B8
  + Pane.svelte mount pin update)

File-disjoint from @@LaneA's A5 + A6 (WorkspaceInfoBody.svelte +
their EmptyPaneCarousel.svelte slide-1 mount edits at line 36 +
~428). My EmptyPaneCarousel.svelte changes are the import + the
`<img>` src + the deleted `oncontextmenu` prop + its outer div
binding - non-overlapping with their changes. If they queue ready
to merge, the integration tree should auto-merge ort-clean.

Per-slice gate: green across the board.
- cargo fmt --check
- cargo clippy --all-targets -- -D warnings
- cargo test --workspace
- cargo build --no-default-features
- web npx svelte-check --threshold warning (0 errors, 0 warnings)
- web npm run build (rolldown ok; only pre-existing
  ineffective-dynamic-import warnings)
- web npx vitest run (1639 passed / 11 skipped / 164 files)

Pre-release per `feedback_pre_release_no_backcompat`: empty-pane
right-click affordance + vestigial oncontextmenu forwarders deleted
outright (no shims).

Empirical caveat per `feedback_svelte_static_gate_misses_runtime` +
`feedback_terminal_webgl_wkwebview` + `feedback_pre_release_merge_unverified`:
B2c, B4c, and the empty-pane right-click no-op are WKWebView-shape
(reactivity timing + non-root mount + contextmenu suppression).
Chrome MCP cannot drive WKWebView from this CLI; gated-green and
flagged for your chan-desktop smoke as part of the v0.17.0 release-
cut walk. Confirm flow:
- Open + reload window with Dashboard tab: it persists (B1c).
- Cmd+, on Dashboard flips to back; focus another pane and back -
  no auto re-flip; flip-back via Cmd+, returns to carousel (B2c).
- Dashboard back-of-card: Appearance / Screen lock / Metadata
  archive (3 sections); toggle Screen lock OFF - timeout/PIN +
  theme picker hide together; ON - both reappear (B3c).
- About slide: QR code renders at 160x160 (B4c).
- Graph: double-click a folder/file node rescopes "from here"
  (B7a); slider on the default workspace graph drags through
  depths 1..N revealing more directory tiers (B7b).
- Pane hamburger (top-bar ⋮): 7 spawn rows (New Draft / Terminal /
  File Browser / Rich Prompt / Graph / Search / Dashboard) then
  Enter Hybrid Nav etc. (B8).
- Empty single-pane welcome surface: 5 tiles + divider + 2
  secondary tiles (Search Cmd+S, Dashboard Cmd+I) each with chord
  hints (B9).
- Right-click on an empty pane (single AND multi-pane): NO popover
  opens (lane-b-empty-pane-menu).
- Right-click on a loaded pane: still shows Reload / Open Inspector.

Standing by for @@LaneA's merge-ready signal to open the
combined-tree gate in ../chan-integration.

## 2026-05-29 00:18 @@LaneB -> @@Alex (merge-gate cycle)

merged: phase-13-lane-a@4280d5f3 + phase-13-lane-b@b428c4b7 -> main@4e19d8d2  -  round-1 closing-2 (Lane A A5+A6 + Lane B 8 closing items + lane-b-empty-pane-menu retire)

Both lanes drained for closing-2; combined-tree gate green; main
fast-forwarded.

**Cycle**
- Walked the FULL TAILS of both `event-lane-a-alex.md` +
  `event-lane-b-alex.md` per round-1 retrospective lesson; lane-a
  queued `4280d5f3` (A5+A6); lane-b queued `b428c4b7` (closing-2
  batch). Both ready-to-merge.
- `git worktree add ../chan-integration -b phase-13-integration main`.
- `git merge --no-ff phase-13-lane-a`: ort, clean. 6 files /
  +281 -7. Includes the shared-turf
  `EmptyPaneCarousel.svelte` import + slide-1 mount prop pass-
  through declared on `event-lane-a-lane-b.md`.
- `git merge --no-ff phase-13-lane-b`: ort, clean (auto-merging
  EmptyPaneCarousel.svelte + GraphPanel.svelte +
  dashboardTabAndCarousel.test.ts succeeded without manual edits;
  Lane A's edits at line 36 + ~428 and my edits at the `<img>` +
  the removed `oncontextmenu` prop were file-disjoint as
  declared).

**Combined-tree gate**
- cargo fmt --check ✓
- cargo clippy --all-targets -- -D warnings ✓
- cargo test --workspace ✓ (indexer flake did NOT fire this run)
- cargo build --no-default-features ✓
- web `npm install` + `npx svelte-check --threshold warning` ✓
  (4117 files / 0 errors / 0 warnings)
- web `npm run build` ✓
- web `npx vitest run` ✓ (1646 passed / 11 skipped / 164 files;
  +7 over my lane-b-only 1639 from Lane A's A5/A6 pins)
- Fast-forwarded main `e30f73ef` -> `4e19d8d2`. Cleaned up
  `../chan-integration` worktree + `phase-13-integration` branch.

**Highlights**
- Round-1 closing-2 scope drained across both lanes onto main.
  Including the lane-b-empty-pane-menu retire (right-click on an
  empty pane is now a no-op; pane hamburger carries the full
  7-entry spawn set).
- A5 (clickable Languages) lands on main so the user's
  empirical-test Bug 3 from the post-build smoke is now
  resolved.

**Next moves**
1. Address @@Alex's just-reported empirical bugs in a closing-3
   wave: Bug 1 (Drafts reindex notification persistence; my
   territory), Bug 2 (Dashboard indexing slide not maximized;
   my territory), Bug 4 (missing COCOMO ↔ NOTES DIRECTORIES
   separator; Lane A's `WorkspaceInfoBody.svelte` territory -
   routing to them on `event-lane-b-lane-a.md`).
2. The v0.17.0 release-cut stays on hold until the closing-3
   wave drains.

Detail in `lane-b/journal.md`.

---- Round 2 ----

## 2026-05-29 @@LaneB -> @@Alex
Turn 1: spec correction on B-slice 2 (App.svelte IS needed)

Picked up round 2. Worktree on `phase-13-r2-lane-b` @ main
(76f5e18b), clean. Working B-slices 4/2/3/1.

CONTENTION (one item, low-risk, already declared to @@LaneA):
- The request file says B-slice 2 ("Dashboard off Cmd+I") needs
  "no App.svelte change". That's wrong. The actual Cmd+I ->
  Dashboard binding is a hardcoded `e.code === "KeyI"` branch in
  App.svelte::onWindowKey (~849), plus the desktop KEY_BRIDGE
  (serve.rs:634) - NOT the shortcuts.ts registry (display-only).
  To honour your "Cmd+I = editor italic, Dashboard via Hybrid Nav"
  decision AND avoid a double-fire (CM6 doesn't stopPropagation),
  I must delete that branch. App.svelte is @@LaneA's file this
  round, so I declared a surgical KeyI-only deletion to them on
  event-lane-b-lane-a.md and will reconcile at the merge gate.
  Say the word if you'd rather @@LaneA own that 6-line deletion.

Minor call I'm making (flag if you disagree): instead of dropping
app.dashboard.open's chord to nothing (which would hide Dashboard
from the cheatsheet + menus), I'm repointing it to its real
reachable chord `Mod+. i` (Hybrid Nav). Keeps it discoverable;
Cmd+I is still freed for italic.

Still waiting on @@LaneA's Team Work label string before the
"Rich Prompt" -> "Team Work" rename in my shared files.

Detail in `lane-b/journal.md`.

## 2026-05-29 @@LaneB -> @@Alex
Turn 1: all 4 B-slices + 2 new asks done, gated, web-smoked

All on `phase-13-r2-lane-b`. Full gate green (cargo fmt/clippy
-Dwarnings/test/build --no-default-features; web check 0/0/4117 +
build + vitest 1660). Commits:
- f2f78e52 B4 hamburger split labels -> Cmd+/ , Cmd+?
- dc3a1230 B2 Bold/Italic chords + Dashboard off Cmd+I
- b16e699d B3 desktop Cmd+Shift+N -> current workspace
- 3eb7f4c4 B1 list glyphs (en-dash / U+25CF / U+25EF)
- 8c6f4a94 Cmd+, per-pane flip fix (your bug report)

HIGHLIGHTS
- B2: the real Cmd+I->Dashboard binding was NOT shortcuts.ts; it was a
  hardcoded App.svelte KeyI branch + the desktop KEY_BRIDGE. Removed
  both (App.svelte overlap declared to @@LaneA). Smoke: Cmd+I
  italicises and does NOT open Dashboard.
- B1: browser smoke caught a real bug the static gate missed - an
  absolute-positioned ::before detached nested glyphs to the gutter.
  Fixed with an in-flow approach; re-smoked OK. Glyphs are your final
  pick: ● U+25CF top / ◯ U+25EF nested / en-dash for hyphens.
- Cmd+, bug: two root causes - splitPane copied the flip onto the new
  EMPTY pane, and setActivePane (round-1 B2c) cleared the previous
  pane's flip on focus-move. Now flip is strictly per-pane, >= 1 tab,
  cleared on empty, persisted across reload. Smoked all five invariants
  via the serialized layout (flip + split-clean + focus-isolation +
  empty no-op + reload-persist).

FLAGS / DECISIONS FOR YOU
- App.svelte KeyI deletion is in @@LaneA's file; declared on the
  cross-lane channel. Say if you'd rather @@LaneA own it.
- Cmd+, fix changes one prior behaviour: closing the last tab of a
  flipped pane now drops the flip (was: keep mid-config back). Per your
  "strictly >= 1 tab". Flag if you wanted the old behaviour.
- DESKTOP-ONLY, needs your chan-desktop (WKWebView) walk, can't Chrome:
  B3 Cmd+Shift+N new-window, and the Cmd+I removal on the native
  KEY_BRIDGE. Per feedback_terminal_webgl_wkwebview.

DOCS: committed the phase-13 docs to main now as a docs(phase-13)
in-flight commit per your ask (deviates from the usual keep-untracked-
until-round-close; noted in the journal + addenda). New asks captured in
`lane-b-round-2-addenda.md`.

STILL PENDING: @@LaneA's Team Work label string for the Rich Prompt ->
Team Work rename in shortcuts.ts / Pane.svelte / EmptyPaneWelcome.svelte.

Not merged to main yet (slices are merge-gate-ready on the branch; I
hold merges for the round per the bootstrap). Detail: lane-b/journal.md.

## 2026-05-29 @@LaneB -> @@Alex
Merge gate PAUSED - Lane A doing more cleanup

You flagged Lane A is doing further dead-code cleanup, so their
55179ad9 is stale. Pausing the combined-tree merge gate; I'll re-run it
against Lane A's NEW commit once it lands + they signal on the cross-
lane channel. Nothing merged to main (main is still 248bc830, docs
only). A background cargo gate was running against the stale combined
tree - disregarding its result.

My lane is done + independent of Lane A's cleanup:
- Applied the Team Work label (ae06398b) - chord id
  app.terminal.richPrompt stays STABLE, so it's just a label swap and
  is unaffected by whatever Lane A deletes.
- Re your Q3 to Lane A ("cmd+p -> rich prompt now team work, right?"):
  confirmed on my side - the chord id is unchanged, only the label +
  (Lane A's) handler flow changed.

phase-13-r2-lane-b @ ae06398b: B1-4 + Cmd+, fix + Team Work label, all
gated + (web) smoked. Standing by for Lane A's cleanup commit.

## 2026-05-29 @@LaneB -> @@Alex
combined gate GREEN @ Lane A 25c81182; 2 rename residuals in Lane A files

Re-gated against Lane A's NEW head (25c81182, after their dead-code
cleanup) + my ae06398b. Clean auto-merge. FULL gate GREEN:
- cargo fmt / clippy -Dwarnings / test (0 failed) / build
  --no-default-features.
- web svelte-check 0/0/4107 + build + vitest 1570 passed.

Two user-facing "Rich Prompt" residuals my sweep caught, BOTH in Lane A
files + both look like dead/stale leftovers of their deletion (so it's a
rename-vs-REMOVE judgment that's theirs, not a blind swap):
1. store.svelte.ts: "Broken Rich Prompt" warning label - but
   `broken_rich_prompt` has ZERO backend refs now, so it's a dead
   frontend branch. -> remove.
2. FileInfoBody.svelte Drafts notice "Rich Prompt submissions persist as
   Drafts/rich-prompt-N/" - the archival it describes was deleted. ->
   stale clause, drop/rewrite.
Flagged to @@LaneA on event-lane-b-lane-a.md with exact lines.

These don't break the gate (green). HOLDING the merge to main pending
the call:
(a) Lane A fixes the 2 residuals on their branch -> I re-gate + merge; or
(b) merge the gated-green tree now, Lane A cleans up in a follow-up.
My lean: (a), since Lane A is warm + #2 is a now-incorrect user-facing
notice. Your call. Still holding the v0.18.0 tag for you regardless.

## 2026-05-29 @@LaneB -> @@Alex
DONE: full rich-prompt scrub + combined tree MERGED to main (no push)

Per your "do the cleanup yourself" + "do not leave any rich prompt code
behind": I fixed the 2 residuals myself AND scrubbed EVERY remaining
"rich prompt" identifier across the combined tree (160 refs, 35 files) -
pre-release so no back-compat:
- chord id app.terminal.richPrompt -> app.terminal.teamWork (all sites
  + desktop KEY_BRIDGE moved atomically).
- tab.richPrompt field + serialization -> tab.teamWork.
- .rich-prompt CSS class -> .team-work; backend rich_prompt/rich-prompt
  -> team_work/team-work; 5 richPrompt* test files renamed; all comments.
- 0 "rich.?prompt" refs left in web/src + crates + desktop/src.

GATE GREEN: cargo fmt/clippy -Dwarnings/test (0 failed)/build
--no-default-features; web svelte-check 0/0/4107 + build + vitest 1570.
BROWSER-SMOKED the wire/CSS/field renames: Cmd+P (app.terminal.teamWork)
fires the lead terminal + embedded editor (.team-work renders clean) +
Lane A's Spawn-agents dialog (Neo / New-Load / dropdown / Lead / real
estate). No console errors.

MERGED to main: ff'd 248bc830 -> c4a4adc6 (Lane A 25c81182 + Lane B
ae06398b + residual cleanup 74ec13d7 + scrub c4a4adc6). NO push (per
feedback_merge_is_not_push).

You can now build the FULL combined dmg from the main checkout
(/Users/fiorix/dev/github.com/fiorix/chan) for your chan-desktop walk:
`make macos-chan-dmg`. The desktop bits needing your WKWebView eyes:
Cmd+Shift+N new-window, Cmd+I removal, and the Cmd+P->teamWork KEY_BRIDGE.

HOLDING the v0.18.0 cut (version bump + tag) for your explicit go - say
the word, or tell me if there's more to change first.
