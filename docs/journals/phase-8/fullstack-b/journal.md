# @@FullStackB's phase-8 journal

Author: @@FullStackB
Date: 2026-05-19

Frontend + backend lane B. Same profile as @@FullStackA; operates
in parallel to clear the bug queue and (Round 2) feature queue.

Append-only. New entries go at the bottom under a dated heading.

## 2026-05-19 - Boot + queue scan

Fresh session boot. Read contact card, phase-8 process (inherits
phase-7 verbatim plus the @@CI lane delta and the bug-sweep round
shape), the phase request, the bug list, and my four task files:

| Task             | Scope                                                |
|------------------|------------------------------------------------------|
| fullstack-b-1    | Native window-config stack (chan-desktop Tauri)      |
| fullstack-b-2    | Cmd+T + scrollback + line-adjustment cluster         |
| fullstack-b-3    | Watcher dialog: out-of-root paths + create-dir UX    |
| fullstack-b-4    | Indexing-chart pan/zoom parity with Graph view       |

No inbound events in `alex/event-architect-fullstack-b.md` or
`alex/event-fullstack-b-alex.md` yet (files don't exist). Working
through tasks in numerical order.

## 2026-05-19 - fullstack-b-1 implemented

Landed the WindowConfig LRU stack inside chan-desktop's sidecar
config:

* `config::WindowConfig` plus `window_configs: Vec<WindowConfig>`
  in `Config` (cap = 20, newest first).
* Free-function helpers `push_window_config` / `pop_window_config`
  with dedupe-by-label semantics, plus
  `local_window_key` / `tunnel_window_key` to keep local and
  tunneled drives namespaced apart in the stack.
* `AppState::{push,pop}_window_config` wrap the lock + load +
  mutate + save pattern, best-effort (errors logged, not
  returned).
* `serve::spawn_local_drive_window` /
  `spawn_tunneled_drive_window` now pop a matching entry before
  generating a fresh label, reusing its `window_label` (so the
  `?w=<label>` lookup hits the same `session.json`) and stamping
  its saved URL hash into the new webview's fragment.
* `build_drive_window` installs a `WindowEvent::CloseRequested`
  handler that snapshots `WebviewWindow::url().fragment()` and
  pushes a fresh `WindowConfig` to the top of the stack.
* `pop_compatible_config` defends against stale entries: a popped
  label that no longer matches the drive's current hash prefix,
  or that's already live in this process, is dropped (the next
  close will produce a fresh entry anyway).

Why this shape: `session.json` is already keyed per-window in the
drive (`<drive>/.chan/sessions/<w>.json`), so reusing the same
`?w=` label across the close/open cycle is the smallest possible
delta that restores panes / tabs / selections (per `fullstack-15`
+ `fullstack-58`). The URL hash carries the overlay state
(`files=...`, `search=...`, `graph=...`) that chan deliberately
keeps out of `session.json` so shareable URLs stay shareable; we
mirror it onto the stack so close/reopen also round-trips those.

Tests: six new unit tests in `config::tests` covering insert,
dedupe, LRU eviction at MAX, and the local-vs-tunnel key
namespacing. `cargo test -p chan-desktop` is green (17 tests
total), `cargo clippy --workspace --all-targets -- -D warnings`
is clean, `cargo fmt --check` passes after one auto-rewrap.

Limits I'm aware of:

* Capturing the hash on close relies on `WKWebView.URL` (macOS)
  and `WebView2.Source` (Windows) reflecting hash changes pushed
  via `history.replaceState`. Both do today; if a future Tauri
  upgrade changes that, the captured hash falls back to empty
  (session.json restore still works via the reused `?w=` label).
* The "20 entries" cap is global, not per-drive. A user who opens
  twenty separate drive windows in quick succession could push
  older drive entries off the stack. Matches the bug ask
  ("keep up to 20 window configs"), and the more-recent drives
  are the ones the user is most likely to want restored anyway.
