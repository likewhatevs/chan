# @@LaneA journal - Phase 13 round 1

Append-only self-documenting log. One entry per meaningful action;
date / time + intent + outcome. Subagent activity rolls up here through
@@LaneA.

## 2026-05-28 13:00 @@LaneA - open round 1

Intent: opening round-1 lane. Recovered context from CLAUDE.md,
design.md, roadmap-round-1.md, bootstrap.md, coordination/README.md,
lane-a-request.md. Inbox (event-alex-lane-a.md) empty; cross-lane from
@@LaneB (event-lane-b-lane-a.md) does not exist yet so KIND route
signature has not landed - slice 4 (KIND chip wiring) is gated until
@@LaneB posts.

Worktree: removed stale phase-12-lane-a worktree (only `node_modules`
untracked) and re-created `../chan-lane-a` on a fresh
`phase-13-lane-a` branch off `main` @ b2caa7e9.

Subagent budget: spawning 3 in parallel up front since slices 1/2/3
are file-disjoint and unblocked:
- Subagent 1: editor bugs 1+2+3 (Wysiwyg / Source / draft / list).
- Subagent 2: terminal Shift-Enter bug 4.
- Subagent 3: Inspector path + copy + workspace-root parity.
- Subagent 4: KIND chip wiring - holds for @@LaneB signature.

Per-slice gate (cargo fmt/clippy/test/build + npm check/build) is run
before any merge-ready ping on event-lane-a-alex.md.

## 2026-05-28 13:30 @@LaneA - turn-1 close, bugs 1-4 + inspector landed

Subagent 1 (editor bugs 1+2+3):
- Bug 1: deferred-focus `requestAnimationFrame(() => view?.focus())`
  in `onMount` of `web/src/editor/Wysiwyg.svelte` and
  `web/src/editor/Source.svelte`, gated on `autoFocus`. Mirrors the
  bug-10 precedent but unconditional on caretPending so brand-new
  drafts (no persisted caret) re-claim focus once the empty mount
  settles past the chord handler's body-focus park.
- Bug 2: introduced `NEW_DRAFT_SEED = "# Draft\n"` constant in
  `web/src/state/tabs.svelte.ts` and extended the existing
  empty-and-no-attachments early discard in `handleDraftTabClose`
  to also fire on pristine-seed-and-no-attachments. Seed string
  hardcoded; cross-referenced
  `crates/chan-server/src/routes/drafts.rs::NEW_DRAFT_CONTENT` in the
  docstring rather than plumbing through the create-draft response.
- Bug 3: swapped `Decoration.replace` for `Decoration.mark` in
  `web/src/editor/decorations/blocks.ts` so source `-` / `*` / `+`
  and `1.` / `2)` render verbatim; the existing
  `cm-md-ul-marker` / `cm-md-ol-marker` CSS classes still style them.
  Removed the obsolete `orderedMarkerLabel` dotted-chain entirely.
  No edits needed in `list.ts`, `list_guide_visibility.ts`, or
  `paste_html.ts` (the first two already preserved authored markers;
  the last is HTML-paste turndown where there's nothing authored to
  preserve).
- Subagent also patched a slice-2 typo in `Source.svelte` (missing
  `onSubmit` destructure that was already typed in props) so the
  integrated `npm run check` would pass.

