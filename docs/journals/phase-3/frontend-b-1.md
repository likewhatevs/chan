# frontend-b-1: Read-only frontend support review

Owner: @@Syseng (historical task originally assigned as @@FrontendB).

Status: REVIEW.

Related:

- [request.md](./request.md)
- [journal.md](./journal.md)
- [frontend-1.md](./frontend-1.md)
- [frontend-2.md](./frontend-2.md)
- [frontend-3.md](./frontend-3.md)
- [backend-1.md](./backend-1.md)
- [backend-2.md](./backend-2.md)
- [webtest-1.md](./webtest-1.md)

## Goal

Provide read-only frontend support while @@Frontend owns the implementation
queue for [frontend-1.md](./frontend-1.md), [frontend-2.md](./frontend-2.md),
and [frontend-3.md](./frontend-3.md).

## Scope

Load the frontend/webdev skill and inspect the codebase for risks, file
ownership boundaries, likely implementation points, and test/smoke ideas.

Do not edit files. Do not change task statuses except this file. Do not run
formatters or commands that modify the workspace.

## Requested output

Record concise findings in this file:

- likely files/functions for each frontend task;
- collision risks with current dirty work;
- missing acceptance criteria or unclear behavior;
- test and browser-smoke suggestions for @@Webtest;
- any small implementation notes that would help @@Frontend avoid rework.

## Acceptance criteria

- Findings are specific and cite relative source paths.
- Notes are actionable for @@Frontend and @@Webtest.
- No source files are modified.
- Any recommendation that would require backend/API changes is routed through
  @@Architect before implementation.

## Progress notes

### 2026-05-16 @@Architect identity cleanup

Alex clarified the agent slot used as @@FrontendB is the same slot now
operating as @@Syseng. Treat this as historical REVIEW/support notes only; do
not ping a separate @@FrontendB identity for this task.

### Current dirty-work snapshot (read 2026-05-16)

`git diff --stat` shows substantive in-flight changes that overlap all three
frontend tasks. Key files (do not revert):

- `web/src/components/{AccessoryPill,AssistantInspectorBody,FileEditorTab,InlineAssist,Pane,ScopeHistoryOverlay,SettingsPanel}.svelte`
  and `web/src/state/shortcuts.ts` — visible Assistant → Agent string flips
  for frontend-1.
- `web/src/components/{FileBrowserOverlay,FileTree}.svelte` — File Browser
  Cmd+F + GitHub-style folder/chevron icons (frontend-2 landed bullets).
- `web/src/components/{FindBar.svelte}` + `web/src/editor/{find.ts,base.ts}`
  — `FindAdapter.placeCursor` for Cmd+F Enter landing (frontend-2 landed).
- `web/src/components/PathPromptModal.svelte` + new `web/src/state/lcp.ts`
  — Tab-complete via longest common prefix (frontend-2 landed).
- `web/src/editor/Wysiwyg.svelte` + new `web/src/editor/extensions/list_guide_visibility.{ts,test.ts}`
  — multi-indent hang-indent + 1.5s list-guide fade (frontend-2 landed).
- `crates/chan-llm/src/session.rs` — `LlmSession::backend()` now reads
  `active_backend()` (gates on `enabled`), partial belt-and-braces for the
  CODEx-on-CLAUDE symptom.
- `crates/chan/src/main.rs` — "assistant config" → "agent config" error
  context strings from [backend-1.md](./backend-1.md).

[frontend-2.md](./frontend-2.md) is REVIEW; verified all the dirty work
matches the "Landed" section there.

### Frontend-1 implementation map

#### Assistant → Agent visible rename

Already mostly landed in the dirty diff. Remaining surfaces to audit:

- Search hits in `web/src` for the strings `"Assistant"`, `"assistant"`
  (capitalized + spoken) in user-visible templates. Be careful to skip:
  - `Role::Assistant` / `"assistant"` JSON role tokens (LLM protocol, keep).
  - JSON keys like `assistant.*` in preferences/config (external schema,
    keep — see [backend-1.md](./backend-1.md) note).
  - `web/src/api/types.ts` field names (paired with backend serde rename
    pass; treat as schema until @@Architect decides on a coordinated
    migration).
