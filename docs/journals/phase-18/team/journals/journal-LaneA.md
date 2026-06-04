# journal - @@LaneA (Editor lane)

## phase-18 round-1 wave-1

Scope (4 items, all owned files):
1. Bullet + hyphen list cursor/indent/click parity with ENUMERATED lists
   (arrow-down lands cursor BEFORE the glyph today).
2. Restore distinct HYPHEN (`-`) lists (phase-17 regressed into bullets).
3. Trackpad free-scroll hang (reproduce, then fix).
4. `[[` workspace-PATH autocomplete (HOLD route decision; cross-lane via @@Lead).

Owned: web/src/editor/decorations/blocks.ts, web/src/editor/commands/list.ts,
web/src/editor/bubbles/{triggers.ts,wiki.ts}, web/src/editor/widgets/image.ts,
web/src/editor/Wysiwyg.svelte.

### Log

- Started: read bootstrap, task, plan, draft spec. Reading owned files to
  re-verify anchors against HEAD.

- Recon (anchors re-verified against HEAD):
  - blocks.ts: BULLET_GLYPHS depth-cycle (disc/circle/square) at ~445-471;
    decorateBulletList ~473-504 marks the ListMark with the depth glyph and
    IGNORES the source char (- / * / +). All three render identical glyphs.
  - Wysiwyg.svelte CSS: `.cm-md-ul-bullet { font-size: 0 }` (1037-1039)
    collapses the literal marker char; `::before` (1053-1069) draws the
    disc/circle/square. Ordered marker (`cm-md-ol-marker`, 1017-1020) keeps
    the literal `1.` visible (no font-size:0). THIS is the bullet-vs-ordered
    difference behind item 1's cursor weirdness: the bullet marker is a
    zero-width char with a CSS pseudo-glyph, so the caret can land in/around
    it oddly; ordered keeps real visible text.
  - list.ts: clampListCaretPosition (178-184) snaps a caret inside the
    prefix to after-prefix; only wired on mousedown via listCaretGuard
    (199-213). NO arrow-key clamp -> vertical nav can drop the caret in the
    prefix region (item 1).
  - Item 3 scroll: `scroll-behavior: smooth` on `.cm-scroller`
    (Wysiwyg.svelte 775-777, mirrored in Source.svelte 461 which is NOT in
    my lane). Classic CM6 footgun: it animates CM6's own scrollTop
    height-estimation corrections, which fights a trackpad pan ->
    "hang, jump opposite, settle". breathing_room.ts confirms the Google-
    Docs lift comes from 60px padding + scrollMargin, NOT from smooth, so
    removing smooth keeps the lift effect.
  - Item 4 (`[[`): wiki.ts fetchFile -> api.linkTargets -> /api/link-targets
    (basename/title/heading fuzzy). An EXISTING `api.list(dir)` ->
    /api/files returns TreeEntry[] (path,is_dir,kind) = real workspace
    paths; client-side path completion is feasible WITHOUT a route change if
    @@Alex wants PATHS. If link-targets must also return path candidates ->
    chan-server change (cross-lane). HOLD for survey answer; do 1-3 first.

- Plan: reproduce 1/2/3 on a throwaway drive (Chrome), then fix:
  1. arrow-up/down caret clamp (reuse clampListCaretPosition) for parity.
  2. `-` -> distinct visible dash class; `*`/`+` keep depth glyphs.
  3. remove `scroll-behavior: smooth` from Wysiwyg `.cm-scroller`.

- Reproduced all 3 empirically (Chrome, /tmp/chan-laneA-drive, port 8791):
  - Item 1: ArrowDown into `  * nested star a` lands caretX=268 ==
    marker-span left edge (the `○` glyph, font-size 0px) = caret AT the
    glyph, inside the prefix. Ordered reference lands caretX=290 > marker
    right (286) = at the text (offset = line.from + prefixLen). So ordered
    is already correct; bullets are not.
  - Item 2: hyphen list renders identical disc/circle glyphs as `*`/`+`.
  - Item 3: getComputedStyle(scroller).scrollBehavior == "smooth"; setting
    scrollTop=3000 reads back 2 (it animates). Doc is 26612px tall (mostly
    height-estimated) so CM6's scrollTop corrections animate and fight a
    trackpad pan -> the stall.

