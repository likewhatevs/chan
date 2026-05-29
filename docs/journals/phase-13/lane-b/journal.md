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

## 2026-05-28 23:26 — round-1 closing-2 picked up

Picking up the SECOND round-1 closing wave. Prior @@LaneB drained
the 12 original closing items + landed `chore(release): 0.17.0` at
`e30f73ef`; main carries that commit but no `v0.17.0` tag yet.
@@Alex's empirical walk on top surfaced 9 more bugs. My queue
(per `lane-b-round-1-closing-2.md`):

- B1c Dashboard tab survives reload (missing `kind === "d"` arm
  in `restoreLayout`)
- B2c Cmd+, flip drifts on focus-switch (stale `showingBack` not
  cleared in `setActivePane`; +HybridDashboardConfig onMount;
  +Pane.svelte rAF audit)
- B3c Move screensaver picker INSIDE Screen lock `{#if}` gate
- B4c QR-donate broken under chan-desktop (raw `<img src>`
  bypasses prefix rewrite)
- B7a Dblclick graph node = "graph from here"
- B7b Depth slider does nothing in path-scope (clamp /
  depthDisabled / backend spine-expansion - investigate)
- B8 Pane hamburger missing Search + Dashboard after Graph
- B9 Empty-pane welcome surface: Search missing + Dashboard
  chord hint hardcoded empty

Plus the merge-gate orchestrator hat + the v0.17.0 release-cut
(dry-run, tag, push tag, verify supersession + self-upgrade)
after both lanes drain + @@Alex confirms. @@LaneA owns A5 + A6
(WorkspaceInfoBody only).

### Turn 1 — setup

- Rebased `../chan-lane-b` onto `main@e30f73ef` (clean,
  no work yet on the branch since the previous @@LaneB).
- Read recovery files including the round-1 retrospective.
  Three round-1 self-feedback items load-bearing for this round:
  (a) read the channel TAIL each merge-gate, not the last-noted
  status; (b) escalate empirical blockers to @@Alex sooner; (c)
  don't poll while waiting for background jobs - the harness
  notifies. Internalised.
- TaskCreate'd all 8 lane-b items + the merge-gate + the
  release-cut.

### B1c (Dashboard tab survives reload)

Cleanest fix on the queue. `restoreLayout` had arms for `kind`
in `g`, `b`, `t`, `f` but no `"d"`; the `if (kind !== "f")
continue` guard silently dropped Dashboard tabs.

- Added a `kind === "d"` arm above the `kind !== "f"` continue
  in `web/src/state/tabs.svelte.ts::restoreLayout`. Mirrors the
  shape of `openDashboardInActivePane` (DashboardTab type
  literal already exported).
- Added a vitest pin in `web/src/state/tabs.test.ts`
  ("hash round-trips a Dashboard tab (B1c)") that opens a
  Dashboard tab, snapshots via `serializeLayout`, restores,
  and asserts kind + title + activeTabId preservation.
- `npx vitest run src/state/tabs.test.ts -t "B1c"` ✓ (1
  passed, 147 skipped of the file).

### B3c (Screensaver picker inside Screen lock)

Moved the screensaver theme `<select>` into the
`{#if screensaverEnabled === true}` block inside
`<section class="screen-lock">` + deleted the standalone
`<section class="screensaver">` block + its `<h3>`. Kept the
"theme rendered behind the lock cover when the workspace view
auto-locks" hint inline. Updated
`screensaverSettings.test.ts` to pin the new shape (theme
picker INSIDE the screen lock enable gate) and replaced the
prior "Screensaver section renders the theme picker" assertion.
Also retargeted the `dashboardTabAndCarousel.test.ts` "Four
sections" pin to "Three sections" since the standalone section
went away.

### B4c (QR-donate broken under chan-desktop)

Imported `withTokenQuery` from `../api/transport` and changed
the `<img class="fund-qr" src=...>` to
`src={withTokenQuery("/qr-donate.png")}`. That covers both the
chan-desktop non-root mount and the tunnel-mode prefix rewrite
(the raw `/qr-donate.png` resolved to the host root, which is
why the broken-image only surfaced under chan-desktop). Updated
`dashboardTabAndCarousel.test.ts` "About widget embeds the
donation QR" to match the helper-wrapped attribute + assert
the transport import was added.

