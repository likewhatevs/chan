# Phase 13 round 1 retrospective

Round close: 2026-05-28. Main at `92ea0677` (Lane A's A1/A3/A4
merged into the round-1 closing slices from Lane B). Release
target: **v0.17.0**.

Per `feedback_round_close_retrospective` and
`feedback_curated_status_reports`: this is the curated round-close
view (highlights / lowlights / contention + honest feedback for
agents, @@Alex, and the architect role). Detail lives in the
per-lane journals + the coordination channel tails.

## Scope status

### Done

**Lane A (content surfaces)** — full roadmap-round-1 plus
round-1 closing follow-ups:
- Bug 1: new-doc cursor focus (`b2ef3f3b`)
- Bug 2: fresh-draft "Unsaved changes" prompt suppression (`b2ef3f3b`)
- Bug 3: list marker source preservation (hyphen / `*` / number)
  (`b2ef3f3b`)
- Bug 4: terminal Shift+Enter newline (`b2ef3f3b`)
- Inspector slice: absolute path + COPY button + workspace-root
  parity (`ad184179`)
- KIND chip wiring slice 4a: clickable path + tag chips
  (`39fd3373`)
- KIND chip wiring slice 4b: clickable contact + language chips
  (`08b28da8`)
- A4: `@`-completion surfaces the `@@mention` corpus (`70ab238e`)
- A3: language bubble inspector body (`a46e0944`)
- A1: workspace-root inspector reads like a directory; Notes
  dirs gated to `variant="dashboard"` (`3c9f57bd`)
- A2 + A5: investigated and reported as already-satisfied in
  current code; no change needed.

**Lane B (structural shell + merge-gate)** — full roadmap-round-1
plus all 12 round-1 closing smoke items:
- Pane bugs slice: focus-ring thickness parity + outer-halo
  wobble (`33e180c9`, `975efe40`, `a5c589b5`, `ea23a691`)
- KIND slice 2a: contact + language helpers + `kind=` title
  prefix (`24f1f31d`)
- KIND slice 2b: contact + language lens semantics
  (`11e5fb37`)
- Dashboard 3a: InfographicsTab -> DashboardTab rename
  (`fa8c0c25`)
- Dashboard 3b-1: About + Workspace widgets (`b84c1507`)
- Dashboard 3c: Settings flip-back + Cmd+, rebind +
  SettingsPanel retired (`0bb01492`)
- Dashboard 3b-2: indexing slide reuses GraphCanvas read-only
  (`5a241f0f`)
- B1 + B4: graphTabLabel keeps `kind=` prefix; flipHybrid
  empty-pane guard (`d5417e88`)
- B5 + B6: Infographics -> Dashboard rename + menu reorder +
  DashboardTab aria (`24bae323`)
- B7: verified `/qr-donate.png` serves (no code change)
- B8: tag lens BFS now bidirectional (`a74ec43d`)
- B11 + B12: Dashboard right-click Reload menu; indexing graph
  fit-on-resize + selection labels (`cc978c5f`)
- B3: HybridDashboardConfig extracted; Pane.svelte dashboard
  back-of-card arm (`05431c96`)
- B2 defensive: layout-independent Cmd+, matcher +
  stopImmediatePropagation (`fc8b1fc3`)
- B9 + B10: language layer emits per-file edges; SCP toggle
  placement pinned (`ff22fe1c`)

### Pending / carryover

- **B2 empirical verification**. The defensive matcher is
  gated-green but the root cause behind "press 2 is a no-op" was
  not confirmed without chan-desktop devtools access. If the
  defensive patch doesn't repair the bug on @@Alex's smoke,
  next step is removing the macOS app menu accelerator
  (`desktop/src-tauri/src/main.rs:1834-1836`) so the SPA's
  keymap is the single source of truth for Cmd+,. That edit is
  desktop-side and outside Lane B's stated `web/src` +
  `chan-server` scope.
- **chan-desktop empirical smoke** on the combined tree (B1
  reactivity, B3 back-of-card render, B8 tag lens render, B11
  Reload IPC, B12 indexing graph behaviour, B2 keymap, B9
  language-lens render, A4 `@`-picker, A1 inspector +
  Dashboard slide 1 variant). Chrome MCP doesn't drive
  WKWebView from CLI; @@Alex owns the live walk per
  `feedback_terminal_webgl_wkwebview`.
- **Workspace-root hash persistence micro-nit** flagged by
  Lane A: `tabs.svelte.ts:3811` drops `id=""` from graph-tab
  hash via a falsy check; reload-only. Lane B serialization
  scope. Round-2 candidate.
- **SETTINGS_DISABLED meta tag** still emitted by chan-server
  as an SPA meta tag even though slice 3c retired the
  SettingsPanel; small cleanup queued by the prior @@LaneB for
  round 2.
- **roadmap-round-2.md** is @@Alex's already-drafted plan for
  the next round (Team Work rework, Cmd+Shift+N native chord
  for current workspace, etc.). Tracked.

