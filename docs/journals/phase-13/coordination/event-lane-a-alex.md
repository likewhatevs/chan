# @@LaneA -> @@Alex

Append-only. @@LaneA writes progress + merge-ready slices here. Most
recent entry at the bottom.

Merge-ready format:

```
ready to merge: phase-13-lane-a@<sha>  -  <one-line slice summary>
```

## 2026-05-28 13:30 @@LaneA -> @@Alex
round-1 turn-1: bugs 1-4 + inspector slice landed on phase-13-lane-a

Highlights
- Three in-session subagents ran in parallel on file-disjoint scope
  except the Wysiwyg.svelte + Source.svelte overlap (slice 1 = rAF
  focus in onMount, slice 2 = high-prec Enter keymap in extensions).
  Hunks were cleanly separable; integrated gate is green.
- Two commits ready for @@LaneB to merge-gate:
  - `ready to merge: phase-13-lane-a@b2ef3f3b  -  bugs 1-4 (new-doc
    cursor focus, fresh-draft prompt suppression, list marker source
    preservation, terminal Shift+Enter newline)`
  - `ready to merge: phase-13-lane-a@ad184179  -  inspector absolute-
    path + COPY button + workspace-root parity with folder inspector`
- Pinged @@LaneB on event-lane-a-lane-b.md: WorkspaceInfoBody parity
  is on `ad184179`; Dashboard widget can rebase its workspace-root
  reuse on top.

Lowlights
- Slice 1 + slice 3 both edited tests that referenced the placeholder
  pin from slice 2; per-subagent gate runs reported transient red
  while peers were mid-flight. Integrated re-gate (cargo
  fmt/clippy/test + no-default-features build, npm run check, npm run
  build, full vitest 1615 pass / 11 skip) was clean before commit.
- Bug 2 fix hardcodes the literal `"# Draft\n"` seed in
  `tabs.svelte.ts` rather than plumbing it from
  `crates/chan-server/src/routes/drafts.rs::NEW_DRAFT_CONTENT`.
  Cross-referenced in the constant's docstring. Calling out so you
  can redirect if you'd rather plumb it through the create-draft
  response.
- No browser-smoke run yet. Per
  `feedback_svelte_static_gate_misses_runtime` the bug-2 + inspector
  reactivity changes want a Chrome walk; per
  `feedback_terminal_webgl_wkwebview` bug 4 wants a chan-desktop
  smoke. Will batch a smoke after @@LaneB lands their merge gate so
  we're walking the combined tree.

Contention
- KIND chip wiring (slice 4) is gated on @@LaneB's KIND route
  signature landing on event-lane-b-lane-a.md. Idle until then; I
  will watch the channel on each turn.

Detail in `lane-a/journal.md`.

## 2026-05-28 14:30 @@LaneA -> @@Alex
round-1 turn-2: slice 4a (KIND chip path + tag) on top of f97146cc

Highlights
- @@LaneB merge-gated my turn-1 work into main (`f97146cc`) and
  posted the KIND route signature. Rebased `phase-13-lane-a` on the
  new main (clean), then landed slice 4a:
  - `ready to merge: phase-13-lane-a@39fd3373  -  clickable KindChip
    wires path + tag inspector chips to Graph (slice 4a)`
- KindChip grew an optional `onClick`; when set it renders as a
  styled <button> with focus ring, otherwise stays a <span>.
  FileInfoBody wires both the file and directory header chips to
  `onSetAsScope`; TagInfoBody wires the tag/mention header chip to
  `openGraphForTag`. The existing "Graph from here" button stays as
  a parallel affordance per the roadmap.
- Integrated gate green: cargo fmt/clippy/test + no-default-features
  build, npm check/build, vitest 1618 pass / 11 skip (3 new in
  `KindChip.test.ts`).

Lowlights
- Slice 4b (contact + language KindChip wiring) is gated on @@LaneB
  shipping `openGraphForContact` + `openGraphForLanguage` in their
  KIND backend slice; importing those today would break the build.
  Will pick up the moment LaneB pings the channel.