- `crates/chan/src/main.rs` `SERVE_LONG_ABOUT` block (line 91 still reads
  "Assistant Cmd+I"). The block is regenerated from
  `web/src/state/shortcuts.ts` via
  `node web/scripts/shortcuts-table.mjs --serve-long-about`; the shortcuts
  label already flipped to "Agent" so a regen + paste is the safe path.

#### Agent banners

`web/src/components/agentBanner.ts` already maps `claude_cli` / `gemini_cli`
/ `codex_cli` to display names and the ANSI Shadow alphabet. Per-agent
banner tint is on `.agent-banner.{claude,gemini,codex,ollama}` in
`InlineAssist.svelte:2791-2810`. To "stop copying Claude's banner", change
each backend's `displayAgentName` text or the rendered ASCII art per
backend; the simplest move is unique strings per backend rather than
distinct ASCII glyphs (current banner is text rendered through one
alphabet, so "CLAUDE" vs "CODEX" already differ visually — the request
likely means the THEME/copy on the empty-state hero is too Claude-coded).
Confirm intent with Alex before redesigning the ASCII art.

#### Banner state-sync (CODEX-on-CLAUDE)

Hand-off from [backend-1.md](./backend-1.md): the banner reads
`configuredAssistantBackend()` in `InlineAssist.svelte:624-630`. Today it
prefers `assistantSelection.backend` (a module-scope $state global) over
the conversation's own `assistant_switch` history.

Specific fix surface:

- `web/src/components/InlineAssist.svelte:624` — change order to "current
  conversation's last `assistant_switch` → assistantSelection → prefs
  default → llmStatus".
- Or derive a Svelte `$derived` near `currentAssistantConversation` in
  `web/src/state/store.svelte.ts:1926` that returns the per-conversation
  backend tag, and consume that in the banner site.

The dirty `chan-llm/src/session.rs` already disables silently-active
configs at the wire (good), so the new derivation can lean on
`/api/llm/status` to know when the saved backend is actually selectable.

#### Status bar event-click → overlay routing

The bar lives in `web/src/components/AppStatusBar.svelte`. Today it shows
three sections: `indexStatus`, `importStatus`, transient `ui.status`. None
of them are clickable.

Hook points:

- Wrap each `<span class="section">` in a `<button>` (keep
  `class:section`) so the click target is keyboard-reachable.
- For `indexVisible`: open the indexer overlay/page (today
  `SettingsPanel` is the closest surface; verify intent — there is no
  dedicated "Index Status overlay" yet, so frontend-1 may need to add
  one or pivot to settings open at the Search Index tab).
- For agent activity (NOT in the bar today per the comment at
  `AppStatusBar.svelte:14-18`): if the request implies pulling agent
  status into the bar, that's net-new wire. The wire is fine —
  `/ws "llm.status"` / `"llm.activity"` carry `session_id` + `backend`
  per [backend-1.md](./backend-1.md) — but the SPA side needs a new
  store and a section.

Ambiguity flagged for @@Architect: request.md mentions "one of the
assistant/agent chats" as a status-bar event example, but AppStatusBar's
own comment says agent activity is *intentionally* not surfaced there to
avoid a third copy of the same signal. Confirm whether (a) only existing
sections need click handlers, or (b) a new agent-activity section is in
scope.

#### URL state reloadable

Already substantially complete in `web/src/state/store.svelte.ts`:

- `HASH_LAYOUT` — pane/tab tree (JSON).
- `HASH_BROWSER` — inspector bit + selected path.
- `HASH_SEARCH` — inspector + scope + query.
- `HASH_GRAPH` — scope|depth|filter-chips|inspector|mode.
- `HASH_ASSIST` — inspector + contextId + prompt buffer.
- `HASH_SETTINGS` / `HASH_SCOPE_HISTORY` — presence flag.