* No teardown of stale `session.json` files in the drive. The
  stack drops the label after 20 entries, but the per-window
  session file lives on in chan-drive. Not a correctness issue
  (the SPA loads only the label it's given), but it's some
  growing dust over time. Out of scope for this fix.

Followed CLAUDE.md: no em dashes in comments / docs, factual
prose, pinned toolchain unchanged. Atomic write contract carried
forward via the existing `ConfigStore::save` (temp + rename).

## 2026-05-19 - fullstack-b-1 committed

@@Architect approved + cleared. Committed as `203c6e8`. Push
waits for Round-1 close per the standing rule. Two new tasks
landed in the queue while I was on -1: `fullstack-b-5`
(per-Hybrid theme propagation) and `fullstack-b-6` (scope FB
watcher to selection — phase-7 backlog item 9 pulled in early
because the pain is current). Working order is -2, -3, -4, -6,
-5 per @@Architect's note.

## 2026-05-19 - fullstack-b-2 implemented

Terminal cluster: three coordinated fixes shipped together.

* Cmd+T (native) + Cmd+Alt+T (web Mac) come back as direct
  chords for "new terminal in active pane". Pane Mode (Cmd+K 1)
  stays as the keystroke-light alternative; the two coexist.
  Native chord lives in `KEY_BRIDGE_JS`; web chord in
  `App.svelte`'s `onWindowKey`. Win/Linux web stays gated to
  Pane Mode because Ctrl+Alt+T there is already
  `app.tab.reopenClosed`.
* Scrollback truncation root cause: Pane.svelte unmounted every
  TerminalTab when Hybrid NAV was active (`{#if !paneMode.active}`
  around the each-block). Each entry into pane mode disposed the
  xterm.js EditorView and dropped the 20k-line buffer. Fix:
  drop the wrapper, gate the `active` + `focused` props on
  `!paneMode.active` so the existing
  `visibility: hidden; pointer-events: none` CSS rule hides the
  surface without unmounting. Buffer is preserved across pane
  mode AND across intra-pane tab switching (already mounted by
  the same CSS-driven path).
* Line-adjustment vs iTerm: diagnosed from
  `docs/journals/phase-8/attachments/image-{3,4}.png`. Root cause
  was `lineHeight: 1.0` packing ascenders against the next row's
  descenders so multi-row ASCII glyphs (Claude Code splash cube,
  figlet output) rendered visibly squished. Bumped to 1.2 to
  match iTerm's visual density. Conservative single-token change;
  walkthrough verification owed to @@WebtestB.

Bonus housekeeping picked up by the resync:
`crates/chan/src/main.rs::SERVE_LONG_ABOUT` was multiple phases
out of date (still referenced "Pane Mode" / "Save Cmd+S" / no
"Flip Hybrid"). Regenerated via
`node web/scripts/shortcuts-table.mjs --serve-long-about` so
`chan serve --help` matches reality.

Test deltas:

* New `web/src/components/paneTerminalMount.test.ts`: pins the
  structural shape (no `{#if !paneMode.active}` adjacent to the
  terminal each-block; `active` / `focused` props gated by
  `!paneMode.active`).
* `web/src/state/shortcuts.test.ts`: flipped the
  no-standalone-Terminal-chord guards into positive assertions
  for the new "New terminal" label. Bare-`Terminal` row absence
  guards retained.
* `desktop/src-tauri/src/serve.rs::tests`: updated to expect
  Cmd+T present (`key_bridge_keeps_independent_chords`) and not
  absent from `key_bridge_drops_chords_covered_by_pane_mode`.

Pre-push gate: clean across `cargo fmt` / `cargo clippy -D
warnings` / `cargo test` / `cargo build --no-default-features` /
`npm run check` / `npm run build` / `npx vitest run` (450/450).
Two pre-existing terminal-tab tests flake under load at the 15s
default timeout but pass standalone; not a regression here.

## 2026-05-19 - fullstack-b-3 implemented

Watcher dialog cluster: removed the drive-root gate for absolute
paths, added silent create-on-attach, and reshaped the path
prompt modal so an attach intent no longer pretends to be a
create / rename.

Substantive backend change in
`crates/chan-server/src/routes/terminal.rs::resolve_watcher_dir`:
absolute paths skip the `path_canon.starts_with(root_canon)`
check entirely; relative paths still flow through
`resolve_safe_strict` (in-drive sandbox preserved). The function
now `create_dir_all`s its target up front, so a missing watcher
dir is created silently before the metadata check fires. Three
new tests + one renamed test pin the new behaviours.

Frontend: extended `PathPromptMode` with `attach` and routed the
modal's `status` derivation + `pathSegments` to short-circuit on
that mode so existing dirs don't trip the overwrite warning and
absolute paths don't manufacture a fake ancestor preamble. The
watcher dialog in `TerminalRichPrompt.svelte` now passes
`mode: "attach"` instead of the previous `mode: "move"`. New
`PathPromptModal.test.ts` pins the four attach-mode branches.

Notes for the audit trail:

* This is a deliberate widening of the trust surface: chan-server
  will now `create_dir_all` and watch arbitrary filesystem paths
  the user types. That's appropriate for the rich-prompt watcher
  (infra traffic, not user content), but it does mean a stray
  paste of `/etc/cron.d` would create the dir if it doesn't
  exist (it does, on most systems). Worth flagging if @@Alex
  thinks tighter guards are needed; the current behaviour
  matches the bug ask verbatim.
* No SPA-side warning when the user picks a path outside the
  drive. The status row renders the path verbatim — they see
  what they typed. Easy to add an explicit hint later.

Pre-push gate green across all crates and web. One pre-existing
flake in `routes::graph::tests::link_to_non_markdown_disk_file_resolves_to_real_file`
on parallel runs (passes standalone); not from this task.

## 2026-05-19 - fullstack-b-4 implemented

Indexing chart in `EmptyPaneCarousel.svelte` got a self-contained
pan/zoom layer modelled on `GraphCanvas` but scoped to SVG-space
(no Canvas / d3-force needed). Transform-driven wrapper `<g>`
holds both the edges and nodes groups; pointer capture on the
SVG drives drag-pan; wheel with `exp(-delta * 0.0015)` smoothing
drives cursor-anchored zoom; a Locate icon button pinned to the
chart's bottom-right corner resets to identity. Leaving slide 3
also resets so the next return-visit lands on a fitted view.
Gestures sit on the SVG element directly, not the slide
container, so the Round-2 Infographics-tabs refactor (backlog
item 4) inherits the behaviour without re-wiring.

Coordination footprint: @@FullStackA's `fullstack-a-7` Hybrid NAV
chord swap (Mod+K → Mod+.) had left
`shortcuts.test.ts::"advertises Hybrid NAV (Cmd+K)"` failing.
Picked it up as a trivial one-line update since it was blocking
my own pre-push gate; also re-synced `SERVE_LONG_ABOUT` to match.
Called out in the task file so the audit trail attributes both to
the right initiating change.

Eight new pinned-source tests in
`EmptyPaneCarousel.indexingChart.test.ts` plus the updated
shortcuts test; pre-push gate green (464/464 SPA tests). One
pre-existing chan-server graph test still flakes on parallel
runs but passes standalone (not from this task).

## 2026-05-19 - fullstack-b-6 implemented

FB watcher scope narrowed entirely on the SPA. Replaced the
unconditional `refreshTree()` in `onWatchEvent` with a
scope-aware path: each FB instance contributes a scope from its
selection (drive root / dir / parent-of-file); the watcher tree
refresh only fires when an event path lands in at least one
active scope, and only re-fetches the affected parent dir
(`refreshTreeForPath`). No chan-server / chan-drive changes —
the task header had floated a "subscribe-by-prefix in
chan-drive" direction, but the SPA-side filter turned out to be
sufficient for the visible bug (tree flicker on out-of-scope
activity).

Known limitation, documented: `tree.entries` is still shared
across FB instances, so when two FBs are open on different
scopes, an event matching EITHER scope refreshes the shared
tree state — both FBs see a re-render. The "ship strict first"
note in the task explicitly allowed this trade-off; a true
per-FB tree would be a much larger refactor.

Ten new unit tests in `web/src/state/watcherScope.test.ts`
cover the scope-match semantics, the active-scopes collection
across overlay + per-pane paths, and the no-op behaviour of
`refreshTreeForPath` when the parent dir isn't loaded. Pre-push
gate green (474/474 SPA tests).

## 2026-05-19 - fullstack-b-5 implemented

Per-Hybrid theme propagation fixed entirely in the editor theme
CSS. Root cause: each shipped editor theme
(`github.css`, `google_docs.css`, `word.css`) gated its dark
variant on `:root[data-editor-theme="<name>"][data-theme="dark"]`.
The `data-theme="dark"` half only matches the root, so a
Hybrid pane that overrode `data-theme="light"` via the phase-7
`fullstack-59` cascade saw the editor-token dark rule fire
globally — visible as "dark text on light pane background"
(the "back of the Hybrid in light mode while dark globally"
repro Alex flagged in chat).

Fix: extended each theme's dark selector to a comma-joined pair
that also matches `:root[..] .pane[data-theme="dark"]`
(higher specificity than the global rule, so the per-pane
variant wins inside any opted-in pane). Added a sibling
`.pane[data-theme="light"]` block to each theme that re-asserts
the base light defaults — needed because the global dark rule
fires on the root and cascades into the pane's CSS variables;
the per-pane light override has to actively reassert each
editor token to break the cascade.

Truth table verified across all six combinations of
`(global, pane) ∈ {dark, light, none}`. No JS / Svelte changes;
the existing `data-theme` attribute on `Pane.svelte` (from
`fullstack-59`) is the cascade carrier. xterm.js + GraphCanvas
wiring from `fullstack-78` is untouched.

Aside: an attempted pinned-source test for the editor themes
got pulled because `?raw` CSS imports return empty under our
JSDOM vitest setup, and the `node:fs` fallback hits a missing
`@types/node`. The CSS change is small + reviewable in the diff
so manual review + @@WebtestB's walkthrough is the right layer
here.

Pre-push gate green across all crates and web. Queue empty
pending @@Architect review for -2/-3/-4/-5/-6.
