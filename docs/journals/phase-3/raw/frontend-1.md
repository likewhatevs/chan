# frontend-1: Agent surfaces, URL state, status routing, dashboard shell

Owner: @@Frontend.

Status: REVIEW (banner state-sync fix, status-bar click routing,
SERVE_LONG_ABOUT regen, Agent Cmd+F, dashboard, URL state, and
visible Agent rename all landed; only Settings layout
standard/compact remains, gated on [backend-3.md](./backend-3.md)
LineSpacing enum rename).

Related:

- [request.md](./request.md)
- [journal.md](./journal.md)
- [backend-1.md](./backend-1.md)
- [webtest-1.md](./webtest-1.md)

## Goal

Implement the main visible application shell changes:

- Rename Assistant to Agent across visible UI surfaces.
- Show actual banners for supported agents instead of copied Claude banners.
- Clicking an event in the status bar opens the relevant overlay.
- Active screen/overlay/resource state is reflected in the URL and reloadable.
- Agent overlay Cmd+F searches the chat history of the current session.
- Settings layout choices become standard/compact, with standard as default.
- Empty tab/background window becomes the primary dashboard.

## Acceptance criteria

- No user-visible "Assistant" remains unless @@Architect records a compatibility
  exception in [journal.md](./journal.md).
- Agent banners/labels reflect the selected supported agent.
- Status bar event clicks route to agent chat, index status, or other relevant
  overlay for known event types.
- Reloading a URL restores the same primary screen/overlay and selected
  resource where practical.
- Cmd+F inside the Agent overlay opens an overlay-scoped find control for the
  current conversation/session, supports next/previous navigation, and does not
  hijack document or browser find outside that overlay.
- Layout setting labels and defaults match [request.md](./request.md).
- Dashboard replaces the current shortcuts/logo-only background without becoming
  a marketing page.

## Test expectations

- Run `cd web && npm run check`.
- Add focused component/unit tests where current test harness supports the
  changed state/routing behavior.
- Coordinate browser smoke cases with [webtest-1.md](./webtest-1.md).

## Review expectations

- @@Backend confirmation for status event and agent metadata contracts.
- @@Webtest browser validation on desktop and narrow viewport.

## Progress notes

- 2026-05-16 @@Architect: Alex added a requirement that Cmd+F in the Agent
  overlay searches the current session's chat history. Treat this as
  overlay-scoped find behavior parallel to File Browser find in
  [frontend-2.md](./frontend-2.md), but scoped to the active Agent conversation.
- 2026-05-16 @@Frontend: started.

Landed (frontend-only portions):

- **Visible Assistant → Agent rename**. Renamed every user-visible
  rendered string ("Assistant" → "Agent") across:
  `SettingsPanel.svelte` (section header + "Assistant CLI"
  label), `Pane.svelte` ("Call Assistant" empty-pane row +
  "assistant working" tooltip/aria + dashboard hint copy),
  `FileEditorTab.svelte` ("Call Assistant" button), `AccessoryPill.svelte`
  (title/aria/disabled hint), `InlineAssist.svelte` (Inspector
  title), `AssistantInspectorBody.svelte` (placeholder copy +
  "Active assistant" field label), `ScopeHistoryOverlay.svelte`
  ("who" labels for assistant + assistant_switch turns),
  `state/shortcuts.ts` (shortcut label). Schema fields, CSS
  variables (`--assistant-*`), internal identifiers, file/dir
  names, and module/file naming intentionally NOT renamed —
  staged per [journal.md](./journal.md) compatibility note.
- **URL state restoration: search scope** — `searchPanel.scopeId`
  now round-trips through a sibling `search_scope=` hash param.
  Encoder omits the field at the drive default to keep common
  URLs short; decoder seeds the field only when present, so
  legacy URLs without the key restore at the drive root as
  before. Lives in a separate param (not folded into the
  existing `search=` value) because user queries can contain
  any character; mixing scope and query in one value risks
  collisions with `|` / `:` / `,` separators that legacy queries
  might already have. See `state/store.svelte.ts`, `App.svelte`.
