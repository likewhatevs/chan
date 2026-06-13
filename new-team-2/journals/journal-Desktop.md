# journal-Desktop — new-team-2 round 1

Append-only.

## 2026-06-12 — item 6 + B3 complete

- Item 6 (launcher Open always-enabled + auto-turn-on + failure
  dialog) landed as `3d4f564b`; B3 (default.json + main-window
  negative pins for read_dropped_paths) as `54b65a60`. Both
  pathspec-atomic, staged-stat/show-stat verified single-file.
- Gate: isolated worktree at `e0ec0d3c` + my two files only (main
  tree blocked by peer chan-server WIP mid-signature-change). fmt +
  clippy `-D warnings` + tests all green, after the last edit.
- Verification: full design walk executed INSIDE a real dev
  WKWebView build via a throwaway in-page driver (worktree-only,
  reverted) against an isolated $HOME and a real flock conflict —
  36/36 checks pass, including the verbatim Rust lock reason in the
  dialog and all three dismissal paths. Evidence + harness recipe in
  tasks/task-Desktop-Conductor-15.md.
- Gotcha worth remembering: display-asleep Macs suspend WKWebView
  pages ~10s after launch (fetch callbacks too, not just timers);
  three walk attempts stalled at identical spots until
  `backgroundThrottling: "disabled"` (tauri 2.11 window config) +
  App Nap off. Wrote it up in the completion file for reuse.
- Standing duty: persistent isolated build base ready at
  /tmp/chan-desktop-gate (+ warm target dir); main-tree desktop
  builds don't compile until the chan-server WIP lands, so WKWebView
  build requests route through the worktree.
- B5/B6/B4: context recovered from phase-22/23 docs + new-team-1
  bus; per-item notes + proposed scopes written into the completion
  file; awaiting @@Conductor ack before starting any.

## 2026-06-13 — B5/B6/B4 complete

