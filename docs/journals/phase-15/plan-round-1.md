# Phase 15 Round 1 — technical decomposition

Root causes, file:line references, and design decisions behind the lane
tasks. Lane task files are the actionable checklists; this is the reference.

## Phase-14 carryover (reviewed, not blocking)

All phase-14 lane work merged. Open items deferred by @@Alex, none gating
this round: A3 desktop default-workspace relocation + embedding-model-prompt
policy; A5d WKWebView manual walk + "blank white window" UX gap; identity
charset mismatch (`Workspaces.svelte` advertises `._-` vs
`--tunnel-workspace-name` `[a-z0-9-]`); B1b depth-slider frontier-only
reoptimization; `.txt`-vs-`.md` graph tone split. Appended to round-2
backlog.

## BUG-1 — Cmd+, flip: true two-face card flip (Lane A)

Current mechanism: per-pane `showingBack` toggles an `{#if}` that *swaps*
content (`Pane.svelte:1247-1268`); a `paneFlip` version bus
(`tabs.svelte.ts:764-770`) + an rAF "double-tap" on a `flipActive` class
(`Pane.svelte:405-420`) drives a single-face half-flip keyframe
(`Pane.svelte:1445-1457`). The bug: the rAF/effect toggle races the
content-swap teardown + focus blur, so the keyframe only fires once focus
leaves the pane.

Fix: render front and back **simultaneously** in a `.flip-card`
(`transform-style: preserve-3d`); front `rotateY(0)`, back `rotateY(180deg)`,
both `backface-visibility: hidden`, absolutely stacked. The card rotates
`0deg <-> 180deg` via a CSS **transition on transform** driven directly by
`pane.showingBack`. No keyframe, no rAF, no `paneFlip` bus (remove
`requestPaneFlip` + `flipActive`; `flipHybrid` just toggles `showingBack`).
The front face stays mounted but `pointer-events:none` + `aria-hidden` while
flipped; preserve the `f6684aba` focus-follows-active-pane behavior. Back is
always mounted while the tab exists — keep config bodies cheap and lazy-init
polling.

## BUG-2 — Dashboard Search inspector buttons (Lane A)

Carousel Search-slot directory inspector callsite
(`EmptyPaneCarousel.svelte:582-592`) passes no callbacks, so
`FileInfoBody.actionsSection` (`FileInfoBody.svelte:657-752`) shows only
Upload + Download. Wanted: drop Upload here, add **Show Directory**
(`onReveal`, already gated at `:726-729`) and **New Terminal** ($cwd = sel).
Pass `onReveal` + a new-terminal callback + an Upload-suppress flag through
`InspectorBody.svelte:92-108`. Reuse `openInActivePane` /
`openTerminalInActivePane`.

## BUG-3 — Shift+Enter with an agent running (Lane C)

`keymap.ts:48-61`: commit `eb1ae07b` gated the modified-Enter sequence on the
SPA having observed the agent's keyboard-protocol negotiation
(`xtermModifyOtherKeys` or kitty `KITTY_REPORT_ALL_KEYS`). `start()` resets
that state on every (re)connect (`TerminalTab.svelte:579`), but a long-lived
agent won't re-emit its negotiation after a reconnect -> state is stale-zero
-> `terminalMetaKeyBytes` returns null -> Shift+Enter falls through to plain
`\r` (submits instead of newline). Candidate fixes (decide empirically):
(a) don't reset protocol state on reconnect to a surviving session;
(b) re-query negotiation on reconnect; (c) sane default sequence when Shift
is held and state is unknown. Validate with a real agent in the terminal.

## Search cleanup (Lane B)

Remove SCOPE `<select>` (`SearchPanel.svelte:726-739`) + the client-side
scope filtering it drives (`pathInScope`, scope `$effect`s,
`scopeId`/`availableSearchScopes` in `store.svelte.ts:1476-1494`,
`scope.svelte.ts`); search is workspace-wide (server `searchContent` already
ignores scope). Resolve `openSearchForFile`/`openSearchForDirectory`
(`store.svelte.ts:1530-1538`). Remove SEARCH STATUS button
(`SearchPanel.svelte:741-749`). Delete `SearchStatusOverlay.svelte` (gated on
`CK-INDEX`).

