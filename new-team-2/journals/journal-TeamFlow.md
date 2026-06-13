# journal-TeamFlow (append-only)

## 2026-06-12 — task-Conductor-TeamFlow-3 (item 3 + item 5)

- Read task + both designs. Verified line numbers against HEAD e0ec0d3c
  (one lockfile-sync commit past the design's 3ebee587; all anchors held).
- Item 3: deleted the lead-enable + worker-target loop in
  teamOrchestrator.svelte.ts (kept the clear-all sweep), dropped the
  orphaned setTerminalBroadcastTarget import and the dead workerTabs
  accumulator. Re-pinned teamBootstrapOrchestrator.test.ts (membership
  empty, broadcastEnabled false for all; pre-existing-group-clear test
  unchanged). Scoped vitest 6/6 green.
- Item 5 Part B: rewrote "Reaching the host" in generate_bootstrap_md()
  (survey-first "whenever possible", 1..N/F/X host keys, --tab-name
  window-ownership fallback). Extended
  bootstrap_contains_team_host_lead_and_poke_chord (whenever-possible,
  key docs, fallback guidance pins; ASCII assert kept).
- chan-server was three-lane hot: a peer's burst in terminal_sessions.rs
  broke the shared-tree lib-test build right between my clippy (green)
  and test runs. Did NOT block: gated Part B in an isolated worktree
  (/tmp/teamflow-gate @ HEAD + only my file, own target dir):
  team_config 20/20, clippy --all-targets -D warnings, fmt --check green.
- Committed Part B first per binding sequencing: 86a0dce9 (1 file).
  Milestone-poked @@Conductor (releases @@CtxPass wave 4b).
- Item 5 Part A: x/X alongside Escape in BubbleOverlay keydown,
  "[X] Dismiss" label, comment updates; ?raw source pin added to
  survey.svelte.test.ts. make web-check (svelte-check + vitest + build)
  green AFTER the final web edit.
- Commits: 0f146fcf (item 3), c9fbb909 (item 5 Part A). All three
  pathspec-atomic (`git commit -F msg -- <paths>`), staged-stat before +
  show-stat after, each exactly its own files.
- Standalone verification (binary built from c9fbb909 in the worktree,
  fresh web/dist copied in; served /tmp/teamflow-ws on :8799 from a
  renamed copy /tmp/teamflow-srv): bootstrapped a 3-member bash team —
  broadcast picker all-unchecked after bootstrap, lead typing did NOT
  fan out; Select All re-enabled fan-out (worker echoed lead's keys),
  Deselect All restored. Surveys: '1' → CLI printed "Alpha"; 'F' (with
  --followup-dir) → created followups/followup-Worker1-Lead-1.md and CLI
  printed its path; 'X' → overlay dismissed, CLI printed "survey
  dismissed; no answer". Generated bootstrap.md carried the new
  "Reaching the host" text with correct handle interpolation.
- Teardown: closed my Chrome tab, pkill scoped to teamflow-srv (peer's
  :5173 verified untouched), unregistered + removed /tmp/teamflow-ws.
  Kept /tmp/teamflow-gate + /tmp/teamflow-target for cheap re-gates
  during review duty.
- Holding per task: awaiting review routing from @@Conductor.

## 2026-06-12 — task-Conductor-TeamFlow-10 (review ffbcc3ff + item-1 prep)

- Items 3+5 accepted by @@Conductor; flex task: adversarial review of
  @@Editor's item-4 commit + pre-read item-1 design.
- Key process catch: the shared tree carries @@Editor's in-flight
  item-1 WIP (M Pane.svelte/FileEditorTab/Wysiwyg/Source + untracked
  keep-alive test). Reviewed against COMMIT state (git show) and ran
  the pinned suites in my isolated worktree at ffbcc3ff (node_modules
  symlinked from main tree) — 24/24 green, @@Editor's WIP untouched.
- Verdict: CLEAN PASS on all five targets (mousedown byte-identical
  parent->commit; guard exact; close/drag paths parity-checked; the
  mouseup-without-mousedown edge is focused-gate-neutralized; pin
  regex anchors uniquely). Three non-blocking observations recorded.
- Report: tasks/task-TeamFlow-Conductor-11.md; poked.
- Item-1 design pre-read complete; review focus list staged in the
  report. Holding for the item-1 sha.

## 2026-06-12 — task-Conductor-TeamFlow-14 (standing flex: item-1 + item-2-web reviews)

- ffbcc3ff clean pass accepted. Two standing review assignments:
  item-1 restructure (primed since task-10) and item-2 WEB-HALF
  (pre-assigned; @@CtxPass has the server half).
- Pre-read item-2 design §§ Web changes / UX decisions / Tests done.
  Staged review targets for @@PromptQueue's web-half sha:
  - pendingPrompt transitions id-guarded (stale/foreign ids no-op),
    failPendingPrompt deliberately unguarded (WS close) — state tests.
  - sendPromptToTerminal's optional trailing id: team-orchestrator
    call sites (deliverLeadIdentity) must stay untagged — the design's
    one named in-repo contract; adjacent to my item-3 ground.
  - RichPrompt machine: submit does NOT clear doc; delivered clears
    doc + flushWrite (draft clears there, not at submit); readOnly
    compartment reconfigure; 300ms grace / 5s ack-timeout; rejected
    keeps text; second Cmd+Enter no-op; failed = honest label, never
    text loss.
  - Depth semantics: messages-not-writes (gemini pair = 1), 0 →
    undefined on the tab field, closed/exit → depth 0 + fail, depth
    re-sync via session frame on (re)attach.
  - Pane badge is restructure-gated and may arrive as a second piece —
    review scope per routing.
  - vitest coverage vs the design's three named test surfaces.
- Holding; surveys-to-host stay routed via @@Conductor.

## 2026-06-12 — task-Conductor-TeamFlow-16 (rerouted launcher review)

- Queue-tail reroute: @@Desktop's item 6 (3d4f564b, launcher JS) + B3
  (54b65a60, capability pins), originally @@Editor's pairing.
- CLEAN PASS both. Anchors: in-flight guard verified safe because the
  only refresh in the handler path runs AFTER the awaited turn-on and
  render() discards the stale node wholesale; dialog's three close
  paths funnel into one close() that removes its own keydown; gating
  split byte-checked (caret expression unchanged, rename faithful);
  direction-keyed failure routing confirmed (toggle.checked stable
  mid-flight since the pill disables itself); verbatim error via
  textContent; B3 pins parse the SHIPPED default.json + panic on
  missing set id, so no vacuous pass and no string-match weakness.
- 3 observations logged (external-re-render guard evaporation = pill
  parity; one-Escape-closes-stacked-dialogs cosmetic; B3 verified by
  helper-read + file-grep, not a desktop-workspace build).
- Report: tasks/task-TeamFlow-Conductor-17.md; poked. Standing
  assignments (item-1, item-2 web-half) still primed and outrank.

## 2026-06-12 — task-Conductor-TeamFlow-19 (item-2 web-half review, 86d50a25)

- Standing assignment activated; launcher review had already shipped
  (report 17), so nothing parked.
- CLEAN PASS on all nine targets. Key verifications: only ONE composer
  clear exists (consumeTerminalPhase "delivered"); submit flushes the
  draft pre-send and begins pending only on a true sink return;
  caller audit found exactly two non-test sendPromptToTerminal sites
  (RichPrompt tagged, teamOrchestrator untagged — lead-identity
  byte-identical); timer lifecycle is leak-free incl. onDestroy and
  the hidden-while-sent re-arm; compartment lock is seeded at
  EditorState.create (load-bearing since `view` is a non-reactive
  let); no $derived-reachable $state mutation; depth 0→undefined and
  closed/exit ordering pinned by tests. Ran the 3 test files (29/29)
  + svelte-check (0 errors) at the commit in the isolated worktree.
- 3 observations (fail-overwrites-unconsumed-delivered edge inside
  the accepted duplicate class; immediate chip on reshow is
  deliberate; queued has no delivery timeout by design) + 6 smoke
  flags for @@PromptQueue (runtime reactivity class, hide/reshow
  catch-up, reload mid-pending, cap rejection, multi-window,
  deliver-while-hidden+disconnect).
- Report: tasks/task-TeamFlow-Conductor-20.md; poked. Holding —
  item-1 restructure review still the top standing assignment.

## 2026-06-12 — task-19 review accepted

- Web-half review accepted; my 6 smoke flags routed to @@PromptQueue
  as task-Conductor-PromptQueue-21. Holding; item-1 restructure
  review remains my next standing assignment.

## 2026-06-13 — task-Conductor-TeamFlow-22 (item-1 keep-alive review, dadd5e64)

- The round's highest-stakes review, CLEAN PASS on all 11 targets.
- Method anchor: the 484-line FileEditorTab diff collapsed under
  `diff -w` (parent blob vs commit blob) to exactly the design's
  authorized changes — re-indentation from the {#key} removal was the
  bulk; the only unlisted edit (flex:1 drop) is the documented
  consequence of the absolute host. Zero riders.
- Subtlest verification: the WebKitGTK flip workaround hides
  .face.front via visibility, which a visibility:visible child would
  paint through — the !pane.showingBack term in the ACTIVE gate is
  the precise guard, identical to the terminal precedent.
- Effects inventory (6 $effects + svelte:window + onDestroy) all
  gated or path-keyed per the design's verified-safe list; terminal
  each-block byte-compared identical; fileDropGuard target-based so
  pointer-events:none keeps hidden editors unreachable; re-pin
  strengthened not loosened; 114/114 over ten suites + svelte-check
  0/0 at commit in the isolated worktree.
- Seconded @@Editor's undo-past-load-boundary watch item as a
  near-round follow-up (more reachable now that history survives
  switches).
- Report: tasks/task-TeamFlow-Conductor-23.md; poked. All three
  standing/rerouted reviews delivered (items 4, 6+B3, 2-web, 1).
  Holding for routing.

## 2026-06-13 — task-Conductor-TeamFlow-25 (report data prep; narrow-fix review pending)

- Item-1 review accepted (called the round's review high-water mark);
  undo narrow fix authorized to @@Editor as task-24, my O1 second
  absorbed into it.
- Assignment 2 delivered: designs/round-1-report-data.md — 19-commit
  table (landing order, lane/item attribution sourced from completion
  files), review matrix (7 delivered verdicts + wave-3 and wave-4a/4b
  reviews marked IN FLIGHT — the Explore sweep initially over-claimed
  wave-3 from its routing task; corrected against the bus), evidence
  index (incl. the kept-for-transparency failed walker log), deduped
  follow-ups (authorized/survey/v2/nits/unstarted/recorded-only), and
  the WKWebView checklist draft grouped by item with [instrumentable]
  vs [hand-smoke] marks. f198df7b has no review row yet — flagged "?".
- Method: fanned the bus sweep to an Explore agent, verified its
  ambiguous claims directly before writing.
- Assignment 1 (narrow undo-fix review, base.ts + vitest pin) primed:
  targets = annotation hits ONLY the initial empty->content apply,
  reload path byte-unchanged, negative test bites, no second
  clear-path into the item-2 doc-clear contract. Waiting on sha.

## 2026-06-13 — task-25 assignment 1 (narrow undo-fix review, bb877a87)

- CLEAN PASS on all three targets. New review tool for the kit:
  MUTATION TESTING in the isolated worktree — widened the annotation
  (initialFill=true) and the two reload-guard tests failed; removed it
  (initialFill=false) and 4/5 boundary pins failed; reverted clean.
  Strongest possible "the negative test bites" evidence, ~2 min cost.
- Scope verified: per-instance initialFillPending consumed by
  non-empty applies AND non-empty dedupes (creation-seeded mounts),
  left armed by empty->empty dedupes; reload/sibling-mirror applies
  behaviorally unchanged. createValueSync consumers = Wysiwyg+Source
  only; RichPrompt untouched → no second clear-path into the item-2
  doc-clear contract.
- O1 corner flagged for the round-close survey context: an
  empty-at-open file whose first-ever content arrives via file-watch
  reload gets the annotation (it IS the first content fill; the
  alternative undo target is the empty doc). Not a code change ask.
- Report: tasks/task-TeamFlow-Conductor-27.md; poked. Data file
  updated per assignment 2 (commit row + review row). Holding.

## 2026-06-13 — report data acceptance + B5 checklist append

- Data prep accepted; the f198df7b no-review-row catch routed that
  review to @@Editor. Appended to round-1-report-data.md: the B5
  30-second human check (Window-menu header + cap exclusion, from
  b5-buried-window-cap-decision.md §Verification status) and updated
  the f198df7b matrix row to routed/pending. bb877a87 rows were
  already in from the review pass. Standing: append review rows as
  verdicts land. Holding.

## 2026-06-13 — narrow-fix acceptance + data appends (round nearly swept)

- bb877a87 review accepted; mutation-verification named the round
  standard. O1 RULED accepted-as-is: the corner is a first-content
  fill by the annotation's own semantics; folded into the reload-undo
  survey item as boundary context (recorded in the data file's
  survey-items section).
- Data appends from the bus sweep: wave-3 batch ALL CLEAN
  (task-PromptQueue-Conductor-28), waves 4a+4b BOTH CLEAN — B1 review
  queue closed (task-PromptQueue-Conductor-29), f198df7b CLEAN PASS
  (task-Editor-Conductor-29), badge commit 7c976a68 added to the
  commit table. New no-review-row catch: 7c976a68 (badge) has no
  reviewer yet — flagged "?" in the matrix, same shape as the
  f198df7b catch.
- Holding.

## 2026-06-13 — lane DONE; holding for round close

- Data appends accepted; badge no-review-row catch credited (2nd gap
  caught by the matrix audit — institutionalized for the retro);
  badge review routed to @@Editor. No open work on this lane.
- Round-1 lane summary: items 3 + 5A + 5B authored (3 commits, all
  accepted + peer-reviewed clean); 5 reviews delivered (items 4, 6+B3,
  2-web, 1, undo narrow fix — all CLEAN PASS, 0 riders found, 2
  review-matrix gaps caught); round report data file built and
  maintained. Tooling that proved out: isolated-worktree gating +
  commit-state review under peer WIP, diff -w rider walks, mutation
  bite-tests.
- Holding for round close.

## 2026-06-13 — final append: badge+rider review row

- 7c976a68 + b82a0a27 (N1 docs rider) CLEAN PASS by @@Editor
  (task-Editor-Conductor-31) appended to the matrix; b82a0a27 added
  to the commit table (22 commits); N1 follow-up marked DONE. Matrix
  audit credited as institutionalized for the retro. Every round-1
  commit now has a clean review row — tracker fully resolved. Holding.

## 2026-06-13 — round-2 add-on pre-read (graph keep-alive, cross-review pending on sha)

- @@Editor extends their own dadd5e64 keep-alive to graph tabs (3rd
  tab kind). I cross-review on the sha. Design: round-2-graph-keepalive.md.
  Baseline 00a585b3 == HEAD == round-1 close; verified every design
  anchor against the clean baseline blob (Pane.svelte already carries
  @@Editor WIP, so I'll review the COMMIT, not the tree — same as
  round 1).
- Anchors confirmed at baseline: GraphPanel props {tab,onClose,onFlip}
  (~84-92, +active); `const visible: boolean = true;` (line 100 →
  $derived(active)); `.graph-tab` block = display:flex/flex:1/
  flex-direction:column/min-h-0/min-w-0/bg (2815, drop flex:1 + add
  visibility pair); GraphCanvas open prop (59/81), loop() (1147),
  resize() (820), start() (1314), stop() (1438), open effect (1497).
- Staged review targets (lead's + mine):
  1. LATCH: the open effect (1497) `else { stop() }` is the killer —
     stop() discards sim+node arrays, start() resets transform (1323).
     open={canvasEverShown} must latch true-once-shown so hide never
     calls stop(); reverting to open={active} kills pan/zoom with NO
     runtime test catching it — confirm the ?raw pin guards it.
  2. PAUSED short-circuit must sit at the TOP of loop() (before the
     trailing requestAnimationFrame(loop) at 1173) or the loop re-arms
     itself; resume effect must re-arm + resize() WITHOUT start()/
     transform reset (pan/zoom preserved).
  3. .graph-tab CSS: flex:1 dropped, position:absolute+inset:0+
     visibility:hidden+pointer-events:none, .active restores; NEVER
     display:none (0x0 → resize refits → loses pan/zoom).
  4. onClose/onFlip capture `t`, not `active` (old branch closed
     active.id) — the each-item closure bug.
  5. Plus: latch/dirty load gating (!hasLoadedOnce||keyChanged||
     graphDirty), lazy-on-first-activation not mount (N-load storm),
     watcher hidden→graphDirty (no bg reload) vs visible→debounce,
     no `focused` prop, terminal/file each-blocks byte-untouched,
     menuTrims Reload order pin (Depth→Reload→Copy-link), new
     paneGraphTabKeepAlive.test.ts non-tautology + mutation-bite where
     cheap.
- Held: review fires on @@Editor's sha. No deliverable yet.

## 2026-06-13 — round-2 graph keep-alive review (3fdd4bfe) — CLEAN PASS, WKWebView build cleared

- 7/7 targets clean, no riders. My verdict gated the @@Desktop build;
  CLEARED at this HEAD. 132/132 across 10 graph/keep-alive/menu suites
  + svelte-check 0/0 at the commit (isolated worktree).
- Latch traced fully: canvasEverShown is monotonic (no false path), so
  the open effect's `else stop()` never fires on hide → sim+transform
  survive; paused guard at loop() TOP genuinely stops the rAF; resume
  re-arms with resize() not start(). Load gating one-shot verified
  (lazy-not-mount, graphDirty cleared after the single reactivation
  reload, clean switch fires nothing). Plain latches, no $derived
  reads them; canvasEverShown the only $state, written only in its
  $effect. Watcher nonce advances for hidden tabs (no reprocess);
  changeAffectsScope first.
- graphInspectorActionsHotfix (unplanned diff) ruled legitimate: it
  pinned the old load-effect string the restructure rewrote; re-pinned
  same contract, correctly loosened. Not scope creep.
- MUTATION bite-tests (round standard): open={active} fails the latch
  pin; dropping the loop pause guard fails the loop pin — both
  runtime-untestable risks the design flagged are caught statically.
- 2 additive WKWebView walk items handed up: out-of-scope hidden edit
  on a DIR/TAG-scoped graph (#5, @@Editor couldn't exercise in Chrome —
  workspace-scoped test graph), and resize-while-hidden → reactivate.
- Report: tasks/task-TeamFlow-Conductor-42.md; poked. Holding.
