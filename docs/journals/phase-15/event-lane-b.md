# Event channel — @@LaneB (round 2)

Append-only. Newest at the bottom.

## 2026-05-31

- **wave-1 code complete + static-gated green.** A6, A7, A4, A3, BUG-GRAPH all
  implemented. svelte-check 0/0, `vite build` clean, vitest 1576/1577 (the one
  failure is @@LaneD's uncommitted Cmd+R WIP in App.svelte/shortcuts.ts/
  keymap.ts — not a file I touched). Files: EmptyPaneCarousel.svelte,
  dashboard/AboutSlotConfig.svelte, screensaver/PlainScreensaverPreview.svelte
  (new), FileInfoBody.svelte, InspectorBody.svelte, DashboardTab.svelte,
  GraphPanel.svelte, tabs.svelte.ts (DashboardTab slot region),
  dashboardTabAndCarousel.test.ts. Detail in round-2-lane-b-journal.md.
- **REQUEST -> @@Architect: seeded test drive for browser-smoke.** Need a
  drive with nested directories (for BUG-GRAPH "Graph from here" + the A4
  index-graph inspector dir actions) to smoke A3 (menu / skip-rotation / min-one
  / reload persistence), A7 (live preview swap), A4 (Show Directory / Graph from
  here / New Terminal + no-Upload, and no regression on FB/editor/Graph-tab),
  and BUG-GRAPH (dir -> filesystem files + double-click expand). Holding the
  merge until smoked.
- **COORDINATION -> @@Architect: A3 tab-title menu surface.** Implemented the
  slot menu on the DashboardTab body right-click (in scope). The roadmap also
  wants it reachable from the tab TITLE, which lives in Pane.svelte (not mine).
  Needs a cross-lane task to whoever owns Pane.svelte this round, or a call to
  leave the body-menu as the surface.
- **CK-CAROUSEL note -> @@LaneD (via @@Architect):** for `cs dashboard
  --carousel-off`, the per-tab field is now `disabledSlots?: number[]` on
  DashboardTab with `DASHBOARD_SLOT_COUNT`/`dashboardSlotEnabled`/
  `toggleDashboardSlot`/`firstEnabledSlot`/`nextEnabledSlot` helpers exported
  from tabs.svelte.ts. `--carousel-off` (turn the whole carousel cycling off,
  not a slot toggle) maps better to the existing server-persisted
  `empty_pane_carousel_cycling` preference than to `disabledSlots`. Let's
  confirm the intended semantics with @@LaneD before they wire it.

- **wave-1 SMOKED + MERGED (local, on main).** All five items empirically
  verified on my binary @ 8822 against a small seeded drive (the heavy repo
  clone wedges preflight = the CK-INDEX-IDLE embedding bug; small drive idles in
  ~6s). A6 license-on-version-row; A7 live theme preview swap (Default enso <->
  Matrix frame + hint); A4 dir inspector (Show Dir + Graph + New Terminal, no
  Upload, Download kept; New Terminal cwd = dir; Graph-tab inspector still shows
  Upload = no regression); A3 menu from BOTH body + tab title, toggle/clamp/
  min-one-guard/dots/reload-persistence confirmed; BUG-GRAPH both symptoms
  (filesystem-mode switch + double-click expand). Commits: `fc1730e5`
  (BUG-GRAPH) + `37d68bef` (part-1 + autoRotate). Detail + caveats in
  round-2-lane-b-journal.md. NOT pushed.
- **CK-CAROUSEL RESOLVED -> @@LaneD:** field is `tab.autoRotate?: boolean`
  (default true), NOT `disabledSlots` (per @@Architect ruling; I'd flagged the
  same). Set `tab.autoRotate = false` in `store.svelte.ts` `open_dashboard`
  handler (~733-745, mirror the `carousel_index` path) when `--carousel-off`.
  Carousel already gates auto-advance on it.
- **REBASE -> @@LaneD:** `tabs.svelte.ts` on main now carries my DashboardTab
  region (disabledSlots + autoRotate + helpers + serialize/restore/clone).
  Rebase your TerminalTab/keymap/TeamWork hunks on `37d68bef`.
- **A3 tab-title hook needed NO Pane.svelte edit** (contra the dispatch
  assumption): the tab menu renders per-tab-component off the shared `tabMenu`
  state, and Pane's `openTabMenu` trigger is already kind-agnostic. Hook lives
  in DashboardTab. @@LaneD's 2 Pane.svelte lines untouched.
- **Data point -> @@LaneC (CK-INDEX-IDLE):** heavy-clone initial bulk index in
  a debug build pegs CPU (80.9%) and pins the counter at 4099/4096 - slow, not
  an OS-level hang; small drive completes fine. The wedge you're chasing is the
  post-draft *reindex* path, distinct from initial bulk embedding.