- Implemented:
  1. list.ts: listAwareArrowDown/Up = cursorLineDown/Up then
     clampListCaretPosition; only re-dispatches when the caret actually
     landed in a prefix (ordered/hyphen keep native goal-column tracking).
     Wired in Wysiwyg.svelte Prec.high keymap AFTER mermaid's stepInto
     (earlier extension, same Prec.high -> tried first) and escapeFence.
  2. blocks.ts: bulletMarkerDecoration(markerChar, depth) -> HYPHEN_MARK
     (`cm-md-ul-marker cm-md-ul-hyphen`, literal dash, no depth cycle) for
     `-`; depth glyph for `*`/`+`. Wysiwyg.svelte: `.cm-md-ul-hyphen` rule
     (no font-size:0, dash visible, matched glyph-to-text margin).
  3. Wysiwyg.svelte: removed `scroll-behavior: smooth` + reduced-motion
     block from `.cm-scroller` (WHY documented inline).
  - Tests: blocks.test.ts rewritten (old tests encoded the regressed
     all-markers->glyph behavior; now `*`/`+` = depth glyph, `-` = hyphen,
     + new hyphen-distinct test + source-pins for cm-md-ul-hyphen).
     list.test.ts: range-selection guard + Wysiwyg keymap source-pins.

- Scoped own-gate GREEN:
  - svelte-check: pass (whole tree).
  - vitest scoped to my files (blocks.test.ts + list.test.ts): 44/44 pass.
  - npm run build: pass (built in 2.71s, with all peers' WIP in tree).
  - NOTE: full `make web-check` shows ONE red, fileTreeSelectionMenu.test.ts
    -> @@LaneC's WIP (FileTree.svelte + its test both peer-modified); a
    concurrent-WIP false-red, NOT my change. Reported to @@Lead, not
    blocking. My 5 files are the only editor/ changes in the tree.

- Empirical smoke (rebuilt binary, fresh bundle, /tmp/chan-laneA-drive:8791):
  - Item 1: star nested ArrowDown caretX 286 > markerRight 281 (was 268 ==
    glyph) = lands at text. ArrowUp into nested hyphen b: caretPastMarker
    true. Sequential nested downs all land at text. Matches ordered.
  - Item 2: hyphen list renders literal `-` at all levels, distinct from
    star/plus disc/circle glyphs (which are unchanged).
  - Item 3: scrollBehavior now "auto"; scrollTop=3000 applies instantly.
  - Mermaid composition: 2nd ArrowDown crosses INTO the mermaid block,
    de-renders to source (renderedMermaid 0, fence visible) -> mermaid's
    stepInto still wins; my listAware handler did not pre-empt it.
  - blocks.ts decorations consumed ONLY by Wysiwyg.svelte (RichPrompt etc.
    don't import chanDecorations) -> hyphen change is fully contained.

- Teardown: stopped scoped server, unregistered + rm'd /tmp/chan-laneA-drive,
  removed /tmp/chan-laneA-bin, closed Chrome tab. No stray laneA processes.

- Completion cut to docs/journals/phase-18/team/tasks/task-LaneA-Lead-1.md;
  pathspec diff sha 50dc82ea (base d5f7dd38). Item 4 HELD for @@Alex survey
  (recon: api.list(dir)->/api/files gives workspace paths -> client-side
  path completion possible with NO route change if @@Alex wants paths).
  Poking @@Lead.

## item 4 unblocked (task-Lead-LaneA-2)

- @@Alex survey answer: `[[` = BOTH (paths + existing link-targets), ADDITIVE.
- SEQUENCING: chan-workspace/src/graph.rs is SHARED with @@LaneB
  (NodeKind/contact-stamp). HOLD my graph.rs WRITE until @@Lead pokes
  "chan-workspace graph.rs stable". Do NOW: client side (wiki.ts + TS
  LinkTarget type + client.ts) + DESIGN backend + write the Rust test.
  Do NOT touch routes/graph.rs (@@LaneB's; it's a pure passthrough).
- Plan: add a Path/Directory LinkTargetKind variant; link_targets also
  prefix-matches workspace paths (files + dirs) so `[[docs/jo` completes
  paths. Client renders the new kind; backend write lands after the poke,
  then browser-smoke path candidates.

## item 4 REVISED (task-Lead-LaneA-3) - CLIENT-SIDE, no backend

- @@Lead superseded task-2: items 1-3 ACCEPTED. Item 4 = do it CLIENT-SIDE
  off api.list (my own recon won), keep /api/link-targets unchanged ->
  graph.rs sequencing behind @@LaneB is MOOT/disregarded. Also AUTHORIZED:
  apply the Source.svelte:461 parallel scroll-behavior fix I flagged.
- Implemented item 4 fully in my owned files:
  - types.ts: LinkTarget.kind += "Path" (client-synthesized; backend
    unchanged).
  - wiki.ts: computePathHits() filters the workspace file tree (api.list,
    GET /api/files recursive) -> Path candidates (rank-1 path-prefix; +
    contains once the query has a `/`); ensureTreeLoaded() lazy-caches the
    tree per bubble; fetchPaths() recomputes per keystroke; activeHits()
    merges link-targets + path candidates, deduped against same-path file
    rows. Path rows render with a "PATH" tag + the full path. Commit/open
    treat Path like File (anchor null). NO triggers.ts change (`/` is
    already a legal `[[` query char).
  - GET /api/files: no `dir` = recursive whole tree; with `dir` = one
    level. Used the recursive form once-per-bubble (bounded notes tree;
    same route the file browser/image picker use).
  - Source.svelte:461: removed `scroll-behavior: smooth` (identical to the
    Wysiwyg item-3 fix) for source-mode parity.
  - Tests: wikiLinkTargets.test.ts += Path-kind union + render + client
    api.list pins. (Fixed a pin: api.list() is a method chain `api\n
    .list()`, used /api\s*\.list\(\)/.)
- Scoped own-gate GREEN: svelte-check 0 errors (1 pre-existing RichPrompt
  a11y WARNING in @@LaneE's file, not mine); vitest 54/54 across my 3 test
  files; npm run build OK.
- Empirical smoke (rebuilt binary, /tmp/chan-laneA-drive:8791):
  - `[[docs/` -> 2 PATH rows (docs/phases/phase-17.md, docs/journals/
    phase-18/summary.md). `[[carb` -> File + H1 Heading link-targets, NO
    path-noise dupes (hasSlash gate + dedup). `[[docs/phases/ph` + Enter ->
    committed `[phase-17](../docs/phases/phase-17.md)` (relativized). BOTH
    halves confirmed.
  - Source mode src-scroll.md: scrollBehavior now "auto", scrollTop=4000
    instant. Parity with Wysiwyg.
- Teardown: stopped scoped server, unregistered+rm'd drive, removed bin,
  closed my Chrome tab (left a peer's :8782 tab untouched - shared group).
- Full pathspec (9 files): sha 1f55ffc8, base d5f7dd38. Completion ->
  task-LaneA-Lead-2.md, poking @@Lead.
- Hand-smoke deferred to @@Alex (Blink can't do trackpad momentum): item 3
  real-trackpad no-stall (Wysiwyg + Source), per @@Lead's tracking.

## ACCEPTED (task-Lead-LaneA-4) - lane DONE, standing by for Wave-2

- @@Lead accepted all 4 editor items + Source parallel fix (fingerprint
  1f55ffc8, base d5f7dd38, 9 files).
- Stale-red correction: @@Lead re-ran fileTreeSelectionMenu.test.ts on the
  CURRENT tree -> 9/9 PASS. The red I saw was @@LaneC mid-update, now landed;
  it was STALE not current. My scoped 54/54 is the authoritative signal for
  my files. (Confirms the concurrent-WIP-false-red call was right.)
- types.ts "Path" union ratified (grouped into my editor commit at Wave 3).
- Files-only `[[` ratified (meets "complete PATHS" spec; directory drill-down
  rows = optional bounded follow-up @@Lead will FYI to @@Alex; ships as-is).
- STATUS: lane complete, no open blockers. Standing by for @@Lead's Wave-2
  consolidated editor smoke on the merged server (lists/glyph/hyphen cursor +
  `[[` both halves + free-scroll). Available for any fixups it surfaces.

## item 5 (task-Lead-LaneA-5) - graph-link click-to-open (cross-lane follow-up)

- Completes Graph item 5: @@LaneB built the copy half ("Copy link to graph"
  tab menu -> chan://graph?... link + openGraphFromLink/GRAPH_LINK_PREFIX
  exports); I wire the EDITOR click side so such a link in a note OPENS the
  graph tab. In-lane (external_links.ts is editor domain); I only IMPORT B's
  exports (no cycle; Wysiwyg already imports store.svelte).
- Implemented in web/src/editor/external_links.ts:
  - Imported openGraphFromLink (store.svelte) + GRAPH_LINK_PREFIX
    (tabs.svelte).
  - Click handler: before the external-URL path, linkUrlAtPos(state,pos)
    (new) returns the raw URL of any scheme; if it startsWith
    GRAPH_LINK_PREFIX && openGraphFromLink(raw) -> preventDefault +
    short-circuit. Else fall through unchanged.
  - Refactored externalUrlForNode to reuse the new rawUrlForNode/
    rawUrlFromChild (DRY; behavior identical - openable-scheme filter still
    lives in externalUrlAtPos). No renderer change needed: handleLink
    (marks.ts) already classes any scheme'd URL as external -> `.cm-md-link`
    (clickable), so chan:// links render clickable already.
  - external_links.test.ts: linkUrlAtPos pure tests (chan:// raw + external
    raw + image null) + source-pins for the click routing.
- Scoped own-gate GREEN: svelte-check 0 errors (same pre-existing RichPrompt
  warning); vitest 64/64 across my 4 test files; npm run build OK;
  external_links.test executes fine (store.svelte transitive import is
  jsdom-safe, proven by component tests).
- Empirical smoke (rebuilt binary, linked-notes drive): opened a workspace
  Graph tab -> right-clicked the TAB -> "Copy link to graph" present (NO
  "Reload", per spec) -> wrote the matching markdown link
  `[..](chan://graph?s=workspace&m=s&f=2ltmaifds)` into a note -> it RENDERS
  as a clickable link -> CLICK opened a NEW graph tab (path=workspace,
  semantic, 7/7 nodes, scope/mode/filters restored). External link path
  intact (linkUrlAtPos non-graph -> falls through to openExternalUrl).
- Teardown: stopped server, unregistered+rm'd drive, removed bin, closed tab.
- Full lane pathspec now 11 files: sha 9fad907c, base d5f7dd38 (adds
  external_links.ts + .test.ts to the prior 9). Completion ->
  task-LaneA-Lead-3.md, poking @@Lead. This was the round's last work item.

## LANE CLOSED - all items ACCEPTED

- @@Lead accepted item 5 (verified my end-to-end smoke). Editor lane fully
  DONE: items 1-4 + Source.svelte parallel scroll fix + graph-link click hook.
- @@Lead running the integrated gate; I stand by for the Wave-2 consolidated
  editor smoke on the merged server. No open blockers from my lane.
- Final deliverable: 11 files, pathspec sha 9fad907c, base d5f7dd38, no
  shared-file touches. Scoped own-gate green throughout (64/64 vitest,
  svelte-check 0 errors, build OK). Empirically smoked every behavioral
  change in Chrome. One hand-smoke deferred to @@Alex (real-trackpad item-3
  no-stall, Wysiwyg + source; Blink can't do momentum).

## BUG fix (task-Lead-LaneA-6): click at EOL of nested bullet -> line START

- @@Alex (Wave-2 smoke): clicking the trailing blank space at the END of a
  NESTED list row dropped the caret at line START; depth-0 (top) correct.
- Root cause (reproduced + probed in Chrome): list lines use a large
  negative `text-indent` (hanging indent). On a deeply-indented row CM6's
  `posAtCoords` mis-resolves a far-right EOL click INTO the marker prefix;
  listCaretGuard's pre-existing prefix clamp then snapped that to text
  START. Browser `caretRangeFromPoint` returns the correct text-end -> it's
  CM6 posAtCoords + text-indent, not the DOM. NOT my item-1 arrow snap
  (keyboard-only); the click clamp is the pre-existing listCaretGuard.
- Fix (list.ts): prepend an EOL branch to listCaretGuard that, by POINTER
  geometry (clickX right of `coordsAtPos(line.to)` on the end row), pins the
  caret to line.to BEFORE the prefix clamp. Pure `isListEolClick(...)`
  extracted for unit tests. Original precise prefix-clamp kept intact (a
  pure-geometry rewrite regressed near-start clicks; the prepend hybrid
  keeps both). Added single-click / modifier / gutter guards.
- Tests: isListEolClick unit tests; geometry click path browser-smoked
  (jsdom has no layout). Own-gate GREEN: svelte-check 0 err; vitest 68/68
  (my 4 editor test files); build OK.
- Smoke (fresh binary): synthetic EOL sweep bullet/hyphen/ordered x depth
  0/1/2 (8 cases) all land at LINE END; REAL clicks: nested EOL -> 45 (line
  end, was 33/start), marker-zone -> 33 (text-start preserved).
- Append commit: list.ts + list.test.ts, sha f20252fe, base 2e372a93.
  Completion -> task-LaneA-Lead-4.md, poking @@Lead.

## CLEANUP (task-Lead-LaneA-7): SUPERSEDES the snap - real bullet markers

- @@Alex after Wave-2: bullets are over-scaffolded; clicking IN-text of a
  NESTED bullet still jumps to line start. Approach change: stop scaffolding,
  make bullet markers REAL positioned chars like hyphen/ordered so default CM
  cursor/click/arrow works, DELETE the snap.
- Diagnosis confirmed: `*`/`+` used a zero-width source char + CSS ::before
  glyph -> visual glyph decoupled from source position -> click/cursor coords
  mapped into the prefix -> each case needed another snap band-aid. The
  scaffolding WAS the bug surface.
- Did:
  - blocks.ts: `*`/`+` marker is now `Decoration.replace({widget:
    BulletGlyphWidget})` rendering the REAL glyph char (● / ○ / ■ by depth,
    real width), like the checkbox replace-widget (no atomicRanges). Hyphen
    stays a literal `-` mark. DOC unchanged (render-only; round-trip writes
    `*`).
  - DELETED the snap: clampListCaretPosition, listCaretGuard, isListEolClick,
    listAwareArrowDown/Up, verticalMoveClampingPrefix (list.ts); the
    ArrowDown/ArrowUp keymap entries + listCaretGuard() extension
    (Wysiwyg.svelte); the cursorLineDown/Up import; the font-size:0 +
    ::before glyph CSS. Kept clickToPlaceCaret (general blank-area helper,
    not bullet-specific) + listLineAt (used by image_drop).
  - Tests reworked: blocks.test (widget glyph char + doc preserved), list.test
    (snap tests removed), clickCaret.test (drop the listCaretGuard ordering
    pin). Net -92 lines (135 ins / 227 del) - less code.
- NO TENSION to flag: got BOTH the google-docs glyph AND correct positioning
  (a real-character glyph widget gives both, as @@Lead hoped).
- Gate GREEN: svelte-check 0 err; full editor vitest 221/221; build OK.
- Smoke (fresh binary, regression matrix - bullet/hyphen/ordered x depth 0/1/2):
  - click MID-text of nested -> caret where clicked (bullet caretX 335 past
    glyph; hyphen 315; ordered 328). The task-7 bug GONE.
  - click EOL -> line end: all 8 cases (synthetic sweep, clickToPlaceCaret).
  - arrow-down AND arrow-up between items -> caret past the glyph (native goal
    column, NOT before-glyph). The ORIGINAL item-1 bug gone WITHOUT snap.
  - glyphs render: ●/○/■ depth cycle, dashes, ordered numbers.
- Cleanup commit (supersedes the bullet bits of c9ea3c56 + the task-6 snap):
  7 files, sha 6b11d952, base 948faed1. Recommend landing as its own
  refactor commit (append-only, clean diff) vs reworking c9ea3c56 - @@Lead's
  call. Completion -> task-LaneA-Lead-5.md, poking @@Lead.

## indent refinement (@@Alex direct chat request, mid-cleanup)

- @@Alex (watching the test, direct in chat + reference image): nested-item
  glyphs should align under the FIRST TEXT CHARACTER of the parent line
  (Google-Docs outline indent) - "push the glyphs a few pixels to the right
  on all lists with nested items". Folded into the bullet cleanup (same
  file/domain).
- Root: row-1 glyph x came purely from the markdown source indent (2
  spaces/level for bullet/hyphen, 3 for ordered), which renders narrower
  than the marker column in the proportional body font, so the child glyph
  sat LEFT of the parent text. The CSS prefix/text-indent cancel for row 1
  so they don't move the glyph.
- Fix (Wysiwyg.svelte .cm-md-list-line): added
  `margin-left: calc(var(--cm-md-list-depth) * var(--chan-editor-body-size)
  * 0.6)`. margin (not padding/text-indent) shifts the WHOLE line so row-1 +
  wrapped rows move together (hanging indent preserved). Value tuned LIVE in
  the browser (injected style, no rebuild): 0.6em/level aligns the bullet
  child glyph exactly under the parent text at depth 1 AND 2.
- Smoke (baked binary, measured): bullet d1 childGlyph==parentText, d2
  grandGlyph==childText (exact); hyphen +2px, ordered +3px (within @@Alex's
  "a few pixels"). Click mid-text of a nested item still lands mid-text
  (margin didn't break CM6 coordinate mapping).
- Gate GREEN: svelte-check 0 err; editor vitest 221/221; build OK.
- Full pathspec now 7 files (cleanup + indent): sha 361b7894, base 948faed1.
  NOTE for @@Lead: @@Alex requested the indent refinement DIRECTLY in chat
  (he was watching) - folding it in + flagging so you're in the loop.

## ACCEPTED + COMMITTED (688955c5)

- @@Lead accepted + committed the bullet cleanup as 688955c5
  ("refactor(editor): real glyph-widget bullet markers, delete caret-snap
  scaffolding", net -81 lines). task-8 (HOLD task-4) was satisfied by the
  same cleanup (task-4's EOL branch + all bullet guards deleted, not
  committed). @@Lead confirmed @@Alex reaching me directly in chat is fine
  (he's the host) and routing the OUTCOME through @@Lead was correct.
- Editor lane DONE. Torn down my :8791 server/tab/drive/bin (no laneA
  procs); @@Lead rebuilding :8787 as the canonical re-verify server for
  @@Alex. Standing by.
- Full editor-lane summary (all committed/accepted this round): items 1-4 +
  Source.svelte scroll + graph-link click hook + bullet cleanup (real glyph
  markers, snap removed, outline indent). Only open follow-up = @@Alex's
  hand-smoke of item-3 no-stall on a real trackpad (Blink can't reproduce
  momentum).

## STAND DOWN

- @@Lead cleared @@LaneA to stand down: round work done + committed (c9ea3c56
  editor items + 688955c5 bullet cleanup). @@LaneE owns the release from here.
- Nothing open from this lane. Worktree/servers/tabs torn down. Closing out.