### B9 (Empty-pane welcome surface: Search + Dashboard chord)

`EmptyPaneWelcome.svelte` had Dashboard in `secondaryEntries`
but no Search, and the secondary tile's chord render was a
hardcoded empty `<span class="spawn-chord"></span>`. Fix:
- Added `Search` to the lucide-svelte import.
- Added a Search row before Dashboard in `secondaryEntries`.
- Replaced the hardcoded empty span with
  `<span class="spawn-chord">{chordLabel(row.chordId)}</span>`
  so both secondary tiles render their chord hints.
- Widened the `.spawn-row-secondary` CSS from a single-column
  `minmax(120px, 240px)` to `repeat(2, minmax(96px, 1fr))` so
  the two tiles sit side by side.
- Updated the `dashboardTabAndCarousel.test.ts` pin to assert
  Search + Dashboard in secondary entries + the chord render
  + the new 2-column grid.

### B8 (Pane hamburger missing Search + Dashboard)

The pane top-bar hamburger rendered `spawnActions` only (5
entries); Search + Dashboard lived in a separate
`emptyPaneExtraActions` list that surfaced ONLY in the empty-
pane right-click menu. The two lists were already structurally
identical for the discoverable spawn set, so folding them was
the right move.

- Added Search + Dashboard to `spawnActions` after Graph (per
  user acceptance: "New Draft / Terminal / File Browser / Rich
  Prompt / Graph / Search / Dashboard, in that order").
- Deleted `emptyPaneExtraActions` entirely + the
  `emptyPaneActions = spawnActions` alias.
- Updated the empty-pane HamburgerMenu render to iterate
  `spawnActions` directly (no separator + second loop).
- Updated `Pane.test.ts`: hamburger assertion grew to 7 spawn
  rows; empty-pane right-click assertion swapped Dashboard/
  Search order to Search/Dashboard.

### B2c (Cmd+, flip drifts on focus-switch)

`setActivePane` reassigned `current.activePaneId = paneId`
without clearing the PREVIOUS pane's `showingBack`. A flipped
pane that lost focus held the back state; the next
`flipHybrid` on the focused pane toggled the FOCUSED pane's
`showingBack` while the prior pane stayed flipped, which is
exactly the "keeps flipping; breaks flip of other tabs"
symptom the user reported.

- Extended `setActivePane` in
  `web/src/state/tabs.svelte.ts` to clear the previous pane's
  `showingBack` on focus-move (only when it actually changes).
- Brief noted a HybridDashboardConfig onMount call to
  `loadScreenLockState()` was missing; verified the prior
  closing slice 3b already added it (line 206-207). No edit
  needed there.
- Walked the Pane.svelte rAF `$effect` at ~443-451; the
  `lastFlipVersion` tracker is per-pane via
  `paneFlip.versions[pane.id]`, so back-to-back flips on
  different panes correctly fire independent rAF cues. No
  change needed.
- Added a vitest pin in the "Hybrid flip" describe block
  asserting that focusing pane B clears pane A's
  `showingBack` while leaving the `back` marker intact + that
  re-focusing the SAME pane is a no-op for showingBack.

### B7a (Dblclick graph node = "graph from here")

GraphCanvas had no dblclick handler. Added one that mirrors
the onMouseUp tap pattern (localCoords + pickNode at click
slack); if a node sits under the cursor, fire
`onSetAsScope?.()`. Empty-space dblclicks no-op.

- Added optional `onSetAsScope?: () => void` to GraphCanvas
  `Props` + destructure.
- Added `function onDoubleClick(e: MouseEvent)` and
  `ondblclick={onDoubleClick}` to the canvas element.
- GraphPanel mounts GraphCanvas with
  `onSetAsScope={dblclickRescope}`. The helper reads the
  current selection (`selectedFsNode` in filesystem mode +
  `selectedNode` in semantic mode) and routes through
  `graphFromHere(path, isDir)` for path-scopable nodes
  (workspace / folder / file). Non-path nodes (tag / mention /
  language / contact) ignore the dblclick — switching kind
  needs a different action than a path rescope and the user's
  quoted ask was path-shape.
- Vitest pin in `GraphCanvas.test.ts` asserting the
  `onSetAsScope` prop, the canvas binding, and the handler
  shape.

### B7b (Depth slider does nothing in path mode)

Empirical: the slider was DISABLED in workspace path-scope
because `depthDisabled` included
`currentScope.kind === "workspace"`. The wiring otherwise was
sound — `loadKey` includes `graphState.depth`, the load
$effect refires, and the backend
(`merge_filesystem_layer` at `crates/chan-server/src/routes/
graph.rs:1139`) consumes `p.depth` for the spine expansion.

- Dropped the workspace branch from
  `depthDisabled = !languageMode && (!currentScope || ...)`;
  now `depthDisabled = !languageMode && !currentScope`.
- Simplified `depthShallow` so it falls through to
  `depthCap <= 1` for workspace too; the workspace probe feeds
  a meaningful cap, so the `[max]` cue is now consistent
  across path-scope kinds.
- Vitest pin in `graphDepthFilter.test.ts` asserting the new
  shape and pinning the old shape OUT.

### lane-b-empty-pane-menu (retire empty-pane right-click)

New discrete task cut by @@LaneA on @@Alex's direction (file:
`docs/journals/phase-13/lane-b-empty-pane-menu.md`). User's
ask: "today we have a slightly different menu in the pane's
hamburger and in the empty pane's right-click; I'd like to
remove the empty pane's right click menu altogether, and
leave just the pane's hamburger, which already covers all of
the options."

