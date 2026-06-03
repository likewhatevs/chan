# Journal - @@LaneA (lead) - phase-17 round-1

Append-only running log. (The round-16 launcher round's lead journal is
frozen at docs/journals/round-16/journals/journal-LaneA.md.)

## 2026-06-02

- Phase-17 round-1 went live: @@Alex ran `cs terminal team load
  docs/journals/phase-17/team`. Agents bootstrapping off the hand-authored
  bootstrap.md plan. @@LaneC reported ready first, self-identified correctly
  as editor & graph (B2/B6/B9) - confirms the plan is read as intended.
- Plan recap (full detail in bootstrap.md): 4 lanes. A (me) = launcher
  follow-ups S1/S2/S3 + TeamDialog B3/E1 + coord. B = terminal/cs (B1/B4/B8).
  C = editor/graph (B2/B6/B9). D = platform+docs (B5/B10/B11/D1). Shared
  files: tabs.svelte.ts (B/C), App.svelte (B/C), chan-server crate (B/D).
- Dispatch policy: Wave-1 isolated items go out as each lane reports ready;
  Wave-2 shared-file items I sequence; D1 docs verify-late.
- Dispatched @@LaneC Wave-1 = B2 (unordered list glyphs), task-LaneA-LaneC-1.
  Isolated (blocks.ts + Wysiwyg.svelte only); safe to start immediately.
  Holding for @@LaneB + @@LaneD ready pokes to dispatch their Wave-1.

## 2026-06-02 (continuation - new @@LaneA session, reconciled from disk)

- Resumed @@LaneA. On-disk state was ahead of my context: all four journals
  written, task-LaneA-LaneC-1.md (B2) already cut. Verified, did not redo.
