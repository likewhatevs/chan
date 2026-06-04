# journal - @@Lead (phase-18 round-1)

Append-only running log of coordination, dispatch, and gate decisions.

## 2026-06-04 - round-1 open + dispatch

- Self-identified @@Lead via $CHAN_TAB_NAME. Read bootstrap.md + round-1-plan.md
  + round-1/draft.md (v0.26.0 TODO).
- HEAD at dispatch: d5f7dd38 (v0.25.0). All 6 worker tabs (@@LaneA..F) + @@Lead
  spawned and holding in tab-group phase-18 (cs terminal list confirms).
- Cut Wave-1 task files (lean pointers; context lives in plan + draft):
  - task-Lead-LaneA-1.md  Editor: lists parity + hyphen restore + scroll-hang + `[[`
  - task-Lead-LaneB-1.md  Graph: select-on-from-here + dir-edges + contact-stamp
                          (cross-crate lockstep) + STOP auto-reload + copy-link
  - task-Lead-LaneC-1.md  File Browser: ctx-menu regression + shortcut hints +
                          loading-hang (history.replaceState debounce)
  - task-Lead-LaneD-1.md  Inspector: pill + dropdown redesign per category
  - task-Lead-LaneE-1.md  Terminal+desktop: rich-prompt focus + copy/paste +
                          UTF-8 locale + remove desktop pre-flight dialog
  - task-Lead-LaneF-1.md  Repo/docs: consolidate phases 1-16 -> docs/phases via
                          subagent fan-out; NO deletions until Wave 3
- Shared-file sequencing I own this round:
  - shortcuts.ts: C appends FB chords first, THEN E appends terminal chords;
    I run web/scripts/shortcuts-table.mjs ONCE after both, resync main.rs.
  - store.svelte.ts (B graph / C persist), tabs/tabMenu/App: .ts interleave-safe,
    I commit merged. fromHere.ts: D owns seed format, C consumes.