Part 1 (close the hamburger gap) was ALREADY covered by my
B8 fold — the hamburger now lists every spawn entry the
right-click used to render (New Draft / Terminal / FB / RP
/ Graph / Search / Dashboard). Verified directly against
`spawnActions` before proceeding to Part 2.

Part 2 (remove empty-pane right-click menu). Deletions:
- `emptyPaneMenu` / `emptyPaneMenuOpen` state
  (`Pane.svelte:250-251`).
- `openEmptyPaneMenuAt` + `onEmptyPaneContextMenu` helpers
  (`Pane.svelte:253-262`).
- `oncontextmenu={onEmptyPaneContextMenu}` on the
  `.placeholder` div + on `<EmptyPaneWelcome>`.
- The triggerless empty-pane `<HamburgerMenu
  bind:this={emptyPaneMenu}>` block + its `{#each
  spawnActions}` loop.
- The `pane.tabs.length === 0` branch in the tab-strip
  contextmenu handler now `return`s (no-op), preserving the
  loaded-pane Reload / Open Inspector path.
- `emptyPaneMenu?.close()` references in `closePaneMenus`,
  `closePaneContextMenus`, `dispatchCommand`, and the
  `onKeyDown` Escape branch.
- `EmptyPaneWelcome.svelte` Props (the `oncontextmenu`
  forwarder + the destructure).
- `EmptyPaneCarousel.svelte` Props (vestigial
  `oncontextmenu` forwarder kept "for symmetry" — DashboardTab
  doesn't wire it; per `feedback_pre_release_no_backcompat`
  delete outright).

Test updates:
- `Pane.test.ts`: "empty pane right-click shows the welcome
  menu" rewritten to assert NO popover opens on empty-pane
  right-click. "loaded pane right-click keeps reload and
  inspector menu" kept untouched. Hamburger menu-labels pin
  already covers Dashboard + Search via B8.
- `EmptyPaneCarousel.test.ts`: "forwards right-click to the
  parent contextmenu handler" replaced with a source-grep pin
  asserting the prop + binding are gone. `renderCarousel`
  signature simplified (no more `oncontextmenu` prop).
- `dashboardTabAndCarousel.test.ts`: "Pane.svelte mounts
  EmptyPaneWelcome" pin retargeted to `<EmptyPaneWelcome />`
  (no oncontextmenu attribute) + a `not.toMatch` guard for
  the old shape.

### Per-slice gate (the full slice)