- Coordination friction: a stale `new-team-1` tab-group (round-16 launcher
  round) is still live alongside `phase-17`. `cs terminal scrollback
  --tab-name=@@LaneX` is AMBIGUOUS (2 live sessions per name), so I cannot peek
  worker terminals; falling back to journals + completion-task files (the
  team's actual coordination channel). All pokes scoped `--tab-group=phase-17`
  (name+group intersect). Flagging the stale group to @@Alex (cs has no
  terminal-close; teardown is a UI action @@Alex must do).
- Dispatched Wave-1: @@LaneB = B8 (task-LaneA-LaneB-1.md, repro codex chord
  first); @@LaneD = B11 + B10 (task-LaneA-LaneD-1.md); re-poked @@LaneC to
  confirm B2. All three queued at position 1 (idle = fired immediately).
- Starting my own Wave-1: S1/S2/S3 (desktop launcher) + B3 (TeamDialog
  team-load path UX). S1/S2/S3 are WKWebView - @@Alex hand-smokes; B3 is SPA,
  I browser-smoke. Surfacing to @@Alex: B5 scope (global vs codex-only), the
  empirical test-server/client plan, and the stale new-team-1 group.

- Did my Wave-1 edits: S1 (index.html header reorder [icon][New]), S2/S3
  (main.js outbound + inbound code blocks + shared wireSnippetCopy helper,
  node --check OK), B3 (TeamDialog refreshDirSuggestions: prefix-filter before
  the 50-cap + append "/" so a bare prefix suggests matching dirs). Not yet
  gated (frontend web-check + desktop build pending).

- @@Alex answered surveys: B5 = GLOBAL off + opt-in toggle; search overlay =
  autocomplete NOW (round-1), live/prioritized leaf-index LATER (round-2).
- @@Alex new asks (routed): (1) rich-prompt cmd+enter eats input -> @@LaneB B1
  (task-LaneA-LaneB-2, queued behind B8; confirm-before-reap + loader/cancel).
  (2) 2x glyph-to-text gap -> @@LaneC (followup-LaneA-LaneC-1, B2 fold-in).
  (3) team MCP env toggle in the team setup dialog + cs team new/load -> this
  is B5's opt-in SURFACE: team config.toml mcp_env field, TeamDialog
  (my file) + cs team new/load (chan-shell, @@LaneB crate) expose it, spawn
  path honors it (default off). Wave-2 with B5.

- ** CRITICAL host-level blocker **: TWO teams share this ONE worktree:
  `phase-17` (mine) and the leftover `new-team-1` (round-16 launcher round,
  already committed fd27d29d). Both have a full @@LaneA-D roster; pokes by
  tab-name alone hit BOTH teams. The prior @@LaneA's B2 poke (unscoped) pulled
  new-team-1's @@LaneC into phase-17 work: it implemented B2 in
  blocks.ts/Wysiwyg/blocks.test.ts AND wrote task-LaneC-LaneA-1.md ("DONE,
  gate green") into phase-17's tasks dir, while phase-17's @@LaneC correctly
  HELD (its journal entries 57-97 document the collision). So the two
  @@LaneC's are double-writing phase-17's coordination files = corruption.
  Resolution (escalated to @@Alex): tear down new-team-1 so phase-17 is the
  sole team; KEEP the in-tree B2 work (gate-green + @@Alex reviewed the
  screenshot), have phase-17's @@LaneC ADOPT it (apply 2x gap, verify dead
  BULLET_MARK, re-gate, re-report). My pokes were group-scoped (phase-17), so
  @@LaneB/@@LaneD only got phase-17's copy; B8 + B11/B10 touch different files
  and keep running. Told phase-17's @@LaneC to keep holding the editor files
  until @@Alex confirms new-team-1 is down.

## 2026-06-02 (collision resolved + Wave-2 dispatched)

- @@Alex clarified new-team-1 is the round-16 launcher team (its dir is the
  smoke source new-team-1/desktop-redesign-design.md), leftover + not driven by
  @@Alex ("it's one of us"). Resolution: KEEP phase-17 (my team = @@Alex's
  command channel + most in-flight work); STAND DOWN new-team-1. Broadcast the
  stand-down to --tab-group=new-team-1 (4 sessions). My CHAN_TAB_GROUP=phase-17
  confirmed. Did NOT close tabs (no cs close; suggested @@Alex close them for a
  clean teardown). new-team-1's @@LaneC's B2 work survives in the tree
  (uncommitted) - adopted by phase-17.
- B2 + 2x gap: DONE by new-team-1's @@LaneC, quiescent (blocks.ts 22:52,
  Wysiwyg 22:56, test 22:36; stable post-standdown). Gate green, margin-right
  4.48px (doubled), depth cycle disc/circle/square marker-agnostic.