Subagent 2 (terminal Shift+Enter bug 4):
- Three-layer defense:
  1. `web/src/editor/Wysiwyg.svelte` and `web/src/editor/Source.svelte`
     get a high-prec `"Enter"` keymap entry that submits when host
     wires `onSubmit` (CM6 routes `Shift-Enter` to its own newline
     binding via defaultKeymap's `shift:` alt).
  2. `web/src/components/TerminalRichPrompt.svelte` wrapper
     short-circuits any bubbled keydown when `shiftKey && !meta &&
     !ctrl && !alt`.
  3. Wrapper still submits on plain Enter / Cmd+Enter as fallback.
- Placeholder copy at line 61 updated to advertise the corrected
  chord set.
- `web/src/terminal/keymap.ts` NOT touched: it owns the xterm.js PTY
  byte-sequence path (`enterModifier` already correctly returns
  modifier code 2 for Shift+Enter); the bug lives in the Svelte
  composer above the terminal.

Subagent 3 (inspector path + copy + workspace-root parity):
- Factored `copyTextToClipboard(text, {onSuccess, onError})` in
  `web/src/state/store.svelte.ts`; the legacy
  `copyWorkspaceWarningPath` stays for its dialog-scoped sinks but
  now routes through the shared helper.
- `web/src/components/FileEditorTab.svelte` `doCopyPath` and
  `doCopyCwdPath` rewired through the shared helper, preserving the
  transient-status-on-success / persistent-status-on-error behavior.
- `web/src/components/FileInfoBody.svelte` "Show path" toggle now
  reveals `workspace.info.root + entry.path` (absolute) with a
  [COPY] button.
- `web/src/components/WorkspaceInfoBody.svelte` brought to folder-
  inspector parity (aggregate stats grid, File Kinds chips, full
  Code section). Header chip + Notes-directories config section +
  root icon stay workspace-specific by design.
- KIND chip click dispatch is OUT OF SCOPE this turn (slice 4 is
  gated on @@LaneB's KIND route signature).

Integration:
- Two files saw overlapping slice 1 + slice 2 edits
  (`Wysiwyg.svelte`, `Source.svelte`); hunks cleanly separable.
- Per-subagent reports flagged transient red on peer tests during
  mid-flight (slice 1's `blocks.test.ts` rewrite, slice 2's
  placeholder pin updates); integrated re-gate on the worktree HEAD
  was clean.

Gate (worktree HEAD, integrated):
- `cargo fmt --check` pass
- `cargo clippy --all-targets -- -D warnings` pass
- `cargo test` pass (no failures across all binaries)
- `cargo build --no-default-features` pass
- `cd web && npm run check` 0 errors 0 warnings
- `cd web && npm run build` pass (only pre-existing chunk-size +
  ineffective-dynamic-import warnings)
- `cd web && npx vitest run` 1615 passed / 11 skipped

Commits on phase-13-lane-a (off `main` @ b2caa7e9):
- `b2ef3f3b  fix(web): focus new docs, suppress fresh-draft prompt,
  preserve list markers, fix terminal Shift+Enter`
- `ad184179  feat(web): inspector absolute-path + copy button,
  workspace-root parity`

Reported merge-ready on `event-lane-a-alex.md`. Cross-lane ping on
`event-lane-a-lane-b.md` for WorkspaceInfoBody parity so @@LaneB can
rebase the Dashboard workspace-info widget on top.

Browser-smoke deferred to combined-tree re-gate after @@LaneB
merges; per `feedback_svelte_static_gate_misses_runtime` +
`feedback_terminal_webgl_wkwebview` bugs 2 / 4 / inspector
reactivity want a Chrome walk + chan-desktop smoke at that point.

Idle pending:
- @@LaneB's KIND route signature on `event-lane-b-lane-a.md` (slice
  4 gate).
- @@Alex direction (inbox empty).

## 2026-05-28 14:30 @@LaneA - turn-2: slice 4a (path + tag chips)

Poke from @@Alex. Inbox unchanged (empty). Surface check turned up:
- `event-lane-b-lane-a.md` had two appends from @@LaneB at 14:00
  (KIND route signature) and 14:11 (merge gate green, both my
  slices on main `f97146cc`).
- main moved from `b2caa7e9` to `f97146cc` (LaneB's pane bugs +
  integration merge). My commits `b2ef3f3b` + `ad184179` are
  linear ancestors of main.

Worktree rebase: `phase-13-lane-a` rebased onto `main@f97146cc`,
clean (no conflicts).

Slice 4a subagent:
- `KindChip.svelte`: optional `onClick` prop. When set, renders as
  `<button type="button" class="kind-chip clickable">` with
  `cursor: pointer` and `:focus-visible` outline `var(--link)`.
  Unset stays `<span>`. All existing modifiers carry through.
- `FileInfoBody.svelte`: wired `onClick={onSetAsScope}` on both the
  file branch chip and the directory non-Drafts branch chip. The
  `onSetAsScope` callback already routes to
  `openGraphForFile/Directory` per host. Existing "Graph from here"
  button stays. The Drafts pseudo-chip (`<span
  class="kind-chip drafts-chip">`) is NOT a KindChip component, so
  it stays unclickable.
- `TagInfoBody.svelte`: imported `openGraphForTag`; wired the
  header KindChip `onClick` for kind=tag and kind=mention to
  `() => openGraphForTag(nodeId, label)`. The kind=date chip stays
  unclickable (no kind=date graph helper).
- `KindChip.test.ts` (new): three vitest cases covering span vs
  button render branches, click forwarding, and modifier
  pass-through.
- `draftsInspectorFileInfoBody.test.ts`: updated the static `?raw`
  regex pin to cover the new `onClick={onSetAsScope}` attribute on
  the directory branch chip.

Out-of-scope deferrals (slice 4b):
- `openGraphForContact(relPath)` and `openGraphForLanguage(lang)`
  do NOT exist in `state/store.svelte.ts` yet; importing them
  today would break the build. LaneB ships them in the KIND
  backend slice and will ping `event-lane-b-lane-a.md` when they
  land. Slice 4b then wires:
  - FileInfoBody language list (currently plain text rows) to
    `openGraphForLanguage`.
  - FileInfoBody contact buttons (currently route to
    `openGraphForFile` as best-effort) to `openGraphForContact`.
  - Any ContactInfoBody / LanguageInfoBody inspector body chips
    to their respective helpers.

Gate (worktree HEAD `39fd3373`):
- `cargo fmt --check` pass
- `cargo clippy --all-targets -- -D warnings` pass
- `cargo test` pass
- `cargo build --no-default-features` pass
- `cd web && npm run check` 0 errors 0 warnings
- `cd web && npm run build` pass
- `cd web && npx vitest run` 1618 passed / 11 skipped (3 new in
  `KindChip.test.ts`)

Commit on phase-13-lane-a (off `main` @ `f97146cc`):
- `39fd3373 feat(web): clickable KindChip wires path + tag
  inspector chips to Graph`

Reported merge-ready on `event-lane-a-alex.md`. Pinged @@LaneB on
`event-lane-a-lane-b.md` with the slice 4a/4b split + the KindChip
`onClick` API summary (so the Dashboard widget reuse can pick the
clickable variant if needed).

Idle pending:
- @@LaneB's KIND backend slice (helpers for contact + language)
  for slice 4b.
- @@Alex direction (inbox empty).
- Combined-tree browser-smoke + chan-desktop smoke; batching after
  LaneB's KIND backend lands so we're walking one tree.

## 2026-05-28 15:25 @@LaneA - turn-3: slice 4b (contact + language)

Poke from @@Alex. Surface check:
- `event-lane-b-lane-a.md` had two new appends from @@LaneB at
  14:30 (KIND helpers 2a landed at `24f1f31d`) and 15:00 (slice 4a
  merged + 2b live at `11e5fb37` and `7c936504`).
- main moved to `7c936504`. My `39fd3373` (slice 4a) is a linear
  ancestor.
- @@Alex inbox empty.

Worktree rebase: `phase-13-lane-a` rebased onto `main@7c936504`,
clean (no conflicts).

Slice 4b subagent:
- `FileInfoBody.svelte`:
  - Imported `openGraphForContact` + `openGraphForLanguage`.
  - Contact-pill fallback: swapped `openGraphForFile(p)` to
    `openGraphForContact(p)`. `m.path` / `l.path` already carry
    the workspace-relative path; no new field needed.
  - Directory Code section: each `lang.name` row promoted from
    `<span class="lang-name">` to
    `<button class="lang-name">` with
    `onclick={() => openGraphForLanguage(lang.name)}` + CSS reset
    + hover/focus.
  - File Code section language label: new
    `<button class="lang-link">` calling
    `openGraphForLanguage(fileLang)`. `fileLang` captured via
    `{@const fileLang = fileReport.language}` because svelte-check
    loses `{#if fileReport}` narrowing across the arrow handler -
    matches an existing pattern elsewhere in this file.
- `fileInfoBodyKindWiringSlice4b.test.ts` (new): six `?raw`
  source-pattern vitest cases pinning the new wiring strings + CSS
  rules. Mirrors `draftsInspectorFileInfoBody.test.ts` shape.

ContactInfoBody / LanguageInfoBody parity check:
- Neither file exists in `web/src/components/`.
- `InspectorBody.svelte` dispatches `kind: "tag" | "mention" |
  "date"` to `TagInfoBody` and everything else (files including
  `node_kind === "contact"`, plus directories) to `FileInfoBody`.
  Language has no inspector body at all - it lives only as Code
  section rows inside FileInfoBody.
- The lane brief asked for "kind-specific dispatch for `tag`,
  `contact`, `language`" via the chip onClick path. With no
  standalone body files, the contact + language chip-click
  behavior lands as row-level wiring on the surface that actually
  renders those rows (FileInfoBody). Per task spec, did NOT create
  new body files. Flagged to @@Alex + @@LaneB.

Gate (worktree HEAD `08b28da8`):
- `cargo fmt --check` pass
- `cargo clippy --all-targets -- -D warnings` pass
- `cargo test` pass
- `cargo build --no-default-features` pass
- `cd web && npm run check` 0 errors 0 warnings
- `cd web && npm run build` pass
- `cd web && npx vitest run` 1625 passed / 11 skipped (6 new in
  `fileInfoBodyKindWiringSlice4b.test.ts`)

Commit on phase-13-lane-a (off `main` @ `7c936504`):
- `08b28da8 feat(web): wire FileInfoBody contact + language rows
  to KIND graph`

Reported merge-ready on `event-lane-a-alex.md`. Pinged @@LaneB on
`event-lane-a-lane-b.md` with the wiring map + the
ContactInfoBody/LanguageInfoBody absence note (Dashboard reuse can
either use FileInfoBody as-is or LaneB introduces dedicated body
components, which would belong on their side since coupled to the
inspector dispatcher).

Lane-a's round-1 roadmap is now end-to-end shipped:
- Bug 1: new-doc cursor focus (b2ef3f3b).
- Bug 2: fresh-draft "unsaved changes" prompt (b2ef3f3b).
- Bug 3: list marker source preservation (b2ef3f3b).
- Bug 4: terminal Shift+Enter newline (b2ef3f3b).
- Inspector path + COPY + workspace-root parity (ad184179).
- KIND chip wiring path + tag (39fd3373) - slice 4a.
- KIND chip wiring contact + language (08b28da8) - slice 4b.

Idle pending:
- @@Alex direction (inbox empty). All known lane-a scope done.
- Combined-tree browser-smoke + chan-desktop Shift+Enter smoke.
  Ready to drive that walk on @@Alex's nod, or @@LaneB picks it
  up as merge-gate orchestrator.

## 2026-05-28 16:15 @@LaneA - turn-4: empirical smokes on main `b84c1507`

@@Alex nod to drive both pending smokes. Documented on
`event-lane-a-lane-b.md` BEFORE serving (per @@Alex direction to
let LaneB see the plan first).

Setup:
- Built `target/debug/chan` from main `b84c1507`.
- Copied to renamed `/tmp/chan-lane-a-srv` per
  `feedback_persistent_test_server`.
- Seeded throwaway `/tmp/chan-test-phase13a` with welcome.md
  (tags `#phase-13` `#smoke`, contacts `@@alice` `@@bob`,
  rust + python code blocks), notes.md (`-`/`*`/numbered lists),
  Contacts/alice.md + bob.md (frontmatter `chan.kind: contact`).
- Registered + renamed via the scoped binary; served on port 8765.

Chrome walk findings (PASS):
- Bug 1 (new-doc cursor focus): Cmd+N opens a draft; JS probe
  `document.activeElement?.closest('.cm-editor')` is truthy and
  `document.activeElement.tagName === "DIV"` with class
  `cm-content cm-lineWrapping`. Cursor placed at `[2,7]` (just
  after `# Draft`).
- Bug 2 (fresh-draft modal): silent discard fires. Initial
  attempt showed the modal because `web/dist/` was stale
  (pre-slice-1 bundle); after `npm run build` the rust-embed
  debug-mode picked up the fresh bundle and the guard worked.
  Debug `console.log` in `handleDraftTabClose` confirmed
  `isPristineSeed: true`, `isDirty: false`, content/saved/seed
  all `"# Draft\n"` (8 chars, codes `[35,32,68,114,97,102,116,10]`).
  Reverted the debug log + rebuilt.
- Bug 3 (list markers): notes.md renders `- alpha`, `- beta`,
  `- gamma`, `* one`, `* two`, `1. first`, `2. second`,
  `3. third` verbatim with no auto-glyph substitution. The
  `Decoration.mark` swap from slice 1 is doing exactly what was
  intended.
- Slice 3 path: "Show path" toggle reveals
  `/private/tmp/chan-test-phase13a/welcome.md` + a `[COPY]`
  button (`button "Copy absolute path to clipboard"`). Click
  fires without error; clipboard read prompt would need user
  permission grant so couldn't auto-verify the clipboard
  contents - flagged but the shared `copyTextToClipboard`
  helper is gated-green via vitest.
- KindChip path: DOCUMENT chip renders as `<button>` (slice 4a
  onClick prop wiring confirmed); click opens
  `path=workspace` filesystem graph (`gs:"workspace"`, gm:"f")
  with welcome.md highlighted as focal node.
- KindChip tag: `#phase-13` chip (button labeled "open in graph
  (scoped to this tag)") opens `tag=#phase-13` tab
  (`gs:"tag:#phase-13"`, gm:"s"). Inspector swaps to TAG kind
  with `documents: 2` and lists notes + welcome.
- Slice 4b contact: alice pill (button "open in graph (scoped
  to Contacts/alice.md)") opens `contact=alice.md`
  (`gs:"contact:Contacts/alice.md"`, gm:"s"). Backlinks lens
  rendered: 3 nodes / 2 edges (alice -> Contacts/ -> welcome).
- Slice 4b language: Markdown row button (button "open in graph
  (scoped to this language)") opens `lang=Markdown`
  (`gs:"language:Markdown"`, gm:"s"). One-hop bubble lens: 2
  nodes / 1 edge.

Bug 4 (terminal Shift+Enter) walk:
- Opened Hybrid Terminal via Cmd+Alt+P (Rich Prompt).
- Placeholder copy reads "Write your prompt; Enter to send,
  Shift+Enter for a new line" - slice-2 update verbatim.
- Typed "line one" + Shift+Enter + "line two" + Shift+Enter +
  "line three". JS probe confirms
  `cm-content innerText = "line one\nline two\nline three"`,
  hasNewlines: 3. Prompt did NOT submit on Shift+Enter.
- Pressed plain Enter (no modifier). Multi-line buffer submitted
  to shell as 3 separate commands (each got "command not found"
  because the strings aren't shell commands; that's the expected
  shell response). Status bar shows "prompt archived". Prompt
  buffer is empty (placeholder text reappeared).

Chan-desktop gap:
- Chrome MCP only drives Blink. WKWebView is not automatable
  from this CLI environment without GUI tools (osascript /
  cliclick are options but invasive).
- Slice-2 is purely Svelte composer (TerminalRichPrompt) + CM6
  high-prec `"Enter"` keymap entries in Wysiwyg.svelte +
  Source.svelte. No WKWebView-specific code. Chrome exercises
  the exact same JS path; the Chrome PASS above covers the
  behavior.
- Built the desktop binary at
  `/Users/fiorix/dev/github.com/fiorix/chan/target/debug/chan-desktop`
  so @@Alex can spot-check manually if desired.
- Per `feedback_pre_release_merge_unverified`: slice gated-green
  AND Chrome-verified; flagging the WKWebView smoke as
  unverified rather than blocking the round-close.

Cross-lane finding (flagged on `event-lane-a-lane-b.md`):
- TAG + LANGUAGE lens canvas sometimes shows
  "0/N nodes 0/N edges" in status bar while inspector has the
  right counts (contact lens renders correctly). Looks like a
  layout-pass timing issue or an empty-render on the first
  paint. LaneB slice 2b's lens-render side, not lane-a.

Teardown:
- `pkill -f /tmp/chan-lane-a-srv` (scoped per
  `feedback_persistent_test_server`).
- `chan remove /tmp/chan-test-phase13a`.
- `rm -rf /tmp/chan-test-phase13a /tmp/chan-lane-a-srv*`.
- Closed all MCP-group browser tabs.

Process learning to capture for future smokes:
- Always `cd web && npm run build` BEFORE the smoke server, even
  if the binary is freshly built, because debug rust-embed reads
  `web/dist/` per-request and a stale dist can mask a freshly
  merged source fix (this caused a 5-minute false-positive on
  bug 2). Adding this to my mental checklist.

Lane-a's round-1 roadmap is now CODE-shipped AND empirically
verified (Chrome layer). Standing by for release-cut or new
direction.

## 2026-05-28 (round-1 CLOSING) @@LaneA - open: recovered context, surveyed A1-A5

Intent: picking up the round-1 CLOSING task list
(`lane-a-round-1-closing.md`). The smoke report
(`round-1-closing-tests.md`) surfaced 5 lane-a items: A1
WorkspaceInfoBody parity, A2 graph parent-dir inspector, A3
language-node inspector, A4 editor `@{name}` autocomplete, A5
`@@mention` graph nodes/edges.

Worktree: `../chan-lane-a` rebased on `main@5a241f0f`, clean (no
conflicts; my four round-1 commits are linear ancestors of main).

Inbox/surface check:
- `event-alex-lane-a.md`: empty (header only).
- `event-lane-b-lane-a.md`: last @@LaneB append 15:30 (slice 4b
  merged, lane-a roadmap drained). Nothing new for closing.
- `event-lane-b-alex.md`: @@LaneB's 19:30 smoke-report triage.
  @@LaneB owns regressions 1-6 (Cmd+, flip-back, Dashboard empty
  back, empty-pane flip, "Infographics" label, tab kind= prefix,
  QR-donate) before tagging v0.17.0. @@LaneB flagged my A4/A5
  (their items 11) and A2 (their item 12) as Lane A scope.

Source survey (read before acting):
- A2: `InspectorBody.svelte` ALREADY has a `kind: "directory"`
  arm (lines 82-98 -> FileInfoBody dir branch). So "inspector
  missing on parent-dir click" is upstream: `GraphPanel`'s
  folder-node selection isn't producing a `directory` selection
  (or strips the `directory:` id without re-dispatching). Fix
  lives in GraphPanel.selectFromList, not the dispatcher.
- A3: `InspectorBody.svelte` has NO `language` arm; the `else`
  currently misroutes a `kind: "language"` node to TagInfoBody.
  Need a new `language` arm. No LanguageInfoBody component exists
  yet (confirmed in slice-4b survey). Decide inline render vs new
  body file.
- A1: `WorkspaceInfoBody.svelte` renders only one action button
  ("Graph from here", gated on onSetAsScope) and still carries
  the "Notes directories" config section (lines 345-383). Need:
  drop the Notes-directories section entirely (pre-release, no
  migration); add the folder-inspector button ROW (Show in File
  Browser, Graph from here, Upload, Download) by mirroring
  FileInfoBody's folder branch.
- A4/A5: backend `mentions.rs` + `graph.rs` exist; survey of the
  mention-edge emission + the editor completion provider pending
  at execution time.

Cross-lane note for A1: `WorkspaceInfoBody.svelte` is shared turf
(LaneB's Dashboard Workspace widget reuses it). Will ping
`event-lane-a-lane-b.md` BEFORE editing it so LaneB's Dashboard
slice rebases on the new button row + the dropped config section.

Proposed sequencing (smallest-first per the brief): A2 (graph
parent-dir dispatch) or A4 (editor autocomplete) first, then A3,
A5, A1. Standing by for @@Alex go.

## 2026-05-28 (round-1 closing) @@LaneA - empirical triage + A4/A3/A1 shipped

@@Alex "go on". Spun up an ad-hoc diagnostic server (renamed binary
`/tmp/chan-p13close-srv`, port 8791, throwaway workspace seeded with
@@mentions across 4 plain .md files + 1 contact file + rust/python
code) to ground the diagnosis. Torn down at turn close (pkill scoped
to my binary, `remove`, rm -rf, browser tabs closed).

### Empirical findings (curl + Chrome)

- A4 backend WORKS: `/api/mentions?q=` returns all 6 mentions
  (resolved @@Alex + unresolved @@Architect/@@Bob/@@Carol/@@Dana/
  @@Eve). The indexer extracts mention edges from ALL .md
  (`workspace.rs:4213`), not just kind:contact.
- A5 ALREADY SATISFIED: `/api/graph?scope=workspace` returns 5
  mention nodes + 13 mention edges + the resolved contact-file node.
  The semantic workspace graph (default for the Graph button,
  gm:"s") renders 26/26 nodes incl all mentions. The user's "no
  mentions in the graph at all" traces to viewing a FILESYSTEM-mode
  graph (gm:"f", what "graph from here" opens) which has no
  mention/tag/language nodes by design. No data-layer change.
- A2 ALREADY SATISFIED: directory selection renders the full dir
  inspector (Upload/Download/Show Directory/Graph from here + stats)
  in BOTH semantic and filesystem modes (verified via the
  reload-hash `gn=` round-trip for directory:notes + fs dir notes).
  `pickNode` has no kind filter. The user's "parent dir inspector
  missing" is the workspace-ROOT-as-parent case (top-level file's
  parent is the root, id="", which renders WorkspaceInfoBody) -
  resolved by A1's button-row rework. Found one micro-nit:
  workspace-root selection (id="") is dropped from hash persistence
  by the falsy check at `tabs.svelte.ts:3811` (`t.selectedNodeId ?`);
  reload-only, arguably LaneB graph-tab serialization - flagged, not
  fixed.

### Shipped (3 commits on phase-13-lane-a, off main 5a241f0f)

- `70ab238e fix(editor): @-completion surfaces the @@mention corpus
  (A4)` - `includeMentions = true` in contact.ts so the single-`@`
  (wiki) picker also fetches the mention corpus. Merge/dedup +
  dual-commit (contact->wikilink, mention->@@name) already existed
  from fullstack-a-70; just un-gated. Updated the one test that
  pinned `mode === "mention"`.
- `a46e0944 feat(graph): language bubble inspector body (A3)` - new
  `LanguageInfoBody.svelte` (chip + name + files + code lines +
  Graph from here). InspectorBody grew a `language` arm + selection
  variant; GraphPanel maps a selected language node to it and wires
  Graph from here -> `rescopeFromHere("language:<lang>")`. Previously
  a language node fell through to TagInfoBody with a null selection
  (empty placeholder). +7 vitest. RUNTIME-VERIFIED in Chrome:
  selecting the Rust node renders "LANGUAGE | Rust | Graph from here
  | files 1 | code lines 6".
- `3c9f57bd feat(inspector): workspace root reads like a directory;
  Notes dirs dashboard-only (A1)` - WorkspaceInfoBody gained a
  `variant: "inspector" | "dashboard"` prop (+`onReveal`). Inspector
  variant (default) renders the standard dir action row (Upload /
  Download / Show in File Browser / Graph from here, rooted at "");
  Notes-directories config gated to `variant="dashboard"`. GraphPanel
  + FileBrowserSurface use the inspector default; EmptyPaneCarousel
  passes `variant="dashboard"` so LaneB's Dashboard keeps its config.
  Subagent-built (file-disjoint mechanical mirror of FileInfoBody's
  transfer machinery); diff reviewed, reactivity clean ($derived,
  no $state-in-$derived). +9 vitest. SHARED TURF: pinged @@LaneB on
  event-lane-a-lane-b.md before editing EmptyPaneCarousel.

### Gate (worktree HEAD 3c9f57bd, integrated)

cargo fmt --check pass / clippy --all-targets -D warnings pass /
cargo test pass / build --no-default-features pass / npm run check
0 err 0 warn (4112 files) / npm run build pass / vitest 1619 passed
/ 11 skipped (+16: A3 7 + A1 9).

### Verification status

- A3: RUNTIME-verified (Chrome).
- A4 + A1: STATIC-verified (gates + parity tests + reactivity
  review). A4's CM6 contenteditable input + A1's variant render want
  a Chrome walk; canvas-tap + CM6 synthetic input were flaky from
  this harness. Per `feedback_pre_release_merge_unverified` not
  stalling - flagged for @@LaneB's merge-gate combined-tree smoke.
- A2 + A5: reported already-satisfied with evidence; no code change.

Reported merge-ready on event-lane-a-alex.md.





## 2026-05-28 (round-1 closing-2) @@LaneA - open: A5 + A6 surveyed

Intent: picking up the round-1 closing-2 task list
(`lane-a-round-1-closing-2.md`). Two inspector-side items:
- A5: workspace inspector (Dashboard slide 1 + Graph/FB workspace-root)
  doesn't render Languages as clickable graph links like FileInfoBody.
- A6: workspace inspector has no Contacts section at all.

Worktree: `../chan-lane-a` rebased on `main@e30f73ef` (0.17.0 version
bump committed, tag NOT cut - LaneB owns the cut). Clean rebase; my
four+three round-1 commits are linear ancestors.

Inbox/surface check:
- `event-alex-lane-a.md`: empty (header only).
- `event-lane-b-lane-a.md`: last @@LaneB append 15:30 (slice 4b merged,
  lane-a roadmap drained). KIND helpers (`openGraphForLanguage`,
  `openGraphForContact`) live on main since 24f1f31d.
- `event-lane-b-alex.md`: LaneB closing slices 1+2 landed (B1-B12),
  merged + retrospective at 92ea0677, 0.17.0 bump at e30f73ef.

Source survey (read before acting):
- A5: `WorkspaceInfoBody.svelte` lines 447-453 render each language as
  a plain `<span class="lang-name">`. FileInfoBody (844-849) renders it
  as a `<button ... onclick={() => openGraphForLanguage(lang.name)}>`.
  Fix = mirror the button + a callback prop, wire from 3 mounts.
- A6: `WorkspaceInfoBody.svelte` has no Contacts section. FileInfoBody's
  contactPills derive from per-FILE `selectionEdgesFor(path)` refs -
  not applicable to the workspace ROOT. The brief's `prefixReport /
  directReport` hint is wrong (`directReport` doesn't exist;
  `prefixReport` is a code report with no mention/link refs). The
  correct workspace-level source is `graphData.view.nodes` (the shared
  semantic workspace graph): every `kind:"file" node_kind:"contact"`
  node = a resolved contact; every `kind:"mention"` node = an
  unresolved `@@name`. That IS "all contacts in the workspace".

Three mount sites (all need the new props wired):
- EmptyPaneCarousel.svelte:428 (variant="dashboard")
- GraphPanel.svelte:2066 (variant=inspector default)
- FileBrowserSurface.svelte:610 (variant=inspector default)

Plan: single slice (A5+A6 together, same file + same 3 mounts). Add
`onLanguageClick?` + `onContactNavigate?` props (fallback to the store
helpers, mirroring FileInfoBody's contactPills pattern), a
graphData-driven `contactPills` derivation + `ensureGraphLoaded()`
trigger, the language button swap + Contacts section, mirrored CSS,
and extend workspaceInfoBodyParity.test.ts. Then full per-slice gate.

## 2026-05-28 (round-1 closing-2) @@LaneA - A5 + A6 shipped

Single slice (`4280d5f3`), web-only. WorkspaceInfoBody + the three
mount sites + parity test.

A5 (clickable Languages):
- Added `onLanguageClick?: (language: string) => void` prop, defaulting
  to `openGraphForLanguage`. Swapped the Code section's
  `<span class="lang-name">` for a `<button>` firing
  `onLanguageClick(lang.name)` + `title="open in graph (scoped to this
  language)"`. CSS mirrored from FileInfoBody (button reset + hover +
  focus-visible). No layout shift (still grid column 1).

A6 (Contacts section):
- The brief's `prefixReport / directReport` hint was wrong:
  `directReport` doesn't exist and `prefixReport` is a code report
  (no mention/link refs). FileInfoBody's contactPills come from a
  single file's `selectionEdgesFor(path)` - no workspace-root
  analogue. Correct workspace-level source = the shared semantic graph
  snapshot `graphData.view.nodes`:
  - `kind:"file" node_kind:"contact" !missing` -> resolved contact;
    navigates via `onContactNavigate` prop (fallback
    `openGraphForContact`).
  - `kind:"mention"` -> unresolved `@@name`; opens node in-graph via
    `openGraphAtNode`. Label strips the leading `@@`.
  - Deduped by node id, sorted by label.
- `$effect` calls `ensureGraphLoaded()` (cheap, shared global cache;
  FileInfoBody already triggers it for any file's refs).
- Added `onContactNavigate?: (path: string) => void` prop + a Contacts
  `<section class="refs">` with the same `.ref.contact` pill markup +
  person-silhouette `::before` icon FileInfoBody uses. Renders in BOTH
  variants whenever contactPills is non-empty.

Mount wiring (all three): EmptyPaneCarousel:36/428, GraphPanel
import + :2066, FileBrowserSurface import + :610 - each passes
`onLanguageClick={openGraphForLanguage}` +
`onContactNavigate={openGraphForContact}`. Declared the
EmptyPaneCarousel touch on event-lane-a-lane-b.md before editing.

Tests: extended workspaceInfoBodyParity.test.ts (+14 net: A5 prop +
button-swap + 3-mount pins; A6 prop + contactPills derivation + section
render + 3-mount pins). Loosened two pre-existing A1 carousel-mount
regexes (workspaceInfoBodyParity + dashboardTabAndCarousel) for the
now multi-line mount.

Gate (worktree HEAD 4280d5f3):
- cargo fmt --check pass
- cargo clippy --all-targets -- -D warnings pass
- cargo test pass (the known indexer flake
  `writes_to_drafts_subtree_get_indexed_under_drafts_prefix` tripped
  once on the full run, passed in isolation - web-only change can't
  touch it; per feedback_fresh_binary_rewalks)
- cargo build --no-default-features pass
- npm run check 0 err 0 warn (4117 files)
- npm run build pass
- npx vitest run 1639 passed / 11 skipped

Verification status: STATIC + reactivity-reviewed. Per
feedback_svelte_static_gate_misses_runtime A5/A6 are reactive Svelte
($derived contactPills + ensureGraphLoaded $effect); the Chrome /
chan-desktop smoke lives with @@LaneB's merge-gate cycle. Handed a
clean static gate over. Reported merge-ready on event-lane-a-alex.md.

Lane-a's round-1 closing-2 scope (A5 + A6) is drained.

## 2026-05-29 (round-1 closing-3) @@LaneA - COCOMO/Notes-dirs separator

Poke. Surface check:
- main moved 4280d5f3 -> a8d15a88 (my A5/A6 merged in 4e19d8d2; Lane B's
  closing-2 batch incl. the empty-pane right-click retirement I cut at
  b428c4b7; closing-3 + closing-4 Lane B web fixes).
- event-alex-lane-a.md: empty.
- event-lane-b-lane-a.md (01:01): one Lane A closing-3 item routed -
  @@Alex "I explicitly asked for a separator between cocomo and NOTES
  DIRECTORIES, still missing". File-disjoint from Lane B.

Rebased ../chan-lane-a on main@a8d15a88 (clean). WorkspaceInfoBody
untouched on main since my 4280d5f3, so A6 version is current.

Fix (`2506533c`, web-only):
- Added `.notes-dirs` class to the dashboard-variant Notes-directories
  `<section>` (was `class="refs"`, now `class="refs notes-dirs"`) +
  a CSS rule `padding-top: 0.7rem; border-top: 1px dashed var(--border)`.
  Matches the existing `.cocomo` dashed-divider idiom in the same file
  for visual consistency. The `.refs` margin-top supplies the gap above
  the rule. Dashboard-variant-only (the inspector variant drops NOTES
  DIRECTORIES entirely). The divider sits at the TOP of the section, so
  it correctly separates NOTES DIRECTORIES from whatever precedes it
  (COCOMO when no contacts; the A6 Contacts section when present).
- Parity test +1 pin (`.notes-dirs` class + dashed border).

Gate (worktree HEAD 2506533c): cargo fmt --check / clippy
--all-targets -D warnings / test (clean, no indexer flake this run) /
build --no-default-features / npm run check 0 err 0 warn (4117) / npm
run build / vitest 1654 passed / 11 skipped.

Verification: CSS-only render change, no reactivity. Static gate +
parity pin; the visual sits with @@LaneB's combined-tree smoke at
merge-gate. Reported merge-ready on event-lane-a-alex.md.

# @@LaneA journal - Phase 13 round 2

Round-2 scope: the Team Work (formerly Rich Prompt) full-stack revamp.
Dispatch: lane-a-request-round-2.md. Worktree ../chan-lane-a brought to
main (76f5e18b) on branch phase-13-r2-lane-a (clean reset; round-1 work
all merged so worktree was clean).

## 2026-05-29 @@LaneA r2 turn-1 - recon + orchestration plan

Recovered context (CLAUDE.md, design, roadmap-round-2, bootstrap,
README, inbox, this journal). Inbox (event-alex-lane-a.md) holds only
the kickoff. Cross-lane event-lane-b-lane-a.md tail = round-1 closing
items, nothing round-2.

Recon (verified the request's anchors against current source; mapped
every richPrompt/RichPrompt/watcher reference across web/src):
- A1 backend files all present (terminal_sessions 2760, terminal.rs
  2208, rich_prompts route 466, event_watcher 844, ws rich_prompts
  535, lib.rs 1126).
- Frontend rename ripples to ~30 files incl. ~20 test files. The TEST
  surface crosses A2/A3/A4 file-ownership boundaries (source-pattern
  ?raw tests pin the OLD Cmd+P flow). Decision: partition each test
  file to exactly ONE subagent owner; I reconcile any cross-cutting /
  unanticipated test fallout single-threaded at integration. No two
  concurrent subagents edit the same file.
- FileTree.svelte teamLoad refs are COMMENTS only (no code) - A2
  scrubs them. Bubble.svelte is a generic stateless shell with no
  watcher coupling - A4's stub work is BubbleOverlay.svelte only.

FROZEN CONTRACT (dictated top-down so subagents never block on each
other's symbol reports; I verify A2's output matches before Wave 2):
- State: tab.teamWork : TeamWorkState (was tab.richPrompt :
  TerminalRichPromptState). Watcher types removed:
  WatcherEvent/SurveyQuestion/SurveyOption/ScopeGrant/
  TerminalWatcherState + the tab.watcher field.
- Component: web/src/components/TeamWork.svelte (renamed from
  TerminalRichPrompt.svelte), default export, <TeamWork .../>.
- <TeamWork> props (current set MINUS three): prompt={tab.teamWork},
  onSubmit={submitTeamWork}, terminalSessionId={tab.terminalSessionId}.
  DROP watcherPath, onSpawned, bubbleCount. Submit (Cmd+Enter) resets
  tab.teamWork.buffer = "" after send.
- tabs.svelte.ts renames: openActiveTerminalRichPrompt ->
  openActiveTeamWork; primeTerminalRichPrompt -> primeTeamWork;
  paneModeOpenRichPromptTerminal -> paneModeOpenTeamWorkTerminal;
  showOrSpawnRichPromptInFocusedPane REMOVED (new flow replaces it).
  NEW: createTeamWorkLeadTerminal(opts?: OpenTerminalOptions):
  TerminalTab | null - fresh terminal in active pane + opens the Team
  Work editor + returns the created tab (for Cancel deletion).
- Dialog open (A3 implements, App.svelte calls):
  openTeamDialog({ leadTabId, leadPaneId }). Cancel ->
  closeTab(leadPaneId, leadTabId); Bootstrap -> lead-first orchestrator
  using the existing lead tab.
- Chord id app.terminal.richPrompt stays STABLE (Lane B's
  shortcuts.ts). Label string -> "Team Work" (sent to Lane B).

ORCHESTRATION (2 waves; max 3 concurrent subagents):
- Wave 1 (concurrent): A1 Rust backend deletion (crates/ only,
  isolated) + A2 frontend foundation (tabs.svelte.ts F0, client.ts F1,
  api/types.ts, App.svelte F4 new flow, module deletions F2/F11,
  FileTree comment scrub, A2 test partition). A2 = the F0 blocking
  checkpoint; I verify its symbol map before Wave 2.
- Wave 2 (concurrent): A3 TeamWork.svelte + TeamDialog + teamDialog +
  teamOrchestrator + SpawnDialog deletion + A3 test partition; A4
  TerminalTab.svelte + BubbleOverlay/Bubble stub + A4 test partition.
- Integration (me): full gate (cargo fmt/clippy/test/no-default +
  npm check/build/vitest), reconcile remaining test fallout +
  cross-cutting refs, browser-smoke the Cmd+P flow / dialog
  Cancel+Bootstrap / submit-reset / right-click menu / bubble stub,
  then report merge-ready on event-lane-a-alex.md.

Scope-creep flag queued for the bus: the empty-pane pane-mode "P"
picker (paneModeOpenRichPromptTerminal) is OUT of the new dialog flow;
it keeps spawning a plain Team Work terminal (terminal + embedded
editor, no team dialog). The new lead-terminal+dialog flow is the
Cmd+P / Cmd+Alt+P / hamburger "Team Work" entry only. Will confirm
with @@Alex.

## 2026-05-29 @@LaneA r2 turn-1 (cont) - waves landed + integration + smoke

### Wave 1 (A1 backend deletion + A2 frontend foundation, parallel)
- A1 (crates/ only, isolated): deleted the agent-event watcher +
  rich-prompt-workspace spool across terminal_sessions / routes/
  terminal / rich_prompts / event_watcher / chan-workspace rich_prompts
  / lib.rs router + the /api/drafts/rich-prompt spool. Forced-removal
  surfaces beyond the explicit list (all flagged, all direct consumers
  of deleted symbols): workspace.rs wrappers, drafts.rs preflight,
  routes/workspace.rs warnings branch, routes/health.rs
  terminal_event_watcher block (permanently-zero dead counter - A1's
  call, accepted). KEEP surfaces (bus/ws/content-watch/self_writes)
  verified untouched. All 5 cargo gates green.
- A2 (state/api/foundation): all frozen names landed (verified table) -
  TeamWorkState, tab.teamWork, openActiveTeamWork, primeTeamWork,
  paneModeOpenTeamWorkTerminal, createTeamWorkLeadTerminal;
  showOrSpawnRichPromptInFocusedPane removed; watcher types + tab.watcher
  + serialize/restore gone. client.ts: 9 methods + RichPrompt* types
  removed; kept openWatchSocket (verified = content-change WS, NOT the
  agent watcher). Deleted 7 dead modules+tests. Kept teamCreate/teamLoad
  (still called by teamOrchestrator) - flagged.

### Integration call I made BEFORE wave 2
- Pane.svelte (Lane B file): removed the dead t.watcher unread-dot block
  + its CSS (forced by the tab.watcher deletion). Declared on
  event-lane-a-lane-b.md; Lane B reconciles at merge-gate (separate
  worktrees, so no live collision).
- Deleted docs/templates/team-process/ (A2 flagged it was under docs/).

### Wave 2 (A3 dialog/orchestrator/config + A4 TerminalTab/bubbles + A6
### component rename/menu, parallel)
- A3: new path-based team-config vertical. Backend route
  routes/team_config.rs (POST /api/team-config/{read,write}), std::fs
  OUTSIDE the Workspace sandbox per risk #6 (my pre-authorized "small
  path-based capability" call - chosen over shell-cwd because Load must
  read contents back to prepopulate). Reused chan_workspace::TeamConfig
  (TeamConfigWire) WITHOUT extending it (real-estate round-trips via the
  existing per-member position field). Dialog redesign (Neo / New-Load /
  1-9 dropdown / drag-me / real-estate kept / copy-paste removed) +
  lead-first orchestrator (writeTeamConfigFile -> lead into existing tab
  via restartTerminal, no respawn -> workers -> CHAN_TAB_NAME -> identity
  prompt -> deselect-all then enable lead+workers). openTeamDialog({
  leadTabId, leadPaneId}). Deleted teamLoadDialog/teamSplitPaneRealEstate
  tests (pinned removed behavior); rewrote 6 partition tests.
- A4: TerminalTab -382 lines (watcher poll + rich-prompt-workspace
  archival block gone), submitTeamWork resets tab.teamWork.buffer=""
  unconditionally after send (chord logic intact), <TeamWork> drops the
  3 props, <BubbleOverlay> self-contained. New state/bubbleStub.svelte.ts
  (showBubbleStub/hideBubbleStub/bubbleStubVisible). BubbleOverlay is a
  static stub (single+multi-question+F-follow-up). Bubble.svelte
  unchanged (generic shell).
- A6: git mv TerminalRichPrompt.svelte -> TeamWork.svelte (+ .test.ts);
  dropped watcherPath/onSpawned/bubbleCount props; menu reorder (page
  width/source/style | sep | Bubble stack/tray | sep | Collapse prompt);
  deleted SpawnDialog + spawnDialog. Updated component tests.

### Integration (me, single-threaded)
- App.svelte (the glue, depends on both waves): removed SpawnDialog
  import+mount; openActiveTerminalRichPrompt dead-import dropped;
  paneModeOpenRichPromptTerminal -> paneModeOpenTeamWorkTerminal;
  spawnRichPromptFromContext -> spawnTeamWorkFromContext rewritten to
  createTeamWorkLeadTerminal + openTeamDialog({leadTabId, leadPaneId}).
- FileTree.svelte: RETIRED the name-registry "Load Team" menu
  (loadTeamFromMenu + team-dir badge + isTeamDir/TEAM_DIR_RE + the
  Users/Play/uiPrompt imports). It was built on the old name-based
  /api/teams registry, conceptually orphaned by the path-based single
  flow, and could not be meaningfully half-wired (no lead terminal,
  name-registry vs path-config mismatch). Flagged to @@Alex.
- Cross-cutting test sweep (the ?raw source-pattern tests that crossed
  ownership boundaries): rewrote paneModeKeymap, cmdPRichPrompt3State,
  paneModeStaging, toastAutoDismissSweep, PathPromptModal (import path);
  deleted richPromptHistoryPersist.test.ts (entirely the deleted
  archival API). Net web vitest 1568 passed / 156 files.

### Full integrated gate (HEAD 55179ad9)
- cargo fmt --check / clippy --all-targets -D warnings / test (all
  pass) / build --no-default-features: GREEN.
- npm run check 0 errors / npm run build / npx vitest run 1568 passed:
  GREEN.

### Browser-smoke (Chrome, scoped throwaway /tmp/chan-test-p13r2 on a
### renamed binary /tmp/chan-lane-a-r2-srv:8793, torn down)
All PASS, no console errors:
- Cmd+P -> Team Work Lead Terminal (editor armed) + Spawn dialog over it.
- Dialog: Neo default, New/Load toggle, path /tmp/new-team-1/chan-team.toml
  + info line, 1-9 dropdown, members grow with N, drag-me chips (x3 at
  N=3), Split-panes 1x3/3x1 grid + "drop bot(s) here" slots, real-estate.
- Cancel -> exact lead tab deleted, hash empty, dialog closed.
- Bootstrap (N=1, command set to harmless `echo` to avoid launching a
  real agent): tab renamed @@Lead, lead command ran in the SAME tab (no
  respawn), identity prompt placed in the lead editor with the CORRECTED
  text ("We are a team of 1. Our host is @@Neo and the team lead is
  @@Lead... - (no other agents)"), chan-team.toml written to
  /tmp/new-team-1/ (well-formed TOML, OUTSIDE the sandbox).
- Submit (Cmd+Enter): 187-char buffer -> empty (placeholder reappears).
- Right-click menu: new order, removed items gone.
- Bubble stack -> two static example bubbles (single-question +
  multi-question with topic tabs + F-follow-up); clicking an option
  dismisses; no network/fs.
- Teardown: tab closed, server pkilled (scoped), workspace removed,
  /tmp/new-team-1 cleaned.

### Commit + open decisions (reported on event-lane-a-alex.md)
- One atomic commit 55179ad9 on phase-13-r2-lane-a (the feature only
  builds green as a whole; intermediate splits would be red).
- Decisions made (flagged for @@Alex to flip): (1) FileTree Load-Team
  retired; (2) dialog title kept "Spawn agents" (roadmap renamed the
  concept but not this title string); (3) pane-mode P stays a plain
  Team Work terminal (no dialog); (4) chan-team.toml via a path-based
  backend route bypassing the Workspace sandbox (risk #6 authorized);
  (5) teamCreate/teamLoad/teamGetConfig/teamDuplicate/teamListLoaded +
  the /api/teams backend now orphaned dead surface - left for a
  follow-up cleanup, not deleted (out of roadmap scope).

## 2026-05-29 @@LaneA r2 turn-1 (cont) - @@Alex review + name-registry cleanup

@@Alex reviewed the 5 flagged decisions: (1) sandbox-bypass fine, (2)
FileTree retirement fine, (3) confirmed Cmd+P -> new Team Work flow is
the intended change (pane-mode P is the pure rename, no behavior
change), (4) "Spawn agents" title fine, (5) NOT left behind - pre-
release, no back-compat, DELETE the orphaned name-registry. Heads-up to
@@LaneB handled by @@Alex.

Cleanup commit 25c81182 (on top of 55179ad9; new branch HEAD). Pure
dead-code deletion, net -1479 lines, 15 files (crates/ + client.ts):
- client.ts: removed the 6 name-registry methods + dead TeamRefView/
  TeamLoadResponse types. Kept TeamConfigWire/TeamMemberWire (reused by
  the path-based readTeamConfigFile/writeTeamConfigFile).
- Backend (subagent, compiler-guided): deleted routes/teams.rs +
  6 /api/teams* routes + loaded_teams AppState field (+ metadata import
  clear + test fixtures); chan-workspace teams.rs registry (TeamRef/
  TEAM_DIR_PREFIX/create/write_config/load/list/duplicate/owns_preflight
  + tests); workspace.rs team_* wrappers (incl. the dead watch_team) +
  tests; drafts.rs team-dir preflight skip + test. KEPT TeamConfig/
  Member/Position + routes/team_config.rs.
- Gate green: cargo fmt/clippy(0 warnings)/test/build --no-default-
  features; npm check 0 err / build / vitest 1568.
- No re-smoke needed: pure deletion of code with no remaining UI path
  (FileTree retirement removed the only entry); static gate covers it.

New merge-ready HEAD for @@LaneB: phase-13-r2-lane-a@25c81182
(= 55179ad9 feature + 25c81182 cleanup).