## Dashboard redesign (Lane A)

Carousel at `EmptyPaneCarousel.svelte` (slides `:411-610`, controls
`:613-661`, cycling `:305-347`). Per-slot front+back; slot picker on both
faces; back force-paused. Dashboard back becomes per-slot (replaces the
monolithic `HybridDashboardConfig` arm at `Pane.svelte:1261-1268`).
- **About** back: Appearance + Screen lock (from `HybridDashboardConfig`,
  theme dropdown relabel `Plain`->`Default`, wire value stays `"plain"`,
  `:388-398`; info `s/yet/set`) + new screensaver **Preview** widget.
- **Workspace** back: chan-reports (from
  `HybridFileBrowserConfig.svelte:324-356`). Proposed home for the
  Metadata-archive section (`HybridDashboardConfig.svelte:428-506`), which
  the new spec omits — confirm with @@Alex.
- **Search** front: legend conditional (Indexed always; Indexing/Pending only
  when present, `:594-607`). Back: Index widget copied from
  `SearchStatusOverlay.svelte:149-178` + Semantic search + Embedding model
  from `HybridFileBrowserConfig.svelte:235-322`.
- `HybridFileBrowserConfig` left with a placeholder ("No settings here,
  cheers.").
- **Right-click menu** on the Dashboard tab title (infra wired at
  `Pane.svelte:1004-1016`; pattern in `TerminalTab.svelte`): vertical slot
  list with on/off checkboxes (>=1 enforced, default all-checked, **per-tab**
  state on the `DashboardTab` node, serialized in the session hash beside
  `carouselSlide`), separator, "Settings (Cmd+,)". Unchecked slots skipped by
  the auto-advance `$effect` (`:322-329`).
- Screensaver preview: `MatrixRain.svelte` is hardcoded to
  `window.innerWidth/Height` with no props; add `width`/`height` props (or a
  `MatrixRainPreview.svelte`) reusing `drawStaticMatrix()`
  (`MatrixRain.svelte:211`). Plain theme preview is a CSS div.

## Terminal groups + broadcast (Lane C)

**Design decision — a group is a plain string label, not an allocated
resource.** A `group: string` field lives on each `TerminalTab` (SPA state),
defaulting to `"default"`. A group "exists" iff >=1 terminal references that
string. Closing the last terminal in a group leaves nothing referencing it
(implicitly destroyed); opening one with that string references it again
(implicitly created) — with **zero lifecycle code**. No pre-allocated pool,
no fixed cap, no name<->slot re-association, no cleanup. `"default"` is not
special in code; it is just the default value, so it is always present
because new terminals adopt it. A single-member group simply has no
broadcast targets (no-op). This is the simplest correct model and gives
exactly the "destroyed when last exits, re-created when first joins"
behavior for free. (Rejected: a 20-slot pre-created pool — arbitrary cap,
allocation/cleanup complexity, stale-association bugs, no benefit.)

- "Group" field in the terminal context menu below "Name"
  (`TerminalTab.svelte:1352-1369`); default `"default"`; change requires
  restart (mirror stale-env prompt `:1382-1388`).
- Carry group on the WS query (`session.ts:13-35`) + restart body; inject
  `$CHAN_TAB_GROUP` in `terminal_sessions.rs` beside `CHAN_TAB_NAME`
  (`:502-505`) and add it to the clear-list (`:983-996`); thread through
  `TerminalQuery`/`CreateOptions` (`routes/terminal.rs:34,179`). The server
  also stores `tab_group` (+ `tab_name`) per live session in the
  `TerminalRegistry`, so `cs term list` / `term write` can resolve groups
  server-side. The SPA keeps its own `group` per tab for the Cmd+Shift+I
  client broadcast; both are set at spawn from the same value (group change
  requires restart) so they never diverge.
- Group-scoped broadcast: filter `terminalBroadcastMemberIds` /
  `broadcastTerminalInput` (`tabs.svelte.ts:1245-1420`) to terminals whose
  `group` matches the source's. Validate with 4 terminals (2 `default`,
  2 `foobar`).

## chan shell / cs (Lane C)