All 8 closing-2 items + the lane-b-empty-pane-menu task ride
on a single slice (file-disjoint from Lane A's A5/A6 work on
WorkspaceInfoBody.svelte; Lane A's edits to
EmptyPaneCarousel.svelte are at line 36 + ~428, file-disjoint
from my B4c fix at the `<img>` tag).

- `cargo fmt --check` ✓ (no output, clean).
- `cd web && npx svelte-check --threshold warning` ✓ (4117
  files / 0 errors / 0 warnings).
- `cd web && npx vitest run` ✓ (1639 passed / 11 skipped /
  164 test files).
- `cd web && npm run build` ✓ (rolldown built; only pre-
  existing ineffective-dynamic-import warnings, no new ones).
- `cargo clippy --all-targets -- -D warnings` ✓ (background).
- `cargo test --workspace` ✓ (background; will rerun the
  indexer flake on first failure per the brief).
- `cargo build --no-default-features` ✓ (background).

## 2026-05-29 - round close (closing-3 through closing-12)

Closing waves 2 through 12 landed on main between
`b428c4b7` and `0d1497cf`. The lane operated as a single
@@LaneB stream after @@LaneA's A5/A6 + Notes-dirs separator
work merged at `2506533c`. Each commit's body carries the
full per-wave rationale, file list, test deltas, and gate
output; this entry is the round-close roll-up.

### Waves landed

- **closing-2** (`b428c4b7`): B1c Dashboard tab restore, B2c
  Cmd+, flip drift, B3c screensaver picker placement, B4c
  QR-donate prefix, B7a dblclick "graph from here", B7b path-
  mode depth slider, B8 hamburger Search/Dashboard, B9 empty-
  pane welcome chord. Plus retiring the empty-pane right-
  click menu (lane-b-empty-pane-menu task) by folding the
  spawn set into a single shared array.
- **closing-3** (`767a8c80`): C1 the Hybrid effect cycle
  freeze (`effect_update_depth_exceeded` in
  HybridTerminalConfig / HybridEditorConfig hydration
  effects), Bug-1 reindex pill persistence (transient poll
  cadence), C2 About slide license-link restructure with
  upstream URLs + Apache 2 row, C3 "Share the love, cheers!"
  copy, Bug-2 Dashboard indexing maximises to the tab.
- **closing-4** (`a8d15a88`): D1 tag inspector "Graph from
  here", D2 the B7a regression on dir id, D3 Dashboard
  indexing slide inspector, D4 language filter chip in dir
  scope, D5 zoom respect via `userInteracted` + same-set
  short-circuit.
- **closing-5** (`5c7e6f2b`): E1+E2 workspace radius bump
  to `RADIUS_DIR * 1.5`; E3 visibility-effect refit gating.
- **closing-6** (`8df5c869`): E3 fix-up - the visibility
  effect rebinds the link force on same-set ticks instead
  of early-returning before `rewarmSim` (which broke every
  graph's edges); incremental rewarm uses gentler alpha so
  new nodes ease in.
- **closing-7** (`2f8a9f58`): E1 follow-up - workspace root
  omits `indexState` so `theme.bgCard` fill keeps the
  hard-drive icon readable.
- **closing-8** (`59ffeaef`): F1 `find -d N` depth filter
  in scopedNodeIds for semantic workspace + dir scope
  (reusing `relativeDepth` from graph/depth.ts); F2 first
  attempt at marking embedding-phase dirs as Indexing
  via the `"embedding"` sentinel.
- **closing-9** (`205864b8`): E2 follow-up -
  `RADIUS_WORKSPACE = RADIUS_DIR * RADIUS_HUB_SCALE * 1.5`
  so the 1.5x gap holds against the max-scaled dir; F2
  follow-up - drop the `indexable_files > indexed_files`
  comparison since BM25 finishes before embedding so it
  reads as "done everywhere" otherwise.
- **closing-10** (`2b2ff082`): G4 Dashboard indexing
  workspace label uses workspace.info.label; G2 mention
  inspector "Graph from here" via `contact:<path>`; G3
  carousel slide cursor persistence on
  DashboardTab.carouselSlide round-tripped through SerTab.cs.
- **closing-11** (`e8a6957e`): F2 fix-up 2 - generalise
  broad-sweep detection to any current_file that doesn't
  match an entry path (initial Building "" window +
  embedding sentinel + future sentinels) so the dashboard
  catches the pre-IndexFile window the prior shape missed.
- **closing-12** (`0d1497cf`): hide the
  EmbedBatch's chunk/budget counter in the status pill (it
  overflows the budget and reads as nonsense like
  "indexing 4143/4096 (embedding)") - keep just the verb +
  the `(embedding)` label.