- LaneA item 4 (`[[`) + smoke-client are the two open @@Alex questions. Raising
  the `[[` one now (it gates a chan-server route decision for A); holding the
  smoke-client one for Wave 2 convergence (sequence, don't fire two tiny surveys).
- Told A to do items 1-3 + recon `[[` client, HOLD the route change for the
  survey answer.

### Survey-target gotcha (fixed)
- `cs terminal survey --tab-name=@@Alex` FAILED: "no live terminal session
  matched". This team's config.toml has NO @@Alex member - host has no terminal
  tab. The 7 tabs (Lead + 6 lanes) sit in ONE grid window (row/col positions)
  that @@Alex views. The survey overlay is window-level, so target a tab that
  HIS window owns. Retried `--tab-name=@@Lead` (he poked @@Lead, so that window
  is his) -> blocking live. Use @@Lead (or --tab-group phase-18 if single
  window) to reach the host in this team layout.

### `[[` survey resolved -> cross-lane sequencing
- @@Alex chose "Paths + existing link-targets (both)" - ADDITIVE. Item 4
  unblocked.
- Recon: backend change is confined to chan-workspace `link_targets`
  (graph.rs:1308) + `LinkTargetKind` (graph.rs:1429). chan-server
  `api_link_targets` (routes/graph.rs:127) is a PURE PASSTHROUGH -> A does NOT
  touch routes/graph.rs (that stays @@LaneB's wire-kinds file). Good: removes
  the chan-server contention I worried about.
- BUT `crates/chan-workspace/src/graph.rs` is shared: B = NodeKind/GraphNode/
  contact-stamp (~80-210); A = link_targets (~1308-1480). `.rs` not
  interleave-safe -> SEQUENCED. B has priority (bug, in flight): lands its
  chan-workspace graph.rs portion first, re-cargo check/test -p chan-workspace
  green, pokes me "chan-workspace graph.rs stable"; THEN I release A's edit. If
  B's stamp fix lands in a different file (indexer path), no contention -> I
  release A in parallel.
- Cut task-Lead-LaneA-2.md (decision + boundaries + hold-graph.rs-write) and
  task-Lead-LaneB-2.md (heads-up + the stability-poke ask). Poked both.
- Wave-1 dispatch complete. Now in wait-mode: re-invoked on lane completion
  pokes + B's "graph.rs stable" poke. Holding the smoke-client survey for Wave 2.

### @@LaneF Wave-1 DONE (first lane in)
- 18 consolidated phase docs (1-16 + pub-site-release) -> docs/phases/ in one
  6-section template + docs/phases/README.md index; playbook.md + keep/cut +
  Wave-3 scrub plan. NOTHING deleted (correct - deletions are Wave 3). Untracked;
  commits at round close.
- 3 decisions resolved: #3 orchestration/ KEEP (automation blueprint, project
  memory backs it); #2 pub-site-release KEEP as own doc; #1 per-agent skills/
  subdirs = provisional CUT but ESCALATED to @@Alex (reverses a prior
  self-containment decision = risk class). Cut task-Lead-LaneF-2.md, poked F to
  hold for Wave-3 go.
- PENDING SURVEY BATCH (raise when Wave-1 frontend lands, one decision each,
  sequenced): (1) smoke-client Chrome vs WKWebView [Wave 2]; (2) skills/ subdirs
  cut confirm [Wave 3]. Both non-blocking now.

### @@LaneF Wave-3 plan ratified (hold-time recon caught 5 scrub misses)
- F found 5 stale docs/journals refs the first-pass scrub list missed: shipping
  code comments (desktop/src/connecting.js), CHANGELOG.md, a KEPT orchestration
  file, README, and the PUBLIC docs/coordination.md. Scrubbing refs BEFORE the
  deletion = correct (avoids dead links).
- Ratified task-Lead-LaneF-3.md with 3 guardrails: (1) connecting.js scrub only
  AFTER E's desktop work commits (no concurrent desktop-tree touch); (2)
  CHANGELOG one stale line only - I own the v0.26.0 entry at round-close; (3)
  coordination.md is a PUBLIC content rewrite (not a path swap) -> approved as a
  required consequence of the approved journals deletion, but F stages it +
  pokes me the DIFF for review before the round-close commit. To mention to
  @@Alex for transparency (public-doc edit).
- F still holding for explicit Wave-3 go (after all lanes land + gate + commit).

### @@LaneD Inspector DONE (own-gate-green) - 2nd lane in (1st frontend)
- Whole pill+dropdown redesign inside FileInfoBody.svelte (4 files: it + 3 test
  pins). All 5 categories. svelte-check clean (own files), vitest 162/162, build
  green. Full-tree red = LaneC/LaneA WIP only. Pathspec fingerprint
  70290a427f... base d5f7dd38. Commit = Wave 3.
- fromHere.ts UNCHANGED (D verified the existing ` ${seed}\x01` already does
  {cursor}{space}{path}). Closes the D<->C fromHere coupling: C consumes as-is.
- Ratified Export-to-PDF KEEP (phase-17 A3-iii test-pinned; removing = regress).
  Added to @@Alex survey batch as a one-line confirm (now item 3).
- FLAGGED test-pin blast radius: D edited fileTreeDragOut.test.ts (C-adjacent) +
  dashboardTabAndCarousel.test.ts. Asked D to confirm pin-only; I reconcile at
  merge if C also touches fileTreeDragOut.test.ts.
- SURVEY BATCH now 3: (1) smoke-client; (2) skills/ cut; (3) Export-PDF keep.
  Raise at frontend convergence (2-3 frontend lanes landed). D on call for the
  Chrome smoke.

### coordination.md (public) rewrite reviewed + D pin-flag closed
- Reviewed F's pre-staged coordination.md proposal vs the live doc: line refs +
  BEFORE text accurate, both mandatory edits faithful (journals path the doc
  sends readers to is being deleted). Approved all 3 edits (incl. the optional
  Edit-3 one-sentence hedge so the "How work flows" section doesn't point at
  now-absent alex/event-*.md + role task files). ADDED scope: convert the doc's
  existing em dashes (pre-existing CLAUDE.md violation) as mechanical
  meaning-preserving swaps, since it's THE cleanup round on a public doc.
  F poke-me-the-git-diff before commit stands. task-Lead-LaneF-4.md.
- @@LaneD confirmed (via git diff) its fileTreeDragOut/dashboard test edits are
  PIN-ONLY (inspector Upload/Download pins + onclick->onClick prop rename),
  FileTree-domain + carousel assertions untouched. Blast-radius flag CLOSED:
  clean to merge against C even if C touches fileTreeDragOut.test.ts (different
  regions). D standing by for Wave-2 smoke.

### @@LaneA items 1-3 DONE + item-4 simplified + B contention DISSOLVED
- A landed items 1-3 (cursor parity, hyphen restore, scroll-hang) own-gate-green,
  smoked in Chrome. Fingerprint 50dc82ea..., base d5f7dd38, 5 files, no shared
  touch. Item-3 has a REAL-TRACKPAD hand-smoke caveat (Blink can't repro
  momentum) -> @@Alex hand-smoke list. A also flagged Source.svelte:461 has the
  SAME scroll-behavior:smooth -> I AUTHORIZED A to apply the identical one-line
  fix there (unassigned file, trivial consistency).
- A's report crossed with my task-2 (A hadn't integrated the "both" decision yet)
  BUT its client recon delivered a better design: `[[` path completion can be
  built CLIENT-SIDE off the existing api.list / GET /api/files - NO chan-server
  route change, NO chan-workspace link_targets/graph.rs change. So "both" =
  /api/link-targets unchanged + client-side path completion merged in wiki.ts.
- => the graph.rs SEQUENCING I set up in task-2 is MOOT. task-Lead-LaneA-3.md
  unblocks A's item 4 now (client-side, no B dep); task-Lead-LaneB-3.md RELEASES
  B (owns chan-workspace/graph.rs free & clear, no stable-poke needed). Cleaner
  outcome - the contention dissolved itself via A's recon. Lesson: let the lane
  recon before locking a cross-lane sequence.

### @@LaneC File Browser DONE (own-gate green) + shortcuts C->E released
- 3 items: ctx-menu regression (drop Reload + dead reloadTree/refreshTree, add 3
  root actions), shortcut hints via chordFor(), loading-hang fix (dedup
  replaceState when URL unchanged + 150ms debounce schedulePersistStateToHash;
  sync path kept for pagehide flush; no $state in scheduler). svelte-check 0,
  vitest 169f/1679 PASS (full tree now GREEN -> resolves A/D transient reds),
  build OK. NOT committed (shared imports; my merge role). Fingerprints: clean
  0ccda12b... (8 files), full 6aac8719... (9 incl B store hunk).
- C appended EXACTLY ONE shortcuts.ts entry (app.files.delete=Backspace, new
  "File" group), did NOT resync. C-then-E gate now OPEN -> poked @@LaneE: cleared
  to append terminal chords after C's entry, still no resync (I run it ONCE after
  both land, commit resync'd main.rs).