- No browser-smoke yet on either turn-1 or turn-2 work. Per
  `feedback_svelte_static_gate_misses_runtime` + `feedback_terminal_webgl_wkwebview`,
  the bug-2 / bug-4 / inspector reactivity walks + chan-desktop
  Shift+Enter smoke want a combined-tree pass before release cut.
  Plan to batch after LaneB lands their KIND backend slice so I'm
  smoking one tree, not two.

Contention
- Idle pending LaneB's KIND backend slice (helpers).
- Pre-release per `feedback_pre_release_no_backcompat`: I have not
  added migration / graceful-degrade for any of the renamed /
  reshaped surfaces this round (KindChip API, NEW_DRAFT_SEED guard,
  copyTextToClipboard helper). Call out if you want different
  behavior.

Detail in `lane-a/journal.md`.

## 2026-05-28 15:25 @@LaneA -> @@Alex
round-1 turn-3: slice 4b (contact + language) - lane-a roadmap done

Highlights
- @@LaneB landed KIND backend slices 2a (`24f1f31d`,
  `openGraphForContact` + `openGraphForLanguage` helpers + title
  prefixes) and 2b (`11e5fb37`, contact lens BFS + language 1-hop
  bubble), then merged slice 4a into main at `7c936504`. Rebased
  `phase-13-lane-a` on the new main (clean).
- Slice 4b shipped on top:
  - `ready to merge: phase-13-lane-a@08b28da8  -  contact pills +
    language rows in FileInfoBody dispatch to openGraphForContact /
    openGraphForLanguage (slice 4b - completes KIND chip wiring)`
- All four KIND kinds (path / tag / contact / language) now route
  to their kind-specific graph helper. The roadmap's "FILE KIND and
  LANGUAGES become links" + "clicking the hashtag or a contact or a
  language is effectively 'Graph from here' for that kind" is
  satisfied. The "Graph from here" header button stays for
  kind=path per the spec.
- Integrated gate green: cargo fmt/clippy/test + no-default-features
  build, npm check/build, vitest 1625 passed / 11 skipped (6 new in
  `fileInfoBodyKindWiringSlice4b.test.ts`).

Lowlights
- ContactInfoBody / LanguageInfoBody don't exist. `InspectorBody`
  dispatches every non-tag/mention/date kind to `FileInfoBody`, and
  language lives only as a Code-section row. The header-KindChip
  wiring requested in the lane brief lands as ROW-LEVEL wiring
  rather than per-kind inspector body. Calling out so you can
  redirect if you wanted standalone inspector body files (would be
  a larger refactor).
- `fileReport` narrowing leaks across Svelte arrow handlers;
  required a `{@const fileLang = fileReport.language}` capture
  (existing pattern in this file). Not a bug, but worth flagging
  for the next wave of inspector wiring.
- Still no browser-smoke. With LaneB's KIND backend now live and
  all my chips wired, the combined-tree walk (bug 2 draft prompt,
  bug 4 Shift+Enter under chan-desktop per
  `feedback_terminal_webgl_wkwebview`, KindChip click → graph
  lens render, language row buttons, contact pill lens fallback)
  is the natural next step. Ready to drive that walk now if you
  give the nod, or LaneB might pick it up as merge-gate
  orchestrator.

Contention
- Lane-a's round-1 roadmap is now end-to-end shipped on
  `phase-13-lane-a`. No open lane-a scope. Slice 4b is the last
  merge-ready item from this lane unless you redirect.

Detail in `lane-a/journal.md`.

## 2026-05-28 15:50 @@LaneA -> @@Alex
driving both smokes - heads-up posted to LaneB first

You pinged "ok go on" - picking up both pending empirical smokes
(Chrome + chan-desktop). Plan posted on `event-lane-a-lane-b.md`
ahead of serving so LaneB sees it before I touch a binary, per
your "document it before you execute" direction.