## Highlights

- **Round closed end-to-end across both lanes**: every item
  in `roadmap-round-1.md` shipped; every item from the
  `round-1-closing-tests.md` smoke report addressed; no scope
  creep declined or punted past round close.
- **File-disjoint coordination held**: the only inside-file
  contention in this round was Lane A's `variant="dashboard"`
  single-line touch on Lane B's `EmptyPaneCarousel.svelte`,
  declared on `event-lane-a-lane-b.md` ahead of editing per
  the `feedback_lane_boundaries` + `feedback_shared_worktree_commits`
  conventions. Auto-merge on the combined tree was clean (ort
  strategy resolved every overlap without manual edits).
- **The KIND graph rework landed coherently across surfaces**:
  Lane B's backend `?kind=` discriminator + the four
  `openGraphFor{Path,Tag,Contact,Language}` helpers + the
  `graphTitle()` prefix slot in (slice 2a/2b) plus Lane A's
  Inspector `KindChip` onClick API (slice 4a/4b) wired into
  the per-kind BFS lenses (tag bidirectional in B8, language
  per-file in B9). The end-to-end click-on-chip -> lens-shaped
  tab title -> filtered subgraph paths all walk on a single
  consistent vocabulary.
- **The Dashboard back-of-card extraction (B3) made the
  Cmd+, rebind coherent.** Pre-B3 the Dashboard had a
  parallel `settingsOpen` path that worked from right-click
  but not from Cmd+, (Pane.svelte's back-side switch lacked a
  dashboard arm). Post-B3 there is one canonical surface
  (`HybridDashboardConfig`) and one canonical entry point
  (Cmd+, -> `flipHybrid`).
- **The KIND lens semantics now match the roadmap spec** end-to-end
  on the tag + language lenses: tag bidirectional BFS picks up
  backlinks (the regression that surfaced "0 docs in tag lens"
  in the smoke); language layer emits Language -> File edges
  directly so the lens renders the bubble + every file
  regardless of directory rollup.

## Lowlights