`persistStateToHash()` (line 771) writes, `applyOverlaysFromHash()` (line
678) restores at boot. Gaps to verify:

- `HASH_GRAPH` encodes 5 filter chips; adding new chips for frontend-3
  (folder/symlink/hardlink/contact/media) needs the encoder/decoder
  extended in lockstep — see `encodeGraphFilters/decodeGraphFilters`
  (lines 649-660). Keep the trailing-default trim or the URL will bloat
  every time.
- Settings overlay has no sub-section in the URL; if the index-event
  click is supposed to deep-link to the Search Index tab, add a
  sub-section param.
- The assistant overlay's `assistant_switch` backend is currently not
  in the URL hash. If the fix above moves banner selection to the
  conversation history, that's stored in the conversation blob — fine.
  But a saved URL with `assist=<context>` will still rely on the
  conversation having been persisted server-side first.

#### Layout setting: standard / compact

Backing field: `LineSpacing` enum in `crates/chan-server/src/preferences.rs:135`
(values `Tight`, `Standard`; default `Tight`). Serialized
`lowercase` so the on-disk TOML / API uses `"tight"` and `"standard"`.

Consumers:

- `web/src/api/types.ts:344` `line_spacing: LineSpacing`
- `web/src/components/SettingsPanel.svelte:511-527` radio (currently
  `[tight, standard]`).
- `web/src/editor/Source.svelte:325-327` and
  `web/src/editor/Wysiwyg.svelte:658-659` CSS line-height attached to
  `[data-density="tight"|"standard"]`.

The request wants `[standard, compact]` with `standard` as the default,
and compact's spacing to land **between** the old tight and old
standard. Options:

1. **Rename on-disk** — change `Tight` → `Compact` in the enum (snake_case
   serde → "compact"), update default to `Standard`, and add a
   compatibility shim in deserialize so old TOMLs with `"tight"` decode
   to `Compact`. Touches `crates/chan-server`, `crates/chan/src/main.rs`
   `parse_line_spacing` / `line_spacing_label`, `web/src/api/types.ts`,
   the editor CSS attributes, the settings UI. Coordinated backend +
   frontend change.
2. **Keep on-disk, relabel UI only** — leave the enum as `Tight` /
   `Standard`, switch the default to `Standard`, change the UI label
   from "Tight" to "Compact", and edit the CSS line-heights for the
   `tight` attribute to land between the old 1.5/1.7 values. Pure
   frontend change plus a `default` flip in Rust.

Option 2 is the minimum-blast-radius path and matches the journal's
"preserve compatibility where it's an external schema name" rule.
Calling out for @@Architect because the default flip is still a behavior
change every existing drive will observe on next preferences read.

Specific line-heights for option 2: today Wysiwyg uses `1.5` (tight) and
`1.8` (standard); Source uses `1.4` / `1.7`. The "between" target for
compact would be roughly `1.65` (Wysiwyg) / `1.55` (Source).

#### Dashboard behind tabs

Current empty-pane surface: `web/src/components/Pane.svelte:682-700`. It
renders `.placeholder-mark` (Chan logo) + a hint paragraph + a `<pre>`
with `shortcutTable` (line 76, generated by `renderTable`). The
hamburger menu rows in `emptyPaneContent` / `emptyPaneNavigation` (lines
97-130) already mirror the app's command surface.

For "primary dashboard from now on" the simplest stretch is:

- Replace the prose-only placeholder with a small "What's new since you
  were last here" / "Open recents" / "Continue where you left off"
  panel that reuses existing stores (`tabs.svelte.ts` recents, drive
  info, scope history).
- Don't import heavyweight surfaces (assistant / graph) — the dashboard
  should be one-screen ambient like the current placeholder.

