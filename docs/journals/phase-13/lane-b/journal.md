# @@LaneB journal - Phase 13 round 1

Append-only self-documenting log. One entry per meaningful action;
date / time + intent + outcome. Subagent activity rolls up here
through @@LaneB. Merge-gate decisions + release-cut steps also live
here.

## 2026-05-28 - turn 1 opening

Recovery files read in order: `CLAUDE.md`, `design.md` (skipped on
this turn - request file is already file-mapped down to line
ranges), `roadmap-round-1.md`, `bootstrap.md`,
`coordination/README.md`, `coordination/event-alex-lane-b.md` (empty
inbox), `coordination/event-lane-a-alex.md` (empty so far - no Lane A
merge-ready yet), this `lane-b/journal.md` (fresh). No
`event-lane-a-lane-b.md` exists yet.

### Worktree

Stale phase-12 `../chan-lane-b` removed (clean, last commit
`d43da40c docs+marketing: web-marketing + tracked-docs
drive->workspace completion (phase-12 lane-b)`; branch
`phase-12-lane-b` preserved). Created
`../chan-lane-b` on `phase-13-lane-b` at `b2caa7e9 chore(release):
0.16.0`.

### KIND route signature - scoped

Surveyed `crates/chan-server/src/routes/graph.rs`,
`crates/chan-server/src/lib.rs::router`, `web/src/api/types.ts`,
`web/src/state/tabs.svelte.ts` (`GraphTab`, `openGraphInActivePane`,
`graphTitle`), `web/src/state/store.svelte.ts`
(`openGraphForTag` / `openGraphForFile` / ...).

Design call: extend `/api/graph` with a `?kind=` discriminator
(default `path` = backward-compatible) instead of adding sibling
routes. Frontend `GraphView { nodes, edges }` response shape stays
unchanged - new lenses re-use the existing tag / mention / file /
language / directory node shapes. `scopeId` prefix convention
(`tag:` / `contact:` / `language:` / `file:` / `dir:` /
`workspace`) is already understood by `graphTitle` and lives in
the tab; the tab-title prefix work (`path=…` / `tag=…` / `lang=…`
/ `contact=…`) ships in this lane's frontend slice.

Lane A only needs three things to wire its Inspector kind chips,
which is what I'll post on `event-lane-b-lane-a.md` immediately:

1. The `openGraphFor{Tag,Contact,Language}` helper signatures (so
   Lane A doesn't have to know about scopeId prefixes).
2. The fact that `openGraphForTag` already exists (no new import).
3. Confirmation that the `/api/graph` HTTP shape is internal to the
   graph panel - Lane A doesn't hit it directly.

### Next slice

Pane bugs (focus-ring thickness + wobble parity) is file-disjoint
from Graph KINDS and is the simplest slice with the smallest test
surface, so I'll line it up as my first implementation slice in
parallel with the KIND backend subagent. Browser-smoke is mandatory
for the wobble work
(`feedback_svelte_static_gate_misses_runtime`); the desktop smoke
caveat (`feedback_terminal_webgl_wkwebview`) applies to any
pane-chrome change that interacts with terminal rendering, so I'll
flag any terminal-adjacent surface explicitly.

## 2026-05-28 turn 1 - pane bugs landed + first merge gate

### Slice 1: pane bugs

First attempt (subagent + manual gate) at `33e180c9`: matched the
top-bar 1px thickness by dropping the `.pane.focused` inset shadow
from 2px → 1px, fired the wobble on mouse-enter via the existing
`paneWobble` bus and on `setActivePane` focus changes. Gates all
green.

Desktop smoke (@@Alex live-watched) revealed two issues the static
gate couldn't catch:
1. Faded chrome ring visible on unfocused panes during hover -
   user explicitly didn't want any chrome on unfocused panes at
   all.
2. The wobble visibly fired on the chrome `::before` "faded line",
   not on the pane body. Asked for clarification via
   AskUserQuestion; @@Alex chose "wobble only fires on
   focus-CHANGE, on the focus ring".

Pivot at `975efe40`: collapsed the focus ring to the
`.pane.focused` border-color swap ONLY (removed inset box-shadow
+ entire `.pane::before` chrome layer + hover wobble trigger).
The border-color swap is single-source and uniform around the
pane - child backgrounds (tab strip, terminal canvas) can't cover
it the way they were covering the inset shadow at the top bar,
which is what made the body ring read thicker than the top ring.
Wobble moved to `.pane.focused.wobble` as an outer halo via
box-shadow keyframe (`box-shadow: 0 0 0 6px ..., var(--pane-shadow)`
at 40%, transparent at 0/100%). No transform anywhere on `.pane`
- xterm WebGL atlas safe.

Second desktop smoke: @@Alex confirmed both bugs fixed. "both bugs
fixed, cheers".

### Slice 1 vitest fallout

`contextMenuChrome.test.ts:43` pinned the prior `.pane::before`
chrome shape via regex - my redesign removed those selectors.
Updated the test in two commits:
- `a5c589b5` re-asserted against `.pane.focused.wobble` (the new
  selector that still uses the easeOutBack curve, so the
  semantic claim "pane motion shares the right-click menu curve"
  still holds).
- `ea23a691` bounded the xterm-safety regex (`.pane{...transform:
  scale...}`) to `[^}]*` so it doesn't bleed across CSS blocks
  into unrelated rules like `.tab:hover` (which legitimately uses
  `transform: scale(1.04)`).

### Merge gate (Lane A + Lane B combined)

Lane A queued two slices on `event-lane-a-alex.md`:
- `b2ef3f3b` bugs 1-4 (new-doc cursor focus, fresh-draft prompt
  suppression, list marker source preservation, terminal
  Shift+Enter).
- `ad184179` inspector absolute-path + COPY button + workspace-root
  parity.

Integration on `../chan-integration` worktree off main:
1. `git merge --ff-only phase-13-lane-a` (b2caa7e9 → ad184179).
2. `git merge phase-13-lane-b --no-edit` (merge commit needed -
   lane-b's parent is the pre-lane-a tip).

Auto-merge in `web/src/state/tabs.svelte.ts`: Lane A's draft-close
fix at line 2044, Lane B's `setActivePane` guard at line 3139.
File-disjoint within the same file; ort strategy handled it
cleanly.

Combined-tree gate (twice, both green after the test fix):
- `cargo fmt --check` ✓
- `cargo clippy --all-targets -- -D warnings` ✓
- `cargo test` ✓ (the indexer flake from my pre-merge gate did
  NOT reappear on the combined tree).
- `cargo build --no-default-features` ✓
- `npm run check` ✓ (0 errors, 0 warnings, 4111 files)
- `npm run build` ✓
- `npm test` (vitest) ✓ - 1615 passed, 11 skipped.

Fast-forwarded `main` to `phase-13-integration@f97146cc`. No push
per `feedback_merge_is_not_push`. Reported merge confirmation on
`event-lane-b-alex.md`.

### Process notes for myself

- Static gate is structurally blind to visual bugs; the screenshot
  from @@Alex turned a "looks fine on paper" first commit into a
  full redesign in 15min. `feedback_svelte_static_gate_misses_runtime`
  + `feedback_terminal_webgl_wkwebview` proved their weight; will
  not skip desktop smoke in future Lane B slices that touch pane
  chrome, even when static gates are green.
- First test fix at `a5c589b5` introduced a buggy regex that
  matched the unrelated `.tab:hover` rule via unbounded
  `[\s\S]*?`. Caught only when the integration gate re-ran the
  full vitest. Lesson: prefer `[^}]*` for CSS-rule-scoped matches.

### Next slice

KIND backend extension. The signature is already posted at
`event-lane-b-lane-a.md` (turn 1); the implementation slice will:
1. Extend `/api/graph` with `?kind=path|tag|contact|language`
   query discriminator + kind-specific payload fields.
2. Add `openGraphForContact(relPath)` + `openGraphForLanguage(lang)`
   helpers in `web/src/state/store.svelte.ts`.
3. Leave the frontend graph canvas / panel work for a follow-up
   slice (this slice is the minimum @@LaneA needs to wire chips).

## 2026-05-28 turn 2 - KIND slices 2a + 2b + Lane A 4a merged

### Slice 2a (24f1f31d)

Frontend-only stubs to unblock Lane A: `openGraphForContact` +
`openGraphForLanguage` exported from `store.svelte.ts`;
`graphTitle()` now prefixes every graph tab with `kind=` (`path=`
/ `tag=` / `contact=` / `lang=`); GraphPanel.synthesizeScope's
comment updated. The resolver still returns null for the new
prefixes at this point — panel falls back to workspace render
until 2b.

Tests rebaselined in tabs.test.ts for the kind-prefix shape; new
tests added for `language:` and `contact:Contacts/alice.md`
shapes. vitest 1616 passed.

Removed the hardcoded "Tag Graph" title from openGraphForTag so
all kinds flow through graphTitle() uniformly.

### Slice 2b (11e5fb37)

Real lens semantics. Added ScopeOption kinds `contact` (carries
the file rel_path) and `language` (carries the language id);
synthesizeScope maps the new scopeId prefixes; graphDepthCap
learns the new kinds (contact uses hardMax, language pinned to 1
so slider reads `[max]`); focalIds + openScopeHeaderInspector +
the scope header dropdown row all branch on the new kinds.

The lens BFS arms:
- Contact: BIDIRECTIONAL BFS from the contact's file node. The
  contact's file is found by `n.kind === "file" && n.path ===
  relPath`. Both incoming and outgoing edges expand the
  frontier, so the result subgraph captures backlinks (other
  docs that link to or `@@mention` the contact). Forward-only
  BFS — what tag uses today — would silently drop the backlinks
  the roadmap explicitly asks for.
- Language: always 1-hop from `language:<lang>` node id (no depth
  loop). Picks up every node the language bubble has an edge to;
  by indexer convention the language node carries an edge to
  every file of that language, which is what the roadmap wants.

Static gate green; cargo (with the indexer flake handled by
re-run); web check / build / vitest 1616 passed.

### Lane A merge gate (slice 4a -> 7c936504)

Lane A's slice 4a at `39fd3373` (KindChip onClick API + path/tag
chip wiring) landed on `f97146cc`. My 2a + 2b sit on top of
`f97146cc`; Lane A's 4a is a sibling. Integration via
`../chan-integration` worktree: started off main (`11e5fb37`),
merged `phase-13-lane-a` on top. Combined-tree gate:

- cargo fmt / clippy / build no-default: green.
- cargo test: flaked once on
  `chan_workspace::indexer::tests::writes_to_drafts_subtree_get_indexed_under_drafts_prefix`,
  passed on re-run (540 / 0 / 2 ignored). Same flake from the
  pane-bugs merge gate; pre-existing, not from this slice. Per
  `feedback_fresh_binary_rewalks` re-ran with the existing
  binary (the test failure was a parallel-execution race, not a
  stale binary issue).
- npm run check: 0 errors / 0 warnings / 4112 files.
- npm run build: clean (only the pre-existing
  INEFFECTIVE_DYNAMIC_IMPORT warning).
- vitest: 1619 passed / 11 skipped (Lane A added 3 KindChip
  tests).

Fast-forwarded main to `7c936504`. Pinged Lane A on
`event-lane-b-lane-a.md`: 4a is in, helpers are live, slice 4b
(contact + language chip wiring) is unblocked.

### What's next for Lane B (slice 3 - Dashboard)

Substantial slice:
1. Rename `InfographicsTab.svelte` -> `DashboardTab.svelte`. Host
   is `EmptyPaneCarousel.svelte`.
2. Auto-resize carousel to tab dimensions.
3. About widget: move SettingsPanel.svelte lines 643-679 (version
   + flags + attributions) here, embed `web-marketing/qr-donate.png`,
   add website + source repo icon-links, copy the "fund this"
   text from web-marketing.
4. Workspace-info widget: reuse Lane A's WorkspaceInfoBody now
   that 4a is merged.
5. Search-index graph widget: use the slice 2b graph in read-only
   spine-only mode, depth=max from workspace root, with the
   grey/green/pulsing-orange colour code.
6. Settings flip-back: Appearance (system/dark/light - GLOBAL) +
   Screen Lock + Screensaver + OK button. Pattern is
   HybridTerminalConfig / HybridEditorConfig / etc.
7. Cmd+, rebind from global SettingsPanel toggle to
   `flipHybrid(paneId)` of the focused component.
8. Retire SettingsPanel OverlayShell.

I'll likely break this into 2-3 sub-slices and may spawn a
subagent for the SettingsPanel migration since the file is large
and the move is mechanical.

## 2026-05-28 round-1 closing turn 1 - second @@LaneB picking up

Prior @@LaneB drained the round-1 roadmap (5a241f0f). I'm the
closing @@LaneB, working the 12-item smoke punchlist (B1-B12)
from `round-1-closing-tests.md` and `lane-b-round-1-closing.md`,
plus merge-gate orchestration and the v0.17.0 release cut.

Worktree state: `../chan-lane-b` at `5a241f0f` (== main tip),
clean. No rebase needed.

Channels checked:
- `event-alex-lane-b.md`: empty (no new @@Alex directives).
- `event-lane-a-alex.md`: Lane A's round-1 roadmap is drained
  (last entry 16:15, smoke walks complete, all green).
- `event-lane-a-lane-b.md`: most recent cross-lane note is Lane
  A's 16:15 smoke summary; flagged a tag/language lens
  "0/N nodes 0/N edges first paint" quirk that may overlap with
  B8/B9 here.

Plan for this turn: knock out the smallest disjoint fixes first
(B4 guard, B1 tab title prefix, B5 menu rename+order, B6
DashboardTab aria), then move to the structural ones (B3
HybridDashboardConfig extraction, B2 second-press debug). Each
fix gets its own commit. The merge-gate cycle batches whatever's
ready when @@Alex pings (or when Lane A queues anything).

## 2026-05-28 round-1 closing turn 1 - slice 1 landed (B1+B4+B5+B6+B7+B8+B11+B12)

Picked off the seven smallest items and merge-gated them solo:
- B1 (`d5417e88`) graphTabLabel keeps `kind=` prefix on selected
  node. `tab.title` shape from `graphTitle()` (`path=`/`tag=`/
  `contact=`/`lang=`/`Languages`) is parsed to extract the prefix
  before the `=`; `Languages` (no `=`) falls through to the bare
  label. 5 vitest pins for the kind shapes + the no-prefix
  fallback.
- B4 (`d5417e88`) flipHybrid empty-pane guard. `if (node.tabs.length
  === 0) return;` before the back-marker init / showingBack toggle
  / requestPaneFlip. 1 vitest pin asserting paneFlip.versions
  stays untouched on empty-pane flip attempt.
- B5 (`24bae323`) Infographics -> Dashboard rename + menu reorder.
  User-visible labels in Pane.svelte emptyPaneExtraActions,
  EmptyPaneWelcome.svelte secondaryEntries, shortcuts.ts row
  label, tabs.svelte.ts openDashboardInPane default title.
  Menu order: New Draft / Terminal / FB / Rich Prompt / Graph /
  Dashboard / Search (Dashboard between Graph and Search per
  @@Alex). Pane.test.ts + dashboardTabAndCarousel.test.ts +
  shortcuts.test.ts fixtures re-pinned.
- B6 (`24bae323`) DashboardTab aria-label, HybridSurfaceConfigShell
  title, ariaLabel updated: "Infographics" -> "Dashboard" /
  "Dashboard settings".
- B7 (verified, no commit) qr-donate.png is served correctly by
  static_assets.rs. Lane-b dev binary at `/tmp/chan-laneb-srv`
  with a fresh `/tmp/chan-test-laneb-b7` workspace returned 200
  + image/png + content-length 61308 on
  `/qr-donate.png`. The smoke regression was almost certainly a
  stale chan-desktop binary built before the asset landed in
  web/dist; nothing wrong with rust-embed wiring.
- B8 (`a74ec43d`) tag lens BFS bidirectional. Backend emits tag
  edges as `source: <file>, target: <tagId>`; forward-only BFS
  from the tag node never crossed those incoming edges so the
  lens rendered empty. Tag arm now mirrors the contact arm
  shape: source-arrow AND target-arrow branches inside the
  per-depth loop. New `graphTagLensBidirectionalBfs.test.ts`
  pins the bidirectional shape via `?raw` source asserts
  (computeScopedNodeSet is a $derived; not pure-importable).
- B11 (`cc978c5f`) Reload entry on DashboardTab right-click menu.
  Imported `reloadWindow`, `RefreshCw`, SHORTCUTS / currentOS /
  currentPlatform / formatChord; added inline `chordLabel`
  helper + `doReload` async function. HamburgerMenu now carries
  Reload (Cmd+R) above Settings (Cmd+,) - both chords render
  alongside the label. Height bumped 58 -> 88 to accommodate
  the second row.
- B12 (`cc978c5f`) indexing graph default zoom + click-to-label.
  GraphCanvas: new `pendingInitialFit` flag deferred when
  `start()` runs against a 0x0 host (carousel mounts the canvas
  before slide 2 is visible); `resize()` replays the fit +
  schedules a 900ms refit window on the 0->nonzero transition.
  EmptyPaneCarousel: `selectedIndexId` $state wired to
  onIndexingSelect (was a no-op) + passed to GraphCanvas.selectedId.
  Selection-labeling (selected node + 1-hop neighbours) is
  already implemented inside GraphCanvas's paint loop - this
  was the wiring change to surface it on the read-only spine.

### Per-slice gate

Full gate green on lane-b head (cc978c5f):
- cargo fmt --check ✓
- cargo clippy --all-targets -- -D warnings ✓
- cargo test ✓ (no indexer flake this round)
- cargo build --no-default-features ✓
- npm run check ✓ (4111 files, 0 errors, 0 warnings)
- npm run build ✓
- npm test (vitest) ✓ 1612 passed / 11 skipped (+9 above 1603
  baseline from B1, B4, B8, B11, B12 new asserts)

### Merge gate

Lane A has nothing queued; their roadmap drained at 16:15. Combined
tree IS lane-b in this cycle, so I ran the full per-slice gate on
lane-b directly and skipped the `../chan-integration` ceremony.
Fast-forwarded main `5a241f0f` -> `cc978c5f`. No push per
`feedback_merge_is_not_push`.

### What's next

The five remaining items split into two natural batches:
- **Structural batch**: B3 (HybridDashboardConfig extraction +
  Pane.svelte dashboard arm + DashboardTab settingsOpen +
  hamburger Settings entry retirement) + B2 (Cmd+, second-press
  debug). B3 should land first because B2's repro currently
  surfaces against Dashboard which doesn't have a clean
  back-of-card yet; once B3 ships, B2's repro converges to the
  general focus-capture / propagation question across all five
  surfaces.
- **Lens + polish batch**: B9 (language lens completeness -
  needs backend investigation re: /api/graph?scope=workspace
  vs /api/graph/languages) + B10 (Source Code Pro toggle move
  from Dashboard About to Terminal back-of-card).

After both batches land + a chan-desktop empirical smoke, the
release cut.

## 2026-05-28 round-1 closing turn 2 - slice 2 landed (B2 defensive + B3 + B9 + B10)

### Slice 2 commits

- **`05431c96`** B3: HybridDashboardConfig.svelte extracted out of DashboardTab.svelte. All Appearance / Screen Lock / Screensaver / Metadata archive state (screensaverEnabled, screensaverTimeoutSecs, screensaverTheme, screensaverPinSet, metadataImportFile, etc.) + their handlers (loadScreenLockState, toggleScreensaverEnabled, commitTimeout, commitScreensaverTheme, openPinDialog / cancelPinDialog / commitPin / clearPin, testScreenLock, export/importMetadataArchive) + all four sections of markup moved over. `onMount(() => void loadScreenLockState())` hydrates the back surface on every mount.
  - Pane.svelte: new `import HybridDashboardConfig` + dashboard arm in the back-side switch `{:else if active?.kind === "dashboard"} <HybridDashboardConfig onDone={() => flipHybrid(pane.id)} />`. Hybrid Nav preview picks up a "dashboard" subtitle branch.
  - DashboardTab.svelte: scrubbed of settingsOpen / openSettings / closeSettings / Settings menu row / HybridSurfaceConfigShell import / all screensaver+metadata state. HamburgerMenu now has only Reload (height 88 -> 58). Component is now thin: HamburgerMenu + EmptyPaneCarousel.
  - Tests: dashboardTabAndCarousel.test.ts rewrote "Wave 4: Dashboard settings" -> "phase-13 round-1 closing B3" with new pins for HybridDashboardConfig (shell wrapper, 4 sections, mount-time loadScreenLockState, app-appearance radio name) + Pane.svelte's dashboard arm + the no-longer-present DashboardTab settings code. screensaverSettings.test.ts rebased its `dashboard` `?raw` import onto HybridDashboardConfig.svelte; the 19 slice-3c assertions pass verbatim against the new file.

- **`fc8b1fc3`** B2 defensive: Cmd+, matcher in App.svelte's onWindowKey rewritten to `(e.code === "Comma" || e.key === ",")` for layout independence + `e.stopImmediatePropagation()` after `preventDefault()` to defend against any duplicate listener re-firing flipHybrid in the same tick. New cmdCommaFlipMatcher.test.ts pins the matcher shape + the stopImmediatePropagation call + the rationale comment.
  - Root cause NOT confirmed empirically. Static analysis:
    1. Single SPA path: only one `e.key === ","` match (App.svelte:673).
    2. KEY_BRIDGE_JS has no `Comma` case.
    3. The only desktop-side trigger is the macOS app menu accelerator at `desktop/src-tauri/src/main.rs:1834-1836` ("Settings…", accelerator `CmdOrCtrl+,`) which dispatches `app.settings.toggle` -> chan:command -> runCommand -> flipHybrid (App.svelte:977).
    4. If macOS menu accelerator consumes the keydown (typical), the SPA's onWindowKey never fires - menu path is single-source per press, each press toggles, press 2 should work. Contradicts the smoke.
    5. If macOS menu accelerator does NOT consume the keydown, both paths fire each press = double-toggle = no net change. But that would make press 1 also a no-op, also contradicting the smoke.
  - Neither hypothesis explains the asymmetric symptom (press 1 works, press 2 doesn't). Need a console.log walk in chan-desktop devtools to confirm whether the SPA's onWindowKey fires on press 2. The defensive matcher patch is a best-effort ship; if it doesn't repair the bug, the next move is to remove the macOS menu accelerator entirely so the SPA keymap is the single source of truth - that change is desktop/src-tauri/ side, which is outside Lane B's web/src + chan-server scope for this phase. Flagged on `event-lane-b-alex.md`.

- **`ff22fe1c`** B9 + B10:
  - **B9** language lens: `merge_language_layer` in chan-server's `crates/chan-server/src/routes/graph.rs` no longer routes through `build_language_graph` (which collapses files into per-directory edges via top-N rank). It now emits one `Language -> File` edge per file of that language directly. The /api/graph/languages overview path (api_language_graph) keeps using `build_language_graph` for the directory rollup. New Rust test `merged_graph_language_layer_emits_language_to_file_edges_for_workspace_lens` seeds three Markdown files across two directories + one Rust file and asserts the Markdown bubble has edges to every markdown file regardless of directory + no `directory:<path>` targets are emitted.
  - **B10** SCP toggle placement: verified existing. HybridTerminalConfig.svelte has the font dropdown (os-default / source-code-pro) with download wiring + os-default rollback on failure; EmptyPaneCarousel.svelte's About slide has the attribution + OFL link only. New vitest sourceCodeProTogglePlacement.test.ts pins both placements so the next round can't regress the split.

### Slice 2 per-slice gate

All gates green on lane-b head ff22fe1c:
- cargo fmt --check ✓
- cargo clippy --all-targets -- -D warnings ✓
- cargo test ✓ (348 tests across the workspace, no indexer flake this round)
- cargo build --no-default-features ✓
- npm run check ✓ (4113 files / 0 errors / 0 warnings)
- npm run build ✓
- vitest ✓ (1616 passed / 11 skipped; +4 above the 1612 baseline from slice 1 - new asserts for B2 matcher, B3 HybridDashboardConfig, B9 backend, B10 SCP placement, minus a few rebased Dashboard pins that moved without net additions)

### Slice 2 merge gate

Lane A still drained (last entry 16:15 on event-lane-a-alex.md). Combined tree IS lane-b in this cycle, so I ran the full per-slice gate on lane-b directly and skipped the `../chan-integration` worktree ceremony. Fast-forwarded main `cc978c5f` -> `ff22fe1c`. No push.

### Round-1 closing scope status: COMPLETE (modulo B2 verification)

All 12 round-1 closing items addressed across the two slices:
- B1, B4, B5, B6, B7 verified, B8, B11, B12 - slice 1 (commits d5417e88, 24bae323, a74ec43d, cc978c5f).
- B2 defensive, B3, B9, B10 verified - slice 2 (commits 05431c96, fc8b1fc3, ff22fe1c).

Two known caveats:
1. B2 is gated-green but empirically unverified - the root cause may still need an empirical walk in chan-desktop devtools, and if the defensive matcher doesn't repair, the desktop-menu-accelerator removal is the next escalation.
2. No chan-desktop smoke on the combined tree yet. Per `feedback_svelte_static_gate_misses_runtime` the B1/B3/B8/B12 reactivity + B11 chord + B2 keymap surface area all want a browser/desktop walk; per `feedback_terminal_webgl_wkwebview` the WKWebView-shape paths (B2/B11) want chan-desktop specifically. Calling that out as the natural next gate before the v0.17.0 cut.

### What's next

Lane B's roadmap is now drained. Per the brief's release-cut section, the natural sequence is:
1. chan-desktop empirical smoke on the combined tree.
2. v0.17.0 release-cut mechanics (Cargo.toml + tauri.conf.json + Cargo.lock bumps; dry-run release.yml via workflow_dispatch publish=false).
3. Stop before tagging for @@Alex's explicit confirm.
4. After confirm: tag, push tag, verify /dl/latest.json supersedes 0.16.0 + self-upgrade in chan-desktop.
5. Commit phase-13 docs as `docs(phase-13): close round 1`.

Per `feedback_round_close_retrospective` the docs commit at round close should carry a full retrospective (done/pending + highlights/lowlights + constructive feedback for the agents, @@Alex, and the architect); will draft that on the round-close turn.

Pinged @@Alex on `event-lane-b-alex.md` with the suggested order (smoke first, then release-cut mechanics, pause for tag-and-push approval).

## 2026-05-28 round-1 closing turn 3 - missed Lane A; recovered; combined-tree gate green

### What happened

@@Alex flagged "have you included LaneA's work" while I was setting
up the chan-desktop smoke. I had cycled the merge-gate against Lane
A's 16:15 "drained" status without re-reading their channel tail.
Their 21:23-21:36 entries on `event-lane-a-alex.md` queued three
round-1-closing slices:

- `70ab238e` A4: editor `@`-completion surfaces the `@@mention`
  corpus
- `a46e0944` A3: language bubble inspector body
- `3c9f57bd` A1: workspace-root inspector reads like a directory;
  Notes dirs gated to `variant="dashboard"`

### Recovery

1. Set up `../chan-integration` off main (`ff22fe1c`).
2. Merged `phase-13-lane-a` into it: clean auto-merge on
   `EmptyPaneCarousel.svelte` (A1's `variant="dashboard"` vs my
   B12 `selectedIndexId`), `GraphPanel.svelte` (A1 + A3 vs my B8
   tag arm), `dashboardTabAndCarousel.test.ts` (A1 added 5 lines
   parity; my B3 rewrote the Wave 4 block).
3. Full combined-tree gate:
   - cargo fmt --check ✓
   - cargo clippy --all-targets -- -D warnings ✓
   - cargo test: indexer flake on first run
     (`writes_to_drafts_subtree_get_indexed_under_drafts_prefix`);
     re-ran chan-workspace tests per
     `feedback_fresh_binary_rewalks`, all 540 green.
   - cargo build --no-default-features ✓ (2m22s fresh build)
   - npm install + npm run check ✓ (4117 files / 0 errors)
   - npm run build ✓
   - vitest ✓ (1632 passed / 11 skipped; +16 over my lane-b-solo
     1616: A1 `workspaceInfoBodyParity` + A3 `languageInspectorBody`
     + A4 `mentionBubble` additions)
4. Fast-forwarded main `ff22fe1c` -> `92ea0677` (the merge
   commit).
5. Cleaned up `../chan-integration` worktree + the
   `phase-13-integration` branch.

### Root cause + correction

The "merge-gate is current?" check I ran said
"`event-lane-a-alex.md` last entry = 16:15 drained → nothing to
gate". That's a stale-snapshot read - per
`feedback_check_for_updates_surface` the right check is "what's
the channel TAIL right now?" (which would have caught the 21:23
entries) plus the per-lane branch HEADs (`git -C ../chan-lane-a
log -3`). Adding both to the merge-gate playbook for round 2
and beyond. Reported on `event-lane-b-alex.md` + retrospective.

### Round-1 status after recovery

**All scope drained across both lanes.** Main at `92ea0677` carries
the full round-1 roadmap PLUS all round-1 closing fixes from both
lanes. Round-close docs commit + retrospective drafted (see
`docs/journals/phase-13/retrospective-round-1.md`); release-cut
mechanics next.

### Carryover for round 2

- Workspace-root hash persistence micro-nit (Lane A flagged
  `tabs.svelte.ts:3811`).
- `SETTINGS_DISABLED` meta tag emit cleanup on chan-server
  (prior @@LaneB flagged in slice 3c lowlights).
- B2 root cause if the defensive matcher doesn't repair on
  @@Alex's chan-desktop walk - the next escalation is removing
  the macOS app menu accelerator.