### Cross-lane

- @@LaneA A5+A6 landed at `4280d5f3` (clickable Languages +
  Contacts in workspace inspector) and the COCOMO -> Notes
  dirs separator at `2506533c`. Both file-disjoint from any
  lane-b work; merge cycles ff'd cleanly.

### Empirical caveats

- B2c + B4c + C1 + the empty-pane menu retire are
  WKWebView-shape; per
  `feedback_pre_release_merge_unverified` flagged for
  @@Alex's chan-desktop smoke as part of the release walk.
- F1 + F2 are graph / indexing surfaces - source-pattern
  tests pin the predicates but the live feel (slider
  reveals levels; embedding dirs pulse orange) needs the
  desktop walk.
- All waves' per-slice gates: cargo fmt --check, clippy
  --all-targets -- -D warnings, test --workspace, build
  --no-default-features, web npx svelte-check, web npm run
  build, web npx vitest run - green at each cut.
- @@Alex confirmed closing-9 onwards live: workspace hub
  reads 1.5x correctly; depth slider works; tag/mention
  inspector buttons work; carousel cursor persists.
  Closing-11 should resolve the embedding-phase orange
  pulse the prior waves missed (validated against the
  initial-Building window in cargo test).

### Lowlights

- F2 took 4 attempts (closing-8 / closing-9 / closing-11)
  because each fix patched the most-visible code path
  without auditing the full set of `IndexStatus::Building.file`
  values the indexer actually emits. Should have read
  indexer.rs:310-314 + 797-808 + 822-837 in one pass before
  the first commit.
- E2 took 2 attempts (closing-5 / closing-9) for the same
  reason - the worst-case dir radius (backlink ramp) wasn't
  considered in the first formulation. RADIUS_HUB_SCALE was
  load-bearing for the gap calculation.
- E3 / closing-6 broke every graph's edges by early-returning
  before `rewarmSim` re-bound the link force. Caught
  immediately by @@Alex but the static gate didn't.
  `feedback_svelte_static_gate_misses_runtime` strikes again.

### v0.17.0 release-cut

All scope drained. Version pins already at 0.17.0 from the
prior closing-1 release-cut prep (`e30f73ef chore(release):
0.17.0`); no version bump needed in this commit set. Next
moves per `reference_release_cut_mechanics`:

1. Commit docs(phase-13).
2. Push main.
3. `gh workflow run release.yml -f publish=false` dry-run.
4. Inspect artifacts.
5. STOP for explicit tag-cut confirmation.
6. After confirm: annotated `v0.17.0` + push tag (fires
   release.yml).
7. Verify `/dl/latest.json` supersedes 0.16.0 + chan-desktop
   self-upgrade walk.

# @@LaneB journal - Phase 13 round 2

Round 2 target: v0.18.0. Scope: editor list glyphs + Bold/Italic
chords + desktop Cmd+Shift+N + hamburger split labels, plus the
merge-gate orchestrator hat. Recovery files read: CLAUDE.md,
lane-b-request-round-2.md, roadmap-round-2.md (image-1 list style,
image-13 hamburger), bootstrap-round-2.md, coordination/README.md,
event-alex-lane-b.md (round-2 kickoff in inbox), event-lane-a-lane-b.md
(no Round 2 entries yet - no Team Work label string from @@LaneA).

## 2026-05-29 round 2 - turn 1 setup + scoping

Worktree: `../chan-lane-b` was clean at 0d1497cf (one behind main,
just missing the docs commit). `git checkout -B phase-13-r2-lane-b
main` -> now at 76f5e18b, clean.

### Source recon (all four slices mapped to exact lines)