Pin down the dashboard's content scope with Alex before building. The
current Pane.svelte placeholder also gates extra hints behind
`!multiPane` (line 694), so the multi-pane case keeps the bare logo.
Decide whether the new dashboard inherits the same gate.

### Frontend-2 review (REVIEW status)

Confirmed against the dirty diff:

- **Cmd+F Enter cursor placement** — `placeCursor` added to
  `FindAdapter`, wired in `FindBar.goNext/goPrev`. Tests in
  `web/src/editor/find.test.ts` (referenced by frontend-2.md; not in
  this scan).
- **File Browser Cmd+F over visible** — `FileTree` exports
  `setFindQuery/findStep/clearFind`; `FileBrowserOverlay` adds a
  sticky find bar with counter + prev/next/close. Match scope is
  `visibleRows` (only currently-expanded entries — matches the
  acceptance criterion).
- **Tab-complete** — `PathPromptModal` Tab does LCP-extend → fallback
  to cycle; helper extracted to `web/src/state/lcp.ts`.
- **Multi-indent hang-indent** — per-depth `padding-left` + negative
  `text-indent` on `.cm-md-list-line.cm-md-list-depth-{1..6}` in
  Wysiwyg.svelte.
- **List-guide auto-hide** — new view plugin in
  `web/src/editor/extensions/list_guide_visibility.ts`; CSS opacity
  transition gated on `[data-list-guides="off"]`. Test file present.
- **GitHub-style icons** — lucide `ChevronRight/Down` + `Folder/FolderOpen`
  in FileTree.

Deferred bugs (frontend-2.md says "pending browser repro"):

#### Cursor height inherited from image on previous line

Hypothesis worth probing during browser repro:

- The image widget wraps in `display: inline-block; line-height: 0`
  (Wysiwyg.svelte:1019-1024). A line that contains an image is therefore
  as tall as the image. CM6 derives `.cm-cursor` height from the line's
  rendered box.
- If the screenshot shows the cursor on a SOURCE line that follows an
  image-containing line, the culprit is more likely the list-guide
  `::before` rule (`top: 0; bottom: 0`) extending the visual ::before
  bar down through the next list line's box.
- Worth testing both: cursor on the SAME source line as the image (end-
  of-line after the image), versus the NEXT source line. Different
  fixes.

#### Stale blue selection rectangles around image/list

`web/src/editor/widgets/image.ts:392-421` already installs document-level
mousedown + keydown listeners to clear `data-selected` on the image
wrap. But:

- The clear path only runs on `mousedown` (outside the wrap) or on a
  recognized key inside the listener. A simple arrow-key caret move
  triggers neither — the image wrap retains `data-selected` and its
  outline ring (Wysiwyg.svelte:1030-1034) stays visible.
- The "blue rectangles spanning image/list rows" pattern also matches
  the BROWSER-native text selection: the Wysiwyg editor does NOT
  install CM6's `drawSelection()` extension (verified — grep returns
  no hits). So selection paints by the browser. When the selection
  crosses an image widget, the browser's per-fragment selection
  rectangles render around the image — and they only clear when the
  selection actually changes. If the user clicks the find input or
  another non-CM target, the document selection doesn't move and the
  rectangles persist visually.
- Recommendation for @@Frontend: in the image widget's outside-
  mousedown listener, also bind to CM6 transactions that change
  selection (`EditorView.updateListener` with `update.selectionSet`)
  and call `clearImageSelection` whenever the editor selection moves.
  That covers arrow-key caret moves and Find-bar Enter cursor placement.
- For the browser-native selection ghost: a possible workaround is to
  call `view.contentDOM.blur()` and re-focus when the FindBar takes
  focus, but a less-invasive fix is to verify the selection paints by
  injecting CM6's `drawSelection()` and using the synthetic layer
  instead. Confirm with Alex's screenshot which symptom dominates
  before swinging at the layer.

#### Image-line guide bars break around images

Likely shares root cause with the cursor-height symptom:
`.cm-md-list-line::before` uses `top: 0; bottom: 0` so the vertical
guide spans the line's full height (image-tall). When the visual row
is image-height tall, the bar reads as a chunky rectangle next to the
image. Possible fixes:

- Constrain the `::before` to `top: 0.2em; bottom: 0.2em` or use a
  fixed pixel height so it remains a thin bar.
- Or move the `::before` to a `position: sticky` widget that tracks
  the prose baseline rather than the line box. More work.
- Same `data-list-guides="off"` fade kicks in 1.5s after caret leaves,
  so the chunkiness is only visible while the caret is near.

### Frontend-3 implementation map (IN_PROGRESS)

#### Centralized color tokens

`web/src/state/kinds.ts::colorVarFor` already routes:

- `document` / `text` → `--g-doc`
- `contact` / `mention` → `--warn-text`
- `media` → `--g-img`
- `tag` → `--g-tag`
- `binary` / `date` → `--text-secondary`
- `folder` → `--accent`

Request colors:

- Markdown → orange. ✓ `--g-doc: #ff8a3d` (dark) / `#c25a1f` (light).
- Contact md frontmatter + `@@mention` → yellow. ✓ `--warn-text: #e3b341` (dark).
- Media → purple. ✓ `--g-img: #b07dff` (dark) / `#7a4cd8` (light).
- Binary → blue matching FILE blue. ✗ Currently `--text-secondary` (grey).
  Need a `--g-binary` token (suggest `#58a6ff` to match `--link`'s blue
  on dark / `#0969da` on light).
- Tag → green. ✓ `--g-tag: #6cd07a` (dark) / `#2f9444` (light).
- Folder → grey. ✗ Currently `--accent` (green). Re-route to
  `--text-secondary` or introduce `--g-folder`.

Suggested edits:

1. Add `--g-binary` (blue) and `--g-folder` (grey) to both `:global(:root)`
   and `[data-theme="light"]` blocks in `App.svelte:520-660`.
2. Update `colorVarFor` to return `--g-binary` / `--g-folder` for
   `binary` / `folder`.
3. Sweep consumers (search hits, inspector pills) to confirm none
   reference `--accent` for folder rows or `--text-secondary` for
   binary rows directly.

#### Graph filter chips

Current chips: `link, tag, mention, language, img` (see
`store.svelte.ts:2428-2434` `GraphFilters` type). URL hash encoding is
positional 5-bit string (`encodeGraphFilters`/`decodeGraphFilters`).

Request wants the cross-mode filter set: `language, folder, symlink,
hardlink, link, tag, contact, media`. Implementation notes:

- `media` ≅ existing `img` chip (rename or alias). `language` chip
  exists. So new chips are `folder, symlink, hardlink, contact`.
- `contact` is a node-kind filter (file nodes flagged `node_kind:
  "contact"` from the wire; classification already in GraphPanel
  `classifyFile`).