Setup:
- Built `target/debug/chan` from current main `b84c1507`
  (includes all 4 of my lane-a commits + LaneB's pane bug slice +
  KIND backend 2a/2b + Dashboard rename 3a + Dashboard 3b-1).
- Seeded throwaway workspace at `/tmp/chan-test-phase13a` with a
  welcome.md (tags + contacts + rust/python code blocks),
  notes.md (hyphen / asterisk / numbered lists for bug 3), and
  two contact files under `Contacts/`.
- Will serve from a renamed binary copy at `/tmp/chan-lane-a-srv`
  on a non-default port (per `feedback_persistent_test_server`)
  so a broad pkill from LaneB's slice 3b smoke doesn't stomp
  mine, and I scope my pkill to `/tmp/chan-lane-a-srv`.

Walk targets (Chrome MCP):
- bug 2: Cmd+N opens an empty draft; closing it must NOT show the
  "Unsaved changes from a previous session" modal.
- bug 1: same Cmd+N draft has cursor focus ready to type.
- bug 3: hyphen / asterisk / numbered list lines in notes.md
  render their source markers verbatim (no auto bullet glyphs).
- Inspector path slice: "Show path" reveals absolute path with
  COPY button; COPY puts text on the clipboard.
- KindChip clicks per kind: open welcome.md, click each
  KIND-style chip in the Inspector and verify a graph tab opens
  with the lens-shaped title (`path=...`, `tag=...`,
  `contact=...`, `lang=...`).
- Language row buttons in directory + file Code section.
- Contact pill click → openGraphForContact.
- (parity with workspace-root inspector)

Then chan-desktop:
- Build chan-desktop with current main.
- Open Hybrid Terminal, run an agent prompt (codex/claude if
  available, else a shell heredoc), press Shift+Enter; verify
  newline (no submit). Plain Enter + Cmd+Enter must still submit.

Teardown: stop my server, `rm -rf /tmp/chan-test-phase13a`,
`chan remove /tmp/chan-test-phase13a`. Tabs closed.

Will report findings + any regressions on this channel in the
next entry.

## 2026-05-28 16:15 @@LaneA -> @@Alex
smoke walks complete - everything green, one process gotcha to share

Highlights
- Chrome smoke walked the full combined tree (main `b84c1507`) and
  every lane-a target is PASS:
  - Bug 1 (new-doc cursor focus): Cmd+N opens the draft, JS probe
    confirms `focusInEditor: true` with the cursor in `.cm-content`.
  - Bug 2 (fresh-draft "Close Draft" modal): silent discard fires
    on close, no modal. Debug log confirmed `isPristineSeed: true`,
    `isDirty: false`, content/saved/seed all match `"# Draft\n"`.
  - Bug 3 (list markers): notes.md renders `- alpha` / `* one` /
    `1. first` verbatim, no auto-glyph substitution.
  - Slice 3 path: "Show path" reveals
    `/private/tmp/chan-test-phase13a/welcome.md` (absolute) +
    COPY button beside it.
  - KindChip path: DOCUMENT chip click opens
    `path=workspace` filesystem graph centred on welcome.md.
  - KindChip tag: `#phase-13` chip opens `tag=#phase-13` semantic
    graph (inspector swaps to TAG with `documents: 2`).
  - Slice 4b contact: alice pill opens `contact=alice.md` with 3
    nodes / 2 edges (backlinks lens).
  - Slice 4b language: Markdown row button opens `lang=Markdown`
    with 2 nodes / 1 edge.
- Bug 4 (terminal Shift+Enter):
  - Placeholder copy now reads "Write your prompt; Enter to send,
    Shift+Enter for a new line" (slice-2 update verbatim).
  - Shift+Enter inserted newlines (3 lines preserved as
    `line one\nline two\nline three`, no submit).
  - Plain Enter submitted the multi-line buffer to the shell (3
    separate commands run, prompt archived).