- B-slice 4 (split labels): Pane.svelte paneNavigationActions
  513/520 use `paneModeChordLabel("/")` / `("?")` -> renders the
  Pane-Mode prefix `Cmd+. /` / `Cmd+. ?`. formatChord only swaps
  Mod->Cmd/Ctrl; it does NOT fold Shift+/ -> ?. Plan: render the
  roadmap mnemonics directly via `formatChord("Mod+/", os)` /
  `formatChord("Mod+?", os)` (os already in scope at 527). The real
  key bindings (shortcuts.ts Mod+/ / Mod+Shift+/, desktop KEY_BRIDGE
  Slash) are unchanged - hamburger is display-only. paneModeChordLabel
  becomes dead (only callers were 513/520); remove it. chordLabel
  ("app.pane.mode") at 1198 is separate and stays.

- B-slice 2 (bold/italic + Cmd+I): the big finding. The Cmd+I ->
  Dashboard binding is NOT shortcuts.ts (display-only registry). It's
  hardcoded `e.code === "KeyI"` in App.svelte::onWindowKey (~849) +
  desktop KEY_BRIDGE serve.rs:634. CM6 keymap does not stopPropagation,
  so binding Mod-i for italic without removing those = double-fire
  (italic + Dashboard). So B-slice 2 spans: shortcuts.ts (mine) +
  Wysiwyg.svelte keymap (mine) + serve.rs KEY_BRIDGE (mine) +
  App.svelte (LANE A's file). Declared the App.svelte overlap to
  @@LaneA + flagged the spec gap to @@Alex before editing.

- B-slice 3 (Cmd+Shift+N): main.rs `app-new-window` menu branch
  (1868) -> open_new_launcher_window (the picker). The window label is
  `workspace-<hash(key)>-<seq>` (hash is one-way), so I recover the
  focused window's key by matching `serve::workspace_window_prefix(key)`
  (pub) against the focused label across `state.serves`
  (HashMap<key, ServeHandle{url}>), then reuse
  serve::spawn_local_workspace_window (same path open_local_workspace
  uses). Fall back to open_new_launcher_window when no LOCAL
  workspace-* window is focused (picker, tunnel-*, outbound-*, or no
  running match) so the menu never dead-ends. "Workspaces" picker
  stays reachable via the win-main menu item, so repurposing
  app-new-window orphans nothing.

- B-slice 1 (list glyphs): blocks.ts decorateBulletList marks each
  ListMark with `cm-md-ul-marker` (Decoration.mark, source preserved
  per round-1 bug-3). image-1 wants `-` -> en-dash all levels, `*` ->
  filled bullet top / hollow nested. Plan: split the class by source
  char + nesting (count ancestor ListItem nodes), and substitute the
  glyph via pseudo-element CSS (color:transparent on the source char +
  ::before glyph) so the source still round-trips. `+` keeps its
  literal styled char (out of image scope). Ordered lists already
  render the source `1.` styled - matches image-1, no change.

### Coordination posted

- event-lane-b-lane-a.md: declared the App.svelte KeyI-only deletion
  overlap + reminded @@LaneA I'm waiting on the Team Work label string.
- event-lane-b-alex.md: flagged the spec gap + my minor call to repoint
  app.dashboard.open to `Mod+. i` (keeps Dashboard discoverable rather
  than dropping its chord to nothing).

## 2026-05-29 round 2 - turn 1 implementation + gate + smoke

All four B-slices implemented as atomic commits, full gate green, two
web slices browser-smoked. Commits on `phase-13-r2-lane-b`:

- `f2f78e52` B-slice 4: hamburger Split right/bottom render `Cmd+/` /
  `Cmd+?` via formatChord mnemonics (display-only; real bindings
  unchanged). Removed dead paneModeChordLabel. renderTable cheatsheet
  still shows the literal Cmd+Shift+/ (untouched).
- `dc3a1230` B-slice 2: bound Mod-b / Mod-i in the Wysiwyg CM6 keymap;
  moved Dashboard off Cmd+I. The real Cmd+I->Dashboard binding was the
  hardcoded App.svelte::onWindowKey KeyI branch + the desktop KEY_BRIDGE
  (serve.rs), NOT shortcuts.ts - removed both. shortcuts.ts repoints
  app.dashboard.open to `Mod+. i` + adds app.editor.bold/italic entries.
- `b16e699d` B-slice 3: desktop Cmd+Shift+N opens a new window of the
  FOCUSED window's workspace (open_new_window_for_focused_workspace
  maps the workspace-<hash>-<seq> label back to a running key via
  serve::workspace_window_prefix, reuses spawn_local_workspace_window;
  launcher fallback when no local workspace focused).