- B5 landed as `f198df7b` (visible-only window cap + "Hidden Windows
  (N, kept warm in memory)" header); decision note with revert path
  in designs/b5-buried-window-cap-decision.md for the round-close
  survey. Isolated gate green after last edit.
- B4 took a turn: the "no Linux route by design" claim is half-wrong
  — wry's GTK source proves capture-at-drag-time delivers real paths.
  Stop-rule honored (no code); @@Conductor ratified option 1: closed
  as documented-no-op with the corrected note
  (designs/b4-linux-drop-path-print-note.md), shim to follow-ups.
- B6 closed the phase-22 unknown with a clean empirical answer:
  in-place Window-menu mutation is SAFE on GTK (12+1 cycles + destroy
  storm + recovery in an sdme Ubuntu container, menu-model readback
  every mutation, zero GTK criticals, menubar visually intact in the
  end screenshot); set_menu fallback stays unwired. Finding + two
  non-menu incidentals (muda text() empty-read after destroys;
  Linux 2nd-window non-materialization in-container) in
  designs/b6-gtk-menu-mutation-finding.md.
- Plumbing learned the hard way: chan needs
  `RUSTFLAGS="-C target-feature=+fp16"` on aarch64-linux (gemm fp16
  asm); registry seeds need metadata_key/created_at/last_seen_at;
  launcher assets are compile-time embedded even in debug (edit →
  rebuild); persisted window-config stacks pollute repeat test runs —
  clear state between runs.
- Container b6gtk stopped (harness + binary retained); teardown at
  round close. Ready for the round-close WKWebView walk build.

## 2026-06-13 — round-close instrumented WKWebView walk (joint w/ @@Editor)

- Walk run at final HEAD b82a0a27 (4 cycles; binary 5d7d5b0f =
  clean base 58b6d195 + declared instrumentation). Full table in
  tasks/task-Desktop-Conductor-36.md.
- Headline greens: item-1 keep-alive core EMPIRICAL on WKWebView
  (hosts mounted + visibility-switched, no raw-markdown flash with
  102 live decorations ×4 readbacks incl. post-flip, undo-across-
  switch + bb877a87 never-empties, flip suite); console sweep
  0 errors / 0 state_unsafe_mutation / 0 warns; A1.5 memory ~8MB/doc
  linear; execCommand text path proven.
- Honest splits: display was asleep+locked all night → compositing-
  dependent asserts (deep scroll, caret/focus, item-2 dynamic SPA
  block) recorded [blocked-env] with driver evidence; A4/DnD/drop
  hand-smoke per @@Editor's gates. Harness retained for a ~2-min
  awake re-run.
- Finding-candidate: hidden-terminal fit-loop SIGWINCH spam starves
  the cs-write queue idle gate (real item-2 hazard IF it reproduces
  composited; 2-min awake check specced).
- Editor coordination: specs + addendum consumed pre-poke (crossed);
  live amendments documented in task-Desktop-Editor-35; co-sign slot
  open in the report.
- Awake-block prep (per Conductor's acceptance + finding-1 decision):
  worktree restored pristine at b82a0a27; CLEAN smoke binary built +
  marker-absence verified (sha 8b64ec7d); walk binary + SPA driver +
  harness preserved at /tmp/chan-rc-walk-bin (walk sha b2ab624b —
  post-relink, the one cycles 2-4 ran on; discrepancy vs first-build
  5d7d5b0f documented). Runbook with finding-1 repro as LINE 1 +
  walk re-run + teardown: designs/awake-block-runbook.md.
- Co-sign received (task-Editor-Desktop-36): all amendments approved;
  A1.2 + drain lines reframed [degraded-env] per spec owner (the
  .pane-mode-preview anchor was right — Pane.svelte:1392 — the chord
  just didn't engage headless); fit-loop finding upgraded to a JOINT
  observation with the B5 production framing (buried windows may
  starve their own poke queues — "bury the lead, lose the pokes");
  awake repro + fix candidates (visibility-gated fit observer /
  idle-signal exemption) specced for next round. Report appended,
  Conductor re-poked.
- @@Editor's formal co-sign filed clean as task-Editor-Conductor-37
  (zero contests). Joint walk CLOSED both sides; lane is idle pending
  @@Conductor's awake-block sequencing (runbook staged, human-gated).
- @@Conductor's finding-1 disposition encoded into the runbook:
  CASE 1 hidden-tab repro GATES the bus commit (stop-rule inline,
  "item-2 delivery broken for hidden tabs = round bar"); CASE 2
  buried-window repro recorded-only in the same session (~5 min,
  fix next-round, B5 escape-hatch data either way); survey
  deliberately NOT amended (starvation is cap-semantics-independent).
- Runbook ACCEPTED as-staged. Lane on HOLD per @@Conductor: survey 2
  carries the awake-session ask to @@Alex; I drive the walk re-run
  when he is at the machine. Standing artifacts: clean binary
  8b64ec7d (target dir), walk harness b2ab624b + driver + report
  server + fixture HOME (/tmp/chan-rc-walk-bin, /private/tmp/chan-rc-*),
  worktree pristine at b82a0a27.
- LINE-1 restructure verified faithful by @@Conductor; runbook
  declared FINAL (no further edits — execute as written). Round
  holding on survey 1.

## 2026-06-13 — AWAKE BLOCK executed (surveys answered: B5 KEEP, undo KEEP)

- LINE 1: CASE 1 (hidden tab) CLEAN — gate PASSES, commit unblocked;
  CASE 2 (buried) CLEAN — fit-loop is asleep-display-only, finding-1
  downgraded to benign, B5 escape hatch needs no data. (Deviation
  disclosed: ran on walk binary + clean dist for the bury IPC.)
- Awake walk re-run (2 cycles; root-caused the chord failures to the
  app keymap matching e.code — App.svelte:434): flipped to PASS:
  A1.1 deep-scroll (3070 preserved, mid-doc raw-flash clean),
  A1.4 caret-lands-once FULL (the Chrome-impossible check),
  A1.6 real pass, Cmd+. engages, I2.1 busy-submit FULL (chip 312ms,
  read-only, pill), I2.9 flip counter-mirror exact; console sweep
  0/0/0 composited. Degraded remainder (drain-dependent I2 items;
  synthetic Ctrl-C inert — xterm keyCode gap) → 30s recipes on
  Alex's shrunk hand-smoke list. New observation for @@Editor:
  pane-mode round-trip resets scrollTop (flip preserves).
- Report: tasks/task-Desktop-Conductor-38.md. Torn down; dist clean;
  Alex's hand-smoke ready on binary 8b64ec7d.

## 2026-06-13 — ROUND 1 CLOSED (@@Alex hand-smoke "All clean"); teardown executed

- Runbook teardown complete: all walk/fixture artifacts removed
  (/tmp/chan-rc-*, /tmp/chan-item6-*, /tmp/rc-bin, walk-binary copy,
  fixture HOME+ws), App Nap pref reverted, b6gtk container removed
  (fs chan-desktop-ubuntu retained), no stray processes.
- KEPT per order: worktree build base /tmp/chan-desktop-gate
  (pristine at b82a0a27) + warm target dir — ready for the next
  build request. Lane STAYING WARM: one host item queued pre-release.

## 2026-06-13 — ROUND 2 add-on queued (graph keep-alive)

- Spec read: designs/round-2-graph-keepalive.md. @@Editor owns the
  web-only feature (graph tab keep-alive + Reload menu item,
  extending their dadd5e64 keep-alive to a third tab kind); my role
  is the WKWebView gate ONLY = walk items 1/6/7 + console sweep,
  same surface/harness as the round-1 item-1 walk. Baseline 00a585b3.
- STANDBY: build request routes through @@Conductor when @@Editor's
  commits land + tree settles. At that point: re-sync worktree
  forward to settled HEAD, rebuild web/dist + binary, recreate the
  walk harness (drivers + report server were torn down at round-1
  close; recoverable in minutes from the recorded synthetic-event
  contract — e.code chords, front-via-unbury, /api/graph+fs-graph
  network watch). Driver = file-keepalive driver retargeted to
  .graph-tab + the network panel. NOT pre-building against spec
  anchors (assert against landed code, not the spec).
- @@Editor LANDED graph keep-alive @ 3fdd4bfe (own-gate green,
  Chrome-verified). Build still HELD by @@Conductor pending
  @@TeamFlow review — walk runs ONCE at settled HEAD, @@Editor drives
  (same harness), @@Conductor pokes me when review clears.
- ADDED walk item (@@Editor flag): out-of-scope hidden-edit on a
  DIR/TAG-scoped graph — their workspace-scoped Chrome test couldn't
  exercise spec verification #5 (workspace scope = everything
  in-scope, no genuine "outside"). Harness implication for fixture
  recreation: seed a scope boundary (subdir of linked notes + files
  outside it); open `cs graph <subdir>/` (dir-scoped), hide, edit an
  OUT-of-scope file on DISK (shell echo, not API — API dedupes), then
  assert ZERO /api/graph on re-activation; pair with the in-scope
  variant (#4) on the same graph for contrast. Assertion wording
  settled with @@Editor at walk start.

## 2026-06-13 — ROUND 2 BUILD GO (graph keep-alive @ 3fdd4bfe)

- Worktree synced forward to 3fdd4bfe; clean smoke binary 36e7e132,
  instrumented walk binary 36ae19d0 (debug IPCs + CSP + throttling).
- Grounded the graph internals: load/reload = GET /api/graph (stream)
  via graphStream; depth probe = /api/fs-graph; menu via right-click
  (.mbtn, Reload between Depth/Copy-link); transform is a
  component-local (not DOM-exposed) so pan/zoom-preservation proxies
  via no-remount (canvas-node identity) + zero refetch.
- changeAffectsScope (GraphPanel ~2318): workspace=always (why #5 was
  unexercisable in Chrome); dir=under-subtree OR visible node; in
  filesystem mode scopedNodeIds uses ancestorsExpanded up to the dir
  root → files outside scoped/ are genuinely out-of-scope. Fixture
  built on that: scoped/ subdir (3 linked #proj notes) + root
  outside-one/two.md (#other) outside the scope.
- Bring-up validated keep-alive structure live (2 .graph-tab mount
  simultaneously; tab texts path=workspace / path=scoped/ — fixed the
  driver's activate() targeting). Network watch via PerformanceObserver
  + fetch wrap on /api/graph + /api/fs-graph.
- Harness staged (report server, launcher + graph drivers, fixture).
  Contract + 2 spec questions (Q1 pan/zoom proxy, Q2 #5 scope) sent
  to @@Editor (task-Desktop-Editor-44); scoring run on their bless.
- @@Editor's specs (round-2-graph-walk-editor-assertion-specs.md)
  corrected the approach: raw /api/graph fetch counts are NOISY
  (fs-graph depth-probe + watcher nonce re-emits), so GOLD signal =
  window.__graphLoads (load() counter injected in source) + per-canvas
  __xform hook for 1b/6 transform read-back. Re-instrumented the
  SOURCE (GraphPanel.load() + GraphCanvas onMount), rebuilt web/dist
  (binary reads dist from disk → no binary rebuild), adopted Editor's
  inside/outside boundary fixture.
- WALK RESULT: 30/30 machine-asserted PASS, 0 FAIL (full 7 items +
  control). Headlines: 1a zero load() on switch (fsProbe noise
  excluded), 1b transform byte-identical across switch, #5 OUT-of-scope
  ZERO reload (the Chrome-impossible gap, now empirical) + in-scope
  control +1, item-6 resume resize()-not-start() via divider-drag,
  console sweep 0 state_unsafe_mutation (canvasEverShown $state-in-
  $effect clean on the real engine).
- Item-6 method correction: cs pane split REMOUNTS (Workspace.svelte
  {#key split.a/b} = tree-shape change), expected + out of the
  tab-switch feature scope; a divider-drag (split.ratio only, no
  remount) is the correct resize and passed clean. No bug filed.
- Report task-Desktop-Conductor-45 (Editor co-sign open). Harness +
  fixture retained for amendments; rebuild clean dist + strip before
  any release smoke. Round-2 walk DONE pending co-sign.
- ACCEPTED both sides: @@Conductor accepted 30/30 pending co-sign
  (split-vs-divider catch validated, #5 closed, item-6 verified,
  console clean); @@Editor co-signing — confirmed the __xform hook
  UPGRADED items 1b/6 from their proxy+eyeball to a literal
  transform-value check (stronger than specced), 1c hand-smoke
  accepted. Per @@Conductor: harness RETAINED for any co-sign re-run
  (verified intact: build base + drivers + fixture + evidence, no
  stray app), integrated gate running their side. STANDING BY for
  round-2 close.
- @@Editor co-sign FILED CLEAN (task-Editor-Conductor-46), zero
  contests; they verified the {#key split.a/b} correction against
  Workspace.svelte:73/89. Joint graph-keepalive walk CLOSED both
  sides. Lane idle, harness retained, awaiting @@Conductor's round-2
  close.