Lowlights
- Process gotcha: the very FIRST close attempt on a fresh draft
  showed the modal because `web/dist/` was stale from BEFORE the
  slice-1 fix. After `npm run build` (debug rust-embed reads
  web/dist on every request) the fix worked. For smoke walks
  always `cd web && npm run build` before serving, even if you
  rebuilt the binary, to make sure dist matches the committed
  source. I'll bake this into my future smoke checklists.
- Chan-desktop NOT driven empirically. Chrome MCP only drives
  Blink; from the CLI I have no GUI automation for WKWebView.
  Slice-2's fix is purely Svelte composer + CM6 keymap — no
  WKWebView-specific code, so Chrome exercises the exact same JS
  path and the bug-4 PASS above covers the behavior. Built the
  desktop binary so you can spot-check yourself if you want:
  `/Users/fiorix/dev/github.com/fiorix/chan/target/debug/chan-desktop`.
  Per `feedback_pre_release_merge_unverified` the slice is gated-
  green; flagging the WKWebView smoke as unverified.
- KIND-lens graph render quirks: tag and language lenses
  occasionally show "0/12 nodes 0/13 edges" briefly even though
  the inspector lists the right counts. Contact lens renders the
  backlinks subgraph correctly. This is LaneB's lens-render side
  (slice 2b), not lane-a — flagging so you have visibility, but
  not blocking lane-a.

Contention
- Lane-a's round-1 roadmap is now CODE-shipped AND empirically
  verified (modulo the WKWebView gap above). No open lane-a
  scope. Ready for release-cut whenever you give LaneB the nod.

Teardown
- Stopped /tmp/chan-lane-a-srv.
- `chan remove /tmp/chan-test-phase13a`.
- `rm -rf /tmp/chan-test-phase13a /tmp/chan-lane-a-srv*`.
- Closed all MCP-group browser tabs.

Detail in `lane-a/journal.md`.






## 2026-05-28 (round-1 closing) @@LaneA -> @@Alex
round-1 closing: A4+A3+A1 shipped; A2+A5 already-satisfied (evidence)

Three slices ready to merge on phase-13-lane-a (off main 5a241f0f):

```
ready to merge: phase-13-lane-a@70ab238e  -  A4: editor single-@ completion now surfaces the @@mention corpus
ready to merge: phase-13-lane-a@a46e0944  -  A3: language bubble inspector body (name + file/code + Graph from here)
ready to merge: phase-13-lane-a@3c9f57bd  -  A1: workspace-root inspector reads like a directory; Notes dirs dashboard-only
```

**Highlights**
- A4: the single-`@` picker only listed contact FILES; it now also
  fetches the mention corpus, so `@name` surfaces `@@<Name>` handles
  referenced anywhere in markdown. Picking a contact still inserts a
  wiki-link, picking a mention inserts `@@Name`. The endpoint +
  merge/dedup already existed (fullstack-a-70); this un-gates it.
- A3: clicking a language bubble now opens a real inspector
  (LANGUAGE chip + name + file count + code lines + Graph from here).
  Verified live in Chrome.
- A1: workspace-root inspector now carries the standard directory
  button row (Show in File Browser / Graph from here / Upload /
  Download) and drops the Notes-directories config. To avoid
  breaking LaneB's Dashboard slide 1 (which reuses this body for the
  config), I gated config behind a `variant="dashboard"` prop rather
  than deleting it; the Dashboard keeps it, the inspector doesn't.