- **B2 root cause was not pinned down.** The bug was reported
  on chan-desktop (WKWebView); Chrome MCP from CLI can't drive
  WKWebView; without empirical access I exhausted the static
  analysis paths (App.svelte onWindowKey single-source,
  KEY_BRIDGE_JS has no Comma case, macOS app menu accelerator
  is the only desktop trigger and it either consumes the
  keydown or doesn't). Shipped a defensive matcher patch
  (layout-independent + stopImmediatePropagation) flagged as
  unverified per `feedback_pre_release_merge_unverified`.
  Round-2 risk if it doesn't repair.
- **I missed Lane A's three round-1-closing slices on the
  first merge-gate cycle.** Their 21:23-21:36 entries on
  `event-lane-a-alex.md` queued A4 / A3 / A1 as ready to
  merge, but I cycled the gate against my last-known "Lane A
  drained at 16:15" status without re-reading the channel
  tail. @@Alex's "have you included LaneA's work" nudge
  caught the gap. Reprocessed the cycle, integrated their
  three commits, re-ran the full gate. Self-feedback below.
- **No chan-desktop empirical smoke from Lane B.** The
  `feedback_svelte_static_gate_misses_runtime` +
  `feedback_terminal_webgl_wkwebview` reminders applied to
  five of the round-1 closing items (B1/B3/B8/B11/B12); I
  shipped them gated-green and queued the WKWebView walk for
  @@Alex per `feedback_pre_release_merge_unverified`. Better
  than holding the merge but a known empirical-debt at round
  close.
- **The Pane.svelte structure surprise**. Working through B2,
  I read `onWindowKey` and convinced myself that line 428
  closed it (matching indent); the function actually nests
  `handlePaneModeKey` inside itself and runs for ~280 more
  lines, with the Cmd+, branch at line 673 inside that body.
  Took a careful brace-by-brace walk to recover. Pre-existing
  code shape (not from this round), but flagging because
  future agents working B2-style debug paths will hit the
  same surprise.

## Constructive feedback

### To Lane A

- **The "already satisfied" investigation pattern was the
  right call** on A2 + A5. Refusing to fabricate a change
  when current code already handles the symptom is exactly
  the right move pre-release; the lane-a/journal.md +
  channel write-up cited concrete evidence (file counts,
  the FS-mode vs semantic-mode distinction) so @@Alex could
  redirect if the diagnosis was wrong. Keep doing that.
- **Cross-file edit declarations were clean** -
  `variant="dashboard"` on EmptyPaneCarousel.svelte was
  declared on `event-lane-a-lane-b.md` *before* editing
  with the rationale + "LaneB can reclaim at merge-gate"
  escape hatch. That's how the convention is meant to work.
- **Pace note**: the round-1-closing slices landed in a
  tight 21:23-21:36 window after a long quiet stretch
  since 16:15. The condensed delivery is great for
  throughput but it widened the window where the
  merge-gate (me) missed them. Maybe a brief "queueing N
  ready slices" heads-up entry on the channel ahead of the
  full reports would have caught my eye sooner; a single
  pointer line costs little and pre-loads the merge-gate.

### To Lane B (myself + the prior @@LaneB)

- **Self: read the channel TAIL before each merge-gate, not
  the last-noted status**. The 16:15 "Lane A drained" was
  a snapshot; agents append in the round-close window
  exactly when the queue gets the most movement. Bake this
  into the merge-gate playbook so the next round
  systematically grep-tails `event-lane-{a,b}-alex.md`
  before opening `../chan-integration`.
- **Self: B2 should have been escalated to @@Alex SOONER**
  rather than after I'd burned ~15 minutes on hypothesis
  scaffolding. The brief explicitly named live-launch
  debug as the prescribed first step; once Chrome MCP nav
  was denied I should have asked for the chan-desktop
  walk + devtools console output and parked the slice
  while continuing with the structural items. Defensive
  patches are a fallback, not a substitute for empirical
  evidence.
- **Self: be careful with the "let me poll once more" cadence
  while waiting for background jobs**. Each poll is a token
  spend; the harness notifies on completion. I burned 5+
  polling rounds on the integration-tree cargo build.
  Default to NOT polling unless a notification suggests a
  problem.
- **Prior @@LaneB: the slice 3c sequencing was clean** but
  the Cmd+, behaviour shipped with no chan-desktop smoke
  per the their own lowlight call-out. Phase-13's
  empirical-gate convention is: any chord rebind affecting
  WKWebView-shape paths goes through a desktop walk
  before "ready to merge", not after. That convention
  would have surfaced B2 before the round-1 closing smoke.

### To @@Alex

- **The round-1 roadmap (`roadmap-round-1.md`) was the
  right shape** - one document covering both lanes with
  the load-bearing decisions (KIND graph layers, Dashboard
  rename + carousel auto-resize, Cmd+, rebind, Settings
  overlay retirement) called out individually. Easy to
  parse, easy to slice against. Keep the shape.
- **The round-1 closing brief was specific where it
  needed to be (root-cause hypotheses, file pointers, line
  numbers) and explicit about pre-release constraints
  (`feedback_pre_release_no_backcompat` etc.).** That
  upfront precision saved a lot of re-asking. For round 2,
  same shape.
- **The smoke report (`round-1-closing-tests.md`) directly
  walking the brief's checklist with @@Alex words verbatim**
  was load-bearing for the closing slice. The pattern
  "what was tested / what the user reported / what
  screenshot showed it" mapped 1-to-1 onto the B1-B12
  triage. Keep this pattern for the v0.17.0 acceptance
  walk and beyond.
- **One genuine ask**: when you queue ready-to-merge
  slices in a tight window (as both lanes did at round
  close), a single "lane-a queued 3, lane-b queued 4,
  please re-gate" channel poke would have pre-empted my
  missed-tail issue. The merge-gate role currently relies
  on the lanes self-announcing; an Alex-side announcement
  bracket would close the gap.

### To the architect role

(In phase-13, this is @@Alex wearing the planning hat
that authored the round-1 roadmap + the round-1 closing
brief. Phase-12-style "@@Architect" role isn't a separate
handle this round.)

- **Per-item file-path + line-number scaffolding in the
  closing brief was the right level of structure.** Saved
  multiple re-greps. Keep this for the round-2 brief.
- **The "may need backend changes; coordinate with Lane A"
  flag on B9** was useful but it would have been more
  useful with a hard direction ("Lane B owns the workspace
  lens emission; if Lane A needs a parallel /api/graph
  change for their inspector, surface a separate slice").
  Ambiguity around shared backend surfaces is the most
  common source of merge-time surprise; pre-resolving
  who-owns-what at brief time is cheap.
- **Consider hard-pointing the empirical-verification
  responsibility per item** at brief time. The current
  pattern leaves it implicit ("smoke before tagging") and
  agents land gated-green slices with unverified runtime
  paths. A brief field "empirical owner: <lane> / @@Alex"
  per item would close that gap without forcing every
  agent to do every walk.

## Release-cut readiness

After the docs commit, the v0.17.0 release-cut sequence per
the closing brief + `reference_release_cut_mechanics`:

1. Bump `Cargo.toml [workspace.package].version` -> `0.17.0`
2. Bump `desktop/src-tauri/tauri.conf.json` `"version"` -> `0.17.0`
3. Refresh `Cargo.lock` via `cargo build`
4. Dry-run via `gh workflow run release.yml -f publish=false`
5. Inspect dry-run artifacts
6. STOP. Wait for @@Alex confirm to tag.
7. After confirm: tag `v0.17.0` (annotated) on main
8. Push the tag (this fires `release.yml`)
9. Verify `/dl/latest.json` supersedes 0.16.0 + chan-desktop
   self-upgrade walk 0.16.0 -> 0.17.0

`feedback_merge_is_not_push`: no remote push without an
explicit ask. The tag-push at step 8 IS the explicit ask.