- Merges I own at convergence: store.svelte.ts (C persist + B graph), App.svelte
  (C layout + E rich-prompt), shortcuts.ts (C + E).

### Convergence tracker
- DONE: F(docs W1), D(inspector), A(items 1-3), C(FB).
- IN FLIGHT: A(item 4 client-side), B(graph 5 items), E(terminal+desktop 4 items
  + shortcuts append).
- Build clean test server + raise survey batch (smoke-client FIRST, then skills/
  + Export-PDF confirms) once A item-4 + B + E land.

### @@LaneB contact-stamp = FRONTEND, not the Rust indexer I hypothesized
- B's poke crossed my task-LaneB-3; both directions now agree NOBODY touches
  chan-workspace/src/graph.rs (A is client-side; B's fix is frontend). My recon
  hypothesis (Rust indexer node_kind:contact stamp + TS lockstep) was WRONG.
- Real cause (image-10): it's a FILESYSTEM graph; a symlink (binary) hit a
  catch-all `else -> mention` in mapFsNodes (GraphPanel), and `mention` shares
  the contact silhouette + amber in GraphCanvas. Fix (landed): map `symlink` to a
  file-shaped node so the canvas classifies by name (binary). Verified on wire.
  Lesson: two of my cross-crate/contention worries (this + the `[[` route) both
  evaporated once the lane reconned - recon before locking sequences/lockstep.
- @@LaneB merge map (for my Wave-3 merges): GraphPanel.svelte (B exclusive,
  items 1,3,4,5); store.svelte.ts graph region (items 4,5, shared w/ C - I
  merge); tabs.svelte.ts GraphTab (item 5, additive - I merge); graphLink.test.ts
  (NEW); crates/chan-server/src/routes/graph.rs (item 2 drafts-layer scope gate,
  B's chan-server file - A does NOT touch it; E's is terminal_sessions.rs, diff
  file same crate). B still finishing; completion task to follow.

### @@LaneE Terminal+desktop DONE (4 items) + shortcuts appended
- 1 rich-prompt hide->focus (one $effect, all 3 hide paths); 2 copy/paste chords
  (Cmd+C/V mac, Ctrl+Shift+C/V else, bare Ctrl stays SIGINT); 3 UTF-8 locale on
  PTY spawn (LANG=C.UTF-8 when no UTF-8 codeset; LC precedence; em dash e2 80 94
  == image-14, empirically validated); 4 desktop pre-flight removal.
- Own-gate green: fmt + clippy -p chan-server/-p chan-desktop, test -p
  chan-server (locale) + -p chan-desktop (74+7), web-check 1685 vitest after the
  shortcuts append. Pathspec: 6 single-lane files (terminal_sessions.rs,
  desktop src-tauri main.rs/serve.rs, desktop main.js/styles.css, TerminalTab.svelte)
  + shortcuts.ts shared. Base d5f7dd38. NOT committed.
- FLAG 1 RATIFIED: E removed the DEAD Rust pre-flight backend too
  (compute_workspace_preflight IPC + PreflightReport + 6 helpers + tests +
  handler reg), not just the JS dialog. Aligns with spec (no desktop pre-flight)
  + pre-release drop-dead-code; self-contained in desktop crate; tests rewritten
  to a contract pin. Asked E to confirm grep: no caller outside renderLocal.
- FLAG 2 ACCEPTED: desktop `make build` (DMG, ~15min target lock) deferred to my
  isolated gate.sh worktree at Wave-3/pre-tag. E's dev-mode desktop gate suffices
  for own-gate.
- CAVEAT -> REQUIRED osChord: chords stored literal Cmd+ display "Cmd+" on
  Linux/Windows though handler uses Ctrl+Shift+. @@Alex spec wants linux/macos/web
  porting + quality bar = no known bug -> told E to add the reload-style osChord
  special-case for terminal.copy/paste. I HOLD the single shortcuts-table.mjs
  resync until osChord lands (resync ONCE, final).
- HAND-SMOKE (WKWebView, @@Alex): E items 1,2,4 (focus, clipboard, desktop
  double-dialog) + A item 3 trackpad + A source-mode scroll. Item 3 UTF-8 is
  Chrome-drivable -> Wave-2 server.

### Convergence tracker (updated)
- DONE: F(W1 docs), D(inspector), A(items 1-3), C(FB), E(terminal+desktop, pending
  osChord).
- IN FLIGHT: A(item 4 client-side), B(graph 5 items), E(osChord one-liner).
- NEXT (mine, at convergence): merge shared files (store/App/tabs/shortcuts),
  resync main.rs ONCE (after E osChord), build clean test server, raise survey
  batch (smoke-client first), run Chrome smokes, assemble @@Alex hand-smoke list.

### @@LaneA item 4 DONE -> editor lane complete + STALE-RED caught
- Item 4 `[[` built client-side off api.list (GET /api/files), BOTH halves:
  /api/link-targets untouched + client PATH candidates (LinkTarget.kind gains a
  client-synth "Path"; computePathHits prefix/contains; tree fetched once per
  `[[`; merged + deduped; "PATH" tag + full path; commit treats Path like File).
  Smoked Chrome: `[[docs/`->2 PATH rows, `[[carb`->File+Heading, `[[docs/phases/
  ph`+Enter-> [phase-17](../docs/phases/phase-17.md). Source.svelte:461 parallel
  scroll fix done. Fingerprint 1f55ffc8..., 9 files incl web/src/api/types.ts
  (additive Path union, no contention - goes in A's commit).
- VERIFIED + corrected a stale-red: A reported full-tree fileTreeSelectionMenu
  .test.ts red ("peer WIP"); I ran it on the shared tree -> 9/9 PASS. C had
  finished that test after A's last full-run. Stale, not a bug. Did NOT let it
  propagate. (cross-agent staleness discipline: verify current state, both can
  be stale.)
- Ratified files-only `[[` (meets "complete paths" spec; dir-link unresolvable;
  dir drill-down rows = optional follow-up, FYI to @@Alex, ships as-is).
- Remaining to converge: B (graph completion) + E (osChord one-liner). Then I
  merge/resync/serve/smoke.

### @@LaneE round-2 DONE + SHORTCUTS RESYNC landed (mine)
- E confirmed compute_workspace_preflight caller-free (whole-repo grep) AND
  caught a GATE-BLIND stale ref the cargo gate can't see: a dead
  `allow-compute-workspace-preflight` permission still DEFINED in tracked
  desktop/src-tauri/permissions/app.toml. Removed; gen/ regenerated clean
  (gitignored). Classic gate-blind-wire (Tauri perms runtime-validated). Would
  have ridden my commit.
- osChord done: mac Cmd+C/V, Linux/Win Ctrl+Shift+C/V; new
  terminalCopyPasteChords.test.ts; web-check 1692 vitest green.
- RESYNC (I ran it): script is PRINT-ONLY; real resync = `--serve-long-about` +
  replace the SERVE_LONG_ABOUT const in crates/chan/src/main.rs. Did it via a
  one-shot py replace (whitespace-safe). Diff: +File group (C Delete) +Terminal
  group (E copy/paste w/ Ctrl+Shift notes) + corrected PRE-EXISTING Dashboard
  drift (Cmd+. i -> Alt+Shift+D (or Mod+. i)). Verified: cargo check -p chan
  green, rustfmt --check main.rs green, regenerate-and-diff idempotent. main.rs
  is MY Wave-3 commit artifact (generated from C+E shortcuts).
- Convergence: ONLY @@LaneB (graph, 5 items) remains. shortcuts.ts merge is the
  last shared-file step done early (resync complete). On B-land: merge
  store/App/tabs, build clean server, raise survey batch, run Chrome smokes.

### @@LaneB Graph lane DONE (all 5) -> all 6 lanes' primary work landed
- 1 select-on-from-here (resolveSelectId matches id OR path; dir id is
  directory:<path>); 2 dir-no-edge = the DRAFTS drafts_link floating at dir/file
  scope -> gated synthesize_drafts_layer to Workspace scope (chan-server graph.rs
  + test); 3 binary-as-contact = frontend mapFsNodes symlink->file-shaped (zero
  Rust); 4 STOP auto-reload (graphReloadSignal carries paths; GraphPanel gates on
  changeAffectsScope; DEFINITIVE smoke: same edit, no-reload subtree vs reload
  workspace); 5 copy-link half done (chan://graph?s=&d=&m=&f=&n=; graphLinkFor/
  parseGraphLink/openGraphFromLink; graphLink.test 5/5). Gate green
  (vitest 1685+5, cargo -p chan-server fmt/clippy/test). Teardown done.
- LAST work item: item-5 EDITOR HOOK (click chan://graph -> openGraphFromLink) is
  @@LaneA's editor domain. Verified B's exports landed (store:2175, tabs:3568/
  3595) + external_links.ts is the editor link-click handler. Dispatched
  task-Lead-LaneA-5 (A imports B's done exports; no contention). Authorized A to
  touch external_links.ts (+ test) as editor coherent-domain.
- NUL-byte in GraphPanel.svelte ~308 (literal NUL edge-key seps): out-of-scope
  follow-up, flag to @@Alex as future cleanup.
- B merge regions: store.svelte.ts (graphReloadSignal+watcher+openGraphFromLink,
  vs C persist), tabs.svelte.ts (filters+graphLinkFor). I merge at convergence.

### Convergence imminent (waiting on A's hook only)
On A's task-5 land: (1) merge shared .ts (store: B graph + C persist;
tabs: B + creators; App: C layout + E rich-prompt), (2) main.rs resync ALREADY
done, (3) build clean test server (npm build -> cargo build -p chan), (4) raise
survey batch (smoke-client FIRST), (5) run Chrome smokes (editor lists/glyph/
hyphen/[[ + graph 5 + FB menu/hang + inspector pills + UTF-8 less/vim + graph-
link click-to-open), (6) assemble @@Alex hand-smoke list (WKWebView/trackpad).

## 2026-06-04 - CONVERGENCE (all 6 lanes done)
- @@LaneA item-5 hook DONE + smoked end-to-end (copy graph link -> paste in note
  -> click opens graph tab w/ scope/mode/filters restored). Editor lane fully
  done (fingerprint 9fad907c..., 11 files incl external_links.ts/.test). ALL
  ROUND-1 WORK LANDED.
- Changeset sanity (git status): maps cleanly to lanes, no surprise files.
  Untracked: docs/phases/ + playbook.md (F), docs/journals/phase-18 (live bus),
  graphLink.test.ts (B) + terminalCopyPasteChords.test.ts (E).
- Convergence gate kicked off: make web-check in background (b337wyagv ->
  /tmp/webcheck_integrated.log). Next: cargo gate (sequential, avoid target-lock
  contention), then build clean server + Chrome smokes.
- store/App/tabs/shortcuts already coexist in the ONE shared tree (no
  branch-merge needed); "merge" = my commit-grouping decision at Wave 3.

### Integrated gate GREEN (frontend + backend)
- Frontend (make web-check): svelte-check 0 errors, vitest 171f/1694 PASS,
  build OK. All 6 lanes' .svelte/.ts + new tests integrate clean.
- Backend (cargo): clippy --all-targets -D warnings GREEN, cargo test GREEN
  (538+404+... all ok). cargo fmt --check FAILED on ONE spot: B's
  chan-server/routes/graph.rs:2581 (drafts_root_only_at_workspace_scope test,
  method-chain reflow). Applied `rustfmt --edition 2021` (semantics-preserving;
  fmt --check now green; drafts test re-confirmed 1 passed). B's -p chan-server
  fmt check went stale before its final edit. Fix rides into B's commit group;
  poked B (FYI, nothing to do).
- Binary build kicked off (cargo build -p chan, bid0xf32i) for the smoke server;
  dist/ bundle already fresh from web-check.
- NEXT: seed a throwaway drive -> serve -> Chrome smokes, prioritizing the
  NOT-yet-lane-smoked items (C FB menu/hang, D inspector pills) + UTF-8 live
  less/vim + integrated sanity; A editor + B graph were lane-smoked, spot-check.
  WKWebView/trackpad -> @@Alex hand list. Survey batch (skills/ cut, Export-PDF)
  after smokes.

### Chrome smoke results (server /tmp/chan-smoke-p18 @ :8787)
- BOOT sanity: PASS (SPA boots, boot menu + correct chord hints, post-boot
  onboarding card = the SPA pre-flight replacement; zero console errors).
- @@LaneC FB: item-1 PASS (tab menu: "Reload" GONE, + New file or Directory /
  New Terminal / New Graph below "Expand all directories"); item-2 PASS (in-tree
  selection menu chords from chordFor: New Terminal Cmd+Alt+T, New Graph
  Cmd+Shift+M, Delete Backspace, Settings Cmd+,); item-3 PASS (rapid dir
  expand/collapse -> ZERO console errors, no SecurityError, no Loading hang).
- @@LaneD inspector pills: Directory (main Open + dropdown Upload here/Download
  tarball/New terminal/Graph), File DOCUMENT (main Open + dropdown Download/New
  terminal/Export-to-PDF/Graph), Media (main View/Zoom), Binary (main Download
  file) - all PASS, main actions + dropdowns per spec.
- INTERRUPTED: Chrome extension disconnected mid-batch (claude.ai/chrome, @@Alex's
  browser - external; 2 reconnect attempts failed). Did NOT get to: UTF-8 live
  less/vim (E - but codeset empirically validated by E), editor hyphen-distinct
  visual (A lane-smoked), graph (B lane-smoked definitively). Binary-dropdown +
  editor Show-Details (5th D category) unsmoked (same uniform pattern).
- Net: the two DEFERRED lanes (C, D) are smoke-validated clean. Remaining
  Chrome-drivable gap = UTF-8 live render (strong backend evidence). Server left
  UP for a possible retry / @@Alex poke-around; tear down at round close.

### Chrome smoke RESUMED (extension reconnected) - completed high-value items
- @@LaneE UTF-8 (item 3): PASS in BOTH less AND vim. New Terminal (C root action)
  opened at workspace root; `less docs/utf8.txt` + `vim docs/utf8.txt` both render
  em dash (a - b), accents (cafe naive resume), CJK, emoji as proper GLYPHS, not
  <E2><80><94> raw bytes. image-14/15 bugs FIXED. (Chrome-valid: it's a PTY-locale
  byte issue, not a renderer one.)
- @@LaneA hyphen-distinct (item 2): PASS. notes.md in Wysiwyg: star/plus lists ->
  disc/open-circle depth glyphs; HYPHEN list -> literal dashes (-), visually
  DISTINCT; ordered -> numbers. Directly resolves @@Alex's "where are my
  hyphenated lists?". (cursor-parity + `[[` live + free-scroll lane-smoked by A.)
- SMOKE COVERAGE COMPLETE: boot, C(1/2/3 + root New Terminal), D(Dir/File/Media/
  Binary pills), E UTF-8 less+vim, A hyphen+list-render. B graph lane-smoked
  definitively. Remaining = @@Alex HAND-SMOKE only (WKWebView/trackpad).
- Round-close package: report green+smoked + hand-smoke list + proceed-survey.

### Round-close survey -> "Commit + run Wave-3 now" + a NEW @@Alex bug
- @@Alex chose option 1 (commit gated-green + run Wave-3; he hand-smokes after;
  NO push). FYI defaults left as-is: Export-PDF kept, `[[` files-only, skills/ CUT.
- 6 atomic code commits landed (docs/* held for F Wave-3):
  c9ea3c56 fix(editor) A | 296f6495 feat(inspector) D | 2e429f27 fix(terminal) E
  | ae22d5a1 fix(graph) B | 9fcf0187 fix(file-browser) C | 3a6623a0 chore(state)
  shared+resync. Each pathspec-scoped, post-commit git show --stat verified.
- NEW BUG (@@Alex, live-testing): click at EOL of a NESTED bullet (2nd level+)
  lands cursor at line START; 1st level OK. Editor lane. Hypothesis: A's
  caret-snap (prefix->first-text-column) over-fires on CLICKS / a nested EOL
  click resolves into the prefix and snaps to text-START instead of text-END.
  Cut task-Lead-LaneA-6, poked A. Fix APPENDS (editor already committed). @@Alex
  also confirmed `[[` works (he completed a real link in notes.md).
- F Wave-3: PARTIAL go (task-Lead-LaneF-5) - phase-17 fold + scrubs (connecting.js
  unblocked, coordination.md diff to me first). HELD: phase-18 fold + ALL
  deletions until editor bug fixed + @@Alex done + final gate (deleting
  docs/journals now would kill the live bus mid-fix).
- HELD round-done: no "done" + no deletions + no push until the editor bug (and
  any further @@Alex finds) are fixed + re-smoked + final make pre-push green.

### 2nd @@Alex bug: graph selection not persisted across reload (-> @@LaneB)
- Root-caused (recon, B-only): serializeLayout (tabs.svelte.ts:3674 `gn`) +
  restore (3855 / store:2183) are ALREADY correct; @@LaneC persistStateToHash
  just calls serializeLayout (no C change). The missing link: GraphPanel's live
  `selectedId` ($state line 728) is never written back to graphState
  .selectedNodeId, so the serializer captures null on a normal click ->
  reload loses it. Fix = sync selectedId -> graphState.selectedNodeId/Label in
  GraphPanel (B's file). NUL-byte grep-binary on GraphPanel (use grep -a).
- Cut task-Lead-LaneB-5, poked B. Fix APPENDS (graph already committed ae22d5a1).
- Two @@Alex bugs now in flight: A (click-EOL-nested), B (persist-selection).
  Both fix-and-append; round-done still held.

### @@Alex ARCHITECTURAL steer: bullet lists need CLEANUP not scaffolding
- 3rd find + a direction change: click IN-text of a NESTED bullet jumps cursor to
  line START (not where clicked). @@Alex's insight: hyphen + ordered "just work"
  (real-text markers); bullet got zero-width-source + CSS-::before depth glyph +
  caret-snap scaffolding, which decouples visual from source position -> breaks
  click/cursor -> each case needs another band-aid. The bugs are all
  bullet-SPECIFIC = the scaffolding is the smell. He wants CLEANUP.
- Cut task-Lead-LaneA-7 (SUPERSEDES task-6 snap): re-approach bullets to use REAL
  positioned marker chars like hyphen/ordered so default CM cursor/click works,
  DELETE the snap (clampListCaretPosition/listAwareArrowDown-Up), unify the 3
  list types on one path. Flagged the one tension: if the google-docs
  disc/circle/square glyph truly can't coexist with correct positioning -> @@Alex
  call (I survey). Regression bar: click-in-text + EOL-click + arrow, depth 1&2,
  all 3 list types. Poked A; A pivots from the snap.
- This validates the "ground-in-source / simplify" instinct; lesson candidate:
  CSS-::before + zero-width-source marker glyphs break CM click/cursor mapping;
  use real positioned chars. Hold the memory until the cleanup proves it out.

### @@LaneB persist fix COMMITTED + F coordination.md signed off
- B's persist-selection fix accepted + committed 0408db30 (GraphPanel.svelte +32,
  explicit pathspec - concurrent WIP in tree). B corrected my recon: GraphPanel
  DOES write tab.selectedNodeId (:2279); my grep missed it (the `tab.` form, +
  NUL-binary). Real cause = the persist TRIGGER (App.svelte effect didn't track
  selectedNodeId). KEPT B's B-only trigger approach over the App.svelte one-liner
  (smoked + matches the open*-fn precedent; not worth rework + shared-file touch).
- F Wave-3 partial committed: 74909e64 (phases 1-17 + playbook + README, 20
  files), 2e372a93 (scrubs: embeddings.rs/graph.rs/pages.yml/connecting.js/
  CHANGELOG 1-line/agent cards, 17 files; cargo green, comment-only). EXCLUDED
  docs/journals + coordination.md + A's WIP - clean pathspec discipline.
- coordination.md diff REVIEWED + SIGNED OFF (Edits 1-3 + em-dash->ASCII + 3
  stale-prose fixes + `<->` arrow). F commits it as commit 3. Greenlit: kept-card
  em-dash sweep (~16, mechanical) + a phase-8 roster row. All safe/reversible.
- STILL HOLDING F's FINAL go (phase-18 fold + ALL deletions): round not settled
  (@@Alex testing; @@LaneA mid bullet CLEANUP, list.ts WIP).
- Bug tracker: B-persist DONE(committed). IN FLIGHT: A bullet-cleanup (task-7,
  supersedes task-6 snap). Round-done held on A's cleanup + @@Alex's testing.

### @@LaneA task-4 (EOL fix) HELD -> fold into cleanup; F coordination.md crossed
- A's task-4 reported the EOL-click fix (task-6 bug) BEFORE processing task-7's
  cleanup directive. Root cause (valuable): large negative text-indent
  (hanging-indent) makes CM6 posAtCoords mis-resolve far-right clicks into the
  marker prefix -> listCaretGuard clamps to text-start. A's fix PREPENDS an EOL
  branch to listCaretGuard = MORE guard scaffolding (the opposite of @@Alex's
  "cleanup not scaffolding"). A even tried a pure-geometry rewrite -> regressed
  near-start clicks (proves the GEOMETRY is the fragility, = @@Alex's point).
  Also: task-4 only fixes EOL clicks, NOT the in-text-nested-bullet bug (task-7).
- Decision: HOLD the task-4 commit; fold it into the task-7 cleanup. task-8 tells
  A to use the negative-text-indent root cause as the cleanup lever (saner
  geometry/real markers -> native CM click resolution -> REMOVE the guards), and
  to FLAG if a minimal guard is genuinely unavoidable (CM6 limitation -> @@Alex
  tradeoff). Report the UNIFIED cleanup (what was removed + the depth-1/2 x
  3-list-type smoke). I commit that, not the standalone EOL branch.
- F coordination.md sign-off poke CROSSED my task-6 (already approved the live
  diff there). Confirmed: commit it + the greenlit sweeps. Deletions still HELD.

### F safe-Wave-3 fully committed (verified) - deletions still HELD
- 4 doc commits verified on HEAD: 74909e64 (phases 1-17 + playbook), 2e372a93
  (scrubs), d5886380 (coordination.md, matches my reviewed diff 32+/27-),
  948faed1 (em-dash sweep + phase-8 roster, 7 files). Tree clean of F's work
  (only A's list.ts/.test WIP + docs/journals live bus remain). Doc gate green.
- F re-staged-and-waiting on the FINAL go. HOLD stands: phase-18 fold + ALL
  deletions wait on A's bullet-cleanup landing + @@Alex's testing settling +
  final make pre-push. I poke F the moment it settles.
- Outstanding before round-close: (1) @@LaneA bullet-cleanup (task-8, the long
  pole); (2) any further @@Alex finds; (3) rebuild :8787 for @@Alex re-verify;
  (4) F final-go (phase-18 fold + deletions); (5) full make pre-push from gate.sh
  worktree; (6) round-done report. NO push (separate @@Alex ask).

### @@LaneA bullet CLEANUP DONE + COMMITTED (688955c5) - the rework @@Alex wanted
- `*`/`+` markers now real glyph-CHARACTER replace-widgets (disc/circle/square by
  depth, real width, like the task checkbox) -> CM handles cursor/click/arrow
  NATIVELY. DELETED all the snap scaffolding (clampListCaretPosition,
  listCaretGuard, isListEolClick, listAwareArrowDown/Up, ::before CSS) - net -81
  lines. NO tension: got BOTH the Google-Docs glyph AND correct positioning (real
  glyph widget gives both). Committed 688955c5 (refactor, append; 146+/227-,
  pathspec 7 files, fingerprint 361b789). Working tree now CLEAN except the live
  bus = all code committed.
- Regression smoke (A, fresh binary, bullet/hyphen/ordered x depth 0/1/2): click
  mid-text-of-nested -> caret where clicked (the task-7 bug GONE); EOL -> line
  end; arrow -> native goal column (item-1 bug gone WITHOUT snap); glyphs render;
  nested outline-indent aligns. ALL pass.
- PROCESS NOTE: @@Alex contacted @@LaneA DIRECTLY in chat (with a ref image) for
  the nested-glyph outline-indent refinement (Part B). That's the host's
  prerogative (host acts outside the team). A folded it in (same bullet domain) +
  routed the OUTCOME to me - correct handling. Acked A: keep routing direct-ask
  outcomes to me. Indent refinement is in-scope + committed in 688955c5.
- Rebuilding :8787 (bf56b2ix6: npm build -> cargo build) so @@Alex re-verifies the
  cleanup + all fixes on a clean bundle. Then: spot-check, F final-go, pre-push,
  round-done.

### Rebuilt :8787 + LEAD SPOT-CHECK confirms the cleanup
- Rebuild green (vite 2.81s + cargo 7.62s); restarted :8787 (PID 22445, fresh
  bundle = A's cleanup baked). No console errors on load.
- Spot-check (my tab, :8787): clicked MID-TEXT of nested bullet "nested two-a"
  -> caret at doc offset 55 (between "ne" and "sted" = where I clicked), NOT
  line-start. THE BUG IS FIXED on a real build. Glyphs (disc/open-circle depth)
  + outline-indent render. A's regression smoke + this confirm the editor lane.
- NOTE: @@Alex has his OWN server :8791 (drive with lists.md) - that's where he's
  been testing. It needs ITS OWN rebuild to show the fixes (or he uses my :8787).
  Did NOT touch his server. Shared Chrome: kept my driving to my own tab.
- Round now SETTLED on code: both @@Alex bugs fixed+committed, cleanup confirmed.
  Asking @@Alex for his final hand-smoke + "satisfied, finalize" before the
  irreversible deletions (he's still actively testing; pre-authorized deletions
  in the round-close survey but I won't delete the live bus mid-test).

### task-8 resolved (crossed task-5) - editor lane fully DONE
- A's task-8 = already satisfied by task-5 (committed 688955c5). Verified in the
  diff: clampListCaretPosition / listCaretGuard / isListEolClick /
  listAwareArrowDown-Up all DELETED; task-4's EOL branch never committed
  standalone. Zero bullet-specific guard code. No "minimal guard unavoidable"
  flag - real-glyph-widget geometry made the bullet path fully native. Only
  clickToPlaceCaret remains = GENERAL blank-area helper (all content, predates
  bullets); kept, not scaffolding, no @@Alex escalation. Editor lane DONE.
- WAITING on @@Alex: final hand-smoke (WKWebView/trackpad) + "satisfied,
  finalize" -> then F final-go (phase-18 fold + deletions) + full make pre-push
  + round-done report. NO push.

## 2026-06-04 - ENDGAME RESTRUCTURE + @@Lead clear-out

@@Alex wound the team down: clear all lanes, recycle ONE for a new desktop bug +
the release, @@Lead clears out once that lane confirms the handoff.
- Recycled @@LaneE as the RELEASE lane (desktop domain for the new bug + can
  execute F's documented Wave-3). Cleared @@LaneA/B/C/D/F (all work committed).
- New bug: ./desktop-bug-report - chan-desktop OFF-toggle flips the UI before the
  server shuts down -> race -> broken UI (workspace ON, OPEN gone). Handed to E.
- Wrote RELEASE-HANDOFF.md (E's takeover: committed state, F's pending Wave-3,
  release mechanics + caveats, NO push w/o @@Alex) + smoke-checklist.md (@@Alex's
  CHECKED vs YET-TO-CHECK). @@Alex does the YET-TO-CHECK after E's patch.
- Clearing out on E's "release handoff accepted".
- 2026-06-04: @@LaneE CONFIRMED "release handoff accepted" with the correct plan
  (desktop fix -> version bump -> isolated full gate + DMG -> F Wave-3
  fold/deletions -> dry-run -> NO push/tag without @@Alex). @@LaneB acked stand-
  down. @@Lead CLEARED OUT - @@LaneE owns the release end-to-end with @@Alex.
  Reminder left for E: fold phase-18.md (retrospective is in THIS journal) BEFORE
  `git rm docs/journals`, since that deletes the journal + handoff.

## ROUND-1 RETROSPECTIVE (for the phase-18.md fold-in)

### Shipped (12 commits, base d5f7dd38, NOT pushed, version still 0.25.0)
Editor (lists parity, distinct hyphen, free-scroll, `[[` paths, + the bullet
cleanup), Inspector (pill+dropdown per category), Terminal+desktop (UTF-8 locale,
copy/paste chords, rich-prompt focus, pre-flight removal), Graph (5 items incl
the auto-reload kill + copy-link click-to-open), File Browser (menu/hints/
loading-hang), docs consolidation (phases 1-17 + playbook + scrubs +
coordination.md). Plus 3 @@Alex live-test bugs fixed: nested-bullet click,
graph-selection persist, the bullet over-scaffolding cleanup.

### Pending (handed to @@LaneE)
Desktop OFF-toggle bug; F's Wave-3 final-go (phase-18 fold + deletions);
0.25.0->0.26.0 version bump; full make pre-push (gateway + --no-default-features
+ desktop DMG) from the gate.sh worktree; tag/release; the outstanding hand-smoke
(smoke-checklist.md); the known external /dl Pages routing gap.

### Highlights
- All lanes landed + the gate stayed green; clean per-lane atomic commits.
- RECON DISSOLVED CONTENTION: two cross-crate worries I set up (the `[[` route,
  the contact-stamp Rust lockstep) both evaporated once the lanes reconned the
  real code (A's client-side `[[` off api.list; B's frontend symlink fix). Let
  the lane recon BEFORE locking a cross-lane sequence.
- @@Alex's "cleanup not scaffolding" steer was the single highest-value signal -
  turned a growing pile of bullet caret-snap band-aids into a -81-line
  simplification (real glyph-widget markers -> native CM cursor/click).
- Agents VERIFIED rather than followed: @@LaneB corrected my recon twice
  (contact=frontend not the Rust indexer; persist=the trigger not a missing
  write). @@LaneE's grep caught a gate-blind dead Tauri permission. @@LaneF's
  hold-time recon caught 5 scrub misses (incl the public coordination.md) before
  a deletion would've made dead links. @@LaneA empirically root-caused (negative
  text-indent breaks CM posAtCoords) instead of trusting my hypothesis.

### Lowlights / what bit us
- MY RECON WAS REPEATEDLY OFF: the graph.rs `[[` contention was moot; the
  contact-stamp Rust hypothesis was wrong; my selectedNodeId grep missed the
  `tab.` form + the NUL-binary; I mis-attributed the bullet bug to item-1. Cost a
  couple of redirect cycles. Architect lesson: ground recon harder + grep -a on
  GraphPanel; offer hypotheses as hypotheses, and trust the lane to correct them.
- @@Alex found 3 click-mapping bugs by hand that the agent self-smokes AND my
  consolidated Chrome smoke MISSED - they were runtime pointer-geometry issues
  (mid-text/EOL clicks on nested rows). Lesson: smoke real POINTER interactions
  at depth, not just element-presence.
- A fmt nit slipped a lane's scoped gate (gate ran before the last edit) - the
  integrated gate caught it. Reinforces gate-after-last-edit.
- Survey-target gotcha (host has no member tab) cost a retry early on.

### Feedback
- Agents: excellent - empirical root-causing, recon corrections, commit
  discipline, and clean pathspec hygiene in a shared tree. The willingness to
  push back on the architect's wrong recon is exactly what kept quality up.
- @@Alex: the hands-on testing + the architectural "cleanup" steer were the
  round's quality backbone; the automated gates would have shipped the
  over-scaffolded bullets green. Direct-to-lane asks worked because the lane
  routed the outcome back to @@Lead.
- @@Lead (me): held the irreversible deletions correctly (never deleted the live
  bus mid-fix), kept the round open rather than declaring done with known bugs,
  and the lean task-file + 1-line-poke bus held. But tighten recon before
  cutting cross-lane sequencing - several of my pre-emptive couplings were
  unnecessary and a couple of hypotheses were simply wrong.