**Lowlights / scope finding (please weigh in)**
- A2 and A5 appear ALREADY SATISFIED in current code; I could not
  reproduce the reported breakage and did not fabricate a change:
  - A5: `/api/graph?scope=workspace` already returns mention nodes +
    edges (5 nodes / 13 edges on my seed), and the default semantic
    Graph renders them (26/26 nodes). Mentions are extracted from
    ALL .md, not just kind:contact. Your "no mentions at all" most
    likely came from a FILESYSTEM-mode graph (what "graph from here"
    opens, gm:"f") which has no mention/tag/language nodes by design.
    Making mentions show in the fs graph would be a graph-default /
    mode decision (LaneB's territory), not a data fix.
  - A2: directory selection renders the full dir inspector in both
    graph modes (verified). The "parent dir inspector missing" is the
    workspace-ROOT-as-parent case (a top-level file's parent is the
    root -> WorkspaceInfoBody), which A1 now makes read like a normal
    directory. So A1 resolves the A2 symptom.
  - If you DID see a non-root directory render blank, it may be a
    canvas hit-test edge case I couldn't reproduce - point me at the
    exact file/dir and I'll dig.
- Verification: A3 runtime-verified (Chrome). A4 + A1 static-verified
  (full gate + parity tests + reactivity review); CM6 input + canvas
  taps were flaky to automate from here, so the A4 `@`-picker walk +
  A1 inspector/dashboard variant walk are flagged for the merge-gate
  combined smoke (per feedback_pre_release_merge_unverified).

**Contention**
- A1 touched EmptyPaneCarousel.svelte (LaneB's file) for the
  one-line `variant="dashboard"`; declared on event-lane-a-lane-b.md
  before editing. LaneB can reclaim that edit at merge-gate.
- Micro-nit found in A2 dig: workspace-root selection (id="") is
  dropped from graph-tab hash persistence by a falsy check
  (tabs.svelte.ts:3811); reload-only, LaneB serialization - flagged,
  left for them.

Full gate green (cargo fmt/clippy/test/build-nodefault + npm
check/build + vitest 1619/11). Detail in lane-a/journal.md.

## 2026-05-28 (round-1 closing-2) @@LaneA -> @@Alex
ready to merge: phase-13-lane-a@4280d5f3 - A5 + A6 (workspace inspector Languages + Contacts)

ready to merge: phase-13-lane-a@4280d5f3  -  clickable Languages + Contacts in workspace inspector (A5+A6)

**Highlights**
- A5: workspace inspector language rows are now graph-opening buttons
  (Dashboard slide 1 + Graph/FB workspace-root), matching FileInfoBody.
- A6: new Contacts section in the workspace inspector. Derived from the
  shared semantic graph snapshot (`graphData.view.nodes`) since the
  workspace ROOT has no single-file refs - resolved contact files +
  unresolved `@@name` mentions both surface as clickable pills.
- Both wired through all three WorkspaceInfoBody mount sites.
- Full per-slice gate green; vitest 1639/11 (+14 net from A5/A6 pins).

**Lowlights**
- The brief's A6 data-source hint (`prefixReport / directReport`) was
  wrong - `directReport` doesn't exist; `prefixReport` is a code
  report with no refs. Used `graphData.view` (the correct
  workspace-level contacts source) instead. Flagged in journal.
- STATIC + reactivity-reviewed only. A5/A6 are reactive Svelte;
  Chrome / chan-desktop smoke belongs to @@LaneB's merge-gate cycle
  (per feedback_svelte_static_gate_misses_runtime). Not stalling
  per feedback_pre_release_merge_unverified.

**Contention**
- A5/A6 touch EmptyPaneCarousel.svelte (LaneB's file) for the import +
  the slide-1 mount props; declared on event-lane-a-lane-b.md before
  editing. File-disjoint from LaneB's closing-2 carousel work
  (their QR fix ~line 411; my edits at import + the slide-1 mount
  ~428). Both props default-fallback to the store helpers, so an
  un-wired mount stays functional. LaneB can reclaim at merge-gate.

Lane-a round-1 closing-2 scope (A5 + A6) is drained. Detail in
lane-a/journal.md.

## 2026-05-29 (round-1 closing-3) @@LaneA -> @@Alex
ready to merge: phase-13-lane-a@2506533c - COCOMO/Notes-dirs separator

ready to merge: phase-13-lane-a@2506533c  -  separator between COCOMO and NOTES DIRECTORIES in the workspace inspector (dashboard variant)

**Highlights**
- The separator you asked for (twice) is in. `.notes-dirs` dashed
  top-border on the Notes-directories section, matching the COCOMO
  divider idiom already in WorkspaceInfoBody. Dashboard-variant-only.
- Rebased on main@a8d15a88; file-disjoint from Lane B. Full gate
  green (vitest 1654/11, no flake).

**Lowlights**
- CSS-only, so static-verified + parity-pinned only; the actual
  pixels want a glance at @@LaneB's merge-gate combined smoke (per
  feedback_svelte_static_gate_misses_runtime, though a static CSS
  rule is low-risk).

**Contention**
- None. Single file (WorkspaceInfoBody.svelte) + its parity test.

Lane A closing-3 scope (the one routed item) is drained. Detail in
lane-a/journal.md.

---- Round 2 ----

## 2026-05-29 @@LaneA -> @@Alex
r2 open: Team Work revamp - recon done, 4 subagents dispatching

Highlights
- Worktree on phase-13-r2-lane-a @ main 76f5e18b (clean reset).
- Verified the request's file/line anchors against current source and
  mapped every richPrompt/RichPrompt/watcher ref across web/src
  (~30 files, ~20 tests). Froze the full rename + prop contract
  top-down (detail in lane-a/journal.md) so the A1-A4 subagents never
  block on each other's symbol reports.
- Orchestration: Wave 1 = A1 (Rust backend deletion, crates/ only,
  isolated) + A2 (frontend foundation: tabs/client/types/App + dead-
  module deletions). Wave 2 = A3 (TeamWork component + dialog redesign
  + lead-first orchestrator + SpawnDialog deletion) + A4 (TerminalTab
  + static bubble stub). I reconcile cross-cutting tests + run the full
  gate + browser-smoke at integration.
- Sent the "Team Work" label string to @@LaneB on
  event-lane-a-lane-b.md (chord id app.terminal.richPrompt stays
  stable per the plan).

Scope flag (one decision, non-blocking - proceeding with the call,
flip me if you disagree)
- The empty-pane pane-mode "P" picker keeps spawning a PLAIN Team Work
  terminal (terminal + embedded editor, no dialog). The new lead-
  terminal + Spawn-agents dialog flow fires on Cmd+P / Cmd+Alt+P /
  Hybrid hamburger "Team Work" only. Routing a modal into pane-mode
  staging would be odd; the roadmap only names Cmd+P + the hamburger.

Lowlights / contention
- None yet. The riskiest piece is the ~20 source-pattern (?raw) tests
  pinning the OLD Cmd+P flow; I'm partitioning them one-owner-per-file
  and sweeping any cross-cutting fallout single-threaded at
  integration so no two subagents edit the same file.

Detail in lane-a/journal.md.

## 2026-05-29 @@LaneA -> @@Alex
ready to merge: Team Work revamp - full slice gated + browser-smoked

ready to merge: phase-13-r2-lane-a@55179ad9  -  Team Work full-stack revamp (rename + agent-event watcher backend deletion + lead-first Cmd+P/dialog flow + path-based chan-team.toml + bubble stub)

Highlights
- One atomic commit (the backend deletion + state rename + dialog/flow
  redesign all rewrite the same surface and only build green together;
  71 files, net -8325 lines). 4 in-session subagents ran in 2 waves on
  disjoint files; I owned App.svelte (the glue) + the cross-cutting
  ?raw source-pattern test sweep at integration single-threaded.
- Full gate GREEN: cargo fmt/clippy/test/build --no-default-features;
  npm run check (0 err) / build / vitest 1568 passed (156 files).
- Browser-smoked the whole flow end to end, no console errors:
  Cmd+P -> lead terminal + dialog; dialog new shape (Neo / New-Load /
  1-9 dropdown / drag-me / real-estate / drop slots); Cancel deletes
  the exact lead tab; Bootstrap is lead-first (renamed @@Lead, ran in
  the SAME tab - no respawn dance - identity prompt placed with the
  corrected text, chan-team.toml written to /tmp/new-team-1 OUTSIDE the
  sandbox); Cmd+Enter resets the draft to empty; right-click menu new
  order; bubble stub shows single+multi-question+F-follow-up and just
  dismisses.

Decisions I made (proceeded per your "make obvious calls" steer; flip
any of these and I'll redo)
1. chan-team.toml New/Load = a small path-based backend route
   (/api/team-config/{read,write}) using std::fs OUTSIDE the Workspace
   sandbox, per your risk #6 authorization. Chose this over the
   "lead-terminal shell cwd" option because Load must read the file
   back to prepopulate the dialog - the shell path can't feed that.
2. FileTree's right-click "Load Team" (the old name-registry team
   loader) is RETIRED. It was built on the /api/teams name registry,
   orphaned by the new path-based single flow, and can't be cleanly
   half-wired (no lead terminal; name-vs-path mismatch). If you want a
   tree-based load kept, I'd rewire it to open the dialog in Load mode
   seeded with the file's path - say the word.
3. Pane-mode "P" (the empty-pane Hybrid-Nav picker) stays a PLAIN Team
   Work terminal spawn (no dialog). The lead-terminal+dialog flow fires
   on Cmd+P / Cmd+Alt+P / hamburger "Team Work" only.
4. Dialog title still reads "Spawn agents" (the roadmap renamed the
   Rich-Prompt concept but didn't name this title string). Easy to
   switch to "Team Work" if you'd prefer.

Lowlights / contention
- Pane.svelte (Lane B file): I removed its dead t.watcher unread-dot
  (forced by the watcher-field deletion). Declared on
  event-lane-a-lane-b.md; @@LaneB reconciles at merge-gate.
- Orphaned dead surface left for a follow-up (NOT deleted, out of
  roadmap scope): the name-based /api/teams backend +
  teamCreate/teamLoad/teamGetConfig/teamDuplicate/teamListLoaded client
  methods + chan-workspace team registry are now unused after the FileTree
  retirement. Worth a cleanup pass next round if you agree they're dead.
- Desktop/WKWebView not driven (Chrome/Blink only, per
  feedback_terminal_webgl_wkwebview); the flow is pure Svelte/CM6 with
  no WKWebView-specific code, so Blink exercises the same path. Flagging
  per feedback_pre_release_merge_unverified, not stalling.

Detail in lane-a/journal.md. Lane-A round-2 scope is fully shipped.
Standing by for @@LaneB's combined-tree merge-gate + any redirect.

## 2026-05-29 @@LaneA -> @@Alex
done: dead name-registry deleted - new HEAD @25c81182

Your call on (5) done. Decisions (1)-(4) confirmed unchanged.

ready to merge: phase-13-r2-lane-a@25c81182  -  Team Work revamp (55179ad9) + orphaned name-registry deletion (25c81182)

- Deleted the whole name-based team registry: client.ts 6 methods +
  dead types; backend routes/teams.rs + 6 /api/teams* routes +
  loaded_teams AppState; chan-workspace teams.rs registry lifecycle +
  workspace.rs team_* wrappers (incl. a dead watch_team I'd missed) +
  the drafts team-dir preflight skip. Net -1479 lines, pure deletion.
- KEPT TeamConfig/Member/Position + the path-based
  /api/team-config/{read,write} route (the live New/Load surface).
- Gate green (cargo fmt/clippy 0-warn/test/no-default; npm check 0-err/
  build/vitest 1568). No re-smoke: dead code had no UI path after the
  FileTree retirement, so the prior end-to-end smoke still stands.

New branch HEAD is 25c81182 (two commits: feature + cleanup). @@LaneB
should gate THIS, not 55179ad9. Lane-A round-2 scope fully drained.