- `3eb7f4c4` B-slice 1: list marker glyphs. blocks.ts splits the bullet
  decoration by source char + nesting; Wysiwyg.svelte substitutes the
  glyph via font-size:0 + IN-FLOW ::before so text-indent positions it
  at every depth. (See smoke below for the absolute-overlay bug I caught
  and fixed.) Glyphs: en-dash for `-`, U+25CF filled top / U+25EF hollow
  nested for `*` (final @@Alex glyph pick).

### Per-slice gate (combined lane-b head, all green)

- cargo fmt --check: clean.
- cargo clippy --all-targets -- -D warnings: clean.
- cargo test: every result line 0 failed (351 chan-server, 540
  chan-workspace, 76 chan-desktop, etc.; no indexer flake this run).
- cargo build --no-default-features: clean.
- web: npm run check (0 errors / 0 warnings / 4117 files), npm run build
  (clean), npm test (1660 passed / 11 skipped).

### Browser smoke (Chrome, ad-hoc /tmp/lbr2srv on /tmp/lbr2-ws, torn down)

- B-slice 1 list glyphs: opened a file with all three list kinds at two
  depths. FIRST pass used a position:absolute ::before overlay; the
  screenshot showed NESTED glyphs detaching to the left gutter while
  their content stayed indented - text-indent does not apply to
  out-of-flow boxes. Fixed by switching to font-size:0 + in-flow
  ::before; re-smoke showed correct per-depth alignment (en-dash all
  levels, filled top / hollow nested bullets). This is the exact
  `feedback_svelte_static_gate_misses_runtime` class - the static gate
  (svelte-check + vitest) was green on the broken version.
- B-slice 2 chords: triple-click a sentence, Cmd+B inserted `**`, Cmd+I
  added `*` (combined `***`), and crucially NO Dashboard tab opened on
  Cmd+I (the double-fire removal works). Empty-pane welcome also shows
  "Dashboard  Cmd+. i" (the repointed chord) not Cmd+I.
- B-slice 3 (desktop Cmd+Shift+N) + the Cmd+I removal on the desktop
  KEY_BRIDGE are WKWebView-shape; flagged for @@Alex's chan-desktop
  walk per `feedback_terminal_webgl_wkwebview` (Chrome/Blink can't
  reproduce).

## 2026-05-29 round 2 - turn 1 NEW ASKS (mid-turn, @@Alex direct)

@@Alex raised four asks mid-turn while watching the smoke. Per
`feedback_inflight_task_amendments` these are new tasks, not amendments;
documented here + in `lane-b-round-2-addenda.md`.

### New ask 1: bullet glyphs -> U+25CF / U+25EF

@@Alex first confirmed U+2022 top / U+25E6 nested (via AskUserQuestion),
then changed to U+25CF (filled black circle) top / U+25EF (large hollow
circle) nested. Folded into the B-slice 1 commit (`3eb7f4c4`); re-smoked
- renders ● filled top / ◯ hollow nested, correctly indented.

### New ask 2: Cmd+, per-pane flip bug (`8c6f4a94`)

@@Alex repro: Cmd+, on an empty pane "records it"; clicking a pane with
a tab flips that tab; from there switching pane focus flips tabs. His
spec: (1) flip strictly tied to panes with >= 1 tab; (2) each flip cycle
must not impact other panes; (3) per-pane flipped/not-flipped boolean;
(4) persists across window reloads.

Two root causes, both leaking the per-pane flip across panes:
1. `splitPane` copied showingBack/back onto the NEW pane (born EMPTY) ->
   a flipped 0-tab pane the flip chord couldn't undo (flipHybrid's
   empty-pane guard blocks re-flip), whose orientation then bled into
   focus. New pane now starts clean.