- B8: DONE by @@LaneB. Root cause = codex coalesces text+CR into a paste burst;
  fix = bracketed-paste wrap (ESC[200~ text ESC[201~ CR). Gate green (scoped),
  live-verified codex 0.136.0. submit.rs + cli.rs + submitMode.ts(+test)
  lockstep. FLAG: routes/team_config.rs submit_chord_literal is a 3rd doc-only
  chord mirror, now stale - low-pri, fold into B5.
- New @@Alex asks routed: rich-prompt submit bug -> B1; 2x glyph gap -> done;
  search overlay -> autocomplete round-1 (mine, SearchPanel) + leaf-index
  round-2; team MCP toggle -> B5 surface; dashboard cmd+shift+d -> B12/@@LaneB
  (web=Alt+Shift+D to dodge browser bookmark-all).
- Dispatched Wave-2: @@LaneB = B1 (task-LaneA-LaneB-2; B12=task-...-3 next; HOLD
  B4 for chan-server sequencing vs D's B5). @@LaneC = adopt B2 + B6 + B9
  (task-LaneA-LaneC-2). @@LaneD still on B11/B10 (its fs_ops/workspace.rs WIP is
  why whole-tree fmt is red - expected).

## 2026-06-02 (@@LaneC Wave-2 progress + B9 boundary call)

- @@LaneC: B2 ADOPTED+green (dead BULLET_MARK confirmed removed). B6 DONE - and
  it diverged for the RIGHT reason: recon premise was wrong (modal suggestions
  are kind-agnostic); real cause is lazy tree.entries (deep paths have no
  autocomplete entries). Fixed in its PathPromptModal.svelte via a progressive
  folderSet-gated loadTreeDir cascade - improves EVERY path dialog, smoked
  green. Approved. (Note: my B3 uses api.list-direct, not tree.entries, so no
  overlap; my round-1 search-autocomplete will also use api.list-direct.)
- B9 boundary: graph logic lives in web/src/components/GraphCanvas.svelte, NOT
  GraphPanel.svelte (the bootstrap/recon misnamed it). GraphCanvas is unowned +
  uncontended (git clean, 0 bootstrap mentions). AUTHORIZED @@LaneC to own
  GraphCanvas.svelte for B9 (+ store.svelte.ts already its) -
  followup-LaneA-LaneC-2. Boundary kept: cmd+shift+m OPEN handler in App.svelte
  stays @@LaneB's; route any shared touch through me. @@LaneC proceeding on B9.
- Process: @@Alex set two rules this session - (1) surveys via `cs terminal
  survey --tab-name @@LaneA` (NOT --tab-group=broadcast, NOT the TUI/
  AskUserQuestion - his typing collides with the poke queue + rich-prompt
  compose); (2) pokes stay 1-line pointers, substance in files. Saved as memory
  feedback-ask-via-cs-survey.
- My own queue (defer to the consolidation point when the web tree settles, so
  I implement+smoke+gate in one clean block, not amid B/C web churn): round-1
  search-autocomplete (SearchPanel.svelte, mine/uncontested), E1 spawn
  auto-assign (TeamDialog, Wave-3), gate B3 (frontend) + S1/S2/S3 (desktop
  build for @@Alex WKWebView hand-smoke).

## 2026-06-02 (B1 + B11/B10 done; 3 decisions; Wave-2 continuation dispatched)

- @@LaneB B1 DONE: data-loss root cause = window-global bubble visibility while
  submit routed to the focused pane's tab; fixed = per-tab visibility (byTab) +
  reap-only-on-delivery. Chrome-smoked (2 panes/2 terminals). DECISION (B1
  loader/cancel + server-ack): the WS prompt frame is fire-and-forget; a TRUE
  confirm needs a chan-server prompt-ack (shared w/ D). ARCHITECT CALL: ACCEPT
  B1 now (the data-loss = @@Alex's actual bug, fixed+verified); the loader/cancel
  + server-ack is an additive follow-up, sequenced with the chan-server prompt
  handler. Disclosed to @@Alex.
- @@LaneD B11+B10 DONE (gate green, sha b29ba5241fd8d224). B10 finding: the
  ~13s pre-URL stall is workspace.watch() setup, NOT indexing (URL prints ~0.1s;
  9000-md vault watch()=13s). Shipped a cold heads-up + throttled progress
  stream -> the silent window (= @@Alex's actual complaint) is FIXED. DECISION
  (B10 async-watch): eliminating the 13s = async watch() setup w/ an event-loss
  correctness window. ARCHITECT CALL: ACCEPT B10 now; defer async-watch (risky
  under release pressure, silence already fixed). Disclosed.
- B11 DECISION (searchable): @@Alex's spec said sniffed text should be
  searchable; D found BM25-indexing it = config/asm/Kconfig noise + ~15-site
  change, shipped editable-only. This DIVERGES from the written spec -> SURVEYED
  @@Alex (background, bg id btrm06xp9): defer searchable vs index now.
- Dispatched Wave-2 continuation: @@LaneB = B12 dashboard chord
  (task-LaneA-LaneB-3); B4 HELD until D lands B5 chan-server (I'm SERIALIZING
  the chan-server crate: D's B5 burst first, then B4's control_socket touch).
  @@LaneD = B5 (task-LaneA-LaneD-2; land the team mcp_env config struct FIRST +
  poke me, so @@LaneB's cs-surface + my TeamDialog toggle land against it).
- Deferred backlog (for round-close): B1 loader/cancel+prompt-ack, B10
  async-watch elimination, B11 searchable (pending survey), search-overlay
  prioritized/live leaf-index (round-2). Tracking in deferred-backlog.md.
- CORRECTION (record fix): my followup-LaneA-LaneC-2 said graph logic lives in
  GraphCanvas NOT GraphPanel - that was from @@LaneC's flaky/truncated grep,
  which it self-corrected via atomic Read. Truth: GraphPanel.svelte is NOT dead;
  Pane.svelte renders it for graph tabs, and GraphPanel imports+wraps GraphCanvas
  and owns scope/depth/graph-from-here. So B9 = GraphPanel.svelte + store.svelte
  .ts (both @@LaneC's) + GraphCanvas.svelte (authorized). No new clearance.
  Boundary held: Pane.svelte is @@LaneB's (B4 ~484-500) - B9 edits GraphPanel
  (the wrapper), not Pane. @@LaneC proceeding with B9.
- B11 searchable: @@Alex survey = DEFER (round-2). Editable-core B11 is final
  for r1. Backlog F3 updated; @@LaneD confirmed (no more B11 work).

## 2026-06-02 (search-autocomplete integration plan; staying lead-available)

- Read SearchPanel.svelte internals. Integration approach for the round-1
  search-autocomplete (so the consolidation block is fast): SearchRow is a
  discriminated union already carrying a `{kind:"file"; path; key}` variant with
  full open/inspect wiring. PLAN: add a path-detection gatherer - when the
  trimmed query looks path-like (starts with `./` or `/`, or contains `/`),
  call api.list(parent) DIRECTLY (NOT tree.entries - dodges @@LaneC's B6
  lazy-tree trap), prefix-filter by the typed value, and emit "file" rows for
  matching entries so they open via the existing wiring. Guard with the existing
  queryToken staleness. Round-1 = surface matching paths as openable rows even
  when content is unindexed (no "no matches" dead-end). Round-2 = the
  scan/reindex-on-path-miss + live leaf search (backlog F4). SearchPanel is
  uncontested (no lane owns it).
- DECISION (mine): implement search-autocomplete + TeamDialog mcp_env toggle +
  E1 + frontend gate + desktop build + ALL browser-smokes as ONE consolidation
  block when the web tree quiesces (B12 + B9 + B5-surfaces landed), rather than
  half-building across the imminent @@LaneD B5-struct handoff. Staying
  lead-available for that handoff (next coordination beat).

## 2026-06-02 (round-2 added; @@Alex AWAY + push authorized; autonomous mode)

- @@LaneD B5 STRUCT CHECKPOINT landed green: chan_workspace::TeamConfig.mcp_env
  (serde default false, toml key mcp_env, top-level) + ServerConfig.terminal
  .mcp_env (default false). Team spawn reads config.mcp_env; WS/HTTP create
  default to terminal.mcp_env (off); ?mcp_env=on overrides. Released: @@LaneB
  cs-surface (task-LaneA-LaneB-4, chan-shell only, after B12, BEFORE B4) + my
  TeamDialog toggle. B4 STILL held until D cuts B5-done (chan-server gate
  window). D finishing routes/team_config.rs cosmetic + own-gate.
- ROUND-2 added: @@Alex's alex-report-2/ moved to docs/journals/phase-17/round-2
  (draft.md + image*.png; relative refs intact). 3 items triaged (round-2/
  plan.md): R2-1 references/about-page -> @@LaneD (folds D1), R2-2 list
  paste-link indent bug -> @@LaneC (editor), R2-3 per-terminal survey -> @@LaneB
  (BubbleOverlay, follows B1 pattern). Queued behind round-1 Wave-2 per lane;
  no mid-task interrupt.
- ** @@Alex AWAY a few hours + AUTHORIZED commit + push ** ("keep moving,
  gradually commit the new code and push"). Supersedes the no-push rule for this
  window. Autonomous mode:
  - Make obvious calls; queue genuine product decisions for @@Alex's return (no
    surveys while away).
  - Commit at COHERENT GREEN boundaries (not half-done mid-Wave), atomic per
    item/lane with explicit pathspecs (git commit -F msg -- <paths>); the
    working tree's in-progress WIP stays uncommitted.
  - Gate the COMMITTED state in an ISOLATED worktree (immune to peers' WIP) ->
    full make pre-push -> ONLY push if green. Foreground push + git ls-remote
    verify (gated-push SIGPIPE risk). On main (team round-close pattern).
  - First commit boundary: round-1 Wave-2 complete + green. Then round-2.
  - My own work between worker reports: TeamDialog mcp_env toggle (released),
    search-autocomplete, E1, then gate + desktop build + browser-smokes.

## 2026-06-03 (TeamDialog MCP toggle landed; dispatched B4 + D1/R2-1)

- Released B4 to @@LaneB (task-LaneA-LaneB-5; D's chan-server was spawn-only,
  pane-exec clear) + dispatched D1 + R2-1 to @@LaneD (task-LaneA-LaneD-3, draft
  early / verify late). All lanes busy: B cs-surface->B4->R2-3; C B9; D D1+R2-1.
- TeamDialog MCP-env toggle DONE (web-only; @@LaneD's TeamConfig.mcp_env wire
  already serializes via serde, so no Rust change). Mirrored autoPrefix across 6
  spots: teamDialog.svelte.ts (interface + default mcpEnv:false),
  teamOrchestrator.svelte.ts (dialogToWire + wireToDialog), client.ts
  TeamConfigWire (mcp_env), TeamDialog.svelte (checkbox), + 2 wire test
  fixtures. VALIDATED: scoped vitest teamOrchestrator + teamLoadFlow = 41 pass
  (mcpEnv round-trips). svelte-check + browser-smoke deferred to consolidation.
- COMMIT STRATEGY clarified: round-1 work is fully interdependent +
  uncommitted (my web toggle needs D's Rust TeamConfig.mcp_env; the .ts needs
  the whole web tree to svelte-check). So there is NO clean sub-part commit
  boundary - the boundary is round-1 COMPLETE (all lanes done). Then ONE
  consolidation: implement my search-autocomplete + E1, full gate the committed
  state in an isolated worktree, browser-smoke, commit round-1 per-lane/feature,
  push. Then round-2. Event-driven on worker reports until then.

## 2026-06-03 (lanes' round-1 nearly closed; E1 landed; dispatched R2-2 + e2e smoke)

- Completions in: @@LaneC ALL round-1 DONE (B2/B6/B9 smoked, vitest 1650, shas
  captured - GraphPanel+store+3 tests for B9, PathPromptModal+test for B6,
  blocks+Wysiwyg+test for B2). @@LaneB cs-surface DONE (`cs team new --mcp-env
  on|off`, chan-shell only, e2e verified). @@LaneD B5 done, offered the e2e
  toggle smoke.
- Dispatched: @@LaneC R2-2 (list paste-link indent bug, task-LaneA-LaneC-3);
  @@LaneD e2e MCP-toggle smoke (both surfaces landed) + D1+R2-1. @@LaneB on B4.
- E1 (spawn auto-assign) LANDED: pure autoAssignSlots in teamDialog.svelte.ts
  + autoAssign() + a robot-icon button right-aligned in the shape-picker row +
  CSS. VALIDATED: new teamDialogAutoAssign.test.ts (5) + team tests = 46 pass.
  Button visual + grid update need the consolidation browser-smoke.
- My round-1 own-work: S1/S2/S3 + B3 + MCP toggle + E1 DONE (ungated);
  search-autocomplete is the only one left - deferring to the consolidation
  block (interactive UX, not cleanly vitest-able; implement+smoke together).
  ROUND-1 BOUNDARY gated on: @@LaneB B4 + my search-autocomplete + the full
  isolated-worktree gate. When B4 lands -> search-autocomplete + gate +
  browser-smoke + commit round-1 per-lane + push.

## 2026-06-03 (B4 boundary ruling; front-loading search-autocomplete)

- @@LaneB B4 recon: the cs pane split dir + focus fix BOTH land in
  store.svelte.ts::applyPaneExec (@@LaneC's file, ~781-813), NOT control_socket.rs
  (which forwards the PaneOp opaquely). RULED Option (A): @@LaneB owns the
  applyPaneExec region for B4 + lands atomically in lockstep with the chan-shell
  SplitDir(Left->Right) change (the wire dir string + the applyPaneExec check
  must match). followup-LaneA-LaneB-1. @@LaneC clear (on R2-2 editor, not store);
  sent a defensive heads-up. COMMIT NOTE: store.svelte.ts will carry C's B9
  region + B's B4 applyPaneExec region -> one merged commit, B lists its B4 lines.
- Front-loading my search-autocomplete on SearchPanel.svelte (uncontested by
  B4/R2-x) so the consolidation is a single gate+smoke pass once B4 lands.

## 2026-06-03 (search-autocomplete LANDED - all my round-1 code in)

- Search-autocomplete implemented in SearchPanel.svelte (round-1 part; the
  live/prioritized leaf-index stays round-2 per @@Alex). New `path` SearchRow
  kind: when the query isPathLike (has "/" or leading "./"), refreshPathHits
  lists the typed parent via api.list DIRECTLY (not the lazy tree -> deep /
  unindexed paths resolve, the @@Alex complaint), prefix-filters, surfaces
  files (open) + dirs (drill: re-seed query to dir + "/"). Runs ALONGSIDE
  content search (same queryToken staleness), path matches ranked FIRST. 10
  touch points: union, pathHits state, isPathLike+refreshPathHits, scheduleSearch
  (clear-on-empty + call), rows combiner, rowCounts, selection, activate
  (drill/open), template branch, status summary. Verified all 4 SearchRow
  switches exhaustive on "path"; api.list(dir?)->TreeEntry[]{path,is_dir} matches.
- ALL my round-1 own-work now LANDED (ungated): S1/S2/S3, B3, MCP toggle, E1,
  search-autocomplete. Remaining = the round-1 consolidation (gate + browser-
  smoke). Boundary gated ONLY on @@LaneB's B4 now. When B4 lands -> isolated-
  worktree full gate + browser-smoke (B3/toggle/E1/search + regression) ->
  commit round-1 per-lane -> push. Event-driven on B4 + round-2 reports.

## 2026-06-03 (two boundary calls; both re-confirmed/authorized)

- @@LaneB re-asked the B4 A/B call (its poke crossed my ruling) - re-confirmed
  Option A (own applyPaneExec, land atomically w/ chan-shell SplitDir). Go.
- @@LaneC R2-2: root-caused bug 2 (commands/list.ts shiftListLines - a top-level
  Shift-Tab outdent strips the whole prefix, ejecting the item; fix = no-op) +
  bug 1 likely paste_html.ts (turndown indent, reproduce-first). Both editor-
  extension files, unowned (0 bootstrap mentions) + clean. AUTHORIZED
  web/src/editor/commands/list.ts + editor/paste_html.ts (+tests) for R2-2
  (followup-LaneA-LaneC-3) - clearly editor lane, no cross-lane contention.
- Holding event-driven: B4 (last round-1 item -> my consolidation) + round-2
  (C R2-2, D D1/R2-1 + e2e toggle smoke).

## 2026-06-03 (** ROUND-1 COMMITTED + PUSHED **)

- B4 landed (last round-1 item). Consolidation:
  - Build + gate: a Chrome browser-smoke of my SPA changes was BLOCKED (Chrome
    navigate permission-denied, @@Alex away to approve). Fell back to the
    pre-release-merge-unverified norm: push gated-green, record the interactive
    smoke as pending.
  - Found + fixed a gate break: @@LaneD's B10 added ServeConfig.verbose but
    missed the chan-desktop initializer (desktop/src-tauri/embedded.rs:104) -
    chan-desktop is outside the default workspace, so D's scoped gate passed but
    full `make pre-push` failed. Added verbose:false (rides D's B10 commit).
  - FULL `make pre-push` GREEN (fmt/clippy/test/web-check/build/gateway/
    chan-desktop). @@LaneC reverted its R2-2 list.ts WIP so the tree was clean.
  - 5 per-lane/feature commits (verified each staged + post-commit stat):
    cfc160cc workspace(B11/B10/B5, D), 6dc279a4 terminal(B8/B1/B12/B4+cs, B),
    23d8db15 editor(B2/B6/B9, C; store.svelte.ts carries B4 too),
    ed02e60c team+search(S1/S2/S3/B3/toggle/E1/search, A),
    03bb91f8 docs(R2-1/D1).
  - PUSHED foreground (gated): origin/main 45a6e341..03bb91f8; verified via
    git ls-remote (remote=local=03bb91f8, 0 ahead). Also carried up the 2
    pre-existing unpushed commits (fd27d29d launcher, d8425aad phase-16 close).
  - Coordination docs (docs/journals/phase-17, round-16) kept UNTRACKED (live
    round-2 bus; commit at round-2 close). `.codex/config.toml` deletion left
    (pre-existing, not mine).
- ** PENDING @@Alex (browser-smoke) **: my round-1 SPA changes are gated-green
  (svelte-check 0 err + vitest + build; toggle has D's e2e spawn test) but NOT
  interactively browser-smoked (Chrome was unavailable): B3 team-load
  autocomplete, MCP-toggle UI, E1 auto-assign button, search path-autocomplete.
  Recorded in deferred-backlog. The lanes' own behavioral changes ARE smoked.
- Round-2 dispatched: @@LaneC resumed R2-2 (re-apply bug-2 + bug-1);
  @@LaneB R2-3 per-terminal survey (task-LaneA-LaneB-6). R2-1 already pushed.
  When R2-2 + R2-3 land + gate -> commit + push round-2, then round close.

## 2026-06-03 (R2-2 done; waiting on R2-3 to batch the round-2 commit)

- @@LaneC R2-2 DONE + gate-green (4 files: list.ts/list.test.ts/paste_html.ts/
  paste_html.test.ts; vitest 1661). Bug2 = top-level Shift-Tab no-op (real
  EditorView test); bug1 = dedentListPaste strips turndown's stray "-   " marker
  on paste into a list line (5 unit tests + turndown root-cause probe). Same
  Chrome-denied caveat -> 30s @@Alex confirm tracked in backlog. Deterministic
  CM6 transforms (not Svelte reactivity) = lower runtime risk.
- Round-2 commit will batch R2-2 (4 files) + R2-3 (@@LaneB, in progress) - one
  gate+commit+push when R2-3 lands. R2-1 already shipped in round-1's push.
  Then round CLOSE: commit the coordination docs as docs(phase-17) + write the
  retrospective (done/pending + highlights/lowlights + feedback). Event-driven
  on R2-3.

## 2026-06-03 (R2-3 = contract change; ratified + split across 3 lanes)

- @@LaneB recon'd R2-3: per-terminal surveys need the open_survey FRAME to carry
  the target tab (today it carries none -> only a window-wide modal). That is a
  CONTRACT change (the @@Architect-held round-3-survey-contract.md C<->D seam),
  bigger than B's BubbleOverlay file.
- RATIFIED (contract AMENDMENT 2026-06-03): open_survey gains `tab_name:
  Option<String>` -> SPA `tabName` (serde rename; pin the wire string). Some =
  attach to that terminal; None (tab-group broadcast) = window-wide fallback.
  Purely additive - SurveySpec/reply/survey_id/bus untouched.
- SPLIT (B's recommendation): @@LaneD transport ~2 lines (task-LaneA-LaneD-4,
  control_socket.rs OpenSurvey push); @@LaneB the full SPA atomically
  (followup-LaneA-LaneB-2, AUTHORIZED survey.svelte.ts + store.svelte.ts
  open_survey handler ~1013 [C's file, distinct region; C heads-up'd] +
  api/client.ts frame + BubbleOverlay/TerminalTab per-terminal render, B1
  pattern). Additive field -> B can start now (forward-compat); D lands the
  frame in parallel.
- Round-2 commit will now batch R2-2 (4 C files) + R2-3 (D transport + B SPA +
  the contract doc). Event-driven on D-transport + B-SPA.

## 2026-06-03 (** ROUND-2 COMMITTED + ROUND CLOSE **)

- @@LaneB R2-3 SPA DONE + Chrome-verified e2e (B had working Chrome: 2 terminals
  each showed their own survey, independent replies round-tripped). Last item.
- Round-2 gate: full make pre-push GREEN (R2GATE_EXIT=0) - confirms B's "8
  errors" report was STALE (svelte-check 0 errors; my round-1 mcpEnv fix +
  SearchPanel are committed in 03bb91f8).
- Round-2 commits (verified staged + post-commit stat):
  f6eb16e1 fix(editor) R2-2 (4 files), 1afb1d61 feat(terminal) R2-3 (10 files
  incl the round-3-survey-contract.md amendment).
- Round CLOSE: wrote docs/journals/phase-17/retrospective.md (done/pending +
  highlights/lowlights + per-member + @@Alex + self feedback). Committing the
  coordination tree (docs/journals/phase-17 + round-16 archive) as docs(phase-17),
  then ONE push (R2-2 + R2-3 + docs).
- LEFT uncommitted: .codex/config.toml deletion (pre-existing at session start,
  NOT mine - surfacing to @@Alex, not committing someone else's deletion).

## 2026-06-03 (my svelte-check break FIXED; D1/R2-1 drafted)

- MY BUG: adding required TeamDialogConfig.mcpEnv broke whole-tree svelte-check
  (5 errs) - I fixed the WIRE fixtures (auto_prefix_at) but missed the DIALOG
  literals (autoPrefix) in 4 test files. My scoped vitest passed because vitest
  STRIPS TYPES; @@LaneD's whole-tree svelte-check caught it. FIXED: added
  mcpEnv:false to all 5 literals (teamOrchestrator.test x3, teamLeadRestart.test,
  teamBootstrapOrchestrator.test). VALIDATED: `npm run check` = 0 ERRORS / 4322
  files (1 pre-existing RichPrompt a11y warning). LESSON (retro): a required-field
  add to a shared TS type needs a grep of ALL literals (wire snake + dialog
  camel) + svelte-check, not just vitest.
- @@LaneD D1+R2-1 DRAFTED, gate-green (sha 756fa643). README/home open with the
  usage example; NEW docs/manual/desktop.md + gateway.md; /dl links already
  present. R2-1: About page is IN-APP (EmptyPaneCarousel about-licenses), NOT
  web-marketing - D added an about-credits block (Svelte/xterm/CodeMirror/
  Mermaid/Cytoscape+d3/KaTeX/Lucide/Tauri; axum/Tantivy+Candle/notify/rust-embed/
  portable-pty/yamux) using CANONICAL mermaid.js.org (not @@Alex's
  mermaid-cjv mirror - pinned a no-leak test). EmptyPaneCarousel uncontended.
- Tracking for @@Alex (return): WKWebView New->Remote hand-smoke; the
  mermaid-mirror-vs-canonical choice; D1 publish waits on @@Alex's live-command
  verification (curl install / git clone [repo private pre-release] / ssh -L /
  tunnel E2E); About-slide visual smoke (fold into joint frontend smoke).
- Whole-tree svelte-check is GREEN now (B4 not yet landed). Round-1 boundary
  still gated on B4. Re-poked D to run the e2e MCP-toggle smoke.