Existing proven push path: `cs -> $CHAN_CONTROL_SOCKET (UDS) ->
control_socket.rs -> events_tx.send(JSON) -> /ws -> store.svelte.ts
handleWindowCommand`. `chan open` (`main.rs:1726-1791`) already traverses it.

**Subcommand surface** (`term` is a parent with `new`/`write`/`list`):
- `open [path]`, `graph [path]`, `dashboard [--carrousel-on] [--carrousel-index]`
- `term new [path] [--tab-name] [--tab-group]` (was `term`)
- `term write [cmd] [--stdin] [--tab-name] [--tab-group]` (was `term-write`)
- `term list` -> JSON of live terminal sessions grouped by group

**Two clean categories** (the `term list` JSON return forces this split):
1. **Open a UI tab** (`open`, `graph`, `term new`, `dashboard`): only the SPA
   can create tabs, so these push a `window_command` on `events_tx` and the
   SPA acts — the proven `open` pattern. No registry needed. Frame carries
   the originating `$CHAN_WINDOW_ID`; only that window's SPA acts. Control
   socket returns "sent" immediately (best-effort, no end-to-end ACK).
2. **Act on / inspect live sessions** (`term write`, `term list`): these
   operate on running PTYs, which the **server** owns. The control socket
   gets a read handle to the `TerminalRegistry`; the registry stores
   `tab_name` + `tab_group` per live session (added by C-term). `term write`
   finds sessions by `--tab-name` (single; writes to all sessions with that
   name) or `--tab-group` (broadcast) and calls `session.send_input()`
   (`terminal_sessions.rs:685`) directly — the natural PTY-stdin path,
   independent of SPA state. `term list` returns the registry grouped by
   group, e.g. `{"groups":{"default":[{"name","session_id","cwd"}],...}}`.
   `--stdin` streams chunks to the control socket, each written through as it
   arrives.

The registry handle is plumbed where the control socket is constructed (the
AppState wiring, today carries `workspace_cell` + `events_tx`); C-shell adds
the handle there and reads it. C-term owns the session-struct/registry edits
in `terminal_sessions.rs`. Coordinate at `CK-GROUP`.

- **CLI:** `Command::Shell { action }` + `cmd_shell` in `main.rs` (pattern:
  existing `cmd_*`); `term` is a nested subcommand group (new/write/list);
  `argv[0]=="cs"` dispatch rewrite at top of `main()`; integration test for
  the symlink trigger (tempdir `cs -> chan`; do not create the symlink in the
  build). Error clearly when `$CHAN_CONTROL_SOCKET` / `$CHAN_WINDOW_ID` absent
  (reuse `open_env_from()`, `main.rs:1746`).
- **Control socket:** extend `control_socket.rs` (today `OpenPath` /
  `OpenFile`+`OpenBrowser`, `:26-53,208-270`). `open` reuses `OpenPath`
  wholesale. Add `OpenGraph` / `OpenTermNew` / `OpenDashboard` (category 1,
  push window_command) and `TermWrite` / `TermList` (category 2, registry).
  `TermList` returns its JSON on the same control-socket connection.
- **SPA:** extend `handleWindowCommand` (`store.svelte.ts:670-691`) +
  `WindowCommandFrame` with arms for `open_graph` / `open_term_new` /
  `open_dashboard` (set `carouselSlide`), calling existing
  `openGraphInActivePane` / `openTerminalInActivePane` /
  `openDashboardInActivePane`. (No SPA arm for `term write`/`term list` — both
  are server-side.) The SPA-side `group` on each `TerminalTab` is still kept
  for the Cmd+Shift+I client broadcast UX; it and the server's per-session
  `tab_group` are both set at spawn from the same value (group change
  requires restart), so they stay consistent.

## Release v0.20.0

Unified version bump to 0.20.0 across all pins (root `[workspace.package]`,
gateway workspace, `web/package.json`, `tauri.conf.json`, `Cargo.lock`) ->
full gate incl. gateway + `--no-default-features` -> dry-run `release.yml`
(`workflow_dispatch publish=false`) -> tag `v0.20.0` (fires `release.yml`;
self-upgrade is data-driven from `/dl latest.json`). Only on @@Alex's go.