- **Next:** wave-2 BUG-EDITOR.

- **BUG-EDITOR done + committed `d861b61b`** (candidate (a): `geometryChanged`
  in the conceal walker's recompute condition + a source-pattern test). Gate:
  svelte-check 0/0, vitest 1584/1584, build clean.
- **!! BUG-EDITOR is EMPIRICALLY UNVERIFIED IN CHROME -> @@Architect (decision):**
  the bug does NOT reproduce in Blink. I built + served the PRE-FIX binary and
  ran the repro (tall raw-marker doc, switch editor->editor->back, no click):
  lower blocks rendered concealed anyway; post-fix binary also clean. It's a
  WKWebView layout-measure race (chan-desktop only), same class as the terminal
  WKWebView garble bug. I can't automate a WKWebView repro (Chrome MCP = Blink).
  Verified: no Chrome regression. Pending: a **chan-desktop manual verify**
  (recommend routing to @@Host). I committed it anyway as gated-green +
  root-cause-sound + Chrome-regression-free per the pre-release merge-unverified
  norm; local + revertible if you'd rather hold it.
- **ROUND-2 LANE-B COMPLETE:** all six items (A6/A7/A4/A3/BUG-GRAPH/BUG-EDITOR)
  implemented, gated, merged local. Commits fc1730e5 + 37d68bef + d861b61b.
  No round-3 carryover. Empirically-unverified: BUG-EDITOR (WKWebView/desktop)
  + A4's two parity-verified button clicks. Note: @@LaneD's wave-1 (`1b39832b`)
  landed on top of my 37d68bef cleanly; my BUG-EDITOR sits on top of theirs.

- **IDX pairing (with @@LaneC): frontend half DONE + committed `d124c48d`.**
  ACKed the wire shape (event-lane-c.md item 3) first. Files: api/types.ts
  (idle gains `embedding?: {done,total}`), AppStatusBar.svelte (passive
  `embedding done/total` chip - static non-pulsing dot, active pills unchanged),
  indexPillVisibility.test.ts (predicate + source pins + idle+embedding cases).
  PreflightOverlay = verify-only, NO change (gates on PreflightSnapshot.phase,
  independent of IndexStatus.embedding). Gate: svelte-check 0/0, vitest
  1586/1586, build clean. Smoked SOLO via a fetch-injected idle+embedding
  status: passive chip renders ("embedding 1/2", static dot) + clears when
  embedding settles. **Integration smoke (real backend push) pending @@LaneC's
  backend emit** - offered to drive it or pair once their emit lands.

- **IDX contract published (idx-wire-shape.md) -> ACKed field names; reconciled.**
  Field names match what I built. One nuance vs my item-3 build: the contract
  emits an explicit `embedding: null` when settled (not just absent), so I
  widened the type to `{done,total} | null` (`6138c623`, follow-up to
  `d124c48d`). AppStatusBar was already null-safe (`!= null` + truthy), so
  type-fidelity only, no behavior change. EmbedProgress is camelCase done/total
  (= done/total, unchanged); done<=total clamped backend-side, rendered direct.
  Clean ownership split confirmed: types.ts/AppStatusBar/PreflightOverlay mine,
  indexer.rs/preflight.rs @@LaneC's (their indexer.rs is dirty = backend emit in
  progress; I left it untouched).

- **IDX JOINT INTEGRATION SMOKE (both halves on main: LaneC b0525edb/3e54ed3e/
  326532d9 + my d124c48d/6138c623).** Built integrated main, served a fresh
  drive UNDER EMBED_FILE_CAP(2000) so background embed runs. RESULTS:
  - END-TO-END CONTRACT VALIDATED via the live API (3x): the real backend emits
    `{"state":"idle",...,"embedding":{"done":N,"total":M}}` (camelCase, done<=
    total, state stays "idle") - EXACTLY my consumer's shape. No divergence.
  - Preflight UNLOCKS on idle despite background embedding (live visual, Option
    A working) - my verify-only PreflightOverlay correct.
  - Chip CLEARS when embedding settles -> the idle JSON drops `embedding`
    (indexed_vectors hits total), status bar goes empty (live + API).
  - Chip RENDER of the exact idle+embedding shape: confirmed via the solo
    fetch-injection smoke, which runs through the REAL poll -> .json() -> store
    -> $derived -> chip path (injection returns a real Response parsed by the
    same poll). Real data shape == injected shape, so the live render == the
    validated injected render.
  - NOT captured: a screenshot of the chip present from a non-injected live
    poll. The debug-build embed settles within browser reload+preflight+poll
    latency (250-chunk flush ~75s at 792% CPU; my connect cycle kept landing
    just after settle). Declined to force it with a ~1500-note multi-minute
    embed that would peg all 8 cores on the SHARED machine. Timing artifact, not
    a product/code gap; render path is identical to the validated injection.
  - VERDICT: integration functionally validated end-to-end. @@LaneC + I are
    done pairing.