- **Dashboard shell** — empty-pane background now reads as the
  primary dashboard rather than a placeholder. Adds a compact
  drive header (drive name + file/folder/contact counts +
  current index activity) above the existing keyboard-shortcut
  table, only on the lone-pane case (multi-pane background
  stays bare so empty panes don't grow noise). Wording stays
  factual per [journal.md](./journal.md) — no marketing copy.
  See `Pane.svelte`.

Also landed (initial pass):

- **Agent overlay Cmd+F over current session's chat history** —
  Cmd+F (Ctrl+F on non-Mac) opens an in-overlay find bar pinned
  between the header and the chat scroll area. The query walks
  the rendered `.bubble` DOM nodes' `textContent` for substring
  matches (case-insensitive), highlights matched bubbles with
  the same `--warn-text` family used by the File Browser
  `find-match` rows, and steps with Enter / Shift+Enter. Esc
  closes and clears highlights. Scoped to the active overlay
  via a keydown handler on `.assistant-shell`; bubbles up from
  the prompt's CodeMirror because CM6 doesn't claim Cmd+F.
  See `InlineAssist.svelte`. No backend dependency — the chat
  history is already a client-side derivation of `conv.turns`.

Architect reconciliation (acted on):

- [backend-1.md](./backend-1.md) confirmed agent banner metadata
  and status-routing fields already exist. The two "blocked"
  items in my earlier note were actually frontend-only fixes;
  unblocked + landed below.
- Settings layout standard/compact does need backend config/CLI
  support; tracked in [backend-3.md](./backend-3.md). Still
  the only remaining frontend-1 dependency.

Unblocked + landed after architect reconciliation:

- **CODEx-on-CLAUDE banner state-sync fix** — root cause per
  [backend-1.md](./backend-1.md): `configuredAssistantBackend()`
  in `InlineAssist.svelte` preferred the global
  `assistantSelection.backend` over the active conversation's
  own `assistant_switch` history, so a Claude conversation
  rendered the CODEX banner whenever the selector last picked
  CODEX. Added a `conversationBackend()` helper that walks the
  current conversation's `turns` from newest to oldest for the
  most-recent `assistant_switch` and uses that as the first
  resolution step; falls back to selector → drive default →
  `llmStatus.backend` in order. `configuredAssistantModel()`
  also tightened: the selector's model is only adopted when
  its backend agrees with the resolved banner backend, so
  reopening a Claude thread no longer shows a Codex model id
  in the empty-state hero. See `InlineAssist.svelte`
  `conversationBackend()` / `configuredAssistantBackend()`
  / `configuredAssistantModel()`.
- **Status-bar event click → overlay routing** — added click
  handlers on each `AppStatusBar` section per
  [backend-1.md](./backend-1.md) ("no new backend route or
  shape change"). Index section opens Settings (closest "index
  status page" we have; the journal "narrow rather than new
  surfaces" note vetoes a dedicated overlay). Import section
  opens the File Browser overlay (where the contacts importer
  lives). Transient `ui.status` section clears on click (these
  are error crumbs like "rename failed: …"). Agent activity
  is intentionally NOT in this bar (confirmed by the source
  comment at the top of `AppStatusBar.svelte` and
  [frontend-b-1.md](./frontend-b-1.md)'s read-only review), so
  no agent-routing case. Sections now render as `<button>`
  elements so they're keyboard-reachable; visual style is
  preserved via `.section.btn { background: transparent;
  border: 0; padding: 0; }`.
- **SERVE_LONG_ABOUT regen** — per
  [backend-1.md](./backend-1.md) "once @@Frontend renames the
  shortcuts.ts label to Agent under frontend-1, run
  `node web/scripts/shortcuts-table.mjs --serve-long-about`
  and paste between the BEGIN/END markers": ran the regen and
  swapped "Assistant" → "Agent" in
  `crates/chan/src/main.rs::SERVE_LONG_ABOUT`. `cargo check -p
  chan` passes. This is the only Rust change in this commit
  unit and should ride with the frontend shortcuts.ts rename
  so the `chan serve --help` output stays in lockstep with
  the in-app keybindings table.

Pulled forward from [frontend-2.md](./frontend-2.md) deferred
cluster (stale selection rectangles around image/list blocks):

- **CodeMirror `drawSelection()` extension added to Wysiwyg** —
  per [frontend-b-1.md](./frontend-b-1.md) analysis: browser-
  native text selection rectangles render per-fragment around
  image widgets and don't clear when the caret moves to a
  non-CM target (e.g. focusing the FindBar input). CM6's
  `drawSelection()` replaces the browser layer with a
  synthetic selection that tracks the editor's selection
  state directly. Imported in `Wysiwyg.svelte` and added
  near the top of the extensions list (before decorations).
  The image widget's existing `imageCaretRedirect()` already
  clears the `data-selected` ring on selection change, so
  arrow-key caret moves off an image now both: clear the ring
  (existing) AND clear the blue selection rectangles (new).
  Still recommend @@Webtest verifies with the
  [frontend-b-1.md](./frontend-b-1.md) repro fixture
  (`- item with image ![](media/sample.png)\n- Let's switch`).

Backend-3 landed; frontend wiring remains:

- **Settings Layout `[tight] [standard]` → `[standard] [compact]`** —
  backend support landed in [backend-3.md](./backend-3.md). Frontend side is
  tracked in [syseng-frontend-4.md](./syseng-frontend-4.md): update radio labels in
  `SettingsPanel.svelte:511-528`, type in
  `web/src/api/types.ts:327`, CSS density values in
  `Wysiwyg.svelte:656-657` + `Source.svelte:326-327` to land
  compact between today's tight (1.5/1.4) and standard
  (1.8/1.7) per request.md "between the two".

## Commit readiness notes

Two suggested commit units, both ready for @@Webtest:

1. The frontend-only landed items as one
   `chan-web: agent rename + URL state + dashboard + status-bar routing + banner sync + Cmd+F` commit.
2. The `crates/chan/src/main.rs` SERVE_LONG_ABOUT keybinding
   rename rides with #1 since it's the docstring counterpart
   of the same shortcuts.ts rename. (One coherent unit per
   [backend-1.md](./backend-1.md) section 4.)
3. Settings layout standard/compact frontend wiring is now
   [syseng-frontend-4.md](./syseng-frontend-4.md).