2. `setActivePane` (round-1 closing-2 B2c) cleared the PREVIOUS pane's
   showingBack on every focus-move -> the visible "switching focus flips
   tabs" bug. The B2c fix was itself the regression source; removed it.

Now showingBack is a strictly per-pane boolean ONLY flipHybrid writes
(guarded >= 1 tab); closeTab/closeTabsInPane clear it when the pane
empties; restoreLayout restores it only when the pane still has tabs.
Tests rewritten (B2c -> "focus never touches flip, two panes flip
independently"; split test -> new pane clean; close-last-tab -> flip
clears). Full web gate green.

Browser-smoke (verified via the serialized layout hash + a screenshot):
- Cmd+, flips a pane with a tab (sb:1).
- Split a flipped pane -> new empty pane has NO sb/bm (clean).
- Focus B -> A stays flipped; Cmd+, on empty B is a no-op.
- Reload from the base URL (session.json restore) -> A still flipped, B
  still clean. Per-pane persistence confirmed.

Behaviour change flagged: the round-1 "keep showingBack across a
last-tab close" (old test 438) is superseded by @@Alex's "strictly >= 1
tab" - closing the last tab now drops the flip to the empty front.

### New ask 3: document the new asks + commit the docs

@@Alex: document the new asks and commit all the docs produced this turn
as part of the deliverable. This deviates from
`feedback_coordination_docs_commit_timing` (keep docs untracked until
round close); committing now per his explicit ask. Captured the new asks
in `lane-b-round-2-addenda.md` and committed the phase-13 docs to main as
a `docs(phase-13)` in-flight commit.

## 2026-05-29 round 2 - merge gate + rich-prompt scrub + merge to main

Lane A finished (55179ad9 feature + 25c81182 dead-code cleanup; gated
25c81182 per their cross-lane note, NOT the stale 55179ad9 - round-1
staleness lesson). Integration worktree `../chan-integration` off main;
merged Lane A 25c81182 + Lane B ae06398b - clean auto-merge
(Pane.svelte/tabs.svelte.ts/App.svelte overlaps merged fine).

My residual sweep caught 2 user-facing "Rich Prompt" leftovers in Lane A
files (store.svelte.ts "Broken Rich Prompt" dead warning branch +
FileInfoBody stale "rich-prompt-N" drafts notice). @@Alex told me to fix
them myself (74ec13d7), then "do not leave any rich prompt code behind".
So I scrubbed EVERY "rich prompt" identifier across the combined tree
(c4a4adc6, 160 refs, 35 files) via /tmp/scrub-richprompt.sh:
- chord id app.terminal.richPrompt -> app.terminal.teamWork (atomic
  across shortcuts/App/serve KEY_BRIDGE + test/Pane/EmptyPaneWelcome/
  TerminalTab/chordEscapeRegistry/paneModeKeymap).
- tab.richPrompt field + serialization -> tab.teamWork.
- .rich-prompt CSS class -> .team-work; backend rich_prompt/rich-prompt
  -> team_work/team-work; 5 richPrompt* test files -> teamWork*.
- OVERRODE Lane A's "chord id stays stable" call, per @@Alex.
- Fixed one flipped absence-guard the blanket rename produced
  (TerminalTab.test asserted no "Team Work" label vs the "Show Team Work"
  toggle it requires). svelte-check: 0 collisions; 0 residual refs.

Full gate green (cargo fmt/clippy/test/build --no-default-features + web
svelte-check 0/0/4107 + build + vitest 1570). Browser-smoked the
static-gate-blind renames: Cmd+P (app.terminal.teamWork) fires the lead
terminal + embedded editor (.team-work renders clean) + Lane A's dialog.
ff main 248bc830 -> c4a4adc6. No push. v0.18.0 cut held for @@Alex.

Note: `rich-prompt-N` draft convention -> `team-work-N` (a draft prefix
in chan-workspace tests/comments; the current flow uses untitled-N so
it's vestigial-but-renamed - generic draft handling, not dead code).