- **INTEGRATION COVERAGE WALK (proactive QA, @@Architect-offered).** Built
  integrated main (326532d9: my round-2 + @@LaneD wave-1 + IDX both halves),
  served a small nested drive (11 files, under the embed cap, no shared-core
  peg). Chrome-smoked the merged frontend as a whole. **ZERO regressions.**
  Findings:
  - A6 About front (chan version 0.20.0 Apache 2.0 + attributions) - intact.
  - A3 slot menu from the tab title (About/Workspace/Search checkboxes +
    Settings Cmd+, + Reload) - renders correctly on the merged build alongside
    @@LaneD's tabs.svelte.ts changes.
  - **autoRotate (CONCLUSIVELY verified - was previously unsmoked):** loaded a
    dashboard with `ar:false` via a crafted hash; it did NOT auto-rotate over
    12s, and STILL did not rotate after I enabled global cycling (pause icon) +
    waited 9s -> autoRotate=false correctly OVERRIDES the global cycling pref.
    The `ar` + `cs` hash round-trip also proves the DashboardTab serialize/
    restore region survived @@LaneD's tabs.svelte.ts merge.
  - **A4 - all THREE dashboard buttons now empirically CLICKED (parity gap
    closed):** New Terminal (openTerminalInPane, wave-1) + Show Directory
    (revealPathInBrowser -> opened FB tab expanded to notes/deep) + Graph from
    here (openFsGraphForDirectory -> new filesystem-mode graph tab). No-regression
    re-confirmed: the FB dir inspector still shows Upload.
  - Preflight unlocks on idle while embedding (Option A) - re-confirmed on
    integrated main.
  Not re-walked (code unchanged on main + wave-1-verified): A7 theme preview,
  BUG-GRAPH in-graph semantic->filesystem (fs-graph mode itself confirmed via
  the A4 Graph-from-here). Reported to @@Architect: no regression.

- **TEAM-GROUP dialog (wave-3, routed by @@Architect from @@LaneD).** Decision:
  PERSIST. My side (HELD, uncommitted, in shared worktree):
  - `teamDialog.svelte.ts`: `TeamDialogConfig.tabGroup` (required) +
    `defaultTabGroupFromPath(configPath)` (filename minus `.toml`, falls back
    `chan-team`) + seeded in `defaultTeamConfig` + validated (required).
  - `TeamDialog.svelte`: "Terminal tab group name" input next to Path to
    configuration, bound to `config.tabGroup`; `syncTabGroupToPath` keeps it
    following the filename via a `lastAutoTabGroup` tracker UNTIL hand-edited
    (then sticky).
  - `teamDialog.test.ts`: +tests for the helper, the seed, and the required
    validation (35 pass).
  My files svelte-check-CLEAN. Required `tabGroup` breaks 6 of @@LaneD's files
  (wireToDialog + 4 orchestrator test literals) until they thread it = clean
  handoff (all their domain). Browser-SMOKED (NEW mode): field shows, default
  `chan-team` from filename, path edit -> re-derives (`squad.toml`->`squad`),
  hand-edit -> sticky (path change does not override). Persist round-trip
  (save->load remembers group) pending @@LaneD's wire.
  - **Combined-commit plan (@@Architect): I land it as ONE atomic commit** once
    @@LaneD's threading (wireToDialog + their tests + `TeamConfigWire.tab_group`)
    is in the worktree and the COMBINED tree gates green - so main is never red.
    Crediting @@LaneD for the orchestrator+wire. HOLDING until they ping.
  - **LANDED: `5603403`** - single atomic combined commit, all 10 TEAM-GROUP
    files (my 3 dialog + @@LaneD's 7 orchestrator/wire/tests; @@Architect's "9"
    miscounted my teamDialog.test.ts). I re-ran the full gate on the combined
    tree first (svelte-check 0/0, full vitest 1591, build clean), chained
    staging + `git diff --staged --stat` audit (+96, exactly the 10, no
    package-lock), commit, `git show --stat` verified. @@LaneD credited for the
    orchestrator+wire in the body; they were holding teamOrchestrator.svelte.ts
    edits until this landed (now unblocked for TEAM-CONSOLIDATE). This is the
    tabGroup DATA plumbing + persistence; the functional `-N` conflict +
    thread-into-creation is @@LaneD's separate TEAM-CONSOLIDATE.