- `folder` is mode-dependent: in semantic/markdown mode, folder nodes
  would be SYNTHETIC (derived from each file's path); in filesystem
  mode, folder nodes are real (from /api/fs-graph). The chip toggle
  needs to act on the synthetic set in semantic mode and on the wire
  set in fs mode.
- `symlink` / `hardlink` come from /api/fs-graph (`kind: "symlink"`).
  Hardlinks aren't exposed by chan-drive today; treat the chip as
  disabled-with-explanation until backend grows the distinction OR
  document that hardlinks render as normal file nodes.
- URL hash encoder needs to grow to N bits. Bump the encoder to a
  fixed-position N-bit string keyed off `Object.keys(filters)` order;
  trim trailing default `1`s the same way.

Bigger question for @@Architect: the request says "consider whether
this means basically having all filters for all graph modes". That's a
lot of chrome on the graph bar. An alternative: a single "Filters" menu
button (already used for the hamburger in GraphPanel) that lists every
applicable chip per-mode. Avoids a 10-chip toolbar.

#### Markdown graph parent-folder / path-to-root overlay

Confirmed by [backend-2.md](./backend-2.md): no backend change needed.
The implementation:

1. Synthesize `kind: "folder"` nodes from each file node's `path`
   field; emit containment edges from each file to its immediate
   parent folder, and chain folder → folder up to the drive root.
2. Gate the synthesis behind a filter chip (proposed `folder`).
3. Use the same `folder:<path>` id format as the language graph
   (`graph.rs:351-353` server-side, mirrored on the wire) so id
   collision with file-path ids is impossible.

#### Folder graph: markdown cross-link overlay

Fetch `/api/graph` alongside `/api/fs-graph` (the GraphPanel already
calls both when switching modes); intersect on the `path` field. Render
the link edges as a second edge family with a distinct stroke color
(maybe `--g-doc` low-alpha) so it reads as "markdown overlay" against
the folder mode's primary edges.

Edge case from [backend-2.md](./backend-2.md): `/api/graph` treats in-
drive symlinks as missing while `/api/fs-graph` exposes them as
`kind: "symlink"`. When overlaying, a symlinked `.md` file on the
folder canvas will have no incoming link edges from the markdown graph.
Document with a tooltip on the symlink node.

#### Scope options: parent folder + common ancestor

Frontend-only operation in `availableGraphScopes()`
(`store.svelte.ts:2273-...`). Implementation:

1. For each visible `.md` file scope, derive the parent path
   (`path.lastIndexOf('/')`) and call `addDirScope(parent, "parent")`.
2. When multiple `.md` files are in scope (pane structure has multi-
   file split), compute the longest-common-prefix at `/` boundaries
   and call `addDirScope(lcp, "common ancestor")`.
3. Reuse the existing `dir:<path>` scope id format so the URL hash and
   the SCOPE picker stay in lockstep without backend coordination.

The new helper from frontend-2's `lcp.ts` covers the LCP work; reuse it
here.

### Test / browser-smoke suggestions for @@Webtest

Once @@Frontend lands more slices, suggested smoke cases per area:

**Agent labels (frontend-1)**: open every overlay (Settings, Files,
Search, Graph, Assistant), inspect labels for any remaining "Assistant"
strings. Particular spots: tab strip pane menu (Pane.svelte already
flipped to "Call Agent"), AccessoryPill tooltip, ScopeHistoryOverlay
turn peek, AssistantInspectorBody headers.

**Banner per-agent (frontend-1)**: switch active provider in
AssistantInspectorBody → empty-state hero should swap ASCII (CLAUDE CLI
/ GEMINI CLI / CODEX CLI) with the matching tint class. Verify on a
freshly-opened drive conversation AND on a resumed file conversation
whose last `assistant_switch` is a different backend than the global
selection (the CODEx-on-CLAUDE repro).

**URL state (frontend-1)**: open each overlay, change knobs (search
query, graph filters, assistant context), copy the URL, paste in a new
tab → restore should reproduce the exact state. Especially:
graph mode flip, browser inspector bit, assistant prompt buffer with
a `|` character (the hash encoder uses `|` as a separator).

**Status bar click (frontend-1)**: with the indexer building (boot a
fresh drive against `/tmp/chan-phase3-drive` to force a reindex),
click the index section → expect the index status overlay/page to open.
Click the import section while a contacts import is running.

**Layout preset (frontend-1)**: flip to Compact / Standard, confirm
prose density actually differs in Wysiwyg + Source. Confirm new default
is Standard on a fresh launch (delete preferences.toml between runs).

**Dashboard (frontend-1)**: close every tab, view the background; verify
it surfaces something more than the logo + shortcuts table.

**frontend-2 (already REVIEW)**:

- Cmd+F Enter — type "hello", Enter, Esc → caret on the H of the
  current match; Shift+Enter wraps; mixed Wysiwyg vs Source mode.
- File Browser Cmd+F — overlay open, Cmd+F → find bar shows; expand
  one folder, type a filename → match scrolls into view; type a name
  inside a collapsed folder → no matches.
- Tab-complete — `notes/` typed in PathPromptModal Tab → completes to
  the LCP across shown suggestions; second Tab cycles.
- Multi-level indent — open `projects/phase3/research/notes-a.md`,
  create a nested list at depth ≥ 2, write a long sentence that wraps
  → the wrapped line hangs under the parent text (no de-indent to the
  gutter).
- List guide fade — caret on a list line → bars visible; click off
  to a non-list paragraph → bars fade ~1.5s later.
- Icons — file tree shows lucide chevrons + folder glyphs (compare to
  GitHub screenshot in request.md).

**frontend-2 deferred bugs (need browser repro before fixing)**:

Build a fixture markdown file in the test drive:

```md
- list item 1 with image ![](media/sample.png)
- Let's switch to a normal line
```

Then:

1. Open in Wysiwyg.
2. Caret on the "Let's switch" line. Screenshot the `.cm-cursor` height
   vs `.cm-md-image-wrap` rect — confirm whether the caret is image-
   height tall.
3. Click the image (data-selected ring appears).
4. Arrow-down to text line — confirm the ring persists (validates the
   data-selected-not-cleared hypothesis).
5. Click + drag a selection across the image — observe the blue
   rectangles. Click in the FindBar (without moving caret) — confirm
   the rectangles persist (browser-native selection bug).

**frontend-3 (graph + colors)**:

- Folder rows in FileTree paint grey (after color token fix). Binary
  rows in FileTree paint blue.
- Same per-kind colors in inspector kind chips (chip uses
  `colorVarFor`).
- Open Graph overlay → markdown mode → toggle the new `folder` chip;
  folder nodes appear / disappear without breaking existing edges.
- Switch to folder graph mode → toggle the new `link` overlay chip;
  markdown link edges paint as a secondary overlay.
- Set scope to a single `.md` file (open it in a tab, focus the
  overlay) → SCOPE picker lists `dir:<parent>` alongside `file:<path>`.
- Split panes with two `.md` files in different folders → SCOPE picker
  lists the common-ancestor folder.

### Risk / collision notes

- @@Frontend's frontend-2 commit is already coherent and ready for
  @@Webtest review. Avoid re-touching FindBar.svelte, FileTree.svelte,
  FileBrowserOverlay.svelte, PathPromptModal.svelte, or
  list_guide_visibility.ts in the frontend-1 work — let frontend-2 land
  first to keep the diff readable.
- The `LineSpacing` rename (option 1) crosses backend; route through
  @@Architect before either side commits to it.
- The `--g-binary` / `--g-folder` palette additions need both dark and
  light variants. Easy to miss the light block at
  `App.svelte:608-651`.
- Banner CODEx-on-CLAUDE fix in `InlineAssist.svelte:624` interacts
  with the in-progress `LlmSession::backend() -> active_backend()`
  change. Verify that after the rust-side disable-then-active fix,
  the frontend banner still has a backend to render (it should — the
  conversation's last `assistant_switch` is independent of the live
  config).
- frontend-3's filter chip URL encoder change is a breaking URL
  format: old saved `#graph=...` URLs with the 5-char chip string
  decode wrong against the new N-char encoder. Either keep
  positional with the new bits appended (old URLs decode as
  "new chips default to on/off as appropriate") or add a length
  check and treat short encodings as legacy.

### Unclear / needs Alex

- Status-bar agent-activity click: AppStatusBar today does NOT show
  agent activity (intentional per the source comment). Does the
  request want a new section, or only click-handlers on existing
  sections?
- Layout preset rename: rename on-disk (option 1) or keep schema and
  relabel (option 2)? Default change observable on existing drives
  either way.
- Dashboard scope: what goes in it? Recents only? Pending agent
  turns? Per-drive welcome? Marketing surface is explicitly off the
  table per the request.

## Commit readiness notes

Documentation/support task only. No source files modified. Findings are
ready for @@Frontend, @@Webtest, and @@Architect to consume.
